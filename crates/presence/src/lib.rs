//! Enhanced presence and the contact list (M2): publishing the own state, polling the presence of
//! contacts in one batch, reading the roaming contact list.
//!
//! Evidence: `docs/protocol/evidence/07-pres-presence-contacts.md` (PR-01 … PR-36, PR-50,
//! K-PR-1) and `05-sip-base-extensions.md` (SIP-01, SIP-02, SIP-04, SIP-07).
//! Sources: [MS-PRES] 20250520 §2.2.2 (pp. 33–77), §3.4.5.1.2 (pp. 99–100), §3.8.5.1 (pp. 116–125),
//! examples §4.2 (pp. 135–138), §4.5 (pp. 175–180), §4.7 (pp. 183–184), §4.14 (pp. 197–200);
//! [MS-SIP] 20170601 §2.2.4.1 (pp. 17–18), §3.3 (pp. 30–35), example p. 49.
//!
//! Security by design (`docs/security.md`):
//! - every document is parsed with the bounded `commscraft-xml` reader; the multipart parser is
//!   bounded in parts and size and needs a well-formed boundary;
//! - URIs from the network are checked before they go into our own XML or headers, and all text
//!   we write into XML is escaped;
//! - the own state is published with `expireType="endpoint"` by default, so it disappears when this
//!   endpoint signs out and nothing on the account stays changed by accident;
//! - names, notes and activities are only data: [`printable`] removes control characters before
//!   anything reaches a terminal.

use commscraft_sip_msg::Message;
use commscraft_xml::{escape, Element, Limits};

pub const NS_PUBLISH: &str = "http://schemas.microsoft.com/2006/09/sip/rich-presence";
pub const NS_STATE: &str = "http://schemas.microsoft.com/2006/09/sip/state";
pub const NS_CATEGORIES: &str = "http://schemas.microsoft.com/2006/09/sip/categories";
pub const NS_BATCH: &str = "http://schemas.microsoft.com/2006/01/sip/batch-subscribe";
pub const NS_CATEGORY_LIST: &str = "http://schemas.microsoft.com/2006/09/sip/categorylist";
pub const NS_RLMI: &str = "urn:ietf:params:xml:ns:rlmi";
/// Contact list schema namespace ([MS-SIP] §9.1); the examples use no namespace.
pub const NS_CONTACTS: &str = "http://schemas.microsoft.com/sip/types";

/// Instance ID of the manual user state in the examples (§4.3.1.1, p. 143: 603979776 =
/// 0x24000000). Inference: the specification fixes no value; the example's one is used.
pub const USER_STATE_INSTANCE: u32 = 0x2400_0000;
/// Largest number of resources per batch (SIP-01: servers allow typically 100–250).
pub const MAX_BATCH: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Xml(commscraft_xml::Error),
    /// The document is not what was expected (root, namespace, attribute).
    WrongDocument(&'static str),
    /// A multipart body that cannot be split safely.
    BadMultipart,
    /// A URI that is not safe to put into a request.
    UnsafeUri,
    TooManyResources,
}

impl From<commscraft_xml::Error> for Error {
    fn from(e: commscraft_xml::Error) -> Self {
        Error::Xml(e)
    }
}

/// Availability classes (PR-30; MS-PRES §2.2.2.7.1, p. 51).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Unknown,
    Online,
    Idle,
    Busy,
    BusyIdle,
    DoNotDisturb,
    Away,
    Offline,
}

impl Availability {
    pub fn from_number(n: u32) -> Self {
        match n {
            0..=2999 => Self::Unknown,
            3000..=4499 => Self::Online,
            4500..=5999 => Self::Idle,
            6000..=7499 => Self::Busy,
            7500..=8999 => Self::BusyIdle,
            9000..=11999 => Self::DoNotDisturb,
            12000..=17999 => Self::Away,
            _ => Self::Offline,
        }
    }
}

/// A state the user sets manually. Numbers as in the specification examples (3500 online,
/// 6500 busy, 9500 do not disturb, 15500 away; §2.2.2.7.1, §4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    Online,
    Busy,
    DoNotDisturb,
    Away,
}

