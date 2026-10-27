//! MS-SIPAE message signing: security associations, signature buffers, the Authorization header
//! and verification of signed server messages with replay protection.
//!
//! Evidence: `docs/protocol/evidence/03-sipae-authentication.md` (AE-01 … AE-35).
//! Source: [MS-SIPAE] 20250520 §2.2.1–2.2.3 (pp. 13–15), §3.2.4.1 (pp. 21–22),
//! §3.2.5.2 (pp. 26–28), examples §4.1–4.2 (pp. 41–45).
//!
//! Security by design (`docs/security.md`):
//! - a server message is accepted only with a valid signature from a matching SA, a fresh `snum`
//!   and all identity headers present at most once; everything else is dropped (fail closed);
//! - values copied from a challenge into the Authorization header or the signature buffer must not
//!   contain quotes, backslashes, angle brackets or control characters, so a hostile server cannot
//!   inject header text or make two different messages produce the same buffer;
//! - protocol versions below [`Policy::min_version`] (default 3) are refused, because version 2
//!   signs neither the To URI nor P-Asserted-Identity (downgrade protection, inference);
//! - signatures are compared in constant time; key material is wiped on drop and never printed.
//!
//! The crate is sans-I/O: the caller supplies random numbers (`crand`) from a CSPRNG and the
//! clock; NTLM context establishment and Kerberos/TLS-DSK live elsewhere.

use commscraft_sip_msg::{read_quoted, Message};
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The header value is not a valid challenge or authentication-info (AE-10, AE-11).
    BadAuthHeader,
    /// A required parameter is missing.
    MissingParam(&'static str),
    /// A parameter occurs twice.
    DuplicateParam,
    /// A value contains characters that are unsafe to copy into a header or buffer.
    UnsafeValue,
    /// The server offers a protocol version below the policy minimum.
    VersionTooLow(Option<u32>),
    /// The message itself is malformed (duplicate identity header, bad CSeq, …).
    Message(commscraft_sip_msg::Error),
    /// The message carries no Authentication-Info/Proxy-Authentication-Info for this SA.
    NoMatchingSa,
    /// The signature does not verify (AE-32 step 4).
    BadSignature,
    /// `snum` is a replay or outside the window of 256 (AE-05, AE-32 step 5).
    Replay,
    /// A 403 arrived while the SA was being established: ask for other credentials (AE-33).
    CredentialsRejected,
    /// The SA is not in the state the operation needs.
    WrongState,
    /// The outgoing counter would overflow; establish a new SA.
    CounterExhausted,
    /// The MIC provider failed.
    Mic,
}

impl From<commscraft_sip_msg::Error> for Error {
    fn from(e: commscraft_sip_msg::Error) -> Self {
        Error::Message(e)
    }
}

/// Authentication protocol (AE-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Ntlm,
    Kerberos,
    TlsDsk,
}

impl Scheme {
    /// The name as used in headers and in the signature buffer (§3.2.4.1 step 2.1).
    pub fn name(self) -> &'static str {
        match self {
            Scheme::Ntlm => "NTLM",
            Scheme::Kerberos => "Kerberos",
            Scheme::TlsDsk => "TLS-DSK",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        [Scheme::Ntlm, Scheme::Kerberos, Scheme::TlsDsk]
            .into_iter()
            .find(|k| k.name().eq_ignore_ascii_case(s))
    }
}

const MAX_PARAMS: usize = 32;

/// Parameter names (lower case) and unquoted values.
pub type AuthParams = Vec<(String, String)>;

