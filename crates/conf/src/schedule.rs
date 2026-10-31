//! Scheduling meetings at the focus factory (function 40; [MS-CONFPRO] 20250520): create a
//! meeting with subject, expiry time and admission policy (addConference), list the organizer's
//! meetings (getConferences), read one (getConference), change it (modifyConference, with the
//! current version) and delete it (deleteConference) – PV-04, PV-06, PV-09, PV-13, PV-14.
//!
//! Security by design: every value that goes into a request is checked (conference ID 8–32
//! ASCII alphanumerics, subject bounded and printable, times in UTC); answers are parsed with the
//! bounded reader, the number of meetings and the length of every text are limited, a join URL is
//! only reported if it is an `https` URL.

use super::*;

/// The admission policy of a meeting ([MS-CONFPRO] §2.2.1.2, p. 14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Only the participants of the preset roster.
    ClosedAuthenticated,
    /// Everyone with an enterprise identity (also federated users).
    OpenAuthenticated,
    /// Also anonymous users with a valid passcode.
    Anonymous,
}

impl Admission {
    pub fn as_str(self) -> &'static str {
        match self {
            Admission::ClosedAuthenticated => "closedAuthenticated",
            Admission::OpenAuthenticated => "openAuthenticated",
            Admission::Anonymous => "anonymous",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "closedAuthenticated" => Some(Admission::ClosedAuthenticated),
            "openAuthenticated" => Some(Admission::OpenAuthenticated),
            "anonymous" => Some(Admission::Anonymous),
            _ => None,
        }
    }
}

/// What a scheduled meeting is created or changed with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingSpec {
    /// 8–32 ASCII alphanumerics, unique per organizer (§2.2.1.2, p. 14).
    pub conference_id: String,
    pub subject: String,
    /// Unix time after which the focus factory may delete the meeting (expiry-time).
    pub expiry: u64,
    pub admission: Admission,
    /// Further options (PV-15); the default leaves everything to the server.
    pub extras: Extras,
}

/// Options of a scheduled meeting beyond subject, lifetime and admission (PV-15; [MS-CONFPRO]
/// §2.2.1.2–2.2.1.4, pp. 14–16; example §4.2, pp. 42–44). `None` = not sent, the server's default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extras {
    /// Who becomes presenter on admission ([`crate::AUTOPROMOTE_COMPANY`],
    /// [`crate::AUTOPROMOTE_EVERYONE`], 0 = nobody).
    pub autopromote: Option<u32>,
    /// Phone callers skip the lobby.
    pub pstn_lobby_bypass: Option<bool>,
    /// Everyone but the organizer waits in the lobby (conference-state/locked, §2.2.1.3).
    pub locked: Option<bool>,
    /// Invitees with their role (SIP URI, presenter); definitive for `closedAuthenticated`; the
    /// organizer is implicit and not listed (§2.2.1.4).
    pub users: Vec<(String, bool)>,
}

/// Most invitees in one meeting document (design).
pub const MAX_INVITEES: usize = 100;

/// A meeting as the focus factory reports it (getConferences / getConference, §2.2.3.4–2.2.3.5).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Meeting {
    /// The conference URI (focus), when the answer names it.
    pub focus: Option<String>,
    pub conference_id: String,
    pub subject: Option<String>,
    /// expiry-time as sent by the server (printable).
    pub expiry: Option<String>,
    pub admission: Option<Admission>,
    /// The HTTP join URL (join-url, §2.2.1.2, p. 15), only if it is an `https` URL.
    pub join_url: Option<String>,
    /// The conference version (needed for modifyConference).
    pub version: Option<u32>,
    /// The organizer's static meeting (`static="true"`, PV-16).
    pub static_meeting: bool,
}

/// Longest subject we send (design; the specification sets no limit).
pub const MAX_SUBJECT: usize = 256;
/// Most meetings taken from one getConferences answer (design).
pub const MAX_MEETINGS: usize = 200;
/// Latest expiry we ask for: one year from now (design; the server enforces its own bound, PV-13).
pub const MAX_LIFETIME: u64 = 366 * 24 * 3600;

/// 8–32 ASCII alphanumerics (PV-04: "SHOULD allow for a minimum of 8 and a maximum of 32").
pub fn valid_conference_id(id: &str) -> bool {
    (8..=32).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric())
}

pub use commscraft_xml::xs_datetime;

fn check_subject(s: &str) -> Result<(), Error> {
    if s.trim().is_empty() || s.chars().count() > MAX_SUBJECT || s.chars().any(char::is_control) {
        return Err(Error::BadValue("subject"));
    }
    Ok(())
}

