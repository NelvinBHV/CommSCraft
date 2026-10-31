//! Conferences over the conferencing server (sans-I/O): C3P commands (MS-CONFPRO addConference,
//! MS-CONFBAS addUser join and dial-out, MS-CONFAV modifyEndpointMedia), the conference roster
//! (RFC 4575 with MS-CONFBAS/MS-CONFAV extensions: MCU URIs, lobby state, A/V media and filters),
//! the join manager's meeting document (ocsmeet), delivery notifications of the IM MCU
//! (MS-CONFIM IMDN) and invitations.
//!
//! Evidence: `docs/protocol/evidence/` files 18 (CB-10 … CB-37), 19 (PV-01 … PV-11) and 21
//! (CI-01 … CI-12). Sources: [MS-CONFPRO] 20250520 §2.2.2, §4.1 (pp. 19–26, 41–42);
//! [MS-CONFBAS] 20250520 §2.2.2–2.2.3 (pp. 24–49); [MS-CONFIM] 20250520 §2.2.2–2.2.4 (pp. 13–14),
//! §4.1–4.2 (pp. 28–58); [MS-CONFBAS] §2.2.6.3 (ocsmeet, pp. 55–56), §3.4 (lobby, pp. 92–97),
//! example §3.2.4 addUser with clientInfo/lobby-capable (pp. 63–64); [MS-CONFAV] 20250520
//! §2.2.2.1 (A/V endpoint media), §2.2.3.3 and example §4.3 (modifyEndpointMedia, pp. 24, 54–56).
//!
//! Security by design: every URI taken from the network is checked (`sip:` only, no characters
//! that could break a header or XML); values we put into XML are escaped; all documents are read
//! with the bounded XML reader (no DTD); the roster is bounded (participants, endpoints); display
//! text is stripped of control characters.

use commscraft_presence::{checked_sip_uri, printable};
use commscraft_xml::{escape, Element, Limits};

