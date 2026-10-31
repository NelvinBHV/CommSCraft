//! The network side of a call's media (L2 adapter): UDP sockets for the host candidates, MS-TURN
//! allocations on the A/V Edge over UDP and over TCP (port 443, pseudo-TLS), the ICE agent, and
//! the audio media core. The caller drives it every few milliseconds with [`CallMedia::poll`],
//! feeds captured frames and takes frames for the speaker; devices (ALSA) stay outside.
//!
//! Evidence: OCS-UC12 ([MS-OCSPROT] §2.5.12, pp. 38–41); IC-07, IC-09 (peer without ICE: media to
//! the default address), IC-10/IC-11 and [MS-ICE2] §3.1.4.8.1.3 (regular mode: TCP relayed
//! candidates advertised as active and passive, RTP and RTCP on the same address and port);
//! TU-02/TU-04 (pseudo-TLS, TCP framing), TU-24/TU-25 (Send until the active destination is set);
//! draft-rosenberg-midcom-turn-08 §8.6–8.10 (TCP relay: Send opens the connection to the peer,
//! incoming connections from permitted peers); draft-ietf-mmusic-ice-tcp-07 §4.2/§5 and RFC 4571
//! (STUN and RTP over a TCP stream with a 16-bit length prefix); AV-04 (relay ports); SD-45 (the
//! default candidate is never TCP).
//!
//! Security by design: all sockets are non-blocking and every receive is bounded (1500 octets per
//! datagram or frame, write buffer 64 KiB); relay sockets accept only the TURN server, the TCP
//! relay only a valid pseudo-TLS answer and well-formed frames (otherwise it is dropped);
//! RTP/RTCP only from known peer addresses; keys and ICE passwords come from the OS CSPRNG (fail
//! closed).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, UdpSocket};
use std::time::Duration;

use commscraft_ice_agent::{self as ice, Agent};
use commscraft_media::{self as media, Audio, Kind};
use commscraft_sdp as sdp;
use commscraft_turn as turn;

const MAX_DATAGRAM: usize = 1500;
const MAX_WRITE_BUFFER: usize = 64 * 1024;
const TCP_CONNECT_MS: u64 = 3000;

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

/// A relay to use, with credentials from MS-AVEDGEA (`udpPort` and `tcpPort`, AV-04).
pub struct Relay {
    pub server: SocketAddr,
    /// The relay's TCP address (port 443), for networks that block UDP.
    pub tcp_server: Option<SocketAddr>,
    pub username: String,
    pub password: zeroize::Zeroizing<Vec<u8>>,
}

/// RFC 4571 framing: 16-bit length, then the packet.
pub fn frame_4571(p: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(p.len() + 2);
    v.extend_from_slice(&(p.len().min(0xffff) as u16).to_be_bytes());
    v.extend_from_slice(p.get(..p.len().min(0xffff)).unwrap_or(&[]));
    v
}

/// RFC 4571 stream reassembly, bounded to [`MAX_DATAGRAM`] per packet.
#[derive(Debug, Default)]
pub struct Deframer4571 {
    buf: Vec<u8>,
}

/// A frame longer than 1500 octets on an RFC 4571 stream: the stream is dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OversizedFrame;

impl Deframer4571 {
    /// Complete packets of the stream so far.
    pub fn push(&mut self, data: &[u8]) -> Result<Vec<Vec<u8>>, OversizedFrame> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        while let Some(&[a, b]) = self.buf.get(..2).and_then(|s| <&[u8; 2]>::try_from(s).ok()) {
            let len = usize::from(u16::from_be_bytes([a, b]));
            if len > MAX_DATAGRAM {
                return Err(OversizedFrame);
            }
            let Some(p) = self.buf.get(2..2 + len).map(<[u8]>::to_vec) else {
                break;
            };
            self.buf.drain(..2 + len);
            out.push(p);
        }
        Ok(out)
    }
}

