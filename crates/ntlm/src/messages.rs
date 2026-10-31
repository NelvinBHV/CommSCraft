//! NTLM CHALLENGE_MESSAGE parsing and AUTHENTICATE_MESSAGE construction for connectionless NTLMv2
//! as used by MS-SIPAE (AE-24).
//!
//! Source: [MS-NLMP] 20260330 §2.2.1.2–2.2.1.3 (pp. 24–30), §2.2.2.1 AV_PAIR (pp. 31–33),
//! §2.2.2.5 flags, §3.1.5.2 connectionless client (pp. 48–49), §3.3.2 NTLMv2 (pp. 60–61),
//! example §4.2.4.3 (p. 82).
//!
//! Security by design: the CHALLENGE comes from the network and is parsed with bounds checks on
//! every offset and length; the client refuses anything weaker than NTLMv2 with extended session
//! security, 128-bit keys, key exchange, signing and Unicode (OB-43, `docs/security.md`); the
//! password-derived key and the session key are wiped after use and never printed.

use crate::{
    encrypt_session_key, hmac_md5, lmv2_response, nt_proof_str, ntlmv2_temp, ntowf_v2,
    session_base_key, utf16le,
};
use zeroize::Zeroize;

/// NegotiateFlags (§2.2.2.5).
pub mod flags {
    pub const UNICODE: u32 = 0x0000_0001;
    pub const OEM: u32 = 0x0000_0002;
    pub const REQUEST_TARGET: u32 = 0x0000_0004;
    pub const SIGN: u32 = 0x0000_0010;
    pub const SEAL: u32 = 0x0000_0020;
    pub const DATAGRAM: u32 = 0x0000_0040;
    pub const LM_KEY: u32 = 0x0000_0080;
    pub const NTLM: u32 = 0x0000_0200;
    pub const ANONYMOUS: u32 = 0x0000_0800;
    pub const ALWAYS_SIGN: u32 = 0x0000_8000;
    pub const TARGET_TYPE_DOMAIN: u32 = 0x0001_0000;
    pub const TARGET_TYPE_SERVER: u32 = 0x0002_0000;
    pub const EXTENDED_SESSIONSECURITY: u32 = 0x0008_0000;
    pub const IDENTIFY: u32 = 0x0010_0000;
    pub const TARGET_INFO: u32 = 0x0080_0000;
    pub const VERSION: u32 = 0x0200_0000;
    pub const N128: u32 = 0x2000_0000;
    pub const KEY_EXCH: u32 = 0x4000_0000;
    pub const N56: u32 = 0x8000_0000;
}

/// AV_PAIR identifiers (§2.2.2.1).
pub mod av {
    pub const EOL: u16 = 0;
    pub const FLAGS: u16 = 6;
    pub const TIMESTAMP: u16 = 7;
    pub const TARGET_NAME: u16 = 9;
    pub const CHANNEL_BINDINGS: u16 = 10;
}

const SIGNATURE: &[u8; 8] = b"NTLMSSP\0";
/// Upper bound for a CHALLENGE (inference: real ones are a few hundred bytes).
pub const MAX_CHALLENGE_LEN: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    TooLong,
    BadSignature,
    WrongMessageType,
    /// A field offset or length points outside the message.
    BadField,
    /// The AV_PAIR list is malformed (no MsvAvEOL, a pair past the end, a duplicate).
    BadTargetInfo,
    /// The server does not offer a flag the client policy requires.
    MissingFlag(u32),
    /// A name is too long for a 16-bit length field.
    NameTooLong,
}

/// A parsed CHALLENGE_MESSAGE (§2.2.1.2). `raw` is kept for the MIC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub flags: u32,
    pub server_challenge: [u8; 8],
    pub target_name: Vec<u8>,
    /// AV_PAIR list without the terminating MsvAvEOL: (id, value).
    pub target_info: Vec<(u16, Vec<u8>)>,
    pub raw: Vec<u8>,
}

fn u16_at(b: &[u8], at: usize) -> Result<u16, Error> {
    let s = b.get(at..at + 2).ok_or(Error::BadField)?;
    Ok(u16::from_le_bytes([
        *s.first().ok_or(Error::BadField)?,
        *s.get(1).ok_or(Error::BadField)?,
    ]))
}

