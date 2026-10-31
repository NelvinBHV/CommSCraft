//! SDP for audio calls (MS-SDPEXT): a bounded parser, the audio offer, the answer to a peer's
//! offer, SRTP keys (`a=crypto`) and ICE attributes.
//!
//! Evidence: `docs/protocol/evidence/14-sdpext-media-negotiation.md` (SD-01 … SD-05, SD-10,
//! SD-11, SD-13, SD-14, SD-21, SD-24, SD-25, SD-40 … SD-45). Sources: [MS-SDPEXT] 20250520
//! §3.1.5.1 (crypto, pp. 19–20), §3.1.5.3–3.1.5.5 (formats, PT order, DTMF, pp. 23–25), §3.1.5.8
//! (optional encryption, pp. 25–26), §3.1.5.15 (label, p. 30), §3.1.5.17 (rtcp, p. 31), §3.1.5.19
//! (o= line, p. 33), §3.1.5.21 (ICE-19 candidates, pp. 34–36), examples §4.1.3 (pp. 53–56) and
//! §4.3 (pp. 57–61); RFC 4566, RFC 3264 §6, RFC 4568 §6.1.
//!
//! Security by design: input bounded (16 KiB, 512 lines, 16 media sections, 64 candidates per
//! section); every value that later reaches a socket or a key is parsed into a typed value
//! (IP address, port, 30-byte key ‖ salt, 1-byte MKI) or refused; only AES_CM_128_HMAC_SHA1_80
//! with inline keys; an answer without usable keys to an encrypted offer is refused; plaintext
//! media only with an explicit policy (`Encryption::Off`, provisioned `DoNotSupport`) – never as
//! a silent fallback.

use std::net::IpAddr;

use commscraft_sipae::base64;

pub const MAX_SDP: usize = 16 * 1024;
const MAX_LINES: usize = 512;
const MAX_MEDIA: usize = 16;
const MAX_CANDIDATES: usize = 64;
pub const SUITE: &str = "AES_CM_128_HMAC_SHA1_80";
/// RTP payload type for telephone events in all examples (SD-25).
pub const DTMF_PT: u8 = 101;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    TooLarge,
    Syntax(&'static str),
    /// No audio stream, or none with a codec we support.
    NoAudio,
    /// The peer requires encryption but sent no usable key, or answered our key with a bad one.
    Crypto(&'static str),
    /// The peer wants plaintext media and our policy does not allow it.
    Unencrypted,
}

// ----- parsing ----------------------------------------------------------------------------------

/// One `m=` section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Media {
    pub kind: String,
    pub port: u16,
    pub proto: String,
    pub formats: Vec<u8>,
    pub connection: Option<IpAddr>,
    /// Attributes in order: (name, value) with the value trimmed.
    pub attributes: Vec<(String, String)>,
}

