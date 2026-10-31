//! Call park (function 37, M4): the three `application/ms-call-park+xml` documents of
//! [MS-SIPAPP] 20250520 – the park request the parker sends in an INVITE to the call park server
//! (CPS), the park response in the CPS's 200 OK (orbit and parked-call identifier) and the unpark
//! notification the CPS sends in INFO on the parker's dialog. Sans-I/O; the SIP side lives in
//! `commscraft-session`.
//!
//! Evidence: `docs/protocol/evidence/31-sipapp-call-park.md` (CK-01 … CK-07).
//! Sources: [MS-SIPAPP] §2.2.1.1–2.2.1.3 (pp. 10–13), §3.1.5.1–3.1.5.6 (pp. 15–17), schema §6.1
//! (p. 36), examples §4.1 (pp. 21–30).
//!
//! Security by design (`docs/security.md`):
//! - the park request is built only from a checked SIP URI and dialog identifiers made of SIP
//!   token characters, so nothing can be injected into the XML;
//! - answers are parsed with the bounded reader (no DTD, no entities, small limits) and must be the
//!   expected root element in the callpark namespace;
//! - the orbit must have the documented shape (1–9 digits, or `*`/`#` and 1–8 digits) – it is
//!   dialled later, so nothing else is accepted; other texts are bounded and made printable.

use commscraft_presence::{checked_sip_uri, printable};
use commscraft_xml::{escape, Element, Limits};

pub const CONTENT_TYPE: &str = "application/ms-call-park+xml";
pub const NS: &str = "http://schemas.microsoft.com/rtc/2008/12/callpark";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Xml(commscraft_xml::Error),
    WrongDocument,
    BadValue(&'static str),
}

fn limits() -> Limits {
    Limits {
        max_input: 8 * 1024,
        max_depth: 6,
        ..Limits::default()
    }
}

/// SIP token characters (RFC 3261 §25.1): what Call-IDs and tags are made of.
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-.!%*_+`'~@:".contains(c))
}

/// A SIP or SIPS URI without characters that could leave the XML text or a header (the target
/// is the peer's contact – a GRUU for Skype for Business clients, a host address for gateways).
fn safe_sip_uri(u: &str) -> bool {
    let scheme = u.get(..4).is_some_and(|s| s.eq_ignore_ascii_case("sip:"))
        || u.get(..5).is_some_and(|s| s.eq_ignore_ascii_case("sips:"));
    scheme
        && u.len() <= 1024
        && !u
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "<>\"',&".contains(c))
}

/// The dialog of the call to park (CK-01: "the SIP from-tag … the SIP to-tag of the call that is
/// to be parked", §2.2.1.1, p. 11 – the tags of the call's From and To headers as the CPS puts
/// them into Replaces, example pp. 21–22).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallDialog {
    pub call_id: String,
    pub from_tag: String,
    pub to_tag: String,
}

/// The park request (CK-01; §2.2.1.1, pp. 10–11; §3.1.5.1, p. 15): version 1.0, our request ID,
/// exactly one `audio` element with the dialog and the GRUU of the party to park.
pub fn park_request(request_id: u32, dialog: &CallDialog, target: &str) -> Result<String, Error> {
    if !token(&dialog.call_id) || !token(&dialog.from_tag) || !token(&dialog.to_tag) {
        return Err(Error::BadValue("dialog"));
    }
    if !safe_sip_uri(target) {
        return Err(Error::BadValue("target"));
    }
    Ok(format!(
        "<?xml version=\"1.0\"?><park-request xmlns=\"{NS}\" version=\"1.0\" request-id=\"{request_id}\"><audio><dialog-info><call-id>{}</call-id><from-tag>{}</from-tag><to-tag>{}</to-tag></dialog-info><target>{}</target></audio></park-request>",
        escape(&dialog.call_id),
        escape(&dialog.from_tag),
        escape(&dialog.to_tag),
        escape(target)
    ))
}

/// The orbit shape (CK-02; §2.2.1.2, p. 12): 1–9 digits, or `*` or `#` followed by 1–8 digits.
pub fn valid_orbit(o: &str) -> bool {
    let digits =
        |s: &str, max: usize| (1..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    match o.strip_prefix(['*', '#']) {
        Some(rest) => digits(rest, 8),
        None => digits(o, 9),
    }
}

/// The park response (CK-02; §2.2.1.2, pp. 11–12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parked {
    pub request_id: String,
    /// Where the call waits; dialled to retrieve it (CK-05).
    pub orbit: String,
    /// The CPS's identifier of the parked call (printable, bounded).
    pub parked_call: String,
}

fn root(xml: &str, name: &str) -> Result<Element, Error> {
    // NUL characters after the document are padding, not XML (not allowed anywhere in a
    // document, XML 1.0 §2.2): cut off at the end only; anywhere else the parser refuses them.
    let xml = xml.trim_end_matches('\0');
    let r = commscraft_xml::parse(xml, &limits()).map_err(Error::Xml)?;
    if r.name != name || r.namespace.as_deref() != Some(NS) {
        return Err(Error::WrongDocument);
    }
    // Major version 1 only (§3.1.5.1: "major version of anything other than 1" is refused).
    if !r
        .attr("version")
        .is_some_and(|v| v.trim().split('.').next() == Some("1"))
    {
        return Err(Error::BadValue("version"));
    }
    Ok(r)
}

fn bounded(s: &str, max: usize) -> String {
    printable(s.trim()).chars().take(max).collect()
}

pub fn parse_park_response(xml: &str) -> Result<Parked, Error> {
    let r = root(xml, "park-response")?;
    let orbit = r
        .child("orbit")
        .map(|e| e.text().trim().to_string())
        .filter(|o| valid_orbit(o))
        .ok_or(Error::BadValue("orbit"))?;
    Ok(Parked {
        request_id: bounded(r.attr("request-id").unwrap_or(""), 64),
        orbit,
        parked_call: r
            .child("ms-parked-call")
            .map(|e| bounded(&e.text(), 256))
            .unwrap_or_default(),
    })
}

/// Why a parked call left its orbit (CK-04; §2.2.1.3, p. 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnparkReason {
    /// Someone retrieved the call (target: who).
    Retrieval,
    /// The parked party hung up.
    HangUp,
    /// The call rings back to the parker after the timeout.
    Ringback,
    /// Ringback failed; the call went to the fallback destination (target).
    Fallback,
    /// The call was dropped.
    Drop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unpark {
    pub request_id: String,
    pub reason: UnparkReason,
    /// The retriever or fallback destination, if named and a usable SIP URI.
    pub target: Option<String>,
}

pub fn parse_unpark(xml: &str) -> Result<Unpark, Error> {
    let r = root(xml, "unpark-notification")?;
    let reason = match r
        .child("reason")
        .map(|e| e.text())
        .as_deref()
        .map(str::trim)
    {
        Some("retrieval") => UnparkReason::Retrieval,
        Some("hang-up") => UnparkReason::HangUp,
        Some("ringback") => UnparkReason::Ringback,
        Some("fallback") => UnparkReason::Fallback,
        Some("drop") => UnparkReason::Drop,
        _ => return Err(Error::BadValue("reason")),
    };
    Ok(Unpark {
        request_id: bounded(r.attr("request-id").unwrap_or(""), 64),
        reason,
        target: r
            .child("target")
            .and_then(|e| checked_sip_uri(e.text().trim()).ok()),
    })
}

#[cfg(test)]
mod tests;
