//! Offline security-association primitives for MS-SIPAE §§2.2.2, 3.2.1,
//! 3.2.5.1–3.2.5.2. This is NOT a GSS, NTLM or TLS-DSK handshake.
//!
//! Server messages MUST pass a real MIC verifier before their `snum` is
//! recorded. No method in this module performs network I/O, creates credentials,
//! or authorizes actual registration. Live integration needs audited key
//! derivation/GSS contexts, server identity and transaction binding.

use crate::challenge::{ChallengeOffer, ChallengeSource, Scheme};
use crate::sip::{SipMessage, StartLine};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityError {
    UnsupportedScheme, InvalidChallenge, InvalidAuthenticationInfo, DuplicateHeader,
    MissingHeader, IdentityMismatch, InvalidSignedFields, InvalidMic,
    Replay, TooOld, Exhausted, Expired, Forbidden, SigningFailed, InvalidNonce,
}
impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for SecurityError {}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SaKey {
    pub scheme: String,
    pub realm: String,
    pub targetname: String,
    pub source: AuthSource,
}
impl fmt::Debug for SaKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SaKey").field("scheme", &self.scheme)
            .field("source", &self.source).field("identity", &"[REDACTED]").finish()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthSource { Registrar, Proxy }
impl From<ChallengeSource> for AuthSource {
    fn from(value: ChallengeSource) -> Self {
        match value { ChallengeSource::Registrar => Self::Registrar, ChallengeSource::Proxy => Self::Proxy }
    }
}
fn scheme_name(s: Scheme) -> Result<&'static str, SecurityError> {
    match s {
        Scheme::Kerberos => Ok("Kerberos"),
        Scheme::Ntlm => Ok("NTLM"),
        Scheme::TlsDsk => Ok("TLS-DSK"),
        _ => Err(SecurityError::UnsupportedScheme),
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AuthenticationInfo {
    pub key: SaKey,
    pub version: u32,
    pub opaque: String,
    pub snum: u64,
    /// Keep exact on-the-wire decimal form for signed buffer construction.
    snum_text: String,
    pub srand: String,
    signature: Vec<u8>,
}
impl fmt::Debug for AuthenticationInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthenticationInfo")
            .field("scheme", &self.key.scheme)
            .field("source", &self.key.source)
            .field("version", &self.version)
            .field("snum", &self.snum)
            .field("credentials_and_mic", &"[REDACTED]")
            .finish()
    }
}
fn unique_header<'a>(msg: &'a SipMessage, name: &str) -> Result<Option<&'a str>, SecurityError> {
    let mut iter = msg.headers.iter().filter(|(n, _)| n.eq_ignore_ascii_case(name));
    let first = iter.next().map(|(_, v)| v.as_str());
    if iter.next().is_some() { return Err(SecurityError::DuplicateHeader); }
    Ok(first)
}

/// Parses the MS-SIPAE server authentication header; refuses ambiguity and
/// oversized fields rather than falling back to unauthenticated processing.
/// This is intentionally not the general SIP authentication grammar.
pub fn parse_authentication_info(msg: &SipMessage) -> Result<AuthenticationInfo, SecurityError> {
    let registrar = unique_header(msg, "Authentication-Info")?;
    let proxy = unique_header(msg, "Proxy-Authentication-Info")?;
    let (raw, source) = match (registrar, proxy) {
        (Some(raw), None) => (raw, AuthSource::Registrar),
        (None, Some(raw)) => (raw, AuthSource::Proxy),
        _ => return Err(SecurityError::InvalidAuthenticationInfo),
    };
    if raw.is_empty() || raw.len() > 4096 || !raw.is_ascii() || raw.bytes().any(|b|
        b == b'\r' || b == b'\n' || b == 0 || b == 127 || (b < 32 && b != b'\t')) {
        return Err(SecurityError::InvalidAuthenticationInfo);
    }
    let (scheme, tail) = raw.trim().split_once(char::is_whitespace)
        .ok_or(SecurityError::InvalidAuthenticationInfo)?;
    if !matches!(scheme, "Kerberos" | "NTLM" | "TLS-DSK") {
        return Err(SecurityError::UnsupportedScheme);
    }
    let fields = parse_kv(tail)?;
    let required = |name: &'static str| -> Result<&str, SecurityError> {
        fields.get(name).map(String::as_str).ok_or(SecurityError::MissingHeader)
    };
    let realm = required("realm")?;
    let targetname = required("targetname")?;
    let opaque = required("opaque")?;
    let snum_text = required("snum")?;
    if snum_text.is_empty() || !snum_text.bytes().all(|b| b.is_ascii_digit()) || snum_text.len() > 20 {
        return Err(SecurityError::InvalidAuthenticationInfo);
    }
    let snum = snum_text.parse::<u64>().map_err(|_| SecurityError::InvalidAuthenticationInfo)?;
    let srand = required("srand")?;
    if srand.len() != 8 || !srand.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(SecurityError::InvalidAuthenticationInfo);
    }
    if required("qop")? != "auth" || realm.is_empty() || targetname.is_empty() || opaque.is_empty() {
        return Err(SecurityError::InvalidAuthenticationInfo);
    }
    let version = required("version")?.parse::<u32>().map_err(|_| SecurityError::InvalidAuthenticationInfo)?;
    if version != 3 && version != 4 { return Err(SecurityError::InvalidAuthenticationInfo); }
    let signature = decode_hex(required("rspauth")?)?;
    if signature.is_empty() || signature.len() > 512 { return Err(SecurityError::InvalidAuthenticationInfo); }
    Ok(AuthenticationInfo {
        key: SaKey { scheme: scheme.into(), realm: realm.into(), targetname: targetname.into(), source },
        version, opaque: opaque.into(), snum, snum_text: snum_text.into(),
        srand: srand.into(), signature,
    })
}

