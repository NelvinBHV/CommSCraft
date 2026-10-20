//! Offline subset of RFC 6665 / RFC 3856 and MS-PRES PIDF subscription handling.
//!
//! Only ordinary `SUBSCRIBE` and `NOTIFY` are modeled. Microsoft BENOTIFY,
//! enhanced category subscription, XML parsing, authentication, publications,
//! routing and contacts synchronization are intentionally NOT implemented.
//! SIP headers received here must first be authenticated by the caller in a
//! future live client. This module does no I/O and does not authorize traffic.

use crate::sip::{SipMessage, StartLine};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionState {
    Pending,
    Active,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceError {
    InvalidIdentity,
    WrongMessage,
    MissingHeader(&'static str),
    DuplicateHeader,
    DialogMismatch,
    InvalidCSeq,
    StaleCSeq,
    InvalidEvent,
    InvalidState,
    UnsupportedMedia,
    Oversize,
}
impl fmt::Display for PresenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PresenceError {}

fn header<'a>(msg: &'a SipMessage, name: &'static str) -> Result<&'a str, PresenceError> {
    let mut items = msg
        .headers
        .iter()
        .filter(|(n, _)| n.eq_ignore_ascii_case(name));
    let value = items
        .next()
        .ok_or(PresenceError::MissingHeader(name))?
        .1
        .as_str();
    if items.next().is_some() {
        return Err(PresenceError::DuplicateHeader);
    }
    Ok(value)
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-.!%*_+`'~@".contains(&c))
}
fn uri(s: &str) -> bool {
    s.len() <= 512
        && s.starts_with("sip:")
        && s[4..].contains('@')
        && s.bytes()
            .all(|b| b > 32 && b < 127 && !b"<>\r\n,\"".contains(&b))
}
fn name_addr(s: &str) -> Result<(&str, Option<&str>), PresenceError> {
    let begin = s.find('<').ok_or(PresenceError::InvalidIdentity)?;
    let end = s[begin + 1..]
        .find('>')
        .ok_or(PresenceError::InvalidIdentity)?
        + begin
        + 1;
    let addr = &s[begin + 1..end];
    if !uri(addr) {
        return Err(PresenceError::InvalidIdentity);
    }
    let mut tag = None;
    for p in s[end + 1..].split(';').skip(1) {
        let p = p.trim();
        if p.get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("tag="))
        {
            let value = &p[4..];
            if tag.is_some() || !token(value) {
                return Err(PresenceError::InvalidIdentity);
            }
            tag = Some(value);
        }
    }
    Ok((addr, tag))
}
fn cseq(value: &str, method: &str) -> Result<u32, PresenceError> {
    let mut p = value.split_ascii_whitespace();
    let number = p.next().ok_or(PresenceError::InvalidCSeq)?;
    if p.next() != Some(method) || p.next().is_some() || number.starts_with('+') {
        return Err(PresenceError::InvalidCSeq);
    }
    number
        .parse::<u32>()
        .map_err(|_| PresenceError::InvalidCSeq)
}
fn event_is_presence(raw: &str) -> bool {
    raw.split(';')
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case("presence")
}
fn parse_state(raw: &str) -> Result<(SubscriptionState, Option<u32>), PresenceError> {
    let mut p = raw.split(';');
    let value = p.next().unwrap_or("").trim();
    let state = if value.eq_ignore_ascii_case("active") {
        SubscriptionState::Active
    } else if value.eq_ignore_ascii_case("pending") {
        SubscriptionState::Pending
    } else if value.eq_ignore_ascii_case("terminated") {
        SubscriptionState::Terminated
    } else {
        return Err(PresenceError::InvalidState);
    };
    let mut expires = None;
    for param in p {
        let param = param.trim();
        if let Some((k, v)) = param.split_once('=') {
            if k.trim().eq_ignore_ascii_case("expires") {
                if expires.is_some() || v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(PresenceError::InvalidState);
                }
                expires = Some(v.parse::<u32>().map_err(|_| PresenceError::InvalidState)?);
            }
        }
    }
    if state == SubscriptionState::Terminated && expires.is_some() {
        return Err(PresenceError::InvalidState);
    }
    Ok((state, expires))
}

#[derive(Debug, Clone)]
pub struct PresenceSubscription {
    pub call_id: String,
    pub local_uri: String,
    pub remote_uri: String,
    pub local_tag: String,
    pub remote_tag: Option<String>,
    pub state: SubscriptionState,
    pub expires: Option<u32>,
    pub last_notify_cseq: Option<u32>,
}
impl PresenceSubscription {
    pub fn new(
        call_id: &str,
        local_uri: &str,
        remote_uri: &str,
        local_tag: &str,
    ) -> Result<Self, PresenceError> {
        if !token(call_id) || !token(local_tag) || !uri(local_uri) || !uri(remote_uri) {
            return Err(PresenceError::InvalidIdentity);
        }
        Ok(Self {
            call_id: call_id.into(),
            local_uri: local_uri.into(),
            remote_uri: remote_uri.into(),
            local_tag: local_tag.into(),
            remote_tag: None,
            state: SubscriptionState::Pending,
            expires: None,
            last_notify_cseq: None,
        })
    }

    /// Local test template; it is NOT signed and MUST NOT be sent directly.
    /// RFC 3856's PIDF branch uses `Accept: application/pidf+xml` and does not
    /// advertise Microsoft BENOTIFY support until that extension exists.
    pub fn offline_subscribe(
        &self,
        via: &str,
        branch: &str,
        contact: &str,
        cseq_number: u32,
        expires: u32,
    ) -> Result<SipMessage, PresenceError> {
        if !token(branch)
            || !branch.starts_with("z9hG4bK")
            || !uri(contact)
            || via.is_empty()
            || via.len() > 255
            || !via
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b))
            || cseq_number == 0
            || expires > 86400
        {
            return Err(PresenceError::InvalidIdentity);
        }
        Ok(SipMessage {
            start_line: StartLine::Request {
                method: "SUBSCRIBE".into(),
                uri: self.remote_uri.clone(),
            },
            headers: vec![
                ("Via".into(), format!("SIP/2.0/TLS {via};branch={branch}")),
                ("Max-Forwards".into(), "70".into()),
                (
                    "From".into(),
                    format!("<{}>;tag={}", self.local_uri, self.local_tag),
                ),
                ("To".into(), format!("<{}>", self.remote_uri)),
                ("Call-ID".into(), self.call_id.clone()),
                ("CSeq".into(), format!("{cseq_number} SUBSCRIBE")),
                ("Contact".into(), format!("<{contact}>")),
                ("Event".into(), "presence".into()),
                ("Accept".into(), "application/pidf+xml".into()),
                ("Expires".into(), expires.to_string()),
            ],
            body: Vec::new(),
        })
    }

    /// Accepts an already authenticated SIP NOTIFY as a protocol observation.
    /// RFC 6665 allows NOTIFY to arrive before the final SUBSCRIBE response.
    /// No state is mutated until every check succeeds.
    pub fn observe_notify(&mut self, msg: &SipMessage) -> Result<SubscriptionState, PresenceError> {
        if !matches!(&msg.start_line, StartLine::Request { method, .. } if method == "NOTIFY") {
            return Err(PresenceError::WrongMessage);
        }
        if self.state == SubscriptionState::Terminated {
            return Err(PresenceError::InvalidState);
        }
        if header(msg, "Call-ID")? != self.call_id {
            return Err(PresenceError::DialogMismatch);
        }
        let (to, to_tag) = name_addr(header(msg, "To")?)?;
        let (from, from_tag) = name_addr(header(msg, "From")?)?;
        if to != self.local_uri
            || to_tag != Some(self.local_tag.as_str())
            || from != self.remote_uri
        {
            return Err(PresenceError::DialogMismatch);
        }
        let remote_tag = from_tag.ok_or(PresenceError::DialogMismatch)?;
        if self
            .remote_tag
            .as_deref()
            .is_some_and(|tag| tag != remote_tag)
        {
            return Err(PresenceError::DialogMismatch);
        }
        if !event_is_presence(header(msg, "Event")?) {
            return Err(PresenceError::InvalidEvent);
        }
        let sequence = cseq(header(msg, "CSeq")?, "NOTIFY")?;
        if self.last_notify_cseq.is_some_and(|last| sequence <= last) {
            return Err(PresenceError::StaleCSeq);
        }
        let (state, expires) = parse_state(header(msg, "Subscription-State")?)?;
        if msg.body.len() > 128 * 1024 {
            return Err(PresenceError::Oversize);
        }
        if !msg.body.is_empty() {
            let media = header(msg, "Content-Type")?;
            let mime = media.split(';').next().unwrap_or("").trim();
            if ![
                "application/pidf+xml",
                "application/xpidf+xml",
                "text/xml+msrtc.pidf",
            ]
            .iter()
            .any(|v| mime.eq_ignore_ascii_case(v))
            {
                return Err(PresenceError::UnsupportedMedia);
            }
        }
        // Body is deliberately opaque here. Parsing categories requires a
        // separate bounded XML implementation with container privacy rules.
        self.remote_tag = Some(remote_tag.into());
        self.last_notify_cseq = Some(sequence);
        self.state = state;
        self.expires = expires;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sub() -> PresenceSubscription {
        PresenceSubscription::new(
            "call123",
            "sip:alice@example.org",
            "sip:bob@example.org",
            "local123",
        )
        .unwrap()
    }
    fn notify(cseq: u32, state: &str) -> SipMessage {
        SipMessage {
            start_line: StartLine::Request {
                method: "NOTIFY".into(),
                uri: "sip:alice@example.org".into(),
            },
            headers: vec![
                ("Call-ID".into(), "call123".into()),
                ("To".into(), "<sip:alice@example.org>;tag=local123".into()),
                ("From".into(), "<sip:bob@example.org>;tag=remote45".into()),
                ("CSeq".into(), format!("{cseq} NOTIFY")),
                ("Event".into(), "presence".into()),
                ("Subscription-State".into(), state.into()),
                ("Content-Type".into(), "application/pidf+xml".into()),
            ],
            body: b"<presence/>".to_vec(),
        }
    }
    #[test]
    fn creates_unsigned_but_valid_subscribe() {
        let msg = sub()
            .offline_subscribe(
                "client.example.org:5061",
                "z9hG4bKxyz",
                "sip:alice@client.example.org",
                1,
                3600,
            )
            .unwrap();
        let wire = msg.encode().unwrap();
        assert!(wire.starts_with(b"SUBSCRIBE sip:bob@example.org SIP/2.0\r\n"));
        assert!(!wire.windows(11).any(|x| x == b"ms-benotify"));
    }
    #[test]
    fn receives_initial_notify_before_subscribe_response_and_rejects_replays() {
        let mut s = sub();
        assert_eq!(
            s.observe_notify(&notify(1, "active;expires=300")).unwrap(),
            SubscriptionState::Active
        );
        assert_eq!(s.expires, Some(300));
        assert_eq!(
            s.observe_notify(&notify(1, "active;expires=300")),
            Err(PresenceError::StaleCSeq)
        );
        assert_eq!(s.last_notify_cseq, Some(1));
        assert_eq!(
            s.observe_notify(&notify(2, "terminated;reason=timeout"))
                .unwrap(),
            SubscriptionState::Terminated
        );
        assert_eq!(
            s.observe_notify(&notify(3, "active")),
            Err(PresenceError::InvalidState)
        );
    }
    #[test]
    fn mismatched_or_unsupported_notify_does_not_advance_state() {
        let mut s = sub();
        let mut n = notify(1, "active");
        n.headers.iter_mut().find(|(k, _)| k == "Event").unwrap().1 = "conference".into();
        assert_eq!(s.observe_notify(&n), Err(PresenceError::InvalidEvent));
        assert_eq!(s.last_notify_cseq, None);
        n = notify(1, "active");
        n.headers.iter_mut().find(|(k, _)| k == "From").unwrap().1 =
            "<sip:mallory@example.org>;tag=remote45".into();
        assert_eq!(s.observe_notify(&n), Err(PresenceError::DialogMismatch));
        n = notify(1, "active");
        n.headers
            .iter_mut()
            .find(|(k, _)| k == "Content-Type")
            .unwrap()
            .1 = "text/html".into();
        assert_eq!(s.observe_notify(&n), Err(PresenceError::UnsupportedMedia));
        assert_eq!(s.last_notify_cseq, None);
    }
    #[test]
    fn rejects_injection_and_duplicate_headers() {
        assert!(PresenceSubscription::new(
            "x\r\nEvil: 1",
            "sip:a@example.org",
            "sip:b@example.org",
            "tag"
        )
        .is_err());
        let mut s = sub();
        let mut n = notify(1, "active");
        n.headers.push(("Event".into(), "presence".into()));
        assert_eq!(s.observe_notify(&n), Err(PresenceError::DuplicateHeader));
    }
}