pub const CONTENT_TYPE: &str = "application/cccp+xml";
pub const ROSTER_TYPE: &str = "application/conference-info+xml";
pub const IMDN_TYPE: &str = "application/ms-imdn+xml";
pub const INVITATION_TYPE: &str = "application/ms-conf-invite+xml";
pub const NS_CCCP: &str = "urn:ietf:params:xml:ns:cccp";
pub const NS_CI: &str = "urn:ietf:params:xml:ns:conference-info";
pub const NS_MSCI: &str = "http://schemas.microsoft.com/rtc/2005/08/confinfoextensions";
pub const NS_MSCP: &str = "http://schemas.microsoft.com/rtc/2005/08/cccpextensions";
pub const NS_IMDN: &str = "http://schemas.microsoft.com/rtc/2005/08/imdn";
pub const NS_MSCI_2008: &str = "http://schemas.microsoft.com/rtc/2008/12/confinfoextensions";
pub const NS_SEPARATOR: &str = "urn:ietf:params:xml:ns:conference-info-separator";
pub const NS_OCSMEET: &str = "http://schemas.microsoft.com/rtc/2009/05/simplejoinconfdoc";
/// The join manager's meeting document for clients (CB-41).
pub const MEETING_ACCEPT: &str = "Application/vnd.microsoft.lync.meeting+xml;ver=15.0";
/// Bounds of the roster (design).
pub const MAX_PARTICIPANTS: usize = 250;
pub const MAX_ENDPOINTS: usize = 16;
/// sendMessageTimer of the IM MCU flow (CI-05, §3.3.1.5.1, p. 22).
pub const IMDN_TIMEOUT: u64 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Xml(commscraft_xml::Error),
    UnsafeUri,
    WrongDocument(&'static str),
    BadValue(&'static str),
}

fn limits() -> Limits {
    Limits {
        max_input: 512 * 1024,
        max_depth: 16,
        ..Limits::default()
    }
}

fn uri(raw: &str) -> Result<String, Error> {
    checked_sip_uri(raw).map_err(|_| Error::UnsafeUri)
}

/// A conference URI of the focus: `sip:…;gruu;opaque=app:conf:focus:id:…` (form of all examples,
/// PV-01/CB-10; the focus factory names it in the addConference response).
pub fn is_focus_uri(u: &str) -> bool {
    checked_sip_uri(u).is_ok() && u.to_ascii_lowercase().contains(";opaque=app:conf:focus:")
}

/// Upper-case hex of 16 random bytes: a conference ID of 32 characters (PV-04: 8–32
/// alphanumeric, unique per organizer). 128 random bits also make the conference URI unguessable
/// for the `openAuthenticated` admission policy (design).
pub fn conference_id(random: &[u8; 16]) -> String {
    random.iter().map(|b| format!("{b:02X}")).collect()
}

/// An endpoint GUID `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}` (CB-11: "SHOULD use a GUID").
pub fn guid(random: &[u8; 16]) -> String {
    let h: String = random.iter().map(|b| format!("{b:02X}")).collect();
    format!(
        "{{{}-{}-{}-{}-{}}}",
        h.get(0..8).unwrap_or(""),
        h.get(8..12).unwrap_or(""),
        h.get(12..16).unwrap_or(""),
        h.get(16..20).unwrap_or(""),
        h.get(20..32).unwrap_or("")
    )
}

fn request(request_id: u32, from: &str, to: &str, command: &str) -> Result<String, Error> {
    let from = uri(from)?;
    let to = uri(to)?;
    Ok(format!(
        "<?xml version=\"1.0\"?><request xmlns=\"{NS_CCCP}\" xmlns:mscp=\"{NS_MSCP}\" C3PVersion=\"1\" to=\"{}\" from=\"{}\" requestId=\"{request_id}\">{command}</request>",
        escape(&to),
        escape(&from)
    ))
}

/// addConference to the focus factory for an ad-hoc IM conference (PV-02, PV-03, PV-06;
/// [MS-CONFPRO] §4.1, p. 41): only the `chat` MCU, admission policy `openAuthenticated` as in the
/// example.
pub fn add_conference(
    request_id: u32,
    organizer: &str,
    focus_factory: &str,
    conference_id: &str,
) -> Result<String, Error> {
    if conference_id.is_empty()
        || conference_id.len() > 32
        || !conference_id.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return Err(Error::BadValue("conference-id"));
    }
    request(
        request_id,
        organizer,
        focus_factory,
        &format!(
            "<addConference><ci:conference-info xmlns:ci=\"{NS_CI}\" entity=\"\" xmlns:msci=\"{NS_MSCI}\"><ci:conference-description><msci:conference-id>{conference_id}</msci:conference-id><msci:admission-policy>openAuthenticated</msci:admission-policy></ci:conference-description><msci:conference-view><msci:entity-view entity=\"chat\"/></msci:conference-view></ci:conference-info></addConference>"
        ),
    )
}

/// The role asked for when joining (CB-11). The focus decides and answers the granted role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Presenter,
    Attendee,
}

impl Role {
    fn as_str(self) -> &'static str {
        match self {
            Role::Presenter => "presenter",
            Role::Attendee => "attendee",
        }
    }
}

/// addUser in the INVITE to the focus (CB-10, CB-11; [MS-CONFIM] §4.1.1 step 1, p. 28).
pub fn add_user_join(
    request_id: u32,
    user: &str,
    focus: &str,
    endpoint_guid: &str,
    role: Role,
) -> Result<String, Error> {
    let focus_c = uri(focus)?;
    let user_c = uri(user)?;
    request(
        request_id,
        user,
        focus,
        &format!(
            // CB-11/CB-23: we can wait in the lobby (clientInfo/lobby-capable as in the example).
            "<addUser><conferenceKeys confEntity=\"{}\"/><ci:user xmlns:ci=\"{NS_CI}\" entity=\"{}\"><ci:roles><ci:entry>{}</ci:entry></ci:roles><ci:endpoint entity=\"{}\" xmlns:msci=\"{NS_MSCI}\"><clientInfo xmlns=\"{NS_MSCI}\"><separator xmlns=\"{NS_SEPARATOR}\"/><lobby-capable xmlns=\"{NS_MSCI_2008}\">true</lobby-capable></clientInfo></ci:endpoint></ci:user></addUser>",
            escape(&focus_c),
            escape(&user_c),
            role.as_str(),
            escape(endpoint_guid)
        ),
    )
}