impl UserStatus {
    pub fn availability(self) -> u32 {
        match self {
            Self::Online => 3500,
            Self::Busy => 6500,
            Self::DoNotDisturb => 9500,
            Self::Away => 15500,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "online" | "available" => Some(Self::Online),
            "busy" => Some(Self::Busy),
            "dnd" | "do-not-disturb" => Some(Self::DoNotDisturb),
            "away" => Some(Self::Away),
            _ => None,
        }
    }
}

/// Removes control characters (terminal escape sequences, bidirectional overrides) from text that
/// came from the network, and bounds its length.
pub fn printable(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !c.is_control()
                && !('\u{202a}'..='\u{202e}').contains(c)
                && !('\u{2066}'..='\u{2069}').contains(c)
        })
        .take(256)
        .collect()
}

/// A SIP URI that is safe inside our XML and headers: `sip:user@host`, no whitespace, quotes,
/// angle brackets or commas. A bare `user@host` (contact list examples, §4.14) gets `sip:`.
pub fn checked_sip_uri(raw: &str) -> Result<String, Error> {
    let t = raw.trim();
    let uri = if t.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("sip:")) {
        t.to_string()
    } else if t.contains(':') {
        return Err(Error::UnsafeUri);
    } else {
        format!("sip:{t}")
    };
    let rest = uri.get(4..).unwrap_or("");
    let ok = uri.len() <= 512
        && rest.contains('@')
        && !rest.starts_with('@')
        && !rest.ends_with('@')
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@._-+=;:%~*".contains(c));
    ok.then_some(uri).ok_or(Error::UnsafeUri)
}

/// Body of a SERVICE that publishes the manual user state into containers 2 and 3 (PR-11, PR-31;
/// §2.2.2.2.1, §3.8.5.1.1). `versions` are the current versions in container 2 and 3 (0 for a new
/// instance, PR-05). With `endpoint_id` the publication expires with this endpoint (PR-02).
pub fn publish_state_body(
    aor: &str,
    status: UserStatus,
    versions: [u32; 2],
    endpoint_id: Option<&str>,
) -> Result<String, Error> {
    let aor = checked_sip_uri(aor)?;
    let expire = match endpoint_id {
        Some(id) => {
            if !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
                return Err(Error::UnsafeUri);
            }
            format!("expireType=\"endpoint\" endpointId=\"{id}\"")
        }
        None => "expireType=\"static\"".to_string(),
    };
    let mut b = format!(
        "<publish xmlns=\"{NS_PUBLISH}\"><publications uri=\"{}\">",
        escape(&aor)
    );
    for (container, version) in [2u32, 3].iter().zip(versions) {
        b.push_str(&format!(
            "<publication categoryName=\"state\" instance=\"{USER_STATE_INSTANCE}\" container=\"{container}\" version=\"{version}\" {expire}>\
             <state xmlns=\"{NS_STATE}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" manual=\"true\" xsi:type=\"userState\">\
             <availability>{}</availability></state></publication>",
            status.availability()
        ));
    }
    b.push_str("</publications></publish>");
    Ok(b)
}

