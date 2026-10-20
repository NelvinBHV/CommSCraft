//! Offline Microsoft REGISTER Presence-State parsing, not authentication.
//! References: MS-SIPREGE 2.2.1.4, 3.1.2.5.1; RFC 3261 section 10.3.
#![forbid(unsafe_code)]

/// The registrar's reported binding action; never a verified sign-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterAction {
    Added,
    Refreshed,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cluster {
    Central,
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterState {
    pub kind: Cluster,
    pub connected: bool,
}

/// Untrusted claim from a header. Must not promote session authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresenceHint {
    pub action: RegisterAction,
    pub cluster: Option<ClusterState>,
    pub user_services_unavailable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    DuplicateHeader,
    TooLong,
    InvalidSyntax,
    UnsupportedValue,
    UnknownField,
}
const MAX_HEADER_BYTES: usize = 1024;

fn key_value(segment: &str) -> Result<(&str, &str), ParseError> {
    let (key, value) = segment
        .trim()
        .split_once('=')
        .ok_or(ParseError::InvalidSyntax)?;
    let (key, value) = (key.trim(), value.trim());
    if key.is_empty() || value.is_empty() || key.contains('=') {
        return Err(ParseError::InvalidSyntax);
    }
    Ok((key, value))
}
fn require_key(key: &str, expected: &str) -> Result<(), ParseError> {
    if key.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(ParseError::UnknownField)
    }
}
fn quoted(raw: &str) -> Result<&str, ParseError> {
    let value = raw
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .ok_or(ParseError::InvalidSyntax)?;
    if value.is_empty() || value.contains('"') {
        return Err(ParseError::InvalidSyntax);
    }
    Ok(value)
}

