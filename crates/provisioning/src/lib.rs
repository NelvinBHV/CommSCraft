//! In-band provisioning: the SUBSCRIBE for `vnd-microsoft-provisioning-v2` and the parsed
//! `provisionGroupList` with a validated view of the ServerConfiguration group.
//!
//! Evidence: `docs/protocol/evidence/04-siprege-registration-provisioning.md` (RG-40 … RG-44,
//! ServerConfiguration values, K-RG-4). Source: [MS-SIPREGE] 20250819 §2.2.2 (pp. 19–72), example
//! §4.2 (pp. 97–111).
//!
//! Security by design (`docs/security.md`):
//! - the body is parsed with the bounded `commscraft-xml` reader (no DTD, no entities);
//! - provisioning arrives over the signed SIP channel, but its values are still treated as hostile
//!   input: web service URLs must be `https://` with a plain host name, and a host outside the
//!   user's SIP domain is marked so that the caller asks the user before sending credentials
//!   there; SIP URIs must be `sip:` without characters that could break a header; numbers and
//!   enumerations are range-checked; invalid values are dropped, never guessed;
//! - the caller verifies the MS-SIPAE signature of the 200 OK or NOTIFY before calling
//!   [`from_message`].

use commscraft_sip_msg::Message;
use commscraft_xml::{Element, Limits};

pub const EVENT: &str = "vnd-microsoft-provisioning-v2";
pub const CONTENT_TYPE: &str = "application/vnd-microsoft-roaming-provisioning-v2+xml";
const REQUEST_NS: &str = "http://schemas.microsoft.com/2006/09/sip/provisioninggrouplist";
const RESPONSE_NS: &str =
    "http://schemas.microsoft.com/2006/09/sip/provisiongrouplist-notification";

/// Groups requested by the client (RG-42; MS-SIPREGE §2.2.2.4 and example §4.2.1).
pub const GROUPS: &[&str] = &[
    "ServerConfiguration",
    "meetingPolicy",
    "ucPolicy",
    "userSetting",
    "endpointConfiguration",
    "locationPolicy",
    "mediaConfiguration",
    "presencePolicyV2",
    "publicationGrammar",
    "privacyPublicationGrammar",
    "persistentChatConfiguration",
    "publicProviders",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Xml(commscraft_xml::Error),
    /// Root element or namespace is not the provisioning notification.
    WrongDocument,
    /// The message is not a provisioning response (Event or Content-Type missing or different).
    NotProvisioning,
    /// A group without a name, or a property without a name.
    MissingName,
    InvalidValue,
}

impl From<commscraft_xml::Error> for Error {
    fn from(e: commscraft_xml::Error) -> Self {
        Error::Xml(e)
    }
}

/// Inputs of the provisioning SUBSCRIBE.
#[derive(Debug, Clone)]
pub struct Request<'a> {
    pub aor: &'a str,
    pub gruu: &'a str,
    pub epid: &'a str,
    pub from_tag: &'a str,
    pub call_id: &'a str,
    pub cseq: u32,
    /// Top Via value incl. branch, chosen by the transaction layer.
    pub via: &'a str,
    pub user_agent: &'a str,
    pub subnet: Option<&'a str>,
}

/// Builds the SUBSCRIBE (RG-40, RG-42, RG-44; example §4.2.1, pp. 97–98). `Expires: 0`: no
/// subscription dialog, the answer comes piggybacked in the 200 OK or in one NOTIFY. The caller
/// signs it with the security association. `ms-benotify` is not offered (BENOTIFY not supported
/// yet).
pub fn subscribe(r: &Request<'_>) -> Message {
    let mut m = Message::request("SUBSCRIBE", r.aor);
    m.push_header("Via", r.via);
    m.push_header("Max-Forwards", "70");
    m.push_header(
        "From",
        &format!("<{}>;tag={};epid={}", r.aor, r.from_tag, r.epid),
    );
    m.push_header("To", &format!("<{}>", r.aor));
    m.push_header("Call-ID", r.call_id);
    m.push_header("CSeq", &format!("{} SUBSCRIBE", r.cseq));
    m.push_header("Contact", &format!("<{}>", r.gruu));
    m.push_header("User-Agent", r.user_agent);
    m.push_header("Event", EVENT);
    m.push_header("Accept", CONTENT_TYPE);
    m.push_header("Supported", "ms-piggyback-first-notify");
    m.push_header("Expires", "0");
    m.push_header("Content-Type", CONTENT_TYPE);
    let subnet = r
        .subnet
        .map(|s| format!(" subnet=\"{}\"", commscraft_xml::escape(s)))
        .unwrap_or_default();
    let mut body = format!("<provisioningGroupList xmlns=\"{REQUEST_NS}\"{subnet}>");
    for g in GROUPS {
        body.push_str(&format!("<provisioningGroup name=\"{g}\"/>"));
    }
    body.push_str("</provisioningGroupList>");
    m.body = body.into_bytes();
    m
}