/// addUser dial-out over INFO to the focus: the IM MCU invites `invitee` (CB-37; [MS-CONFBAS]
/// §2.2.3.15, pp. 44–45; [MS-CONFIM] §4.1.2 step 1, p. 41).
pub fn add_user_dial_out(
    request_id: u32,
    from: &str,
    focus: &str,
    chat_mcu: &str,
    invitee: &str,
    endpoint_guid: &str,
) -> Result<String, Error> {
    let focus_c = uri(focus)?;
    let mcu = uri(chat_mcu)?;
    let invitee = uri(invitee)?;
    request(
        request_id,
        from,
        focus,
        &format!(
            "<addUser mscp:mcuUri=\"{}\"><conferenceKeys confEntity=\"{}\"/><ci:user xmlns:ci=\"{NS_CI}\" entity=\"{}\"><ci:roles><ci:entry>attendee</ci:entry></ci:roles><ci:endpoint entity=\"{}\" xmlns:msci=\"{NS_MSCI}\"><ci:joining-method>dialed-out</ci:joining-method><ci:media id=\"chat\"><ci:type>chat</ci:type><ci:status>sendrecv</ci:status></ci:media></ci:endpoint></ci:user></addUser>",
            escape(&mcu),
            escape(&focus_c),
            escape(&invitee),
            escape(endpoint_guid)
        ),
    )
}

/// modifyEndpointMedia (CA-20; [MS-CONFAV] §2.2.3.3, example §4.3): block or unblock the audio
/// a user's A/V endpoint sends to the MCU (mute). Sent over INFO to the focus.
pub fn modify_endpoint_media(
    request_id: u32,
    from: &str,
    focus: &str,
    av_mcu: &str,
    target: &AudioKeys,
    block: bool,
) -> Result<String, Error> {
    let focus_c = uri(focus)?;
    let mcu = uri(av_mcu)?;
    let user = uri(&target.user)?;
    if !safe_token(&target.endpoint) || !safe_token(&target.media_id) {
        return Err(Error::BadValue("media keys"));
    }
    request(
        request_id,
        from,
        focus,
        &format!(
            "<modifyEndpointMedia mscp:mcuUri=\"{}\"><mediaKeys confEntity=\"{}\" userEntity=\"{}\" endpointEntity=\"{}\" mediaId=\"{}\"/><ci:media xmlns:ci=\"{NS_CI}\" id=\"{}\"><ci:type>audio</ci:type><media-ingress-filter>{}</media-ingress-filter></ci:media></modifyEndpointMedia>",
            escape(&mcu),
            escape(&focus_c),
            escape(&user),
            escape(&target.endpoint),
            escape(&target.media_id),
            escape(&target.media_id),
            if block { "block" } else { "unblock" }
        ),
    )
}

/// Endpoint GUIDs and media IDs from the roster: short, printable, no XML specials.
fn safe_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "{}-_.".contains(c))
}

/// The keys of one user's audio in the A/V MCU (modifyEndpointMedia mediaKeys).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioKeys {
    pub user: String,
    pub endpoint: String,
    pub media_id: String,
}

/// The conference URI from the join manager's meeting document (CB-40/CB-41; [MS-CONFBAS]
/// §2.2.6.3): `conf-uri` must be a focus URI; an empty one means the URL was not understood.
pub fn parse_meeting(xml: &str) -> Result<String, Error> {
    let root = commscraft_xml::parse(xml, &limits()).map_err(Error::Xml)?;
    if root.name != "conf-info" || root.namespace.as_deref() != Some(NS_OCSMEET) {
        return Err(Error::WrongDocument("ocsmeet conf-info"));
    }
    let u = root
        .child("conf-uri")
        .map(|c| c.text().trim().to_string())
        .unwrap_or_default();
    if is_focus_uri(&u) {
        uri(&u)
    } else {
        Err(Error::BadValue("conf-uri"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Success,
    Pending,
    Failure,
}

/// A C3P response (CB-30, PV-07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub request_id: u32,
    pub code: Code,
    /// Name of the command element (addConference, addUser …).
    pub command: String,
    /// `reason` of the response or the command element (printable).
    pub reason: Option<String>,
    /// addConference: the conference URI (conference-info/@entity, K-PV-1).
    pub conference: Option<String>,
    /// addUser: the user and the granted role.
    pub user: Option<String>,
    pub role: Option<String>,
}

fn attr_local<'a>(e: &'a Element, local: &str) -> Option<&'a str> {
    e.attributes
        .iter()
        .find(|a| a.name == local || a.name.rsplit_once(':').is_some_and(|(_, l)| l == local))
        .map(|a| a.value.as_str())
}

