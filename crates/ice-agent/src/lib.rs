//! ICE agent for one media stream (MS-ICE2 over UDP, sans-I/O): candidate pairs, connectivity
//! checks in both STUN formats, peer-reflexive learning, triggered checks, regular nomination,
//! keep-alive and consent freshness.
//!
//! Evidence: `docs/protocol/evidence/13-ice-connectivity.md` (IC-01 … IC-17, K-IC-2, O-IC-2).
//! Sources: [MS-ICE2] 20250520 §2.2.2–2.2.3 (pp. 17–19), §3.1.4.8.2 (pp. 25–28), §3.1.5.2–3.1.5.3
//! (pp. 28–30), §3.1.6 (pp. 30–31); draft-ietf-mmusic-ice-19 §4.1.2 (priorities), §5.7
//! (pairs, priority, states), §5.8 (scheduling), §7.1 (requests, responses, peer-reflexive),
//! §7.2 (incoming requests, triggered checks), §8.1.1.1 (regular nomination), §16 (Ta, RTO).
//!
//! Scope: UDP candidates (TCP candidates – IC-10/IC-11 – follow with the TCP media path). The
//! 2007-generation duplicate with the misprinted CRC table (IC-14) is not sent (OB-42: only
//! towards peers without IMPLEMENTATION-VERSION from Office Communicator 2007).
//!
//! Security by design: requests must carry our ufrag, FINGERPRINT and a valid
//! MESSAGE-INTEGRITY (otherwise discarded or answered 401/431, §3.1.5.2.2); responses are
//! matched by transaction ID and by the exact address pair, and must verify with the peer's
//! password and carry a usable XOR-MAPPED-ADDRESS (§3.1.5.3.1); at most 40 local + 40 remote
//! candidates and 80 pairs (IC-10); retransmissions and all phases are bounded by timers.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};

use commscraft_stun::{self as stun, Integrity, Message};

/// Source of randomness (a CSPRNG in production): tie breaker and transaction IDs.
pub type Random = Box<dyn FnMut(&mut [u8]) + Send>;

pub const MAX_CANDIDATES: usize = 40;
pub const MAX_PAIRS: usize = 80;
/// Pacing of checks (ice-19 §16: Ta ≥ 20 ms).
pub const TA_MS: u64 = 20;
/// Retransmissions of one check: RTO 100 ms doubled, 5 attempts (design within ice-19 §16).
const RTO_MS: u64 = 100;
const MAX_ATTEMPTS: u32 = 5;
/// IC-06 timers (milliseconds).
pub const CHECK_PHASE_MS: u64 = 10_000;
pub const CHECK_PHASE_AFTER_PEER_MS: u64 = 5_000;
pub const NOMINATION_MS: u64 = 10_000;
pub const KEEPALIVE_MS: u64 = 15_000;
pub const CONSENT_INTERVAL_MS: u64 = 5_000;
pub const CONSENT_TIMEOUT_MS: u64 = 30_000;
/// IMPLEMENTATION-VERSION we announce: we understand RFC 5389 (§3.1.5.2).
pub const OUR_IMPLEMENTATION_VERSION: u32 = 3;
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_RESPONSE: u16 = 0x0101;
const BINDING_ERROR: u16 = 0x0111;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Host,
    ServerReflexive,
    PeerReflexive,
    Relayed,
}

impl Kind {
    /// Type preference (ice-19 §4.1.2.2; IE-05: host 126, srflx 100, relay 0, prflx 110).
    pub fn preference(self) -> u8 {
        match self {
            Kind::Host => 126,
            Kind::PeerReflexive => 110,
            Kind::ServerReflexive => 100,
            Kind::Relayed => 0,
        }
    }
}

/// A local candidate. `base` is where it sends from (host socket or TURN allocation); the
/// adapter maps `Transmit::local` back to that socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub foundation: String,
    pub component: u8,
    pub kind: Kind,
    pub address: SocketAddr,
    pub base: SocketAddr,
    pub priority: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    pub foundation: String,
    pub component: u8,
    pub address: SocketAddr,
    pub priority: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Frozen,
    Waiting,
    InProgress,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone)]
