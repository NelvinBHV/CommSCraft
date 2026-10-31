//! Emergency location (function 39, M4): the location information service request and answer of
//! [MS-E911WS] 20250520 (`GetLocations`, SOAP 1.1 over HTTPS) and the PIDF-LO of an emergency call
//! ([MS-SIPRE] 20250520 §2.2.23 and example §4.14: RFC 3863 presence with a GEOPRIV location
//! object, RFC 4119, civic address RFC 5139, Microsoft extension `msftE911PidfExtn`). Sans-I/O; the
//! HTTPS transport is in the client, the SIP side in `commscraft-session`.
//!
//! Evidence: `docs/protocol/evidence/29-e911ws-location-service.md` (E9-01 … E9-07), file 06 RE-67, file
//! 04 RG-51. Sources: [MS-E911WS] §2.2, §3.1.4.1 (pp. 11–18), example §4 (p. 23), WSDL §6
//! (`soapAction="LIService/GetLocations"`, p. 33); [MS-SIPRE] §2.2.23 (p. 36), §4.14 (p. 94).
//!
//! Security by design: the request contains only checked values (SIP URI, IP addresses, MAC in
//! the documented form); answers are parsed with the bounded reader, at most 16 locations with
//! bounded, printable fields; the PIDF-LO escapes every value and marks addresses the user typed
//! as `Manual` (not verified by the network, E9-04).

use std::net::IpAddr;

use commscraft_xml::{escape, xs_datetime, Element, Limits};

pub const CONTENT_TYPE: &str = "text/xml; charset=utf-8";
pub const SOAP_ACTION: &str = "\"LIService/GetLocations\"";
pub const NS_LIS: &str = "urn:schema:Microsoft.Rtc.WebComponent.Lis.2010";
const NS_SOAP: &str = "http://schemas.xmlsoap.org/soap/envelope/";
pub const PIDF_TYPE: &str = "application/pidf+xml";
const NS_PIDF: &str = "urn:ietf:params:xml:ns:pidf";
const NS_GP: &str = "urn:ietf:params:xml:ns:pidf:geopriv10";
const NS_BP: &str = "urn:ietf:params:xml:ns:pidf:geopriv10:basicPolicy";
const NS_CA: &str = "urn:ietf:params:xml:ns:pidf:geopriv10:civicAddr";
const NS_MS: &str = "urn:schema:Rtc.LIS.msftE911PidfExtn.2008";
const NS_TAG: &str = "urn:schema:Rtc.Lis.LocationPolicyTagID.2008";

pub const MAX_LOCATIONS: usize = 16;
const MAX_FIELD: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Xml(commscraft_xml::Error),
    WrongDocument,
    BadValue(&'static str),
}

/// A civic address with the fields the client evaluates (E9-04: country, A1, A3, PRD, RD, STS,
/// POD, HNO, HNS, LOC, NAM, PC; RFC 5139).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Location {
    pub country: String,
    pub a1: String,
    pub a3: String,
    pub prd: String,
    pub rd: String,
    pub sts: String,
    pub pod: String,
    pub hno: String,
    pub hns: String,
    pub loc: String,
    pub nam: String,
    pub pc: String,
    /// The address was typed by the user (method `Manual`, not verified by the network).
    pub manual: bool,
}