pub fn parse_response(xml: &str) -> Result<Response, Error> {
    let root = commscraft_xml::parse(xml, &limits()).map_err(Error::Xml)?;
    if root.name != "response" || root.namespace.as_deref() != Some(NS_CCCP) {
        return Err(Error::WrongDocument("cccp response"));
    }
    let request_id = root
        .attr("requestId")
        .and_then(|v| v.trim().parse().ok())
        .ok_or(Error::BadValue("requestId"))?;
    let code = match root.attr("code") {
        Some("success") => Code::Success,
        Some("pending") => Code::Pending,
        Some("failure") => Code::Failure,
        _ => return Err(Error::BadValue("code")),
    };
    let cmd = root.elements().next();
    let reason = root
        .attr("reason")
        .or_else(|| cmd.and_then(|c| c.attr("reason")))
        .map(printable);
    let conference = cmd
        .and_then(|c| c.child("conference-info"))
        .and_then(|ci| ci.attr("entity"))
        .and_then(|e| uri(e).ok());
    let user_el = cmd.and_then(|c| c.child("user"));
    Ok(Response {
        request_id,
        code,
        command: cmd.map(|c| c.name.clone()).unwrap_or_default(),
        reason,
        conference,
        user: user_el
            .and_then(|u| u.attr("entity"))
            .and_then(|e| uri(e).ok()),
        role: user_el
            .and_then(|u| u.child("roles"))
            .and_then(|r| r.child("entry"))
            .map(|e| printable(&e.text())),
    })
}

/// One endpoint of a participant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub entity: String,
    /// Session type: focus, chat, audio-video …
    pub kind: String,
    /// connected, on-hold (lobby, CB-23), …
    pub status: String,
    /// The endpoint's SIP URI (GRUU), if named.
    pub uri: Option<String>,
    /// A/V endpoint: the audio media's ID and whether the MCU blocks it (CA-12, CA-20).
    pub audio: Option<(String, bool)>,
}

/// One participant of the conference roster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    pub uri: String,
    pub name: Option<String>,
    pub roles: Vec<String>,
    pub endpoints: Vec<Endpoint>,
}

impl Participant {
    /// True if the participant takes part in the IM conference (an endpoint with
    /// `session-type="chat"`, CB-24).
    pub fn in_chat(&self) -> bool {
        self.endpoints.iter().any(|e| e.kind == "chat")
    }

    /// The participant's audio in the A/V MCU: `Some(muted)`.
    pub fn audio_muted(&self) -> Option<bool> {
        self.endpoints
            .iter()
            .find_map(|e| e.audio.as_ref().map(|(_, b)| *b))
    }
}

/// The conference state as far as group IM needs it (CB-22, CB-24).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roster {
    pub version: u32,
    /// MCU conference URI with purpose `chat` (from `conf-uris`).
    pub chat_uri: Option<String>,
    /// MCU conference URI with purpose `audio-video` (CA-01).
    pub av_uri: Option<String>,
    pub participants: Vec<Participant>,
}

impl Roster {
    /// Applies a conference-info document (RFC 4575 §3.7: `full` replaces, `partial` merges,
    /// `deleted` removes; an older or equal version of a partial document is ignored). Returns
    /// true if something changed.
    pub fn apply(&mut self, xml: &str, focus: &str) -> Result<bool, Error> {
        let root = commscraft_xml::parse(xml, &limits()).map_err(Error::Xml)?;
        if root.name != "conference-info" || root.namespace.as_deref() != Some(NS_CI) {
            return Err(Error::WrongDocument("conference-info"));
        }
        if !root
            .attr("entity")
            .is_some_and(|e| e.eq_ignore_ascii_case(focus))
        {
            return Err(Error::WrongDocument("other conference"));
        }
        let version: u32 = root
            .attr("version")
            .and_then(|v| v.trim().parse().ok())
            .ok_or(Error::BadValue("version"))?;
        let full = root.attr("state") == Some("full");
        if !full && version <= self.version {
            return Ok(false);
        }
        if full {
            *self = Roster::default();
        }
        self.version = version;
        if let Some(uris) = root
            .child("conference-description")
            .and_then(|d| d.child("conf-uris"))
        {
            for e in uris.children_named("entry") {
                let purpose = e.child("purpose").map(|p| p.text()).unwrap_or_default();
                let u = e.child("uri").and_then(|u| uri(u.text().trim()).ok());
                match purpose.trim() {
                    "chat" => self.chat_uri = u,
                    "audio-video" => self.av_uri = u,
                    _ => {}
                }
            }
        }
        if let Some(users) = root.child("users") {
            for u in users.children_named("user") {
                self.apply_user(u);
            }
        }
        Ok(true)
    }

