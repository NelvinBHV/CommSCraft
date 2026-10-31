//! Address book search over the Group Expansion / Address Book Web Query service (MS-DLX
//! `SearchAbEntry`, basic search).
//!
//! Evidence: `docs/protocol/evidence/28-dlx-distribution-list-address-book-search.md` (DL-01 …
//! DL-07, OB-44, K-DL-3). Sources: [MS-DLX] 20250520 §3.1.4.2 (pp. 22–31), schema §6 (pp. 58–65),
//! examples §4.4–4.6 (pp. 43–45); endpoint from provisioning (`abwq*`/`dlx*`, OB-44).
//! Secondary: [SIPE] `sipe-svc.c` l. 406–426 (wrapped request, FromDialPad, MaxResultNum,
//! ReturnList with `msRTCSIP-PrimaryUserAddress`).
//!
//! Security by design: the search text is length-bounded and XML-escaped; the response is parsed
//! with the bounded XML reader (no DTD); returned SIP URIs are checked before use; all display
//! text is stripped of control characters; result count bounded.

use commscraft_xml::{escape, Element, Limits};

pub const SOAP_ACTION: &str = "\"DistributionListExpander/IAddressBook/SearchAbEntry\"";
pub const CONTENT_TYPE: &str = "text/xml; charset=utf-8";
const NS_SOAP: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const NS_DLX: &str = "DistributionListExpander";
pub const MAX_QUERY: usize = 256;
pub const MAX_RESULTS: u32 = 20;
/// Attributes searched (AD names, MS-DLX §3.1.1 <1>; the same set as SIPE, secondary).
const SEARCH_LIST: &str = "displayName,givenName,sn,mail,mailNickname,msRTCSIP-PrimaryUserAddress";
const RETURN_LIST: &str =
    "displayName,msRTCSIP-PrimaryUserAddress,mail,title,department,company,telephoneNumber,mobile";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    EmptyQuery,
    QueryTooLong,
    Xml(commscraft_xml::Error),
    /// Not a SOAP envelope with a SearchAbEntry answer.
    WrongDocument(&'static str),
    /// SOAP fault from the server (fault string, printable).
    Fault(String),
    /// ResponseCode other than Succeeded/NoEntryFound.
    Server(String),
}

/// The SOAP request of a prefix search ("BeginsWith") over names, mail and SIP address.
/// Wrapped in `SearchAbEntry` as the WSDL requires (document/literal; K-DL-3: the examples of §4
/// show the body without the wrapper and without the required Metadata elements).
pub fn search_request(query: &str) -> Result<String, Error> {
    let q = query.trim();
    if q.is_empty() {
        return Err(Error::EmptyQuery);
    }
    if q.chars().count() > MAX_QUERY {
        return Err(Error::QueryTooLong);
    }
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"{NS_SOAP}\"><soap:Body>\
<SearchAbEntry xmlns=\"{NS_DLX}\"><AbEntryRequest><BasicSearch><SearchList>{SEARCH_LIST}</SearchList>\
<Value>{}</Value><Verb>BeginsWith</Verb></BasicSearch><Metadata><FromDialPad>false</FromDialPad>\
<MaxResultNum>{MAX_RESULTS}</MaxResultNum><ReturnList>{RETURN_LIST}</ReturnList></Metadata>\
</AbEntryRequest></SearchAbEntry></soap:Body></soap:Envelope>",
        escape(q)
    ))
}

/// One found entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    pub display_name: Option<String>,
    /// Checked `sip:` URI (from `msRTCSIP-PrimaryUserAddress`).
    pub sip_uri: Option<String>,
    pub mail: Option<String>,
    pub title: Option<String>,
    pub department: Option<String>,
    pub company: Option<String>,
    pub phone: Option<String>,
}

fn printable(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !c.is_control()
                && !('\u{202a}'..='\u{202e}').contains(c)
                && !('\u{2066}'..='\u{2069}').contains(c)
        })
        .take(256)
        .collect::<String>()
        .trim()
        .to_string()
}

fn sip_uri(v: &str) -> Option<String> {
    let v = v.trim();
    let ok = v.len() <= 256
        && v.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("sip:"))
        && v.contains('@')
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || ":@._-+".contains(c));
    ok.then(|| v.to_string())
}

fn entry(e: &Element) -> Entry {
    let mut out = Entry::default();
    let Some(attrs) = e.child("Attributes") else {
        return out;
    };
    for a in attrs.children_named("Attribute") {
        let name = a
            .child("Name")
            .map(|n| n.text().trim().to_ascii_lowercase())
            .unwrap_or_default();
        let value = a.child("Value").map(|v| v.text()).or_else(|| {
            a.child("Values")
                .and_then(|vs| vs.elements().next().map(|s| s.text()))
        });
        let Some(value) = value else { continue };
        match name.as_str() {
            "displayname" => out.display_name = Some(printable(&value)),
            "msrtcsip-primaryuseraddress" => out.sip_uri = sip_uri(&value),
            "mail" => out.mail = Some(printable(&value)),
            "title" => out.title = Some(printable(&value)),
            "department" => out.department = Some(printable(&value)),
            "company" => out.company = Some(printable(&value)),
            "telephonenumber" | "mobile" if out.phone.is_none() => {
                out.phone = Some(printable(&value))
            }
            _ => {}
        }
    }
    out
}

/// Parses the SOAP answer (§2.2.5.1, §3.1.4.2.3.8 ff.). Accepts the wrapped form of the WSDL
/// (`SearchAbEntryResponse/SearchAbEntryResult`) and the bare `AbEntryResponse` of the examples.
pub fn parse_response(xml: &str) -> Result<Vec<Entry>, Error> {
    let root = commscraft_xml::parse(
        xml,
        &Limits {
            max_input: 1024 * 1024,
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
    let result = match body.child("SearchAbEntryResponse") {
        Some(w) => w
            .child("SearchAbEntryResult")
            .ok_or(Error::WrongDocument("SearchAbEntryResult"))?,
        None => body
            .child("AbEntryResponse")
            .ok_or(Error::WrongDocument("AbEntryResponse"))?,
    };
    if result.namespace.as_deref() != Some(NS_DLX) {
        return Err(Error::WrongDocument("namespace"));
    }
    let code = result
        .child("Metadata")
        .and_then(|m| m.child("ResponseCode"))
        .map(|c| c.text().trim().to_string())
        .unwrap_or_else(|| "Succeeded".into());
    match code.as_str() {
        "Succeeded" => {}
        "NoEntryFound" => return Ok(Vec::new()),
        other => return Err(Error::Server(printable(other))),
    }
    let mut out: Vec<Entry> = Vec::new();
    if let Some(items) = result.child("Items") {
        for e in items.children_named("AbEntry") {
            let en = entry(e);
            // The answer can contain duplicates (secondary: SIPE sipe-buddy.c l. 1146).
            if !out
                .iter()
                .any(|x| x.sip_uri.is_some() && x.sip_uri == en.sip_uri)
            {
                out.push(en);
            }
            if out.len() >= MAX_RESULTS as usize * 5 {
                break;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