impl Location {
    fn fields(&self) -> [(&'static str, &str); 12] {
        [
            ("country", &self.country),
            ("A1", &self.a1),
            ("A3", &self.a3),
            ("PRD", &self.prd),
            ("RD", &self.rd),
            ("STS", &self.sts),
            ("POD", &self.pod),
            ("HNO", &self.hno),
            ("HNS", &self.hns),
            ("LOC", &self.loc),
            ("NAM", &self.nam),
            ("PC", &self.pc),
        ]
    }

    /// One line for the user: street, house number, place, country.
    pub fn summary(&self) -> String {
        let parts = [
            format!("{} {} {} {}", self.prd, self.rd, self.sts, self.pod),
            format!("{}{}", self.hno, self.hns),
            self.loc.clone(),
            self.nam.clone(),
            format!("{} {}", self.pc, self.a3),
            format!("{} {}", self.a1, self.country),
        ];
        parts
            .iter()
            .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The network identifiers of a GetLocations request (E9-03; §3.1.4.1.1.1, pp. 16–17).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkIds {
    /// MAC address of the local interface, `12-22-22-22-22-22` as in the example (§4).
    pub mac: Option<String>,
    pub subnet: Option<IpAddr>,
    pub ip: Option<IpAddr>,
}

fn valid_mac(m: &str) -> bool {
    m.len() == 17
        && m.split('-').count() == 6
        && m.split('-')
            .all(|b| b.len() == 2 && b.chars().all(|c| c.is_ascii_hexdigit()))
}

/// GetLocationsRequest (E9-03): Entity (own SIP URI, 1–64 characters), then the identifiers in
/// the schema's order (MAC, SubnetID, IP). Namespace of the schema (K-E9-1).
pub fn get_locations_request(entity: &str, ids: &NetworkIds) -> Result<String, Error> {
    let entity_ok = (1..=64).contains(&entity.chars().count())
        && entity
            .get(..4)
            .is_some_and(|s| s.eq_ignore_ascii_case("sip:"))
        && !entity
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "<>\"'&".contains(c));
    if !entity_ok {
        return Err(Error::BadValue("Entity"));
    }
    let mut ident = String::new();
    if let Some(m) = &ids.mac {
        if !valid_mac(m) {
            return Err(Error::BadValue("MAC"));
        }
        ident.push_str(&format!("<MAC>{m}</MAC>"));
    }
    if let Some(s) = ids.subnet {
        ident.push_str(&format!("<SubnetID>{s}</SubnetID>"));
    }
    if let Some(ip) = ids.ip {
        ident.push_str(&format!("<IP>{ip}</IP>"));
    }
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"{NS_SOAP}\"><soap:Body><GetLocationsRequest xmlns=\"{NS_LIS}\"><Entity>{entity}</Entity>{ident}</GetLocationsRequest></soap:Body></soap:Envelope>"
    ))
}

fn limits() -> Limits {
    Limits {
        max_input: 256 * 1024,
        max_depth: 16,
        ..Limits::default()
    }
}

fn field(e: &Element, name: &str) -> String {
    e.child(name)
        .map(|c| {
            c.text()
                .trim()
                .chars()
                .filter(|c| !c.is_control())
                .take(MAX_FIELD)
                .collect()
        })
        .unwrap_or_default()
}

fn find<'a>(e: &'a Element, path: &[&str]) -> Option<&'a Element> {
    path.iter().try_fold(e, |e, n| e.child(n))
}

/// GetLocationsResponse (E9-04; §2.2.3–2.2.5, pp. 12–14; example §4): the return code (200, 400,
/// 404, 500) and the civic addresses of the presence list.
pub fn parse_get_locations(xml: &str) -> Result<(u16, Vec<Location>), Error> {
    let root = commscraft_xml::parse(xml, &limits()).map_err(Error::Xml)?;
    if root.name != "Envelope" {
        return Err(Error::WrongDocument);
    }
    let resp = find(&root, &["Body", "GetLocationsResponse"]).ok_or(Error::WrongDocument)?;
    let code = resp
        .child("ReturnCode")
        .and_then(|c| c.text().trim().parse::<u16>().ok())
        .ok_or(Error::BadValue("ReturnCode"))?;
    let mut out = Vec::new();
    if let Some(list) = resp.child("presenceList") {
        for p in list.children_named("presence").take(MAX_LOCATIONS) {
            for t in p.children_named("tuple") {
                let Some(gp) = find(t, &["status", "geopriv"]) else {
                    continue;
                };
                let Some(ca) = find(gp, &["location-info", "civicAddress"]) else {
                    continue;
                };
                let manual = gp
                    .child("method")
                    .is_some_and(|m| m.text().trim().eq_ignore_ascii_case("Manual"));
                if out.len() < MAX_LOCATIONS {
                    out.push(Location {
                        country: field(ca, "country"),
                        a1: field(ca, "A1"),
                        a3: field(ca, "A3"),
                        prd: field(ca, "PRD"),
                        rd: field(ca, "RD"),
                        sts: field(ca, "STS"),
                        pod: field(ca, "POD"),
                        hno: field(ca, "HNO"),
                        hns: field(ca, "HNS"),
                        loc: field(ca, "LOC"),
                        nam: field(ca, "NAM"),
                        pc: field(ca, "PC"),
                        manual,
                    });
                }
            }
        }
    }
    Ok((code, out))
}