    fn apply_user(&mut self, u: &Element) {
        let Some(entity) = u.attr("entity").and_then(|e| uri(e).ok()) else {
            return;
        };
        let pos = self
            .participants
            .iter()
            .position(|p| p.uri.eq_ignore_ascii_case(&entity));
        if u.attr("state") == Some("deleted") {
            if let Some(i) = pos {
                self.participants.remove(i);
            }
            return;
        }
        let i = match pos {
            Some(i) if u.attr("state") == Some("full") => {
                if let Some(p) = self.participants.get_mut(i) {
                    p.endpoints.clear();
                    p.roles.clear();
                }
                i
            }
            Some(i) => i,
            None if self.participants.len() < MAX_PARTICIPANTS => {
                self.participants.push(Participant {
                    uri: entity,
                    name: None,
                    roles: Vec::new(),
                    endpoints: Vec::new(),
                });
                self.participants.len() - 1
            }
            None => return,
        };
        let Some(p) = self.participants.get_mut(i) else {
            return;
        };
        if let Some(d) = u.child("display-text") {
            p.name = Some(printable(&d.text())).filter(|s| !s.is_empty());
        }
        if let Some(r) = u.child("roles") {
            p.roles = r
                .children_named("entry")
                .map(|e| printable(&e.text()))
                .take(4)
                .collect();
        }
        for ep in u.children_named("endpoint") {
            let Some(id) = ep.attr("entity").map(printable) else {
                continue;
            };
            let previous = p
                .endpoints
                .iter()
                .position(|e| e.entity == id)
                .map(|i| p.endpoints.remove(i));
            if ep.attr("state") == Some("deleted") {
                continue;
            }
            // A disconnected endpoint is no longer in the session (CB-24 status).
            let status = ep.child("status").map(|s| printable(s.text().trim()));
            if status.as_deref() == Some("disconnected") {
                continue;
            }
            // A partial endpoint element updates what it names.
            let mut e = previous.unwrap_or(Endpoint {
                entity: id,
                kind: String::new(),
                status: String::new(),
                uri: None,
                audio: None,
            });
            if let Some(k) = attr_local(ep, "session-type") {
                e.kind = printable(k);
            }
            if let Some(s) = status {
                e.status = s;
            }
            if let Some(u) = attr_local(ep, "endpoint-uri").and_then(|u| uri(u).ok()) {
                e.uri = Some(u);
            }
            for m in ep.children_named("media") {
                let is_audio = m.child("type").is_some_and(|t| t.text().trim() == "audio");
                let Some(mid) = m.attr("id").filter(|i| safe_token(i)) else {
                    continue;
                };
                if is_audio {
                    let blocked = m
                        .child("media-ingress-filter")
                        .map(|f| f.text().trim() == "block")
                        .unwrap_or_else(|| e.audio.as_ref().is_some_and(|(_, b)| *b));
                    e.audio = Some((mid.to_string(), blocked));
                }
            }
            if p.endpoints.len() < MAX_ENDPOINTS {
                p.endpoints.push(e);
            }
        }
    }
}

impl Roster {
    /// Our own focus endpoint is in the lobby (`on-hold`, CB-23).
    pub fn in_lobby(&self, own: &str) -> bool {
        self.own(own).is_some_and(|p| {
            p.endpoints
                .iter()
                .any(|e| e.kind == "focus" && e.status == "on-hold")
        })
    }

