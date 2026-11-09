//! Exchange web services the client uses (evidence file 37): Autodiscover `GetUserSettings`
//! ([MS-OXWSADISC] 20250520, EX-08) for the user's EWS endpoint, and user photos
//! `GetUserPhoto` ([MS-OXWSPHOTO] 20250520, EX-06), and the contact list in Exchange
//! ([MS-OXWSCOS] 20250520, EX-01 … EX-05, module [`ucs`]).
//!
//! Security by design: the mailbox and setting names are checked and XML-escaped; the response
//! is parsed with the bounded XML reader (no DTD); only HTTPS endpoint URLs are taken over,
//! values are stripped of control characters and bounded in length and count; a redirect is
//! only reported – the caller decides whether to follow it (only to HTTPS, at most a few times).

use commscraft_xml::{escape, Element, Limits};

pub mod ucs;

pub const CONTENT_TYPE: &str = "text/xml; charset=utf-8";
/// `soapAction` of GetUserSettings ([MS-OXWSADISC] §3.1.4.4, p. 49).
pub const GET_USER_SETTINGS_ACTION: &str =
    "\"http://schemas.microsoft.com/exchange/2010/Autodiscover/Autodiscover/GetUserSettings\"";
const NS_SOAP: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const NS_ADISC: &str = "http://schemas.microsoft.com/exchange/2010/Autodiscover";
/// Longest mailbox address accepted (design; an SMTP path is at most 256 octets, RFC 5321).
pub const MAX_MAILBOX: usize = 256;
/// Most settings kept from an answer, and the longest value (design).
pub const MAX_SETTINGS: usize = 64;
pub const MAX_VALUE: usize = 2048;

/// The settings the client asks for: the EWS endpoint (internal first) and the photo endpoint
/// named by [MS-OXWSPHOTO] §3.1.5.1.1 (p. 9).
pub const SETTINGS: [&str; 4] = [
    "InternalEwsUrl",
    "ExternalEwsUrl",
    "InternalPhotosUrl",
    "ExternalPhotosUrl",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadMailbox,
    Xml(commscraft_xml::Error),
    /// Not a SOAP envelope with a GetUserSettings answer.
    WrongDocument(&'static str),
    /// SOAP fault from the server (fault string, printable).
    Fault(String),
    /// An ErrorCode other than NoError and the redirects (§3.1.4.4.4.2, p. 75).
    Server(String),
}

/// The Autodiscover endpoints of a domain, in the order to try ([MS-OXWSADISC] §1.5,
/// pp. 11–12: U1, U2).
pub fn autodiscover_urls(domain: &str) -> Option<[String; 2]> {
    let d = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    let ok = !d.is_empty()
        && d.len() <= 253
        && d.contains('.')
        && d.split('.').all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    ok.then(|| {
        [
            format!("https://autodiscover.{d}/autodiscover/autodiscover.svc"),
            format!("https://{d}/autodiscover/autodiscover.svc"),
        ]
    })
}

/// A mailbox address as the request carries it: `local@domain`, printable, bounded.
fn mailbox(m: &str) -> Result<&str, Error> {
    let m = m.trim();
    let (local, domain) = m.split_once('@').ok_or(Error::BadMailbox)?;
    let ok = m.len() <= MAX_MAILBOX
        && !local.is_empty()
        && !domain.is_empty()
        && !domain.contains('@')
        && m.chars()
            .all(|c| c.is_ascii_graphic() && !"<>\"&'".contains(c));
    ok.then_some(m).ok_or(Error::BadMailbox)
}

/// The GetUserSettings request ([MS-OXWSADISC] §3.1.4.4, p. 49; example §4.3, pp. 80–81) for
/// `user` with the settings `settings`, addressed to `to` (the endpoint URL, WS-Addressing To).
pub fn user_settings_request(user: &str, settings: &[&str], to: &str) -> Result<String, Error> {
    let user = mailbox(user)?;
    let mut s = String::new();
    for name in settings {
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(Error::WrongDocument("setting name"));
        }
        s.push_str(&format!("<a:Setting>{name}</a:Setting>"));
    }
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<soap:Envelope xmlns:a=\"{NS_ADISC}\" xmlns:wsa=\"http://www.w3.org/2005/08/addressing\" \
xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:soap=\"{NS_SOAP}\">\
<soap:Header><a:RequestedServerVersion>Exchange2013</a:RequestedServerVersion>\
<wsa:Action>http://schemas.microsoft.com/exchange/2010/Autodiscover/Autodiscover/GetUserSettings</wsa:Action>\
<wsa:To>{}</wsa:To></soap:Header>\
<soap:Body><a:GetUserSettingsRequestMessage><a:Request><a:Users><a:User><a:Mailbox>{}</a:Mailbox>\
</a:User></a:Users><a:RequestedSettings>{s}</a:RequestedSettings></a:Request>\
</a:GetUserSettingsRequestMessage></soap:Body></soap:Envelope>",
        escape(to),
        escape(user)
    ))
}