/// What the PIDF-LO of an emergency call carries besides the address (locationPolicy, RG-51).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extension {
    pub conference_uri: Option<String>,
    pub conference_mode: Option<String>,
    pub tag_id: Option<String>,
}

/// The Content-ID of the PIDF-LO part: the address of the caller without the scheme, as in the
/// example (`<voip_911_user1@contoso.com>`, §4.14).
pub fn content_id(entity: &str) -> String {
    entity
        .strip_prefix("sip:")
        .or_else(|| entity.strip_prefix("SIP:"))
        .unwrap_or(entity)
        .chars()
        .filter(|c| !c.is_control() && !"<>\" ".contains(*c))
        .collect()
}

/// The PIDF-LO of an emergency call (RE-67; [MS-SIPRE] §2.2.23, example §4.14; RFC 4119):
/// presence for `entity` with one tuple whose status holds the GEOPRIV object (civic address,
/// usage rules, `Manual` for an address the network did not verify) and the
/// `msftE911PidfExtn` (conference URI and mode, LocationPolicyTagID), and a timestamp.
pub fn pidf_lo(entity: &str, loc: &Location, ext: &Extension, now: u64) -> Result<String, Error> {
    if !entity
        .get(..4)
        .is_some_and(|s| s.eq_ignore_ascii_case("sip:"))
    {
        return Err(Error::BadValue("entity"));
    }
    let timestamp = xs_datetime(now).ok_or(Error::BadValue("timestamp"))?;
    let civic: String = loc
        .fields()
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(n, v)| format!("<ca:{n}>{}</ca:{n}>", escape(v)))
        .collect();
    let method = if loc.manual {
        "<gp:method>Manual</gp:method>"
    } else {
        ""
    };
    let mut ms = String::new();
    if let Some(u) = &ext.conference_uri {
        ms.push_str(&format!(
            "<ms:ConferenceUri>{}</ms:ConferenceUri>",
            escape(u)
        ));
    }
    if let Some(m) = &ext.conference_mode {
        ms.push_str(&format!(
            "<ms:ConferenceMode>{}</ms:ConferenceMode>",
            escape(m)
        ));
    }
    if let Some(t) = &ext.tag_id {
        ms.push_str(&format!(
            "<LocationPolicyTagID xmlns=\"{NS_TAG}\">{}</LocationPolicyTagID>",
            escape(t)
        ));
    }
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><presence xmlns=\"{NS_PIDF}\" xmlns:gp=\"{NS_GP}\" xmlns:bp=\"{NS_BP}\" xmlns:ca=\"{NS_CA}\" xmlns:ms=\"{NS_MS}\" entity=\"{}\"><tuple id=\"0\"><status><gp:geopriv><gp:location-info><ca:civicAddress>{civic}</ca:civicAddress></gp:location-info><gp:usage-rules><bp:retransmission-allowed>true</bp:retransmission-allowed></gp:usage-rules>{method}</gp:geopriv><ms:msftE911PidfExtn>{ms}</ms:msftE911PidfExtn></status><timestamp>{timestamp}</timestamp></tuple></presence>",
        escape(entity)
    ))
}

#[cfg(test)]
mod tests;