struct TcpRelay {
    stream: TcpStream,
    client: Box<turn::Client>,
    frames: turn::Deframer,
    out: Vec<u8>,
    allocated: Option<SocketAddr>,
    active: Option<SocketAddr>,
    failed: bool,
    peers: HashMap<SocketAddr, Deframer4571>,
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
    Tcp(Box<TcpRelay>),
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
    /// ICE local candidate index → (socket index, transport).
    candidate_socket: Vec<(usize, ice::Transport)>,
    ufrag: String,
    pwd: String,
    key: sdp::Key,
    ice: Option<Agent>,
    audio: Option<Audio>,
    /// Peer without ICE (IC-09): RTP/RTCP straight to the default addresses.
    direct: Option<(SocketAddr, SocketAddr)>,
    selected: Option<(Path, Option<Path>)>,
    usable: Option<Path>,
    /// Remote TCP-passive candidates (to know our role on a TCP relay connection).
    remote_passive: Vec<SocketAddr>,
    /// We put the call on hold (MS-SDPEXT §3.1.5.23).
    local_hold: bool,
    /// The peer's direction from the last offer or answer (hold, music on hold).
    remote_direction: sdp::Direction,
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

fn turn_client(r: &Relay, transport: turn::Transport) -> turn::Client {
    turn::Client::new(
        turn::Credentials {
            username: r.username.clone(),
            password: r.password.clone(),
        },
        transport,
        turn::Stream::Audio,
        Box::new(os_random),
    )
}

/// TCP connection to the relay with the pseudo-TLS handshake (TU-02); blocking with timeouts.
fn connect_tcp_relay(server: SocketAddr, now: u64) -> std::io::Result<TcpStream> {
    let timeout = Duration::from_millis(TCP_CONNECT_MS);
    let mut s = TcpStream::connect_timeout(&server, timeout)?;
    s.set_read_timeout(Some(timeout))?;
    s.set_write_timeout(Some(timeout))?;
    let mut random = [0u8; 28];
    os_random(&mut random);
    s.write_all(&turn::pseudo_tls_client_hello((now / 1000) as u32, random))?;
    let mut answer = [0u8; turn::PSEUDO_TLS_SERVER_HELLO_LEN];
    s.read_exact(&mut answer)?;
    if !turn::check_pseudo_tls_server_hello(&answer) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "pseudo-TLS answer",
        ));
    }
    s.set_nonblocking(true)?;
    s.set_nodelay(true)?;
    Ok(s)
}