/// What GetUserSettings answered for the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserSettings {
    /// The settings that came back (name, value), in order.
    Settings(Vec<(String, String)>),
    /// ErrorCode RedirectAddress: ask again for this mailbox.
    RedirectAddress(String),
    /// ErrorCode RedirectUrl: ask this endpoint (HTTPS only).
    RedirectUrl(String),
}

impl UserSettings {
    /// A returned setting by name.
    pub fn get(&self, name: &str) -> Option<&str> {
        match self {
            UserSettings::Settings(s) => s.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str()),
            _ => None,
        }
    }

    /// The first of `names` that came back as an HTTPS URL.
    pub fn https_url(&self, names: &[&str]) -> Option<&str> {
        names
            .iter()
            .filter_map(|n| self.get(n))
            .find(|v| v.starts_with("https://"))
    }
}

fn printable(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(MAX_VALUE)
        .collect::<String>()
        .trim()
        .to_string()
}

fn error_code(e: &Element) -> String {
    e.child("ErrorCode")
        .map(|c| c.text().trim().to_string())
        .unwrap_or_default()
}

/// Reads the GetUserSettings answer (example §4.3, pp. 81–82): the first user response decides.
pub fn parse_user_settings(xml: &str) -> Result<UserSettings, Error> {
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
    let response = body
        .child("GetUserSettingsResponseMessage")
        .filter(|m| m.namespace.as_deref() == Some(NS_ADISC))
        .and_then(|m| m.child("Response"))
        .ok_or(Error::WrongDocument("GetUserSettingsResponseMessage"))?;
    match error_code(response).as_str() {
        "NoError" | "" => {}
        other => return Err(Error::Server(printable(other))),
    }
    let user = response
        .child("UserResponses")
        .and_then(|u| u.child("UserResponse"))
        .ok_or(Error::WrongDocument("UserResponse"))?;
    let target = || {
        user.child("RedirectTarget")
            .map(|t| printable(&t.text()))
            .filter(|t| !t.is_empty())
    };
    match error_code(user).as_str() {
        "NoError" | "" => {}
        "RedirectAddress" => {
            let t = target().ok_or(Error::WrongDocument("RedirectTarget"))?;
            return Ok(UserSettings::RedirectAddress(mailbox(&t)?.to_string()));
        }
        "RedirectUrl" => {
            return target()
                .filter(|t| t.starts_with("https://"))
                .map(UserSettings::RedirectUrl)
                .ok_or(Error::WrongDocument("RedirectTarget"));
        }
        other => return Err(Error::Server(printable(other))),
    }
    let settings = user
        .child("UserSettings")
        .map(|s| {
            s.children_named("UserSetting")
                .filter_map(|u| {
                    let name = printable(&u.child("Name")?.text());
                    let value = printable(&u.child("Value")?.text());
                    (!name.is_empty()).then_some((name, value))
                })
                .take(MAX_SETTINGS)
                .collect()
        })
        .unwrap_or_default();
    Ok(UserSettings::Settings(settings))
}

