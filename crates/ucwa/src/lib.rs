//! UCWA 2.0 for contact photos (function 15, second path): the messages of the Skype for
//! Business web API that lead from Autodiscover to a contact's photo, as a sans-I/O crate. The
//! caller performs the HTTPS requests.
//!
//! Evidence: `docs/protocol/evidence/40-ucwa-contact-photos.md` (UW-01 … UW-10). Sources:
//! [MS-OCDISCWS] 20250520 §2.2.5.5 (pp. 16–17), §3.1.5.5 (pp. 21–22) for the `OAuth` link and the
//! `Internal/Ucwa`/`External/Ucwa` links of the User resource; the official UCWA 2.0 reference
//! (Microsoft, "Applies to: Skype for Business 2015", source repository OfficeDev/skype-docs,
//! commit d6b0925, pages dated 2022-04-11): "Authentication in UCWA", "Create an application",
//! applications, contact, contactPhoto, photo, myContacts, search, distributionGroup,
//! expandDistributionGroup.
//!
//! Security by design:
//! - every answer is hostile: XML through `commscraft-xml` with tight limits, the token answer
//!   through a bounded flat JSON reader; every length is checked;
//! - a link from an answer is followed only to the host (and port) of the UCWA service it came
//!   from, over HTTPS ([`resolve`]); the OAuth token goes nowhere else;
//! - only the `urn:microsoft.rtc:windows` grant is used: the caller authenticates the token
//!   request with NTLM over HTTP, so the password never travels in a form field.

use commscraft_url::{parse_https, HttpsUrl};
use commscraft_xml::{Element, Limits};

/// The UCWA XML namespace (UW-06).
pub const NS: &str = "http://schemas.microsoft.com/rtc/2012/03/ucwa";
/// The media type of the UCWA XML representation (UW-06).
pub const XML: &str = "application/xml";
/// The Autodiscover media type for the User resource ([MS-OCDISCWS]).
pub const AUTODISCOVER_XML: &str = "application/vnd.microsoft.rtc.autodiscover+xml;v=1";
/// The grant used (UW-02): Windows authentication of the token request.
pub const WINDOWS_GRANT: &str = "urn:microsoft.rtc:windows";
/// Largest UCWA document read (design: a contact list or a large expanded group).
pub const MAX_DOCUMENT: usize = 2 * 1024 * 1024;
/// Largest token answer read (design: the example is under 1 KiB).
pub const MAX_TOKEN_ANSWER: usize = 16 * 1024;
/// Longest access token kept (design).
pub const MAX_TOKEN: usize = 8 * 1024;
/// Most contacts taken from one document (design).
pub const MAX_CONTACTS: usize = 5000;
/// The longest search query (UW-08: 256 characters).
pub const MAX_QUERY: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The answer is not what the reference describes (the step).
    BadAnswer(&'static str),
    Xml(commscraft_xml::Error),
}

/// The `MsRtcOAuth` challenge of a 401 (UW-01): where to ask for a token and which grants the
/// service takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub token_url: HttpsUrl,
    pub grants: Vec<String>,
}

impl Challenge {
    pub fn offers_windows(&self) -> bool {
        self.grants.iter().any(|g| g == WINDOWS_GRANT)
    }
}

/// Reads the `MsRtcOAuth` challenge from the values of `WWW-Authenticate` headers (UW-01):
/// `MsRtcOAuth href="https://…/WebTicket/oauthtoken",grant_type="urn:microsoft.rtc:windows,…"`.
pub fn oauth_challenge<'a>(values: impl IntoIterator<Item = &'a str>) -> Option<Challenge> {
    for v in values {
        let v = v.trim();
        let Some(rest) = v
            .get(..10)
            .filter(|p| p.eq_ignore_ascii_case("MsRtcOAuth"))
            .and_then(|_| v.get(10..))
        else {
            continue;
        };
        let href = param(rest, "href")?;
        let token_url = parse_https(&href)?;
        let grants = param(rest, "grant_type")
            .map(|g| {
                g.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .take(16)
                    .collect()
            })
            .unwrap_or_default();
        return Some(Challenge { token_url, grants });
    }
    None
}

/// One `name=value` or `name="value"` parameter of a challenge.
fn param(s: &str, name: &str) -> Option<String> {
    let mut rest = s;
    while let Some(i) = rest.find(name) {
        let before_ok = i == 0
            || rest[..i]
                .chars()
                .next_back()
                .is_some_and(|c| c == ',' || c.is_whitespace());
        let after = &rest[i + name.len()..];
        let after_trim = after.trim_start();
        if before_ok && after_trim.starts_with('=') {
            let value = after_trim[1..].trim_start();
            return Some(match value.strip_prefix('"') {
                Some(q) => q.split('"').next()?.to_string(),
                None => value.split(',').next().unwrap_or("").trim().to_string(),
            })
            .filter(|v| v.len() <= 2048 && !v.chars().any(char::is_control));
        }
        rest = after;
    }
    None
}