/// Splits `Scheme param=value, param="value", …` (RFC 3261 §25.1 auth-param; MS-SIPAE §2.2).
/// Quoted values are unescaped. Duplicate parameters are refused.
pub fn parse_auth_params(value: &str) -> Result<(&str, AuthParams), Error> {
    let value = value.trim();
    let sp = value.find([' ', '\t']).ok_or(Error::BadAuthHeader)?;
    let scheme = value.get(..sp).ok_or(Error::BadAuthHeader)?;
    let mut rest = value.get(sp..).ok_or(Error::BadAuthHeader)?;
    let mut params: AuthParams = Vec::new();
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let eq = rest.find('=').ok_or(Error::BadAuthHeader)?;
        let name = rest.get(..eq).ok_or(Error::BadAuthHeader)?.trim();
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::BadAuthHeader);
        }
        let after = rest.get(eq + 1..).ok_or(Error::BadAuthHeader)?.trim_start();
        let (val, tail) = if after.starts_with('"') {
            read_quoted(after).ok_or(Error::BadAuthHeader)?
        } else {
            let end = after.find([',', ' ', '\t']).unwrap_or(after.len());
            let v = after.get(..end).ok_or(Error::BadAuthHeader)?;
            if v.is_empty() || v.contains('"') {
                return Err(Error::BadAuthHeader);
            }
            (v.to_string(), after.get(end..).ok_or(Error::BadAuthHeader)?)
        };
        if params.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)) {
            return Err(Error::DuplicateParam);
        }
        if params.len() >= MAX_PARAMS {
            return Err(Error::BadAuthHeader);
        }
        params.push((name.to_ascii_lowercase(), val));
        let tail = tail.trim_start();
        rest = match tail.strip_prefix(',') {
            Some(t) => t,
            None if tail.is_empty() => tail,
            None => return Err(Error::BadAuthHeader),
        };
    }
    Ok((scheme, params))
}

fn take(params: &[(String, String)], name: &str) -> Option<String> {
    params
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.clone())
}

/// Characters that must never be copied from the network into a header or a signature buffer.
fn checked(v: String) -> Result<String, Error> {
    if v.chars().any(|c| c.is_control() || "\"\\<>".contains(c)) {
        return Err(Error::UnsafeValue);
    }
    Ok(v)
}

fn parse_version(v: Option<String>) -> Result<Option<u32>, Error> {
    match v {
        None => Ok(None),
        Some(v) if !v.is_empty() && v.len() <= 4 && v.bytes().all(|b| b.is_ascii_digit()) => {
            v.parse().map(Some).map_err(|_| Error::BadAuthHeader)
        }
        Some(_) => Err(Error::BadAuthHeader),
    }
}

/// A WWW-Authenticate or Proxy-Authenticate challenge for NTLM, Kerberos or TLS-DSK (AE-10, AE-51).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub scheme: Scheme,
    pub realm: String,
    pub targetname: String,
    pub opaque: Option<String>,
    pub gssapi_data: Option<String>,
    /// Absent before version 3 (§1.7, p. 12).
    pub version: Option<u32>,
    pub sts_uri: Option<String>,
    /// True for Proxy-Authenticate: the SA then signs with Proxy-Authorization (§3.2.4.1 step 5).
    pub proxy: bool,
}

impl Challenge {
    /// Parses one challenge. `Ok(None)` for other schemes (Digest is only used for anonymous
    /// conference join, AE-22/AE-40).
    pub fn parse(value: &str, proxy: bool) -> Result<Option<Self>, Error> {
        let (scheme, params) = parse_auth_params(value)?;
        let Some(scheme) = Scheme::parse(scheme) else {
            return Ok(None);
        };
        let realm = checked(take(&params, "realm").ok_or(Error::MissingParam("realm"))?)?;
        let targetname =
            checked(take(&params, "targetname").ok_or(Error::MissingParam("targetname"))?)?;
        if targetname.is_empty() || targetname.contains(' ') {
            return Err(Error::UnsafeValue);
        }
        // §2.2.1: the Kerberos target is "sip/" followed by the server's token.
        if scheme == Scheme::Kerberos && !targetname.starts_with("sip/") {
            return Err(Error::BadAuthHeader);
        }
        Ok(Some(Self {
            scheme,
            realm,
            targetname,
            opaque: take(&params, "opaque").map(checked).transpose()?,
            gssapi_data: take(&params, "gssapi-data").map(checked).transpose()?,
            version: parse_version(take(&params, "version"))?,
            sts_uri: take(&params, "sts-uri").map(checked).transpose()?,
            proxy,
        }))
    }
}