fn u32_at(b: &[u8], at: usize) -> Result<u32, Error> {
    let s = b.get(at..at + 4).ok_or(Error::BadField)?;
    let mut a = [0u8; 4];
    a.copy_from_slice(s);
    Ok(u32::from_le_bytes(a))
}

/// Reads a `Len, MaxLen, Offset` field descriptor and returns the referenced bytes.
fn field_at(b: &[u8], at: usize) -> Result<&[u8], Error> {
    let len = usize::from(u16_at(b, at)?);
    let off = usize::try_from(u32_at(b, at + 4)?).map_err(|_| Error::BadField)?;
    if len == 0 {
        return Ok(&[]);
    }
    let end = off.checked_add(len).ok_or(Error::BadField)?;
    b.get(off..end).ok_or(Error::BadField)
}

fn parse_av_pairs(mut b: &[u8]) -> Result<Vec<(u16, Vec<u8>)>, Error> {
    let mut out: Vec<(u16, Vec<u8>)> = Vec::new();
    loop {
        let id = u16_at(b, 0).map_err(|_| Error::BadTargetInfo)?;
        let len = usize::from(u16_at(b, 2).map_err(|_| Error::BadTargetInfo)?);
        let value = b.get(4..4 + len).ok_or(Error::BadTargetInfo)?;
        if id == av::EOL {
            return if len == 0 {
                Ok(out)
            } else {
                Err(Error::BadTargetInfo)
            };
        }
        if out.iter().any(|(i, _)| *i == id) || out.len() >= 64 {
            return Err(Error::BadTargetInfo);
        }
        out.push((id, value.to_vec()));
        b = b.get(4 + len..).ok_or(Error::BadTargetInfo)?;
    }
}

fn encode_av_pairs(pairs: &[(u16, Vec<u8>)]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    for (id, v) in pairs {
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(
            &u16::try_from(v.len())
                .map_err(|_| Error::BadTargetInfo)?
                .to_le_bytes(),
        );
        out.extend_from_slice(v);
    }
    out.extend_from_slice(&[0, 0, 0, 0]);
    Ok(out)
}

impl Challenge {
    pub fn parse(raw: &[u8]) -> Result<Self, Error> {
        if raw.len() > MAX_CHALLENGE_LEN {
            return Err(Error::TooLong);
        }
        if raw.get(..8) != Some(SIGNATURE.as_slice()) {
            return Err(Error::BadSignature);
        }
        if u32_at(raw, 8)? != 2 {
            return Err(Error::WrongMessageType);
        }
        let target_name = field_at(raw, 12)?.to_vec();
        let flags = u32_at(raw, 20)?;
        let mut server_challenge = [0u8; 8];
        server_challenge.copy_from_slice(raw.get(24..32).ok_or(Error::BadField)?);
        let target_info = if flags & flags::TARGET_INFO != 0 {
            parse_av_pairs(field_at(raw, 40)?)?
        } else {
            Vec::new()
        };
        Ok(Self {
            flags,
            server_challenge,
            target_name,
            target_info,
            raw: raw.to_vec(),
        })
    }

    pub fn av(&self, id: u16) -> Option<&[u8]> {
        self.target_info
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, v)| v.as_slice())
    }
}

/// Flags the client requires from the server (OB-43): NTLMv2 with extended session security,
/// 128-bit keys, key exchange, signing, Unicode and target info. Inference/design: refusing weaker
/// variants is a security choice, not a protocol rule.
pub const REQUIRED: u32 = flags::UNICODE
    | flags::SIGN
    | flags::NTLM
    | flags::EXTENDED_SESSIONSECURITY
    | flags::TARGET_INFO
    | flags::N128
    | flags::KEY_EXCH;

/// ClientConfigFlags for MS-SIPAE: Datagram, Identify and Integrity (MS-SIPAE §3.2.5.1 step 5,
/// MS-NLMP §3.1.1.2) plus the required security flags. No VERSION: the client does not claim a
/// Windows version (MS-NLMP: the Version field is then all zero).
pub const CLIENT_CONFIG: u32 =
    REQUIRED | flags::REQUEST_TARGET | flags::DATAGRAM | flags::ALWAYS_SIGN | flags::IDENTIFY;

