//! The network side of a call's media (L2 adapter): UDP sockets for the host candidates, MS-TURN
//! allocations on the A/V Edge, the ICE agent, and the audio media core. The caller drives it
//! every few milliseconds with [`CallMedia::poll`], feeds captured frames and takes frames for
//! the speaker; devices (ALSA) stay outside.
//!
//! Evidence: OCS-UC12 ([MS-OCSPROT] §2.5.12, pp. 38–41); IC-07, IC-09 (peer without ICE: media to
//! the default address), IC-10 (candidate types), TU-24/TU-25 (relay: Send until the active
//! destination is set); AV-04 (relay addresses and ports).
//!
//! Scope: UDP candidates (host and relay). TCP candidates and IPv6 follow.
//!
//! Security by design: all sockets are non-blocking and every receive is bounded to 1500 octets;
//! datagrams from addresses that are neither ICE candidates nor the TURN server are dropped
//! before parsing; keys and ICE passwords come from the OS CSPRNG (fail closed); the relay
//! password is passed through and never stored here beyond the TURN client.

use std::net::{IpAddr, SocketAddr, UdpSocket};

use commscraft_ice_agent::{self as ice, Agent};
use commscraft_media::{self as media, Audio, Kind};
use commscraft_sdp as sdp;
use commscraft_turn as turn;

const MAX_DATAGRAM: usize = 1500;

/// A media path: ICE local candidate index and remote address.
type Path = (usize, SocketAddr);
/// The first TURN Allocates per socket index, returned by [`CallMedia::prepare`].
pub type InitialAllocates = Vec<(usize, Vec<u8>)>;
/// Candidate gathering ends after 10 s (IC-06).
pub const GATHER_MS: u64 = 10_000;

/// OS CSPRNG; aborts if the OS cannot deliver randomness (never continue with predictable keys).
pub fn os_random(buf: &mut [u8]) {
    if getrandom::fill(buf).is_err() {
        std::process::abort();
    }
}

fn random_token(len: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut raw = vec![0u8; len];
    os_random(&mut raw);
    raw.iter()
        .map(|b| {
            char::from(
                *ALPHABET
                    .get(usize::from(*b) % ALPHABET.len())
                    .unwrap_or(&b'A'),
            )
        })
        .collect()
}

/// A relay to use, with credentials from MS-AVEDGEA.
pub struct Relay {
    pub server: SocketAddr,
    pub username: String,
    pub password: zeroize::Zeroizing<Vec<u8>>,
}

enum Socket {
    Host(UdpSocket),
    Relay {
        socket: UdpSocket,
        server: SocketAddr,
        client: Box<turn::Client>,
        allocated: Option<SocketAddr>,
        active: Option<SocketAddr>,
        failed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// ICE selected the pairs (controlling side: send the final offer now, IC-07).
    Selected,
    Dtmf(char),
    /// The media path failed (no valid pair, consent lost, …): the call should end.
    Failed(String),
    /// The peer sent RTCP BYE.
    PeerBye,
}

pub struct CallMedia {
    sockets: Vec<Socket>,
    /// ICE local candidate index → socket index.
    candidate_socket: Vec<usize>,
    ufrag: String,
    pwd: String,
    key: sdp::Key,
    ice: Option<Agent>,
    audio: Option<Audio>,
    /// Peer without ICE (IC-09): RTP/RTCP straight to the default addresses.
    direct: Option<(SocketAddr, SocketAddr)>,
    selected: Option<(Path, Option<Path>)>,
    usable: Option<(usize, SocketAddr)>,
    local_ip: IpAddr,
    started: u64,
}

impl std::fmt::Debug for CallMedia {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallMedia")
            .field("sockets", &self.sockets.len())
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

fn bind(ip: IpAddr) -> std::io::Result<UdpSocket> {
    let s = UdpSocket::bind(SocketAddr::new(ip, 0))?;
    s.set_nonblocking(true)?;
    Ok(s)
}

impl CallMedia {
    /// Binds the host sockets (RTP, RTCP) and starts the TURN allocations for both components.
    pub fn prepare(
        local_ip: IpAddr,
        relay: Option<&Relay>,
        now: u64,
    ) -> std::io::Result<(Self, InitialAllocates)> {
        let mut sockets = vec![Socket::Host(bind(local_ip)?), Socket::Host(bind(local_ip)?)];
        let mut first = Vec::new();
        if let Some(r) = relay {
            for _ in 0..2 {
                let socket = bind(if r.server.is_ipv4() {
                    IpAddr::from([0, 0, 0, 0])
                } else {
                    local_ip
                })?;
                let mut client = turn::Client::new(
                    turn::Credentials {
                        username: r.username.clone(),
                        password: r.password.clone(),
                    },
                    turn::Transport::Udp,
                    turn::Stream::Audio,
                    Box::new(os_random),
                );
                if let Some(w) = client.start(now) {
                    first.push((sockets.len(), w));
                }
                sockets.push(Socket::Relay {
                    socket,
                    server: r.server,
                    client: Box::new(client),
                    allocated: None,
                    active: None,
                    failed: false,
                });
            }
        }
        let mut raw = [0u8; 30];
        os_random(&mut raw);
        let me = CallMedia {
            sockets,
            candidate_socket: Vec::new(),
            ufrag: random_token(8),
            pwd: random_token(24),
            key: sdp::Key::new(1, raw),
            ice: None,
            audio: None,
            direct: None,
            selected: None,
            usable: None,
            local_ip,
            started: now,
        };
        Ok((me, first))
    }