/// One provisioning group: named properties and, for policies, instances of properties.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub properties: Vec<(String, String)>,
    pub instances: Vec<Vec<(String, String)>>,
}

impl Group {
    /// Property by name, compared case-insensitively: the specification spells several names
    /// differently in schema, prose and example (K-RG-4).
    pub fn get(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Provisioning {
    pub groups: Vec<Group>,
}

impl Provisioning {
    pub fn group(&self, name: &str) -> Option<&Group> {
        self.groups
            .iter()
            .find(|g| g.name.eq_ignore_ascii_case(name))
    }
}

const MAX_VALUE_LEN: usize = 64 * 1024;

fn props_of(list: &Element) -> Result<Vec<(String, String)>, Error> {
    let mut out = Vec::new();
    for p in list.children_named("property") {
        let name = p.attr("name").ok_or(Error::MissingName)?;
        let value = p.text();
        if value.len() > MAX_VALUE_LEN {
            return Err(Error::InvalidValue);
        }
        out.push((name.to_string(), value.trim().to_string()));
    }
    Ok(out)
}

/// Parses a `provisionGroupList` (RG-43). Three group shapes occur in §4.2.2: `propertyEntryList`
/// with `property name=…`, `instance` lists of properties (policies), and direct child elements
/// (ServerConfiguration, userSetting).
pub fn parse(body: &str) -> Result<Provisioning, Error> {
    let root = commscraft_xml::parse(body, &Limits::default())?;
    if root.name != "provisionGroupList" || root.namespace.as_deref() != Some(RESPONSE_NS) {
        return Err(Error::WrongDocument);
    }
    let mut groups = Vec::new();
    for g in root.children_named("provisionGroup") {
        let mut group = Group {
            name: g.attr("name").ok_or(Error::MissingName)?.to_string(),
            ..Group::default()
        };
        for child in g.elements() {
            match child.name.as_str() {
                "propertyEntryList" => group.properties.extend(props_of(child)?),
                "instance" => group.instances.push(props_of(child)?),
                _ => {
                    let value = child.text();
                    if value.len() > MAX_VALUE_LEN {
                        return Err(Error::InvalidValue);
                    }
                    group
                        .properties
                        .push((child.name.clone(), value.trim().to_string()));
                }
            }
        }
        groups.push(group);
    }
    Ok(Provisioning { groups })
}

/// Extracts the provisioning document from a verified 200 OK (piggyback, RG-41) or NOTIFY.
/// `Ok(None)` for a 200 OK without body (the data then follows in a NOTIFY).
pub fn from_message(msg: &Message) -> Result<Option<Provisioning>, Error> {
    if msg.body.is_empty() && msg.is_response() {
        return Ok(None);
    }
    let event_ok = msg.header_values("Event").any(|e| {
        e.split(';')
            .next()
            .is_some_and(|v| v.trim().eq_ignore_ascii_case(EVENT))
    });
    let type_ok = msg.single("Content-Type").ok().flatten().is_some_and(|t| {
        t.split(';')
            .next()
            .is_some_and(|v| v.trim().eq_ignore_ascii_case(CONTENT_TYPE))
    });
    if !event_ok || !type_ok {
        return Err(Error::NotProvisioning);
    }
    let body = std::str::from_utf8(&msg.body).map_err(|_| Error::InvalidValue)?;
    parse(body).map(Some)
}

/// A web service URL from provisioning, checked (security by design).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceUrl {
    pub url: String,
    pub host: String,
    /// True if the host is the user's SIP domain or a subdomain of it. Otherwise the caller must
    /// ask the user before sending credentials there (`docs/security.md`, Discovery).
    pub in_user_domain: bool,
}

/// Accepts only `https://host[:port]/path` with a DNS host name (inference/design: MS-SIPREGE
/// says these URLs SHOULD or MUST be HTTPS). The checks live in `commscraft-url`.
pub fn service_url(value: &str, user_domain: &str) -> Option<ServiceUrl> {
    let u = commscraft_url::parse_https(value)?;
    Some(ServiceUrl {
        in_user_domain: commscraft_url::in_domain(&u.host, user_domain),
        url: u.url,
        host: u.host,
    })
}

/// Accepts a SIP URI that is safe to put into a header (`sip:` scheme, no whitespace, quotes,
/// angle brackets or commas).
pub fn sip_uri(value: &str) -> Option<String> {
    let ok = value.len() <= 1024
        && value
            .get(..4)
            .is_some_and(|s| s.eq_ignore_ascii_case("sip:"))
        && value.len() > 4
        && !value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "\"<>,\\".contains(c));
    ok.then(|| value.to_string())
}