fn parse_kv(s: &str) -> Result<BTreeMap<String, String>, SecurityError> {
    let mut result = BTreeMap::new();
    let (mut begin, mut quoted, mut escaped) = (0, false, false);
    let mut parts = Vec::new();
    for (i, ch) in s.bytes().enumerate() {
        if escaped { escaped = false; continue; }
        if ch == b'\\' && quoted { escaped = true; continue; }
        if ch == b'"' { quoted = !quoted; }
        if ch == b',' && !quoted { parts.push(&s[begin..i]); begin = i + 1; }
    }
    if quoted || escaped { return Err(SecurityError::InvalidAuthenticationInfo); }
    parts.push(&s[begin..]);
    if parts.len() > 24 { return Err(SecurityError::InvalidAuthenticationInfo); }
    for part in parts {
        let (key, raw) = part.trim().split_once('=').ok_or(SecurityError::InvalidAuthenticationInfo)?;
        let key = key.trim().to_ascii_lowercase();
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_lowercase() || b == b'-') {
            return Err(SecurityError::InvalidAuthenticationInfo);
        }
        let raw = raw.trim();
        let value = if raw.starts_with('"') {
            if !raw.ends_with('"') || raw.len() < 2 { return Err(SecurityError::InvalidAuthenticationInfo); }
            let mut chars = raw[1..raw.len()-1].chars();
            let mut decoded = String::new();
            while let Some(ch) = chars.next() {
                if ch == '\\' {
                    decoded.push(chars.next().ok_or(SecurityError::InvalidAuthenticationInfo)?);
                } else if ch == '"' || ch.is_control() { return Err(SecurityError::InvalidAuthenticationInfo); }
                else { decoded.push(ch); }
            }
            decoded
        } else {
            if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._~!%*+`'".contains(&b)) {
                return Err(SecurityError::InvalidAuthenticationInfo);
            }
            raw.to_string()
        };
        if result.insert(key, value).is_some() { return Err(SecurityError::InvalidAuthenticationInfo); }
    }
    Ok(result)
}
fn decode_hex(s: &str) -> Result<Vec<u8>, SecurityError> {
    if s.len() % 2 != 0 || s.len() > 1024 { return Err(SecurityError::InvalidAuthenticationInfo); }
    s.as_bytes().chunks_exact(2).map(|p| {
        let a = (p[0] as char).to_digit(16).ok_or(SecurityError::InvalidAuthenticationInfo)?;
        let b = (p[1] as char).to_digit(16).ok_or(SecurityError::InvalidAuthenticationInfo)?;
        Ok(((a << 4) | b) as u8)
    }).collect()
}

/// Lossless for only the supported MS-SIPAE name-addr subset. Unsupported
/// address forms fail closed; they are NOT signed using guessed normalization.
fn address(msg: &SipMessage, header: &str) -> Result<(String, String), SecurityError> {
    let Some(s) = unique_header(msg, header)? else { return Ok((String::new(), String::new())); };
    let start = s.find('<').ok_or(SecurityError::InvalidSignedFields)?;
    let end = s[start+1..].find('>').ok_or(SecurityError::InvalidSignedFields)? + start + 1;
    let uri = &s[start+1..end];
    if uri.is_empty() || !uri.starts_with("sip:") || uri.chars().any(|c| matches!(c, '<' | '>' | ',' | ' ' | '\t')) {
        return Err(SecurityError::InvalidSignedFields);
    }
    let suffix = &s[end+1..];
    let mut tag = String::new();
    for part in suffix.split(';').skip(1) {
        let p = part.trim();
        if let Some(v) = p.strip_prefix("tag=") {
            if !tag.is_empty() || v.is_empty() || !v.bytes().all(|b| b.is_ascii_alphanumeric() || b"-.!%*_+`'~".contains(&b)) {
                return Err(SecurityError::InvalidSignedFields);
            }
            tag = v.to_owned();
        }
    }
    Ok((uri.to_owned(), tag))
}

fn asserted(msg: &SipMessage) -> Result<(String, String), SecurityError> {
    // PAI can contain SIP and tel identities; only unambiguous angle-bracketed
    // values are accepted. Reject unfamiliar grammar, don't silently omit it.
    let mut sip = String::new();
    let mut tel = String::new();
    let asserted_present = msg.headers.iter().any(|(name, _)| name.eq_ignore_ascii_case("P-Asserted-Identity"));
    let preferred_present = msg.headers.iter().any(|(name, _)| name.eq_ignore_ascii_case("P-Preferred-Identity"));
    if asserted_present && preferred_present { return Err(SecurityError::InvalidSignedFields); }
    for (name, value) in &msg.headers {
        if !name.eq_ignore_ascii_case("P-Asserted-Identity") &&
           !name.eq_ignore_ascii_case("P-Preferred-Identity") { continue; }
        for part in value.split(',') {
            let text = part.trim();
            if !text.starts_with('<') || !text.ends_with('>') { return Err(SecurityError::InvalidSignedFields); }
            let uri = &text[1..text.len()-1];
            if uri.starts_with("sip:") && sip.is_empty() { sip = uri.to_owned(); }
            else if uri.starts_with("tel:") && tel.is_empty() { tel = uri.to_owned(); }
            else { return Err(SecurityError::InvalidSignedFields); }
        }
    }
    Ok((sip, tel))
}