    /// Sends the first TURN Allocates returned by [`prepare`].
    pub fn send_initial(&self, first: InitialAllocates) {
        for (i, w) in first {
            if let Some(Socket::Relay { socket, server, .. }) = self.sockets.get(i) {
                let _ = socket.send_to(&w, *server);
            }
        }
    }

    /// True when every relay allocation succeeded or failed, or the gathering time is over.
    pub fn gathered(&self, now: u64) -> bool {
        now >= self.started + GATHER_MS
            || self.sockets.iter().all(|s| match s {
                Socket::Host(_) => true,
                Socket::Relay {
                    allocated, failed, ..
                } => allocated.is_some() || *failed,
            })
    }

    fn host_address(&self, i: usize) -> Option<SocketAddr> {
        match self.sockets.get(i)? {
            Socket::Host(s) => s.local_addr().ok(),
            Socket::Relay { .. } => None,
        }
    }

    /// The local candidates in ICE order (host RTP/RTCP, relay RTP/RTCP).
    fn candidates(&self) -> Vec<(usize, ice::Local)> {
        let mut out = Vec::new();
        for (i, s) in self.sockets.iter().enumerate() {
            let component = if i % 2 == 0 { 1 } else { 2 };
            match s {
                Socket::Host(_) => {
                    if let Some(a) = self.host_address(i) {
                        out.push((
                            i,
                            ice::Local {
                                foundation: "1".into(),
                                component,
                                kind: ice::Kind::Host,
                                address: a,
                                base: a,
                                priority: commscraft_ice::candidate_priority(
                                    126, 0xffff, component,
                                ),
                            },
                        ));
                    }
                }
                Socket::Relay {
                    allocated: Some(a), ..
                } => out.push((
                    i,
                    ice::Local {
                        foundation: "3".into(),
                        component,
                        kind: ice::Kind::Relayed,
                        address: *a,
                        base: *a,
                        // IE-05: relay UDP: type 0, local preference 15·2^12 + 7·2^9 (= 0xfe00).
                        priority: commscraft_ice::candidate_priority(0, 0xfe00, component),
                    },
                )),
                Socket::Relay { .. } => {}
            }
        }
        out
    }

    /// Our SDP parameters: ICE credentials and candidates, key, default address (the relay when
    /// allocated – it works from everywhere – otherwise the host).
    pub fn local_description(&self) -> sdp::Local {
        let cands = self.candidates();
        let to_sdp = |(_, c): &(usize, ice::Local)| sdp::Candidate {
            foundation: c.foundation.clone(),
            component: c.component,
            transport: sdp::Transport::Udp,
            priority: c.priority,
            address: c.address.ip(),
            port: c.address.port(),
            kind: match c.kind {
                ice::Kind::Relayed => sdp::CandidateType::Relay,
                ice::Kind::ServerReflexive | ice::Kind::PeerReflexive => {
                    sdp::CandidateType::ServerReflexive
                }
                ice::Kind::Host => sdp::CandidateType::Host,
            },
            related: None,
        };
        let default_rtp = cands
            .iter()
            .find(|(_, c)| c.kind == ice::Kind::Relayed && c.component == 1)
            .or_else(|| cands.iter().find(|(_, c)| c.component == 1))
            .map(|(_, c)| c.address);
        let default_rtcp = cands
            .iter()
            .find(|(_, c)| c.kind == ice::Kind::Relayed && c.component == 2)
            .or_else(|| cands.iter().find(|(_, c)| c.component == 2))
            .map(|(_, c)| c.address);
        let rtp = default_rtp.unwrap_or_else(|| SocketAddr::new(self.local_ip, 9));
        sdp::Local {
            address: rtp.ip(),
            rtp_port: rtp.port(),
            rtcp_port: default_rtcp.map_or(rtp.port().saturating_add(1), |a| a.port()),
            key: Some(self.key.clone()),
            ice: Some(sdp::Ice {
                ufrag: self.ufrag.clone(),
                pwd: self.pwd.clone(),
                candidates: cands.iter().map(to_sdp).collect(),
            }),
            direction: sdp::Direction::SendRecv,
            remote_candidates: Vec::new(),
        }
    }