fn description(spec: &MeetingSpec, organizer: &str) -> Result<String, Error> {
    if !valid_conference_id(&spec.conference_id) {
        return Err(Error::BadValue("conference-id"));
    }
    check_subject(&spec.subject)?;
    let expiry = xs_datetime(spec.expiry).ok_or(Error::BadValue("expiry-time"))?;
    let x = &spec.extras;
    // Schema conference-description-type (MS-CONFBAS §6.3.2): separator, optional pstn-access,
    // separator, then autopromote and pstn-lobby-bypass in this order (PV-15).
    let mut options = String::new();
    if let Some(a) = x.autopromote {
        if a & !(crate::AUTOPROMOTE_COMPANY | crate::AUTOPROMOTE_EVERYONE) != 0 {
            return Err(Error::BadValue("autopromote"));
        }
        options.push_str(&format!("<msci:autopromote>{a}</msci:autopromote>"));
    }
    if let Some(b) = x.pstn_lobby_bypass {
        options.push_str(&format!(
            "<msci:pstn-lobby-bypass>{b}</msci:pstn-lobby-bypass>"
        ));
    }
    if !options.is_empty() {
        options = format!("<cis:separator xmlns:cis=\"{NS_SEPARATOR}\"/><cis:separator xmlns:cis=\"{NS_SEPARATOR}\"/>{options}");
    }
    let state = x
        .locked
        .map(|l| format!("<ci:conference-state><ci:locked>{l}</ci:locked></ci:conference-state>"))
        .unwrap_or_default();
    let users = invitees(&x.users, organizer)?;
    Ok(format!(
        "<ci:conference-description><ci:subject>{}</ci:subject><msci:conference-id>{}</msci:conference-id><msci:expiry-time>{expiry}</msci:expiry-time><msci:admission-policy>{}</msci:admission-policy>{options}</ci:conference-description>{state}{users}",
        escape(&spec.subject),
        spec.conference_id,
        spec.admission.as_str()
    ))
}

/// `users` with one role each (§2.2.1.4): unique, checked SIP URIs, the organizer left out.
fn invitees(users: &[(String, bool)], organizer: &str) -> Result<String, Error> {
    if users.is_empty() {
        return Ok(String::new());
    }
    if users.len() > MAX_INVITEES {
        return Err(Error::BadValue("users"));
    }
    let mut seen: Vec<String> = Vec::new();
    let mut out = String::from("<ci:users>");
    for (u, presenter) in users {
        let u = uri(u)?;
        let key = u.to_ascii_lowercase();
        if key == organizer.to_ascii_lowercase() {
            continue;
        }
        if seen.contains(&key) {
            return Err(Error::BadValue("users"));
        }
        seen.push(key);
        let role = if *presenter {
            crate::Role::Presenter
        } else {
            crate::Role::Attendee
        };
        out.push_str(&format!(
            "<ci:user entity=\"{}\"><ci:roles><ci:entry>{}</ci:entry></ci:roles></ci:user>",
            escape(&u),
            role.as_str()
        ));
    }
    out.push_str("</ci:users>");
    Ok(out)
}

/// addConference for the organizer's static meeting (PV-16; §2.2.2.1, p. 19; example §4.1,
/// p. 42): only the conference ID and `static="true"`; the administrator's defaults apply.
pub fn schedule_static(
    request_id: u32,
    organizer: &str,
    focus_factory: &str,
    conference_id: &str,
) -> Result<String, Error> {
    if !valid_conference_id(conference_id) {
        return Err(Error::BadValue("conference-id"));
    }
    request(
        request_id,
        organizer,
        focus_factory,
        &format!(
            "<addConference><ci:conference-info xmlns:ci=\"{NS_CI}\" entity=\"\" xmlns:msci=\"{NS_MSCI}\" static=\"true\"><ci:conference-description><msci:conference-id>{conference_id}</msci:conference-id></ci:conference-description></ci:conference-info></addConference>"
        ),
    )
}

/// The MCUs of a scheduled meeting: IM and audio (PV-12).
const MEETING_VIEW: &str = "<msci:conference-view><msci:entity-view entity=\"chat\"/><msci:entity-view entity=\"audio-video\"/></msci:conference-view>";

/// addConference for a scheduled meeting (PV-06, PV-09; §2.2.2.1, p. 19; example §4.1, p. 41):
/// entity empty, the conference ID, subject, expiry time, admission policy, IM and A/V MCU.
pub fn schedule_conference(
    request_id: u32,
    organizer: &str,
    focus_factory: &str,
    spec: &MeetingSpec,
) -> Result<String, Error> {
    let d = description(spec, organizer)?;
    request(
        request_id,
        organizer,
        focus_factory,
        &format!(
            "<addConference><ci:conference-info xmlns:ci=\"{NS_CI}\" entity=\"\" xmlns:msci=\"{NS_MSCI}\">{d}{MEETING_VIEW}</ci:conference-info></addConference>"
        ),
    )
}

/// modifyConference (PV-14; §2.2.2.2, p. 20; example §4.2, p. 42): the same document as for
/// addConference, `version` = the conference's current version (from getConference).
pub fn modify_conference(
    request_id: u32,
    organizer: &str,
    focus_factory: &str,
    version: u32,
    spec: &MeetingSpec,
) -> Result<String, Error> {
    if version == 0 {
        return Err(Error::BadValue("version"));
    }
    let d = description(spec, organizer)?;
    request(
        request_id,
        organizer,
        focus_factory,
        &format!(
            "<modifyConference><ci:conference-info xmlns:ci=\"{NS_CI}\" entity=\"\" version=\"{version}\" xmlns:msci=\"{NS_MSCI}\">{d}{MEETING_VIEW}</ci:conference-info></modifyConference>"
        ),
    )
}