/// Authentication-Info or Proxy-Authentication-Info of a signed server message (AE-11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthInfo {
    pub scheme: Scheme,
    pub realm: String,
    pub targetname: String,
    /// `srand` as received (8 hex digits); used verbatim in the buffer.
    pub srand: String,
    /// `snum` as received; used verbatim in the buffer.
    pub snum_text: String,
    pub snum: u64,
    pub rspauth: Vec<u8>,
    pub opaque: Option<String>,
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || s.len() > 1024 {
        return None;
    }
    s.as_bytes()
        .chunks(2)
        .map(|p| {
            let hi = char::from(*p.first()?).to_digit(16)?;
            let lo = char::from(*p.get(1)?).to_digit(16)?;
            u8::try_from(hi * 16 + lo).ok()
        })
        .collect()
}

impl AuthInfo {
    pub fn parse(value: &str) -> Result<Self, Error> {
        let (scheme, params) = parse_auth_params(value)?;
        let scheme = Scheme::parse(scheme).ok_or(Error::BadAuthHeader)?;
        let srand = take(&params, "srand").ok_or(Error::MissingParam("srand"))?;
        if srand.len() != 8 || !srand.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::BadAuthHeader);
        }
        let snum_text = take(&params, "snum").ok_or(Error::MissingParam("snum"))?;
        if snum_text.is_empty()
            || snum_text.len() > 10
            || !snum_text.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(Error::BadAuthHeader);
        }
        let snum = snum_text.parse().map_err(|_| Error::BadAuthHeader)?;
        let rspauth = take(&params, "rspauth").ok_or(Error::MissingParam("rspauth"))?;
        Ok(Self {
            scheme,
            realm: checked(take(&params, "realm").ok_or(Error::MissingParam("realm"))?)?,
            targetname: checked(
                take(&params, "targetname").ok_or(Error::MissingParam("targetname"))?,
            )?,
            srand,
            snum_text,
            snum,
            rspauth: decode_hex(&rspauth).ok_or(Error::BadAuthHeader)?,
            opaque: take(&params, "opaque").map(checked).transpose()?,
        })
    }
}

/// Which identity headers enter fields 13/14: an outgoing request may use P-Preferred-Identity
/// (§3.2.4.1), a verified server message only P-Asserted-Identity (§3.2.5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Outgoing,
    Incoming,
}

/// Inputs of the signature buffer that come from the SA and the auth header.
#[derive(Debug, Clone, Copy)]
pub struct BufferParams<'a> {
    pub scheme: Scheme,
    /// `crand` (outgoing) or `srand` (incoming), as sent.
    pub rand: &'a str,
    /// `cnum` or `snum`, as sent.
    pub num: &'a str,
    pub realm: &'a str,
    pub targetname: &'a str,
    pub version: Option<u32>,
}

fn field(buf: &mut String, v: &str) -> Result<(), Error> {
    // A field with an angle bracket would make the buffer ambiguous (inference; fail closed).
    if v.contains(['<', '>']) {
        return Err(Error::UnsafeValue);
    }
    buf.push('<');
    buf.push_str(v);
    buf.push('>');
    Ok(())
}