    /// The final offer (IC-07): only the selected local candidates, plus remote-candidates.
    pub fn final_description(&self) -> Option<sdp::Local> {
        let ((rtp_l, rtp_r), rtcp) = self.selected?;
        let mut d = self.local_description();
        let cands = self.candidates();
        let keep: Vec<SocketAddr> = std::iter::once(rtp_l)
            .chain(rtcp.map(|(l, _)| l))
            .filter_map(|l| cands.get(l).map(|(_, c)| c.address))
            .collect();
        if let Some(ice) = d.ice.as_mut() {
            ice.candidates
                .retain(|c| keep.contains(&SocketAddr::new(c.address, c.port)));
        }
        if let Some(a) = keep.first() {
            d.address = a.ip();
            d.rtp_port = a.port();
        }
        if let Some(a) = keep.get(1) {
            d.rtcp_port = a.port();
        }
        d.remote_candidates = std::iter::once((1, rtp_r))
            .chain(rtcp.map(|(_, r)| (2, r)))
            .collect();
        Some(d)
    }

    pub fn key(&self) -> &sdp::Key {
        &self.key
    }

    /// The call is answered: start ICE (controlling = we called) and the audio core.
    pub fn start(
        &mut self,
        n: &sdp::Negotiated,
        controlling: bool,
        cname: &str,
        now: u64,
    ) -> Result<(), String> {
        let mut random = [0u8; 10];
        os_random(&mut random);
        let key = n.remote_key.as_ref().map(|_| &self.key);
        self.audio = Some(Audio::new(n, key, random, cname).map_err(|e| format!("{e:?}"))?);
        match &n.remote_ice {
            Some(remote) => {
                let mut agent = Agent::new(
                    controlling,
                    self.ufrag.clone(),
                    self.pwd.clone(),
                    Box::new(os_random),
                );
                self.candidate_socket.clear();
                for (sock, c) in self.candidates() {
                    if agent.add_local(c).is_some() {
                        self.candidate_socket.push(sock);
                    }
                }
                let remotes = remote
                    .candidates
                    .iter()
                    .filter(|c| c.transport == sdp::Transport::Udp)
                    .map(|c| ice::Remote {
                        foundation: c.foundation.clone(),
                        component: c.component,
                        address: SocketAddr::new(c.address, c.port),
                        priority: c.priority,
                    })
                    .collect();
                agent.set_remote(&remote.ufrag, &remote.pwd, remotes, now);
                self.ice = Some(agent);
            }
            None => self.direct = Some((n.remote_rtp.into(), n.remote_rtcp.into())),
        }
        Ok(())
    }

    /// New parameters from a re-INVITE (the peer's final offer or hold): keep ICE, update the
    /// direct addresses of a non-ICE peer.
    pub fn update(&mut self, n: &sdp::Negotiated) {
        if self.ice.is_none() {
            self.direct = Some((n.remote_rtp.into(), n.remote_rtcp.into()));
        }
    }

    /// Sends `data` from ICE local candidate `local` to `to` (through TURN for relayed ones).
    fn send_from(&mut self, local: usize, to: SocketAddr, data: &[u8]) {
        let Some(&sock) = self.candidate_socket.get(local) else {
            return;
        };
        self.send_socket(sock, to, data);
    }

