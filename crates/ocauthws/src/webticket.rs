//! Web tickets ([MS-OCAUTHWS] 20250520 §3.2, evidence file 09, AW-01 … AW-06): a SOAP web service
//! that wants one names its endpoint and the ticket issuer in its metadata (AW-01:
//! `wsdl:port` and `sp:IssuedToken/sp:Issuer/wsa:Address`); the client asks the issuer's
//! Windows-authentication binding (WebTicketServiceWinNegotiate: HTTP Negotiate/NTLM over HTTPS)
//! for a ticket with IssueToken (AW-03; example §4.2.1, p. 62) and sends the SAML assertion
//! (opaque, AW-04) in the WS-Security header of each request (AW-05), with WS-Addressing.
//!
//! K-AW-4 (found 2026-11-09): the service's policy names KeyType `…/Bearer` in its
//! `RequestSecurityTokenTemplate`, but the issuer refuses a bearer request ("Unsupported key type
//! in web ticket request", `tested-in-lab`). The request is therefore the one of the example
//! §4.2.1: KeyType SymmetricKey with at least 128 bits of client entropy (AW-03).
//!
//! Security by design: the assertion is copied byte for byte from the answer (never rebuilt,
//! its signature stays valid) and only after it parsed on its own as a SAML 1.1 Assertion;
//! endpoint URLs are taken only as HTTPS; metadata and answers are read with the bounded XML
//! reader.

use super::{printable, Error, NS_SOAP, NS_WSSE};
use commscraft_xml::{escape, parse_xs_datetime, Element, Limits};

const NS_TRUST: &str = "http://docs.oasis-open.org/ws-sx/ws-trust/200512";
const NS_SAML: &str = "urn:oasis:names:tc:SAML:1.0:assertion";
const NS_WSA: &str = "http://www.w3.org/2005/08/addressing";
const NS_POLICY: &str = "http://schemas.xmlsoap.org/ws/2004/09/policy";
const SAML11: &str = "http://docs.oasis-open.org/wss/oasis-wss-saml-token-profile-1.1#SAMLV1.1";
/// `soapAction` of IssueToken (the issuer's metadata).
pub const ISSUE_ACTION: &str = "\"http://docs.oasis-open.org/ws-sx/ws-trust/200512/RST/Issue\"";
/// Longest assertion accepted (design; the example is about 3 KiB).
pub const MAX_ASSERTION: usize = 64 * 1024;

/// A web service endpoint from the metadata, and who issues its tickets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub address: String,
    pub issuer: Option<String>,
}

fn descendants<'a>(e: &'a Element, name: &str, out: &mut Vec<&'a Element>) {
    for c in e.elements() {
        if c.name == name {
            out.push(c);
        }
        descendants(c, name, out);
    }
}

fn https(s: &str) -> Option<String> {
    let s = s.trim();
    (s.starts_with("https://")
        && s.len() <= 2048
        && s.chars()
            .all(|c| c.is_ascii_graphic() && !"<>\"'".contains(c)))
    .then(|| s.to_string())
}

/// The endpoints of a WSDL ("?singleWsdl" of the service's metadata, AW-01): each port's address
/// and the issuer of its binding's policy (`<binding>_policy`).
pub fn parse_metadata(wsdl: &str) -> Result<Vec<Endpoint>, Error> {
    let root = commscraft_xml::parse(
        wsdl,
        &Limits {
            max_input: 1024 * 1024,
            ..Limits::default()
        },
    )
    .map_err(Error::Xml)?;
    if root.name != "definitions" {
        return Err(Error::WrongDocument("wsdl:definitions"));
    }
    let mut policies = Vec::new();
    descendants(&root, "Policy", &mut policies);
    let issuer_of = |binding: &str| {
        let id = format!("{binding}_policy");
        let p = policies.iter().find(|p| {
            p.attributes
                .iter()
                .any(|a| a.name.ends_with("Id") && a.value == id)
        })?;
        let mut issuers = Vec::new();
        descendants(p, "Issuer", &mut issuers);
        issuers
            .iter()
            .filter_map(|i| i.child("Address"))
            .find_map(|a| https(&a.text()))
    };
    let mut out = Vec::new();
    for service in root.children_named("service") {
        for port in service.children_named("port") {
            let Some(address) = port
                .child("address")
                .and_then(|a| a.attr("location"))
                .and_then(https)
            else {
                continue;
            };
            let binding = port
                .attr("binding")
                .map(|b| b.rsplit(':').next().unwrap_or(b))
                .unwrap_or_default();
            out.push(Endpoint {
                address,
                issuer: issuer_of(binding),
            });
        }
    }
    Ok(out)
}