/// Builds the signature buffer of §3.2.4.1 step 2 (outgoing) or §3.2.5.2 step 2 (incoming)
/// (AE-29, AE-32). Missing fields become `<>`; fields 11, 13 and 14 only from version 3 on, field
/// 16 only for responses.
pub fn signature_buffer(
    p: &BufferParams<'_>,
    msg: &Message,
    dir: Direction,
) -> Result<String, Error> {
    let v3 = p.version.is_some_and(|v| v >= 3);
    let mut b = String::new();
    field(&mut b, p.scheme.name())?;
    field(&mut b, p.rand)?;
    field(&mut b, p.num)?;
    field(&mut b, p.realm)?;
    field(&mut b, p.targetname)?;
    field(&mut b, msg.call_id()?.unwrap_or(""))?;
    let cseq = msg.cseq()?;
    field(&mut b, cseq.map_or("", |c| c.number_text))?;
    field(&mut b, cseq.map_or("", |c| c.method))?;
    let from = msg.from()?;
    field(&mut b, from.as_ref().map_or("", |f| f.uri.as_str()))?;
    field(
        &mut b,
        from.as_ref().and_then(|f| f.param("tag")).unwrap_or(""),
    )?;
    let to = msg.to()?;
    if v3 {
        field(&mut b, to.as_ref().map_or("", |t| t.uri.as_str()))?;
    }
    field(
        &mut b,
        to.as_ref().and_then(|t| t.param("tag")).unwrap_or(""),
    )?;
    if v3 {
        let mut ids = msg.name_addrs("P-Asserted-Identity")?;
        if ids.is_empty() && dir == Direction::Outgoing {
            ids = msg.name_addrs("P-Preferred-Identity")?;
        }
        let by_scheme = |s: &str| {
            ids.iter()
                .find(|a| a.scheme().is_some_and(|x| x.eq_ignore_ascii_case(s)))
                .map_or(String::new(), |a| a.uri.clone())
        };
        field(&mut b, &by_scheme("sip"))?;
        field(&mut b, &by_scheme("tel"))?;
    }
    field(&mut b, msg.expires()?.unwrap_or(""))?;
    if let Some(code) = msg.status_code() {
        field(&mut b, &code.to_string())?;
    }
    Ok(b)
}

/// Sliding window of received `snum` values (AE-05; §3.2.5.2 steps 5–6): a number more than 256
/// below the highest one seen, or seen before, is a replay.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReplayWindow {
    highest: Option<u64>,
    /// Bit i set = number `highest - i` seen (i = 0 … 256).
    seen: [u64; 5],
}

impl ReplayWindow {
    const SPAN: u64 = 256;

    fn bit(&self, i: u64) -> bool {
        let (w, b) = ((i / 64) as usize, i % 64);
        self.seen.get(w).is_some_and(|word| word & (1 << b) != 0)
    }

    fn set(&mut self, i: u64) {
        let (w, b) = ((i / 64) as usize, i % 64);
        if let Some(word) = self.seen.get_mut(w) {
            *word |= 1 << b;
        }
    }

    fn shift(&mut self, by: u64) {
        if by > Self::SPAN {
            self.seen = [0; 5];
            return;
        }
        let mut bits: Vec<u64> = (0..=Self::SPAN).filter(|i| self.bit(*i)).collect();
        self.seen = [0; 5];
        for i in bits.drain(..) {
            if i + by <= Self::SPAN {
                self.set(i + by);
            }
        }
    }

    /// Checks `snum` without recording it.
    pub fn check(&self, snum: u64) -> Result<(), Error> {
        match self.highest {
            None => Ok(()),
            Some(h) if snum > h => Ok(()),
            Some(h) if h - snum > Self::SPAN || self.bit(h - snum) => Err(Error::Replay),
            Some(_) => Ok(()),
        }
    }

    /// Records `snum` after a successful signature check.
    pub fn record(&mut self, snum: u64) -> Result<(), Error> {
        self.check(snum)?;
        match self.highest {
            Some(h) if snum <= h => self.set(h - snum),
            Some(h) => {
                self.shift(snum - h);
                self.highest = Some(snum);
                self.set(0);
            }
            None => {
                self.highest = Some(snum);
                self.set(0);
            }
        }
        Ok(())
    }
}

/// GSS_GetMIC / GSS_VerifyMIC of the authentication protocol bound to an SA (§3.2.4.1 step 3,
/// §3.2.5.2 step 3).
pub trait Mic {
    fn get_mic(&mut self, buffer: &[u8]) -> Result<Vec<u8>, Error>;
    fn verify_mic(&mut self, buffer: &[u8], token: &[u8]) -> bool;
}

