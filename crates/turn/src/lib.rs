//! MS-TURN client (sans-I/O): media relay allocation on the A/V Edge, TURN-encapsulated data,
//! active destination, refresh and teardown; TCP framing and the pseudo-TLS handshake.
//!
//! Evidence: `docs/protocol/evidence/12-turn-relay.md` (TU-01 … TU-29). Sources: [MS-TURN]
//! 20250520 §2.1.1 (pseudo-TLS, pp. 16–19), §2.1.4 (TCP framing, p. 19), §2.2.2.3 (integrity,
//! pp. 23–25), §2.2.2.16–2.2.2.22 (pp. 30–36), §3.1.8–3.1.12 (pp. 38–39), §3.2 (client, pp.
//! 39–44); draft-rosenberg-midcom-turn-08 §8.3–8.10 (allocation, refresh at 3/4 of the lifetime,
//! Send, Data Indication, active destination, teardown).
//!
//! Security by design:
//! - every message must be well formed with the MS magic cookie first (§3.1.10), otherwise it is
//!   ignored (TCP: the caller closes the connection on [`Event::Malformed`]);
//! - responses must carry a valid MESSAGE-INTEGRITY (§3.1.11); only the digest challenge (401
//!   and the other retry codes of §3.2.5.2) may come unauthenticated, and at most three times;
//! - data indications are accepted only from peers we sent to (our own permission list), in
//!   addition to the server's permission check;
//! - HMAC-SHA256 is used when both sides announce version ≥ 3 (TU-14), never downgraded after;
//! - the password is held zeroised-on-drop; retransmissions are bounded (650 ms × 9, TU-27).

use std::net::SocketAddr;

use commscraft_stun::{self as stun, Integrity, Message};
use zeroize::Zeroizing;

pub const ALLOCATE: u16 = 0x0003;
pub const ALLOCATE_RESPONSE: u16 = 0x0103;
pub const ALLOCATE_ERROR: u16 = 0x0113;
pub const SEND: u16 = 0x0004;
pub const DATA_INDICATION: u16 = 0x0115;
pub const SET_ACTIVE_DESTINATION: u16 = 0x0006;
pub const SET_ACTIVE_DESTINATION_RESPONSE: u16 = 0x0106;
pub const SET_ACTIVE_DESTINATION_ERROR: u16 = 0x0116;

/// MS-Version we announce: MS-ICE2 + HMAC-SHA256 (TU-15). No IPv6 relay (4) and no
/// multiplexed TURN (5/6, TU-29: not used by SfB clients).
pub const OUR_VERSION: u32 = 3;
/// Retransmission (TU-27).
pub const RETRANSMIT_MS: u64 = 650;
pub const MAX_RETRANSMITS: u32 = 9;
const MAX_CHALLENGES: u32 = 3;
const MAX_PERMISSIONS: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Udp,
    Tcp,
}

/// MS-Service Quality stream type (§2.2.2.22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Audio = 1,
    Video = 2,
    Data = 4,
}

