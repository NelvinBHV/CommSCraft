//! Calendar and out-of-office state from Exchange (function 16; O-EX-1, O-EX-2 closed by OB-48,
//! client behaviour OB-13/OB-14): the user's free/busy times, current meeting and working hours
//! with GetUserAvailability ([MS-OXWAVLS] 20250520) and the automatic replies with
//! GetUserOofSettings ([MS-OXWOOF] 20250520). The presence side is `presence::calendar`.
//!
//! Security by design: the mailbox is checked and XML-escaped; the answer is read with the
//! bounded XML reader; at most [`MAX_EVENTS`] events and [`MAX_SLOTS`] free/busy blocks are
//! kept; texts are stripped of control characters and bounded; the out-of-office message is
//! reduced to plain text (no markup reaches the presence note).

use super::{mailbox, printable, Error, NS_SOAP};
use commscraft_xml::{escape, parse_xs_datetime, xs_datetime, Element, Limits};

const NS_TYPES: &str = "http://schemas.microsoft.com/exchange/services/2006/types";
const NS_MESSAGES: &str = "http://schemas.microsoft.com/exchange/services/2006/messages";
/// `soapAction` of the two operations ([MS-OXWAVLS] §7, p. 61; [MS-OXWOOF] §6, p. 30).
pub const GET_USER_AVAILABILITY_ACTION: &str =
    "\"http://schemas.microsoft.com/exchange/services/2006/messages/GetUserAvailability\"";
pub const GET_USER_OOF_SETTINGS_ACTION: &str =
    "\"http://schemas.microsoft.com/exchange/services/2006/messages/GetUserOofSettings\"";
/// Length of a free/busy block in minutes (OB-13: 15 minutes, as `PT15M` in MS-PRES §2.2.2.7.8).
pub const INTERVAL_MINUTES: u64 = 15;
/// Free/busy blocks asked for: 384 (MS-PRES §2.2.2.7.8, p. 70: "SHOULD contain 384 entries"),
/// four days of 15 minutes.
pub const SLOTS: u64 = 384;
/// Most events and blocks kept from an answer, and the longest subject, location and message
/// (design).
pub const MAX_EVENTS: usize = 512;
pub const MAX_SLOTS: usize = 2048;
pub const MAX_TEXT: usize = 256;
pub const MAX_MESSAGE: usize = 1024;

/// Free/busy state of a block or event ([MS-OXWAVLS] §3.1.4.1.4.1 merged string digits,
/// pp. 42–43; LegacyFreeBusyType).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeBusy {
    Free,
    Tentative,
    Busy,
    Oof,
    /// No data (4) or WorkingElsewhere: shown as free.
    NoData,
}

impl FreeBusy {
    fn from_digit(c: char) -> Option<FreeBusy> {
        Some(match c {
            '0' => FreeBusy::Free,
            '1' => FreeBusy::Tentative,
            '2' => FreeBusy::Busy,
            '3' => FreeBusy::Oof,
            '4' => FreeBusy::NoData,
            _ => return None,
        })
    }

    fn from_name(s: &str) -> FreeBusy {
        match s.trim() {
            "Free" => FreeBusy::Free,
            "Tentative" => FreeBusy::Tentative,
            "Busy" => FreeBusy::Busy,
            "OOF" => FreeBusy::Oof,
            _ => FreeBusy::NoData,
        }
    }

    /// The 2-bit code of MS-PRES §2.2.2.7.8 (p. 69): 00 free, 01 tentative, 10 busy, 11 out of
    /// facility; no data counts as free.
    pub fn code(self) -> u8 {
        match self {
            FreeBusy::Free | FreeBusy::NoData => 0,
            FreeBusy::Tentative => 1,
            FreeBusy::Busy => 2,
            FreeBusy::Oof => 3,
        }
    }
}

/// One appointment ([MS-OXWAVLS] §3.1.4.1.3.1 CalendarEvent, §3.1.4.1.3.3 details).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    pub start: u64,
    pub end: u64,
    pub busy: FreeBusy,
    /// Subject and location only for an event that is not private (IsPrivate).
    pub subject: Option<String>,
    pub location: Option<String>,
}

