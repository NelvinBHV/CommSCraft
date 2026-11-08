//! Address book search over the Group Expansion / Address Book Web Query service (MS-DLX
//! `SearchAbEntry`, basic search) and distribution list expansion (`ExpandDistributionList`,
//! DL-04).
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
const RETURN_LIST: &str = "displayName,msRTCSIP-PrimaryUserAddress,mail,title,department,company,\
telephoneNumber,mobile,PhotoRelPath,PhotoHash,PhotoSize";
/// Longest photo path and hash accepted (design).
const MAX_PHOTO_NAME: usize = 128;

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
    /// The entry's photo (DL-06): a checked file name relative to the address book folder.
    pub photo: Option<Photo>,
}

/// Where an entry's photo lies and which state it has (DL-06: PhotoRelPath, PhotoHash,
/// PhotoSize; the path is relative to the address book file folder, [MS-ABS] §1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Photo {
    /// Only letters, digits, `.`, `-`, `_`; no path separators, no leading dot.
    pub rel_path: String,
    /// Changes when the photo changes (letters, digits, `-` only); usable as a cache key.
    pub hash: Option<String>,
    pub size: Option<u32>,
}

/// A photo file name we fetch: one path segment, no traversal (security by design).
fn photo_path(v: &str) -> Option<String> {
    let v = v.trim();
    let ok = !v.is_empty()
        && v.len() <= MAX_PHOTO_NAME
        && !v.starts_with('.')
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
    ok.then(|| v.to_string())
}

fn photo_hash(v: &str) -> Option<String> {
    let v = v.trim();
    let ok = !v.is_empty()
        && v.len() <= MAX_PHOTO_NAME
        && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    ok.then(|| v.to_ascii_lowercase())
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
    let (mut path, mut hash, mut size) = (None, None, None);
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
            "photorelpath" => path = photo_path(&value),
            "photohash" => hash = photo_hash(&value),
            "photosize" => size = value.trim().parse::<u32>().ok(),
            _ => {}
        }
    }
    out.photo = path.map(|rel_path| Photo {
        rel_path,
        hash,
        size,
    });
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

/// SOAPAction of ExpandDistributionList as the `AddressBookWinNegotiate` binding prints it (DL-03;
/// K-DL-1 resolved by OB-44: without the space of p. 63).
pub const EXPAND_SOAP_ACTION: &str = "\"DistributionListExpander/ExpandDistributionList\"";
/// Longest distribution list address accepted (design).
pub const MAX_GROUP_ADDRESS: usize = 256;
/// Most members read from one answer (design; the server's default limit is 100, DL-04).
pub const MAX_MEMBERS: usize = 1000;

/// The request to expand the distribution list with the mail address `group` (DL-04;
/// §3.1.4.1.3.1: exactly one `groupMailAddress`).
pub fn expand_request(group: &str) -> Result<String, Error> {
    let g = group.trim();
    if g.is_empty() {
        return Err(Error::EmptyQuery);
    }
    if g.chars().count() > MAX_GROUP_ADDRESS || g.chars().any(char::is_control) {
        return Err(Error::QueryTooLong);
    }
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"{NS_SOAP}\"><soap:Body>\
<ExpandDistributionList xmlns=\"{NS_DLX}\"><groupMailAddress>{}</groupMailAddress></ExpandDistributionList>\
</soap:Body></soap:Envelope>",
        escape(g)
    ))
}

/// ResponseState of an expansion (§3.1.4.1.4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandStatus {
    Success,
    /// The list has more members than the server returns (default above 100).
    MemberCountLimitExceeded,
    NotAuthorized,
    NotFound,
    SimultaneousRequestLimitExceeded,
    /// `Invalid` or an unknown value.
    Invalid,
}

/// A member of a distribution list (ActiveDirectoryObjectInfo, §3.1.4.1.3.5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Member {
    pub display_name: Option<String>,
    pub mail: Option<String>,
    pub mail_nickname: Option<String>,
    /// Checked `sip:` URI.
    pub sip_uri: Option<String>,
}

/// The answer to an expansion: status, users and nested lists (DlxGroup, §3.1.4.1.3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expansion {
    pub status: ExpandStatus,
    pub users: Vec<Member>,
    pub nested_groups: Vec<Member>,
}

fn members(list: Option<&Element>) -> Vec<Member> {
    let Some(list) = list else {
        return Vec::new();
    };
    list.children_named("ActiveDirectoryObjectInfo")
        .take(MAX_MEMBERS)
        .map(|m| {
            let text = |name: &str| {
                m.child(name)
                    .map(|v| printable(&v.text()))
                    .filter(|v| !v.is_empty())
            };
            Member {
                display_name: text("displayName"),
                mail: text("mail"),
                mail_nickname: text("mailNickname"),
                sip_uri: m.child("sipUri").and_then(|v| sip_uri(&v.text())),
            }
        })
        .collect()
}

/// Parses the SOAP answer of an expansion (§3.1.4.1.3.2 ff.).
pub fn parse_expand_response(xml: &str) -> Result<Expansion, Error> {
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
    let result = body
        .child("ExpandDistributionListResponse")
        .and_then(|r| r.child("ExpandDistributionListResult"))
        .ok_or(Error::WrongDocument("ExpandDistributionListResult"))?;
    if result.namespace.as_deref() != Some(NS_DLX) {
        return Err(Error::WrongDocument("namespace"));
    }
    let status = match result
        .child("ResponseStatus")
        .map(|s| s.text().trim().to_string())
        .as_deref()
    {
        Some("Success") => ExpandStatus::Success,
        Some("MemberCountLimitExceeded") => ExpandStatus::MemberCountLimitExceeded,
        Some("NotAuthorized") => ExpandStatus::NotAuthorized,
        Some("NotFound") => ExpandStatus::NotFound,
        Some("SimultaneousRequestLimitExceeded") => ExpandStatus::SimultaneousRequestLimitExceeded,
        _ => ExpandStatus::Invalid,
    };
    Ok(Expansion {
        status,
        users: members(result.child("Users")),
        nested_groups: members(result.child("NestedGroups")),
    })
}

#[cfg(test)]
mod tests;