/// Exact ordered angle-bracket fields described in [MS-SIPAE] §3.2.5.2.
/// Preserve case from the wire; never lower-case identifiers in the MIC input.
pub fn server_mic_buffer(msg: &SipMessage, info: &AuthenticationInfo) -> Result<Vec<u8>, SecurityError> {
    let call_id = unique_header(msg, "Call-ID")?.ok_or(SecurityError::MissingHeader)?;
    let cseq = unique_header(msg, "CSeq")?.ok_or(SecurityError::MissingHeader)?;
    let mut parts = cseq.split_ascii_whitespace();
    let number = parts.next().ok_or(SecurityError::InvalidSignedFields)?;
    let method = parts.next().ok_or(SecurityError::InvalidSignedFields)?;
    if parts.next().is_some() || number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) || method.is_empty() || !method.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err(SecurityError::InvalidSignedFields);
    }
    let (from_uri, from_tag) = address(msg, "From")?;
    let (to_uri, to_tag) = address(msg, "To")?;
    if from_uri.is_empty() || to_uri.is_empty() || call_id.is_empty() { return Err(SecurityError::MissingHeader); }
    let expires = unique_header(msg, "Expires")?.unwrap_or("");
    let mut values = vec![
        info.key.scheme.as_str(), info.srand.as_str(), info.snum_text.as_str(),
        info.key.realm.as_str(), info.key.targetname.as_str(),
        call_id, number, method, &from_uri, &from_tag,
    ];
    let (sip_identity, tel_identity) = if info.version >= 3 { asserted(msg)? }
        else { (String::new(), String::new()) };
    if info.version >= 3 {
        values.extend([to_uri.as_str(), to_tag.as_str(), sip_identity.as_str(), tel_identity.as_str()]);
    } else {
        values.push(&to_tag);
    }
    values.push(expires);
    let code = match &msg.start_line { StartLine::Response { code, .. } => Some(code.to_string()), _ => None };
    if let Some(ref c) = code { values.push(c); }
    let mut out = Vec::new();
    for value in values { out.push(b'<'); out.extend_from_slice(value.as_bytes()); out.push(b'>'); }
    Ok(out)
}

/// GSS_VerifyMic or an RFC 2104 TLS-DSK verifier can implement this interface.
/// Callers must supply a context from an authenticated handshake; this module
/// cannot evaluate whether the handshake or peer identity is trustworthy.
pub trait ServerMicVerifier {
    fn verify(&self, message: &[u8], token: &[u8]) -> bool;
}

/// RFC 2104 HMAC-SHA256 verifier for an externally supplied TLS-DSK *server*
/// authentication key. A validated TLS-DSK handshake and PRF are required to
/// derive this key (MS-SIPAE §3.2.5.1); this type does NOT derive or store one.
/// In particular, this is not SIP-over-TLS channel binding or a generic login.
pub struct TlsDskHmacSha256 {
    key: [u8; 32],
}
impl TlsDskHmacSha256 {
    pub fn from_server_authentication_key(key: [u8; 32]) -> Self { Self { key } }
}
impl Drop for TlsDskHmacSha256 {
    fn drop(&mut self) { use std::sync::atomic::{compiler_fence, Ordering};
        for b in &mut self.key { unsafe { std::ptr::write_volatile(b, 0); } }
        compiler_fence(Ordering::SeqCst);
    }
}
impl ServerMicVerifier for TlsDskHmacSha256 {
    fn verify(&self, message: &[u8], token: &[u8]) -> bool {
        let Ok(mut mac) = <Hmac<Sha256> as Mac>::new_from_slice(&self.key) else { return false; };
        mac.update(message);
        mac.verify_slice(token).is_ok()
    }
}

/// A MIC signer backed by an authenticated client-side GSS context or TLS-DSK
/// client authentication key. The returned bytes must be a binary MIC token.
/// The calling application is responsible for proving the association's peer
/// identity, and for selecting the correct signer for the SA's scheme/target.
pub trait ClientMicSigner {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SecurityError>;
}

/// Offline TLS-DSK *client* HMAC-SHA256 signer with an externally derived key.
/// This cannot establish a TLS-DSK SA or derive handshake secrets. Client and
/// server directional keys are distinct: never reuse the server verifier key.
pub struct TlsDskClientHmacSha256 { key: [u8; 32] }
impl TlsDskClientHmacSha256 {
    pub fn from_client_authentication_key(key: [u8; 32]) -> Self { Self { key } }
}
impl ClientMicSigner for TlsDskClientHmacSha256 {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SecurityError> {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.key)
            .map_err(|_| SecurityError::SigningFailed)?;
        mac.update(message);
        Ok(mac.finalize().into_bytes().to_vec())
    }
}
impl Drop for TlsDskClientHmacSha256 {
    fn drop(&mut self) {
        use std::sync::atomic::{compiler_fence, Ordering};
        for b in &mut self.key { unsafe { std::ptr::write_volatile(b, 0); } }
        compiler_fence(Ordering::SeqCst);
    }
}

