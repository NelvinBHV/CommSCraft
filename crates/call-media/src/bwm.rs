//! Bandwidth management of one call (call admission control, function 32): the state of the
//! bandwidth-managing endpoint. [`crate::CallMedia`] sends the requests through its TURN clients.
//!
//! Evidence: file 23 (BW-01 … BW-16). Sources: [MS-ICE2BWM] 20250520 §3.1.2 (commit timer 500 ms,
//! update timer ≤ 19 s), §3.1.4.1–3.1.4.8 (who checks and when, site addresses of check and
//! commit, UDP server preferred), §3.1.5.1 (responses), pp. 15–20; [MS-TURNBWM] 20250520
//! §3.2.4–3.2.6 (pp. 26–29).
//!
//! Who manages: the callee, which knows the caller's candidates from the offer while it gathers
//! (check, then commit and updates); the caller only towards a peer without ICE (commit and
//! updates, §3.1.4.4.2). The ICE caller towards an ICE peer sends nothing.
//!
//! Inference (documented in file 23): the reserved amount is the audio stream's bandwidth on the
//! wire – 20 ms G.722 or G.711 frames (160 octets) plus RTP (12), SRTP tag and MKI (11), UDP (8)
//! and IPv4 (20) headers, 50 packets/s = 84.4 kbit/s, rounded up to 85 – for minimum and
//! maximum, send and receive. Without an answer to the check within 3 s the call goes on without
//! policy. Commits are repeated at most 120 times (one minute), then given up. At the end of the
//! call an update with the amount 0 cancels the reservation (§2.2.3).
//!
//! Security by design: only authenticated TURN answers reach this state (the TURN client checks
//! MESSAGE-INTEGRITY); a check answer counts only for a check we sent (transaction ID); an
//! invalid answer changes nothing; an all-zero reservation ID ends all bandwidth messages.

use std::net::SocketAddr;

use commscraft_sdp as sdp;
use commscraft_turn_bwm as bwm;

/// The audio stream's bandwidth in kbit/s (see the module documentation).
pub const AUDIO_KBPS: u32 = 85;
/// [MS-ICE2BWM] §3.1.2.
pub const COMMIT_MS: u64 = 500;
pub const UPDATE_MS: u64 = 19_000;
/// How long the check may take (design) and how often a commit is tried.
pub const CHECK_WAIT_MS: u64 = 3_000;
pub const MAX_COMMITS: u32 = 120;

pub fn amount() -> bwm::Amount {
    bwm::Amount {
        min_send: AUDIO_KBPS,
        max_send: AUDIO_KBPS,
        min_receive: AUDIO_KBPS,
        max_receive: AUDIO_KBPS,
    }
}

/// What the application knows when bandwidth management is on (`enableBWPolicyCheck`).
#[derive(Debug, Clone)]
pub struct BwmConfig {
    /// SIP Call-ID of the call (for the check, §2.2.13); may be empty.
    pub call_id: String,
    pub profile: bwm::LocationProfile,
    /// The caller's ICE candidates from the offer: set for the callee, which then checks.
    pub peer: Option<sdp::Ice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Idle,
    Checking,
    /// Check done (with or without a policy).
    Checked,
    Committing,
    Committed,
    /// Not managed or given up: no further bandwidth messages.
    Off,
}

#[derive(Debug)]
pub(crate) struct Bwm {
    pub cfg: BwmConfig,
    pub phase: Phase,
    pub policy: Option<bwm::Policy>,
    /// Outstanding checks: (socket, transaction ID, asked remote relay, asked local relay).
    pub checks: Vec<(usize, [u8; 16], bool, bool)>,
    /// Sockets a check went out on (one check per relay server).
    pub checked_on: Vec<usize>,
    pub check_deadline: u64,
    /// The commit target: socket of the policy server and the media path's sites.
    pub commit: Option<(usize, bwm::Sites)>,
    pub commit_tid: Option<[u8; 16]>,
    pub commits: u32,
    pub reservation: Option<[u8; 16]>,
    /// The send bandwidth the server reserved (kbit/s), for the RTCP policy extension.
    pub reserved_send: Option<u32>,
    pub next: u64,
}

