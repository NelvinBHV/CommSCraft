//! STUN messages in the two formats a Skype for Business client meets (sans-I/O):
//! RFC 5389 (magic cookie in the header, attributes padded to 4 octets) and the legacy format of
//! STUN-02 / MS-TURN (128-bit transaction ID, no padding, MESSAGE-INTEGRITY over a text padded
//! to 64 octets, optionally HMAC-SHA256 for MS-TURN version ≥ 3).
//!
//! Evidence: `docs/protocol/evidence/12-turn-relay.md` (TU-10 … TU-15), `13-ice-connectivity.md`
//! (IC-02 … IC-04, IC-13 … IC-16) and file 38 (IE-02, IE-03). Sources: RFC 5389 §6, §15
//! (attributes, XOR-MAPPED-ADDRESS, MESSAGE-INTEGRITY, FINGERPRINT); RFC 5769 (test vectors);
//! draft-ietf-behave-rfc3489bis-02 §10.1, §10.2, §10.2.8, §10.2.12; [MS-TURN] 20250520 §2.2.1–2.2.2.3
//! (pp. 20–25); [MS-ICE2] 20250520 §2.2.2 (p. 17).
//!
//! Security by design: messages ≤ 1500 octets (IC-02), at most 32 attributes, every attribute
//! length checked; MESSAGE-INTEGRITY is compared in constant time and must be the last
//! attribute (FINGERPRINT may follow in RFC 5389); keys are only passed in, never stored here.

use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Digest as _, Sha256};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub const MAX_MESSAGE: usize = 1500;
const MAX_ATTRIBUTES: usize = 32;
pub const MAGIC_COOKIE: u32 = 0x2112_A442;

// Attribute types used by CommSCraft.
pub const MAPPED_ADDRESS: u16 = 0x0001;
pub const USERNAME: u16 = 0x0006;
pub const MESSAGE_INTEGRITY: u16 = 0x0008;
pub const ERROR_CODE: u16 = 0x0009;
pub const LIFETIME: u16 = 0x000D;
pub const ALTERNATE_SERVER: u16 = 0x000E;
/// MS-TURN: the magic cookie 0x72C64BC6 as the first attribute (TU-10).
pub const MS_MAGIC_COOKIE: u16 = 0x000F;
pub const MS_MAGIC_COOKIE_VALUE: u32 = 0x72C6_4BC6;
pub const BANDWIDTH: u16 = 0x0010;
pub const DESTINATION_ADDRESS: u16 = 0x0011;
pub const REMOTE_ADDRESS: u16 = 0x0012;
pub const DATA: u16 = 0x0013;
pub const NONCE: u16 = 0x0014;
pub const REALM: u16 = 0x0015;
pub const REQUESTED_ADDRESS_FAMILY: u16 = 0x0017;
pub const XOR_MAPPED_ADDRESS: u16 = 0x0020;
pub const PRIORITY: u16 = 0x0024;
pub const USE_CANDIDATE: u16 = 0x0025;
pub const MS_VERSION: u16 = 0x8008;
pub const MS_XOR_MAPPED_ADDRESS: u16 = 0x8020;
pub const SOFTWARE: u16 = 0x8022;
pub const FINGERPRINT: u16 = 0x8028;
pub const ICE_CONTROLLED: u16 = 0x8029;
pub const ICE_CONTROLLING: u16 = 0x802A;
pub const MS_SEQUENCE_NUMBER: u16 = 0x8050;
pub const CANDIDATE_IDENTIFIER: u16 = 0x8054;
pub const MS_SERVICE_QUALITY: u16 = 0x8055;
pub const IMPLEMENTATION_VERSION: u16 = 0x8070;
pub const MS_ALTERNATE_MAPPED_ADDRESS: u16 = 0x8090;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    TooLarge,
    Malformed,
    TooManyAttributes,
    /// MESSAGE-INTEGRITY missing, not last, or wrong.
    Integrity,
    Fingerprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// RFC 5389: magic cookie in octets 4–7, 96-bit transaction ID, padding to 4 octets.
    Rfc5389,
    /// STUN-02 / MS-TURN: 128-bit transaction ID, no padding.
    Legacy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub kind: u16,
    pub value: Vec<u8>,
}