struct Pair {
    local: usize,
    remote: usize,
    priority: u64,
    state: State,
    nominated: bool,
    /// The peer asked for this pair with USE-CANDIDATE (controlled side).
    use_candidate_received: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Purpose {
    Check,
    Nomination,
    Consent,
}

#[derive(Debug, Clone)]
struct Transaction {
    pair: usize,
    purpose: Purpose,
    attempts: u32,
    next: u64,
    wire: Vec<Vec<u8>>,
}

/// Something the adapter sends: from local candidate `local` (its base socket) to `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transmit {
    pub local: usize,
    pub to: SocketAddr,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Media may use this pair now (first succeeded RTP pair, §3.1.4.8.3).
    Usable { local: usize, remote: SocketAddr },
    /// The nominated pairs per component (1 = RTP, 2 = RTCP): the controlling agent puts them
    /// into the final offer (IC-07).
    Selected {
        rtp: (usize, SocketAddr),
        rtcp: Option<(usize, SocketAddr)>,
    },
    /// No usable path, nomination failed or consent lost: the call fails (IC-17, §3.1.6.5).
    Failed(&'static str),
}

/// STUN format of the peer (§3.1.5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeerFormat {
    Unknown,
    Legacy,
    Rfc5389,
}

pub struct Agent {
    controlling: bool,
    tie_breaker: [u8; 8],
    local_ufrag: String,
    local_pwd: String,
    remote_ufrag: String,
    remote_pwd: String,
    locals: Vec<Local>,
    remotes: Vec<Remote>,
    pairs: Vec<Pair>,
    triggered: Vec<usize>,
    transactions: HashMap<[u8; 16], Transaction>,
    next_check: u64,
    check_deadline: Option<u64>,
    shortened: bool,
    nomination_deadline: Option<u64>,
    selected: Option<(usize, Option<usize>)>,
    usable_sent: bool,
    format: PeerFormat,
    last_sent: u64,
    consent_next: u64,
    consent_deadline: Option<u64>,
    random: Random,
    done: bool,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("controlling", &self.controlling)
            .field("pairs", &self.pairs.len())
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

/// ice-19 §5.7.2.
pub fn pair_priority(controlling: u32, controlled: u32) -> u64 {
    let (g, d) = (u64::from(controlling), u64::from(controlled));
    (g.min(d) << 32) + 2 * g.max(d) + u64::from(g > d)
}

fn usable_address(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => !(v.is_unspecified() || v.is_broadcast() || v.is_multicast()),
        IpAddr::V6(v) => !(v.is_unspecified() || v.is_multicast()),
    }
}

impl Agent {
    /// `random` must be a CSPRNG (tie breaker, transaction IDs).
    pub fn new(
        controlling: bool,
        local_ufrag: String,
        local_pwd: String,
        mut random: Random,
    ) -> Self {
        let mut tie_breaker = [0u8; 8];
        random(&mut tie_breaker);
        Agent {
            controlling,
            tie_breaker,
            local_ufrag,
            local_pwd,
            remote_ufrag: String::new(),
            remote_pwd: String::new(),
            locals: Vec::new(),
            remotes: Vec::new(),
            pairs: Vec::new(),
            triggered: Vec::new(),
            transactions: HashMap::new(),
            next_check: 0,
            check_deadline: None,
            shortened: false,
            nomination_deadline: None,
            selected: None,
            usable_sent: false,
            format: PeerFormat::Unknown,
            last_sent: 0,
            consent_next: 0,
            consent_deadline: None,
            random,
            done: false,
        }
    }

    pub fn add_local(&mut self, c: Local) -> Option<usize> {
        if self.locals.len() >= MAX_CANDIDATES || !(1..=2).contains(&c.component) {
            return None;
        }
        self.locals.push(c);
        Some(self.locals.len() - 1)
    }

    pub fn locals(&self) -> &[Local] {
        &self.locals
    }

    /// The peer's credentials and candidates (from its offer or answer); starts the checks.
    pub fn set_remote(&mut self, ufrag: &str, pwd: &str, candidates: Vec<Remote>, now: u64) {
        self.remote_ufrag = ufrag.to_string();
        self.remote_pwd = pwd.to_string();
        for c in candidates.into_iter().take(MAX_CANDIDATES) {
            if (1..=2).contains(&c.component) && usable_address(c.address.ip()) {
                self.add_remote(c);
            }
        }
        self.form_pairs();
        self.next_check = now;
        self.check_deadline = Some(now + CHECK_PHASE_MS);
    }