impl CallMedia {
    /// Binds the host sockets (RTP, RTCP) and starts the TURN allocations: one per component over
    /// UDP, one over TCP (both components share it, §3.1.4.8.1.1).
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
                let mut client = turn_client(r, turn::Transport::Udp);
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
            // The TCP relay is optional: without it, calls still work over UDP.
            if let Some(tcp) = r.tcp_server {
                if let Ok(stream) = connect_tcp_relay(tcp, now) {
                    let mut client = turn_client(r, turn::Transport::Tcp);
                    let mut out = Vec::new();
                    if let Some(w) = client.start(now) {
                        out.extend_from_slice(&w);
                    }
                    sockets.push(Socket::Tcp(Box::new(TcpRelay {
                        stream,
                        client: Box::new(client),
                        frames: turn::Deframer::default(),
                        out,
                        allocated: None,
                        active: None,
                        failed: false,
                        peers: HashMap::new(),
                    })));
                }
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
            remote_passive: Vec::new(),
            local_hold: false,
            remote_direction: sdp::Direction::SendRecv,
            local_ip,
            started: now,
        };
        Ok((me, first))
    }

    /// Sends the first UDP TURN Allocates returned by [`prepare`] (the TCP one is queued).
    pub fn send_initial(&mut self, first: InitialAllocates) {
        for (i, w) in first {
            if let Some(Socket::Relay { socket, server, .. }) = self.sockets.get(i) {
                let _ = socket.send_to(&w, *server);
            }
        }
        self.flush_tcp();
    }

    /// True when every relay allocation succeeded or failed, or the gathering time is over.
    pub fn gathered(&self, now: u64) -> bool {
        now >= self.started + GATHER_MS
            || self.sockets.iter().all(|s| match s {
                Socket::Host(_) => true,
                Socket::Relay {
                    allocated, failed, ..
                } => allocated.is_some() || *failed,
                Socket::Tcp(t) => t.allocated.is_some() || t.failed,
            })
    }

    fn host_address(&self, i: usize) -> Option<SocketAddr> {
        match self.sockets.get(i)? {
            Socket::Host(s) => s.local_addr().ok(),
            _ => None,
        }
    }

    /// The local candidates in ICE order (host RTP/RTCP, UDP relay RTP/RTCP, TCP relay active
    /// RTP/RTCP and passive RTP/RTCP on one address).
    fn candidates(&self) -> Vec<(usize, ice::Local)> {
        let mut out = Vec::new();
        let local = |foundation: &str, component: u8, transport, kind, a: SocketAddr, priority| {
            ice::Local {
                foundation: foundation.into(),
                component,
                transport,
                kind,
                address: a,
                base: a,
                priority,
            }
        };
        for (i, s) in self.sockets.iter().enumerate() {
            let component = if i % 2 == 0 { 1 } else { 2 };
            match s {
                Socket::Host(_) => {
                    if let Some(a) = self.host_address(i) {
                        let p = commscraft_ice::candidate_priority(126, 0xffff, component);
                        out.push((
                            i,
                            local("1", component, ice::Transport::Udp, ice::Kind::Host, a, p),
                        ));
                    }
                }
                Socket::Relay {
                    allocated: Some(a), ..
                } => {
                    // IE-05: relay UDP: type 0, local preference 15·2^12 + 7·2^9 (= 0xfe00).
                    let p = commscraft_ice::candidate_priority(0, 0xfe00, component);
                    out.push((
                        i,
                        local(
                            "3",
                            component,
                            ice::Transport::Udp,
                            ice::Kind::Relayed,
                            *a,
                            p,
                        ),
                    ));
                }
                Socket::Tcp(t) => {
                    if let Some(a) = t.allocated {
                        // IE-04/IE-05: TCP (6), active (5) / passive (2), type relay (0).
                        for (f, tr, dir) in [
                            ("4", ice::Transport::TcpActive, 5),
                            ("5", ice::Transport::TcpPassive, 2),
                        ] {
                            for c in [1u8, 2] {
                                let lp = commscraft_ice::tcp_local_preference(6, dir, 0);
                                let p = commscraft_ice::candidate_priority(0, lp, c);
                                out.push((i, local(f, c, tr, ice::Kind::Relayed, a, p)));
                            }
                        }
                    }
                }
                Socket::Relay { .. } => {}
            }
        }
        out
    }

    /// Our SDP parameters: ICE credentials and candidates, key, default address (the UDP relay
    /// when allocated – it works from everywhere – otherwise the host; never TCP, SD-45).
    pub fn local_description(&self) -> sdp::Local {
        let cands = self.candidates();
        let to_sdp = |(_, c): &(usize, ice::Local)| sdp::Candidate {
            foundation: c.foundation.clone(),
            component: c.component,
            transport: match c.transport {
                ice::Transport::Udp => sdp::Transport::Udp,
                ice::Transport::TcpActive => sdp::Transport::TcpActive,
                ice::Transport::TcpPassive => sdp::Transport::TcpPassive,
            },
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
        let udp =
            |c: &ice::Local, comp: u8| c.transport == ice::Transport::Udp && c.component == comp;
        let default = |comp: u8| {
            cands
                .iter()
                .find(|(_, c)| c.kind == ice::Kind::Relayed && udp(c, comp))
                .or_else(|| cands.iter().find(|(_, c)| udp(c, comp)))
                .map(|(_, c)| c.address)
        };
        let rtp = default(1).unwrap_or_else(|| SocketAddr::new(self.local_ip, 9));
        sdp::Local {
            address: rtp.ip(),
            rtp_port: rtp.port(),
            rtcp_port: default(2).map_or(rtp.port().saturating_add(1), |a| a.port()),
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
        let keep: Vec<(SocketAddr, ice::Transport)> = std::iter::once(rtp_l)
            .chain(rtcp.map(|(l, _)| l))
            .filter_map(|l| cands.get(l).map(|(_, c)| (c.address, c.transport)))
            .collect();
        let tr = |t: sdp::Transport| match t {
            sdp::Transport::Udp => ice::Transport::Udp,
            sdp::Transport::TcpActive => ice::Transport::TcpActive,
            sdp::Transport::TcpPassive => ice::Transport::TcpPassive,
        };
        if let Some(ice) = d.ice.as_mut() {
            ice.candidates
                .retain(|c| keep.contains(&(SocketAddr::new(c.address, c.port), tr(c.transport))));
        }
        if let Some((a, _)) = keep.first() {
            d.address = a.ip();
            d.rtp_port = a.port();
        }
        if let Some((a, _)) = keep.get(1) {
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
        self.remote_direction = n.direction;
        self.apply_direction();
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
                    let tr = c.transport;
                    if agent.add_local(c).is_some() {
                        self.candidate_socket.push((sock, tr));
                    }
                }
                let remotes: Vec<ice::Remote> = remote
                    .candidates
                    .iter()
                    .map(|c| ice::Remote {
                        foundation: c.foundation.clone(),
                        component: c.component,
                        transport: match c.transport {
                            sdp::Transport::Udp => ice::Transport::Udp,
                            sdp::Transport::TcpActive => ice::Transport::TcpActive,
                            sdp::Transport::TcpPassive => ice::Transport::TcpPassive,
                        },
                        address: SocketAddr::new(c.address, c.port),
                        priority: c.priority,
                    })
                    .collect();
                self.remote_passive = remotes
                    .iter()
                    .filter(|r| r.transport == ice::Transport::TcpPassive)
                    .map(|r| r.address)
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
        self.remote_direction = n.direction;
        self.apply_direction();
    }

    /// Our hold (`true`) or resume: RTP stops at once; the re-INVITE is the engine's job.
    pub fn set_hold(&mut self, hold: bool) {
        self.local_hold = hold;
        self.apply_direction();
    }

    /// RTP is sent only if we are not holding and the peer receives (RFC 3264 §6.1: its
    /// `sendrecv` or `recvonly`); RTCP and ICE consent continue in every state.
    fn apply_direction(&mut self) {
        let peer_receives = matches!(
            self.remote_direction,
            sdp::Direction::SendRecv | sdp::Direction::RecvOnly
        );
        let sending = !self.local_hold && peer_receives;
        if let Some(a) = self.audio.as_mut() {
            a.set_sending(sending);
        }
    }

    /// True while RTP is being sent (not on hold on either side).
    pub fn sending(&self) -> bool {
        !self.local_hold
            && matches!(
                self.remote_direction,
                sdp::Direction::SendRecv | sdp::Direction::RecvOnly
            )
    }

    /// Sends `data` from ICE local candidate `local` to `to`.
    fn send_from(&mut self, local: usize, to: SocketAddr, data: &[u8]) {
        let Some(&(sock, _)) = self.candidate_socket.get(local) else {
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
            Some(Socket::Tcp(t)) => {
                // The peer connection is a byte stream: RFC 4571 framing inside TURN.
                let framed = frame_4571(data);
                let wire = if t.active == Some(to) {
                    t.client.send_raw(&framed)
                } else {
                    t.client.send(to, &framed)
                };
                if let Some(w) = wire {
                    if t.out.len() + w.len() <= MAX_WRITE_BUFFER {
                        t.out.extend_from_slice(&w);
                    }
                }
            }
            None => {}
        }
        self.flush_tcp();
    }

    fn flush_tcp(&mut self) {
        for s in self.sockets.iter_mut() {
            if let Socket::Tcp(t) = s {
                while !t.out.is_empty() && !t.failed {
                    match t.stream.write(&t.out) {
                        Ok(0) => {
                            t.failed = true;
                        }
                        Ok(n) => {
                            t.out.drain(..n.min(t.out.len()));
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(_) => t.failed = true,
                    }
                }
            }
        }
    }

    fn rtp_path(&self) -> Option<Path> {
        self.selected.map(|(rtp, _)| rtp).or(self.usable)
    }

    fn rtcp_path(&self) -> Option<Path> {
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
            match s {
                Socket::Relay {
                    socket,
                    server,
                    client,
                    ..
                } => {
                    if let Some(w) = client.close(now) {
                        let _ = socket.send_to(&w, *server);
                    }
                }
                Socket::Tcp(t) => {
                    if let Some(w) = t.client.close(now) {
                        t.out.extend_from_slice(&w);
                    }
                }
                Socket::Host(_) => {}
            }
        }
        self.flush_tcp();
    }

    /// Receives, runs TURN/ICE timers, RTCP. `now`: Unix time in milliseconds.
    pub fn poll(&mut self, now: u64) -> Vec<Event> {
        let mut events = Vec::new();
        // (socket index, peer, packet) of everything that arrived.
        let mut packets: Vec<(usize, SocketAddr, Vec<u8>)> = Vec::new();
        let mut buf = [0u8; MAX_DATAGRAM];
        let mut udp: Vec<(usize, SocketAddr, Vec<u8>)> = Vec::new();
        for (i, s) in self.sockets.iter_mut().enumerate() {
            match s {
                Socket::Host(sock) | Socket::Relay { socket: sock, .. } => {
                    for _ in 0..64 {
                        match sock.recv_from(&mut buf) {
                            Ok((n, from)) => {
                                udp.push((i, from, buf.get(..n).unwrap_or(&[]).to_vec()))
                            }
                            Err(_) => break,
                        }
                    }
                }
                Socket::Tcp(t) => {
                    let mut chunk = [0u8; 4096];
                    for _ in 0..16 {
                        match t.stream.read(&mut chunk) {
                            Ok(0) => {
                                t.failed = true;
                                break;
                            }
                            Ok(n) => match t.frames.push(chunk.get(..n).unwrap_or(&[])) {
                                Ok(frames) => {
                                    for (kind, payload) in frames {
                                        Self::on_tcp_frame(t, i, kind, &payload, now, &mut packets);
                                    }
                                }
                                Err(_) => {
                                    // TU-28: a framing violation ends the TCP relay.
                                    t.failed = true;
                                    break;
                                }
                            },
                            Err(_) => break,
                        }
                    }
                }
            }
        }
        for (i, from, data) in udp {
            if let Some(p) = self.on_udp(i, from, &data, now) {
                packets.push(p);
            }
        }
        for (i, peer, p) in packets {
            self.dispatch(i, peer, &p, now, &mut events);
        }
        // TURN timers.
        for s in self.sockets.iter_mut() {
            match s {
                Socket::Relay {
                    socket,
                    server,
                    client,
                    failed,
                    ..
                } => {
                    let (w, e) = client.on_tick(now);
                    for x in w {
                        let _ = socket.send_to(&x, *server);
                    }
                    if e.iter().any(|e| matches!(e, turn::Event::Failed(_))) {
                        *failed = true;
                    }
                }
                Socket::Tcp(t) => {
                    let (w, e) = t.client.on_tick(now);
                    for x in w {
                        t.out.extend_from_slice(&x);
                    }
                    if e.iter().any(|e| matches!(e, turn::Event::Failed(_))) {
                        t.failed = true;
                    }
                }
                Socket::Host(_) => {}
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
        // RTCP reports; NTP timestamp (RFC 3550 §4) from Unix time in ms.
        let ntp = ((now / 1000 + 2_208_988_800) << 32) | (((now % 1000) << 32) / 1000);
        // Probe, report and train packets back to back ([MS-RTP] §3.2.6).
        let due = self
            .audio
            .as_mut()
            .map(|a| a.rtcp_due(now, ntp))
            .unwrap_or_default();
        for p in due {
            if let Some((_, rtcp)) = self.direct {
                self.send_socket(1, rtcp, &p);
            } else if let Some((l, r)) = self.rtcp_path() {
                self.send_from(l, r, &p);
            }
        }
        self.flush_tcp();
        events
    }

    /// One MS-TURN TCP frame: control (0x02) goes to the TURN client, data (0x03) is the stream
    /// of the active destination; peer streams are reassembled per RFC 4571.
    fn on_tcp_frame(
        t: &mut TcpRelay,
        sock: usize,
        kind: u8,
        payload: &[u8],
        now: u64,
        packets: &mut Vec<(usize, SocketAddr, Vec<u8>)>,
    ) {
        let (peer, bytes) = if kind == 2 {
            let (wires, evs) = t.client.on_packet(payload, now);
            for w in wires {
                t.out.extend_from_slice(&w);
            }
            let mut got = None;
            for e in evs {
                match e {
                    turn::Event::Allocated { relayed, .. } => t.allocated = Some(relayed),
                    turn::Event::Data {
                        from: Some(f),
                        data,
                    } => got = Some((f, data)),
                    turn::Event::ActiveDestination(a) => t.active = Some(a),
                    turn::Event::Failed(_) | turn::Event::Malformed => t.failed = true,
                    _ => {}
                }
            }
            match got {
                Some(x) => x,
                None => return,
            }
        } else {
            match t.active {
                Some(a) => (a, payload.to_vec()),
                None => return,
            }
        };
        if !t.peers.contains_key(&peer) && t.peers.len() >= 16 {
            return;
        }
        let d = t.peers.entry(peer).or_default();
        match d.push(&bytes) {
            Ok(ps) => packets.extend(ps.into_iter().map(|p| (sock, peer, p))),
            Err(OversizedFrame) => {
                t.peers.remove(&peer);
            }
        }
    }

    fn on_ice_events(&mut self, ev: Vec<ice::Event>, now: u64, events: &mut Vec<Event>) {
        for e in ev {
            match e {
                ice::Event::Usable { local, remote } => self.usable = Some((local, remote)),
                ice::Event::Selected { rtp, rtcp } => {
                    self.selected = Some((rtp, rtcp));
                    // [MS-RTP] §3.2: the TURN server's bandwidth of the RTP path goes to the peer.
                    let kbps = self.candidate_socket.get(rtp.0).and_then(|&(sock, _)| {
                        match self.sockets.get(sock) {
                            Some(Socket::Relay { client, .. }) => client.bandwidth(),
                            Some(Socket::Tcp(t)) => t.client.bandwidth(),
                            _ => None,
                        }
                    });
                    if let (Some(k), Some(a)) = (kbps, self.audio.as_mut()) {
                        a.set_turn_bandwidth(k.saturating_mul(1000));
                    }
                    // TU-25: a relayed path becomes the active destination.
                    for (l, r) in std::iter::once(rtp).chain(rtcp) {
                        let Some(&(sock, _)) = self.candidate_socket.get(l) else {
                            continue;
                        };
                        match self.sockets.get_mut(sock) {
                            Some(Socket::Relay {
                                socket,
                                server,
                                client,
                                ..
                            }) => {
                                if let Some(w) = client.set_active(r, now) {
                                    let _ = socket.send_to(&w, *server);
                                }
                            }
                            Some(Socket::Tcp(t)) if t.active.is_none() => {
                                if let Some(w) = t.client.set_active(r, now) {
                                    t.out.extend_from_slice(&w);
                                }
                            }
                            _ => {}
                        }
                    }
                    events.push(Event::Selected);
                }
                ice::Event::Failed(why) => events.push(Event::Failed(why.into())),
            }
        }
    }

    /// A UDP datagram: host sockets pass it on; relay sockets only accept the TURN server.
    fn on_udp(
        &mut self,
        sock: usize,
        from: SocketAddr,
        data: &[u8],
        now: u64,
    ) -> Option<(usize, SocketAddr, Vec<u8>)> {
        match self.sockets.get_mut(sock)? {
            Socket::Relay {
                socket,
                server,
                client,
                allocated,
                active,
                failed,
            } => {
                if from != *server {
                    return None;
                }
                let (wires, evs) = client.on_packet(data, now);
                for w in wires {
                    let _ = socket.send_to(&w, *server);
                }
                let mut got = None;
                for e in evs {
                    match e {
                        turn::Event::Allocated { relayed, .. } => *allocated = Some(relayed),
                        turn::Event::Data {
                            from: Some(f),
                            data,
                        } => got = Some((sock, f, data)),
                        turn::Event::ActiveDestination(a) => *active = Some(a),
                        turn::Event::Failed(_) => *failed = true,
                        _ => {}
                    }
                }
                got
            }
            Socket::Host(_) => Some((sock, from, data.to_vec())),
            Socket::Tcp(_) => None,
        }
    }

    /// The ICE local candidate a packet from `peer` on socket `sock` belongs to. On the TCP relay
    /// our role follows the peer's: towards a passive peer candidate we are active.
    fn local_for(&self, sock: usize, peer: SocketAddr, component: u8) -> Option<usize> {
        let want = match self.sockets.get(sock)? {
            Socket::Tcp(_) if self.remote_passive.contains(&peer) => ice::Transport::TcpActive,
            Socket::Tcp(_) => ice::Transport::TcpPassive,
            _ => ice::Transport::Udp,
        };
        let candidates = self.candidates();
        self.candidate_socket
            .iter()
            .enumerate()
            .position(|(i, &(s, t))| {
                s == sock
                    && t == want
                    && (want == ice::Transport::Udp
                        || candidates
                            .get(i)
                            .is_some_and(|(_, c)| c.component == component))
            })
    }

    fn dispatch(
        &mut self,
        sock: usize,
        peer: SocketAddr,
        payload: &[u8],
        now: u64,
        events: &mut Vec<Event>,
    ) {
        match media::classify(payload) {
            Kind::Stun => {
                // On a TCP connection both components share the stream: try RTP's candidate
                // first, then RTCP's for responses (they are matched by transaction ID).
                let is_request = payload.get(..2) == Some(&[0x00, 0x01]);
                let tries: Vec<usize> = [1u8, 2]
                    .iter()
                    .filter_map(|&c| self.local_for(sock, peer, c))
                    .collect();
                for (n, local) in tries.into_iter().enumerate() {
                    if n > 0 && is_request {
                        break;
                    }
                    let Some(agent) = self.ice.as_mut() else {
                        return;
                    };
                    let (tx, ev) = agent.on_stun(local, peer, payload, now);
                    let effect = !tx.is_empty() || !ev.is_empty();
                    for t in tx {
                        self.send_from(t.local, t.to, &t.data);
                    }
                    self.on_ice_events(ev, now, events);
                    if effect {
                        break;
                    }
                }
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
                if media::classify(payload) == Kind::Rtcp {
                    if audio.on_rtcp(payload, now) {
                        events.push(Event::PeerBye);
                    }
                } else if let Some(d) = audio.on_rtp(payload, (now as u32).wrapping_mul(8)) {
                    events.push(Event::Dtmf(d));
                }
            }
            Kind::Other => {}
        }
    }
}

#[cfg(test)]
mod tests;