/// Long-term credentials from MS-AVEDGEA (TU-20).
pub struct Credentials {
    pub username: String,
    pub password: Zeroizing<Vec<u8>>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username_len", &self.username.len())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Relay (MAPPED-ADDRESS) and our public address as seen by the server (XOR-MAPPED), lifetime.
    Allocated {
        relayed: SocketAddr,
        reflexive: Option<SocketAddr>,
        lifetime: u32,
    },
    /// The server named another address for the next Allocate (TU-21, UDP).
    AlternateServer(SocketAddr),
    /// Data from a peer, TURN-encapsulated (Data Indication) or raw after Set Active Destination.
    Data {
        from: Option<SocketAddr>,
        data: Vec<u8>,
    },
    ActiveDestination(SocketAddr),
    ActiveDestinationFailed(u16),
    /// The allocation is gone or could not be made (error code, 0 = timeout).
    Failed(u16),
    /// A message that is not well-formed MS-TURN (TCP: close the connection, §3.1.10).
    Malformed,
    /// The authenticated answer to a bandwidth admission control request ([MS-TURNBWM] §3.1):
    /// all its attributes and the transaction ID (the site responses are XOR-coded with it).
    Bac {
        attributes: Vec<(u16, Vec<u8>)>,
        transaction: [u8; 16],
    },
    /// The bandwidth request was refused or not answered (error code, 0 = timeout); the
    /// allocation itself stays.
    BacFailed(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Allocate,
    Refresh,
    Teardown,
    SetActive(SocketAddr),
    Bac,
}

#[derive(Debug)]
struct Pending {
    kind: Kind,
    transaction: [u8; 16],
    wire: Vec<u8>,
    next: u64,
    retransmits: u32,
}

/// Source of randomness for transaction IDs (a CSPRNG in production).
pub type Random = Box<dyn FnMut(&mut [u8]) + Send>;

/// One allocation on one TURN server.
pub struct Client {
    /// Peak bandwidth of the allocation in kbit/s (BANDWIDTH in the Allocate response,
    /// draft-rosenberg-midcom-turn-08 §8.2/§9.2.4, [MS-TURN] §2.2.2.9).
    bandwidth: Option<u32>,
    creds: Credentials,
    transport: Transport,
    stream: Stream,
    random: Random,
    realm: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
    sha256: bool,
    challenges: u32,
    sequence: Option<([u8; 20], u32)>,
    pending: Option<Pending>,
    relayed: Option<SocketAddr>,
    lifetime: u32,
    refresh_at: Option<u64>,
    active: Option<SocketAddr>,
    permissions: Vec<SocketAddr>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("transport", &self.transport)
            .field("relayed", &self.relayed)
            .field("active", &self.active)
            .finish_non_exhaustive()
    }
}

/// What the caller sends to the server.
pub type Wire = Vec<u8>;

impl Client {
    pub fn new(creds: Credentials, transport: Transport, stream: Stream, random: Random) -> Self {
        Client {
            creds,
            transport,
            stream,
            random,
            realm: None,
            nonce: None,
            sha256: false,
            challenges: 0,
            sequence: None,
            pending: None,
            relayed: None,
            bandwidth: None,
            lifetime: 0,
            refresh_at: None,
            active: None,
            permissions: Vec::new(),
        }
    }

    /// The allocation's peak bandwidth in kbit/s, if the server named one.
    pub fn bandwidth(&self) -> Option<u32> {
        self.bandwidth
    }

    pub fn relayed(&self) -> Option<SocketAddr> {
        self.relayed
    }

    fn transaction(&mut self) -> [u8; 16] {
        let mut t = [0u8; 16];
        (self.random)(&mut t);
        // Must not look like an RFC 5389 message (magic cookie in octets 4–7).
        if t.get(..4) == Some(&stun::MAGIC_COOKIE.to_be_bytes()[..]) {
            if let Some(b) = t.first_mut() {
                *b ^= 0xff;
            }
        }
        t
    }

    fn frame(&self, kind: u8, payload: Vec<u8>) -> Wire {
        match self.transport {
            Transport::Udp => payload,
            Transport::Tcp => frame(kind, &payload),
        }
    }

    fn base(&mut self, kind: u16) -> Message {
        let t = self.transaction();
        let mut m = Message::new(kind, t);
        m.push(
            stun::MS_MAGIC_COOKIE,
            stun::MS_MAGIC_COOKIE_VALUE.to_be_bytes().to_vec(),
        );
        m.push(stun::MS_VERSION, OUR_VERSION.to_be_bytes().to_vec());
        m
    }

    fn key(&self) -> Option<Vec<u8>> {
        let realm = self.realm.as_ref()?;
        let user = self.creds.username.as_bytes();
        if self.sha256 {
            let nonce = self.nonce.as_ref()?;
            Some(stun::ms_turn_sha256_key(nonce, &self.creds.password, user, realm).to_vec())
        } else {
            Some(stun::long_term_key(user, realm, &self.creds.password).to_vec())
        }
    }