    fn add_remote(&mut self, c: Remote) -> usize {
        if let Some(i) = self
            .remotes
            .iter()
            .position(|r| r.address == c.address && r.component == c.component)
        {
            return i;
        }
        self.remotes.push(c);
        self.remotes.len() - 1
    }

    fn pair_prio(&self, l: &Local, r: &Remote) -> u64 {
        if self.controlling {
            pair_priority(l.priority, r.priority)
        } else {
            pair_priority(r.priority, l.priority)
        }
    }

    fn form_pairs(&mut self) {
        for (li, l) in self.locals.iter().enumerate() {
            // Server-reflexive candidates send from their base: the host pair covers them
            // (ice-19 §5.7.3 pruning).
            if l.kind == Kind::ServerReflexive {
                continue;
            }
            for (ri, r) in self.remotes.iter().enumerate() {
                if l.component == r.component
                    && l.base.is_ipv4() == r.address.is_ipv4()
                    && !self.pairs.iter().any(|p| p.local == li && p.remote == ri)
                    && self.pairs.len() < MAX_PAIRS
                {
                    let priority = self.pair_prio(l, r);
                    self.pairs.push(Pair {
                        local: li,
                        remote: ri,
                        priority,
                        state: State::Frozen,
                        nominated: false,
                        use_candidate_received: false,
                    });
                }
            }
        }
        self.pairs.sort_by_key(|p| std::cmp::Reverse(p.priority));
        // ice-19 §5.7.4: per foundation the pair with the lowest component starts Waiting.
        let mut seen: Vec<(String, String)> = Vec::new();
        for p in self.pairs.iter_mut() {
            let (Some(l), Some(r)) = (self.locals.get(p.local), self.remotes.get(p.remote)) else {
                continue;
            };
            let f = (l.foundation.clone(), r.foundation.clone());
            if p.state == State::Frozen && l.component == 1 && !seen.contains(&f) {
                p.state = State::Waiting;
                seen.push(f);
            }
        }
    }

    /// A fresh transaction ID; IDs still in use (also by their low 96 bits, which identify the
    /// RFC 5389 duplicate) are drawn again.
    fn transaction_id(&mut self) -> [u8; 16] {
        let mut t = [0u8; 16];
        for _ in 0..8 {
            (self.random)(&mut t);
            if !self
                .transactions
                .keys()
                .any(|k| k.get(4..16) == t.get(4..16))
            {
                break;
            }
        }
        t
    }

    fn integrity_key(&self, outgoing: bool) -> Vec<u8> {
        // Short-term credentials (ice-19 §7.1.1.3): requests we send and responses to them use
        // the peer's password; requests we receive and our responses use ours.
        if outgoing {
            self.remote_pwd.as_bytes().to_vec()
        } else {
            self.local_pwd.as_bytes().to_vec()
        }
    }

    /// Encodes a message in one format with MI and FINGERPRINT.
    fn seal(&self, m: &Message, key: &[u8]) -> Option<Vec<u8>> {
        let raw = m.encode().ok()?;
        let integrity = match m.format() {
            stun::Format::Rfc5389 => Integrity::Rfc5389(key),
            stun::Format::Legacy => Integrity::LegacySha1(key),
        };
        stun::with_fingerprint(stun::with_integrity(raw, integrity).ok()?).ok()
    }