impl Media {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn attrs<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.attributes
            .iter()
            .filter(move |(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Session {
    pub origin_address: Option<IpAddr>,
    pub connection: Option<IpAddr>,
    pub attributes: Vec<(String, String)>,
    pub media: Vec<Media>,
}

fn connection(v: &str) -> Result<IpAddr, Error> {
    // c=IN IP4 <addr> (RFC 4566 §5.7); no multicast TTL suffixes.
    let mut f = v.split_whitespace();
    match (f.next(), f.next(), f.next()) {
        (Some("IN"), Some("IP4" | "IP6"), Some(a)) => a.parse().map_err(|_| Error::Syntax("c=")),
        _ => Err(Error::Syntax("c=")),
    }
}

/// Parses an SDP body (RFC 4566) with the bounds above. Unknown lines are ignored.
pub fn parse(body: &[u8]) -> Result<Session, Error> {
    if body.len() > MAX_SDP {
        return Err(Error::TooLarge);
    }
    let text = std::str::from_utf8(body).map_err(|_| Error::Syntax("UTF-8"))?;
    let mut s = Session::default();
    let mut version = false;
    for (n, line) in text.lines().enumerate() {
        if n >= MAX_LINES {
            return Err(Error::TooLarge);
        }
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let (k, v) = line.split_once('=').ok_or(Error::Syntax("line"))?;
        match k {
            "v" => version = v == "0",
            "o" => {
                // SD-02: sess-id and sess-version are ignored.
                let f: Vec<&str> = v.split_whitespace().collect();
                s.origin_address = f.get(5).and_then(|a| a.parse().ok());
            }
            "c" => {
                let c = connection(v)?;
                match s.media.last_mut() {
                    Some(m) => m.connection = Some(c),
                    None => s.connection = Some(c),
                }
            }
            "m" => {
                if s.media.len() >= MAX_MEDIA {
                    return Err(Error::TooLarge);
                }
                let mut f = v.split_whitespace();
                let kind = f.next().ok_or(Error::Syntax("m="))?.to_string();
                let port = f
                    .next()
                    .and_then(|p| p.split('/').next())
                    .and_then(|p| p.parse().ok())
                    .ok_or(Error::Syntax("m= port"))?;
                let proto = f.next().ok_or(Error::Syntax("m= proto"))?.to_string();
                let formats = f.filter_map(|x| x.parse().ok()).take(32).collect();
                s.media.push(Media {
                    kind,
                    port,
                    proto,
                    formats,
                    ..Media::default()
                });
            }
            "a" => {
                let (name, value) = v.split_once(':').unwrap_or((v, ""));
                let a = (name.to_string(), value.trim().to_string());
                match s.media.last_mut() {
                    Some(m) => m.attributes.push(a),
                    None => s.attributes.push(a),
                }
            }
            _ => {}
        }
    }
    if !version {
        return Err(Error::Syntax("v=0"));
    }
    Ok(s)
}

// ----- codecs -----------------------------------------------------------------------------------

/// The audio codecs CommSCraft implements (D-05; SD-21, SD-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    G722,
    Pcmu,
    Pcma,
}

impl Codec {
    /// Static payload type and rtpmap (RFC 3551 Table 4; G722 clock 8000 although 16 kHz).
    pub fn payload_type(self) -> u8 {
        match self {
            Codec::G722 => 9,
            Codec::Pcmu => 0,
            Codec::Pcma => 8,
        }
    }

    fn rtpmap(self) -> &'static str {
        match self {
            Codec::G722 => "G722/8000",
            Codec::Pcmu => "PCMU/8000",
            Codec::Pcma => "PCMA/8000",
        }
    }

    fn from_rtpmap(name: &str) -> Option<Self> {
        // Encoding names are case-insensitive (RFC 4566 §6), except "RED" (SD-23, not used here).
        match name.to_ascii_uppercase().as_str() {
            "G722/8000" | "G722/8000/1" => Some(Codec::G722),
            "PCMU/8000" | "PCMU/8000/1" => Some(Codec::Pcmu),
            "PCMA/8000" | "PCMA/8000/1" => Some(Codec::Pcma),
            _ => None,
        }
    }
}

/// Our preference: wideband first (SD-30 orders the examples the same way).
pub const OUR_CODECS: [Codec; 3] = [Codec::G722, Codec::Pcmu, Codec::Pcma];

/// The codec of each payload type of a media section: rtpmap first, static types otherwise.
fn codec_of(m: &Media, pt: u8) -> Option<Codec> {
    let mapped = m.attrs("rtpmap").find_map(|v| {
        let (p, name) = v.split_once(' ')?;
        (p.trim().parse::<u8>().ok()? == pt).then(|| name.trim().to_string())
    });
    match mapped {
        Some(name) => Codec::from_rtpmap(&name),
        None => OUR_CODECS.into_iter().find(|c| c.payload_type() == pt),
    }
}

fn dtmf_pt(m: &Media) -> Option<u8> {
    m.attrs("rtpmap").find_map(|v| {
        let (p, name) = v.split_once(' ')?;
        let pt: u8 = p.trim().parse().ok()?;
        (name.trim().eq_ignore_ascii_case("telephone-event/8000") && m.formats.contains(&pt))
            .then_some(pt)
    })
}

// ----- keys (a=crypto, SD-10) -------------------------------------------------------------------

/// An SRTP master key and salt with the 1-byte MKI.
#[derive(Clone, PartialEq, Eq)]
pub struct Key {
    pub tag: u32,
    pub key: [u8; 16],
    pub salt: [u8; 14],
    pub mki: Option<u8>,
}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material.
        f.debug_struct("Key")
            .field("tag", &self.tag)
            .field("mki", &self.mki)
            .finish_non_exhaustive()
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.key.zeroize();
        self.salt.zeroize();
    }
}