/// A STUN message. `transaction` is the 16-octet field after the length: for RFC 5389 the magic
/// cookie followed by the 96-bit transaction ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub kind: u16,
    pub transaction: [u8; 16],
    pub attributes: Vec<Attribute>,
}

fn be16(p: &[u8], at: usize) -> Result<u16, Error> {
    let b = p.get(at..at + 2).ok_or(Error::Malformed)?;
    Ok(u16::from_be_bytes(
        b.try_into().map_err(|_| Error::Malformed)?,
    ))
}

fn pad4(n: usize) -> usize {
    (4 - n % 4) % 4
}

/// The positions of all attributes in a raw message: (type, value start, value length, end
/// incl. padding).
fn walk(p: &[u8], format: Format) -> Result<Vec<(u16, usize, usize, usize)>, Error> {
    let total = 20 + usize::from(be16(p, 2)?);
    if p.len() < total {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    let mut off = 20;
    while off < total {
        if out.len() >= MAX_ATTRIBUTES {
            return Err(Error::TooManyAttributes);
        }
        let kind = be16(p, off)?;
        let len = usize::from(be16(p, off + 2)?);
        let start = off + 4;
        let pad = if format == Format::Rfc5389 {
            pad4(len)
        } else {
            0
        };
        let end = start + len + pad;
        if end > total {
            return Err(Error::Malformed);
        }
        out.push((kind, start, len, end));
        off = end;
    }
    Ok(out)
}

impl Message {
    pub fn new(kind: u16, transaction: [u8; 16]) -> Self {
        Message {
            kind,
            transaction,
            attributes: Vec::new(),
        }
    }

    /// A new RFC 5389 transaction field: magic cookie + 96 random bits.
    pub fn rfc5389_transaction(random: [u8; 12]) -> [u8; 16] {
        let mut t = [0u8; 16];
        let (c, r) = t.split_at_mut(4);
        c.copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
        r.copy_from_slice(&random);
        t
    }

    pub fn push(&mut self, kind: u16, value: impl Into<Vec<u8>>) -> &mut Self {
        self.attributes.push(Attribute {
            kind,
            value: value.into(),
        });
        self
    }

    pub fn get(&self, kind: u16) -> Option<&[u8]> {
        self.attributes
            .iter()
            .find(|a| a.kind == kind)
            .map(|a| a.value.as_slice())
    }

    pub fn format(&self) -> Format {
        if self.transaction.get(..4) == Some(&MAGIC_COOKIE.to_be_bytes()[..]) {
            Format::Rfc5389
        } else {
            Format::Legacy
        }
    }

    /// Parses a message; the format follows from the magic cookie (RFC 5389 §6).
    pub fn decode(p: &[u8]) -> Result<Self, Error> {
        if p.len() > MAX_MESSAGE {
            return Err(Error::TooLarge);
        }
        if p.len() < 20 || p.first().is_some_and(|b| b & 0xC0 != 0) {
            return Err(Error::Malformed);
        }
        let mut transaction = [0u8; 16];
        transaction.copy_from_slice(p.get(4..20).ok_or(Error::Malformed)?);
        let mut m = Message::new(be16(p, 0)?, transaction);
        for (kind, start, len, _) in walk(p, m.format())? {
            let value = p.get(start..start + len).ok_or(Error::Malformed)?;
            m.push(kind, value.to_vec());
        }
        Ok(m)
    }

    /// Encodes the attributes (padding per format). Integrity and fingerprint are added with
    /// [`with_integrity`] / [`with_fingerprint`] on the result.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let format = self.format();
        let mut out = Vec::with_capacity(128);
        out.extend_from_slice(&self.kind.to_be_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&self.transaction);
        for a in &self.attributes {
            let len = u16::try_from(a.value.len()).map_err(|_| Error::TooLarge)?;
            out.extend_from_slice(&a.kind.to_be_bytes());
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(&a.value);
            if format == Format::Rfc5389 {
                out.resize(out.len() + pad4(a.value.len()), 0);
            }
        }
        let total = out.len();
        set_length(&mut out, total)?;
        if out.len() > MAX_MESSAGE {
            return Err(Error::TooLarge);
        }
        Ok(out)
    }
}