impl Bwm {
    pub fn new(cfg: BwmConfig) -> Self {
        Bwm {
            cfg,
            phase: Phase::Idle,
            policy: None,
            checks: Vec::new(),
            checked_on: Vec::new(),
            check_deadline: 0,
            commit: None,
            commit_tid: None,
            commits: 0,
            reservation: None,
            reserved_send: None,
            next: 0,
        }
    }

    /// Gathering must wait for the check (callee only, at most [`CHECK_WAIT_MS`]).
    pub fn checking(&self, now: u64) -> bool {
        self.cfg.peer.is_some()
            && matches!(self.phase, Phase::Idle | Phase::Checking)
            && (self.phase == Phase::Idle || now < self.check_deadline)
    }

    /// An authenticated Allocate answer to a bandwidth request.
    pub fn on_answer(&mut self, attributes: &[(u16, Vec<u8>)], transaction: &[u8; 16], now: u64) {
        if let Some(i) = self.checks.iter().position(|c| c.1 == *transaction) {
            let (_, _, rr, lr) = self.checks.remove(i);
            // §3.1.4.8.1.1: the first valid check answer counts, later ones are ignored.
            if self.phase == Phase::Checking {
                if let Some(p) = bwm::check_response(attributes, rr, lr) {
                    self.policy = Some(p);
                    self.phase = Phase::Checked;
                } else if self.checks.is_empty() {
                    self.phase = Phase::Checked;
                }
            }
            return;
        }
        if self.commit_tid == Some(*transaction) {
            self.commit_tid = None;
            if let Some(r) = bwm::reservation_response(attributes, true) {
                if r.managed() && self.phase == Phase::Committing {
                    self.reservation = Some(r.id);
                    self.reserved_send = r.amount.map(|a| a.max_send).filter(|v| *v > 0);
                    self.phase = Phase::Committed;
                    self.next = now + UPDATE_MS;
                } else {
                    // §3.1.5.1.2: a null reservation ID – no further messages.
                    self.phase = Phase::Off;
                }
            }
        }
    }

    /// A bandwidth request on `sock` was refused or not answered.
    pub fn on_failed(&mut self, sock: usize) {
        self.checks.retain(|c| c.0 != sock);
        if self.phase == Phase::Checking && self.checks.is_empty() {
            self.phase = Phase::Checked;
        }
        if self.commit.is_some_and(|(s, _)| s == sock) {
            self.commit_tid = None;
        }
    }

    /// The media path is known: commit it (§3.1.4.8.2.2), first at once, then every 500 ms.
    pub fn start_commit(&mut self, sock: usize, sites: bwm::Sites, now: u64) {
        if matches!(
            self.phase,
            Phase::Off | Phase::Committing | Phase::Committed
        ) {
            return;
        }
        self.commit = Some((sock, sites));
        self.phase = Phase::Committing;
        self.commits = 0;
        self.next = now;
    }
}

/// One candidate of the peer for the site addresses: an IPv4 host candidate of component 1 if
/// there is one (§3.1.4.8.1.1: "any one Host Candidate", IPv4 first), else any non-relayed one.
pub fn peer_site(ice: &sdp::Ice) -> Option<SocketAddr> {
    let addr = |c: &sdp::Candidate| SocketAddr::new(c.address, c.port);
    let rtp = |c: &&sdp::Candidate| c.component == 1 && c.transport == sdp::Transport::Udp;
    ice.candidates
        .iter()
        .filter(rtp)
        .filter(|c| c.kind == sdp::CandidateType::Host)
        .min_by_key(|c| !c.address.is_ipv4())
        .or_else(|| {
            ice.candidates
                .iter()
                .filter(rtp)
                .find(|c| c.kind != sdp::CandidateType::Relay)
        })
        .map(addr)
}

/// The peer's relayed candidate of component 1 (IPv4 first).
pub fn peer_relay(ice: &sdp::Ice) -> Option<SocketAddr> {
    ice.candidates
        .iter()
        .filter(|c| c.component == 1 && c.kind == sdp::CandidateType::Relay)
        .min_by_key(|c| (c.transport != sdp::Transport::Udp, !c.address.is_ipv4()))
        .map(|c| SocketAddr::new(c.address, c.port))
}

/// Whether `addr` is one of the peer's relayed candidates.
pub fn is_peer_relay(ice: &sdp::Ice, addr: SocketAddr) -> bool {
    ice.candidates
        .iter()
        .any(|c| c.kind == sdp::CandidateType::Relay && SocketAddr::new(c.address, c.port) == addr)
}