    fn request(&mut self, pair: usize, purpose: Purpose) -> Option<(Vec<Vec<u8>>, [u8; 16])> {
        let p = self.pairs.get(pair)?.clone();
        let l = self.locals.get(p.local)?.clone();
        let tid = self.transaction_id();
        let formats: Vec<stun::Format> = match (purpose, self.format) {
            // Consent requests always use RFC 5389 (§3.1.6.5).
            (Purpose::Consent, _) | (_, PeerFormat::Rfc5389) => vec![stun::Format::Rfc5389],
            (_, PeerFormat::Legacy) => vec![stun::Format::Legacy],
            // Unknown: STUN-02 plus an RFC 5389 duplicate (§3.1.5.2, IC-03).
            (_, PeerFormat::Unknown) => vec![stun::Format::Legacy, stun::Format::Rfc5389],
        };
        let prflx_priority = commscraft_ice::candidate_priority(
            Kind::PeerReflexive.preference(),
            ((l.priority >> 8) & 0xffff) as u16,
            l.component,
        );
        let key = self.integrity_key(true);
        let mut wires = Vec::new();
        for f in formats {
            let transaction = match f {
                stun::Format::Legacy => tid,
                stun::Format::Rfc5389 => {
                    let mut r = [0u8; 12];
                    r.copy_from_slice(tid.get(4..16).unwrap_or(&[0; 12]));
                    Message::rfc5389_transaction(r)
                }
            };
            let mut m = Message::new(BINDING_REQUEST, transaction);
            m.push(
                stun::USERNAME,
                format!("{}:{}", self.remote_ufrag, self.local_ufrag).into_bytes(),
            );
            m.push(stun::PRIORITY, prflx_priority.to_be_bytes().to_vec());
            if self.controlling {
                m.push(stun::ICE_CONTROLLING, self.tie_breaker.to_vec());
                if purpose == Purpose::Nomination {
                    m.push(stun::USE_CANDIDATE, Vec::new());
                }
            } else {
                m.push(stun::ICE_CONTROLLED, self.tie_breaker.to_vec());
            }
            if purpose != Purpose::Consent {
                // CANDIDATE-IDENTIFIER: foundation, NUL-padded to 4 octets (§2.2.2.1).
                let mut v = l.foundation.as_bytes().to_vec();
                v.resize(v.len().div_ceil(4) * 4, 0);
                m.push(stun::CANDIDATE_IDENTIFIER, v);
            }
            m.push(
                stun::IMPLEMENTATION_VERSION,
                OUR_IMPLEMENTATION_VERSION.to_be_bytes().to_vec(),
            );
            wires.push(self.seal(&m, &key)?);
        }
        Some((wires, tid))
    }

    fn send_check(&mut self, pair: usize, purpose: Purpose, now: u64, out: &mut Vec<Transmit>) {
        let Some((wires, tid)) = self.request(pair, purpose) else {
            return;
        };
        let (Some(p), Some(_)) = (self.pairs.get_mut(pair), Some(())) else {
            return;
        };
        if purpose == Purpose::Check || purpose == Purpose::Nomination {
            p.state = State::InProgress;
        }
        let (local, remote) = (p.local, p.remote);
        let Some(to) = self.remotes.get(remote).map(|r| r.address) else {
            return;
        };
        for w in &wires {
            out.push(Transmit {
                local,
                to,
                data: w.clone(),
            });
        }
        self.last_sent = now;
        // Both formats share the 96 low bits; the key is the legacy 128-bit ID.
        self.transactions.insert(
            tid,
            Transaction {
                pair,
                purpose,
                attempts: 1,
                next: now + RTO_MS,
                wire: wires,
            },
        );
    }

    /// Note that media went out (resets the keep-alive timer, IC-05).
    pub fn on_media_sent(&mut self, now: u64) {
        self.last_sent = now;
    }

    pub fn on_tick(&mut self, now: u64) -> (Vec<Transmit>, Vec<Event>) {
        let mut out = Vec::new();
        let mut events = Vec::new();
        if self.done && self.selected.is_none() {
            return (out, events);
        }
        // Retransmissions.
        let due: Vec<[u8; 16]> = self
            .transactions
            .iter()
            .filter(|(_, t)| now >= t.next)
            .map(|(k, _)| *k)
            .collect();
        for k in due {
            let Some(t) = self.transactions.get_mut(&k) else {
                continue;
            };
            if t.attempts >= MAX_ATTEMPTS {
                let (pair, purpose) = (t.pair, t.purpose);
                self.transactions.remove(&k);
                if purpose != Purpose::Consent {
                    if let Some(p) = self.pairs.get_mut(pair) {
                        if p.state == State::InProgress {
                            p.state = State::Failed;
                        }
                    }
                }
                continue;
            }
            t.attempts += 1;
            t.next = now + (RTO_MS << t.attempts.min(6));
            let (pair, wire) = (t.pair, t.wire.clone());
            if let Some((local, to)) = self.endpoints(pair) {
                for w in wire {
                    out.push(Transmit { local, to, data: w });
                }
            }
        }
        if self.selected.is_none() {
            self.run_checks(now, &mut out, &mut events);
        } else {
            self.run_keepalive(now, &mut out, &mut events);
        }
        (out, events)
    }