/// §3.2.4.1 outgoing signature, version 3 or 4. It intentionally includes
/// the exact case-sensitive header field values and does not normalize URIs.
/// No status code/response field is appended when signing a request.
/// The caller must use an SA whose target and signing provider are trusted.
pub fn client_mic_buffer(msg: &SipMessage, key: &SaKey, version: u32,
    crand: &str, cnum: u64) -> Result<Vec<u8>, SecurityError>
{
    if !matches!(version, 3 | 4) || crand.len() != 8 ||
        !crand.bytes().all(|b| b.is_ascii_hexdigit()) || cnum == 0 {
        return Err(SecurityError::InvalidNonce);
    }
    let method = match &msg.start_line {
        StartLine::Request { method, .. } => method.as_str(),
        _ => return Err(SecurityError::InvalidSignedFields),
    };
    if unique_header(msg, "Authorization")?.is_some() ||
        unique_header(msg, "Proxy-Authorization")?.is_some() {
        return Err(SecurityError::InvalidSignedFields);
    }
    let call_id = unique_header(msg, "Call-ID")?.ok_or(SecurityError::MissingHeader)?;
    let cseq = unique_header(msg, "CSeq")?.ok_or(SecurityError::MissingHeader)?;
    let mut parts = cseq.split_ascii_whitespace();
    let number = parts.next().ok_or(SecurityError::InvalidSignedFields)?;
    let cseq_method = parts.next().ok_or(SecurityError::InvalidSignedFields)?;
    if parts.next().is_some() || number.is_empty() ||
        !number.bytes().all(|b| b.is_ascii_digit()) ||
        cseq_method != method || call_id.is_empty() {
        return Err(SecurityError::InvalidSignedFields);
    }
    let (from_uri, from_tag) = address(msg, "From")?;
    let (to_uri, to_tag) = address(msg, "To")?;
    if from_uri.is_empty() || to_uri.is_empty() {
        return Err(SecurityError::MissingHeader);
    }
    let (sip_identity, tel_identity) = asserted(msg)?;
    let expires = unique_header(msg, "Expires")?.unwrap_or("");
    let cnum_text = cnum.to_string();
    let mut values = vec![
        key.scheme.as_str(), crand, cnum_text.as_str(), key.realm.as_str(),
        key.targetname.as_str(), call_id, number, method, &from_uri, &from_tag,
    ];
    if version >= 3 {
        values.extend([to_uri.as_str(), to_tag.as_str(), sip_identity.as_str(), tel_identity.as_str()]);
    } else {
        values.push(&to_tag);
    }
    values.push(expires);
    let mut result = Vec::new();
    for value in values {
        result.push(b'<');
        result.extend_from_slice(value.as_bytes());
        result.push(b'>');
    }
    Ok(result)
}

fn authorization_value(key: &SaKey, version: u32, opaque: Option<&str>,
    crand: &str, cnum: u64, mic: &[u8]) -> Result<String, SecurityError>
{
    fn safe_quoted(value: &str) -> bool {
        value.len() <= 1024 && !value.is_empty() && value.is_ascii() &&
        value.bytes().all(|b| b >= 32 && b != 127 && b != b'"' && b != b'\\')
    }
    if !safe_quoted(&key.realm) || !safe_quoted(&key.targetname) ||
        opaque.is_some_and(|value| !safe_quoted(value)) ||
        !matches!(key.scheme.as_str(), "TLS-DSK" | "NTLM" | "Kerberos") ||
        !matches!(version, 3 | 4) || mic.is_empty() || mic.len() > 512 {
        return Err(SecurityError::InvalidSignedFields);
    }
    let mut signature = String::with_capacity(mic.len() * 2);
    for b in mic { signature.push_str(&format!("{b:02x}")); }
    let mut value = format!("{} qop=\"auth\", realm=\"{}\", targetname=\"{}\"",
        key.scheme, key.realm, key.targetname);
    if let Some(opaque) = opaque { value.push_str(&format!(", opaque=\"{opaque}\"")); }
    value.push_str(&format!(
        ", version={version}, crand=\"{crand}\", cnum=\"{cnum}\", response=\"{signature}\""
    ));
    Ok(value)
}