/// Instance IDs of this endpoint's device and machineState publications. The specification fixes
/// no values (the examples show endpoint-specific 32-bit numbers, §4.3.2.1, pp. 151–152); they
/// are derived from the endpoint ID so they stay the same across sign-ins (inference).
pub fn endpoint_instances(endpoint_id: &str) -> (u32, u32) {
    // FNV-1a over the normalised endpoint ID; only stability matters, not secrecy.
    let mut h: u32 = 0x811c_9dc5;
    for b in endpoint_id
        .to_ascii_lowercase()
        .bytes()
        .filter(|b| *b != b'-')
    {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    // Keep the two top bits clear (MS-PRES uses them as flags in some instance IDs).
    let device = h & 0x3fff_ffff;
    (device, device ^ 0x0000_5a5a)
}

/// Body of the SERVICE that publishes this endpoint's `device` category (capabilities: text) and
/// its `machineState` into container 2 (PR-31, PR-33; §3.8.5.2.1, p. 126; §2.2.2.7.2, pp. 53–54),
/// both with `expireType="endpoint"`. `availability` is 3500 for an active endpoint (§4.3).
pub fn publish_endpoint_body(
    aor: &str,
    endpoint_id: &str,
    machine_name: &str,
    availability: u32,
    versions: [u32; 2],
) -> Result<String, Error> {
    let aor = checked_sip_uri(aor)?;
    if endpoint_id.is_empty()
        || !endpoint_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        return Err(Error::UnsafeUri);
    }
    let (device_instance, state_instance) = endpoint_instances(endpoint_id);
    let machine = escape(&printable(machine_name));
    let a = escape(&aor);
    Ok(format!(
        "<publish xmlns=\"{NS_PUBLISH}\"><publications uri=\"{a}\">\
<publication categoryName=\"device\" instance=\"{device_instance}\" container=\"2\" version=\"{}\" expireType=\"endpoint\" endpointId=\"{endpoint_id}\">\
<device xmlns=\"http://schemas.microsoft.com/2006/09/sip/device\" endpointId=\"{endpoint_id}\">\
<capabilities preferred=\"false\" uri=\"{a}\"><text capture=\"true\" render=\"true\" publish=\"false\"/></capabilities>\
<machineName>{machine}</machineName></device></publication>\
<publication categoryName=\"state\" instance=\"{state_instance}\" container=\"2\" version=\"{}\" expireType=\"endpoint\" endpointId=\"{endpoint_id}\">\
<state xmlns=\"{NS_STATE}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" manual=\"false\" xsi:type=\"machineState\">\
<availability>{availability}</availability><endpointLocation/></state></publication>\
</publications></publish>",
        versions[0], versions[1]
    ))
}

/// Kind of a roaming contacts document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactDocument {
    /// `contactList`: the complete list.
    Full,
    /// `contactDelta` ([MS-SIP] §3.7.5.2): a change; the client fetches the full list again.
    Delta,
    Other,
}

pub fn contact_document_kind(xml: &str) -> ContactDocument {
    match commscraft_xml::parse(xml, &Limits::default()) {
        Ok(r) if r.name == "contactList" => ContactDocument::Full,
        Ok(r) if r.name == "contactDelta" => ContactDocument::Delta,
        _ => ContactDocument::Other,
    }
}

/// One failed operation of a 409 `application/msrtc-fault+xml` (PR-05; example §4.2.1, p. 138).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionConflict {
    pub index: u32,
    pub version: u32,
    pub cur_version: u32,
}

/// Parses the fault of a 409. K-PR-1: the example's fault code reads `Protocol client.BadCall.
/// WrongDelta`, the text says `Client.BadCall.WrongDelta`; both are accepted.
pub fn parse_fault(body: &str) -> Result<Vec<VersionConflict>, Error> {
    let root = commscraft_xml::parse(body, &Limits::default())?;
    if root.name != "Fault" {
        return Err(Error::WrongDocument("Fault"));
    }
    let code = root
        .child("Faultcode")
        .map(|c| c.text())
        .unwrap_or_default();
    if !code.trim().ends_with("BadCall.WrongDelta") {
        return Err(Error::WrongDocument("Faultcode"));
    }
    let num = |e: &Element, a: &str| e.attr(a).and_then(|v| v.parse::<u32>().ok());
    let mut out = Vec::new();
    if let Some(details) = root.child("details") {
        for op in details.children_named("operation") {
            if let (Some(index), Some(version), Some(cur_version)) =
                (num(op, "index"), num(op, "version"), num(op, "curVersion"))
            {
                out.push(VersionConflict {
                    index,
                    version,
                    cur_version,
                });
            }
        }
    }
    Ok(out)
}

/// Categories asked for in the batch subscription (§4.5, p. 175).
pub const CATEGORIES: &[&str] = &["contactCard", "note", "services", "state"];