    fn endpoints(&self, pair: usize) -> Option<(usize, SocketAddr)> {
        let p = self.pairs.get(pair)?;
        Some((p.local, self.remotes.get(p.remote)?.address))
    }

    fn run_checks(&mut self, now: u64, out: &mut Vec<Transmit>, events: &mut Vec<Event>) {
        if let Some(d) = self.nomination_deadline {
            if now >= d {
                self.done = true;
                events.push(Event::Failed("nomination checks did not complete"));
                return;
            }
            // The controlled agent still answers USE-CANDIDATE with triggered checks.
            if !self.controlling && now >= self.next_check && !self.triggered.is_empty() {
                self.next_check = now + TA_MS;
                let i = self.triggered.remove(0);
                self.send_check(i, Purpose::Check, now, out);
            }
            return;
        }
        if self.check_deadline.is_some_and(|d| now >= d) {
            self.check_deadline = None;
            if self.controlling {
                self.nominate(now, out, events);
            } else {
                // The controlled agent stops checking and waits for USE-CANDIDATE.
                self.nomination_deadline = Some(now + NOMINATION_MS);
            }
            return;
        }
        if self.check_deadline.is_none() || now < self.next_check {
            return;
        }
        self.next_check = now + TA_MS;
        let next = if self.triggered.is_empty() {
            self.pairs
                .iter()
                .position(|p| p.state == State::Waiting)
                .or_else(|| self.pairs.iter().position(|p| p.state == State::Frozen))
        } else {
            Some(self.triggered.remove(0))
        };
        if let Some(i) = next {
            self.send_check(i, Purpose::Check, now, out);
        }
        // All pairs done: end the phase early.
        if self
            .pairs
            .iter()
            .all(|p| matches!(p.state, State::Succeeded | State::Failed))
            && !self.pairs.is_empty()
            && self.triggered.is_empty()
            && self.transactions.is_empty()
        {
            self.check_deadline = Some(now);
        }
    }

    /// The best succeeded RTP pair and its RTCP partner (same local and remote foundations).
    fn best(&self) -> Option<(usize, Option<usize>)> {
        let rtp = self.pairs.iter().position(|p| {
            p.state == State::Succeeded
                && self.locals.get(p.local).is_some_and(|l| l.component == 1)
        })?;
        let p = self.pairs.get(rtp)?;
        let (lf, rf) = (
            self.locals.get(p.local)?.foundation.clone(),
            self.remotes.get(p.remote)?.foundation.clone(),
        );
        let rtcp = self.pairs.iter().position(|q| {
            q.state == State::Succeeded
                && self
                    .locals
                    .get(q.local)
                    .is_some_and(|l| l.component == 2 && l.foundation == lf)
                && self
                    .remotes
                    .get(q.remote)
                    .is_some_and(|r| r.foundation == rf)
        });
        Some((rtp, rtcp))
    }

    /// Regular nomination on the best valid pair (§3.1.4.8.2.6, ice-19 §8.1.1.1).
    fn nominate(&mut self, now: u64, out: &mut Vec<Transmit>, events: &mut Vec<Event>) {
        match self.best() {
            None => {
                self.done = true;
                events.push(Event::Failed("no valid candidate pair"));
            }
            Some((rtp, rtcp)) => {
                self.nomination_deadline = Some(now + NOMINATION_MS);
                self.transactions
                    .retain(|_, t| t.purpose == Purpose::Consent);
                for pair in std::iter::once(rtp).chain(rtcp) {
                    self.send_check(pair, Purpose::Nomination, now, out);
                }
            }
        }
    }

    fn check_selection(&mut self, events: &mut Vec<Event>) {
        if self.selected.is_some() {
            return;
        }
        let nominated = |c: u8| {
            self.pairs.iter().position(|p| {
                p.nominated
                    && p.state == State::Succeeded
                    && self.locals.get(p.local).is_some_and(|l| l.component == c)
            })
        };
        let Some(rtp) = nominated(1) else { return };
        let rtcp = nominated(2);
        let remote_has_rtcp = self.remotes.iter().any(|r| r.component == 2);
        if remote_has_rtcp && rtcp.is_none() {
            return;
        }
        let (Some(a), b) = (self.endpoints(rtp), rtcp.and_then(|r| self.endpoints(r))) else {
            return;
        };
        self.selected = Some((rtp, rtcp));
        self.nomination_deadline = None;
        self.done = true;
        events.push(Event::Selected { rtp: a, rtcp: b });
    }