fn set_length(p: &mut [u8], total: usize) -> Result<(), Error> {
    let l = u16::try_from(total.checked_sub(20).ok_or(Error::Malformed)?)
        .map_err(|_| Error::TooLarge)?;
    let slot = p.get_mut(2..4).ok_or(Error::Malformed)?;
    slot.copy_from_slice(&l.to_be_bytes());
    Ok(())
}

// ----- MESSAGE-INTEGRITY and FINGERPRINT -------------------------------------------------------

/// The integrity algorithm and key.
#[derive(Clone, Copy)]
pub enum Integrity<'a> {
    /// RFC 5389 §15.4 (short-term: the password; long-term: [`long_term_key`]).
    Rfc5389(&'a [u8]),
    /// STUN-02 §10.2.8 / MS-TURN SHA-1: text padded to 64 octets (IE-02, TU-14).
    LegacySha1(&'a [u8]),
    /// MS-TURN version ≥ 3: HMAC-SHA256 with [`ms_turn_sha256_key`], text padded to 64 octets.
    LegacySha256(&'a [u8]),
}

impl std::fmt::Debug for Integrity<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Integrity::Rfc5389(_) => "Rfc5389",
            Integrity::LegacySha1(_) => "LegacySha1",
            Integrity::LegacySha256(_) => "LegacySha256",
        })
    }
}

/// Long-term key MD5(username ":" realm ":" password) (RFC 5389 §15.4; MS-TURN §2.2.2.3).
pub fn long_term_key(username: &[u8], realm: &[u8], password: &[u8]) -> [u8; 16] {
    use md5::Digest;
    let mut h = md5::Md5::new();
    h.update(username);
    h.update(b":");
    h.update(realm);
    h.update(b":");
    h.update(password);
    h.finalize().into()
}

/// MS-TURN §2.2.2.3: K = HMAC-SHA256(Nonce, password);
/// Key = HMAC-SHA256(K, 0x01 ‖ "TURN" ‖ 0x00 ‖ Username ‖ Realm ‖ 0x00000100).
pub fn ms_turn_sha256_key(
    nonce: &[u8],
    password: &[u8],
    username: &[u8],
    realm: &[u8],
) -> [u8; 32] {
    let k = hmac256(nonce, &[password]);
    hmac256(
        &k,
        &[
            &[0x01],
            b"TURN",
            &[0x00],
            username,
            realm,
            &0x100u32.to_be_bytes(),
        ],
    )
}

fn hmac256(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    match <Hmac<Sha256> as Mac>::new_from_slice(key) {
        Ok(mut m) => {
            for p in parts {
                m.update(p);
            }
            m.finalize().into_bytes().into()
        }
        // HMAC accepts keys of any length; unreachable in practice, fail closed.
        Err(_) => Sha256::digest([0u8]).into(),
    }
}

fn mac_parts(integrity: Integrity<'_>, text: &[u8]) -> Vec<u8> {
    let padded = |t: &[u8]| {
        let mut v = t.to_vec();
        v.resize(t.len() + (64 - t.len() % 64) % 64, 0);
        v
    };
    match integrity {
        Integrity::Rfc5389(k) => hmac1(k, text),
        Integrity::LegacySha1(k) => hmac1(k, &padded(text)),
        Integrity::LegacySha256(k) => hmac256(k, &[&padded(text)]).to_vec(),
    }
}

