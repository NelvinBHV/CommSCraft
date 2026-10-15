//! Bounded SIP/2.0 message framing and an offline REGISTER illustration.
//! RFC 3261 is much larger than this parser: complete SIP ABNF,
//! authentication, dialogs and SIP URI grammar are NOT implemented here.
//! The separate transaction module covers only reliable non-INVITE flows.

use std::fmt;

pub const MAX_HEADER_BYTES: usize = 64 * 1024;
pub const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartLine {
    Request { method: String, uri: String },
    Response { code: u16, reason: String },
}

#[derive(Clone, PartialEq, Eq)]
pub struct SipMessage {
    pub start_line: StartLine,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

// Do not reveal credentials, SIP account identities, GSS tokens or message
// bodies through routine debugging. Explicit wire logging needs its own
// reviewed, opt-in redaction layer.
impl fmt::Debug for SipMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match &self.start_line {
            StartLine::Request { .. } => "request",
            StartLine::Response { .. } => "response",
        };
        f.debug_struct("SipMessage")
            .field("kind", &kind)
            .field("header_count", &self.headers.len())
            .field("body_bytes", &self.body.len()).finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SipError {
    BadUtf8,
    MalformedStartLine,
    MalformedHeader,
    Oversize,
    DuplicateContentLength,
    InvalidContentLength,
    InvalidUri,
    MissingContentLength,
    DecoderPoisoned,
    InvalidOutboundMessage,
}

impl fmt::Display for SipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for SipError {}

fn content_length_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("Content-Length") || name.eq_ignore_ascii_case("l")
}

impl SipMessage {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Validated serialization for outbound SIP data. Always use this rather
    /// than interpolating caller-controlled strings into a wire template.
    pub fn encode(&self) -> Result<Vec<u8>, SipError> {
        let mut bytes = match &self.start_line {
            StartLine::Request { method, uri } => {
                if method.is_empty() || !method.bytes().all(|b| {
                    b.is_ascii_alphanumeric() || b"-.!%*_+`'~".contains(&b)
                }) || uri.is_empty() || uri.bytes().any(|b| b <= 32 || b >= 127) {
                    return Err(SipError::InvalidOutboundMessage);
                }
                format!("{method} {uri} SIP/2.0\r\n").into_bytes()
            }
            StartLine::Response { code, reason } => {
                if !(100..=699).contains(code) || reason.bytes().any(|b| b < 32 || b == 127) {
                    return Err(SipError::InvalidOutboundMessage);
                }
                format!("SIP/2.0 {code} {reason}\r\n").into_bytes()
            }
        };
        if self.body.len() > MAX_BODY_BYTES { return Err(SipError::Oversize); }
        for (name, value) in &self.headers {
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                || value.bytes().any(|b| b == 0 || b == b'\r' || b == b'\n'
                    || (b < 32 && b != b'\t') || b == 127)
            {
                return Err(SipError::InvalidOutboundMessage);
            }
            // The encoded Content-Length is always generated from actual body bytes.
            if content_length_header(name) { continue; }
            bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
            if bytes.len() > MAX_HEADER_BYTES { return Err(SipError::Oversize); }
        }
        bytes.extend_from_slice(format!("Content-Length: {}\r\n\r\n", self.body.len()).as_bytes());
        if bytes.len() > MAX_HEADER_BYTES + 4 { return Err(SipError::Oversize); }
        bytes.extend_from_slice(&self.body);
        Ok(bytes)
    }
}