    fn run_keepalive(&mut self, now: u64, out: &mut Vec<Transmit>, events: &mut Vec<Event>) {
        let Some((rtp, _)) = self.selected else {
            return;
        };
        if self.format == PeerFormat::Rfc5389 {
            // Consent freshness (§3.1.6.5, from SfB 2016).
            if self.consent_deadline.is_none() {
                self.consent_deadline = Some(now + CONSENT_TIMEOUT_MS);
            }
            if self.consent_deadline.is_some_and(|d| now >= d) {
                self.consent_deadline = None;
                self.selected = None;
                events.push(Event::Failed("consent lost"));
                return;
            }
            if now >= self.consent_next {
                self.consent_next = now + CONSENT_INTERVAL_MS;
                self.send_check(rtp, Purpose::Consent, now, out);
            }
        } else if now.saturating_sub(self.last_sent) >= KEEPALIVE_MS {
            // ICE keep-alive: Binding Request with MESSAGE-INTEGRITY only (§2.2.3, IC-05).
            let tid = self.transaction_id();
            let m = Message::new(BINDING_REQUEST, tid);
            let key = self.integrity_key(true);
            if let (Some(w), Some((local, to))) = (
                m.encode()
                    .ok()
                    .and_then(|raw| stun::with_integrity(raw, Integrity::LegacySha1(&key)).ok()),
                self.endpoints(rtp),
            ) {
                out.push(Transmit { local, to, data: w });
                self.last_sent = now;
            }
        }
    }

    /// A STUN packet received on local candidate `local` from `from`. Non-STUN data must not be
    /// passed here (first two bits 0 and a plausible length decide, the adapter checks).
    pub fn on_stun(
        &mut self,
        local: usize,
        from: SocketAddr,
        p: &[u8],
        now: u64,
    ) -> (Vec<Transmit>, Vec<Event>) {
        let mut out = Vec::new();
        let mut events = Vec::new();
        let Ok(m) = Message::decode(p) else {
            return (out, events);
        };
        match m.kind {
            BINDING_REQUEST => self.on_request(local, from, p, &m, now, &mut out, &mut events),
            BINDING_RESPONSE | BINDING_ERROR => {
                self.on_response(local, from, p, &m, now, &mut out, &mut events)
            }
            _ => {}
        }
        (out, events)
    }

    fn learn_format(&mut self, m: &Message) {
        if self.format != PeerFormat::Unknown {
            return;
        }
        let v = m
            .get(stun::IMPLEMENTATION_VERSION)
            .and_then(|v| <[u8; 4]>::try_from(v).ok())
            .map(u32::from_be_bytes);
        // §3.1.5.2: ≥ 3 or missing ⇒ RFC 5389; < 3 ⇒ STUN-02.
        self.format = match v {
            Some(v) if v < 3 => PeerFormat::Legacy,
            _ => PeerFormat::Rfc5389,
        };
    }