fn hmac1(key: &[u8], text: &[u8]) -> Vec<u8> {
    match <Hmac<Sha1> as Mac>::new_from_slice(key) {
        Ok(mut m) => {
            m.update(text);
            m.finalize().into_bytes().to_vec()
        }
        Err(_) => vec![0; 20],
    }
}

fn mac_len(integrity: Integrity<'_>) -> usize {
    match integrity {
        Integrity::LegacySha256(_) => 32,
        _ => 20,
    }
}

/// Appends MESSAGE-INTEGRITY to an encoded message.
/// RFC 5389: the length field covers the MI attribute while hashing (§15.4).
/// Legacy: the text is the message up to the MI attribute with the length field counting MI
/// (equal to the transmitted header when MI is last, as in MS-TURN), padded to 64 octets.
pub fn with_integrity(mut p: Vec<u8>, integrity: Integrity<'_>) -> Result<Vec<u8>, Error> {
    let n = mac_len(integrity);
    let total = p.len() + 4 + n;
    set_length(&mut p, total)?;
    let mac = mac_parts(integrity, &p);
    p.extend_from_slice(&MESSAGE_INTEGRITY.to_be_bytes());
    p.extend_from_slice(&(n as u16).to_be_bytes());
    p.extend_from_slice(&mac);
    if p.len() > MAX_MESSAGE {
        return Err(Error::TooLarge);
    }
    Ok(p)
}

/// Appends FINGERPRINT (CRC-32 ⊕ 0x5354554E over the message before it, length covering it;
/// RFC 5389 §15.5, ice-19 §7.1.1 – IE-03).
pub fn with_fingerprint(mut p: Vec<u8>) -> Result<Vec<u8>, Error> {
    let total = p.len() + 8;
    set_length(&mut p, total)?;
    let fp = commscraft_ice::fingerprint(&p);
    p.extend_from_slice(&FINGERPRINT.to_be_bytes());
    p.extend_from_slice(&4u16.to_be_bytes());
    p.extend_from_slice(&fp.to_be_bytes());
    Ok(p)
}

/// Checks MESSAGE-INTEGRITY of a raw message (constant-time comparison). The MI attribute must be
/// the last one, or followed only by FINGERPRINT (RFC 5389).
pub fn verify_integrity(p: &[u8], integrity: Integrity<'_>) -> Result<(), Error> {
    let format = if p.get(4..8) == Some(&MAGIC_COOKIE.to_be_bytes()[..]) {
        Format::Rfc5389
    } else {
        Format::Legacy
    };
    let attrs = walk(p, format)?;
    let pos = attrs
        .iter()
        .position(|a| a.0 == MESSAGE_INTEGRITY)
        .ok_or(Error::Integrity)?;
    let after = attrs.get(pos + 1..).unwrap_or(&[]);
    if !(after.is_empty() || (after.len() == 1 && after.iter().all(|a| a.0 == FINGERPRINT))) {
        return Err(Error::Integrity);
    }
    let (_, start, len, end) = *attrs.get(pos).ok_or(Error::Integrity)?;
    if len != mac_len(integrity) {
        return Err(Error::Integrity);
    }
    let mut text = p.get(..start - 4).ok_or(Error::Malformed)?.to_vec();
    // The length as if MI were the last attribute (RFC 5389 §15.4; for the legacy format with a
    // following FINGERPRINT the same rule – inference, O-IC-2).
    set_length(&mut text, end)?;
    let expected = mac_parts(integrity, &text);
    let received = p.get(start..start + len).ok_or(Error::Malformed)?;
    use subtle::ConstantTimeEq;
    if bool::from(expected.ct_eq(received)) {
        Ok(())
    } else {
        Err(Error::Integrity)
    }
}