/// Returns one complete message and its consumed length from a stream buffer.
/// Missing Content-Length is interpreted as zero, suitable for demonstration;
/// callers must choose a correct strategy for their transport and protocol flow.
/// Deliberately rejects obsolete folded headers and non-UTF-8 headers.
pub fn parse_frame(input: &[u8]) -> Result<Option<(SipMessage, usize)>, SipError> {
    let Some(boundary) = input.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
        if input.len() > MAX_HEADER_BYTES { return Err(SipError::Oversize); }
        return Ok(None);
    };
    if boundary > MAX_HEADER_BYTES { return Err(SipError::Oversize); }
    let head = std::str::from_utf8(&input[..boundary]).map_err(|_| SipError::BadUtf8)?;
    let mut lines = head.split("\r\n");
    let line = lines.next().ok_or(SipError::MalformedStartLine)?;
    let start_line = if let Some(rest) = line.strip_prefix("SIP/2.0 ") {
        let (code, reason) = rest.split_once(' ').ok_or(SipError::MalformedStartLine)?;
        if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_digit()) {
            return Err(SipError::MalformedStartLine);
        }
        let code = code.parse::<u16>().map_err(|_| SipError::MalformedStartLine)?;
        if !(100..=699).contains(&code)
            || reason.bytes().any(|b| b < 0x20 || b == 0x7f)
        {
            return Err(SipError::MalformedStartLine);
        }
        StartLine::Response { code, reason: reason.to_owned() }
    } else {
        let mut fields = line.split(' ');
        let method = fields.next().ok_or(SipError::MalformedStartLine)?;
        let uri = fields.next().ok_or(SipError::MalformedStartLine)?;
        let version = fields.next().ok_or(SipError::MalformedStartLine)?;
        if fields.next().is_some() || method.is_empty() || uri.is_empty() || version != "SIP/2.0"
            || !method.bytes().all(|b| b.is_ascii_alphanumeric() || b"-.!%*_+`'~".contains(&b))
            || uri.bytes().any(|b| b <= 32 || b >= 127)
        {
            return Err(SipError::MalformedStartLine);
        }
        StartLine::Request { method: method.to_owned(), uri: uri.to_owned() }
    };
    let mut headers = Vec::new();
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(SipError::MalformedHeader)?;
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(SipError::MalformedHeader);
        }
        if value.bytes().any(|b| b == 0 || b == b'\r' || b == b'\n'
            || (b < 32 && b != b'\t') || b == 127)
        { return Err(SipError::MalformedHeader); }
        let value = value.trim();
        if content_length_header(name) {
            if length.is_some() { return Err(SipError::DuplicateContentLength); }
            if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err(SipError::InvalidContentLength);
            }
            let n = value.parse::<usize>().map_err(|_| SipError::InvalidContentLength)?;
            if n > MAX_BODY_BYTES { return Err(SipError::Oversize); }
            length = Some(n);
        }
        headers.push((name.to_owned(), value.to_owned()));
    }
    let body_len = length.unwrap_or_default();
    let end = boundary + 4 + body_len;
    if input.len() < end { return Ok(None); }
    Ok(Some((SipMessage {
        start_line, headers, body: input[boundary + 4..end].to_vec(),
    }, end)))
}