    fn integrity<'a>(&self, key: &'a [u8]) -> Integrity<'a> {
        if self.sha256 {
            Integrity::LegacySha256(key)
        } else {
            Integrity::LegacySha1(key)
        }
    }

    /// Adds Username/Realm/Nonce, the sequence number and MESSAGE-INTEGRITY (§3.1.11).
    fn authenticate(&mut self, mut m: Message) -> Option<Vec<u8>> {
        let (realm, nonce) = (self.realm.clone()?, self.nonce.clone()?);
        m.push(stun::USERNAME, self.creds.username.as_bytes().to_vec());
        m.push(stun::REALM, realm);
        m.push(stun::NONCE, nonce);
        if let Some((id, n)) = self.sequence.as_mut() {
            // Echo the connection ID, counter + 1 per request (§2.2.2.21).
            *n = n.wrapping_add(1);
            let mut v = id.to_vec();
            v.extend_from_slice(&n.to_be_bytes());
            m.push(stun::MS_SEQUENCE_NUMBER, v);
        }
        let key = self.key()?;
        stun::with_integrity(m.encode().ok()?, self.integrity(&key)).ok()
    }

    fn service_quality(&self) -> Vec<u8> {
        let mut v = (self.stream as u16).to_be_bytes().to_vec();
        v.extend_from_slice(&0u16.to_be_bytes()); // best effort
        v
    }

    fn send_request(
        &mut self,
        kind: Kind,
        transaction: [u8; 16],
        bytes: Vec<u8>,
        now: u64,
    ) -> Wire {
        let wire = self.frame(2, bytes);
        self.pending = Some(Pending {
            kind,
            transaction,
            wire: wire.clone(),
            next: now + RETRANSMIT_MS,
            retransmits: 0,
        });
        wire
    }

    /// The first Allocate, without integrity (§3.2.4.1). `now` in milliseconds.
    pub fn start(&mut self, now: u64) -> Option<Wire> {
        let mut m = self.base(ALLOCATE);
        m.push(stun::MS_SERVICE_QUALITY, self.service_quality());
        let t = m.transaction;
        let bytes = m.encode().ok()?;
        Some(self.send_request(Kind::Allocate, t, bytes, now))
    }

    fn allocate(&mut self, kind: Kind, now: u64) -> Option<Wire> {
        let mut m = self.base(ALLOCATE);
        m.push(stun::MS_SERVICE_QUALITY, self.service_quality());
        if kind == Kind::Teardown {
            m.push(stun::LIFETIME, 0u32.to_be_bytes().to_vec());
        }
        let t = m.transaction;
        let bytes = self.authenticate(m)?;
        Some(self.send_request(kind, t, bytes, now))
    }

    /// TURN-encapsulated data to a peer; also sets the permission (§3.2.4.2). No response.
    pub fn send(&mut self, peer: SocketAddr, data: &[u8]) -> Option<Wire> {
        self.relayed?;
        if !self.permissions.contains(&peer) {
            if self.permissions.len() >= MAX_PERMISSIONS {
                return None;
            }
            self.permissions.push(peer);
        }
        let mut m = self.base(SEND);
        m.push(stun::DESTINATION_ADDRESS, stun::encode_address(peer));
        m.push(stun::DATA, data.to_vec());
        let bytes = self.authenticate(m)?;
        Some(self.frame(2, bytes))
    }

    /// Raw data to the active destination (§3.2.4.5; TCP framing type 0x03).
    pub fn send_raw(&self, data: &[u8]) -> Option<Wire> {
        self.active?;
        Some(self.frame(3, data.to_vec()))
    }

    /// Set Active Destination (§3.2.4.3).
    pub fn set_active(&mut self, peer: SocketAddr, now: u64) -> Option<Wire> {
        self.relayed?;
        let mut m = self.base(SET_ACTIVE_DESTINATION);
        m.push(stun::DESTINATION_ADDRESS, stun::encode_address(peer));
        let t = m.transaction;
        let bytes = self.authenticate(m)?;
        Some(self.send_request(Kind::SetActive(peer), t, bytes, now))
    }

    /// A bandwidth admission control request ([MS-TURNBWM] §3.1, §3.2.4): an authenticated
    /// Allocate on the existing allocation with the attributes `build` returns for the new
    /// transaction ID (the site addresses are XOR-coded with it). `None` while another request
    /// is outstanding or before the allocation exists – the caller tries again later.
    pub fn bac(
        &mut self,
        build: impl FnOnce(&[u8; 16]) -> Vec<(u16, Vec<u8>)>,
        now: u64,
    ) -> Option<Wire> {
        self.relayed?;
        if self.pending.is_some() {
            return None;
        }
        let mut m = self.base(ALLOCATE);
        m.push(stun::MS_SERVICE_QUALITY, self.service_quality());
        let t = m.transaction;
        for (k, v) in build(&t).into_iter().take(16) {
            m.push(k, v);
        }
        let bytes = self.authenticate(m)?;
        Some(self.send_request(Kind::Bac, t, bytes, now))
    }

    /// Teardown: Allocate with Lifetime 0 (§3.2.4.4, TU-26).
    pub fn close(&mut self, now: u64) -> Option<Wire> {
        self.relayed?;
        self.refresh_at = None;
        self.allocate(Kind::Teardown, now)
    }

    /// Retransmissions and refresh (UDP only, at 3/4 of the lifetime – turn-08 §8.5).
    pub fn on_tick(&mut self, now: u64) -> (Vec<Wire>, Vec<Event>) {
        let mut wires = Vec::new();
        let mut events = Vec::new();
        if let Some(p) = self.pending.as_mut() {
            if now >= p.next {
                if p.retransmits >= MAX_RETRANSMITS {
                    let kind = p.kind;
                    self.pending = None;
                    events.push(match kind {
                        Kind::SetActive(_) => Event::ActiveDestinationFailed(0),
                        Kind::Bac => Event::BacFailed(0),
                        _ => Event::Failed(0),
                    });
                } else {
                    p.retransmits += 1;
                    p.next = now + RETRANSMIT_MS;
                    wires.push(p.wire.clone());
                }
            }
        }
        if self.pending.is_none() && self.refresh_at.is_some_and(|t| now >= t) {
            self.refresh_at = None;
            if let Some(w) = self.allocate(Kind::Refresh, now) {
                wires.push(w);
            }
        }
        (wires, events)
    }

    /// One packet from the server (UDP datagram or one TCP frame payload incl. its type).
    pub fn on_packet(&mut self, p: &[u8], now: u64) -> (Vec<Wire>, Vec<Event>) {
        let mut wires = Vec::new();
        let mut events = Vec::new();
        // Raw data from the active destination: not a TURN message (no MS magic cookie).
        let m = match parse(p) {
            Some(m) => m,
            None => {
                match self.active {
                    Some(peer) if !p.is_empty() => events.push(Event::Data {
                        from: Some(peer),
                        data: p.to_vec(),
                    }),
                    _ => events.push(Event::Malformed),
                }
                return (wires, events);
            }
        };
        if m.kind == DATA_INDICATION {
            let from = m
                .get(stun::REMOTE_ADDRESS)
                .and_then(|v| stun::decode_address(v).ok());
            match (from, m.get(stun::DATA)) {
                (Some(f), Some(d)) if self.permissions.contains(&f) => events.push(Event::Data {
                    from: Some(f),
                    data: d.to_vec(),
                }),
                _ => {}
            }
            return (wires, events);
        }
        let Some(pending) = self.pending.as_ref() else {
            return (wires, events);
        };
        if pending.transaction != m.transaction {
            return (wires, events);
        }
        let kind = pending.kind;
        let is_error = m.kind == ALLOCATE_ERROR || m.kind == SET_ACTIVE_DESTINATION_ERROR;
        let code = m
            .get(stun::ERROR_CODE)
            .and_then(|v| stun::decode_error(v).ok())
            .map(|(c, _)| c);
        // Unauthenticated digest challenge (§3.2.5.2).
        let challenge = is_error && matches!(code, Some(401 | 431 | 432 | 434 | 435 | 438));
        if challenge && kind == Kind::Bac {
            // New nonce/realm: the caller builds the request again (new transaction ID).
            if let (Some(r), Some(n)) = (m.get(stun::REALM), m.get(stun::NONCE)) {
                self.realm = Some(r.to_vec());
                self.nonce = Some(n.to_vec());
            }
            self.pending = None;
            events.push(Event::BacFailed(code.unwrap_or(0)));
            return (wires, events);
        }
        if challenge && matches!(kind, Kind::Allocate | Kind::Refresh) {
            if self.challenges >= MAX_CHALLENGES {
                self.pending = None;
                events.push(Event::Failed(code.unwrap_or(0)));
                return (wires, events);
            }
            self.challenges += 1;
            if let (Some(r), Some(n)) = (m.get(stun::REALM), m.get(stun::NONCE)) {
                self.realm = Some(r.to_vec());
                self.nonce = Some(n.to_vec());
            }
            let server = m
                .get(stun::MS_VERSION)
                .and_then(|v| <[u8; 4]>::try_from(v).ok())
                .map(u32::from_be_bytes)
                .unwrap_or(0);
            // TU-14: SHA-256 only if both ≥ 3; once chosen, never downgraded.
            self.sha256 = self.sha256 || (server >= 3 && OUR_VERSION >= 3);
            if let Some(alt) = m
                .get(stun::ALTERNATE_SERVER)
                .and_then(|v| stun::decode_address(v).ok())
            {
                if self.transport == Transport::Udp {
                    events.push(Event::AlternateServer(alt));
                }
            }
            self.pending = None;
            if let Some(w) = self.allocate(kind, now) {
                wires.push(w);
            } else {
                events.push(Event::Failed(code.unwrap_or(0)));
            }
            return (wires, events);
        }
        // Everything else must be authenticated (§3.1.11).
        let Some(key) = self.key() else {
            return (wires, events);
        };
        if stun::verify_integrity(p, self.integrity(&key)).is_err() {
            return (wires, events);
        }
        self.pending = None;
        match (kind, m.kind) {
            (Kind::Allocate | Kind::Refresh, ALLOCATE_RESPONSE) => {
                self.challenges = 0;
                let relayed = m
                    .get(stun::MAPPED_ADDRESS)
                    .and_then(|v| stun::decode_address(v).ok());
                let reflexive = m
                    .get(stun::MS_XOR_MAPPED_ADDRESS)
                    .and_then(|v| stun::decode_xor_address(v, &m.transaction).ok());
                let lifetime = m
                    .get(stun::LIFETIME)
                    .and_then(|v| <[u8; 4]>::try_from(v).ok())
                    .map(u32::from_be_bytes)
                    .unwrap_or(600)
                    .clamp(30, 86_400);
                if let Some(v) = m.get(stun::MS_SEQUENCE_NUMBER) {
                    if let (Some(id), Some(n)) = (v.get(..20), v.get(20..24)) {
                        if let (Ok(id), Ok(n)) = (<[u8; 20]>::try_from(id), <[u8; 4]>::try_from(n))
                        {
                            if self.sequence.is_none() {
                                self.sequence = Some((id, u32::from_be_bytes(n)));
                            }
                        }
                    }
                }
                if let Some(b) = m
                    .get(stun::BANDWIDTH)
                    .and_then(|v| <[u8; 4]>::try_from(v).ok())
                {
                    self.bandwidth = Some(u32::from_be_bytes(b));
                }
                match relayed {
                    Some(r) => {
                        self.relayed = Some(r);
                        self.lifetime = lifetime;
                        if self.transport == Transport::Udp {
                            self.refresh_at = Some(now + u64::from(lifetime) * 750);
                        }
                        if kind == Kind::Allocate {
                            events.push(Event::Allocated {
                                relayed: r,
                                reflexive,
                                lifetime,
                            });
                        }
                    }
                    None => events.push(Event::Failed(500)),
                }
            }
            (Kind::Bac, ALLOCATE_RESPONSE) => {
                // It refreshes the allocation as well (it is an Allocate on it).
                if let Some(l) = m
                    .get(stun::LIFETIME)
                    .and_then(|v| <[u8; 4]>::try_from(v).ok())
                    .map(u32::from_be_bytes)
                {
                    self.lifetime = l.clamp(30, 86_400);
                    if self.transport == Transport::Udp {
                        self.refresh_at = Some(now + u64::from(self.lifetime) * 750);
                    }
                }
                events.push(Event::Bac {
                    attributes: m
                        .attributes
                        .iter()
                        .map(|a| (a.kind, a.value.clone()))
                        .collect(),
                    transaction: m.transaction,
                });
            }
            (Kind::Bac, ALLOCATE_ERROR) => events.push(Event::BacFailed(code.unwrap_or(0))),
            (Kind::Teardown, ALLOCATE_RESPONSE) => {
                self.relayed = None;
                self.active = None;
            }
            (Kind::Allocate | Kind::Refresh | Kind::Teardown, ALLOCATE_ERROR) => {
                self.relayed = None;
                events.push(Event::Failed(code.unwrap_or(0)));
            }
            (Kind::SetActive(peer), SET_ACTIVE_DESTINATION_RESPONSE) => {
                self.active = Some(peer);
                events.push(Event::ActiveDestination(peer));
            }
            (Kind::SetActive(_), SET_ACTIVE_DESTINATION_ERROR) => {
                self.active = None;
                events.push(Event::ActiveDestinationFailed(code.unwrap_or(0)));
            }
            _ => {}
        }
        (wires, events)
    }
}