/// Body of a batch category SUBSCRIBE (PR-14; §2.2.2.4.1; example §4.5).
pub fn batch_body(aor: &str, resources: &[String], action_id: u32) -> Result<String, Error> {
    if resources.len() > MAX_BATCH {
        return Err(Error::TooManyResources);
    }
    let aor = checked_sip_uri(aor)?;
    let mut b = format!(
        "<batchSub xmlns=\"{NS_BATCH}\" uri=\"{}\" name=\"\"><action name=\"subscribe\" id=\"{action_id}\"><adhocList>",
        escape(&aor)
    );
    for r in resources {
        b.push_str(&format!(
            "<resource uri=\"{}\"/>",
            escape(&checked_sip_uri(r)?)
        ));
    }
    b.push_str(&format!(
        "</adhocList><categoryList xmlns=\"{NS_CATEGORY_LIST}\">"
    ));
    for c in CATEGORIES {
        b.push_str(&format!("<category name=\"{c}\"/>"));
    }
    b.push_str("</categoryList></action></batchSub>");
    Ok(b)
}

/// Adds the headers of a batch SUBSCRIBE. `polling` = `Expires: 0`: a one-time snapshot without a
/// dialog (PR-17; §3.4.5.1.2; example §4.7). BENOTIFY is not offered (not supported yet).
pub fn batch_headers(m: &mut Message, polling: bool) {
    m.push_header("Event", "presence");
    m.push_header(
        "Accept",
        "application/msrtc-event-categories+xml, application/rlmi+xml, multipart/related",
    );
    m.push_header("Supported", "ms-piggyback-first-notify");
    if polling {
        m.push_header("Expires", "0");
    }
    m.push_header("Require", "adhoclist, categoryList");
    m.push_header("Supported", "eventlist");
    m.push_header("Content-Type", "application/msrtc-adrl-categorylist+xml");
}

/// Headers of a single category SUBSCRIBE to one contact (Request-URI = To = contact; no
/// `Require`; §3.4.5.1.3.1, example §4.6, pp. 180–181). With `polling`, a one-time query.
pub fn single_headers(m: &mut Message, polling: bool) {
    m.push_header("Event", "presence");
    m.push_header(
        "Accept",
        "application/msrtc-event-categories+xml, application/rlmi+xml, multipart/related",
    );
    m.push_header("Supported", "ms-piggyback-first-notify");
    if polling {
        m.push_header("Expires", "0");
    }
    m.push_header("Content-Type", "application/msrtc-adrl-categorylist+xml");
}

/// Adds the headers of the contact list SUBSCRIBE (SIP-07; §4.14.3.2, p. 200). No `ms-ucs`: the
/// server then proxies a contact list kept in Exchange (PR-51).
pub fn contacts_headers(m: &mut Message, polling: bool) {
    m.push_header("Event", "vnd-microsoft-roaming-contacts");
    m.push_header("Accept", "application/vnd-microsoft-roaming-contacts+xml");
    m.push_header("Supported", "ms-piggyback-first-notify");
    if polling {
        m.push_header("Expires", "0");
    }
}

/// One body part of a `multipart/related` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub content_type: String,
    pub content_id: Option<String>,
    pub body: String,
}

const MAX_PARTS: usize = 1024;

/// The `boundary` parameter of a Content-Type header value.
pub fn boundary(content_type: &str) -> Option<String> {
    content_type.split(';').find_map(|p| {
        let (k, v) = p.split_once('=')?;
        k.trim()
            .eq_ignore_ascii_case("boundary")
            .then(|| v.trim().trim_matches('"').to_string())
    })
}