/// Connectionless negotiation (§2.2.1.3: "the choices the client has made from the options the
/// server offered"): the offered flags restricted to the client configuration; IDENTIFY and
/// REQUEST_TARGET are client requests and always set (inference). Fails closed when a required
/// flag is not offered.
pub fn choose_flags(offered: u32) -> Result<u32, Error> {
    let missing = REQUIRED & !offered;
    if missing != 0 {
        return Err(Error::MissingFlag(missing));
    }
    Ok((offered & CLIENT_CONFIG) | flags::IDENTIFY | flags::REQUEST_TARGET)
}

/// Key material from the handshake; wiped on drop, never printed.
pub struct SessionKey([u8; 16]);

impl SessionKey {
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl Drop for SessionKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl std::fmt::Debug for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionKey(<redacted>)")
    }
}

/// Inputs of the AUTHENTICATE_MESSAGE. Random values come from the caller (CSPRNG in the adapter).
pub struct AuthenticateInput<'a> {
    pub user: &'a str,
    pub domain: &'a str,
    pub password: &'a str,
    pub workstation: &'a str,
    pub flags: u32,
    /// Version field (only with the VERSION flag; otherwise all zero).
    pub version: [u8; 8],
    pub client_challenge: [u8; 8],
    pub random_session_key: [u8; 16],
    /// Current time as FILETIME, used when the CHALLENGE carries no MsvAvTimestamp.
    pub now_filetime: u64,
    /// §3.1.5.2.1 SHOULDs: omit the LM response, add MsvAvFlags/MIC (with a timestamp), channel
    /// bindings Z(16) and an empty target name. `false` reproduces the older example §4.2.4.
    pub modern: bool,
    /// Connection-oriented NTLM (HTTP, MS-NTHT): the NEGOTIATE_MESSAGE that started the
    /// exchange; the MIC then covers NEGOTIATE ‖ CHALLENGE ‖ AUTHENTICATE (§3.1.5.1.2). `None`
    /// for connectionless NTLM (SIP, §3.1.5.2.1).
    pub negotiate: Option<&'a [u8]>,
}

/// NegotiateFlags of the NEGOTIATE_MESSAGE for connection-oriented NTLM over HTTP: the required
/// security flags (OB-43) plus REQUEST_TARGET and ALWAYS_SIGN; no DATAGRAM, no SEAL (HTTP
/// authentication only), no VERSION (no Windows version claimed).
pub const HTTP_NEGOTIATE: u32 = REQUIRED | flags::REQUEST_TARGET | flags::ALWAYS_SIGN;

/// NEGOTIATE_MESSAGE without domain, workstation or version (§2.2.1.1, pp. 21–23).
pub fn negotiate_message(flags: u32) -> Vec<u8> {
    let mut m = SIGNATURE.to_vec();
    m.extend_from_slice(&1u32.to_le_bytes());
    m.extend_from_slice(&flags.to_le_bytes());
    m.extend_from_slice(&[0; 16]); // DomainNameFields, WorkstationFields: empty
    m
}

/// Connection-oriented flag choice for HTTP: what both sides offered, required flags enforced.
pub fn choose_flags_http(offered: u32) -> Result<u32, Error> {
    let missing = REQUIRED & !offered;
    if missing != 0 {
        return Err(Error::MissingFlag(missing));
    }
    Ok(offered & HTTP_NEGOTIATE)
}

fn put_field(
    header: &mut Vec<u8>,
    payload_offset: usize,
    payload: &mut Vec<u8>,
    data: &[u8],
) -> Result<(), Error> {
    let len = u16::try_from(data.len()).map_err(|_| Error::NameTooLong)?;
    let off = u32::try_from(payload_offset + payload.len()).map_err(|_| Error::NameTooLong)?;
    header.extend_from_slice(&len.to_le_bytes());
    header.extend_from_slice(&len.to_le_bytes());
    header.extend_from_slice(&off.to_le_bytes());
    payload.extend_from_slice(data);
    Ok(())
}