/// A well-formed MS-TURN message: legacy STUN with the MS magic cookie as the first attribute.
pub fn parse(p: &[u8]) -> Option<Message> {
    let m = Message::decode(p).ok()?;
    let first = m.attributes.first()?;
    (m.format() == stun::Format::Legacy
        && first.kind == stun::MS_MAGIC_COOKIE
        && first.value == stun::MS_MAGIC_COOKIE_VALUE.to_be_bytes())
    .then_some(m)
}

// ----- TCP (§2.1.1, §2.1.4) --------------------------------------------------------------------

/// TCP framing header: type (0x02 control, 0x03 data), 0, length (§2.1.4).
pub fn frame(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut v = vec![kind, 0];
    v.extend_from_slice(&(payload.len().min(0xffff) as u16).to_be_bytes());
    v.extend_from_slice(payload.get(..payload.len().min(0xffff)).unwrap_or(&[]));
    v
}

/// Splits a TCP byte stream into frames; bounded buffer.
#[derive(Debug, Default)]
pub struct Deframer {
    buf: Vec<u8>,
}

/// Largest frame accepted (a STUN message or a media packet, IC-02).
pub const MAX_FRAME: usize = 1500;

/// The TCP stream violates the framing (§2.1.4): the caller closes the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramingError;