impl Key {
    /// A fresh key from 30 random bytes (MS-SRTP §5.1: random, send ≠ receive), MKI 1.
    pub fn new(tag: u32, random: [u8; 30]) -> Self {
        let mut key = [0u8; 16];
        let mut salt = [0u8; 14];
        let (k, s) = random.split_at(16);
        key.copy_from_slice(k);
        salt.copy_from_slice(s);
        Key {
            tag,
            key,
            salt,
            mki: Some(1),
        }
    }

    /// `a=crypto` value: `<tag> AES_CM_128_HMAC_SHA1_80 inline:<b64>|2^31|<mki>:1` (SD-10).
    pub fn attribute(&self) -> String {
        let mut raw = Vec::with_capacity(30);
        raw.extend_from_slice(&self.key);
        raw.extend_from_slice(&self.salt);
        let b64 = base64::encode(&raw);
        let mki = self.mki.map(|m| format!("|{m}:1")).unwrap_or_default();
        format!("{} {SUITE} inline:{b64}|2^31{mki}", self.tag)
    }

    /// Parses an `a=crypto` value; anything but the single suite with an inline 30-byte key and an
    /// optional 1-byte MKI is refused (SD-10, RFC 4568 §6.1).
    pub fn parse(v: &str) -> Result<Self, Error> {
        let mut f = v.split(' ');
        let tag: u32 = f
            .next()
            .and_then(|t| t.parse().ok())
            .filter(|t| *t < 1_000_000_000)
            .ok_or(Error::Crypto("tag"))?;
        if f.next() != Some(SUITE) {
            return Err(Error::Crypto("suite"));
        }
        let params = f.next().ok_or(Error::Crypto("key-params"))?;
        if f.next().is_some() {
            return Err(Error::Crypto("session parameters"));
        }
        let info = params
            .strip_prefix("inline:")
            .ok_or(Error::Crypto("key method"))?;
        let mut parts = info.split('|');
        let raw =
            base64::decode(parts.next().unwrap_or("")).map_err(|_| Error::Crypto("base64"))?;
        if raw.len() != 30 {
            return Err(Error::Crypto("key length"));
        }
        let mut mki = None;
        for p in parts.take(2) {
            if let Some((value, len)) = p.split_once(':') {
                if len != "1" {
                    return Err(Error::Crypto("MKI length"));
                }
                mki = Some(value.parse::<u8>().map_err(|_| Error::Crypto("MKI"))?);
            }
            // A lifetime is ignored when receiving (SD-10).
        }
        let mut key = [0u8; 16];
        let mut salt = [0u8; 14];
        let (k, s) = raw.split_at(16);
        key.copy_from_slice(k);
        salt.copy_from_slice(s);
        Ok(Key {
            tag,
            key,
            salt,
            mki,
        })
    }
}

// ----- ICE attributes (SD-41) -------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Udp,
    TcpActive,
    TcpPassive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateType {
    Host,
    ServerReflexive,
    Relay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub foundation: String,
    pub component: u8,
    pub transport: Transport,
    pub priority: u32,
    pub address: IpAddr,
    pub port: u16,
    pub kind: CandidateType,
    pub related: Option<(IpAddr, u16)>,
}

impl Candidate {
    /// `a=candidate` value in the ICE-19 form of MS-SDPEXT §3.1.5.21.
    pub fn attribute(&self) -> String {
        let t = match self.transport {
            Transport::Udp => "UDP",
            Transport::TcpActive => "TCP-ACT",
            Transport::TcpPassive => "TCP-PASS",
        };
        let k = match self.kind {
            CandidateType::Host => "host",
            CandidateType::ServerReflexive => "srflx",
            CandidateType::Relay => "relay",
        };
        let mut s = format!(
            "{} {} {t} {} {} {} typ {k}",
            self.foundation, self.component, self.priority, self.address, self.port
        );
        if let Some((a, p)) = self.related {
            s.push_str(&format!(" raddr {a} rport {p}"));
        }
        s
    }

