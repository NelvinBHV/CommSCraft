//! Offline, bounded SIP IM dialog model (RFC 3261 + MS-OCSPROT §2.5.9).
//!
//! The documented Skype for Business flow is INVITE -> 200 -> ACK -> MESSAGE.
//! This subset observes the INVITE response and models plaintext MESSAGE
//! transactions. It does not implement INVITE/ACK transactions, SDP IM offer,
//! route-sets, authentication, delivery receipts or transport; its generated
//! messages are UNSIGNED offline templates and must never be sent directly.

use crate::sip::{SipMessage, StartLine};
use std::fmt;

pub const MAX_IM_TEXT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImState {
    WaitingInvite,
    NeedsAck,
    Established,
    Closed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImReceipt {
    Pending,
    AcceptedBySipPeer,
    Rejected(u16),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImError {
    InvalidIdentity,
    WrongMessage,
    MissingHeader(&'static str),
    DuplicateHeader,
    DialogMismatch,
    InvalidSequence,
    StaleSequence,
    InvalidState,
    InvalidMedia,
    InvalidText,
    TooLarge,
}
impl fmt::Display for ImError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ImError {}

fn header<'a>(m: &'a SipMessage, name: &'static str) -> Result<&'a str, ImError> {
    let mut fields = m
        .headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case(name));
    let value = fields
        .next()
        .ok_or(ImError::MissingHeader(name))?
        .1
        .as_str();
    if fields.next().is_some() {
        return Err(ImError::DuplicateHeader);
    }
    Ok(value)
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-.!%*_+`'~@".contains(&b))
}
fn uri(s: &str) -> bool {
    s.len() <= 512
        && s.starts_with("sip:")
        && s[4..].contains('@')
        && s.bytes()
            .all(|b| b > 32 && b < 127 && !b"<>\r\n,\"".contains(&b))
}
fn name_addr(s: &str) -> Result<(&str, Option<&str>), ImError> {
    let start = s.find('<').ok_or(ImError::InvalidIdentity)?;
    let end = s[start + 1..].find('>').ok_or(ImError::InvalidIdentity)? + start + 1;
    let addr = &s[start + 1..end];
    if !uri(addr) {
        return Err(ImError::InvalidIdentity);
    }
    let mut tag = None;
    for p in s[end + 1..].split(';').skip(1) {
        let p = p.trim();
        if p.get(..4).is_some_and(|v| v.eq_ignore_ascii_case("tag=")) {
            let value = &p[4..];
            if tag.is_some() || !token(value) {
                return Err(ImError::InvalidIdentity);
            }
            tag = Some(value);
        }
    }
    Ok((addr, tag))
}
fn cseq(m: &SipMessage, method: &str) -> Result<u32, ImError> {
    let value = header(m, "CSeq")?;
    let mut words = value.split_ascii_whitespace();
    let n = words.next().ok_or(ImError::InvalidSequence)?;
    if words.next() != Some(method) || words.next().is_some() || n.starts_with('+') {
        return Err(ImError::InvalidSequence);
    }
    n.parse::<u32>().map_err(|_| ImError::InvalidSequence)
}

#[derive(Debug, Clone)]
pub struct ImDialog {
    local_uri: String,
    remote_uri: String,
    local_tag: String,
    remote_tag: Option<String>,
    remote_target: Option<String>,
    call_id: String,
    invite_cseq: u32,
    next_local_cseq: u32,
    last_remote_cseq: Option<u32>,
    pending_message_cseq: Option<u32>,
    state: ImState,
}
impl ImDialog {
    pub fn new(
        local: &str,
        remote: &str,
        call_id: &str,
        local_tag: &str,
        invite_cseq: u32,
    ) -> Result<Self, ImError> {
        if !uri(local)
            || !uri(remote)
            || !token(call_id)
            || !token(local_tag)
            || invite_cseq == 0
            || invite_cseq == u32::MAX
        {
            return Err(ImError::InvalidIdentity);
        }
        Ok(Self {
            local_uri: local.into(),
            remote_uri: remote.into(),
            local_tag: local_tag.into(),
            remote_tag: None,
            remote_target: None,
            call_id: call_id.into(),
            invite_cseq,
            next_local_cseq: invite_cseq + 1,
            last_remote_cseq: None,
            pending_message_cseq: None,
            state: ImState::WaitingInvite,
        })
    }
    pub fn state(&self) -> ImState {
        self.state
    }
    pub fn pending_message_cseq(&self) -> Option<u32> {
        self.pending_message_cseq
    }

    fn check_common(&self, msg: &SipMessage, outbound_response: bool) -> Result<(), ImError> {
        if header(msg, "Call-ID")? != self.call_id {
            return Err(ImError::DialogMismatch);
        }
        let (from, from_tag) = name_addr(header(msg, "From")?)?;
        let (to, to_tag) = name_addr(header(msg, "To")?)?;
        let (expected_from, expected_to, expected_from_tag, expected_to_tag) = if outbound_response
        {
            (
                self.local_uri.as_str(),
                self.remote_uri.as_str(),
                Some(self.local_tag.as_str()),
                self.remote_tag.as_deref(),
            )
        } else {
            (
                self.remote_uri.as_str(),
                self.local_uri.as_str(),
                self.remote_tag.as_deref(),
                Some(self.local_tag.as_str()),
            )
        };
        if from != expected_from || to != expected_to || from_tag != expected_from_tag {
            return Err(ImError::DialogMismatch);
        }
        if expected_to_tag.is_some() && to_tag != expected_to_tag {
            return Err(ImError::DialogMismatch);
        }
        Ok(())
    }

    /// Apply INVITE final response after the separately managed transaction
    /// and server authentication have succeeded. Never equate this with login.
    pub fn observe_invite_response(&mut self, msg: &SipMessage) -> Result<ImState, ImError> {
        if self.state != ImState::WaitingInvite {
            return Err(ImError::InvalidState);
        }
        let code = match &msg.start_line {
            StartLine::Response { code, .. } => *code,
            _ => return Err(ImError::WrongMessage),
        };
        // Before final response the To-tag can be absent or can vary across forks.
        self.check_common(msg, true)?;
        if cseq(msg, "INVITE")? != self.invite_cseq {
            return Err(ImError::InvalidSequence);
        }
        if code < 200 {
            return Ok(self.state);
        }
        if code >= 300 {
            self.state = ImState::Closed;
            return Ok(self.state);
        }
        let (_, tag) = name_addr(header(msg, "To")?)?;
        let tag = tag.ok_or(ImError::DialogMismatch)?;
        let (target, _) = name_addr(header(msg, "Contact")?)?;
        self.remote_tag = Some(tag.into());
        self.remote_target = Some(target.into());
        self.state = ImState::NeedsAck;
        Ok(self.state)
    }

    /// ACK delivery is caller responsibility; marking it here is only an
    /// offline state transition. No network packet is created by this method.
    pub fn note_ack_sent(&mut self) -> Result<(), ImError> {
        if self.state != ImState::NeedsAck {
            return Err(ImError::InvalidState);
        }
        self.state = ImState::Established;
        Ok(())
    }

    /// Builds one UNSIGNED, offline in-dialog MESSAGE draft. Real use must
    /// insert an authenticated SA MIC and RFC 3261 route-set processing.
    pub fn offline_message(
        &mut self,
        text: &str,
        sent_by: &str,
        branch: &str,
    ) -> Result<SipMessage, ImError> {
        if self.state != ImState::Established || self.pending_message_cseq.is_some() {
            return Err(ImError::InvalidState);
        }
        if text.is_empty() || text.len() > MAX_IM_TEXT_BYTES || text.bytes().any(|c| c == 0) {
            return Err(ImError::InvalidText);
        }
        if !token(branch)
            || !branch.starts_with("z9hG4bK")
            || sent_by.is_empty()
            || sent_by.len() > 255
            || !sent_by
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b))
        {
            return Err(ImError::InvalidIdentity);
        }
        let number = self.next_local_cseq;
        let next = number.checked_add(1).ok_or(ImError::InvalidSequence)?;
        let remote_tag = self.remote_tag.as_deref().ok_or(ImError::InvalidState)?;
        let remote_target = self.remote_target.as_deref().ok_or(ImError::InvalidState)?;
        let msg = SipMessage {
            start_line: StartLine::Request {
                method: "MESSAGE".into(),
                uri: remote_target.into(),
            },
            headers: vec![
                (
                    "Via".into(),
                    format!("SIP/2.0/TLS {sent_by};branch={branch}"),
                ),
                ("Max-Forwards".into(), "70".into()),
                (
                    "From".into(),
                    format!("<{}>;tag={}", self.local_uri, self.local_tag),
                ),
                (
                    "To".into(),
                    format!("<{}>;tag={remote_tag}", self.remote_uri),
                ),
                ("Call-ID".into(), self.call_id.clone()),
                ("CSeq".into(), format!("{number} MESSAGE")),
                ("Content-Type".into(), "text/plain; charset=utf-8".into()),
            ],
            body: text.as_bytes().to_vec(),
        };
        self.next_local_cseq = next;
        self.pending_message_cseq = Some(number);
        Ok(msg)
    }

    /// 2xx = accepted by SIP peer, NOT evidence of rendering, reading, or
    /// storage. Message transaction correlation must also be enforced by the
    /// separate non-INVITE transaction layer in a future live client.
    pub fn observe_message_response(&mut self, msg: &SipMessage) -> Result<ImReceipt, ImError> {
        if self.state != ImState::Established {
            return Err(ImError::InvalidState);
        }
        let pending = self.pending_message_cseq.ok_or(ImError::InvalidState)?;
        let code = match &msg.start_line {
            StartLine::Response { code, .. } => *code,
            _ => return Err(ImError::WrongMessage),
        };
        self.check_common(msg, true)?;
        if cseq(msg, "MESSAGE")? != pending {
            return Err(ImError::InvalidSequence);
        }
        if code < 200 {
            return Ok(ImReceipt::Pending);
        }
        self.pending_message_cseq = None;
        if code < 300 {
            Ok(ImReceipt::AcceptedBySipPeer)
        } else {
            Ok(ImReceipt::Rejected(code))
        }
    }

    /// Return incoming text only after dialog, sequence and content checks.
    /// The body is never executed or interpreted as HTML.
    pub fn observe_incoming_message(&mut self, msg: &SipMessage) -> Result<String, ImError> {
        if self.state != ImState::Established {
            return Err(ImError::InvalidState);
        }
        if !matches!(&msg.start_line, StartLine::Request { method, .. } if method == "MESSAGE") {
            return Err(ImError::WrongMessage);
        }
        self.check_common(msg, false)?;
        let number = cseq(msg, "MESSAGE")?;
        if self.last_remote_cseq.is_some_and(|last| number <= last) {
            return Err(ImError::StaleSequence);
        }
        let content = header(msg, "Content-Type")?.to_ascii_lowercase();
        let mut segments = content.split(';').map(str::trim);
        if segments.next() != Some("text/plain") || segments.any(|s| s != "charset=utf-8") {
            return Err(ImError::InvalidMedia);
        }
        if msg.body.is_empty() || msg.body.len() > MAX_IM_TEXT_BYTES || msg.body.contains(&0) {
            return Err(ImError::TooLarge);
        }
        let text = std::str::from_utf8(&msg.body).map_err(|_| ImError::InvalidText)?;
        self.last_remote_cseq = Some(number);
        Ok(text.to_owned())
    }
    pub fn close(&mut self) {
        self.state = ImState::Closed;
        self.pending_message_cseq = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dialog() -> ImDialog {
        ImDialog::new(
            "sip:alice@example.org",
            "sip:bob@example.org",
            "callABC",
            "aliceTag",
            1,
        )
        .unwrap()
    }
    fn response(code: u16, method: &str, cseq: u32) -> SipMessage {
        SipMessage {
            start_line: StartLine::Response {
                code,
                reason: "OK".into(),
            },
            headers: vec![
                ("From".into(), "<sip:alice@example.org>;tag=aliceTag".into()),
                ("To".into(), "<sip:bob@example.org>;tag=bobTag".into()),
                ("Call-ID".into(), "callABC".into()),
                ("CSeq".into(), format!("{cseq} {method}")),
                ("Contact".into(), "<sip:bob@edge.example.org>".into()),
            ],
            body: Vec::new(),
        }
    }
    fn ready() -> ImDialog {
        let mut d = dialog();
        assert_eq!(
            d.observe_invite_response(&response(200, "INVITE", 1))
                .unwrap(),
            ImState::NeedsAck
        );
        d.note_ack_sent().unwrap();
        d
    }
    #[test]
    fn invite_ack_message_and_sip_acceptance_not_read_receipt() {
        let mut d = dialog();
        assert_eq!(
            d.offline_message("early", "host:5061", "z9hG4bKabc"),
            Err(ImError::InvalidState)
        );
        d.observe_invite_response(&response(200, "INVITE", 1))
            .unwrap();
        d.note_ack_sent().unwrap();
        let msg = d
            .offline_message("Grüße", "host.example.org:5061", "z9hG4bKabc")
            .unwrap();
        assert!(msg
            .encode()
            .unwrap()
            .windows(7)
            .any(|w| w == "Grüße".as_bytes()));
        assert_eq!(
            d.observe_message_response(&response(180, "MESSAGE", 2))
                .unwrap(),
            ImReceipt::Pending
        );
        assert_eq!(
            d.observe_message_response(&response(200, "MESSAGE", 2))
                .unwrap(),
            ImReceipt::AcceptedBySipPeer
        );
        assert_eq!(d.pending_message_cseq(), None);
    }
    #[test]
    fn invalid_responses_do_not_consume_pending_message() {
        let mut d = ready();
        d.offline_message("hello", "host:5061", "z9hG4bKxyz")
            .unwrap();
        assert_eq!(
            d.observe_message_response(&response(200, "MESSAGE", 3)),
            Err(ImError::InvalidSequence)
        );
        assert_eq!(d.pending_message_cseq(), Some(2));
        let mut spoof = response(200, "MESSAGE", 2);
        spoof
            .headers
            .iter_mut()
            .find(|(k, _)| k == "Call-ID")
            .unwrap()
            .1 = "other".into();
        assert_eq!(
            d.observe_message_response(&spoof),
            Err(ImError::DialogMismatch)
        );
        assert_eq!(d.pending_message_cseq(), Some(2));
        assert_eq!(
            d.observe_message_response(&response(403, "MESSAGE", 2))
                .unwrap(),
            ImReceipt::Rejected(403)
        );
    }
    #[test]
    fn malformed_and_replayed_incoming_text_is_rejected() {
        let mut d = ready();
        let mut msg = SipMessage {
            start_line: StartLine::Request {
                method: "MESSAGE".into(),
                uri: "sip:alice@example.org".into(),
            },
            headers: vec![
                ("From".into(), "<sip:bob@example.org>;tag=bobTag".into()),
                ("To".into(), "<sip:alice@example.org>;tag=aliceTag".into()),
                ("Call-ID".into(), "callABC".into()),
                ("CSeq".into(), "3 MESSAGE".into()),
                ("Content-Type".into(), "text/plain; charset=utf-8".into()),
            ],
            body: "Hallo 🌍".as_bytes().to_vec(),
        };
        assert_eq!(d.observe_incoming_message(&msg).unwrap(), "Hallo 🌍");
        assert_eq!(
            d.observe_incoming_message(&msg),
            Err(ImError::StaleSequence)
        );
        msg.headers.iter_mut().find(|(k, _)| k == "CSeq").unwrap().1 = "4 MESSAGE".into();
        msg.headers
            .iter_mut()
            .find(|(k, _)| k == "Content-Type")
            .unwrap()
            .1 = "text/html".into();
        assert_eq!(d.observe_incoming_message(&msg), Err(ImError::InvalidMedia));
        msg.headers
            .iter_mut()
            .find(|(k, _)| k == "Content-Type")
            .unwrap()
            .1 = "text/plain".into();
        assert_eq!(d.observe_incoming_message(&msg).unwrap(), "Hallo 🌍");
    }
    #[test]
    fn rejects_unsolicited_success_or_cross_dialog_messages() {
        let mut d = dialog();
        let mut r = response(200, "INVITE", 1);
        r.headers.iter_mut().find(|(k, _)| k == "From").unwrap().1 =
            "<sip:mallory@example.org>;tag=aliceTag".into();
        assert_eq!(d.observe_invite_response(&r), Err(ImError::DialogMismatch));
        assert_eq!(d.state(), ImState::WaitingInvite);
        assert!(ImDialog::new(
            "sip:alice@example.org\r\nOops",
            "sip:bob@example.org",
            "x",
            "x",
            1
        )
        .is_err());
    }
}