/// Splits a `multipart/related` body (RFC 2046 §5.1; SIP-02). The boundary must be 1–70 safe
/// characters. A missing close delimiter is tolerated at the end of the body (the example on
/// p. 179 omits it; the body is framed by Content-Length anyway).
pub fn split_multipart(body: &str, boundary: &str) -> Result<Vec<Part>, Error> {
    if boundary.is_empty()
        || boundary.len() > 70
        || !boundary
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "'()+_,-./:=?".contains(c))
    {
        return Err(Error::BadMultipart);
    }
    let delimiter = format!("--{boundary}");
    let mut sections = body.split(delimiter.as_str());
    sections.next(); // preamble
    let mut parts = Vec::new();
    for section in sections {
        if section.starts_with("--") {
            break; // close delimiter
        }
        if parts.len() >= MAX_PARTS {
            return Err(Error::BadMultipart);
        }
        let section = section
            .strip_prefix("\r\n")
            .or_else(|| section.strip_prefix('\n'))
            .ok_or(Error::BadMultipart)?;
        let (head, content) = match section.find("\r\n\r\n") {
            Some(i) => (
                section.get(..i).unwrap_or(""),
                section.get(i + 4..).unwrap_or(""),
            ),
            None => match section.find("\n\n") {
                Some(i) => (
                    section.get(..i).unwrap_or(""),
                    section.get(i + 2..).unwrap_or(""),
                ),
                None => return Err(Error::BadMultipart),
            },
        };
        let mut content_type = String::new();
        let mut content_id = None;
        for line in head.lines() {
            let Some((k, v)) = line.split_once(':') else {
                return Err(Error::BadMultipart);
            };
            if k.trim().eq_ignore_ascii_case("content-type") {
                content_type = v.trim().to_string();
            } else if k.trim().eq_ignore_ascii_case("content-id") {
                content_id = Some(v.trim().trim_matches(['<', '>']).to_string());
            }
        }
        let content = content
            .strip_suffix("\r\n")
            .or_else(|| content.strip_suffix('\n'))
            .unwrap_or(content);
        parts.push(Part {
            content_type,
            content_id,
            body: content.trim_end().to_string(),
        });
    }
    Ok(parts)
}

/// What a client shows for one contact.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContactPresence {
    pub uri: String,
    /// From the aggregateState of the state category (PR-30, PR-32).
    pub availability: Option<u32>,
    pub activity: Option<String>,
    pub display_name: Option<String>,
    pub note: Option<String>,
}

impl ContactPresence {
    pub fn class(&self) -> Availability {
        self.availability
            .map_or(Availability::Unknown, Availability::from_number)
    }
}

fn xsi_type(e: &Element) -> Option<&str> {
    e.attributes
        .iter()
        .find(|a| a.name.ends_with(":type") && a.name != "xml:type")
        .map(|a| a.value.as_str())
}