    pub fn parse(v: &str) -> Result<Self, Error> {
        let f: Vec<&str> = v.split_whitespace().collect();
        let get = |i: usize| f.get(i).copied().ok_or(Error::Syntax("candidate"));
        let foundation = get(0)?;
        if foundation.len() > 32
            || !foundation
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/')
        {
            return Err(Error::Syntax("candidate foundation"));
        }
        let transport = match get(2)?.to_ascii_uppercase().as_str() {
            "UDP" => Transport::Udp,
            "TCP-ACT" => Transport::TcpActive,
            "TCP-PASS" => Transport::TcpPassive,
            _ => return Err(Error::Syntax("candidate transport")),
        };
        if get(6)? != "typ" {
            return Err(Error::Syntax("candidate typ"));
        }
        let kind = match get(7)? {
            "host" => CandidateType::Host,
            "srflx" | "prflx" => CandidateType::ServerReflexive,
            "relay" => CandidateType::Relay,
            _ => return Err(Error::Syntax("candidate type")),
        };
        let related = match (f.get(8), f.get(9), f.get(10), f.get(11)) {
            (Some(&"raddr"), Some(a), Some(&"rport"), Some(p)) => Some((
                a.parse().map_err(|_| Error::Syntax("raddr"))?,
                p.parse().map_err(|_| Error::Syntax("rport"))?,
            )),
            _ => None,
        };
        Ok(Candidate {
            foundation: foundation.to_string(),
            component: get(1)?
                .parse()
                .ok()
                .filter(|c| (1..=2).contains(c))
                .ok_or(Error::Syntax("component"))?,
            transport,
            priority: get(3)?.parse().map_err(|_| Error::Syntax("priority"))?,
            address: get(4)?.parse().map_err(|_| Error::Syntax("address"))?,
            port: get(5)?.parse().map_err(|_| Error::Syntax("port"))?,
            kind,
            related,
        })
    }
}

/// ICE credentials and candidates of one side.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ice {
    pub ufrag: String,
    pub pwd: String,
    pub candidates: Vec<Candidate>,
}

fn ice_of(m: &Media) -> Option<Ice> {
    let ok = |s: &str, min: usize| {
        (min..=256).contains(&s.len())
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    };
    let ufrag = m.attr("ice-ufrag").filter(|u| ok(u, 4))?.to_string();
    let pwd = m.attr("ice-pwd").filter(|p| ok(p, 22))?.to_string();
    let candidates = m
        .attrs("candidate")
        .filter_map(|c| Candidate::parse(c).ok())
        .take(MAX_CANDIDATES)
        .collect();
    Some(Ice {
        ufrag,
        pwd,
        candidates,
    })
}

// ----- offer / answer ---------------------------------------------------------------------------

/// Encryption policy (SD-11, SD-14; provisioned `ucPC2PCAVEncryption`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encryption {
    /// `Require`: offer RTP/SAVP; accept only encrypted answers.
    Required,
    /// `Support`: offer RTP/AVP with a key (optional SRTP, §3.1.5.8). CommSCraft still refuses a
    /// plaintext answer unless `allow_plaintext` – design decision (security by design).
    Optional { allow_plaintext: bool },
    /// `DoNotSupport`: no keys, plaintext RTP.
    Off,
}

/// What we put into an audio offer or answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub address: IpAddr,
    pub rtp_port: u16,
    pub rtcp_port: u16,
    pub key: Option<Key>,
    pub ice: Option<Ice>,
    pub direction: Direction,
    /// Final offer of the controlling agent: the chosen remote candidates per component
    /// (ice-19 §9.1.2.2, syntax §15.2: `a=remote-candidates`, SD-41).
    pub remote_candidates: Vec<(u8, std::net::SocketAddr)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    SendRecv,
    SendOnly,
    RecvOnly,
    Inactive,
}

impl Direction {
    fn as_str(self) -> &'static str {
        match self {
            Direction::SendRecv => "sendrecv",
            Direction::SendOnly => "sendonly",
            Direction::RecvOnly => "recvonly",
            Direction::Inactive => "inactive",
        }
    }

    fn of(m: &Media) -> Self {
        m.attributes
            .iter()
            .find_map(|(n, _)| match n.as_str() {
                "sendonly" => Some(Direction::SendOnly),
                "recvonly" => Some(Direction::RecvOnly),
                "inactive" => Some(Direction::Inactive),
                "sendrecv" => Some(Direction::SendRecv),
                _ => None,
            })
            .unwrap_or(Direction::SendRecv)
    }
}