    #[allow(clippy::too_many_arguments)]
    fn on_request(
        &mut self,
        local: usize,
        from: SocketAddr,
        p: &[u8],
        m: &Message,
        now: u64,
        out: &mut Vec<Transmit>,
        events: &mut Vec<Event>,
    ) {
        // §3.1.5.2.2: USERNAME "ourUfrag:…" and FINGERPRINT required, else discard.
        let Some(user) = m.get(stun::USERNAME).map(|u| u.to_vec()) else {
            return;
        };
        let ours = user
            .split(|b| *b == b':')
            .next()
            .is_some_and(|first| first == self.local_ufrag.as_bytes());
        if !ours || stun::verify_fingerprint(p).is_err() || self.locals.get(local).is_none() {
            return;
        }
        let key = self.integrity_key(false);
        let integrity = match m.format() {
            stun::Format::Rfc5389 => Integrity::Rfc5389(&key),
            stun::Format::Legacy => Integrity::LegacySha1(&key),
        };
        let error = if m.get(stun::MESSAGE_INTEGRITY).is_none() {
            Some(401)
        } else if stun::verify_integrity(p, integrity).is_err() {
            Some(431)
        } else {
            None
        };
        let mut r = Message::new(
            if error.is_some() {
                BINDING_ERROR
            } else {
                BINDING_RESPONSE
            },
            m.transaction,
        );
        if let Some(code) = error {
            r.push(
                stun::ERROR_CODE,
                stun::encode_error(
                    code,
                    if code == 401 {
                        "Unauthorized"
                    } else {
                        "Integrity Check Failure"
                    },
                ),
            );
            r.push(stun::USERNAME, user);
            if let Ok(w) = r.encode() {
                out.push(Transmit {
                    local,
                    to: from,
                    data: w,
                });
            }
            return;
        }
        self.learn_format(m);
        // §3.1.5.2.3: XOR-MAPPED-ADDRESS (0x0020), USERNAME, MI, IMPLEMENTATION-VERSION; plus
        // FINGERPRINT, which the receiver requires (§3.1.5.3.1, K-IC-2).
        r.push(
            stun::XOR_MAPPED_ADDRESS,
            stun::encode_xor_address(from, &m.transaction),
        );
        r.push(stun::USERNAME, user);
        r.push(
            stun::IMPLEMENTATION_VERSION,
            OUR_IMPLEMENTATION_VERSION.to_be_bytes().to_vec(),
        );
        if let Some(w) = self.seal(&r, &key) {
            out.push(Transmit {
                local,
                to: from,
                data: w,
            });
        }
        if self.check_deadline.is_some() && !self.shortened {
            // IC-06: after the first request (and response) of the peer, 5 s remain.
            self.shortened = true;
            self.check_deadline = Some(now + CHECK_PHASE_AFTER_PEER_MS);
        }
        if self.remote_pwd.is_empty() {
            return; // Candidates not yet known (§3.1.5.2.1): answered, no checks.
        }
        // ice-19 §7.2.1.3/§7.2.1.4: learn a peer-reflexive remote candidate, triggered check.
        let component = self.locals.get(local).map_or(1, |l| l.component);
        let priority = m
            .get(stun::PRIORITY)
            .and_then(|v| <[u8; 4]>::try_from(v).ok())
            .map(u32::from_be_bytes)
            .unwrap_or(0);
        let known = self
            .remotes
            .iter()
            .position(|r| r.address == from && r.component == component);
        let ri = match known {
            Some(i) => i,
            None if self.remotes.len() < MAX_CANDIDATES && usable_address(from.ip()) => {
                let f = format!("prflx{}", self.remotes.len());
                self.add_remote(Remote {
                    foundation: f,
                    component,
                    address: from,
                    priority,
                })
            }
            None => return,
        };
        let pi = match self
            .pairs
            .iter()
            .position(|q| q.local == local && q.remote == ri)
        {
            Some(i) => i,
            None if self.pairs.len() < MAX_PAIRS => {
                let (Some(l), Some(rc)) = (self.locals.get(local), self.remotes.get(ri)) else {
                    return;
                };
                let priority = self.pair_prio(l, rc);
                self.pairs.push(Pair {
                    local,
                    remote: ri,
                    priority,
                    state: State::Waiting,
                    nominated: false,
                    use_candidate_received: false,
                });
                self.pairs.len() - 1
            }
            None => return,
        };
        let use_candidate = m.get(stun::USE_CANDIDATE).is_some();
        if let Some(pair) = self.pairs.get_mut(pi) {
            if use_candidate && !self.controlling {
                pair.use_candidate_received = true;
                if pair.state == State::Succeeded {
                    pair.nominated = true;
                }
            }
            if !matches!(pair.state, State::Succeeded | State::InProgress)
                && self.selected.is_none()
                && !self.triggered.contains(&pi)
            {
                pair.state = State::Waiting;
                self.triggered.push(pi);
            }
        }
        self.check_selection(events);
    }