/// The image sizes of GetUserPhoto ([MS-OXWSPHOTO] §3.1.5.1.1, p. 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoSize {
    Hr48,
    Hr64,
    Hr96,
    Hr120,
    Hr240,
    Hr360,
    Hr432,
    Hr504,
    Hr648,
}

impl PhotoSize {
    pub fn code(self) -> &'static str {
        match self {
            PhotoSize::Hr48 => "HR48x48",
            PhotoSize::Hr64 => "HR64x64",
            PhotoSize::Hr96 => "HR96x96",
            PhotoSize::Hr120 => "HR120x120",
            PhotoSize::Hr240 => "HR240x240",
            PhotoSize::Hr360 => "HR360x360",
            PhotoSize::Hr432 => "HR432x432",
            PhotoSize::Hr504 => "HR504x504",
            PhotoSize::Hr648 => "HR648x648",
        }
    }
}

/// Percent-encodes a query value (RFC 3986 unreserved characters stay).
fn query_value(v: &str) -> String {
    v.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The GetUserPhoto URL ([MS-OXWSPHOTO] §3.1.5.1.1, p. 9; example §4, p. 11): `base` is the
/// photos URL or the EWS URL (the `Exchange.asmx` handler). K-EX-1: the document leaves open
/// whether the photos URL already ends in the `/s` path segment; Exchange returns it with
/// `/s` (`tested-in-lab`), the EWS URL comes without it – so `/s` is added only when missing.
pub fn photo_url(base: &str, email: &str, size: PhotoSize) -> Result<String, Error> {
    let email = mailbox(email)?;
    let base = base.trim().trim_end_matches('/');
    if !base.starts_with("https://") || base.contains(['?', '#']) {
        return Err(Error::WrongDocument("photo base URL"));
    }
    let s = if base.to_ascii_lowercase().ends_with("/s") {
        ""
    } else {
        "/s"
    };
    Ok(format!(
        "{base}{s}/GetUserPhoto?email={}&size={}",
        query_value(email),
        size.code()
    ))
}

/// What a GetUserPhoto answer means (§3.1.5, pp. 8–10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhotoAnswer {
    /// 200: the image (its Content-Type and ETag, if sent).
    Image {
        content_type: Option<String>,
        etag: Option<String>,
    },
    /// 304: unchanged since the ETag sent in If-None-Match.
    Unchanged,
    /// 404: no image for this mailbox.
    NoPhoto,
    /// 401: not authenticated.
    Unauthorized,
    /// Anything else (400 and others).
    Failed(u16),
}

/// An ETag as it may be sent back in If-None-Match: a quoted string without control
/// characters, bounded (RFC 7232 §2.3; design bound).
fn etag(v: &str) -> Option<String> {
    let v = v.trim();
    let inner = v.strip_prefix("W/").unwrap_or(v);
    (v.len() <= 256
        && inner.len() >= 2
        && inner.starts_with('"')
        && inner.ends_with('"')
        && v.chars().all(|c| c.is_ascii_graphic()))
    .then(|| v.to_string())
}

/// Reads status, Content-Type and ETag of a GetUserPhoto answer.
pub fn photo_answer(
    status: u16,
    content_type: Option<&str>,
    etag_value: Option<&str>,
) -> PhotoAnswer {
    match status {
        200 => PhotoAnswer::Image {
            content_type: content_type.map(printable).filter(|c| !c.is_empty()),
            etag: etag_value.and_then(etag),
        },
        304 => PhotoAnswer::Unchanged,
        404 => PhotoAnswer::NoPhoto,
        401 => PhotoAnswer::Unauthorized,
        s => PhotoAnswer::Failed(s),
    }
}

#[cfg(test)]
mod tests;
