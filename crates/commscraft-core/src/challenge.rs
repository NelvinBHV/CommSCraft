//! Read-only Microsoft SIP authentication challenge introspection.
//!
//! [MS-SIPAE] §2.2.1 defines NTLM, Kerberos, TLS-DSK and Digest challenges.
//! This parses only the advertised scheme and a small bounded parameter subset.
//! It does NOT select a credential, create a security association, decode
//! gssapi-data, sign requests, or authenticate a user. Never log raw challenges.

use crate::sip::{SipMessage, StartLine};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Kerberos,
    Ntlm,
    TlsDsk,
    Digest,
    Unsupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeSource {
    Registrar,
    Proxy,
}

/// Private data is deliberately redacted from Debug output.
#[derive(Clone, PartialEq, Eq)]
pub struct ChallengeOffer {
    pub scheme: Scheme,
    pub source: ChallengeSource,
    pub realm: Option<String>,
    pub targetname: Option<String>,
    pub version: Option<u32>,
    pub sts_uri: Option<String>,
    pub opaque: Option<String>,
    /// Opaque base64 GSS continuation token. Never include in Debug/logs.
    pub gssapi_data: Option<String>,
}
impl fmt::Debug for ChallengeOffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChallengeOffer")
            .field("scheme", &self.scheme)
            .field("source", &self.source)
            .field("version", &self.version)
            .field("challenge_data", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeError {
    NotResponse,
    NotChallenge,
    MissingHeader,
    InvalidSyntax,
    DuplicateParameter,
    Oversized,
}
impl fmt::Display for ChallengeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ChallengeError {}

fn split_params(s: &str) -> Result<Vec<&str>, ChallengeError> {
    // Split on unquoted commas, reject unterminated strings, CRLF, control
    // characters, and oversized inputs before interpreting any token.
    if s.len() > 8192
        || s.bytes()
            .any(|b| b == b'\r' || b == b'\n' || b == 0 || b == 127 || (b < 32 && b != b'\t'))
    {
        return Err(ChallengeError::Oversized);
    }
    let mut parts = Vec::new();
    let (mut start, mut quoted, mut escaped) = (0, false, false);
    for (i, b) in s.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if b == b'\\' && quoted {
            escaped = true;
            continue;
        }
        if b == b'"' {
            quoted = !quoted;
        }
        if b == b',' && !quoted {
            parts.push(&s[start..i]);
            start = i + 1;
        }
    }
    if quoted || escaped {
        return Err(ChallengeError::InvalidSyntax);
    }
    parts.push(&s[start..]);
    if parts.len() > 32 {
        return Err(ChallengeError::Oversized);
    }
    Ok(parts)
}