/// Encryption policy for computer-to-computer audio/video (MS-SIPREGE §2.2.2.5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvEncryption {
    Support,
    Require,
    DoNotSupport,
}

/// A port range; only accepted with 1024 ≤ min ≤ max ≤ 65535 (inference: no privileged ports).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    pub min: u16,
    pub max: u16,
}

/// The ServerConfiguration values the client uses, validated. Unknown or invalid values are left
/// out; `rejected` names what was dropped so it can be logged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerConfiguration {
    pub abs_internal: Option<ServiceUrl>,
    pub abs_external: Option<ServiceUrl>,
    pub dlx_internal: Option<ServiceUrl>,
    pub dlx_external: Option<ServiceUrl>,
    pub dlx_enabled: Option<bool>,
    pub abwq_internal: Option<ServiceUrl>,
    pub abwq_external: Option<ServiceUrl>,
    pub lis_internal: Option<ServiceUrl>,
    pub mras_uri: Option<String>,
    pub qos_uri: Option<String>,
    pub focus_factory_uri: Option<String>,
    pub voice_mail_uri: Option<String>,
    pub call_park_server_uri: Option<String>,
    pub av_encryption: Option<AvEncryption>,
    pub port_range_enabled: Option<bool>,
    pub media_ports: Option<PortRange>,
    pub audio_ports: Option<PortRange>,
    pub video_ports: Option<PortRange>,
    pub app_sharing_ports: Option<PortRange>,
    pub enable_bw_policy_check: Option<bool>,
    pub rejected: Vec<String>,
}

fn boolean(v: &str) -> Option<bool> {
    match v.to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

impl ServerConfiguration {
    pub fn from_group(g: &Group, user_domain: &str) -> Self {
        let mut c = ServerConfiguration::default();
        let url = |name: &str, rejected: &mut Vec<String>| {
            let v = g.get(name)?;
            let u = service_url(v, user_domain);
            if u.is_none() {
                rejected.push(name.to_string());
            }
            u
        };
        let mut rejected = Vec::new();
        c.abs_internal = url("absInternalServerUrl", &mut rejected);
        c.abs_external = url("absExternalServerUrl", &mut rejected);
        c.dlx_internal = url("dlxInternalUrl", &mut rejected);
        c.dlx_external = url("dlxExternalUrl", &mut rejected);
        c.abwq_internal = url("abwqInternalUrl", &mut rejected);
        c.abwq_external = url("abwqExternalUrl", &mut rejected);
        c.lis_internal = url("lisInternalUrl", &mut rejected);
        let mut sip = |name: &str| {
            let v = g.get(name)?;
            let u = sip_uri(v);
            if u.is_none() {
                rejected.push(name.to_string());
            }
            u
        };
        c.mras_uri = sip("mrasUri");
        c.qos_uri = sip("qosUri");
        c.focus_factory_uri = sip("focusFactoryUri");
        c.voice_mail_uri = sip("voiceMailUri");
        c.call_park_server_uri = sip("callParkServerUri");
        let mut flag = |name: &str| {
            let v = g.get(name)?;
            let b = boolean(v);
            if b.is_none() {
                rejected.push(name.to_string());
            }
            b
        };
        c.dlx_enabled = flag("dlxEnabled");
        c.port_range_enabled = flag("ucPortRangeEnabled");
        c.enable_bw_policy_check = flag("enableBWPolicyCheck");
        c.av_encryption = match g.get("ucPC2PCAVEncryption") {
            None => None,
            Some(v) if v.eq_ignore_ascii_case("SupportEncryption") => Some(AvEncryption::Support),
            Some(v) if v.eq_ignore_ascii_case("RequireEncryption") => Some(AvEncryption::Require),
            Some(v) if v.eq_ignore_ascii_case("DoNotSupportEncryption") => {
                Some(AvEncryption::DoNotSupport)
            }
            Some(_) => {
                rejected.push("ucPC2PCAVEncryption".into());
                None
            }
        };
        let mut range = |kind: &str| {
            let (min, max) = (
                g.get(&format!("ucMin{kind}Port")),
                g.get(&format!("ucMax{kind}Port")),
            );
            let (min, max) = (min?, max?);
            let r = match (min.parse::<u16>(), max.parse::<u16>()) {
                (Ok(a), Ok(b)) if (1024..=b).contains(&a) => Some(PortRange { min: a, max: b }),
                _ => None,
            };
            if r.is_none() {
                rejected.push(format!("uc{kind}Ports"));
            }
            r
        };
        c.media_ports = range("Media");
        c.audio_ports = range("Audio");
        c.video_ports = range("Video");
        c.app_sharing_ports = range("AppSharing");
        c.rejected = rejected;
        c
    }
}

#[cfg(test)]
mod tests;
