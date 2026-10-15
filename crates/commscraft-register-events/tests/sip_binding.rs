//! Binding Microsoft REGISTER Presence-State parsing to a concrete SIP response.
//! RFC 3261 §§7.3.1, 8.1.3, 10, 20.16 (CSeq); MS-SIPREGE §2.2.1.4.
//! These are **untrusted message hints**; transaction matching, TLS peer
//! validation and MS-SIPAE MIC verification remain external prerequisites.
use commscraft_register_events::{parse_presence_state, ParseError, PresenceHint};
use sfb_core::sip::{SipMessage, StartLine};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingError {
    NotResponse,
    DuplicateCSeq,
    InvalidCSeq,
    Presence(ParseError),
}

/// Return an untrusted Presence-State hint only for a REGISTER 200 OK.
/// A final 200 for another method is not a registration event. A response
/// missing or duplicating CSeq fails closed rather than guessing its method.
pub fn parse_register_presence_response(
    message: &SipMessage,
) -> Result<Option<PresenceHint>, BindingError> {
    let code = match &message.start_line {
        StartLine::Response { code, .. } => *code,
        _ => return Err(BindingError::NotResponse),
    };
    let mut cseq = None;
    for (name, value) in &message.headers {
        if name.eq_ignore_ascii_case("CSeq") {
            if cseq.replace(value.as_str()).is_some() {
                return Err(BindingError::DuplicateCSeq);
            }
        }
    }
    let raw = cseq.ok_or(BindingError::InvalidCSeq)?;
    let mut words = raw.split_ascii_whitespace();
    let (Some(number), Some(method), None) = (words.next(), words.next(), words.next()) else {
        return Err(BindingError::InvalidCSeq);
    };
    // RFC 3261 CSeq number is a 32-bit unsigned decimal; do not accept +, -,
    // a Unicode digit, trailing bytes or integer overflow.
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit())
        || number.parse::<u32>().is_err()
    {
        return Err(BindingError::InvalidCSeq);
    }
    // RFC 3261 method tokens are case-sensitive. REGISTER is the only
    // method allowed to surface this Microsoft-specific registration hint.
    if method != "REGISTER" || code != 200 {
        return Ok(None);
    }
    let fields: Vec<(&str, &str)> = message.headers.iter()
        .map(|(key, value)| (key.as_str(), value.as_str())).collect();
    parse_presence_state(code, &fields).map_err(BindingError::Presence)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn response(code: u16, cseq: &str) -> SipMessage {
        SipMessage {
            start_line: StartLine::Response { code, reason: "Synthetic".into() },
            headers: vec![
                ("CSeq".into(), cseq.into()),
                ("Presence-State".into(), "register-action=\"added\"".into()),
            ],
            body: vec![],
        }
    }
    #[test]
    fn only_register_200_returns_hint() {
        assert!(parse_register_presence_response(&response(200, "1 REGISTER")).unwrap().is_some());
        for (status, cseq) in [(200, "1 INVITE"), (401, "1 REGISTER"), (200, "1 register")] {
            assert_eq!(parse_register_presence_response(&response(status, cseq)), Ok(None));
        }
    }
    #[test]
    fn malformed_and_duplicate_cseq_fail_closed() {
        for invalid in ["", "-1 REGISTER", "+1 REGISTER", "4294967296 REGISTER",
                        "1 REGISTER EXTRA", "1REGISTER", "notanumber REGISTER"] {
            assert_eq!(parse_register_presence_response(&response(200, invalid)),
                Err(BindingError::InvalidCSeq), "{invalid}");
        }
        let mut message = response(200, "1 REGISTER");
        message.headers.push(("cseq".into(), "2 REGISTER".into()));
        assert_eq!(parse_register_presence_response(&message), Err(BindingError::DuplicateCSeq));
        message.headers.retain(|(key, _)| !key.eq_ignore_ascii_case("CSeq"));
        assert_eq!(parse_register_presence_response(&message), Err(BindingError::InvalidCSeq));
    }
    #[test]
    fn request_never_becomes_a_register_success() {
        let mut m = response(200, "1 REGISTER");
        m.start_line = StartLine::Request { method: "REGISTER".into(), uri: "sip:example.com".into(), version: "SIP/2.0".into() };
        assert_eq!(parse_register_presence_response(&m), Err(BindingError::NotResponse));
    }
    #[test]
    fn duplicate_presence_state_is_rejected() {
        let mut m = response(200, "1 REGISTER");
        m.headers.push(("presence-state".into(), "register-action=\"fixed\"".into()));
        assert_eq!(parse_register_presence_response(&m), Err(BindingError::Presence(ParseError::DuplicateHeader)));
    }
}