/// Highest accepted server number and the last 257 sequence numbers, inclusive.
/// Check only AFTER the cryptographic verification. Reject replay and stale data.
#[derive(Debug, Default)]
pub struct ReplayWindow {
    highest: Option<u64>,
    received: BTreeSet<u64>,
}
impl ReplayWindow {
    fn check(&self, number: u64) -> Result<(), SecurityError> {
        if self.received.contains(&number) { return Err(SecurityError::Replay); }
        if let Some(highest) = self.highest {
            if highest.saturating_sub(number) > 256 { return Err(SecurityError::TooOld); }
        }
        Ok(())
    }
    fn record(&mut self, number: u64) {
        let high = self.highest.map_or(number, |v| v.max(number));
        self.highest = Some(high);
        self.received.retain(|v| high.saturating_sub(*v) <= 256);
        self.received.insert(number);
    }
    pub fn highest_verified(&self) -> Option<u64> { self.highest }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationState { Establishing, Established, Expired }
/// One SA per challenged origin; key data redacted in Debug.
pub struct SecurityAssociation {
    key: SaKey,
    opaque: Option<String>,
    version: u32,
    state: AssociationState,
    expires_at_ms: u64,
    outgoing_number: u64,
    incoming: ReplayWindow,
}
impl fmt::Debug for SecurityAssociation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecurityAssociation")
            .field("scheme", &self.key.scheme)
            .field("source", &self.key.source)
            .field("state", &self.state)
            .field("identity_and_opaque", &"[REDACTED]")
            .finish()
    }
}
impl SecurityAssociation {
    pub fn begin(challenge: &ChallengeOffer, expiry_ms: u64) -> Result<Self, SecurityError> {
        let scheme = scheme_name(challenge.scheme)?.to_owned();
        let realm = challenge.realm.as_ref().filter(|s| !s.is_empty()).ok_or(SecurityError::InvalidChallenge)?;
        let target = challenge.targetname.as_ref().filter(|s| !s.is_empty()).ok_or(SecurityError::InvalidChallenge)?;
        let version = challenge.version.ok_or(SecurityError::InvalidChallenge)?;
        if !matches!(version, 3 | 4) || expiry_ms == 0 { return Err(SecurityError::InvalidChallenge); }
        if challenge.scheme == Scheme::Kerberos && !target.starts_with("sip/") {
            return Err(SecurityError::InvalidChallenge);
        }
        Ok(Self {
            key: SaKey { scheme, realm: realm.clone(), targetname: target.clone(), source: challenge.source.into() },
            opaque: challenge.opaque.clone(), version, state: AssociationState::Establishing,
            expires_at_ms: expiry_ms, outgoing_number: 0, incoming: ReplayWindow::default(),
        })
    }
    pub fn state(&self) -> AssociationState { self.state }
    pub fn key(&self) -> &SaKey { &self.key }
    pub fn highest_verified(&self) -> Option<u64> { self.incoming.highest_verified() }
    pub fn expire_if_due(&mut self, now_ms: u64) -> bool {
        if now_ms >= self.expires_at_ms { self.state = AssociationState::Expired; true } else { false }
    }
    pub fn next_outgoing_number(&mut self, now_ms: u64) -> Result<u64, SecurityError> {
        if self.state == AssociationState::Expired || self.expire_if_due(now_ms) { return Err(SecurityError::Expired); }
        if self.state != AssociationState::Established { return Err(SecurityError::InvalidChallenge); }
        self.outgoing_number = self.outgoing_number.checked_add(1).ok_or(SecurityError::Exhausted)?;
        Ok(self.outgoing_number)
    }
    /// Generate a signed outgoing SIP *draft* for an already-verified SA.
    /// Uses an OS CSPRNG for the 32-bit Microsoft crand, per MS-SIPAE §3.2.4.1.
    /// Failure must not consume cnum; the supplied request is never modified.
    /// This function deliberately does NOT transmit or authorize SIP traffic.
    pub fn sign_client_request<S: ClientMicSigner>(&mut self, request: &SipMessage,
        signer: &S, now_ms: u64) -> Result<SipMessage, SecurityError>
    {
        let mut nonce = [0_u8; 4];
        getrandom::getrandom(&mut nonce).map_err(|_| SecurityError::SigningFailed)?;
        self.sign_client_request_with_nonce(request, signer, now_ms, nonce)
    }

    // Deterministic injection for unit tests. Production callers always use
    // sign_client_request and its OS-backed, unpredictable nonce.
    fn sign_client_request_with_nonce<S: ClientMicSigner>(&mut self, request: &SipMessage,
        signer: &S, now_ms: u64, nonce: [u8; 4]) -> Result<SipMessage, SecurityError>
    {
        if self.state == AssociationState::Expired || self.expire_if_due(now_ms) {
            return Err(SecurityError::Expired);
        }
        if self.state != AssociationState::Established {
            return Err(SecurityError::InvalidChallenge);
        }
        // Do not sign malformed outbound data or smuggle pre-existing auth.
        request.encode().map_err(|_| SecurityError::InvalidSignedFields)?;
        let number = self.outgoing_number.checked_add(1).ok_or(SecurityError::Exhausted)?;
        let crand = format!("{:02x}{:02x}{:02x}{:02x}", nonce[0], nonce[1], nonce[2], nonce[3]);
        let buffer = client_mic_buffer(request, &self.key, self.version, &crand, number)?;
        let signature = signer.sign(&buffer)?;
        let header_value = authorization_value(
            &self.key, self.version, self.opaque.as_deref(), &crand, number, &signature)?;
        let name = match self.key.source {
            AuthSource::Registrar => "Authorization",
            AuthSource::Proxy => "Proxy-Authorization",
        };
        let mut draft = request.clone();
        draft.headers.push((name.into(), header_value));
        draft.encode().map_err(|_| SecurityError::InvalidSignedFields)?;
        self.outgoing_number = number;
        Ok(draft)
    }