    /// The keys of our audio in the A/V MCU; with several devices, the one with our GRUU.
    pub fn own_audio(&self, own: &str, gruu: &str) -> Option<AudioKeys> {
        let p = self.own(own)?;
        let av = |e: &&Endpoint| e.kind == "audio-video" && e.audio.is_some();
        let e = p
            .endpoints
            .iter()
            .filter(av)
            .find(|e| {
                e.uri
                    .as_deref()
                    .is_some_and(|u| u.eq_ignore_ascii_case(gruu))
            })
            .or_else(|| p.endpoints.iter().find(av))?;
        Some(AudioKeys {
            user: p.uri.clone(),
            endpoint: e.entity.clone(),
            media_id: e.audio.as_ref()?.0.clone(),
        })
    }

    fn own(&self, own: &str) -> Option<&Participant> {
        self.participants
            .iter()
            .find(|p| p.uri.eq_ignore_ascii_case(own))
    }
}

/// Delivery notification of the IM MCU (CI-05, schema §6.2, p. 75; example §4.2.2, pp. 57–58):
/// the message ID and the recipients it could not be delivered to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imdn {
    pub message_id: u32,
    pub failures: Vec<(String, u16)>,
}

pub fn parse_imdn(xml: &str) -> Result<Imdn, Error> {
    let root = commscraft_xml::parse(xml, &limits()).map_err(Error::Xml)?;
    if root.name != "imdn" || root.namespace.as_deref() != Some(NS_IMDN) {
        return Err(Error::WrongDocument("imdn"));
    }
    let message_id = root
        .child("message-id")
        .and_then(|m| m.text().trim().parse().ok())
        .ok_or(Error::BadValue("message-id"))?;
    let mut failures = Vec::new();
    for r in root.children_named("recipient").take(MAX_PARTICIPANTS) {
        // The uri attribute is a name-addr in the example ("<sip:…>").
        let raw = r.attr("uri").unwrap_or("");
        let u = raw.trim().trim_start_matches('<').trim_end_matches('>');
        let u = u.split(';').next().unwrap_or("");
        let status = r
            .child("status")
            .and_then(|s| s.text().trim().parse::<u16>().ok())
            .unwrap_or(0);
        if let (Ok(u), true) = (uri(u), status >= 300) {
            failures.push((u, status));
        }
    }
    Ok(Imdn {
        message_id,
        failures,
    })
}

/// `Message-Id` header of a 202 (digits; missing or invalid ⇒ 0, CI-05).
pub fn message_id(value: Option<&str>) -> u32 {
    value.and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

/// The sender of a forwarded message (`Ms-Sender`, CI-06): display name and SIP address
/// (opaque/gruu parameters of the GRUU removed).
pub fn ms_sender(value: &str) -> Option<(Option<String>, String)> {
    let na = commscraft_sip_msg::parse_name_addr(value).ok()?;
    let bare = na.uri.split(';').next().unwrap_or("");
    let u = uri(bare).ok()?;
    let name = na.display.map(|d| printable(&d)).filter(|d| !d.is_empty());
    Some((name, u))
}

/// An invitation to a conference (INVITE with `application/ms-conf-invite+xml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invitation {
    pub focus: String,
    pub subject: Option<String>,
    /// The invitation offers IM (`<im available="true">`).
    pub im: bool,
}

/// Reads an invitation document. **Secondary evidence only** (K-CI-3): MS-SIPRE §3.9.5.1 (p. 58)
/// names the content type and an `audio` element with `available="true"`; the element names
/// `Conferencing`, `focus-uri`, `subject`, `im` come from [SIPE] `sipe-conf.c` l. 70–77. Only a
/// checked focus URI is accepted; joining is always the user's decision.
pub fn parse_invitation(xml: &str) -> Result<Invitation, Error> {
    let root = commscraft_xml::parse(
        xml,
        &Limits {
            max_input: 8 * 1024,
            max_depth: 6,
            ..Limits::default()
        },
    )
    .map_err(Error::Xml)?;
    if root.name != "Conferencing" {
        return Err(Error::WrongDocument("Conferencing"));
    }
    let focus = root
        .child("focus-uri")
        .map(|f| f.text().trim().to_string())
        .ok_or(Error::BadValue("focus-uri"))?;
    if !is_focus_uri(&focus) {
        return Err(Error::UnsafeUri);
    }
    Ok(Invitation {
        focus,
        subject: root
            .child("subject")
            .map(|s| printable(&s.text()))
            .filter(|s| !s.is_empty()),
        im: root
            .child("im")
            .is_some_and(|i| i.attr("available") == Some("true")),
    })
}

#[cfg(test)]
mod tests;
