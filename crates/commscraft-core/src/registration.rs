//! Offline REGISTER planner and response observer, NOT an authenticated client.
//!
//! Normative references:
//! - RFC 3261 §§10.2.1, 10.2.2, 10.2.4, 10.3, 17.1.2, 20
//! - [MS-SIPREGE] §§3.1, 3.4, 4.1.1, 4.1.2
//! - [MS-SIPAE] §§3.2.1, 3.3.5.1, 3.3.5.2 (SA + signature validation)
//!
//! This implementation NEVER opens a socket, signs an authenticated message,
//! retries with credentials, or attests to a real server-side registration.
//! Callers must use CSPRNG-generated, stable endpoint identifiers (and a fresh
//! unpredictable Via branch for EVERY client transaction). The API deliberately
//! does not generate random identity values from timestamps or weak PRNGs.

use crate::challenge::{parse_challenges, ChallengeSource, Scheme};
use crate::sip::{SipMessage, StartLine};
use crate::security::{SecurityAssociation, SecurityError, ServerMicVerifier};
use crate::transaction::{NonInviteClientTransaction, TransactionEvent};
use std::fmt;

pub const MAX_REQUESTED_EXPIRES: u32 = 86_400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationError {
    InvalidIdentity, InvalidExpiry, InvalidState, ExhaustedCseq,
    InvalidResponse, DuplicateHeader, NoMatchingContact, AmbiguousContact,
    InvalidMinExpires, Transaction,
}
impl fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for RegistrationError {}

/// Pre-provisioned identity. Not a credential. All fields must be created by
/// trusted application code; never accept these directly from a remote peer.
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// E.g. sip:alice@example.com (deliberately conservative ASCII subset).
    pub aor: String,
    /// Via sent-by, e.g. 192.0.2.40:5061. Must be a local, routable address.
    pub sent_by: String,
    /// Contact SIP URI, e.g. sip:192.0.2.40:5061;transport=tls.
    pub contact_uri: String,
    /// 36-character UUID, used in +sip.instance; stable across refreshes.
    pub instance_uuid: String,
    /// Stable endpoint identifier, supplied as a token and used in From.
    pub epid: String,
    pub from_tag: String,
    pub call_id: String,
}

