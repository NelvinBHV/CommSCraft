//! Bounded, deterministic RFC 3261 section 17.1.2 client transaction model.
//!
//! **Reliable transports only (TCP/TLS)**. No retransmissions, sockets,
//! INVITE/ACK transaction logic, challenge response, or authenticated sign-in.
//! Callers must send the request separately and drive tick/receive_response.

use crate::sip::{SipMessage, StartLine};
use std::fmt;

/// RFC 3261 defaults: T1=500ms, Timer F=64*T1 for non-INVITE requests.
pub const DEFAULT_T1_MS: u64 = 500;
pub const TIMER_F_MS: u64 = 64 * DEFAULT_T1_MS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    NotRequest,
    NotResponse,
    UnsupportedMethod,
    MissingHeader(&'static str),
    DuplicateHeader(&'static str),
    InvalidVia,
    InvalidCallId,
    InvalidCSeq,
    MethodMismatch,
    ResponseMismatch,
    TimedOut,
    Closed,
}
impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for TransactionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionKey {
    pub branch: String,
    pub sent_by: String,
    /// Exact Via transport token (TCP or TLS); never conflate their responses.
    pub transport: String,
    pub call_id: String,
    pub cseq: u32,
    pub method: String,
}

fn unique<'a>(message: &'a SipMessage, long: &'static str, compact: &str)
    -> Result<&'a str, TransactionError>
{
    let mut matched = message.headers.iter().filter(|(name, _)| {
        name.eq_ignore_ascii_case(long) || name.eq_ignore_ascii_case(compact)
    });
    let result = matched.next().ok_or(TransactionError::MissingHeader(long))?;
    if matched.next().is_some() { return Err(TransactionError::DuplicateHeader(long)); }
    Ok(result.1.as_str())
}

fn parse_via(value: &str) -> Result<(String, String, String), TransactionError> {
    // A conservative subset of topmost Via grammar, sufficient for the
    // outbound requests constructed by our own client. We reject complex
    // forms instead of attempting to guess at a transaction identity.
    let top = value.split(',').next().ok_or(TransactionError::InvalidVia)?;
    let mut params = top.split(';');
    let origin = params.next().ok_or(TransactionError::InvalidVia)?.trim();
    let mut words = origin.split_ascii_whitespace();
    let version = words.next().ok_or(TransactionError::InvalidVia)?;
    let sent_by = words.next().ok_or(TransactionError::InvalidVia)?;
    if words.next().is_some()
        || !["SIP/2.0/TLS", "SIP/2.0/TCP"].iter().any(|v| version.eq_ignore_ascii_case(v))
        || sent_by.is_empty() || sent_by.bytes().any(|b| b <= 32 || b >= 127)
    {
        return Err(TransactionError::InvalidVia);
    }
    let mut branch = None;
    for param in params {
        if let Some((name, value)) = param.trim().split_once('=') {
            if name.trim().eq_ignore_ascii_case("branch") {
                if branch.is_some() || !value.starts_with("z9hG4bK") || value.len() <= 7
                    || !value.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._+!%*`'~".contains(&b))
                {
                    return Err(TransactionError::InvalidVia);
                }
                branch = Some(value.to_owned());
            }
        }
    }
    Ok((branch.ok_or(TransactionError::InvalidVia)?, sent_by.to_ascii_lowercase(), version.to_ascii_uppercase()))
}

fn parse_cseq(value: &str) -> Result<(u32, String), TransactionError> {
    let mut fields = value.split_ascii_whitespace();
    let number = fields.next().ok_or(TransactionError::InvalidCSeq)?;
    let method = fields.next().ok_or(TransactionError::InvalidCSeq)?;
    if fields.next().is_some() || method.is_empty() || !method.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(TransactionError::InvalidCSeq);
    }
    // RFC 3261 section 20.16: CSeq uses a 32-bit unsigned integer.
    Ok((number.parse::<u32>().map_err(|_| TransactionError::InvalidCSeq)?, method.to_owned()))
}

impl TransactionKey {
    pub fn from_message(message: &SipMessage) -> Result<Self, TransactionError> {
        let (branch, sent_by, transport) = parse_via(unique(message, "Via", "v")?)?;
        let call_id = unique(message, "Call-ID", "i")?;
        if call_id.is_empty() || call_id.len() > 256 || call_id.bytes().any(|b| b <= 32 || b >= 127) {
            return Err(TransactionError::InvalidCallId);
        }
        let (cseq, method) = parse_cseq(unique(message, "CSeq", "CSeq")?)?;
        Ok(Self { branch, sent_by, transport, call_id: call_id.to_owned(), cseq, method })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState { Trying, Proceeding, Terminated }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionEvent { Provisional(u16), Final(u16), TimedOut }

#[derive(Debug, Clone)]
pub struct NonInviteClientTransaction {
    key: TransactionKey,
    state: TransactionState,
    deadline_ms: u64,
}

impl NonInviteClientTransaction {
    /// `now_ms` is supplied by an external monotonic clock for reproducible tests.
    /// This constructor does not send anything.
    pub fn new(request: &SipMessage, now_ms: u64) -> Result<Self, TransactionError> {
        let method = match &request.start_line {
            StartLine::Request { method, .. } => method,
            _ => return Err(TransactionError::NotRequest),
        };
        if method == "INVITE" || method == "ACK" || method.is_empty() {
            return Err(TransactionError::UnsupportedMethod);
        }
        let key = TransactionKey::from_message(request)?;
        if method != &key.method { return Err(TransactionError::MethodMismatch); }
        Ok(Self { key, state: TransactionState::Trying,
            deadline_ms: now_ms.saturating_add(TIMER_F_MS) })
    }

    pub fn state(&self) -> TransactionState { self.state }
    pub fn deadline_ms(&self) -> u64 { self.deadline_ms }
    pub fn key(&self) -> &TransactionKey { &self.key }

    /// Processes a response only if it matches Via/Call-ID/CSeq. 401/407 are
    /// terminal *responses*; any authentication retry needs a NEW transaction.
    pub fn receive_response(&mut self, response: &SipMessage, now_ms: u64)
        -> Result<TransactionEvent, TransactionError>
    {
        if self.state == TransactionState::Terminated { return Err(TransactionError::Closed); }
        if now_ms >= self.deadline_ms {
            self.state = TransactionState::Terminated;
            return Err(TransactionError::TimedOut);
        }
        let code = match &response.start_line {
            StartLine::Response { code, .. } => *code,
            _ => return Err(TransactionError::NotResponse),
        };
        if TransactionKey::from_message(response)? != self.key {
            return Err(TransactionError::ResponseMismatch);
        }
        if (100..200).contains(&code) {
            self.state = TransactionState::Proceeding;
            Ok(TransactionEvent::Provisional(code))
        } else if (200..=699).contains(&code) {
            // Reliable transport: RFC 3261 Timer K = 0, terminate immediately.
            self.state = TransactionState::Terminated;
            Ok(TransactionEvent::Final(code))
        } else {
            Err(TransactionError::NotResponse)
        }
    }

    /// Must be called by the owning transport/event loop when the deadline passes.
    pub fn tick(&mut self, now_ms: u64) -> Option<TransactionEvent> {
        if self.state != TransactionState::Terminated && now_ms >= self.deadline_ms {
            self.state = TransactionState::Terminated;
            Some(TransactionEvent::TimedOut)
        } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sip::illustrative_register;

    fn reply(request: &SipMessage, code: u16) -> SipMessage {
        SipMessage {
            start_line: StartLine::Response { code, reason: "Test".into() },
            headers: ["Via", "Call-ID", "CSeq"].iter().map(|name| {
                (name.to_string(), request.header(name).unwrap().to_string())
            }).collect(),
            body: vec![],
        }
    }

    #[test]
    fn correlates_provisional_and_final_replies() {
        let request = illustrative_register("sip:alice@example.com").unwrap();
        let mut txn = NonInviteClientTransaction::new(&request, 100).unwrap();
        assert_eq!(txn.receive_response(&reply(&request, 100), 250), Ok(TransactionEvent::Provisional(100)));
        assert_eq!(txn.state(), TransactionState::Proceeding);
        assert_eq!(txn.receive_response(&reply(&request, 401), 260), Ok(TransactionEvent::Final(401)));
        assert_eq!(txn.state(), TransactionState::Terminated);
        assert_eq!(txn.receive_response(&reply(&request, 200), 270), Err(TransactionError::Closed));
    }

    #[test]
    fn cannot_be_spoofed_by_wrong_branch_or_cseq() {
        let request = illustrative_register("sip:alice@example.com").unwrap();
        let mut txn = NonInviteClientTransaction::new(&request, 0).unwrap();
        let mut bad_branch = reply(&request, 200);
        bad_branch.headers[0].1.push_str("-other");
        assert_eq!(txn.receive_response(&bad_branch, 100), Err(TransactionError::ResponseMismatch));
        let mut bad_cseq = reply(&request, 200);
        bad_cseq.headers[2].1 = "2 REGISTER".into();
        assert_eq!(txn.receive_response(&bad_cseq, 100), Err(TransactionError::ResponseMismatch));
        assert_eq!(txn.state(), TransactionState::Trying);
        assert_eq!(txn.receive_response(&reply(&request, 200), 100), Ok(TransactionEvent::Final(200)));
    }

    #[test]
    fn timeout_and_invalid_methods() {
        let request = illustrative_register("sip:alice@example.com").unwrap();
        let mut txn = NonInviteClientTransaction::new(&request, 0).unwrap();
        assert_eq!(txn.tick(TIMER_F_MS - 1), None);
        assert_eq!(txn.tick(TIMER_F_MS), Some(TransactionEvent::TimedOut));
        assert_eq!(txn.receive_response(&reply(&request, 200), TIMER_F_MS + 1), Err(TransactionError::Closed));
        let mut invite = request.clone();
        invite.start_line = StartLine::Request { method: "INVITE".into(), uri: "sip:bob@example.com".into() };
        assert_eq!(NonInviteClientTransaction::new(&invite, 0).unwrap_err(), TransactionError::UnsupportedMethod);
    }

    #[test]
    fn rejects_missing_and_duplicate_identity_headers() {
        let mut request = illustrative_register("sip:alice@example.com").unwrap();
        request.headers.retain(|(name, _)| name != "Call-ID");
        assert_eq!(NonInviteClientTransaction::new(&request, 0).unwrap_err(),
            TransactionError::MissingHeader("Call-ID"));
        request.headers.push(("Call-ID".into(), "first@invalid".into()));
        request.headers.push(("i".into(), "other@invalid".into()));
        assert_eq!(NonInviteClientTransaction::new(&request, 0).unwrap_err(),
            TransactionError::DuplicateHeader("Call-ID"));
    }
}
