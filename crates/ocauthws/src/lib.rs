//! Certificate provisioning for the TLS-DSK sign-in (function 6): the client sends a PKCS#10
//! request to the server's certificate provisioning service and gets back the user certificate
//! ([MS-OCAUTHWS] 20250520 `GetAndPublishCert`, evidence file 09, AW-10 … AW-15).
//!
//! The service's Windows-authentication binding (`CertProvisioningServiceWin`: HTTP Negotiate or
//! NTLM over HTTPS, at the `sts-uri` of the TLS-DSK challenge itself) needs no web ticket
//! (inference from the service's metadata; `tested-in-lab`). The caller sends the request with
//! its HTTP authentication and hands the answer to [`parse_certificate_answer`].
//!
//! Security by design: entity and device ID are checked and escaped; the answer is read with the
//! bounded XML reader; the certificate is taken over only when the response class is Success,
//! the disposition "Issued", the entity the one asked for and the certificate well-formed DER;
//! the caller still checks that the certificate carries the requested public key.

pub mod webticket;

use commscraft_sipae::base64;
use commscraft_xml::{escape, Element, Limits};

pub const CONTENT_TYPE: &str = "text/xml; charset=utf-8";
pub const SOAP_ACTION: &str =
    "\"http://schemas.microsoft.com/OCS/AuthWebServices/GetAndPublishCert\"";
const NS_SOAP: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const NS_AUTH: &str = "http://schemas.microsoft.com/OCS/AuthWebServices/";
const NS_TRUST: &str = "http://docs.oasis-open.org/ws-sx/ws-trust/200512/";
const NS_WSSE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";
const X509V3: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-x509-token-profile-1.0#X509v3";
const BASE64BINARY: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd#base64binary";
const PKCS10: &str = "http://schemas.microsoft.com/OCS/AuthWebServices.xsd#PKCS10";
/// Largest certificate taken over (design).
pub const MAX_CERTIFICATE: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadEntity,
    BadDeviceId,
    Xml(commscraft_xml::Error),
    WrongDocument(&'static str),
    /// SOAP fault (fault string, printable).
    Fault(String),
    /// ResponseClass other than Success, with the error code if given (§3.1.4.1.4.1).
    Refused(String),
}

/// The entity: the SIP URI without `sip:` (§3.1.4.1.5.2), printable and bounded.
fn entity(aor: &str) -> Result<String, Error> {
    let e = aor.trim().strip_prefix("sip:").unwrap_or(aor.trim());
    let ok = e.len() <= 256
        && e.split_once('@')
            .is_some_and(|(u, d)| !u.is_empty() && d.contains('.'))
        && e.chars()
            .all(|c| c.is_ascii_alphanumeric() || "@._-+".contains(c));
    ok.then(|| e.to_string()).ok_or(Error::BadEntity)
}

/// The device ID in braces, as GUID text (§3.1.4.1.5.1: "MUST be convertible to a GUID").
fn device_id(id: &str) -> Result<String, Error> {
    let g = id.trim().trim_start_matches('{').trim_end_matches('}');
    let groups: Vec<&str> = g.split('-').collect();
    let ok = groups.len() == 5
        && groups
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(p, n)| p.len() == n && p.chars().all(|c| c.is_ascii_hexdigit()));
    ok.then(|| format!("{{{}}}", g.to_ascii_uppercase()))
        .ok_or(Error::BadDeviceId)
}