    #[allow(clippy::too_many_arguments)]
    fn on_response(
        &mut self,
        local: usize,
        from: SocketAddr,
        p: &[u8],
        m: &Message,
        now: u64,
        out: &mut Vec<Transmit>,
        events: &mut Vec<Event>,
    ) {
        // Correlate by transaction ID (legacy: all 128 bits; RFC 5389: the 96 low bits).
        let key_id = match m.format() {
            stun::Format::Legacy => Some(m.transaction),
            stun::Format::Rfc5389 => self
                .transactions
                .keys()
                .find(|k| k.get(4..16) == m.transaction.get(4..16))
                .copied(),
        };
        let Some(key_id) = key_id else { return };
        let Some(t) = self.transactions.get(&key_id).cloned() else {
            return;
        };
        // ice-19 §7.1.2.1: symmetric addresses only.
        if self.endpoints(t.pair) != Some((local, from)) {
            return;
        }
        // §3.1.5.3.1 validation.
        let pw = self.integrity_key(true);
        let integrity = match m.format() {
            stun::Format::Rfc5389 => Integrity::Rfc5389(&pw),
            stun::Format::Legacy => Integrity::LegacySha1(&pw),
        };
        if stun::verify_fingerprint(p).is_err() || stun::verify_integrity(p, integrity).is_err() {
            return;
        }
        // A success response without a usable XOR-MAPPED-ADDRESS is discarded before the
        // transaction is touched, so a forged answer cannot cancel the genuine one.
        if m.kind == BINDING_RESPONSE
            && !m
                .get(stun::XOR_MAPPED_ADDRESS)
                .and_then(|v| stun::decode_xor_address(v, &m.transaction).ok())
                .is_some_and(|a| usable_address(a.ip()))
        {
            return;
        }
        self.learn_format(m);
        self.transactions.remove(&key_id);
        if t.purpose == Purpose::Consent {
            if m.kind == BINDING_RESPONSE {
                self.consent_deadline = Some(now + CONSENT_TIMEOUT_MS);
            }
            return;
        }
        if m.kind == BINDING_ERROR {
            let code = m
                .get(stun::ERROR_CODE)
                .and_then(|v| stun::decode_error(v).ok())
                .map_or(0, |(c, _)| c);
            let Some(pair) = self.pairs.get_mut(t.pair) else {
                return;
            };
            if pair.state == State::Succeeded {
                return;
            }
            match code {
                // ice-19 §7.1.2.1: role conflict – switch role and retry.
                487 => {
                    self.controlling = !self.controlling;
                    pair.state = State::Waiting;
                    self.triggered.push(t.pair);
                }
                401 | 430 | 431 | 432 | 500 => {
                    pair.state = State::Waiting;
                    self.triggered.push(t.pair);
                }
                _ => pair.state = State::Failed,
            }
            return;
        }
        // A different mapped address would reveal a peer-reflexive local candidate (ice-19
        // §7.1.2.2.1); the pair stays valid on its base, which is what we send from.
        let Some(pair) = self.pairs.get_mut(t.pair) else {
            return;
        };
        pair.state = State::Succeeded;
        if t.purpose == Purpose::Nomination || (!self.controlling && pair.use_candidate_received) {
            pair.nominated = true;
        }
        let (lf, rf) = match (self.locals.get(pair.local), self.remotes.get(pair.remote)) {
            (Some(l), Some(r)) => (l.foundation.clone(), r.foundation.clone()),
            _ => return,
        };
        // ice-19 §7.1.2.2.3: unfreeze pairs with the same foundation.
        for q in self.pairs.iter_mut() {
            if q.state == State::Frozen
                && self.locals.get(q.local).is_some_and(|l| l.foundation == lf)
                && self
                    .remotes
                    .get(q.remote)
                    .is_some_and(|r| r.foundation == rf)
            {
                q.state = State::Waiting;
            }
        }
        if self.check_deadline.is_some() && !self.shortened {
            self.shortened = true;
            self.check_deadline = Some(now + CHECK_PHASE_AFTER_PEER_MS);
        }
        if !self.usable_sent {
            if let Some((rtp, _)) = self.best() {
                if let Some((l, r)) = self.endpoints(rtp) {
                    self.usable_sent = true;
                    events.push(Event::Usable {
                        local: l,
                        remote: r,
                    });
                }
            }
        }
        let _ = out;
        self.check_selection(events);
    }
}

#[cfg(test)]
mod tests;