fn write(local: &Local, proto: &str, codecs: &[(Codec, u8)], dtmf: Option<u8>) -> String {
    let (fam, addr) = match local.address {
        IpAddr::V4(a) => ("IP4", a.to_string()),
        IpAddr::V6(a) => ("IP6", a.to_string()),
    };
    let mut fmts: Vec<String> = codecs.iter().map(|(_, pt)| pt.to_string()).collect();
    if let Some(d) = dtmf {
        fmts.push(d.to_string());
    }
    let mut s = format!(
        "v=0\r\no=- 0 0 IN {fam} {addr}\r\ns=session\r\nc=IN {fam} {addr}\r\nt=0 0\r\nm=audio {} {proto} {}\r\n",
        local.rtp_port,
        fmts.join(" ")
    );
    if let Some(ice) = &local.ice {
        s.push_str(&format!(
            "a=ice-ufrag:{}\r\na=ice-pwd:{}\r\n",
            ice.ufrag, ice.pwd
        ));
        for c in &ice.candidates {
            s.push_str(&format!("a=candidate:{}\r\n", c.attribute()));
        }
    }
    if !local.remote_candidates.is_empty() {
        let list: Vec<String> = local
            .remote_candidates
            .iter()
            .take(2)
            .map(|(c, a)| format!("{c} {} {}", a.ip(), a.port()))
            .collect();
        s.push_str(&format!("a=remote-candidates:{}\r\n", list.join(" ")));
    }
    s.push_str("a=label:main-audio\r\n");
    if let Some(k) = &local.key {
        s.push_str(&format!("a=crypto:{}\r\n", k.attribute()));
    }
    s.push_str(&format!("a=rtcp:{}\r\n", local.rtcp_port));
    s.push_str(&format!("a={}\r\n", local.direction.as_str()));
    for (c, pt) in codecs {
        s.push_str(&format!("a=rtpmap:{pt} {}\r\n", c.rtpmap()));
    }
    if let Some(d) = dtmf {
        s.push_str(&format!(
            "a=rtpmap:{d} telephone-event/8000\r\na=fmtp:{d} 0-16\r\n"
        ));
    }
    s.push_str("a=ptime:20\r\n");
    s
}

/// Our audio offer (OCS call setup with SDP in the INVITE; SD-03, SD-10, SD-11, SD-25).
pub fn offer(local: &Local, policy: Encryption) -> String {
    let proto = match policy {
        Encryption::Required => "RTP/SAVP",
        _ => "RTP/AVP",
    };
    let mut l = local.clone();
    if policy == Encryption::Off {
        l.key = None;
    }
    let codecs: Vec<(Codec, u8)> = OUR_CODECS.iter().map(|c| (*c, c.payload_type())).collect();
    write(&l, proto, &codecs, Some(DTMF_PT))
}

/// The negotiated audio stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Negotiated {
    pub codec: Codec,
    pub payload_type: u8,
    pub dtmf_payload_type: Option<u8>,
    /// Where to send RTP and RTCP (the default candidate; ICE may replace it).
    pub remote_rtp: (IpAddr, u16),
    pub remote_rtcp: (IpAddr, u16),
    /// The peer's key (it encrypts with it, we decrypt with it); `None` = plaintext.
    pub remote_key: Option<Key>,
    pub remote_ice: Option<Ice>,
    pub direction: Direction,
    /// The peer streams music on hold (`a=sendonly` + `a=feature:MoH`, MS-SDPEXT §3.1.5.27).
    pub music_on_hold: bool,
}

/// MS-SDPEXT §3.1.5.27.2: `a=feature:MoH` on the audio line.
fn music_on_hold(m: &Media) -> bool {
    m.attrs("feature").any(|v| v.trim() == "MoH")
}

fn audio(s: &Session) -> Result<&Media, Error> {
    s.media
        .iter()
        .find(|m| m.kind == "audio" && m.port != 0)
        .ok_or(Error::NoAudio)
}

/// A transport address.
pub type Endpoint = (IpAddr, u16);

fn endpoints(s: &Session, m: &Media) -> Result<(Endpoint, Endpoint), Error> {
    let addr = m
        .connection
        .or(s.connection)
        .ok_or(Error::Syntax("no c= line"))?;
    let rtcp_port = match m.attr("rtcp") {
        // SD-05: port only.
        Some(p) => p
            .split_whitespace()
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or(Error::Syntax("a=rtcp"))?,
        None => m.port.checked_add(1).ok_or(Error::Syntax("port"))?,
    };
    Ok(((addr, m.port), (addr, rtcp_port)))
}

fn keys_of(m: &Media) -> Vec<Key> {
    m.attrs("crypto")
        .filter_map(|v| Key::parse(v).ok())
        .take(8)
        .collect()
}