/// The GetAndPublishCert request (§3.1.4.1.2.3, pp. 26–27; example §4.1.1, p. 60) for the user
/// `aor` on the device `device` with the DER-encoded PKCS#10 request `csr`.
///
/// K-AW-2 (found 2026-11-09): the example's SOAP header (`To` and `Action` in the namespace
/// `…/addressing/none`, `mustUnderstand="1"`) makes the service answer 400 without a body; without
/// a header (the SOAPAction HTTP header names the operation) it issues the certificate
/// (`tested-in-lab`). WS-Addressing 1.0 headers give 500. So the request has no SOAP header.
pub fn certificate_request(
    aor: &str,
    device: &str,
    csr: &[u8],
    request_id: &str,
) -> Result<String, Error> {
    let entity = entity(aor)?;
    let device = device_id(device)?;
    let request_id = device_id(request_id)?;
    let request_id = request_id.trim_start_matches('{').trim_end_matches('}');
    Ok(format!(
        "<s:Envelope xmlns:s=\"{NS_SOAP}\"><s:Body>\
<GetAndPublishCert DeviceId=\"{device}\" Entity=\"{}\" xmlns=\"{NS_AUTH}\">\
<RequestSecurityToken xmlns=\"{NS_TRUST}\">\
<TokenType>{X509V3}</TokenType>\
<RequestType>http://docs.oasis-open.org/ws-sx/ws-trust/200512/Issue</RequestType>\
<BinarySecurityToken EncodingType=\"{BASE64BINARY}\" ValueType=\"{PKCS10}\" xmlns=\"{NS_WSSE}\">{}</BinarySecurityToken>\
<RequestID xmlns=\"http://schemas.microsoft.com/windows/pki/2009/01/enrollment\">{}</RequestID>\
</RequestSecurityToken></GetAndPublishCert></s:Body></s:Envelope>",
        escape(&entity),
        base64::encode(csr),
        request_id.to_ascii_lowercase()
    ))
}

fn printable(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(256)
        .collect::<String>()
        .trim()
        .to_string()
}

/// A DER SEQUENCE whose length fills exactly the input (a certificate at least looks like one).
fn whole_der_sequence(b: &[u8]) -> bool {
    let (Some(&0x30), Some(&l)) = (b.first(), b.get(1)) else {
        return false;
    };
    let (len, head) = match l {
        0..=0x7f => (usize::from(l), 2),
        0x81..=0x83 => {
            let n = usize::from(l & 0x7f);
            let Some(bytes) = b.get(2..2 + n) else {
                return false;
            };
            (
                bytes.iter().fold(0usize, |a, x| (a << 8) | usize::from(*x)),
                2 + n,
            )
        }
        _ => return false,
    };
    head + len == b.len()
}

/// Reads the GetAndPublishCert answer (§3.1.4.1.2.4, pp. 27–28; example §4.1.2, pp. 60–62): the
/// issued certificate (DER).
pub fn parse_certificate_answer(xml: &str, aor: &str) -> Result<Vec<u8>, Error> {
    let want = entity(aor)?;
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
    let r = body
        .child("GetAndPublishCertResponse")
        .filter(|r| r.namespace.as_deref() == Some(NS_AUTH))
        .ok_or(Error::WrongDocument("GetAndPublishCertResponse"))?;
    if r.attr("ResponseClass") != Some("Success") {
        let code = r
            .elements()
            .find(|e| e.name == "Error" || e.name == "ErrorCode")
            .map(|e| printable(&e.text()))
            .or_else(|| r.attr("ResponseClass").map(printable))
            .unwrap_or_else(|| "no response class".into());
        return Err(Error::Refused(code));
    }
    if r.attr("Entity")
        .is_some_and(|e| !e.eq_ignore_ascii_case(&want))
    {
        return Err(Error::WrongDocument("Entity"));
    }
    let rstr = r
        .child("RequestSecurityTokenResponse")
        .ok_or(Error::WrongDocument("RequestSecurityTokenResponse"))?;
    let disposition = rstr
        .child("DispositionMessage")
        .map(|d| printable(&d.text()))
        .unwrap_or_default();
    if disposition != "Issued" {
        return Err(Error::Refused(disposition));
    }
    let token = rstr
        .child("RequestedSecurityToken")
        .and_then(|t| t.child("BinarySecurityToken"))
        .filter(|t: &&Element| t.attr("ValueType") == Some(X509V3))
        .ok_or(Error::WrongDocument("RequestedSecurityToken"))?;
    let text: String = token
        .text()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if text.len() > MAX_CERTIFICATE * 4 / 3 + 4 {
        return Err(Error::WrongDocument("certificate too large"));
    }
    let der = base64::decode(&text).map_err(|_| Error::WrongDocument("certificate base64"))?;
    if !whole_der_sequence(&der) {
        return Err(Error::WrongDocument("certificate DER"));
    }
    Ok(der)
}

#[cfg(test)]
mod tests;