/// A time-zone transition (SerializableTimeZoneTime, §3.1.4.1.3.20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub bias: i32,
    pub time: String,
    pub day_order: u8,
    pub month: u8,
    pub day_of_week: String,
}

/// One working period (WorkingPeriod, §3.1.4.1.3.26).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingPeriod {
    pub days: String,
    pub start_minutes: u32,
    pub end_minutes: u32,
}

/// The user's working hours (WorkingHours, §3.1.4.1.3.27), kept field by field so that only
/// checked values are written into the presence document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingHours {
    pub bias: i32,
    pub standard: Option<Transition>,
    pub daylight: Option<Transition>,
    pub periods: Vec<WorkingPeriod>,
}

const DAYS: [&str; 10] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Day",
    "Weekday",
    "WeekendDay",
];

fn days_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.split_whitespace().all(|d| DAYS.contains(&d))
}

fn time_ok(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 8
        && b.get(2) == Some(&b':')
        && b.get(5) == Some(&b':')
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 2 || i == 5 || c.is_ascii_digit())
}

fn num<T: std::str::FromStr>(e: &Element, name: &str) -> Option<T> {
    e.child(name)?.text().trim().parse().ok()
}

impl Transition {
    fn from_element(e: &Element) -> Option<Transition> {
        let time = e.child("Time")?.text().trim().to_string();
        let day_of_week = e.child("DayOfWeek")?.text().trim().to_string();
        let t = Transition {
            bias: num(e, "Bias")?,
            day_order: num(e, "DayOrder")?,
            month: num(e, "Month")?,
            time,
            day_of_week,
        };
        let ok = time_ok(&t.time)
            && DAYS[..7].contains(&t.day_of_week.as_str())
            && (1..=5).contains(&t.day_order)
            && (1..=12).contains(&t.month)
            && t.bias.abs() <= 1440;
        ok.then_some(t)
    }

    fn write(&self, name: &str, out: &mut String) {
        out.push_str(&format!(
            "<{name}><Bias>{}</Bias><Time>{}</Time><DayOrder>{}</DayOrder><Month>{}</Month><DayOfWeek>{}</DayOfWeek></{name}>",
            self.bias, self.time, self.day_order, self.month, self.day_of_week
        ));
    }
}

impl WorkingHours {
    fn from_element(e: &Element) -> Option<WorkingHours> {
        let tz = e.child("TimeZone")?;
        let bias: i32 = num(tz, "Bias")?;
        if bias.abs() > 1440 {
            return None;
        }
        let periods = e
            .child("WorkingPeriodArray")?
            .children_named("WorkingPeriod")
            .filter_map(|p| {
                let w = WorkingPeriod {
                    days: p.child("DayOfWeek")?.text().trim().to_string(),
                    start_minutes: num(p, "StartTimeInMinutes")?,
                    end_minutes: num(p, "EndTimeInMinutes")?,
                };
                (days_ok(&w.days) && w.start_minutes <= 1440 && w.end_minutes <= 1440).then_some(w)
            })
            .take(16)
            .collect();
        Some(WorkingHours {
            bias,
            standard: tz.child("StandardTime").and_then(Transition::from_element),
            daylight: tz.child("DaylightTime").and_then(Transition::from_element),
            periods,
        })
    }

    /// The WorkingHours element in the Exchange types namespace, as MS-PRES §2.2.2.7.8 (p. 67)
    /// carries it inside `calendarData`.
    pub fn to_xml(&self) -> String {
        let mut out = format!(
            "<WorkingHours xmlns=\"{NS_TYPES}\"><TimeZone><Bias>{}</Bias>",
            self.bias
        );
        if let Some(s) = &self.standard {
            s.write("StandardTime", &mut out);
        }
        if let Some(d) = &self.daylight {
            d.write("DaylightTime", &mut out);
        }
        out.push_str("</TimeZone><WorkingPeriodArray>");
        for p in &self.periods {
            out.push_str(&format!(
                "<WorkingPeriod><DayOfWeek>{}</DayOfWeek><StartTimeInMinutes>{}</StartTimeInMinutes><EndTimeInMinutes>{}</EndTimeInMinutes></WorkingPeriod>",
                p.days, p.start_minutes, p.end_minutes
            ));
        }
        out.push_str("</WorkingPeriodArray></WorkingHours>");
        out
    }
}