    fn send_socket(&mut self, sock: usize, to: SocketAddr, data: &[u8]) {
        match self.sockets.get_mut(sock) {
            Some(Socket::Host(s)) => {
                let _ = s.send_to(data, to);
            }
            Some(Socket::Relay {
                socket,
                server,
                client,
                active,
                ..
            }) => {
                let wire = if *active == Some(to) {
                    client.send_raw(data)
                } else {
                    client.send(to, data)
                };
                if let Some(w) = wire {
                    let _ = socket.send_to(&w, *server);
                }
            }
            None => {}
        }
    }

    fn rtp_path(&self) -> Option<(usize, SocketAddr)> {
        self.selected.map(|(rtp, _)| rtp).or(self.usable)
    }

    fn rtcp_path(&self) -> Option<(usize, SocketAddr)> {
        self.selected
            .and_then(|(_, rtcp)| rtcp)
            .or_else(|| self.rtp_path())
    }

    /// One captured 20 ms frame.
    pub fn send_frame(&mut self, pcm: &[i16], now: u64) {
        let Some(packet) = self.audio.as_mut().and_then(|a| a.on_capture(pcm)) else {
            return;
        };
        if let Some((rtp, _)) = self.direct {
            self.send_socket(0, rtp, &packet);
        } else if let Some((l, r)) = self.rtp_path() {
            self.send_from(l, r, &packet);
            if let Some(a) = self.ice.as_mut() {
                a.on_media_sent(now);
            }
        }
    }

    /// One frame for the speaker.
    pub fn playout(&mut self) -> Vec<i16> {
        self.audio.as_mut().map(Audio::playout).unwrap_or_default()
    }

    pub fn frame_samples(&self) -> usize {
        self.audio.as_ref().map_or(320, Audio::frame_samples)
    }

    pub fn sample_rate(&self) -> u32 {
        self.audio.as_ref().map_or(16000, Audio::sample_rate)
    }

    pub fn send_dtmf(&mut self, digit: char) -> bool {
        self.audio.as_mut().is_some_and(|a| a.send_dtmf(digit))
    }

    pub fn set_muted(&mut self, m: bool) {
        if let Some(a) = self.audio.as_mut() {
            a.set_muted(m);
        }
    }

    /// RTCP BYE and TURN teardown.
    pub fn stop(&mut self, now: u64) {
        if let Some(bye) = self.audio.as_mut().and_then(Audio::bye) {
            if let Some((_, rtcp)) = self.direct {
                self.send_socket(1, rtcp, &bye);
            } else if let Some((l, r)) = self.rtcp_path() {
                self.send_from(l, r, &bye);
            }
        }
        for s in self.sockets.iter_mut() {
            if let Socket::Relay {
                socket,
                server,
                client,
                ..
            } = s
            {
                if let Some(w) = client.close(now) {
                    let _ = socket.send_to(&w, *server);
                }
            }
        }
    }

    /// Receives, runs TURN/ICE timers, RTCP. `now`: Unix time in milliseconds.
    pub fn poll(&mut self, now: u64) -> Vec<Event> {
        let mut events = Vec::new();
        let mut buf = [0u8; MAX_DATAGRAM];
        // Collect datagrams first: (socket index, from, data).
        let mut received: Vec<(usize, SocketAddr, Vec<u8>)> = Vec::new();
        for (i, s) in self.sockets.iter().enumerate() {
            let socket = match s {
                Socket::Host(s) => s,
                Socket::Relay { socket, .. } => socket,
            };
            for _ in 0..64 {
                match socket.recv_from(&mut buf) {
                    Ok((n, from)) => received.push((i, from, buf.get(..n).unwrap_or(&[]).to_vec())),
                    Err(_) => break,
                }
            }
        }
        for (i, from, data) in received {
            self.on_datagram(i, from, &data, now, &mut events);
        }
        // TURN timers and allocation results.
        let mut turn_out: Vec<(usize, Vec<u8>)> = Vec::new();
        for (i, s) in self.sockets.iter_mut().enumerate() {
            if let Socket::Relay { client, failed, .. } = s {
                let (w, e) = client.on_tick(now);
                turn_out.extend(w.into_iter().map(|w| (i, w)));
                if e.iter().any(|e| matches!(e, turn::Event::Failed(_))) {
                    *failed = true;
                }
            }
        }
        for (i, w) in turn_out {
            if let Some(Socket::Relay { socket, server, .. }) = self.sockets.get(i) {
                let _ = socket.send_to(&w, *server);
            }
        }
        // ICE.
        if let Some(agent) = self.ice.as_mut() {
            let (tx, ev) = agent.on_tick(now);
            for t in tx {
                self.send_from(t.local, t.to, &t.data);
            }
            self.on_ice_events(ev, now, &mut events);
        }
        // RTCP reports.
        // NTP timestamp (RFC 3550 §4): seconds since 1900 ‖ fraction; `now` is Unix time in ms.
        let ntp = ((now / 1000 + 2_208_988_800) << 32) | (((now % 1000) << 32) / 1000);
        if let Some(p) = self.audio.as_mut().and_then(|a| a.rtcp_due(now, ntp)) {
            if let Some((_, rtcp)) = self.direct {
                self.send_socket(1, rtcp, &p);
            } else if let Some((l, r)) = self.rtcp_path() {
                self.send_from(l, r, &p);
            }
        }
        events
    }