/// Evaluates the peer's answer to our offer.
pub fn answer_to_offer(
    answer: &[u8],
    our_key: Option<&Key>,
    policy: Encryption,
) -> Result<Negotiated, Error> {
    let s = parse(answer)?;
    let m = audio(&s)?;
    let secure = m.proto.eq_ignore_ascii_case("RTP/SAVP");
    let remote_key = if secure {
        let ours = our_key.ok_or(Error::Crypto("SAVP answer without our offer of a key"))?;
        // The answer echoes the tag of the accepted crypto line (§4.3.1.3).
        let k = keys_of(m)
            .into_iter()
            .find(|k| k.tag == ours.tag)
            .ok_or(Error::Crypto("no key with our tag"))?;
        if k.key == ours.key {
            // Send and receive keys must differ (MS-SRTP §5.1).
            return Err(Error::Crypto("peer reused our key"));
        }
        Some(k)
    } else {
        match policy {
            Encryption::Off
            | Encryption::Optional {
                allow_plaintext: true,
            } => None,
            _ => return Err(Error::Unencrypted),
        }
    };
    // The answer lists a subset of our formats (SD-03); the first one we know wins.
    let (codec, pt) = m
        .formats
        .iter()
        .find_map(|&pt| codec_of(m, pt).map(|c| (c, pt)))
        .ok_or(Error::NoAudio)?;
    let (rtp, rtcp) = endpoints(&s, m)?;
    Ok(Negotiated {
        codec,
        payload_type: pt,
        dtmf_payload_type: dtmf_pt(m),
        remote_rtp: rtp,
        remote_rtcp: rtcp,
        remote_key,
        remote_ice: ice_of(m),
        direction: Direction::of(m),
        music_on_hold: music_on_hold(m),
    })
}

/// Our answer to the peer's offer (incoming call). `fresh_key` is our own new key; its tag is
/// replaced by the tag of the accepted offer line. Returns the SDP and the negotiation.
pub fn answer(
    offer: &[u8],
    local: &Local,
    fresh_key: Key,
    policy: Encryption,
) -> Result<(String, Negotiated), Error> {
    let s = parse(offer)?;
    let m = audio(&s)?;
    let required = m.proto.eq_ignore_ascii_case("RTP/SAVP");
    let offered = keys_of(m).into_iter().next();
    let (proto, ours, remote_key) = match (offered, policy) {
        (Some(k), Encryption::Required | Encryption::Optional { .. }) => {
            let mut mine = fresh_key;
            mine.tag = k.tag;
            if mine.key == k.key {
                return Err(Error::Crypto("key collision"));
            }
            ("RTP/SAVP", Some(mine), Some(k))
        }
        (Some(_), Encryption::Off) if !required => ("RTP/AVP", None, None),
        (None, _) if required => return Err(Error::Crypto("SAVP offer without usable key")),
        (
            None,
            Encryption::Off
            | Encryption::Optional {
                allow_plaintext: true,
            },
        ) => ("RTP/AVP", None, None),
        _ => return Err(Error::Unencrypted),
    };
    // Keep the offer's order and payload numbers (SD-03).
    let codecs: Vec<(Codec, u8)> = m
        .formats
        .iter()
        .filter_map(|&pt| codec_of(m, pt).map(|c| (c, pt)))
        .collect();
    let (codec, pt) = *codecs.first().ok_or(Error::NoAudio)?;
    let dtmf = dtmf_pt(m);
    let (rtp, rtcp) = endpoints(&s, m)?;
    let mut l = local.clone();
    l.key = ours;
    // Answer with the direction that matches the offer (RFC 3264 §6.1).
    let remote_direction = Direction::of(m);
    l.direction = match remote_direction {
        Direction::SendOnly => Direction::RecvOnly,
        Direction::RecvOnly => Direction::SendOnly,
        Direction::Inactive => Direction::Inactive,
        Direction::SendRecv => local.direction,
    };
    let sdp = write(&l, proto, &codecs, dtmf);
    Ok((
        sdp,
        Negotiated {
            codec,
            payload_type: pt,
            dtmf_payload_type: dtmf,
            remote_rtp: rtp,
            remote_rtcp: rtcp,
            remote_key,
            remote_ice: ice_of(m),
            direction: remote_direction,
            music_on_hold: music_on_hold(m),
        },
    ))
}

#[cfg(test)]
mod tests;