/// Parse at most one optional Presence-State header in a REGISTER 200 OK.
/// Zero headers are valid; MS-SIPREGE uses SHOULD for inclusion.
/// This function NEVER checks server identity, MIC, replay or GRUU.
pub fn parse_presence_state(
    sip_status: u16,
    headers: &[(&str, &str)],
) -> Result<Option<PresenceHint>, ParseError> {
    if sip_status != 200 {
        return Ok(None);
    }
    let mut found = None;
    for (key, value) in headers.iter().copied() {
        if key.eq_ignore_ascii_case("Presence-State") && found.replace(value).is_some() {
            return Err(ParseError::DuplicateHeader);
        }
    }
    let Some(header) = found else {
        return Ok(None);
    };
    if header.len() > MAX_HEADER_BYTES {
        return Err(ParseError::TooLong);
    }
    if header
        .bytes()
        .any(|b| b == b'\r' || b == b'\n' || b == 0x7f || (b < 0x20 && b != b'\t'))
    {
        return Err(ParseError::InvalidSyntax);
    }
    let mut fields = header.split(';');
    let (key, val) = key_value(fields.next().ok_or(ParseError::InvalidSyntax)?)?;
    require_key(key, "register-action")?;
    let action = match quoted(val)? {
        v if v.eq_ignore_ascii_case("added") => RegisterAction::Added,
        v if v.eq_ignore_ascii_case("fixed") => RegisterAction::Fixed,
        v if v.eq_ignore_ascii_case("refreshed") => RegisterAction::Refreshed,
        _ => return Err(ParseError::UnsupportedValue),
    };
    let mut cluster = None;
    let mut unavailable = false;
    if let Some(raw) = fields.next() {
        let (key, value) = key_value(raw)?;
        require_key(key, "primary-cluster-type")?;
        let kind = match quoted(value)? {
            v if v.eq_ignore_ascii_case("central") => Cluster::Central,
            v if v.eq_ignore_ascii_case("remote") => Cluster::Remote,
            _ => return Err(ParseError::UnsupportedValue),
        };
        let (key, value) = key_value(fields.next().ok_or(ParseError::InvalidSyntax)?)?;
        require_key(key, "is-connected-to-primary")?;
        let connected = match quoted(value)? {
            v if v.eq_ignore_ascii_case("yes") => true,
            v if v.eq_ignore_ascii_case("no") => false,
            _ => return Err(ParseError::UnsupportedValue),
        };
        cluster = Some(ClusterState { kind, connected });
        if let Some(raw) = fields.next() {
            let (key, value) = key_value(raw)?;
            require_key(key, "user-services-state")?;
            if !value.eq_ignore_ascii_case("unavailable") {
                return Err(ParseError::UnsupportedValue);
            }
            unavailable = true;
        }
    }
    if fields.next().is_some() {
        return Err(ParseError::InvalidSyntax);
    }
    Ok(Some(PresenceHint {
        action,
        cluster,
        user_services_unavailable: unavailable,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(v: &str) -> Result<Option<PresenceHint>, ParseError> {
        parse_presence_state(200, &[("pReSeNcE-sTaTe", v)])
    }
    #[test]
    fn optional_header_absent() {
        assert_eq!(parse_presence_state(200, &[]), Ok(None));
    }
    #[test]
    fn non_ok_response_does_not_carry_register_hint() {
        assert_eq!(
            parse_presence_state(401, &[("Presence-State", "bad")]),
            Ok(None)
        );
    }
    #[test]
    fn actions_from_microsoft_spec() {
        for (text, action) in [
            ("added", RegisterAction::Added),
            ("refreshed", RegisterAction::Refreshed),
            ("fixed", RegisterAction::Fixed),
        ] {
            assert_eq!(
                parse(&format!("register-action=\"{text}\""))
                    .unwrap()
                    .unwrap()
                    .action,
                action
            );
        }
    }
    #[test]
    fn extended_cluster_state() {
        let parsed = parse("register-action=\"fixed\";primary-cluster-type=\"remote\";is-connected-to-primary=\"no\";user-services-state=unavailable").unwrap().unwrap();
        assert_eq!(
            parsed.cluster,
            Some(ClusterState {
                kind: Cluster::Remote,
                connected: false
            })
        );
        assert!(parsed.user_services_unavailable);
    }
    #[test]
    fn case_insensitive_sip_name_and_abnf_literals() {
        assert_eq!(
            parse("REGISTER-ACTION = \"ADDED\"")
                .unwrap()
                .unwrap()
                .action,
            RegisterAction::Added
        );
    }
    #[test]
    fn duplicate_header_is_rejected() {
        assert_eq!(
            parse_presence_state(
                200,
                &[
                    ("Presence-State", "register-action=\"added\""),
                    ("presence-state", "register-action=\"fixed\"")
                ]
            ),
            Err(ParseError::DuplicateHeader)
        );
    }
    #[test]
    fn malformed_extensions_are_rejected() {
        for value in [
            "", "register-action=added", "register-action=\"other\"",
            "register-action=\"added\";", "register-action=\"added\";extra=x",
            "register-action=\"added\";primary-cluster-type=\"remote\"",
            "register-action=\"added\";user-services-state=unavailable",
            "register-action=\"added\";primary-cluster-type=\"remote\";is-connected-to-primary=\"maybe\"",
            "register-action=\"added\";primary-cluster-type=\"remote\";is-connected-to-primary=\"no\";user-services-state=ready",
        ] { assert!(parse(value).is_err(), "accepted {value}"); }
    }
    #[test]
    fn rejects_header_injection_and_nul() {
        for value in [
            "register-action=\"added\"\r\nTo: x",
            "register-action=\"added\"\0",
        ] {
            assert_eq!(parse(value), Err(ParseError::InvalidSyntax));
        }
    }
    #[test]
    fn accepts_case_insensitive_user_services_token() {
        let parsed = parse("register-action=\"added\";primary-cluster-type=\"central\";is-connected-to-primary=\"yes\";user-services-state=UNAVAILABLE").unwrap().unwrap();
        assert!(parsed.user_services_unavailable);
    }
    #[test]
    fn bounded_header() {
        assert_eq!(parse(&"x".repeat(1025)), Err(ParseError::TooLong));
    }
    #[test]
    fn no_authenticated_state_is_constructed() {
        let state = parse("register-action=\"added\"").unwrap().unwrap();
        assert_eq!(state.action, RegisterAction::Added);
    }
}

/// Strict parsing of SIP REGISTER response context before Presence-State hints.
pub mod sip_binding;
pub use sip_binding::{parse_register_presence_response, BindingError};

/// Compose a correlated, MIC-checked REGISTER result with untrusted metadata.
pub mod gated_presence;
pub use gated_presence::{observe_register_with_presence, GatedPresenceError, GatedPresenceOutcome};
