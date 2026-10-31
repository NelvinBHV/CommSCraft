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
    /// The location profile (dial plan) for phone numbers ([MS-SIPREGE] §2.2.2.5.1
    /// `ucLocationProfile`, overridden by userSetting `ucUserLocationProfile`, §2.2.2.5.5; RE-40).
    pub location_profile: Option<String>,
    pub rejected: Vec<String>,
}

/// A location profile name as it goes into `phone-context` (inference: dial plan names are
/// tokens; anything else is refused so that it cannot break the SIP URI).
pub fn profile_name(v: &str) -> Option<String> {
    let v = v.trim();
    (!v.is_empty()
        && v.len() <= 128
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)))
    .then(|| v.to_string())
}

impl ServerConfiguration {
    /// Applies the userSetting group: the user's own location profile wins (inference: the
    /// user-specific setting is the more specific one).
    pub fn apply_user_setting(&mut self, g: &Group) {
        if let Some(v) = g.get("ucUserLocationProfile") {
            match profile_name(v) {
                Some(p) => self.location_profile = Some(p),
                None => self.rejected.push("ucUserLocationProfile".into()),
            }
        }
    }
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
        if let Some(v) = g.get("ucLocationProfile") {
            c.location_profile = profile_name(v);
            if c.location_profile.is_none() {
                rejected.push("ucLocationProfile".into());
            }
        }
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

/// Whether the client asks the user for a location when none is known (§2.2.2.5.7, p. 56).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocationRequired {
    #[default]
    No,
    Yes,
    /// Ask, and show the administrator's disclaimer if the user goes on without one.
    Disclaimer,
}

/// One emergency dial string and the dial strings that are translated into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmergencyNumber {
    pub dial: String,
    pub masks: Vec<String>,
}

/// The `locationPolicy` provisioning group (RG-51; [MS-SIPREGE] §2.2.2.5.7, pp. 56–57).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocationPolicy {
    /// EnhancedEmergencyServicesEnabled: send PIDF-LO with emergency calls.
    pub enabled: bool,
    /// LocationPolicyTagID: goes into the PIDF-LO of every emergency call.
    pub tag_id: Option<String>,
    pub location_required: LocationRequired,
    /// UseLocationForE911Only.
    pub e911_only: bool,
    /// EmergencyNumbers, or EmergencyDialString with EmergencyDialMask when that is absent.
    pub numbers: Vec<EmergencyNumber>,
    /// NotificationUri: SIP URIs the server notifies (comma separated).
    pub notification_uris: Vec<String>,
    pub conference_uri: Option<String>,
    /// ConferenceMode: `oneway` or `twoway`.
    pub conference_mode: Option<String>,
    pub disclaimer: Option<String>,
    /// LocationRefreshInterval in hours.
    pub refresh_hours: Option<u32>,
}

/// Bound for emergency numbers and masks (design).
const MAX_EMERGENCY_NUMBERS: usize = 16;

/// A dial string: digits, `*`, `#`, a leading `+`; 1–32 characters (design).
fn dial_string(v: &str) -> Option<String> {
    let v = v.trim();
    let body = v.strip_prefix('+').unwrap_or(v);
    let ok = (1..=32).contains(&body.len())
        && body
            .chars()
            .all(|c| c.is_ascii_digit() || c == '*' || c == '#');
    ok.then(|| v.to_string())
}

impl LocationPolicy {
    /// Reads the group; invalid entries are dropped (fail closed: no number, no emergency path).
    pub fn from_group(g: &Group) -> Self {
        let yes = |n: &str| {
            g.get(n)
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
        };
        let text = |n: &str, max: usize| {
            g.get(n)
                .map(|v| {
                    v.trim()
                        .chars()
                        .filter(|c| !c.is_control())
                        .take(max)
                        .collect::<String>()
                })
                .filter(|v| !v.is_empty())
        };
        // EmergencyNumbers takes precedence over EmergencyDialString (§2.2.2.5.7, p. 57).
        let mut numbers = Vec::new();
        if let Some(list) = g.get("EmergencyNumbers").filter(|v| !v.trim().is_empty()) {
            for combo in list.split(';').take(MAX_EMERGENCY_NUMBERS) {
                let (dial, masks) = combo.split_once('-').unwrap_or((combo, ""));
                if let Some(dial) = dial_string(dial) {
                    numbers.push(EmergencyNumber {
                        dial,
                        masks: masks
                            .split(',')
                            .filter_map(dial_string)
                            .take(MAX_EMERGENCY_NUMBERS)
                            .collect(),
                    });
                }
            }
        } else if let Some(dial) = g.get("EmergencyDialString").and_then(dial_string) {
            numbers.push(EmergencyNumber {
                dial,
                masks: g
                    .get("EmergencyDialMask")
                    .unwrap_or("")
                    .split(';')
                    .filter_map(dial_string)
                    .take(MAX_EMERGENCY_NUMBERS)
                    .collect(),
            });
        }
        LocationPolicy {
            enabled: yes("EnhancedEmergencyServicesEnabled"),
            tag_id: text("LocationPolicyTagID", 256),
            location_required: match g
                .get("LocationRequired")
                .map(|v| v.trim().to_ascii_lowercase())
                .as_deref()
            {
                Some("yes") => LocationRequired::Yes,
                Some("disclaimer") => LocationRequired::Disclaimer,
                _ => LocationRequired::No,
            },
            e911_only: yes("UseLocationForE911Only"),
            numbers,
            notification_uris: g
                .get("NotificationUri")
                .unwrap_or("")
                .split(',')
                .filter_map(|u| sip_uri(u.trim()))
                .take(MAX_EMERGENCY_NUMBERS)
                .collect(),
            conference_uri: g.get("ConferenceUri").and_then(|u| sip_uri(u.trim())),
            conference_mode: g
                .get("ConferenceMode")
                .map(|m| m.trim().to_ascii_lowercase())
                .filter(|m| m == "oneway" || m == "twoway"),
            disclaimer: text("EmergencyServiceDisclaimer", 2048),
            refresh_hours: g
                .get("LocationRefreshInterval")
                .and_then(|v| v.trim().parse().ok()),
        }
    }

    /// The emergency dial string a dialled number stands for (the number itself or one of its
    /// masks), only while enhanced emergency services are enabled.
    pub fn emergency(&self, dialled: &str) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        let d = dialled.trim();
        self.numbers
            .iter()
            .find(|n| n.dial == d || n.masks.iter().any(|m| m == d))
            .map(|n| n.dial.as_str())
    }
}

#[cfg(test)]
mod tests;