fn unquote(value: &str) -> Result<String, ChallengeError> {
    let value = value.trim();
    if value.starts_with('"') {
        if !value.ends_with('"') || value.len() < 2 {
            return Err(ChallengeError::InvalidSyntax);
        }
        let mut result = String::new();
        let mut chars = value[1..value.len() - 1].chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                let next = chars.next().ok_or(ChallengeError::InvalidSyntax)?;
                if next == '\r' || next == '\n' {
                    return Err(ChallengeError::InvalidSyntax);
                }
                result.push(next);
            } else if c == '"' || c.is_control() {
                return Err(ChallengeError::InvalidSyntax);
            } else {
                result.push(c);
            }
        }
        Ok(result)
    } else if value.is_empty()
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~!%*+`'".contains(&b))
    {
        Err(ChallengeError::InvalidSyntax)
    } else {
        Ok(value.to_owned())
    }
}

fn parse_offer(value: &str, source: ChallengeSource) -> Result<ChallengeOffer, ChallengeError> {
    let value = value.trim();
    let space = value
        .find(char::is_whitespace)
        .ok_or(ChallengeError::InvalidSyntax)?;
    let scheme = match &value[..space].to_ascii_lowercase()[..] {
        "kerberos" => Scheme::Kerberos,
        "ntlm" => Scheme::Ntlm,
        "tls-dsk" => Scheme::TlsDsk,
        "digest" => Scheme::Digest,
        _ => Scheme::Unsupported,
    };
    let mut offer = ChallengeOffer {
        scheme,
        source,
        realm: None,
        targetname: None,
        version: None,
        sts_uri: None,
        opaque: None,
        gssapi_data: None,
    };
    let mut found = std::collections::HashSet::new();
    for entry in split_params(&value[space..])? {
        let (key, val) = entry
            .trim()
            .split_once('=')
            .ok_or(ChallengeError::InvalidSyntax)?;
        let key = key.trim().to_ascii_lowercase();
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(ChallengeError::InvalidSyntax);
        }
        if !found.insert(key.clone()) {
            return Err(ChallengeError::DuplicateParameter);
        }
        // Validate every parsed value, even an unsupported extension, to avoid
        // accidentally accepting malformed quoted values at this trust boundary.
        let val = unquote(val)?;
        match key.as_str() {
            "realm" => offer.realm = Some(val),
            "targetname" => offer.targetname = Some(val),
            "opaque" => offer.opaque = Some(val),
            "gssapi-data" => offer.gssapi_data = Some(val),
            "sts-uri" => offer.sts_uri = Some(val),
            "version" => {
                offer.version = Some(val.parse().map_err(|_| ChallengeError::InvalidSyntax)?)
            }
            _ => {}
        }
    }
    Ok(offer)
}

/// A response's 401 WWW-Authenticate and 407 Proxy-Authenticate offers.
/// Multiple header fields are supported as in [MS-SIPAE] §3.3.5.1.
/// Only structured, redacted summaries should be exposed in diagnostics.
pub fn parse_challenges(response: &SipMessage) -> Result<Vec<ChallengeOffer>, ChallengeError> {
    let (header, source) = match &response.start_line {
        StartLine::Response { code: 401, .. } => ("WWW-Authenticate", ChallengeSource::Registrar),
        StartLine::Response { code: 407, .. } => ("Proxy-Authenticate", ChallengeSource::Proxy),
        StartLine::Response { .. } => return Err(ChallengeError::NotChallenge),
        StartLine::Request { .. } => return Err(ChallengeError::NotResponse),
    };
    let mut result = Vec::new();
    for (name, value) in &response.headers {
        if name.eq_ignore_ascii_case(header) {
            if result.len() >= 8 {
                return Err(ChallengeError::Oversized);
            }
            result.push(parse_offer(value, source)?);
        }
    }
    if result.is_empty() {
        return Err(ChallengeError::MissingHeader);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn message(code: u16, fields: &[(&str, &str)]) -> SipMessage {
        SipMessage {
            start_line: StartLine::Response {
                code,
                reason: String::from("Test"),
            },
            headers: fields
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            body: vec![],
        }
    }
    #[test]
    fn microsoft_multiple_auth_offers_with_quoted_comma() {
        let m = message(401, &[
            ("WWW-Authenticate", "NTLM realm=\"SIP, Communications Service\", targetname=\"server.example.com\", version=3"),
            ("WWW-Authenticate", "Kerberos realm=\"SIP Communications Service\", targetname=\"sip/server.example.com\", version=4"),
        ]);
        let offers = parse_challenges(&m).unwrap();
        assert_eq!(offers.len(), 2);
        assert_eq!(offers[0].scheme, Scheme::Ntlm);
        assert_eq!(
            offers[0].realm.as_deref(),
            Some("SIP, Communications Service")
        );
        assert_eq!(offers[1].scheme, Scheme::Kerberos);
        assert_eq!(offers[1].version, Some(4));
        let redacted = format!("{:?}", offers[0]);
        assert!(!redacted.contains("server.example.com"));
        assert!(!redacted.contains("Communications"));
    }
    #[test]
    fn distinguishes_proxy_challenges() {
        let m = message(
            407,
            &[(
                "Proxy-Authenticate",
                "TLS-DSK realm=\"x\", targetname=\"a.example.com\", version=4",
            )],
        );
        let offers = parse_challenges(&m).unwrap();
        assert_eq!(offers[0].source, ChallengeSource::Proxy);
        assert_eq!(offers[0].scheme, Scheme::TlsDsk);
    }
    #[test]
    fn gssapi_data_is_parsed_without_exposing_it_in_debug() {
        let m = message(401, &[("WWW-Authenticate",
            "NTLM realm=\"SIP Communications Service\", targetname=\"server.example.com\", version=3, gssapi-data=\"AQIDBA==\"")]);
        let o = parse_challenges(&m).unwrap();
        assert_eq!(o[0].gssapi_data.as_deref(), Some("AQIDBA=="));
        assert!(!format!("{:?}", o[0]).contains("AQIDBA=="));
    }
    #[test]
    fn duplicate_gssapi_data_is_rejected() {
        let m = message(407, &[("Proxy-Authenticate",
            "NTLM realm=\"x\", targetname=\"a.example.com\", version=3, gssapi-data=\"AQ==\", gssapi-data=\"Ag==\"")]);
        assert_eq!(
            parse_challenges(&m),
            Err(ChallengeError::DuplicateParameter)
        );
    }
    #[test]
    fn rejects_malformed_and_duplicate_input() {
        for value in [
            "NTLM realm=\"unterminated",
            "NTLM realm=ok, realm=again",
            "NTLM realm=hi\r\nEvil:yes",
            "NTLM nonsense",
        ] {
            let m = message(401, &[("WWW-Authenticate", value)]);
            assert!(parse_challenges(&m).is_err(), "{value:?}");
        }
        assert_eq!(
            parse_challenges(&message(200, &[])),
            Err(ChallengeError::NotChallenge)
        );
        assert_eq!(
            parse_challenges(&message(401, &[])),
            Err(ChallengeError::MissingHeader)
        );
    }
}