/// The IssueToken request for a ticket valid at `applies_to` (AW-03; example §4.2.1, p. 62):
/// `context` a fresh UUID, `entropy` at least 16 random bytes.
pub fn issue_token_request(
    applies_to: &str,
    context: &str,
    entropy: &[u8],
) -> Result<String, Error> {
    let to = https(applies_to).ok_or(Error::WrongDocument("AppliesTo"))?;
    if context.is_empty() || !context.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err(Error::BadDeviceId);
    }
    if entropy.len() < 16 {
        return Err(Error::WrongDocument("entropy"));
    }
    Ok(format!(
        "<s:Envelope xmlns:s=\"{NS_SOAP}\"><s:Body><RequestSecurityToken Context=\"{context}\" xmlns=\"{NS_TRUST}\">\
<TokenType>{SAML11}</TokenType><RequestType>http://schemas.xmlsoap.org/ws/2005/02/trust/Issue</RequestType>\
<AppliesTo xmlns=\"{NS_POLICY}\"><EndpointReference xmlns=\"{NS_WSA}\"><Address>{}</Address></EndpointReference></AppliesTo>\
<Entropy><BinarySecret>{}</BinarySecret></Entropy><KeyType>{NS_TRUST}/SymmetricKey</KeyType></RequestSecurityToken></s:Body></s:Envelope>",
        escape(&to),
        super::base64::encode(entropy)
    ))
}

/// A ticket: the SAML assertion as sent by the issuer and when it expires. The assertion
/// authorizes requests like a password: `Debug` shows only its length (security by design).
#[derive(Clone, PartialEq, Eq)]
pub struct WebTicket {
    pub assertion: String,
    pub expires: Option<u64>,
}

impl std::fmt::Debug for WebTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebTicket")
            .field(
                "assertion",
                &format_args!("<{} bytes redacted>", self.assertion.len()),
            )
            .field("expires", &self.expires)
            .finish()
    }
}

/// Reads the IssueToken answer (example §4.2.2, pp. 62–64): the assertion inside
/// `RequestedSecurityToken`, copied from the text, and `Lifetime/Expires`.
pub fn parse_issue_token_answer(xml: &str) -> Result<WebTicket, Error> {
    let root = commscraft_xml::parse(
        xml,
        &Limits {
            max_input: 256 * 1024,
            ..Limits::default()
        },
    )
    .map_err(Error::Xml)?;
    if root.name != "Envelope" || root.namespace.as_deref() != Some(NS_SOAP) {
        return Err(Error::WrongDocument("soap:Envelope"));
    }
    let body = root
        .child("Body")
        .ok_or(Error::WrongDocument("soap:Body"))?;
    if let Some(f) = body.child("Fault") {
        let text = f.child("faultstring").map(|s| s.text()).unwrap_or_default();
        return Err(Error::Fault(printable(&text)));
    }
    let mut responses = Vec::new();
    descendants(body, "RequestSecurityTokenResponse", &mut responses);
    let r = responses
        .first()
        .ok_or(Error::WrongDocument("RequestSecurityTokenResponse"))?;
    let expires = r
        .child("Lifetime")
        .and_then(|l| l.child("Expires"))
        .and_then(|e| parse_xs_datetime(&e.text()));
    // The assertion's text between the RequestedSecurityToken tags.
    let open = xml
        .find("RequestedSecurityToken>")
        .ok_or(Error::WrongDocument("RequestedSecurityToken"))?
        + "RequestedSecurityToken>".len();
    let rest = xml.get(open..).ok_or(Error::WrongDocument("assertion"))?;
    let close = rest
        .find("RequestedSecurityToken>")
        .and_then(|i| rest.get(..i)?.rfind("</"))
        .ok_or(Error::WrongDocument("assertion"))?;
    let assertion = rest
        .get(..close)
        .ok_or(Error::WrongDocument("assertion"))?
        .trim()
        .to_string();
    if assertion.len() > MAX_ASSERTION {
        return Err(Error::WrongDocument("assertion too long"));
    }
    let a = commscraft_xml::parse(&assertion, &Limits::default()).map_err(Error::Xml)?;
    if a.name != "Assertion" || a.namespace.as_deref() != Some(NS_SAML) {
        return Err(Error::WrongDocument("saml:Assertion"));
    }
    Ok(WebTicket { assertion, expires })
}

/// A SOAP 1.1 request to a web-ticket service (AW-05): WS-Addressing Action, MessageID and To,
/// and the ticket in the WS-Security header; `action` without quotes, `message_id` a UUID.
pub fn secured_envelope(
    action: &str,
    to: &str,
    message_id: &str,
    ticket: &WebTicket,
    body: &str,
) -> Result<String, Error> {
    let to = https(to).ok_or(Error::WrongDocument("To"))?;
    if !message_id
        .chars()
        .all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        return Err(Error::BadDeviceId);
    }
    Ok(format!(
        "<s:Envelope xmlns:s=\"{NS_SOAP}\" xmlns:a=\"{NS_WSA}\"><s:Header>\
<a:Action s:mustUnderstand=\"1\">{}</a:Action><a:MessageID>urn:uuid:{message_id}</a:MessageID>\
<a:To s:mustUnderstand=\"1\">{}</a:To>\
<o:Security s:mustUnderstand=\"1\" xmlns:o=\"{NS_WSSE}\">{}</o:Security></s:Header><s:Body>{body}</s:Body></s:Envelope>",
        escape(action.trim_matches('"')),
        escape(&to),
        ticket.assertion
    ))
}