/// The user's calendar as GetUserAvailability returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Availability {
    /// Start of the first block (Unix time, UTC).
    pub start: u64,
    pub slots: Vec<FreeBusy>,
    pub events: Vec<CalendarEvent>,
    pub working_hours: Option<WorkingHours>,
}

impl Availability {
    /// The meeting going on at `now`: a busy event (the Lync 2013 SDK "Publication behaviors of
    /// category instances": calendarState for "an ongoing meeting scheduled in the Microsoft
    /// Exchange calendar store", 6500 in-a-meeting; tentative and out-of-facility events are
    /// left out – inference). With several, the one that started last.
    pub fn current_meeting(&self, now: u64) -> Option<&CalendarEvent> {
        self.events
            .iter()
            .filter(|e| e.busy == FreeBusy::Busy && e.start <= now && now < e.end)
            .max_by_key(|e| e.start)
    }
}

/// The window asked for: from the start of the current UTC day, [`SLOTS`] blocks long.
pub fn window(now: u64) -> (u64, u64) {
    let start = now - now % 86_400;
    (start, start + SLOTS * INTERVAL_MINUTES * 60)
}

/// The UTC time zone in the request: times in the answer are then UTC (§3.1.4.1.3.19).
/// Both transitions are required and carry no offset (bias 0); their dates follow the example
/// §4.1 (p. 46). K-AW-3 (found 2026-11-09): with two equal transition dates the server refuses
/// the zone ("the specified time zone is not valid", `tested-in-lab`).
const UTC_ZONE: &str = "<TimeZone xmlns=\"http://schemas.microsoft.com/exchange/services/2006/types\"><Bias>0</Bias>\
<StandardTime><Bias>0</Bias><Time>02:00:00</Time><DayOrder>5</DayOrder><Month>10</Month><DayOfWeek>Sunday</DayOfWeek></StandardTime>\
<DaylightTime><Bias>0</Bias><Time>02:00:00</Time><DayOrder>1</DayOrder><Month>4</Month><DayOfWeek>Sunday</DayOfWeek></DaylightTime></TimeZone>";

fn local(unix: u64) -> Result<String, Error> {
    // The request's times are local to its time zone (UTC here): without the "Z".
    xs_datetime(unix)
        .map(|s| s.trim_end_matches('Z').to_string())
        .ok_or(Error::WrongDocument("time"))
}

/// The GetUserAvailability request for the user's own mailbox (example §4.1, pp. 46–47;
/// OB-13: DetailedMerged, 15-minute blocks) over `[start, end)`.
pub fn availability_request(address: &str, start: u64, end: u64) -> Result<String, Error> {
    let address = escape(mailbox(address)?);
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"{NS_SOAP}\"><soap:Body>\
<GetUserAvailabilityRequest xmlns=\"{NS_MESSAGES}\">{UTC_ZONE}<MailboxDataArray>\
<MailboxData xmlns=\"{NS_TYPES}\"><Email><Address>{address}</Address><RoutingType>SMTP</RoutingType></Email>\
<AttendeeType>Required</AttendeeType><ExcludeConflicts>false</ExcludeConflicts></MailboxData></MailboxDataArray>\
<FreeBusyViewOptions xmlns=\"{NS_TYPES}\"><TimeWindow><StartTime>{}</StartTime><EndTime>{}</EndTime></TimeWindow>\
<MergedFreeBusyIntervalInMinutes>{INTERVAL_MINUTES}</MergedFreeBusyIntervalInMinutes><RequestedView>DetailedMerged</RequestedView>\
</FreeBusyViewOptions></GetUserAvailabilityRequest></soap:Body></soap:Envelope>",
        local(start)?,
        local(end)?
    ))
}

fn parse(xml: &str) -> Result<Element, Error> {
    commscraft_xml::parse(
        xml,
        &Limits {
            max_input: 1024 * 1024,
            ..Limits::default()
        },
    )
    .map_err(Error::Xml)
}

fn body<'a>(root: &'a Element, wanted: &str) -> Result<&'a Element, Error> {
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
    body.child(wanted)
        .filter(|r| r.namespace.as_deref() == Some(NS_MESSAGES))
        .ok_or(Error::WrongDocument("operation response"))
}