    fn on_ice_events(&mut self, ev: Vec<ice::Event>, now: u64, events: &mut Vec<Event>) {
        for e in ev {
            match e {
                ice::Event::Usable { local, remote } => self.usable = Some((local, remote)),
                ice::Event::Selected { rtp, rtcp } => {
                    self.selected = Some((rtp, rtcp));
                    // TU-25: a relayed path becomes the active destination.
                    for (l, r) in std::iter::once(rtp).chain(rtcp) {
                        if let Some(&sock) = self.candidate_socket.get(l) {
                            if let Some(Socket::Relay {
                                socket,
                                server,
                                client,
                                ..
                            }) = self.sockets.get_mut(sock)
                            {
                                if let Some(w) = client.set_active(r, now) {
                                    let _ = socket.send_to(&w, *server);
                                }
                            }
                        }
                    }
                    events.push(Event::Selected);
                }
                ice::Event::Failed(why) => events.push(Event::Failed(why.into())),
            }
        }
    }

    fn on_datagram(
        &mut self,
        sock: usize,
        from: SocketAddr,
        data: &[u8],
        now: u64,
        events: &mut Vec<Event>,
    ) {
        // Relay socket: only the TURN server may talk to it.
        let (payload, peer) = match self.sockets.get_mut(sock) {
            Some(Socket::Relay {
                server,
                client,
                allocated,
                active,
                failed,
                ..
            }) => {
                if from != *server {
                    return;
                }
                let (wires, evs) = client.on_packet(data, now);
                let mut got = None;
                for e in evs {
                    match e {
                        turn::Event::Allocated { relayed, .. } => *allocated = Some(relayed),
                        turn::Event::Data {
                            from: Some(f),
                            data,
                        } => got = Some((data, f)),
                        turn::Event::ActiveDestination(a) => *active = Some(a),
                        turn::Event::Failed(_) => *failed = true,
                        _ => {}
                    }
                }
                let server = *server;
                if let Some(Socket::Relay { socket, .. }) = self.sockets.get(sock) {
                    for w in wires {
                        let _ = socket.send_to(&w, server);
                    }
                }
                match got {
                    Some(x) => x,
                    None => return,
                }
            }
            Some(Socket::Host(_)) => (data.to_vec(), from),
            None => return,
        };
        let local = self.candidate_socket.iter().position(|&s| s == sock);
        match media::classify(&payload) {
            Kind::Stun => {
                let (Some(local), Some(agent)) = (local, self.ice.as_mut()) else {
                    return;
                };
                let (tx, ev) = agent.on_stun(local, peer, &payload, now);
                for t in tx {
                    self.send_from(t.local, t.to, &t.data);
                }
                self.on_ice_events(ev, now, events);
            }
            Kind::Rtp | Kind::Rtcp => {
                // Only from the peer: a candidate path or the direct address (IC-09).
                let known = self.direct.is_some_and(|(r, c)| peer == r || peer == c)
                    || self
                        .selected
                        .is_some_and(|((_, r), c)| peer == r || c.is_some_and(|(_, c)| peer == c))
                    || self.usable.is_some_and(|(_, r)| peer == r);
                if !known {
                    return;
                }
                let Some(audio) = self.audio.as_mut() else {
                    return;
                };
                if media::classify(&payload) == Kind::Rtcp {
                    if audio.on_rtcp(&payload) {
                        events.push(Event::PeerBye);
                    }
                } else if let Some(d) = audio.on_rtp(&payload, (now as u32).wrapping_mul(8)) {
                    events.push(Event::Dtmf(d));
                }
            }
            Kind::Other => {}
        }
    }
}

#[cfg(test)]
mod tests;