/// An illustrative unauthenticated template. NOT a registration client.
/// The Via sent-by and Contact hostname are intentionally non-routable.
pub fn illustrative_register(sip_uri: &str) -> Result<SipMessage, SipError> {
    let uri = sip_uri.strip_prefix("sip:").ok_or(SipError::InvalidUri)?;
    let (user, domain) = uri.split_once('@').ok_or(SipError::InvalidUri)?;
    if user.is_empty() || domain.is_empty() || uri.matches('@').count() != 1
        || !uri.bytes().all(|b| b.is_ascii_alphanumeric() || b".-_@+".contains(&b))
        || !domain.contains('.')
    {
        return Err(SipError::InvalidUri);
    }
    Ok(SipMessage {
        start_line: StartLine::Request {
            method: "REGISTER".to_owned(), uri: format!("sip:{domain}"),
        },
        headers: vec![
            ("Via".into(), "SIP/2.0/TLS client.invalid;branch=z9hG4bK-demo-only".into()),
            ("Max-Forwards".into(), "70".into()),
            ("From".into(), format!("<{sip_uri}>;tag=demo-only")),
            ("To".into(), format!("<{sip_uri}>")),
            ("Call-ID".into(), "demo-only@client.invalid".into()),
            ("CSeq".into(), "1 REGISTER".into()),
            ("Contact".into(), format!("<sip:{user}@client.invalid>")),
            ("Expires".into(), "3600".into()),
        ],
        body: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_stream_with_binary_body() {
        let source = SipMessage { start_line: StartLine::Request {
            method: "MESSAGE".into(), uri: "sip:bob@example.com".into(),
        }, headers: vec![("From".into(), "<sip:alice@example.com>".into())],
            body: vec![0, 10, 13, 255] };
        let mut wire = source.encode().unwrap();
        wire.extend_from_slice(b"TRAILER");
        let (parsed, consumed) = parse_frame(&wire).unwrap().unwrap();
        assert_eq!(parsed, SipMessage { headers: vec![
            ("From".into(), "<sip:alice@example.com>".into()),
            ("Content-Length".into(), "4".into())], ..source });
        assert_eq!(&wire[consumed..], b"TRAILER");
    }
    #[test]
    fn handles_fragmented_stream() {
        let wire = b"SIP/2.0 200 OK\r\nContent-Length: 4\r\n\r\nTEST";
        for i in 0..wire.len() { assert!(parse_frame(&wire[..i]).unwrap().is_none()); }
        assert!(parse_frame(wire).unwrap().is_some());
    }
    #[test]
    fn duplicate_and_oversized_lengths_are_rejected() {
        assert_eq!(parse_frame(b"SIP/2.0 200 OK\r\nl: 2\r\nContent-Length: 2\r\n\r\nhi"),
            Err(SipError::DuplicateContentLength));
        assert_eq!(parse_frame(b"SIP/2.0 200 OK\r\nContent-Length: 99999999\r\n\r\n"),
            Err(SipError::Oversize));
    }
    #[test]
    fn rejects_ambiguous_length_and_method_syntax() {
        assert_eq!(parse_frame(b"SIP/2.0 200 OK\r\nContent-Length: +1\r\n\r\nX"),
            Err(SipError::InvalidContentLength));
        assert_eq!(parse_frame(b"MESS\tAGE sip:bob@example.com SIP/2.0\r\nContent-Length: 0\r\n\r\n"),
            Err(SipError::MalformedStartLine));
    }
    #[test]
    fn response_reason_phrase_rejects_control_bytes_but_allows_utf8() {
        // RFC 3261 §§7.2, 25.1: descriptive UTF-8 is allowed, controls are not.
        for bad in [b'\0', b'\t', b'\x1b', b'\x7f'] {
            let mut wire = b"SIP/2.0 200 O".to_vec();
            wire.push(bad);
            wire.extend_from_slice(b"K\r\nContent-Length: 0\r\n\r\n");
            assert_eq!(parse_frame(&wire), Err(SipError::MalformedStartLine));
        }
        let wire = "SIP/2.0 200 Erfolgreich ✓\r\nContent-Length: 0\r\n\r\n";
        let (response, _) = parse_frame(wire.as_bytes()).unwrap().unwrap();
        assert_eq!(response.start_line, StartLine::Response {
            code: 200, reason: "Erfolgreich ✓".into(),
        });
    }

    #[test]
    fn outbound_serialization_rejects_header_injection() {
        let mut msg = illustrative_register("sip:alice@example.com").unwrap();
        msg.headers.push(("X-Bad".into(), "safe\r\nAuthorization: stolen".into()));
        assert_eq!(msg.encode(), Err(SipError::InvalidOutboundMessage));
        msg.headers.pop();
        msg.body = vec![0; MAX_BODY_BYTES + 1];
        assert_eq!(msg.encode(), Err(SipError::Oversize));
    }
    #[test]
    fn register_is_an_offline_template_and_rejects_injection() {
        let msg = illustrative_register("sip:alice@example.com").unwrap();
        let (decoded, _) = parse_frame(&msg.encode().unwrap()).unwrap().unwrap();
        assert_eq!(decoded.header("Content-Length"), Some("0"));
        assert!(illustrative_register("sip:alice@example.com\r\nEvil: 1").is_err());
    }
}

#[cfg(test)]
mod redaction_tests {
    use super::*;
    #[test]
    fn debug_must_not_expose_authentication_or_user_identifiers() {
        let message = SipMessage { start_line: StartLine::Request {
            method: "REGISTER".into(), uri: "sip:sensitive.example.com".into(),
        }, headers: vec![
            ("Authorization".into(), "Kerberos gssapi-data=TOPSECRET".into()),
            ("Call-ID".into(), "PRIVATE-ID".into())
        ], body: b"PASSWORD".to_vec() };
        let display = format!("{message:?}");
        for secret in ["TOPSECRET", "PRIVATE-ID", "PASSWORD", "sensitive.example.com"] {
            assert!(!display.contains(secret), "Debug leaked {secret}");
        }
    }
}