impl Deframer {
    /// Feeds bytes; returns complete frames (type, payload).
    pub fn push(&mut self, data: &[u8]) -> Result<Vec<(u8, Vec<u8>)>, FramingError> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        while let Some(header) = self.buf.get(..4) {
            let [kind, zero, l0, l1] = <[u8; 4]>::try_from(header).map_err(|_| FramingError)?;
            if !matches!(kind, 2 | 3) || zero != 0 {
                return Err(FramingError);
            }
            let len = usize::from(u16::from_be_bytes([l0, l1]));
            if len > MAX_FRAME {
                return Err(FramingError);
            }
            let Some(payload) = self.buf.get(4..4 + len).map(<[u8]>::to_vec) else {
                break;
            };
            self.buf.drain(..4 + len);
            out.push((kind, payload));
        }
        Ok(out)
    }
}

/// The pseudo-TLS ClientHello record (§2.1.1, 50 octets).
pub fn pseudo_tls_client_hello(unix_time: u32, random: [u8; 28]) -> Vec<u8> {
    let mut v = vec![
        0x16, 0x03, 0x01, 0x00, 0x2d, 0x01, 0x00, 0x00, 0x29, 0x03, 0x01,
    ];
    v.extend_from_slice(&unix_time.to_be_bytes());
    v.extend_from_slice(&random);
    v.extend_from_slice(&[0x00, 0x00, 0x02, 0x00, 0x18, 0x01, 0x00]);
    v
}

/// Length of the ServerHello + ServerHelloDone record (5 + 0x4E).
pub const PSEUDO_TLS_SERVER_HELLO_LEN: usize = 83;

/// Checks the fixed fields of the server's pseudo-TLS record (§2.1.1).
pub fn check_pseudo_tls_server_hello(p: &[u8]) -> bool {
    let fixed = |at: usize, want: &[u8]| p.get(at..at + want.len()) == Some(want);
    p.len() == PSEUDO_TLS_SERVER_HELLO_LEN
        && fixed(
            0,
            &[
                0x16, 0x03, 0x01, 0x00, 0x4e, 0x02, 0x00, 0x00, 0x46, 0x03, 0x01,
            ],
        )
        && fixed(43, &[0x20])
        && fixed(76, &[0x00, 0x18, 0x00, 0x0e, 0x00, 0x00, 0x00])
}

#[cfg(test)]
mod tests;