/// The body of the token request with the Windows grant (UW-02).
pub fn windows_token_request() -> String {
    format!("grant_type={WINDOWS_GRANT}")
}

/// An OAuth access token (UW-02): `access_token` as given (`cwt=…`), its lifetime in seconds.
/// A credential: wiped from memory when dropped, not `Clone` (issue #258).
#[derive(PartialEq, Eq)]
pub struct Token {
    pub access_token: zeroize::Zeroizing<String>,
    pub expires_in: u64,
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The token is a credential: never printed.
        write!(f, "Token({} s)", self.expires_in)
    }
}

impl Token {
    /// The value of the `Authorization` header (UW-03); wiped when dropped.
    pub fn authorization(&self) -> zeroize::Zeroizing<String> {
        let mut h = zeroize::Zeroizing::new(String::with_capacity(7 + self.access_token.len()));
        h.push_str("Bearer ");
        h.push_str(&self.access_token);
        h
    }
}

/// Reads the token answer (UW-02): a flat JSON object with `access_token`, `token_type`
/// ("Bearer") and `expires_in`. Bounded, strict; anything else is refused.
pub fn token(body: &[u8]) -> Result<Token, Error> {
    if body.len() > MAX_TOKEN_ANSWER {
        return Err(Error::BadAnswer("token answer too large"));
    }
    let text = std::str::from_utf8(body).map_err(|_| Error::BadAnswer("token answer not UTF-8"))?;
    let mut fields =
        flat_json(text).ok_or(Error::BadAnswer("token answer not a flat JSON object"))?;
    let get = |k: &str| fields.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    if !get("token_type").is_some_and(|t| t.eq_ignore_ascii_case("bearer")) {
        return Err(Error::BadAnswer("token type"));
    }
    let access_token = get("access_token")
        .filter(|t| {
            !t.is_empty()
                && t.len() <= MAX_TOKEN
                && t.bytes()
                    .all(|b| b.is_ascii_graphic() && b != b'"' && b != b'\\')
        })
        .map(|t| zeroize::Zeroizing::new(t.to_string()));
    let expires_in = get("expires_in")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(3600)
        .min(86_400);
    // The parsed copies of the answer's values (the token among them) are wiped too.
    for (_, v) in fields.iter_mut() {
        zeroize::Zeroize::zeroize(v);
    }
    Ok(Token {
        access_token: access_token.ok_or(Error::BadAnswer("access token"))?,
        expires_in,
    })
}

/// A flat JSON object of strings, numbers, booleans and nulls (no nesting): its members in
/// order, values as text. `None` for anything else.
fn flat_json(s: &str) -> Option<Vec<(String, String)>> {
    let mut chars = s.trim().chars().peekable();
    let ws = |c: &mut std::iter::Peekable<std::str::Chars<'_>>| {
        while c.peek().is_some_and(|x| x.is_whitespace()) {
            c.next();
        }
    };
    let string = |c: &mut std::iter::Peekable<std::str::Chars<'_>>| -> Option<String> {
        if c.next()? != '"' {
            return None;
        }
        let mut out = String::new();
        loop {
            match c.next()? {
                '"' => return Some(out),
                '\\' => match c.next()? {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'b' | 'f' => {}
                    'u' => {
                        let hex: String = (0..4).filter_map(|_| c.next()).collect();
                        let n = u32::from_str_radix(&hex, 16).ok()?;
                        out.push(char::from_u32(n).unwrap_or('\u{fffd}'));
                    }
                    _ => return None,
                },
                ch if (ch as u32) < 0x20 => return None,
                ch => out.push(ch),
            }
            if out.len() > MAX_TOKEN_ANSWER {
                return None;
            }
        }
    };
    if chars.next()? != '{' {
        return None;
    }
    let mut out = Vec::new();
    ws(&mut chars);
    if chars.peek() == Some(&'}') {
        chars.next();
        ws(&mut chars);
        return chars.next().is_none().then_some(out);
    }
    loop {
        ws(&mut chars);
        let name = string(&mut chars)?;
        ws(&mut chars);
        if chars.next()? != ':' {
            return None;
        }
        ws(&mut chars);
        let value = if chars.peek() == Some(&'"') {
            string(&mut chars)?
        } else {
            let mut v = String::new();
            while let Some(&c) = chars.peek() {
                if c == ',' || c == '}' || c.is_whitespace() {
                    break;
                }
                if !(c.is_ascii_alphanumeric() || "+-.".contains(c)) || v.len() > 32 {
                    return None;
                }
                v.push(c);
                chars.next();
            }
            if v.is_empty() {
                return None;
            }
            v
        };
        out.push((name, value));
        if out.len() > 32 {
            return None;
        }
        ws(&mut chars);
        match chars.next()? {
            ',' => continue,
            '}' => {
                ws(&mut chars);
                return chars.next().is_none().then_some(out);
            }
            _ => return None,
        }
    }
}