/// NTLM signing for SIP: connectionless NTLM with the fixed sequence number 100 (AE-30, NL-05).
/// Holds the four keys derived from the exported session key; they are wiped on drop.
pub struct NtlmMic {
    own_sign: [u8; 16],
    own_seal: [u8; 16],
    peer_sign: [u8; 16],
    peer_seal: [u8; 16],
}

impl NtlmMic {
    /// For the client: signs with the client keys, verifies with the server keys.
    pub fn client(exported_session_key: &[u8; 16]) -> Self {
        use commscraft_ntlm::{seal_key, sign_key, Direction as D};
        Self {
            own_sign: sign_key(exported_session_key, D::ClientToServer),
            own_seal: seal_key(exported_session_key, D::ClientToServer),
            peer_sign: sign_key(exported_session_key, D::ServerToClient),
            peer_seal: seal_key(exported_session_key, D::ServerToClient),
        }
    }

    /// The server's view, used in tests to play the server.
    pub fn server(exported_session_key: &[u8; 16]) -> Self {
        let c = Self::client(exported_session_key);
        Self {
            own_sign: c.peer_sign,
            own_seal: c.peer_seal,
            peer_sign: c.own_sign,
            peer_seal: c.own_seal,
        }
    }
}

impl Drop for NtlmMic {
    fn drop(&mut self) {
        self.own_sign.zeroize();
        self.own_seal.zeroize();
        self.peer_sign.zeroize();
        self.peer_seal.zeroize();
    }
}

impl std::fmt::Debug for NtlmMic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NtlmMic { keys: <redacted> }")
    }
}

impl Mic for NtlmMic {
    fn get_mic(&mut self, buffer: &[u8]) -> Result<Vec<u8>, Error> {
        Ok(commscraft_ntlm::signature_connectionless(
            &self.own_sign,
            &self.own_seal,
            commscraft_ntlm::SIP_SEQUENCE_NUMBER,
            buffer,
        )
        .to_vec())
    }

    fn verify_mic(&mut self, buffer: &[u8], token: &[u8]) -> bool {
        let expected = commscraft_ntlm::signature_connectionless(
            &self.peer_sign,
            &self.peer_seal,
            commscraft_ntlm::SIP_SEQUENCE_NUMBER,
            buffer,
        );
        token.len() == expected.len() && bool::from(expected.ct_eq(token))
    }
}

/// Security policy of the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Lowest accepted MS-SIPAE version. Default 3 (downgrade protection; Skype for Business
    /// servers send 3 or 4, examples §4.1–4.3).
    pub min_version: u32,
}

impl Default for Policy {
    fn default() -> Self {
        Self { min_version: 3 }
    }
}

/// SA states (§3.2.1, p. 19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaState {
    Establishing,
    Established,
}

/// One client security association (§3.2.1): created from a challenge, signs outgoing messages and
/// verifies incoming ones.
pub struct SecurityAssociation<M: Mic> {
    challenge: Challenge,
    state: SaState,
    cnum: u64,
    window: ReplayWindow,
    mic: M,
}

impl<M: Mic> std::fmt::Debug for SecurityAssociation<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecurityAssociation")
            .field("scheme", &self.challenge.scheme)
            .field("realm", &self.challenge.realm)
            .field("targetname", &self.challenge.targetname)
            .field("state", &self.state)
            .field("cnum", &self.cnum)
            .finish_non_exhaustive()
    }
}

/// Result of a successful verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub snum: u64,
    /// True if this message moved the SA to "established" (AE-33).
    pub newly_established: bool,
}