/// Parses an `application/msrtc-event-categories+xml` document (PR-01; §2.2.2.1).
pub fn parse_categories(xml: &str) -> Result<ContactPresence, Error> {
    let root = commscraft_xml::parse(xml, &Limits::default())?;
    if root.name != "categories" || root.namespace.as_deref() != Some(NS_CATEGORIES) {
        return Err(Error::WrongDocument("categories"));
    }
    let uri = checked_sip_uri(
        root.attr("uri")
            .ok_or(Error::WrongDocument("categories uri"))?,
    )?;
    let mut p = ContactPresence {
        uri,
        ..ContactPresence::default()
    };
    for cat in root.children_named("category") {
        match cat.attr("name") {
            Some("state") => {
                for st in cat.children_named("state") {
                    // The aggregateState is what the server computed for this watcher (PR-31).
                    if xsi_type(st) == Some("aggregateState") {
                        p.availability = st
                            .child("availability")
                            .and_then(|a| a.text().trim().parse().ok());
                        p.activity = st
                            .child("activity")
                            .and_then(|a| {
                                a.attr("token")
                                    .map(str::to_string)
                                    .or_else(|| a.child("custom").map(|c| c.text()))
                            })
                            .map(|s| printable(&s));
                    }
                }
            }
            Some("contactCard") => {
                if let Some(name) = cat
                    .child("contactCard")
                    .and_then(|c| c.child("identity"))
                    .and_then(|i| i.child("name"))
                    .and_then(|n| n.child("displayName"))
                {
                    p.display_name = Some(printable(name.text().trim()));
                }
            }
            Some("note") => {
                if let Some(body) = cat.child("note").and_then(|n| n.child("body")) {
                    let t = printable(body.text().trim());
                    if !t.is_empty() {
                        p.note = Some(t);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(p)
}

/// State of one resource in the RLMI part (PR-15; RFC 4662).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceState {
    pub uri: String,
    pub state: String,
    pub reason: Option<String>,
    /// For `resubscribe`: the GRUU of the pool that hosts the contact (§3.4.5.1.3.1, p. 102),
    /// checked; absent for contacts of other deployments (then a single SUBSCRIBE, §4.6).
    pub pool_fqdn: Option<String>,
}

/// Parses the `application/rlmi+xml` part.
pub fn parse_rlmi(xml: &str) -> Result<Vec<ResourceState>, Error> {
    let root = commscraft_xml::parse(xml, &Limits::default())?;
    if root.name != "list" || root.namespace.as_deref() != Some(NS_RLMI) {
        return Err(Error::WrongDocument("rlmi list"));
    }
    let mut out = Vec::new();
    for r in root.children_named("resource") {
        let Some(uri) = r.attr("uri") else { continue };
        for i in r.children_named("instance") {
            out.push(ResourceState {
                uri: printable(uri),
                state: printable(i.attr("state").unwrap_or("")),
                reason: i.attr("reason").map(printable),
                pool_fqdn: i.attr("poolFqdn").and_then(|p| checked_sip_uri(p).ok()),
            });
        }
    }
    Ok(out)
}

/// Presence of all contacts in a batch answer (200 OK with piggyback or NOTIFY; SIP-02, SIP-04).
pub fn parse_batch(msg: &Message) -> Result<(Vec<ResourceState>, Vec<ContactPresence>), Error> {
    let ct = msg.single("Content-Type").ok().flatten().unwrap_or("");
    let body = std::str::from_utf8(&msg.body).map_err(|_| Error::BadMultipart)?;
    let main = ct.split(';').next().unwrap_or("").trim();
    if main.eq_ignore_ascii_case("application/msrtc-event-categories+xml") {
        return Ok((Vec::new(), vec![parse_categories(body)?]));
    }
    if !main.eq_ignore_ascii_case("multipart/related") {
        return Err(Error::WrongDocument("batch content type"));
    }
    let b = boundary(ct).ok_or(Error::BadMultipart)?;
    let mut states = Vec::new();
    let mut contacts = Vec::new();
    for part in split_multipart(body, &b)? {
        let t = part
            .content_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        match t.as_str() {
            "application/rlmi+xml" => states.extend(parse_rlmi(&part.body)?),
            "application/msrtc-event-categories+xml" => {
                contacts.push(parse_categories(&part.body)?)
            }
            _ => {}
        }
    }
    Ok((states, contacts))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    pub uri: String,
    pub name: String,
    pub groups: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContactList {
    pub delta_num: Option<u64>,
    pub ucs_mode: Option<String>,
    pub groups: Vec<Group>,
    pub contacts: Vec<Contact>,
    /// Entries that were skipped because their URI is not a safe SIP URI (printable form).
    pub skipped: Vec<String>,
}

/// Parses `application/vnd-microsoft-roaming-contacts+xml` (SIP-07; [MS-SIP] §2.2.4.1; MS-PRES
/// §4.14). Group 1 `~` is the default group. URIs are trimmed (the MS-SIP example has trailing
/// spaces, p. 49) and get `sip:` if they have no scheme.
pub fn parse_contact_list(xml: &str) -> Result<ContactList, Error> {
    let root = commscraft_xml::parse(xml, &Limits::default())?;
    if root.name != "contactList" || !matches!(root.namespace.as_deref(), None | Some(NS_CONTACTS))
    {
        return Err(Error::WrongDocument("contactList"));
    }
    let mut l = ContactList {
        delta_num: root.attr("deltaNum").and_then(|v| v.parse().ok()),
        ucs_mode: root.attr("ucsMode").map(printable),
        ..ContactList::default()
    };
    for g in root.children_named("group") {
        if let Some(id) = g.attr("id").and_then(|v| v.parse().ok()) {
            l.groups.push(Group {
                id,
                name: printable(g.attr("name").unwrap_or("").trim()),
            });
        }
    }
    for c in root.children_named("contact") {
        let raw = c.attr("uri").unwrap_or("");
        match checked_sip_uri(raw) {
            Ok(uri) => l.contacts.push(Contact {
                uri,
                name: printable(c.attr("name").unwrap_or("").trim()),
                groups: c
                    .attr("groups")
                    .unwrap_or("")
                    .split_whitespace()
                    .filter_map(|g| g.parse().ok())
                    .collect(),
            }),
            Err(_) => l.skipped.push(printable(raw)),
        }
    }
    Ok(l)
}

#[cfg(test)]
mod tests;
