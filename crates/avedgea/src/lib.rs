//! Media relay credentials from the Media Relay Authentication Service (MS-AVEDGEA): the SIP
//! SERVICE body and the checked answer (TURN username/password, relays).
//!
//! Evidence: `docs/protocol/evidence/11-avedgea-media-relay-token.md` (AV-01 … AV-09, K-AV-1).
//! Sources: [MS-AVEDGEA] 20250520 §2.2 (pp. 11–18), §3.1.5 (pp. 22–24), schema §6 (pp. 30–32),
//! examples §4.1 (pp. 25–26) and §4.2 (pp. 27–28). Secondary: [SIPE] `sipe-media.c`
//! l. 2257–2371 (username and password used as received).
//!
//! Security by design: the answer is parsed with the bounded XML reader (no DTD) and must name
//! our request ID and status OK; username and password are bounded and limited to the base64
//! alphabet; host names must be DNS names, addresses must parse; ports must be non-zero; at most
//! 16 relays; the password is zeroised on drop and never printed.

use std::net::IpAddr;

use commscraft_xml::{escape, Element, Limits};
use zeroize::Zeroizing;

pub const CONTENT_TYPE: &str = "application/msrtc-media-relay-auth+xml";
pub const NAMESPACE: &str = "http://schemas.microsoft.com/2006/09/sip/mrasp";
/// Requested token lifetime in minutes (AV-03 default).
pub const DURATION_MINUTES: u32 = 480;
const MAX_RELAYS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadValue(&'static str),
    Xml(commscraft_xml::Error),
    WrongDocument(&'static str),
    /// The service answered with a failure (reasonPhrase, printable).
    Refused(String),
}

/// The request body (version 2.0, load-balanced, no location ⇒ intranet and internet relays,
/// AV-03/AV-05; K-AV-1: `route` would be an attribute of `request`, not used).
pub fn request(request_id: u32, from: &str, mras_uri: &str) -> Result<String, Error> {
    let ok = |u: &str| {
        u.len() <= 512
            && u.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("sip:"))
            && !u
                .chars()
                .any(|c| c.is_control() || c.is_whitespace() || "<>\"".contains(c))
    };
    if !ok(from) || !ok(mras_uri) {
        return Err(Error::BadValue("SIP URI"));
    }
    Ok(format!(
        "<request requestID=\"{request_id}\" from=\"{}\" version=\"2.0\" to=\"{}\" xmlns=\"{NAMESPACE}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><credentialsRequest credentialsRequestID=\"{request_id}\"><identity>{}</identity><duration>{DURATION_MINUTES}</duration></credentialsRequest></request>",
        escape(from),
        escape(mras_uri),
        escape(from)
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    Intranet,
    Internet,
}

/// Where a relay is reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayHost {
    Name(String),
    Address(IpAddr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relay {
    pub location: Option<Location>,
    pub host: RelayHost,
    pub udp_port: u16,
    pub tcp_port: u16,
}

/// The TURN credentials (opaque strings, AV-06) and their lifetime.
pub struct Credentials {
    pub username: String,
    pub password: Zeroizing<String>,
    pub duration_minutes: u32,
    pub relays: Vec<Relay>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("duration_minutes", &self.duration_minutes)
            .field("relays", &self.relays)
            .finish_non_exhaustive()
    }
}

fn token(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()
        && s.len() <= 512
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c)))
    .then(|| s.to_string())
}

fn dns_name(s: &str) -> Option<String> {
    let s = s.trim().trim_end_matches('.');
    let ok = !s.is_empty()
        && s.len() <= 253
        && s.split('.').all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    ok.then(|| s.to_ascii_lowercase())
}

fn text(e: &Element, name: &str) -> Option<String> {
    e.child(name).map(|c| c.text().trim().to_string())
}

fn port(e: &Element, name: &str, default: u16) -> Result<u16, Error> {
    match text(e, name) {
        None => Ok(default),
        Some(v) => v
            .parse::<u16>()
            .ok()
            .filter(|p| *p != 0)
            .ok_or(Error::BadValue("port")),
    }
}

/// Parses the answer of the Media Relay Authentication Service (§2.2.3).
pub fn parse_response(xml: &str, request_id: u32) -> Result<Credentials, Error> {
    let root = commscraft_xml::parse(
        xml,
        &Limits {
            max_input: 64 * 1024,
            max_depth: 8,
            ..Limits::default()
        },
    )
    .map_err(Error::Xml)?;
    if root.name != "response" || root.namespace.as_deref() != Some(NAMESPACE) {
        return Err(Error::WrongDocument("mrasp response"));
    }
    if root.attr("requestID") != Some(request_id.to_string().as_str()) {
        return Err(Error::WrongDocument("requestID"));
    }
    let reason = root.attr("reasonPhrase").unwrap_or("");
    if !reason.eq_ignore_ascii_case("OK") {
        let printable: String = reason
            .chars()
            .filter(|c| !c.is_control())
            .take(128)
            .collect();
        return Err(Error::Refused(printable));
    }
    let cr = root
        .child("credentialsResponse")
        .ok_or(Error::WrongDocument("credentialsResponse"))?;
    let creds = cr
        .child("credentials")
        .ok_or(Error::WrongDocument("credentials"))?;
    let username = text(creds, "username")
        .and_then(|u| token(&u))
        .ok_or(Error::BadValue("username"))?;
    let password = text(creds, "password")
        .and_then(|p| token(&p))
        .ok_or(Error::BadValue("password"))?;
    let duration_minutes = text(creds, "duration")
        .and_then(|d| d.parse::<u32>().ok())
        .unwrap_or(DURATION_MINUTES)
        .clamp(1, DURATION_MINUTES);
    let mut relays = Vec::new();
    if let Some(list) = cr.child("mediaRelayList") {
        for r in list.children_named("mediaRelay").take(MAX_RELAYS) {
            let location = match text(r, "location").as_deref() {
                Some("intranet") => Some(Location::Intranet),
                Some("internet") => Some(Location::Internet),
                _ => None,
            };
            let host = match (text(r, "hostName"), text(r, "directIPAddress")) {
                (Some(h), None) => {
                    RelayHost::Name(dns_name(&h).ok_or(Error::BadValue("hostName"))?)
                }
                (None, Some(a)) => {
                    RelayHost::Address(a.parse().map_err(|_| Error::BadValue("directIPAddress"))?)
                }
                // AV-04: hostName or directIPAddress, never both.
                _ => return Err(Error::BadValue("relay address")),
            };
            relays.push(Relay {
                location,
                host,
                udp_port: port(r, "udpPort", 3478)?,
                tcp_port: port(r, "tcpPort", 443)?,
            });
        }
    }
    Ok(Credentials {
        username,
        password: Zeroizing::new(password),
        duration_minutes,
        relays,
    })
}

#[cfg(test)]
mod tests;