/// ResponseMessage with ResponseClass Success and ResponseCode NoError ([MS-OXWSCDATA]
/// §2.2.4.65).
fn success(m: Option<&Element>) -> Result<(), Error> {
    let m = m.ok_or(Error::WrongDocument("ResponseMessage"))?;
    let code = m
        .child("ResponseCode")
        .map(|c| printable(&c.text()))
        .unwrap_or_default();
    if m.attr("ResponseClass") != Some("Success") || (code != "NoError" && !code.is_empty()) {
        return Err(Error::Server(if code.is_empty() {
            "Error".into()
        } else {
            code
        }));
    }
    Ok(())
}

fn text_of(e: &Element, name: &str, max: usize) -> Option<String> {
    let t: String = printable(&e.child(name)?.text())
        .chars()
        .take(max)
        .collect();
    (!t.is_empty()).then_some(t)
}

/// Reads the GetUserAvailability answer (example §4.2, pp. 47–50) for a window that began at
/// `start`: the first FreeBusyResponse (one mailbox was asked for).
pub fn parse_availability(xml: &str, start: u64) -> Result<Availability, Error> {
    let root = parse(xml)?;
    let r = body(&root, "GetUserAvailabilityResponse")?;
    let fb = r
        .child("FreeBusyResponseArray")
        .and_then(|a| a.child("FreeBusyResponse"))
        .ok_or(Error::WrongDocument("FreeBusyResponse"))?;
    success(fb.child("ResponseMessage"))?;
    let view = fb
        .child("FreeBusyView")
        .ok_or(Error::WrongDocument("FreeBusyView"))?;
    let slots = view
        .child("MergedFreeBusy")
        .map(|m| {
            m.text()
                .trim()
                .chars()
                .take(MAX_SLOTS)
                .map(|c| FreeBusy::from_digit(c).ok_or(Error::WrongDocument("MergedFreeBusy")))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let events = view
        .child("CalendarEventArray")
        .map(|a| {
            a.children_named("CalendarEvent")
                .filter_map(|e| {
                    let start = parse_xs_datetime(&e.child("StartTime")?.text())?;
                    let end = parse_xs_datetime(&e.child("EndTime")?.text())?;
                    let busy = FreeBusy::from_name(&e.child("BusyType")?.text());
                    let details = e.child("CalendarEventDetails");
                    let public = details.filter(|d| {
                        d.child("IsPrivate").map(|p| p.text().trim().to_string())
                            != Some("true".into())
                    });
                    (start < end).then(|| CalendarEvent {
                        start,
                        end,
                        busy,
                        subject: public.and_then(|d| text_of(d, "Subject", MAX_TEXT)),
                        location: public.and_then(|d| text_of(d, "Location", MAX_TEXT)),
                    })
                })
                .take(MAX_EVENTS)
                .collect()
        })
        .unwrap_or_default();
    Ok(Availability {
        start,
        slots,
        events,
        working_hours: view
            .child("WorkingHours")
            .and_then(WorkingHours::from_element),
    })
}

/// OofState ([MS-OXWOOF] §2.2.5.2, p. 15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OofState {
    Disabled,
    Enabled,
    Scheduled,
}

/// The automatic replies of the user ([MS-OXWOOF] §2.2.4.4 UserOofSettings, pp. 13–14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oof {
    pub state: OofState,
    /// Duration of a scheduled reply (UTC, §2.2.4.4: "SHOULD … be specified in UTC").
    pub start: Option<u64>,
    pub end: Option<u64>,
    /// The internal reply as plain text.
    pub message: String,
}

impl Oof {
    /// Whether the replies are on at `now`: enabled, or scheduled with `now` in the duration.
    pub fn active(&self, now: u64) -> bool {
        match self.state {
            OofState::Disabled => false,
            OofState::Enabled => true,
            OofState::Scheduled => match (self.start, self.end) {
                (Some(s), Some(e)) => s <= now && now < e,
                _ => false,
            },
        }
    }
}