impl<M: Mic> SecurityAssociation<M> {
    /// Creates the SA from the challenge that the context `mic` answered (AE-24/AE-25).
    pub fn new(challenge: Challenge, mic: M, policy: &Policy) -> Result<Self, Error> {
        match challenge.version {
            Some(v) if v >= policy.min_version => {}
            v => return Err(Error::VersionTooLow(v)),
        }
        Ok(Self {
            challenge,
            state: SaState::Establishing,
            cnum: 0,
            window: ReplayWindow::default(),
            mic,
        })
    }

    pub fn state(&self) -> SaState {
        self.state
    }

    pub fn challenge(&self) -> &Challenge {
        &self.challenge
    }

    /// Signs an outgoing message and appends the Authorization or Proxy-Authorization header
    /// (§3.2.4.1 steps 1–5, AE-29/AE-30). `crand` must come from a CSPRNG.
    pub fn sign(&mut self, msg: &mut Message, crand: u32) -> Result<(), Error> {
        if self.state != SaState::Established {
            return Err(Error::WrongState);
        }
        let cnum = self
            .cnum
            .checked_add(1)
            .filter(|n| *n <= u64::from(u32::MAX))
            .ok_or(Error::CounterExhausted)?;
        let crand = format!("{crand:08x}");
        let cnum_text = cnum.to_string();
        let c = &self.challenge;
        let buffer = signature_buffer(
            &BufferParams {
                scheme: c.scheme,
                rand: &crand,
                num: &cnum_text,
                realm: &c.realm,
                targetname: &c.targetname,
                version: c.version,
            },
            msg,
            Direction::Outgoing,
        )?;
        let token = self.mic.get_mic(buffer.as_bytes())?;
        let response: String = token.iter().map(|b| format!("{b:02x}")).collect();
        let mut value = format!(
            "{} qop=\"auth\", realm=\"{}\", targetname=\"{}\"",
            c.scheme.name(),
            c.realm,
            c.targetname
        );
        if let Some(o) = &c.opaque {
            value.push_str(&format!(", opaque=\"{o}\""));
        }
        if let Some(v) = c.version {
            value.push_str(&format!(", version={v}"));
        }
        value.push_str(&format!(
            ", crand=\"{crand}\", cnum=\"{cnum_text}\", response=\"{response}\""
        ));
        let name = if c.proxy {
            "Proxy-Authorization"
        } else {
            "Authorization"
        };
        msg.push_header(name, &value);
        self.cnum = cnum;
        Ok(())
    }

    /// Verifies a message from the server (§3.2.5.2, AE-32/AE-33). Any error means: drop the
    /// message.
    pub fn verify(&mut self, msg: &Message) -> Result<Verified, Error> {
        let header = if self.challenge.proxy {
            "Proxy-Authentication-Info"
        } else {
            "Authentication-Info"
        };
        let c = &self.challenge;
        let mut matching = None;
        for value in msg.header_values(header) {
            let Ok(info) = AuthInfo::parse(value) else {
                continue;
            };
            if info.scheme == c.scheme && info.realm == c.realm && info.targetname == c.targetname {
                if matching.is_some() {
                    // Two signatures for the same SA are ambiguous: fail closed.
                    return Err(Error::BadAuthHeader);
                }
                matching = Some(info);
            }
        }
        let info = matching.ok_or(Error::NoMatchingSa)?;
        let buffer = signature_buffer(
            &BufferParams {
                scheme: c.scheme,
                rand: &info.srand,
                num: &info.snum_text,
                realm: &c.realm,
                targetname: &c.targetname,
                version: c.version,
            },
            msg,
            Direction::Incoming,
        )?;
        self.window.check(info.snum)?;
        if !self.mic.verify_mic(buffer.as_bytes(), &info.rspauth) {
            return Err(Error::BadSignature);
        }
        self.window.record(info.snum)?;
        let mut newly_established = false;
        if self.state == SaState::Establishing {
            if msg.status_code() == Some(403) {
                return Err(Error::CredentialsRejected);
            }
            self.state = SaState::Established;
            newly_established = true;
        }
        Ok(Verified {
            snum: info.snum,
            newly_established,
        })
    }
}

#[cfg(test)]
mod tests;