fn parse(body: &[u8], max: usize) -> Result<Element, Error> {
    if body.len() > max {
        return Err(Error::BadAnswer("document too large"));
    }
    let text = std::str::from_utf8(body).map_err(|_| Error::BadAnswer("document not UTF-8"))?;
    // A byte order mark before the declaration (seen from IIS) is not XML.
    let text = text.trim_start_matches('\u{feff}');
    let limits = Limits {
        max_input: max,
        max_depth: 16,
        max_attributes: 16,
        max_name_len: 128,
        max_nodes: 400_000,
    };
    commscraft_xml::parse(text, &limits).map_err(Error::Xml)
}

/// The UCWA application links of the User resource (UW-04): `Internal/Ucwa`, `External/Ucwa`
/// (or `Ucwa`), in this order, as absolute HTTPS URLs.
pub fn ucwa_links(body: &[u8]) -> Result<Vec<HttpsUrl>, Error> {
    let root = parse(body, 64 * 1024)?;
    if root.name != "AutodiscoverResponse" {
        return Err(Error::BadAnswer("not an AutodiscoverResponse"));
    }
    let mut links: Vec<(u8, HttpsUrl)> = Vec::new();
    for kind in root.elements() {
        for link in kind.children_named("Link") {
            let order = match link.attr("token") {
                Some(t) if t.eq_ignore_ascii_case("Internal/Ucwa") => 0,
                Some(t) if t.eq_ignore_ascii_case("External/Ucwa") => 1,
                Some(t) if t.eq_ignore_ascii_case("Ucwa") => 2,
                _ => continue,
            };
            if let Some(url) = link.attr("href").and_then(parse_https) {
                links.push((order, url));
            }
        }
    }
    links.sort_by_key(|(o, _)| *o);
    let mut out: Vec<HttpsUrl> = Vec::new();
    for (_, u) in links {
        if !out.contains(&u) {
            out.push(u);
        }
    }
    Ok(out)
}

/// The request that creates the application (UW-05): culture, endpoint ID and user agent, each at
/// most 100 characters; refused otherwise.
pub fn application_request(culture: &str, endpoint_id: &str, user_agent: &str) -> Option<String> {
    let ok =
        |v: &str| !v.is_empty() && v.chars().count() <= 100 && !v.chars().any(char::is_control);
    if !(ok(culture) && ok(endpoint_id) && ok(user_agent)) {
        return None;
    }
    let e = commscraft_xml::escape;
    Some(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><input xmlns=\"{NS}\"><property name=\"culture\">{}</property><property name=\"endpointId\">{}</property><property name=\"userAgent\">{}</property></input>",
        e(culture),
        e(endpoint_id),
        e(user_agent)
    ))
}

/// A UCWA resource (UW-06): its relation and address, its links, properties and embedded
/// resources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resource {
    pub rel: String,
    pub href: String,
    pub links: Vec<(String, String)>,
    pub properties: Vec<(String, String)>,
    pub resources: Vec<Resource>,
}

impl Resource {
    pub fn link(&self, rel: &str) -> Option<&str> {
        self.links
            .iter()
            .find(|(r, _)| r == rel)
            .map(|(_, h)| h.as_str())
    }

    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// The first embedded resource (at any depth) with this relation.
    pub fn find(&self, rel: &str) -> Option<&Resource> {
        self.resources
            .iter()
            .find_map(|r| if r.rel == rel { Some(r) } else { r.find(rel) })
    }

    /// Every embedded resource (at any depth) with this relation, at most `max`.
    pub fn all(&self, rel: &str, max: usize) -> Vec<&Resource> {
        let mut out = Vec::new();
        let mut stack: Vec<&Resource> = self.resources.iter().rev().collect();
        while let Some(r) = stack.pop() {
            if out.len() >= max {
                break;
            }
            if r.rel == rel {
                out.push(r);
            }
            stack.extend(r.resources.iter().rev());
        }
        out
    }
}