    /// Offline validation of a signed server message. Do not call with a
    /// verifier fabricated from an unauthenticated key or a mismatched origin.
    /// Any failure keeps the replay window unchanged.
    pub fn verify_server_message<V: ServerMicVerifier>(
        &mut self, msg: &SipMessage, verifier: &V, now_ms: u64,
    ) -> Result<(), SecurityError> {
        if self.state == AssociationState::Expired || self.expire_if_due(now_ms) { return Err(SecurityError::Expired); }
        let info = parse_authentication_info(msg)?;
        if info.key != self.key || info.version != self.version ||
            self.opaque.as_deref().is_some_and(|o| o != info.opaque.as_str()) {
            return Err(SecurityError::IdentityMismatch);
        }
        let signed = server_mic_buffer(msg, &info)?;
        if !verifier.verify(&signed, &info.signature) { return Err(SecurityError::InvalidMic); }
        self.incoming.check(info.snum)?;
        if matches!(msg.start_line, StartLine::Response { code: 403, .. }) && self.state == AssociationState::Establishing {
            self.state = AssociationState::Expired;
            return Err(SecurityError::Forbidden);
        }
        self.incoming.record(info.snum);
        self.state = AssociationState::Established;
        Ok(())
    }
}

/// Hold SAs separately by (scheme, realm, targetname, UAS/proxy).
/// Insertion never promotes an SA to established.
#[derive(Default)]
pub struct AssociationTable { entries: BTreeMap<SaKey, SecurityAssociation> }
impl AssociationTable {
    pub fn insert_challenge(&mut self, offer: &ChallengeOffer, expires_at_ms: u64) -> Result<(), SecurityError> {
        let sa = SecurityAssociation::begin(offer, expires_at_ms)?;
        self.entries.insert(sa.key.clone(), sa);
        Ok(())
    }
    pub fn get_mut(&mut self, key: &SaKey) -> Option<&mut SecurityAssociation> { self.entries.get_mut(key) }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn expire_due(&mut self, now_ms: u64) {
        self.entries.retain(|_, sa| sa.state != AssociationState::Expired && !sa.expire_if_due(now_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn offer() -> ChallengeOffer {
        ChallengeOffer { scheme: Scheme::TlsDsk, source: ChallengeSource::Registrar,
            realm: Some("SIP Communications Service".into()), targetname: Some("server.example.com".into()),
            opaque: Some("opaque123".into()), version: Some(4), sts_uri: None, gssapi_data: None }
    }
    fn response(number: u64, server_key: &[u8;32], tamper: bool) -> SipMessage {
        let mut m = SipMessage { start_line: StartLine::Response { code: 200, reason: "OK".into() },
            headers: vec![
                ("Call-ID".into(), "aabb@client.invalid".into()),
                ("CSeq".into(), "8 REGISTER".into()),
                ("From".into(), "<sip:alice@example.com>;tag=abc".into()),
                ("To".into(), "<sip:alice@example.com>;tag=server".into()),
                ("Expires".into(), "3600".into()),
            ], body: Vec::new() };
        let prefix = format!("TLS-DSK realm=\"SIP Communications Service\", targetname=\"server.example.com\", opaque=\"opaque123\", qop=auth, snum={number}, srand=abcdef09, version=4, rspauth=");
        // Build input with a syntactically valid placeholder, then sign it.
        m.headers.push(("Authentication-Info".into(), format!("{prefix}00")));
        let info = parse_authentication_info(&m).unwrap();
        let signed = server_mic_buffer(&m, &info).unwrap();
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(server_key).unwrap();
        mac.update(&signed);
        let signature = mac.finalize().into_bytes();
        let mut encoded = String::new();
        for b in signature { encoded.push_str(&format!("{b:02x}")); }
        m.headers.last_mut().unwrap().1 = format!("{prefix}{encoded}");
        if tamper { m.headers[4].1 = "4200".into(); }
        m
    }
    #[test]
    fn signed_response_establishes_then_replay_is_rejected() {
        let key = [0x5a;32];
        let verifier = TlsDskHmacSha256::from_server_authentication_key(key);
        let mut sa = SecurityAssociation::begin(&offer(), 90_000).unwrap();
        assert_eq!(sa.state(), AssociationState::Establishing);
        let msg = response(200, &key, false);
        sa.verify_server_message(&msg, &verifier, 1).unwrap();
        assert_eq!(sa.state(), AssociationState::Established);
        assert_eq!(sa.highest_verified(), Some(200));
        assert_eq!(sa.verify_server_message(&msg, &verifier, 2), Err(SecurityError::Replay));
    }
    #[test]
    fn invalid_signature_never_consumes_replay_number() {
        let key = [7;32];
        let verifier = TlsDskHmacSha256::from_server_authentication_key(key);
        let mut sa = SecurityAssociation::begin(&offer(), 90_000).unwrap();
        assert_eq!(sa.verify_server_message(&response(5, &key, true), &verifier, 1), Err(SecurityError::InvalidMic));
        assert_eq!(sa.highest_verified(), None);
        sa.verify_server_message(&response(5, &key, false), &verifier, 2).unwrap();
    }
    #[test]
    fn replay_window_includes_high_minus_256_and_excludes_257() {
        let mut w = ReplayWindow::default();
        w.record(1000);
        assert!(w.check(744).is_ok());
        assert_eq!(w.check(743), Err(SecurityError::TooOld));
        w.record(744);
        assert_eq!(w.check(744), Err(SecurityError::Replay));
        w.record(1200);
        assert_eq!(w.check(744), Err(SecurityError::Replay));
    }
    #[test]
    fn rejects_wrong_origin_duplicate_and_bad_hex() {
        let key = [3;32];
        let verifier = TlsDskHmacSha256::from_server_authentication_key(key);
        let mut sa = SecurityAssociation::begin(&offer(), 90_000).unwrap();
        let mut msg = response(9, &key, false);
        let changed = msg.headers.last().unwrap().1.replace("server.example.com", "other.example.com");
        msg.headers.last_mut().unwrap().1 = changed;
        assert_eq!(sa.verify_server_message(&msg, &verifier, 1), Err(SecurityError::IdentityMismatch));
        let mut msg = response(9, &key, false);
        msg.headers.push(msg.headers.last().unwrap().clone());
        assert_eq!(parse_authentication_info(&msg), Err(SecurityError::DuplicateHeader));
        let mut msg = response(9, &key, false);
        msg.headers.last_mut().unwrap().1.push('Z');
        assert!(parse_authentication_info(&msg).is_err());
    }
    #[test]
    fn redacts_debug_and_tracks_expiry() {
        let sa = SecurityAssociation::begin(&offer(), 100).unwrap();
        assert!(!format!("{sa:?}").contains("server.example.com"));
        let mut table = AssociationTable::default();
        table.insert_challenge(&offer(), 100).unwrap();
        assert_eq!(table.len(), 1);
        table.expire_due(100);
        assert!(table.is_empty());
    }
    #[test]
    fn verified_403_revokes_an_establishing_sa() {
        let key = [0x5a; 32];
        let verify = TlsDskHmacSha256::from_server_authentication_key(key);
        let mut forbidden = response(15, &key, false);
        forbidden.start_line = StartLine::Response { code: 403, reason: "Forbidden".into() };
        let info = parse_authentication_info(&forbidden).unwrap();
        let buffer = server_mic_buffer(&forbidden, &info).unwrap();
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&key).unwrap();
        mac.update(&buffer);
        let mut signature = String::new();
        for byte in mac.finalize().into_bytes() { signature.push_str(&format!("{byte:02x}")); }
        let field = &mut forbidden.headers.last_mut().unwrap().1;
        let index = field.find("rspauth=").unwrap() + "rspauth=".len();
        field.truncate(index);
        field.push_str(&signature);
        let mut sa = SecurityAssociation::begin(&offer(), 5000).unwrap();
        assert_eq!(sa.verify_server_message(&forbidden, &verify, 1), Err(SecurityError::Forbidden));
        assert_eq!(sa.state(), AssociationState::Expired);
        assert_eq!(sa.highest_verified(), None);
        assert_eq!(sa.verify_server_message(&response(16, &key, false), &verify, 2), Err(SecurityError::Expired));
    }
    #[test]
    fn proxy_is_not_registrar_and_debug_never_exposes_key() {
        let mut proxy = offer();
        proxy.source = ChallengeSource::Proxy;
        let sa = SecurityAssociation::begin(&proxy, 5000).unwrap();
        assert_eq!(sa.key().source, AuthSource::Proxy);
        assert!(!format!("{:?}", sa.key()).contains("server.example.com"));
        let msg = response(1, &[0x5a; 32], false);
        let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
        let mut sa = SecurityAssociation::begin(&proxy, 5000).unwrap();
        assert_eq!(sa.verify_server_message(&msg, &verifier, 1), Err(SecurityError::IdentityMismatch));
    }
    fn client_draft() -> SipMessage {
        SipMessage { start_line: StartLine::Request {
            method: "REGISTER".into(), uri: "sip:example.com".into() },
            headers: vec![
                ("Via".into(), "SIP/2.0/TLS client.invalid;branch=z9hG4bKtest".into()),
                ("Call-ID".into(), "requestCallABC".into()),
                ("CSeq".into(), "9 REGISTER".into()),
                ("From".into(), "<sip:alice@example.com>;tag=localTag".into()),
                ("To".into(), "<sip:alice@example.com>".into()),
                ("Expires".into(), "3600".into()),
            ], body: Vec::new() }
    }
    fn established_sa() -> SecurityAssociation {
        let key = [0x5a; 32];
        let mut sa = SecurityAssociation::begin(&offer(), 90_000).unwrap();
        sa.verify_server_message(&response(10, &key, false),
            &TlsDskHmacSha256::from_server_authentication_key(key), 1).unwrap();
        sa
    }
    #[test]
    fn client_mic_has_microsoft_field_order_without_response_code() {
        let sa = SecurityAssociation::begin(&offer(), 90_000).unwrap();
        let value = String::from_utf8(client_mic_buffer(
            &client_draft(), sa.key(), 4, "11223344", 1).unwrap()).unwrap();
        assert_eq!(value, "<TLS-DSK><11223344><1><SIP Communications Service><server.example.com><requestCallABC><9><REGISTER><sip:alice@example.com><localTag><sip:alice@example.com><><><><3600>");
        assert!(!value.ends_with("<200>"));
    }
    #[test]
    fn outbound_signed_draft_contains_expected_mic_and_preserves_cseq() {
        let mut sa = established_sa();
        let signer = TlsDskClientHmacSha256::from_client_authentication_key([0x6b; 32]);
        let draft = client_draft();
        let signed = sa.sign_client_request_with_nonce(&draft, &signer, 2, [0x11,0x22,0x33,0x44]).unwrap();
        let header = signed.header("Authorization").unwrap();
        assert!(header.starts_with("TLS-DSK qop=\"auth\", realm=\"SIP Communications Service\""));
        assert!(header.contains("crand=\"11223344\""));
        assert!(header.contains("cnum=\"1\""));
        assert!(header.contains("version=4"));
        assert_eq!(signed.header("CSeq"), Some("9 REGISTER"));
        assert!(draft.header("Authorization").is_none());
        // The expected digest comes from a separate Python hashlib/hmac vector.
        assert!(header.contains("response=\"de04e2a668916e5aa4ef22d0e78639983a6fb38f6c6edaa50a831f111d7880e7\""));
        let wire = signed.encode().unwrap();
        let (decoded, used) = crate::sip::parse_frame(&wire).unwrap().unwrap();
        assert_eq!(used, wire.len());
        assert_eq!(decoded.header("Authorization"), Some(header));
        let next = sa.sign_client_request_with_nonce(&draft, &signer, 3, [0xaa,0xbb,0xcc,0xdd]).unwrap();
        assert!(next.header("Authorization").unwrap().contains("cnum=\"2\""));
    }
    #[test]
    fn signing_fails_closed_without_consuming_sequence_number() {
        struct Fail;
        impl ClientMicSigner for Fail {
            fn sign(&self, _: &[u8]) -> Result<Vec<u8>, SecurityError> {
                Err(SecurityError::SigningFailed)
            }
        }
        let draft = client_draft();
        let mut sa = established_sa();
        assert_eq!(sa.sign_client_request_with_nonce(&draft, &Fail, 2, [0; 4]),
            Err(SecurityError::SigningFailed));
        let mut invalid = draft.clone();
        invalid.headers.push(("Authorization".into(), "malicious-auth".into()));
        assert_eq!(sa.sign_client_request_with_nonce(
            &invalid, &TlsDskClientHmacSha256::from_client_authentication_key([1;32]), 2, [0;4]),
            Err(SecurityError::InvalidSignedFields));
        let mut mismatch = draft.clone();
        mismatch.headers.iter_mut().find(|(k, _)| k == "CSeq").unwrap().1 = "9 MESSAGE".into();
        assert_eq!(sa.sign_client_request_with_nonce(
            &mismatch, &TlsDskClientHmacSha256::from_client_authentication_key([1;32]), 2, [0;4]),
            Err(SecurityError::InvalidSignedFields));
        assert_eq!(sa.next_outgoing_number(3), Ok(1));
    }
    #[test]
    fn association_must_be_verified_before_client_signing() {
        let draft = client_draft();
        let signer = TlsDskClientHmacSha256::from_client_authentication_key([0x6b;32]);
        let mut unverified = SecurityAssociation::begin(&offer(), 90_000).unwrap();
        assert_eq!(unverified.sign_client_request_with_nonce(&draft, &signer, 1, [0;4]),
            Err(SecurityError::InvalidChallenge));
        let mut verified = established_sa();
        assert_eq!(verified.sign_client_request_with_nonce(&draft, &signer, 90_000, [0;4]),
            Err(SecurityError::Expired));
    }
    #[test]
    fn proxy_sa_must_use_proxy_authorization() {
        let mut proxy = established_sa();
        proxy.key.source = AuthSource::Proxy; // fixture-only change, not a handshake
        let signer = TlsDskClientHmacSha256::from_client_authentication_key([0x6b;32]);
        let signed = proxy.sign_client_request_with_nonce(&client_draft(), &signer, 2, [0;4]).unwrap();
        assert!(signed.header("Proxy-Authorization").is_some());
        assert!(signed.header("Authorization").is_none());
    }
    #[test]
    fn microsoft_preferred_identity_included_in_client_buffer() {
        let sa = SecurityAssociation::begin(&offer(), 90_000).unwrap();
        let mut draft = client_draft();
        draft.headers.push(("P-Preferred-Identity".into(), "<sip:alice@internal.example>".into()));
        let value = String::from_utf8(client_mic_buffer(&draft, sa.key(), 4, "00112233", 2).unwrap()).unwrap();
        assert!(value.contains("<sip:alice@internal.example><><3600>"));
        draft.headers.push(("P-Asserted-Identity".into(), "<sip:alice@internal.example>".into()));
        assert_eq!(client_mic_buffer(&draft, sa.key(), 4, "00112233", 2),
            Err(SecurityError::InvalidSignedFields));
    }
    #[test]
    fn signature_buffer_carries_exact_ms_order() {
        let key = [3;32];
        let m = response(8, &key, false);
        let info = parse_authentication_info(&m).unwrap();
        let value = String::from_utf8(server_mic_buffer(&m, &info).unwrap()).unwrap();
        assert_eq!(value, "<TLS-DSK><abcdef09><8><SIP Communications Service><server.example.com><aabb@client.invalid><8><REGISTER><sip:alice@example.com><abc><sip:alice@example.com><server><><><3600><200>");
    }
}