/// The GetUserOofSettings request for the user's mailbox (example §4.1, p. 28).
pub fn oof_request(address: &str) -> Result<String, Error> {
    let address = escape(mailbox(address)?);
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"{NS_SOAP}\"><soap:Body>\
<GetUserOofSettingsRequest xmlns=\"{NS_MESSAGES}\"><Mailbox xmlns=\"{NS_TYPES}\"><Address>{address}</Address></Mailbox>\
</GetUserOofSettingsRequest></soap:Body></soap:Envelope>"
    ))
}

/// Reads the GetUserOofSettings answer (example §4.2, pp. 28–29).
pub fn parse_oof(xml: &str) -> Result<Oof, Error> {
    let root = parse(xml)?;
    let r = body(&root, "GetUserOofSettingsResponse")?;
    success(r.child("ResponseMessage"))?;
    let s = r
        .child("OofSettings")
        .ok_or(Error::WrongDocument("OofSettings"))?;
    let state = match s
        .child("OofState")
        .map(|e| e.text())
        .as_deref()
        .map(str::trim)
    {
        Some("Enabled") => OofState::Enabled,
        Some("Scheduled") => OofState::Scheduled,
        Some("Disabled") => OofState::Disabled,
        _ => return Err(Error::WrongDocument("OofState")),
    };
    let duration = s.child("Duration");
    let time = |name: &str| {
        duration
            .and_then(|d| d.child(name))
            .and_then(|t| parse_xs_datetime(&t.text()))
    };
    let message = s
        .child("InternalReply")
        .and_then(|i| i.child("Message"))
        .map(|m| plain_text(&m.text()))
        .unwrap_or_default();
    Ok(Oof {
        state,
        start: time("StartTime"),
        end: time("EndTime"),
        message,
    })
}

/// Plain text of a reply message, which Exchange may keep as HTML: markup, the content of
/// `head`, `style` and `script`, and comments are dropped; the five XML entities, `&nbsp;` and
/// numeric references are decoded; white space is collapsed; control characters removed;
/// at most [`MAX_MESSAGE`] characters.
pub fn plain_text(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    let mut skip_until: Option<&str> = None;
    while !rest.is_empty() && out.len() < MAX_MESSAGE * 4 {
        if let Some(end) = skip_until {
            match find_ci(rest, end) {
                Some(i) => {
                    rest = &rest[i..];
                    skip_until = None;
                }
                None => break,
            }
            continue;
        }
        if let Some(r) = rest.strip_prefix("<!--") {
            rest = r.find("-->").map_or("", |i| &r[i + 3..]);
            continue;
        }
        if rest.starts_with('<') {
            let end = rest.find('>').map_or(rest.len(), |i| i + 1);
            let tag = rest[1..end].trim_start_matches('/').to_ascii_lowercase();
            let name: String = tag
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            if !rest[1..].starts_with('/') {
                skip_until = match name.as_str() {
                    "head" => Some("</head"),
                    "style" => Some("</style"),
                    "script" => Some("</script"),
                    _ => None,
                };
            }
            if matches!(name.as_str(), "br" | "p" | "div" | "li" | "tr") {
                out.push(' ');
            }
            rest = &rest[end..];
            continue;
        }
        if rest.starts_with('&') {
            if let Some(i) = rest.find(';').filter(|i| *i <= 10) {
                let entity = &rest[1..i];
                let c = match entity {
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "amp" => Some('&'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    "nbsp" => Some(' '),
                    e => e
                        .strip_prefix("#x")
                        .or_else(|| e.strip_prefix("#X"))
                        .and_then(|h| u32::from_str_radix(h, 16).ok())
                        .or_else(|| e.strip_prefix('#').and_then(|d| d.parse().ok()))
                        .and_then(char::from_u32),
                };
                if let Some(c) = c {
                    out.push(c);
                    rest = &rest[i + 1..];
                    continue;
                }
            }
        }
        let mut chars = rest.chars();
        if let Some(c) = chars.next() {
            out.push(c);
        }
        rest = chars.as_str();
    }
    let collapsed = out
        .split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    collapsed.chars().take(MAX_MESSAGE).collect()
}

fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    let h = hay.as_bytes();
    let n = needle.as_bytes();
    h.windows(n.len()).position(|w| w.eq_ignore_ascii_case(n))
}