fn keyed(
    request_id: u32,
    organizer: &str,
    focus_factory: &str,
    command: &str,
    attrs: &str,
    conference_id: &str,
) -> Result<String, Error> {
    if !valid_conference_id(conference_id) {
        return Err(Error::BadValue("conference-id"));
    }
    request(
        request_id,
        organizer,
        focus_factory,
        &format!(
            "<{command}{attrs}><conferenceKeys xmlns:msci=\"{NS_MSCI}\" msci:conference-id=\"{conference_id}\"/></{command}>"
        ),
    )
}

/// deleteConference (§2.2.2.3, p. 20; example §4.3, p. 46); a static meeting needs
/// `static="true"` on deleteConference (PV-16; schema `delete-conference-type`).
pub fn delete_conference(
    request_id: u32,
    organizer: &str,
    focus_factory: &str,
    conference_id: &str,
    static_meeting: bool,
) -> Result<String, Error> {
    let attrs = if static_meeting {
        " static=\"true\""
    } else {
        ""
    };
    keyed(
        request_id,
        organizer,
        focus_factory,
        "deleteConference",
        attrs,
        conference_id,
    )
}

/// getConference without an encryption key (§2.2.2.4, p. 21; the conference key is deprecated,
/// OB-24; example §4.5, p. 49).
pub fn get_conference(
    request_id: u32,
    organizer: &str,
    focus_factory: &str,
    conference_id: &str,
) -> Result<String, Error> {
    keyed(
        request_id,
        organizer,
        focus_factory,
        "getConference",
        "",
        conference_id,
    )
}

/// getConferences for the organizer's ordinary (not static) meetings (§2.2.2.5, p. 21;
/// example §4.4, p. 47).
pub fn get_conferences(
    request_id: u32,
    organizer: &str,
    focus_factory: &str,
    static_meetings: bool,
) -> Result<String, Error> {
    // PV-16 (§2.2.2.5, p. 21; example §4.6): `static="true"` lists the static meeting.
    let command = if static_meetings {
        "<getConferences static=\"true\"/>"
    } else {
        "<getConferences/>"
    };
    request(request_id, organizer, focus_factory, command)
}

/// Bounded printable text of an element.
fn text_of(e: &Element, max: usize) -> Option<String> {
    let t = printable(e.text().trim());
    (!t.is_empty()).then(|| t.chars().take(max).collect())
}

fn meeting(ci: &Element) -> Option<Meeting> {
    let d = ci.child("conference-description")?;
    let conference_id = d.child("conference-id").map(|e| e.text())?;
    let conference_id = conference_id.trim().to_string();
    if !valid_conference_id(&conference_id) {
        return None;
    }
    let join_url = d
        .child("join-url")
        .map(|e| e.text().trim().to_string())
        .filter(|u| {
            u.len() <= 2048
                && u.get(..8)
                    .is_some_and(|p| p.eq_ignore_ascii_case("https://"))
                && !u.chars().any(|c| c.is_control() || c.is_whitespace())
        });
    Some(Meeting {
        focus: ci
            .attr("entity")
            .filter(|e| !e.is_empty())
            .and_then(|e| uri(e).ok())
            .filter(|e| is_focus_uri(e)),
        conference_id,
        subject: d.child("subject").and_then(|e| text_of(e, MAX_SUBJECT)),
        expiry: d.child("expiry-time").and_then(|e| text_of(e, 32)),
        admission: d
            .child("admission-policy")
            .and_then(|e| Admission::parse(&e.text())),
        join_url,
        version: ci.attr("version").and_then(|v| v.trim().parse().ok()),
        static_meeting: ci.attr("static") == Some("true"),
    })
}

/// The meetings in a getConferences or getConference answer (§2.2.3.4–2.2.3.5, pp. 26–27):
/// conference-info elements directly in the command element or in its `conferences` child.
/// Entries without a usable conference ID are skipped; at most [`MAX_MEETINGS`].
pub fn parse_meetings(xml: &str) -> Result<(Response, Vec<Meeting>), Error> {
    let response = parse_response(xml)?;
    let root = commscraft_xml::parse(xml, &limits()).map_err(Error::Xml)?;
    let mut meetings = Vec::new();
    if let Some(cmd) = root.elements().next() {
        let direct = cmd.children_named("conference-info");
        let listed = cmd
            .child("conferences")
            .into_iter()
            .flat_map(|c| c.children_named("conference-info"));
        // PV-16: an answer to getConferences static="true" says so itself.
        let all_static = cmd.attr("static") == Some("true");
        for ci in direct.chain(listed).take(MAX_MEETINGS) {
            if let Some(mut m) = meeting(ci) {
                m.static_meeting |= all_static;
                meetings.push(m);
            }
        }
    }
    Ok((response, meetings))
}