/// Builds the AUTHENTICATE_MESSAGE (§2.2.1.3, §3.1.5.2.1, §3.3.2) and returns it with the
/// ExportedSessionKey. With KEY_EXCH the exported key is the random session key, otherwise the
/// KeyExchangeKey (= SessionBaseKey for NTLMv2).
pub fn authenticate(
    ch: &Challenge,
    input: &AuthenticateInput<'_>,
) -> Result<(Vec<u8>, SessionKey), Error> {
    let time = match ch.av(av::TIMESTAMP) {
        Some(t) if t.len() == 8 => {
            let mut a = [0u8; 8];
            a.copy_from_slice(t);
            a
        }
        Some(_) => return Err(Error::BadTargetInfo),
        None => input.now_filetime.to_le_bytes(),
    };
    let with_mic = input.modern && ch.av(av::TIMESTAMP).is_some();
    let mut pairs = ch.target_info.clone();
    if input.modern && !ch.target_info.is_empty() {
        if with_mic {
            match pairs.iter_mut().find(|(i, _)| *i == av::FLAGS) {
                Some((_, v)) if v.len() == 4 => {
                    let f = u32_at(v, 0)? | 0x2;
                    *v = f.to_le_bytes().to_vec();
                }
                Some(_) => return Err(Error::BadTargetInfo),
                None => pairs.push((av::FLAGS, 0x2u32.to_le_bytes().to_vec())),
            }
        }
        pairs.retain(|(i, _)| *i != av::CHANNEL_BINDINGS && *i != av::TARGET_NAME);
        pairs.push((av::CHANNEL_BINDINGS, vec![0; 16]));
        pairs.push((av::TARGET_NAME, Vec::new()));
    }
    let av_bytes = encode_av_pairs(&pairs)?;

    let mut key = ntowf_v2(input.password, input.user, input.domain);
    let temp = ntlmv2_temp(time, input.client_challenge, &av_bytes);
    let proof = nt_proof_str(&key, ch.server_challenge, &temp);
    let mut nt_response = proof.to_vec();
    nt_response.extend_from_slice(&temp);
    let lm_response = if input.modern {
        Vec::new()
    } else {
        lmv2_response(&key, ch.server_challenge, input.client_challenge).to_vec()
    };
    let mut kek = session_base_key(&key, &proof);
    key.zeroize();
    let (enc_key, mut exported) = if input.flags & flags::KEY_EXCH != 0 {
        (
            encrypt_session_key(&kek, input.random_session_key).to_vec(),
            input.random_session_key,
        )
    } else {
        (Vec::new(), kek)
    };
    kek.zeroize();

    let payload_offset = if with_mic { 0x58 } else { 0x48 };
    let mut header = Vec::with_capacity(payload_offset);
    header.extend_from_slice(SIGNATURE);
    header.extend_from_slice(&3u32.to_le_bytes());
    let mut payload = Vec::new();
    let domain = utf16le(input.domain);
    let user = utf16le(input.user);
    let workstation = utf16le(input.workstation);
    // Field descriptors in header order; payload order as in the example: domain, user,
    // workstation, LM, NT, key.
    let mut descr: [Vec<u8>; 6] = Default::default();
    for (slot, data) in [
        (2usize, &domain),
        (3, &user),
        (4, &workstation),
        (0, &lm_response),
        (1, &nt_response),
        (5, &enc_key),
    ] {
        if let Some(d) = descr.get_mut(slot) {
            put_field(d, payload_offset, &mut payload, data)?;
        }
    }
    for d in &descr {
        header.extend_from_slice(d);
    }
    header.extend_from_slice(&input.flags.to_le_bytes());
    let version = if input.flags & flags::VERSION != 0 {
        input.version
    } else {
        [0; 8]
    };
    header.extend_from_slice(&version);
    let mic_at = header.len();
    if with_mic {
        header.extend_from_slice(&[0; 16]);
    }
    let mut msg = header;
    msg.extend_from_slice(&payload);
    if with_mic {
        // MIC = HMAC_MD5(ExportedSessionKey, [NEGOTIATE ‖] CHALLENGE ‖ AUTHENTICATE)
        // (§3.1.5.1.2 connection-oriented, §3.1.5.2.1 connectionless).
        let mic = match input.negotiate {
            Some(n) => hmac_md5(&exported, &[n, &ch.raw, &msg]),
            None => hmac_md5(&exported, &[&ch.raw, &msg]),
        };
        if let Some(slot) = msg.get_mut(mic_at..mic_at + 16) {
            slot.copy_from_slice(&mic);
        }
    }
    let session_key = SessionKey(exported);
    exported.zeroize();
    Ok((msg, session_key))
}