/// Checks FINGERPRINT (must be the last attribute).
pub fn verify_fingerprint(p: &[u8]) -> Result<(), Error> {
    let format = if p.get(4..8) == Some(&MAGIC_COOKIE.to_be_bytes()[..]) {
        Format::Rfc5389
    } else {
        Format::Legacy
    };
    let attrs = walk(p, format)?;
    let &(kind, start, len, _) = attrs.last().ok_or(Error::Fingerprint)?;
    if kind != FINGERPRINT || len != 4 {
        return Err(Error::Fingerprint);
    }
    let value = p.get(start..start + 4).ok_or(Error::Malformed)?;
    let got = u32::from_be_bytes(value.try_into().map_err(|_| Error::Malformed)?);
    let text = p.get(..start - 4).ok_or(Error::Malformed)?;
    if commscraft_ice::fingerprint(text) == got {
        Ok(())
    } else {
        Err(Error::Fingerprint)
    }
}

// ----- addresses -------------------------------------------------------------------------------

/// MAPPED-ADDRESS-style value (family 1 = IPv4, 2 = IPv6).
pub fn encode_address(a: SocketAddr) -> Vec<u8> {
    let mut v = vec![0];
    match a.ip() {
        IpAddr::V4(ip) => {
            v.push(1);
            v.extend_from_slice(&a.port().to_be_bytes());
            v.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            v.push(2);
            v.extend_from_slice(&a.port().to_be_bytes());
            v.extend_from_slice(&ip.octets());
        }
    }
    v
}

pub fn decode_address(v: &[u8]) -> Result<SocketAddr, Error> {
    let port = be16(v, 2)?;
    match (v.get(1), v.len()) {
        (Some(1), 8) => {
            let o: [u8; 4] = v
                .get(4..8)
                .ok_or(Error::Malformed)?
                .try_into()
                .map_err(|_| Error::Malformed)?;
            Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::from(o)), port))
        }
        (Some(2), 20) => {
            let o: [u8; 16] = v
                .get(4..20)
                .ok_or(Error::Malformed)?
                .try_into()
                .map_err(|_| Error::Malformed)?;
            Ok(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(o)), port))
        }
        _ => Err(Error::Malformed),
    }
}

fn xor(v: &mut [u8], transaction: &[u8; 16]) {
    // Port with the top 16 bits; the address with the top 32 bits (IPv4) or all 128 bits (IPv6)
    // of the 16-octet field – identical for RFC 5389 (cookie ‖ ID) and STUN-02 / MS-TURN.
    if let Some(port) = v.get_mut(2..4) {
        for (b, t) in port.iter_mut().zip(transaction.iter()) {
            *b ^= t;
        }
    }
    if let Some(addr) = v.get_mut(4..) {
        for (b, t) in addr.iter_mut().zip(transaction.iter()) {
            *b ^= t;
        }
    }
}

pub fn encode_xor_address(a: SocketAddr, transaction: &[u8; 16]) -> Vec<u8> {
    let mut v = encode_address(a);
    xor(&mut v, transaction);
    v
}

pub fn decode_xor_address(v: &[u8], transaction: &[u8; 16]) -> Result<SocketAddr, Error> {
    let mut v = v.to_vec();
    xor(&mut v, transaction);
    decode_address(&v)
}

/// ERROR-CODE value → (code, reason).
pub fn decode_error(v: &[u8]) -> Result<(u16, String), Error> {
    let class = u16::from(*v.get(2).ok_or(Error::Malformed)? & 0x07);
    let number = u16::from(*v.get(3).ok_or(Error::Malformed)?);
    let reason: String = String::from_utf8_lossy(v.get(4..).unwrap_or(&[]))
        .chars()
        .filter(|c| !c.is_control())
        .take(128)
        .collect();
    Ok((class * 100 + number, reason))
}

pub fn encode_error(code: u16, reason: &str) -> Vec<u8> {
    let mut v = vec![0, 0, (code / 100) as u8, (code % 100) as u8];
    v.extend_from_slice(reason.as_bytes());
    v
}

#[cfg(test)]
mod tests;