/// Reads a UCWA XML document (UW-06): a `resource` in the UCWA namespace.
pub fn resource(body: &[u8]) -> Result<Resource, Error> {
    let root = parse(body, MAX_DOCUMENT)?;
    if root.name != "resource" || root.namespace.as_deref() != Some(NS) {
        return Err(Error::BadAnswer("not a UCWA resource"));
    }
    Ok(read_resource(&root))
}

fn read_resource(e: &Element) -> Resource {
    let mut r = Resource {
        rel: e.attr("rel").unwrap_or("").to_string(),
        href: e.attr("href").unwrap_or("").to_string(),
        ..Resource::default()
    };
    for c in e.elements() {
        match c.name.as_str() {
            "link" => {
                if let (Some(rel), Some(href)) = (c.attr("rel"), c.attr("href")) {
                    r.links.push((rel.to_string(), href.to_string()));
                }
            }
            "property" => {
                if let Some(name) = c.attr("name") {
                    r.properties.push((name.to_string(), c.text()));
                }
            }
            "resource" => r.resources.push(read_resource(c)),
            _ => {}
        }
    }
    r
}

/// A contact with a photo link (UW-07): its SIP URI (lower case) and the photo's address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoLink {
    pub uri: String,
    pub href: String,
}

/// The contacts of a document (contact list, search result, expanded group) that carry a
/// `contactPhoto` link (UW-07 … UW-09).
pub fn photo_links(doc: &Resource) -> Vec<PhotoLink> {
    let mut list: Vec<&Resource> = doc.all("contact", MAX_CONTACTS);
    if doc.rel == "contact" {
        list.insert(0, doc);
    }
    list.into_iter()
        .filter_map(|c| {
            let uri = c.property("uri")?.trim().to_ascii_lowercase();
            let href = c.link("contactPhoto")?.to_string();
            uri.starts_with("sip:").then_some(PhotoLink { uri, href })
        })
        .collect()
}

/// The `expandDistributionGroup` link of the group with this address (mail or SIP) in a search
/// result (UW-09).
pub fn group_expand_link(doc: &Resource, address: &str) -> Option<String> {
    let want = address
        .trim()
        .trim_start_matches("sip:")
        .to_ascii_lowercase();
    doc.all("distributionGroup", 64)
        .into_iter()
        .chain((doc.rel == "distributionGroup").then_some(doc))
        .find(|g| {
            g.property("uri").is_some_and(|u| {
                u.trim()
                    .trim_start_matches("sip:")
                    .eq_ignore_ascii_case(&want)
            }) || g
                .property("name")
                .is_some_and(|n| n.trim().eq_ignore_ascii_case(&want))
        })
        .and_then(|g| g.link("expandDistributionGroup"))
        .map(str::to_string)
}

/// The search request's path and query for an address (UW-08), `limit` 1 … 100.
pub fn search_query(search_href: &str, query: &str, limit: u32) -> Option<String> {
    if query.is_empty() || query.chars().count() > MAX_QUERY {
        return None;
    }
    let sep = if search_href.contains('?') { '&' } else { '?' };
    Some(format!(
        "{search_href}{sep}query={}&limit={}",
        percent(query),
        limit.clamp(1, 100)
    ))
}

/// The photo request for a size `HR{n}X{n}` (UW-07: default HR48X48).
pub fn photo_query(photo_href: &str, side: u32) -> String {
    let side = side.clamp(32, 648);
    let sep = if photo_href.contains('?') { '&' } else { '?' };
    format!("{photo_href}{sep}size=HR{side}X{side}")
}

fn percent(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~@".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// An address from a UCWA document resolved against the service it came from (UW-10): a path
/// (`/ucwa/…`) or an absolute HTTPS URL on the **same host and port**. Anything else – another
/// host, plain HTTP, control characters – is refused, so the token goes nowhere else.
pub fn resolve(service: &HttpsUrl, href: &str) -> Option<HttpsUrl> {
    if href.chars().any(char::is_control) || href.contains(' ') || href.len() > 4096 {
        return None;
    }
    let port = if service.port == 443 {
        String::new()
    } else {
        format!(":{}", service.port)
    };
    let full = if href.starts_with('/') && !href.starts_with("//") {
        format!("https://{}{port}{href}", service.host)
    } else {
        href.to_string()
    };
    let url = parse_https(&full)?;
    (url.host.eq_ignore_ascii_case(&service.host) && url.port == service.port).then_some(url)
}

#[cfg(test)]
mod tests;