fn safe_token(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(|b|
        b.is_ascii_alphanumeric() || b"-.!%*_+`'~".contains(&b))
}
fn safe_ascii(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(|b| (33..=126).contains(&b))
}
fn valid_uuid(s: &str) -> bool {
    s.len() == 36 && s.bytes().enumerate().all(|(i, b)| {
        if [8,13,18,23].contains(&i) { b == b'-' } else { b.is_ascii_hexdigit() }
    })
}
impl Endpoint {
    pub fn validate(&self) -> Result<(), RegistrationError> {
        let Some((user, domain)) = self.aor.strip_prefix("sip:").and_then(|s| s.split_once('@')) else {
            return Err(RegistrationError::InvalidIdentity);
        };
        if !safe_token(user, 128) || !valid_domain(domain) ||
            !safe_ascii(&self.sent_by, 260) || self.sent_by.contains(&['<', '>', ',', ';', '@'][..]) ||
            !self.contact_uri.starts_with("sip:") || !safe_ascii(&self.contact_uri, 260) ||
            self.contact_uri.contains(&['<','>',',','"'][..]) ||
            !self.contact_uri.to_ascii_lowercase().ends_with(";transport=tls") ||
            !valid_uuid(&self.instance_uuid) || !safe_token(&self.epid, 64) ||
            !safe_token(&self.from_tag, 64) || !safe_ascii(&self.call_id, 256) ||
            self.call_id.contains(&['<', '>', ';', ','][..])
        { return Err(RegistrationError::InvalidIdentity); }
        Ok(())
    }
    fn domain(&self) -> &str { self.aor.split_once('@').expect("validated").1 }
    fn sip_instance(&self) -> String { format!("urn:uuid:{}", self.instance_uuid) }
}
fn valid_domain(domain: &str) -> bool {
    domain.len() <= 253 && domain.contains('.') && domain.split('.').all(|label| {
        !label.is_empty() && label.len() <= 63 && !label.starts_with('-') && !label.ends_with('-')
            && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterState {
    AwaitingResponse,
    NeedsAuthentication,
    NeedsIntervalRetry,
    /// A matching response reported binding acceptance. Authentication and
    /// Server SA signatures have NOT been verified, so this is NOT "signed in".
    UnverifiedAcceptance,
    UnverifiedUnregistration,
    /// Only the MS-SIPAE MIC and transaction were checked. This is NOT a
    /// live sign-in or an attestation of Contact/GRUU or TLS peer identity.
    MicCheckedAcceptance,
    MicCheckedUnregistration,
    Rejected,
    TimedOut,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterOutcome {
    Provisional(u16),
    Challenge { from: ChallengeSource, schemes: Vec<Scheme> },
    IntervalTooBrief { minimum_seconds: u32 },
    /// Expires is a claimed lifetime, NOT a trustworthy session.
    AcceptanceReported { expires_seconds: u32, gruu: Option<String> },
    UnregistrationReported,
    Rejected(u16),
    TimedOut,
}

/// The authenticated SIP subset of a 200 REGISTER reply has been checked,
/// but the complete Contact (including GRUU) is NOT included in the Microsoft
/// MIC input. Any supplied GRUU is therefore still only a server *claim*;
/// caller must separately enforce a trusted TLS channel and endpoint identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MicCheckedRegisterResult {
    RegistrarReportedBinding {
        expires_seconds: u32,
        /// Not covered by the MS-SIPAE MIC. Do not route with it until the
        /// transport peer and channel have been independently validated.
        untrusted_gruu: Option<String>,
        /// A local refresh heuristic at 80% of claimed lifetime, not a MUST
        /// from any RFC. Monotonic clock ticks supplied by caller.
        suggested_refresh_at_ms: u64,
        expires_at_ms: u64,
    },
    RegistrarReportedRemoval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MicCheckedRegisterError {
    InvalidState,
    NotFinalOk,
    Transaction,
    Binding(RegistrationError),
    Security(SecurityError),
}

#[derive(Debug, Clone)]
pub struct RegisterFlow {
    identity: Endpoint,
    cseq: u32,
    branch: String,
    requested_expires: u32,
    retry_min: Option<u32>,
    operation: Operation,
    request: SipMessage,
    transaction: NonInviteClientTransaction,
    state: RegisterState,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation { Register, Unregister }

impl RegisterFlow {
    /// Constructs an offline plan. Do not transmit without a correct Microsoft
    /// auth provider, service-origin verification and response MIC checking.
    pub fn new(identity: Endpoint, cseq: u32, branch: &str, requested_expires: u32,
        now_ms: u64) -> Result<Self, RegistrationError>
    {
        identity.validate()?;
        if requested_expires == 0 || requested_expires > MAX_REQUESTED_EXPIRES {
            return Err(RegistrationError::InvalidExpiry);
        }
        Self::prepare(identity, cseq, branch, requested_expires, now_ms, Operation::Register)
    }

    fn prepare(identity: Endpoint, cseq: u32, branch: &str, expires: u32,
        now_ms: u64, operation: Operation) -> Result<Self, RegistrationError>
    {
        // RFC 3261 §8.1.1.7: branch must have z9hG4bK magic cookie.
        // Caller must additionally generate it unpredictably.
        if cseq == 0 || !branch.starts_with("z9hG4bK") || branch.len() <= 15 ||
            !safe_token(branch, 128) { return Err(RegistrationError::InvalidIdentity); }
        let mut headers = vec![
            ("Via".into(), format!("SIP/2.0/TLS {};branch={branch}", identity.sent_by)),
            ("Max-Forwards".into(), "70".into()),
            ("From".into(), format!("<{}>;tag={};epid={}", identity.aor, identity.from_tag, identity.epid)),
            ("To".into(), format!("<{}>", identity.aor)),
            ("Call-ID".into(), identity.call_id.clone()),
            ("CSeq".into(), format!("{cseq} REGISTER")),
            ("Contact".into(), format!("<{}>;proxy=replace;+sip.instance=\"<{}>\"",
                identity.contact_uri, identity.sip_instance())),
            // Do not advertise unimplemented enhanced-presence capabilities.
            // gruu-10 here requests the MS-SIPREGE GRUU extension only.
            ("Supported".into(), "gruu-10".into()),
            ("Event".into(), "registration".into()),
            ("Expires".into(), expires.to_string()),
        ];
        // The protocol's User-Agent is intentionally not spoofed.
        headers.push(("User-Agent".into(), "sfb-client-rs/0.11 (research; unauthenticated)".into()));
        let request = SipMessage {
            start_line: StartLine::Request {
                method: "REGISTER".into(), uri: format!("sip:{}", identity.domain()),
            }, headers, body: Vec::new(),
        };
        let transaction = NonInviteClientTransaction::new(&request, now_ms)
            .map_err(|_| RegistrationError::Transaction)?;
        request.encode().map_err(|_| RegistrationError::InvalidIdentity)?;
        Ok(Self { identity, cseq, branch: branch.to_owned(), requested_expires: expires, retry_min: None,
            operation, request, transaction, state: RegisterState::AwaitingResponse })
    }

    pub fn request(&self) -> &SipMessage { &self.request }
    pub fn state(&self) -> RegisterState { self.state }
    pub fn cseq(&self) -> u32 { self.cseq }
    pub fn expiry_request(&self) -> u32 { self.requested_expires }

    /// RFC 3261: a 401/407 terminates this transaction; it MUST NOT be
    /// "answered" by copying a challenge into an Authorization header.
    /// Proper MS-SIPAE requires a separate GSS/NTLM/TLS-DSK security association.
    pub fn observe(&mut self, response: &SipMessage, now_ms: u64)
        -> Result<RegisterOutcome, RegistrationError>
    {
        if self.state != RegisterState::AwaitingResponse { return Err(RegistrationError::InvalidState); }
        let event = self.transaction.receive_response(response, now_ms)
            .map_err(|_| RegistrationError::Transaction)?;
        match event {
            TransactionEvent::Provisional(code) => Ok(RegisterOutcome::Provisional(code)),
            TransactionEvent::TimedOut => { self.state = RegisterState::TimedOut; Ok(RegisterOutcome::TimedOut) }
            TransactionEvent::Final(code) => self.final_response(code, response),
        }
    }

    fn final_response(&mut self, code: u16, response: &SipMessage)
        -> Result<RegisterOutcome, RegistrationError>
    {
        // Any malformed final response must leave this transaction closed.
        self.state = RegisterState::Rejected;
        match code {
            401 | 407 => {
                let offers = parse_challenges(response).map_err(|_| RegistrationError::InvalidResponse)?;
                // An unsupported-only offer cannot produce a valid new
                // authentication transaction; keep the flow terminal.
                if offers.iter().all(|offer| offer.scheme == Scheme::Unsupported) {
                    return Err(RegistrationError::InvalidResponse);
                }
                self.state = RegisterState::NeedsAuthentication;
                Ok(RegisterOutcome::Challenge {
                    from: offers[0].source,
                    schemes: offers.into_iter().map(|o| o.scheme).collect(),
                })
            }
            423 if self.operation == Operation::Register => {
                let min = unique_header(response, "Min-Expires")?
                    .ok_or(RegistrationError::InvalidMinExpires)?;
                let min = parse_seconds(min).ok_or(RegistrationError::InvalidMinExpires)?;
                if min <= self.requested_expires || min > MAX_REQUESTED_EXPIRES {
                    self.state = RegisterState::Rejected;
                    return Err(RegistrationError::InvalidMinExpires);
                }
                self.retry_min = Some(min);
                self.state = RegisterState::NeedsIntervalRetry;
                Ok(RegisterOutcome::IntervalTooBrief { minimum_seconds: min })
            }
            200 if self.operation == Operation::Unregister => {
                self.state = RegisterState::UnverifiedUnregistration;
                Ok(RegisterOutcome::UnregistrationReported)
            }
            200 => {
                let binding = matching_binding(response, &self.identity)?;
                self.state = RegisterState::UnverifiedAcceptance;
                Ok(RegisterOutcome::AcceptanceReported {
                    expires_seconds: binding.0, gruu: binding.1,
                })
            }
            _ => { self.state = RegisterState::Rejected; Ok(RegisterOutcome::Rejected(code)) }
        }
    }

    /// Offline integration seam for an existing **trusted** MS-SIPAE SA.
    ///
    /// Source: RFC 3261 §§10.2.4/17.1.2, MS-SIPAE §3.2.5.2 and
    /// MS-SIPREGE §4.1.1. FIRST check transaction identity and parse the
    /// claimed binding without mutating state; THEN verify the real MIC and
    /// replay window; ONLY THEN commit the response. An invalid response or
    /// failed MIC consumes neither the transaction nor the replay number.
    ///
    /// IMPORTANT: a MIC does not cover the response's Contact or GRUU.
    /// The caller MUST independently verify the TLS peer and channel before
    /// using the reported URI or treating this as a real registration.
    /// A synthetic verifier is appropriate only for offline tests.
    pub fn observe_mic_checked_200<V: ServerMicVerifier>(
        &mut self, response: &SipMessage, sa: &mut SecurityAssociation,
        verifier: &V, now_ms: u64,
    ) -> Result<MicCheckedRegisterResult, MicCheckedRegisterError> {
        if self.state != RegisterState::AwaitingResponse {
            return Err(MicCheckedRegisterError::InvalidState);
        }
        // Check the full topmost Via/Call-ID/CSeq correlation without consuming
        // the transaction, even when a forged MIC is present.
        let mut candidate = self.transaction.clone();
        let event = candidate.receive_response(response, now_ms)
            .map_err(|_| MicCheckedRegisterError::Transaction)?;
        if event != TransactionEvent::Final(200) {
            return Err(MicCheckedRegisterError::NotFinalOk);
        }
        let result = if self.operation == Operation::Unregister {
            MicCheckedRegisterResult::RegistrarReportedRemoval
        } else {
            let (expires_seconds, untrusted_gruu) = matching_binding(response, &self.identity)
                .map_err(MicCheckedRegisterError::Binding)?;
            // A Contact-specific expiration is not covered by the MS-SIPAE
            // MIC. As a fail-closed first subset, require it to agree with
            // the signed, mandatory Expires field (MS-SIPAE §3.2.5.2 #15).
            // More general RFC 3261 combinations need separate evaluation.
            let signed_lifetime = unique_header(response, "Expires")
                .map_err(MicCheckedRegisterError::Binding)?
                .and_then(parse_seconds)
                .ok_or(MicCheckedRegisterError::Binding(RegistrationError::InvalidResponse))?;
            if expires_seconds != signed_lifetime || expires_seconds == 0 {
                return Err(MicCheckedRegisterError::Binding(RegistrationError::InvalidResponse));
            }
            let lifetime_ms = u64::from(expires_seconds) * 1_000;
            MicCheckedRegisterResult::RegistrarReportedBinding {
                expires_seconds,
                untrusted_gruu,
                suggested_refresh_at_ms: now_ms.saturating_add(lifetime_ms * 4 / 5),
                expires_at_ms: now_ms.saturating_add(lifetime_ms),
            }
        };
        // This MUST happen before committing `candidate`, since an invalid
        // MIC or replay makes the entire response unusable (MS-SIPAE).
        sa.verify_server_message(response, verifier, now_ms)
            .map_err(MicCheckedRegisterError::Security)?;
        self.transaction = candidate;
        self.state = match self.operation {
            Operation::Register => RegisterState::MicCheckedAcceptance,
            Operation::Unregister => RegisterState::MicCheckedUnregistration,
        };
        Ok(result)
    }

    /// Returns a NEW independent transaction after an RFC 3261 423 response.
    /// Does not send the message; the caller supplies a fresh random branch.
    pub fn plan_interval_retry(&self, new_branch: &str, now_ms: u64)
        -> Result<Self, RegistrationError>
    {
        if self.state != RegisterState::NeedsIntervalRetry { return Err(RegistrationError::InvalidState); }
        if new_branch == self.branch { return Err(RegistrationError::InvalidIdentity); }
        Self::prepare(self.identity.clone(), self.cseq.checked_add(1).ok_or(RegistrationError::ExhaustedCseq)?,
            new_branch, self.retry_min.ok_or(RegistrationError::InvalidState)?, now_ms, Operation::Register)
    }

    /// MS-SIPAE 3.2.5.1 / RFC 3261 10.2: a challenge retry is a NEW
    /// REGISTER transaction, preserving Call-ID/endpoint but incrementing CSeq
    /// and using a fresh unpredictable Via branch. A separate GSS provider
    /// must add Authorization before any network transmission.
    pub fn plan_authentication_retry(&self, new_branch: &str, now_ms: u64)
        -> Result<Self, RegistrationError>
    {
        if self.state != RegisterState::NeedsAuthentication {
            return Err(RegistrationError::InvalidState);
        }
        if new_branch == self.branch { return Err(RegistrationError::InvalidIdentity); }
        Self::prepare(self.identity.clone(),
            self.cseq.checked_add(1).ok_or(RegistrationError::ExhaustedCseq)?,
            new_branch, self.requested_expires, now_ms, self.operation)
    }

    /// A refresh is allowed for *planning* only. A production client must
    /// ensure the previous acceptance was authenticated BEFORE transmission.
    pub fn plan_refresh(&self, new_branch: &str, now_ms: u64)
        -> Result<Self, RegistrationError>
    {
        if !matches!(self.state, RegisterState::UnverifiedAcceptance | RegisterState::MicCheckedAcceptance) {
            return Err(RegistrationError::InvalidState);
        }
        if new_branch == self.branch { return Err(RegistrationError::InvalidIdentity); }
        Self::prepare(self.identity.clone(), self.cseq.checked_add(1).ok_or(RegistrationError::ExhaustedCseq)?,
            new_branch, self.requested_expires, now_ms, Operation::Register)
    }

    /// MS-SIPREGE §4.1.2: Contact binding unregistration uses Expires: 0,
    /// not Contact: * (which would remove other devices' bindings).
    pub fn plan_unregistration(&self, new_branch: &str, now_ms: u64)
        -> Result<Self, RegistrationError>
    {
        if !matches!(self.state, RegisterState::UnverifiedAcceptance | RegisterState::MicCheckedAcceptance) {
            return Err(RegistrationError::InvalidState);
        }
        if new_branch == self.branch { return Err(RegistrationError::InvalidIdentity); }
        Self::prepare(self.identity.clone(), self.cseq.checked_add(1).ok_or(RegistrationError::ExhaustedCseq)?,
            new_branch, 0, now_ms, Operation::Unregister)
    }

    pub fn tick(&mut self, now_ms: u64) -> Option<RegisterOutcome> {
        if self.state == RegisterState::AwaitingResponse && self.transaction.tick(now_ms).is_some() {
            self.state = RegisterState::TimedOut;
            Some(RegisterOutcome::TimedOut)
        } else { None }
    }
}

fn unique_header<'a>(msg: &'a SipMessage, name: &str)
    -> Result<Option<&'a str>, RegistrationError>
{
    let mut it = msg.headers.iter().filter(|(k,_)| k.eq_ignore_ascii_case(name));
    let first = it.next();
    if it.next().is_some() { return Err(RegistrationError::DuplicateHeader); }
    Ok(first.map(|(_,v)| v.as_str()))
}
fn parse_seconds(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) { return None; }
    s.parse().ok()
}

// Split Contact header fields on delimiters outside quotes and <URI>.
// No RFC 3261 URI equivalence or full header grammar is claimed here.
fn split_outside(s: &str, sep: u8) -> Result<Vec<&str>, RegistrationError> {
    if s.len() > 16 * 1024 || s.bytes().any(|b| b == b'\r' || b == b'\n' || b == 0) {
        return Err(RegistrationError::InvalidResponse);
    }
    let (mut quoted, mut escape, mut angle, mut begin) = (false, false, false, 0);
    let mut parts = Vec::new();
    for (i, ch) in s.bytes().enumerate() {
        if escape { escape = false; continue; }
        if quoted && ch == b'\\' { escape = true; continue; }
        if ch == b'"' { quoted = !quoted; continue; }
        if !quoted {
            if ch == b'<' { if angle { return Err(RegistrationError::InvalidResponse); } angle = true; }
            if ch == b'>' { if !angle { return Err(RegistrationError::InvalidResponse); } angle = false; }
            if ch == sep && !angle { parts.push(s[begin..i].trim()); begin = i + 1; }
        }
    }
    if quoted || escape || angle { return Err(RegistrationError::InvalidResponse); }
    parts.push(s[begin..].trim());
    if parts.len() > 32 { return Err(RegistrationError::InvalidResponse); }
    Ok(parts)
}
fn unquote(s: &str) -> Result<&str, RegistrationError> {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { Ok(&s[1..s.len()-1]) }
    else if s.contains('"') { Err(RegistrationError::InvalidResponse) }
    else { Ok(s) }
}

fn matching_binding(msg: &SipMessage, endpoint: &Endpoint)
    -> Result<(u32, Option<String>), RegistrationError>
{
    // Directly constructed SipMessage values must be bounded too; not all
    // callers necessarily used the bounded SIP wire parser.
    if msg.headers.len() > 128 || msg.headers.iter().any(|(_, v)| v.len() > 16 * 1024) {
        return Err(RegistrationError::InvalidResponse);
    }
    let default_expiry = unique_header(msg, "Expires")?
        .map(|s| parse_seconds(s).ok_or(RegistrationError::InvalidResponse))
        .transpose()?;
    let mut found: Option<(u32, Option<String>)> = None;
    for (key, value) in &msg.headers {
        if !key.eq_ignore_ascii_case("Contact") && !key.eq_ignore_ascii_case("m") { continue; }
        for contact in split_outside(value, b',')? {
            let start = contact.find('<').ok_or(RegistrationError::InvalidResponse)?;
            let end = contact[start+1..].find('>').ok_or(RegistrationError::InvalidResponse)? + start + 1;
            if !contact[..start].trim().is_empty() { // Display name permitted, don't interpret it.
                if contact[..start].len() > 256 { return Err(RegistrationError::InvalidResponse); }
            }
            // Untrusted Contact URI must be syntactically plausible before
            // endpoint-instance matching; a forged instance is not enough.
            let uri = &contact[start+1..end];
            if !uri.starts_with("sip:") || !safe_ascii(uri, 2048)
                || uri.contains(&['<', '>', '"', ','][..]) {
                return Err(RegistrationError::InvalidResponse);
            }
            let params = contact[end+1..].trim();
            if !params.is_empty() && !params.starts_with(';') { return Err(RegistrationError::InvalidResponse); }
            let mut instance = None;
            let mut expires = None;
            let mut gruu = None;
            for param in split_outside(params.strip_prefix(';').unwrap_or(""), b';')? {
                if param.is_empty() { continue; }
                if let Some((k,v)) = param.split_once('=') {
                    let value = unquote(v.trim())?;
                    if k.trim().eq_ignore_ascii_case("+sip.instance") {
                        if instance.replace(value.to_owned()).is_some() { return Err(RegistrationError::AmbiguousContact); }
                    } else if k.trim().eq_ignore_ascii_case("expires") {
                        if expires.replace(parse_seconds(value).ok_or(RegistrationError::InvalidResponse)?).is_some() {
                            return Err(RegistrationError::AmbiguousContact);
                        }
                    } else if k.trim().eq_ignore_ascii_case("gruu")
                        && (!value.starts_with("sip:") || !safe_ascii(value, 2048) || gruu.replace(value.to_owned()).is_some())
                    {
                        return Err(RegistrationError::InvalidResponse);
                    }
                }
            }
            // MS-SIPREGE identifies the endpoint by its +sip.instance and EPID.
            // We deliberately require instance match (not ambiguous URI string
            // matching) before associating a returned binding with our client.
            if instance.as_deref().map(|s| s.trim_matches(&['<','>'][..]).eq_ignore_ascii_case(&endpoint.sip_instance()))
                == Some(true)
            {
                let seconds = expires.or(default_expiry).ok_or(RegistrationError::InvalidResponse)?;
                if seconds == 0 || found.replace((seconds, gruu)).is_some() {
                    return Err(RegistrationError::AmbiguousContact);
                }
            }
        }
    }
    found.ok_or(RegistrationError::NoMatchingContact)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint() -> Endpoint {
        Endpoint {
            aor: "sip:alice@example.com".into(), sent_by: "192.0.2.50:5061".into(),
            contact_uri: "sip:192.0.2.50:5061;transport=tls".into(),
            instance_uuid: "b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97".into(),
            epid: "492a7ce35f".into(), from_tag: "2d0be03279".into(),
            call_id: "74c55d45a6ee404680aa55c8fe126f11".into(),
        }
    }
    const BRANCH1: &str = "z9hG4bK-branch-00000001";
    const BRANCH2: &str = "z9hG4bK-branch-00000002";

    fn flow() -> RegisterFlow {
        RegisterFlow::new(endpoint(), 3, BRANCH1, 3600, 100).unwrap()
    }
    fn reply(flow: &RegisterFlow, code: u16, extras: &[(&str, &str)]) -> SipMessage {
        let mut headers = ["Via", "Call-ID", "CSeq"].iter().map(|k|
            (k.to_string(), flow.request().header(k).unwrap().to_owned())).collect::<Vec<_>>();
        headers.extend(extras.iter().map(|(k,v)| (k.to_string(), v.to_string())));
        SipMessage {
            start_line: StartLine::Response { code, reason: "Fixture".into() },
            headers, body: Vec::new(),
        }
    }
    #[test]
    fn microsoft_register_has_required_identity_and_capability() {
        let f = flow();
        let m = f.request();
        assert_eq!(m.header("Event"), Some("registration"));
        assert_eq!(m.header("Supported"), Some("gruu-10"));
        assert!(m.header("From").unwrap().contains(";epid=492a7ce35f"));
        assert!(m.header("Contact").unwrap().contains("+sip.instance=\"<urn:uuid:"));
        assert_eq!(m.header("Expires"), Some("3600"));
        assert_eq!(m.header("CSeq"), Some("3 REGISTER"));
        assert!(!m.headers.iter().any(|(k,_)| k == "Authorization"));
        assert!(std::str::from_utf8(&m.encode().unwrap()).unwrap().ends_with("Content-Length: 0\r\n\r\n"));
    }
    #[test]
    fn ms_sipae_challenge_cannot_be_auto_authenticated() {
        let mut f = flow();
        let response = reply(&f, 401, &[
            ("WWW-Authenticate", "NTLM realm=\"SIP Communications Service\", targetname=\"sip.example.com\", version=3"),
            ("WWW-Authenticate", "Kerberos realm=\"SIP Communications Service\", targetname=\"sip/sip.example.com\", version=3"),
        ]);
        assert_eq!(f.observe(&response, 200).unwrap(), RegisterOutcome::Challenge {
            from: ChallengeSource::Registrar, schemes: vec![Scheme::Ntlm, Scheme::Kerberos],
        });
        assert_eq!(f.state(), RegisterState::NeedsAuthentication);
        assert!(matches!(f.plan_refresh(BRANCH2, 300), Err(RegistrationError::InvalidState)));
    }
    #[test]
    fn handles_407_proxy_authentication() {
        let mut f = flow();
        let response = reply(&f, 407, &[("Proxy-Authenticate",
            "TLS-DSK realm=\"SIP Communications Service\", targetname=\"sip.example.com\", version=4")]);
        assert_eq!(f.observe(&response, 200).unwrap(), RegisterOutcome::Challenge {
            from: ChallengeSource::Proxy, schemes: vec![Scheme::TlsDsk],
        });
    }
    #[test]
    fn unsupported_only_401_407_challenges_fail_closed() {
        for (code, header) in [(401, "WWW-Authenticate"), (407, "Proxy-Authenticate")] {
            let mut f = flow();
            let response = reply(&f, code, &[(header, "UnknownAuth realm=\"r\"")]);
            assert_eq!(f.observe(&response, 200), Err(RegistrationError::InvalidResponse));
            assert_eq!(f.state(), RegisterState::Rejected);
            assert!(matches!(f.plan_authentication_retry(BRANCH2, 300), Err(RegistrationError::InvalidState)));
        }
    }
    #[test]
    fn supported_407_challenge_retry_retains_unregistration_intent() {
        let mut f = flow();
        let contact = format!("<sip:198.51.100.25>;expires=3600;+sip.instance=\"<urn:uuid:{}>\"", endpoint().instance_uuid);
        let ok = reply(&f, 200, &[("Contact", &contact)]);
        assert!(matches!(f.observe(&ok, 200), Ok(RegisterOutcome::AcceptanceReported { .. })));
        let mut unreg = f.plan_unregistration(BRANCH2, 300).unwrap();
        assert_eq!(unreg.request().header("Expires"), Some("0"));
        let challenge = reply(&unreg, 407, &[("Proxy-Authenticate", "TLS-DSK realm=\"r\", targetname=\"sip.example.com\", version=4")]);
        assert!(matches!(unreg.observe(&challenge, 400), Ok(RegisterOutcome::Challenge { .. })));
        let branch3 = "z9hG4bK-branch-00000003";
        let retry = unreg.plan_authentication_retry(branch3, 500).unwrap();
        assert_eq!(retry.request().header("Expires"), Some("0"));
        assert_eq!(retry.request().header("CSeq"), Some("5 REGISTER"));
        assert_ne!(retry.request().header("Via"), unreg.request().header("Via"));
        assert!(!retry.request().headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("Authorization")));
    }
    #[test]
    fn rfc3261_423_rejects_duplicate_min_expires_fail_closed() {
        let mut f = flow();
        let response = reply(&f, 423, &[("Min-Expires", "7200"), ("Min-Expires", "8000")]);
        assert_eq!(f.observe(&response, 200), Err(RegistrationError::DuplicateHeader));
        assert_eq!(f.state(), RegisterState::Rejected);
        assert!(matches!(f.plan_interval_retry(BRANCH2, 300), Err(RegistrationError::InvalidState)));
    }
    #[test]
    fn rfc3261_423_rejects_invalid_or_nonincreasing_min_expires() {
        for value in ["0", "3600", "not-a-number", "86401"] {
            let mut f = flow();
            let response = reply(&f, 423, &[("Min-Expires", value)]);
            assert_eq!(f.observe(&response, 200), Err(RegistrationError::InvalidMinExpires));
            assert_eq!(f.state(), RegisterState::Rejected);
        }
    }
    #[test]
    fn rfc_423_creates_new_transaction_with_new_cseq_and_branch() {
        let mut f = flow();
        let provisional = reply(&f, 100, &[]);
        assert_eq!(f.observe(&provisional, 150).unwrap(), RegisterOutcome::Provisional(100));
        let r423 = reply(&f, 423, &[("Min-Expires", "7200")]);
        assert_eq!(f.observe(&r423, 200).unwrap(), RegisterOutcome::IntervalTooBrief { minimum_seconds: 7200 });
        assert_eq!(f.state(), RegisterState::NeedsIntervalRetry);
        let f2 = f.plan_interval_retry(BRANCH2, 300).unwrap();
        assert_eq!(f2.request().header("Call-ID"), f.request().header("Call-ID"));
        assert_eq!(f2.request().header("CSeq"), Some("4 REGISTER"));
        assert!(f2.request().header("Via").unwrap().contains(BRANCH2));
        assert_eq!(f2.request().header("Expires"), Some("7200"));
        assert_eq!(f2.state(), RegisterState::AwaitingResponse);
        assert!(matches!(f.plan_interval_retry(BRANCH1, 300), Err(RegistrationError::InvalidIdentity)));
    }
    #[test]
    fn rfc3261_423_retry_recovery_is_new_independent_transaction() {
        let mut first = flow();
        let response = reply(&first, 423, &[("Min-Expires", "7200")]);
        assert_eq!(first.observe(&response, 200).unwrap(),
            RegisterOutcome::IntervalTooBrief { minimum_seconds: 7200 });
        let retry = first.plan_interval_retry(BRANCH2, 300).unwrap();
        assert_eq!(retry.state(), RegisterState::AwaitingResponse);
        assert_eq!(retry.request().header("Expires"), Some("7200"));
        assert_ne!(retry.request().header("Via"), first.request().header("Via"));
        assert_eq!(retry.request().header("Call-ID"), first.request().header("Call-ID"));
        assert!(matches!(first.plan_refresh(BRANCH2, 301), Err(RegistrationError::InvalidState)));
    }
    #[test]
    fn registration_timeout_disallows_refresh_or_retry_without_new_flow() {
        let mut flow = flow();
        assert_eq!(flow.tick(100 + crate::transaction::TIMER_F_MS), Some(RegisterOutcome::TimedOut));
        assert!(matches!(flow.plan_refresh(BRANCH2, 500), Err(RegistrationError::InvalidState)));
        assert!(matches!(flow.plan_interval_retry(BRANCH2, 500), Err(RegistrationError::InvalidState)));
        assert!(matches!(flow.plan_authentication_retry(BRANCH2, 500), Err(RegistrationError::InvalidState)));
    }
    #[test]
    fn rejects_invalid_or_overlarge_min_expires() {
        for val in ["0", "3600", "invalid", "86401", " 7200 7"] {
            let mut f = flow();
            let response = reply(&f, 423, &[("Min-Expires", val)]);
            assert_eq!(f.observe(&response, 200),
                Err(RegistrationError::InvalidMinExpires));
        }
    }
    #[test]
    fn microsoft_gruu_and_instance_match_only_after_200() {
        let mut f = flow();
        let contact = "<sip:198.51.100.25:5061;transport=tls;ms-received-cid=AB>;expires=7200;\
             +sip.instance=\"<urn:uuid:B43B3D1D-9F8F-5FDC-9F74-3CA273CADB97>\";\
             gruu=\"sip:alice@example.com;opaque=user:epid:ABcd;gruu\"";
        let response = reply(&f, 200, &[("Contact", contact), ("Expires", "3600")]);
        assert_eq!(f.observe(&response, 200).unwrap(), RegisterOutcome::AcceptanceReported {
            expires_seconds: 7200,
            gruu: Some("sip:alice@example.com;opaque=user:epid:ABcd;gruu".into()),
        });
        assert_eq!(f.state(), RegisterState::UnverifiedAcceptance);
        let refresher = f.plan_refresh(BRANCH2, 300).unwrap();
        assert_eq!(refresher.request().header("CSeq"), Some("4 REGISTER"));
        let unregister = f.plan_unregistration(BRANCH2, 300).unwrap();
        assert_eq!(unregister.request().header("Expires"), Some("0"));
        assert!(unregister.request().header("Contact").unwrap().starts_with('<'));
        assert_ne!(unregister.request().header("Contact"), Some("*"));
    }
    #[test]
    fn do_not_confuse_another_devices_binding_with_ours() {
        let mut f = flow();
        let response = reply(&f, 200, &[("Contact", "<sip:192.0.2.60:5061>;expires=3600;+sip.instance=\"<urn:uuid:aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa>\"")]);
        assert_eq!(f.observe(&response, 200), Err(RegistrationError::NoMatchingContact));
        assert_ne!(f.state(), RegisterState::UnverifiedAcceptance);
    }
    #[test]
    fn rejects_duplicate_own_contact_and_missing_expiration() {
        let own = "<sip:198.51.100.25>;expires=3600;+sip.instance=\"<urn:uuid:b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97>\"";
        let mut f = flow();
        let both = format!("{own}, {own}");
        let response = reply(&f, 200, &[("Contact", &both)]);
        assert_eq!(f.observe(&response, 200),
            Err(RegistrationError::AmbiguousContact));
        let mut f2 = flow();
        let own_no_expiry = "<sip:198.51.100.25>;+sip.instance=\"<urn:uuid:b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97>\"";
        let response = reply(&f2, 200, &[("Contact", own_no_expiry)]);
        assert_eq!(f2.observe(&response, 200),
            Err(RegistrationError::InvalidResponse));
    }
    #[test]
    fn oversized_direct_response_headers_are_rejected_without_acceptance() {
        let mut f = flow();
        let own = "<sip:198.51.100.25>;expires=3600;+sip.instance=\"<urn:uuid:b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97>\"";
        let mut response = reply(&f, 200, &[("Contact", own)]);
        for _ in 0..129 { response.headers.push(("X-Filler".into(), "x".into())); }
        assert_eq!(f.observe(&response, 200), Err(RegistrationError::InvalidResponse));
        assert_eq!(f.state(), RegisterState::Rejected);
    }
    #[test]
    fn oversized_contact_header_is_rejected_without_acceptance() {
        let mut f = flow();
        let long = format!("<sip:host.example.com>;+sip.instance=\"<urn:uuid:{}>\";unused={}",
            endpoint().instance_uuid, "a".repeat(16 * 1024));
        let response = reply(&f, 200, &[("Contact", &long)]);
        assert_eq!(f.observe(&response, 200), Err(RegistrationError::InvalidResponse));
        assert_eq!(f.state(), RegisterState::Rejected);
    }
    #[test]
    fn auth_retry_cseq_boundary_rejects_out_of_range_new_transaction() {
        let mut f = RegisterFlow::new(endpoint(), 2_147_483_647, BRANCH1, 3600, 100).unwrap();
        let challenge = reply(&f, 401, &[("WWW-Authenticate",
            "NTLM realm=\"r\", targetname=\"sip.example.com\", version=3")]);
        assert!(matches!(f.observe(&challenge, 200), Ok(RegisterOutcome::Challenge { .. })));
        assert!(matches!(f.plan_authentication_retry(BRANCH2, 300),
            Err(RegistrationError::Transaction)));
    }
    #[test]
    fn malformed_contact_uri_cannot_claim_our_instance() {
        let instance = endpoint().instance_uuid;
        for uri in ["", "https://evil.example", "sip:evil.example<spoof>"] {
            let mut f = flow();
            let contact = format!("<{uri}>;expires=3600;+sip.instance=\"<urn:uuid:{instance}>\"");
            let response = reply(&f, 200, &[("Contact", &contact)]);
            assert_eq!(f.observe(&response, 200), Err(RegistrationError::InvalidResponse));
            assert_eq!(f.state(), RegisterState::Rejected);
        }
    }
    #[test]
    fn valid_contact_uri_can_report_unverified_binding() {
        let mut f = flow();
        let contact = format!("<sip:198.51.100.25:5061;transport=tls>;expires=3600;+sip.instance=\"<urn:uuid:{}>\"", endpoint().instance_uuid);
        let response = reply(&f, 200, &[("Contact", &contact)]);
        assert_eq!(f.observe(&response, 200), Ok(RegisterOutcome::AcceptanceReported {
            expires_seconds: 3600, gruu: None
        }));
        assert_eq!(f.state(), RegisterState::UnverifiedAcceptance);
    }
    #[test]
    fn response_matching_precedes_state_change() {
        let mut f = flow();
        let mut response = reply(&f, 401, &[("WWW-Authenticate", "NTLM realm=\"r\", targetname=\"s\", version=3")]);
        response.headers[2].1 = "99 REGISTER".into();
        assert_eq!(f.observe(&response, 200), Err(RegistrationError::Transaction));
        assert_eq!(f.state(), RegisterState::AwaitingResponse);
    }
    #[test]
    fn rfc_timer_f_expires() {
        let mut f = flow();
        assert_eq!(f.tick(100), None);
        assert_eq!(f.tick(100 + crate::transaction::TIMER_F_MS), Some(RegisterOutcome::TimedOut));
        assert_eq!(f.state(), RegisterState::TimedOut);
    }
    #[test]
    fn offline_discovery_register_challenge_refresh_unregistration_end_to_end() {
        use crate::discovery::{DiscoveryPlan, Service, SrvRecord, SrvRandom, CandidateOrigin};
        struct ZeroDraw;
        impl SrvRandom for ZeroDraw {
            fn choose_inclusive(&mut self, _maximum: u32) -> u32 { 0 }
        }
        let discovery = DiscoveryPlan::from_sip_uri("sip:alice@example.com").unwrap();
        let records = vec![SrvRecord::new(0, 1, 5061, "registrar.example.com").unwrap()];
        let targets = discovery.tls_candidates(&[
            (Service::InternalTcp, vec![SrvRecord::new(0, 1, 5060, "plaintext.example.com").unwrap()]),
            (Service::InternalTls, records),
        ], &mut ZeroDraw);
        assert_eq!(targets[0].host, "registrar.example.com");
        assert_eq!(targets[0].port, 5061);
        assert_eq!(targets[0].origin, CandidateOrigin::Srv(Service::InternalTls));
        assert!(targets.iter().all(|c| c.host != "plaintext.example.com"));

        let mut registration = flow();
        let first = reply(&registration, 401, &[("WWW-Authenticate",
            "Kerberos realm=\"r\", targetname=\"sip/registrar.example.com\", version=3")]);
        assert!(matches!(registration.observe(&first, 200),
            Ok(RegisterOutcome::Challenge { from: ChallengeSource::Registrar, .. })));
        assert_eq!(registration.state(), RegisterState::NeedsAuthentication);
        assert!(matches!(registration.observe(&first, 201), Err(RegistrationError::InvalidState)));

        let mut retry = registration.plan_authentication_retry(BRANCH2, 300).unwrap();
        assert_eq!(retry.request().header("CSeq"), Some("4 REGISTER"));
        assert!(!retry.request().headers.iter().any(|(k,_)| k == "Authorization"));
        let binding = format!("<sip:198.51.100.25:5061;transport=tls>;expires=3600;+sip.instance=\"<urn:uuid:{}>\"",
            endpoint().instance_uuid);
        let ok = reply(&retry, 200, &[("Contact", &binding)]);
        assert!(matches!(retry.observe(&ok, 400), Ok(RegisterOutcome::AcceptanceReported { expires_seconds: 3600, .. })));
        assert_eq!(retry.state(), RegisterState::UnverifiedAcceptance);
        assert_eq!(retry.observe(&ok, 401), Err(RegistrationError::InvalidState));

        const BRANCH3: &str = "z9hG4bK-branch-00000003";
        const BRANCH4: &str = "z9hG4bK-branch-00000004";
        const BRANCH5: &str = "z9hG4bK-branch-00000005";
        let mut refresh = retry.plan_refresh(BRANCH3, 500).unwrap();
        assert_eq!(refresh.request().header("CSeq"), Some("5 REGISTER"));
        assert_eq!(refresh.request().header("Expires"), Some("3600"));
        let refreshed = reply(&refresh, 200, &[("Contact", &binding)]);
        assert!(matches!(refresh.observe(&refreshed, 600), Ok(RegisterOutcome::AcceptanceReported { .. })));

        let mut remove = refresh.plan_unregistration(BRANCH4, 700).unwrap();
        assert_eq!(remove.request().header("Expires"), Some("0"));
        assert_ne!(remove.request().header("Contact"), Some("*"));
        let proxy = reply(&remove, 407, &[("Proxy-Authenticate",
            "TLS-DSK realm=\"r\", targetname=\"registrar.example.com\", version=4")]);
        assert!(matches!(remove.observe(&proxy, 800), Ok(RegisterOutcome::Challenge { from: ChallengeSource::Proxy, .. })));
        let mut remove_retry = remove.plan_authentication_retry(BRANCH5, 900).unwrap();
        assert_eq!(remove_retry.request().header("Expires"), Some("0"));
        assert_eq!(remove_retry.request().header("CSeq"), Some("7 REGISTER"));
        let removed = reply(&remove_retry, 200, &[]);
        assert_eq!(remove_retry.observe(&removed, 1000), Ok(RegisterOutcome::UnregistrationReported));
        assert_eq!(remove_retry.state(), RegisterState::UnverifiedUnregistration);
        assert!(matches!(remove_retry.plan_refresh(BRANCH3, 1100), Err(RegistrationError::InvalidState)));
    }

    #[test]
    fn offline_discovery_register_timeout_and_replayed_response_fail_closed() {
        use crate::discovery::{DiscoveryPlan, Service, SrvRecord, SrvRandom};
        struct ZeroDraw;
        impl SrvRandom for ZeroDraw {
            fn choose_inclusive(&mut self, _maximum: u32) -> u32 { 0 }
        }
        let discovery = DiscoveryPlan::from_domain("example.com").unwrap();
        let candidates = discovery.tls_candidates(&[
            (Service::InternalTcp, vec![SrvRecord::new(0, 1, 5060, "plaintext.example.com").unwrap()]),
            (Service::InternalTls, vec![SrvRecord::new(0, 0, 5061, ".").unwrap()]),
        ], &mut ZeroDraw);
        assert_eq!(candidates.len(), 3);
        assert!(candidates.iter().all(|x| x.port == 443));
        let mut registration = flow();
        let challenge = reply(&registration, 401, &[("WWW-Authenticate",
            "NTLM realm=\"r\", targetname=\"sip.example.com\", version=3")]);
        assert!(matches!(registration.observe(&challenge, 200), Ok(RegisterOutcome::Challenge { .. })));
        let mut retry = registration.plan_authentication_retry(BRANCH2, 300).unwrap();
        assert_eq!(retry.tick(300 + crate::transaction::TIMER_F_MS), Some(RegisterOutcome::TimedOut));
        assert_eq!(retry.observe(&reply(&retry, 200, &[]), 300 + crate::transaction::TIMER_F_MS + 1),
            Err(RegistrationError::InvalidState));
        assert!(matches!(retry.plan_refresh(BRANCH1, 40000), Err(RegistrationError::InvalidState)));
        assert!(matches!(retry.plan_unregistration(BRANCH1, 40000), Err(RegistrationError::InvalidState)));
    }
    #[test]
    fn validates_identity_and_input_instead_of_interpolating_unsafe_text() {
        let mut p = endpoint();
        p.aor = "sip:alice@example.com\r\nInjected: 1".into();
        assert!(matches!(RegisterFlow::new(p, 1, BRANCH1, 3600, 0), Err(RegistrationError::InvalidIdentity)));
        assert!(matches!(RegisterFlow::new(endpoint(), 1, "not-branch", 3600, 0), Err(RegistrationError::InvalidIdentity)));
        assert!(matches!(RegisterFlow::new(endpoint(), 0, BRANCH1, 3600, 0), Err(RegistrationError::InvalidIdentity)));
    }
}
