//! Calendar and out-of-office state as presence (function 16): the `calendarData` category
//! (PR-39, [MS-PRES] §2.2.2.7.8, pp. 67–70; §3.8.5.3, p. 129; example §4.3.3, pp. 154–156), the
//! `calendarState` instance of the `state` category (PR-30, §2.2.2.7.1, pp. 50–53; example
//! §4.3.1.1, pp. 143–145) and the out-of-office note (PR-37, note type `OOF`).
//!
//! Instances and containers follow the Lync 2013 SDK page "Publication behaviors of category
//! instances" (official, secondary – OB-13/OB-14):
//! - `calendarData` instance 0, static: the working hours into containers 1, 200, 300 and 400,
//!   empty into 100 and 32000 (as in §4.3.3);
//! - `calendarData` instance "a hash dependent on the user's SMTP address": the free/busy
//!   blocks into 200, 300 and 400, empty into 1, 100 and 32000;
//! - `state` instance `0x4YYYYYYY`: calendarState 6500 `in-a-meeting` with subject and
//!   location of the ongoing meeting into containers 2 and 3;
//! - `state` instance `0x6YYYYYYY`: the `out-of-office` activity into containers 2 and 3;
//! - `note` instance "a hash dependent on the user's SMTP address": the out-of-office message
//!   (type `OOF`) into 200, 300 and 400, empty into 100 and 32000.
//!
//! The SDK calls the free/busy and note instances time-bounded and the out-of-office state
//! static; this client publishes all three with `expireType="endpoint"` – as §4.3.3 does for
//! the free/busy instance – so nothing stale stays behind after it signs out (design). The
//! free/busy value uses the 2-bit encoding (top two instance bits 00, §2.2.2.7.8, p. 69).
//!
//! Security by design: the mailbox is checked; texts are stripped of control characters,
//! bounded and XML-escaped; the working hours are taken only as a parsed `WorkingHours`
//! element and the free/busy value only in the base64 alphabet.

use super::{checked_sip_uri, printable, Error, NS_PUBLISH, NS_STATE};
use commscraft_xml::{escape, xs_datetime, Limits};

pub const NS_CALENDAR_DATA: &str = "http://schemas.microsoft.com/2006/09/sip/calendarData";
pub const NS_NOTE: &str = "http://schemas.microsoft.com/2006/09/sip/note";
const NS_EX_TYPES: &str = "http://schemas.microsoft.com/exchange/services/2006/types";
/// Longest subject, location and note text published (design; [`printable`] bounds every text
/// from the network to 256 characters as well).
pub const MAX_TEXT: usize = 256;
pub const MAX_NOTE: usize = 256;
/// Longest free/busy value (base64 of 384 blocks of 2 bits is 128 characters; design bound).
pub const MAX_FREE_BUSY: usize = 1024;

/// The ongoing meeting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meeting {
    pub start: u64,
    pub subject: Option<String>,
    pub location: Option<String>,
}

/// What the client publishes from the user's calendar. `None` means "not published".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Calendar {
    /// The SMTP address of the mailbox (`mailboxID`).
    pub mailbox: String,
    /// A `WorkingHours` element in the Exchange types namespace.
    pub working_hours: Option<String>,
    /// Start of the first block and the base64 free/busy value ([`free_busy_value`]).
    pub free_busy: Option<(u64, String)>,
    pub meeting: Option<Meeting>,
    /// The automatic reply as plain text while it is on.
    pub out_of_office: Option<String>,
}

/// The 2-bit codes (0 free, 1 tentative, 2 busy, 3 out of facility; §2.2.2.7.8, p. 69) packed
/// most significant bit first; the last byte is padded with free blocks.
pub fn free_busy_value(codes: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; codes.len().div_ceil(4)];
    for (i, c) in codes.iter().enumerate() {
        if let Some(b) = out.get_mut(i / 4) {
            *b |= (c & 3) << (6 - 2 * (i % 4));
        }
    }
    out
}

/// Number of publications this module manages; their order is fixed (see [`slots`]).
pub const SLOTS: usize = 21;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    WorkingHours,
    FreeBusy,
    Meeting,
    OofState,
    OofNote,
}

/// (group, category, container, carries the content) per slot.
const LAYOUT: [(Group, &str, u32, bool); SLOTS] = [
    (Group::WorkingHours, "calendarData", 1, true),
    (Group::WorkingHours, "calendarData", 200, true),
    (Group::WorkingHours, "calendarData", 300, true),
    (Group::WorkingHours, "calendarData", 400, true),
    (Group::WorkingHours, "calendarData", 100, false),
    (Group::WorkingHours, "calendarData", 32000, false),
    (Group::FreeBusy, "calendarData", 200, true),
    (Group::FreeBusy, "calendarData", 300, true),
    (Group::FreeBusy, "calendarData", 400, true),
    (Group::FreeBusy, "calendarData", 1, false),
    (Group::FreeBusy, "calendarData", 100, false),
    (Group::FreeBusy, "calendarData", 32000, false),
    (Group::Meeting, "state", 2, true),
    (Group::Meeting, "state", 3, true),
    (Group::OofState, "state", 2, true),
    (Group::OofState, "state", 3, true),
    (Group::OofNote, "note", 200, true),
    (Group::OofNote, "note", 300, true),
    (Group::OofNote, "note", 400, true),
    (Group::OofNote, "note", 100, false),
    (Group::OofNote, "note", 32000, false),
];

fn hash(s: &str, salt: u32) -> u32 {
    // FNV-1a; only stability matters (the SDK names "a hash dependent on the SMTP address").
    let mut h: u32 = 0x811c_9dc5 ^ salt;
    for b in s.to_ascii_lowercase().bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// Instance IDs: free/busy, out-of-office note, calendarState, out-of-office state.
pub fn instances(mailbox: &str) -> [u32; 4] {
    let nonzero = |v: u32| if v == 0 { 1 } else { v };
    [
        nonzero(hash(mailbox, 1) & 0x3fff_ffff),
        nonzero(hash(mailbox, 2) & 0x3fff_ffff),
        0x4000_0000 | (hash(mailbox, 3) & 0x0fff_ffff),
        0x6000_0000 | (hash(mailbox, 4) & 0x0fff_ffff),
    ]
}

fn instance_of(group: Group, ids: &[u32; 4]) -> u32 {
    match group {
        Group::WorkingHours => 0,
        Group::FreeBusy => ids[0],
        Group::OofNote => ids[1],
        Group::Meeting => ids[2],
        Group::OofState => ids[3],
    }
}

fn present(c: &Calendar, g: Group) -> bool {
    match g {
        Group::WorkingHours => c.working_hours.is_some(),
        Group::FreeBusy => c.free_busy.is_some(),
        Group::Meeting => c.meeting.is_some(),
        Group::OofState | Group::OofNote => c.out_of_office.is_some(),
    }
}

/// Whether `c` publishes the group of `slot` (a deleted slot's version starts again at 0).
pub fn slot_present(c: &Calendar, slot: usize) -> bool {
    LAYOUT.get(slot).is_some_and(|(g, ..)| present(c, *g))
}

fn same(a: &Calendar, b: &Calendar, g: Group) -> bool {
    match g {
        Group::WorkingHours => a.working_hours == b.working_hours,
        Group::FreeBusy => a.free_busy == b.free_busy,
        Group::Meeting => a.meeting == b.meeting,
        Group::OofState => a.out_of_office.is_some() == b.out_of_office.is_some(),
        Group::OofNote => a.out_of_office == b.out_of_office,
    }
}

/// The slots a publication from `old` (what the server has) to `new` touches: changed groups
/// that are or were published (§3.8.5.3: no publication without a change).
pub fn slots(new: &Calendar, old: &Calendar) -> Vec<usize> {
    let mailbox_changed = !new.mailbox.eq_ignore_ascii_case(&old.mailbox);
    LAYOUT
        .iter()
        .enumerate()
        .filter(|(_, (g, ..))| {
            (present(new, *g) || present(old, *g)) && (mailbox_changed || !same(new, old, *g))
        })
        .map(|(i, _)| i)
        .collect()
}

fn text(s: &str, max: usize) -> String {
    escape(&printable(s).chars().take(max).collect::<String>())
}

fn checked_working_hours(xml: &str) -> Result<&str, Error> {
    let e = commscraft_xml::parse(xml, &Limits::default())?;
    if e.name != "WorkingHours" || e.namespace.as_deref() != Some(NS_EX_TYPES) {
        return Err(Error::WrongDocument("WorkingHours"));
    }
    Ok(xml)
}

/// The SERVICE body publishing `slots` (from [`slots`]) of `new` at the current `versions`
/// (PR-05); a slot whose group `new` no longer has is deleted (`expires="0"`, §4.3.2.1,
/// p. 150). `endpoint_id` is the endpoint's `+sip.instance` UUID (PR-02).
pub fn publish_body(
    aor: &str,
    endpoint_id: &str,
    new: &Calendar,
    old: &Calendar,
    slots: &[usize],
    versions: &[u32; SLOTS],
) -> Result<String, Error> {
    let aor = checked_sip_uri(aor)?;
    if endpoint_id.is_empty()
        || !endpoint_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        return Err(Error::UnsafeUri);
    }
    // The mailbox is the SIP address' form of an SMTP address: the same character check.
    let mailbox = checked_sip_uri(&format!("sip:{}", new.mailbox))?;
    let mailbox = escape(mailbox.trim_start_matches("sip:"));
    let ids = instances(&new.mailbox);
    let old_ids = instances(&old.mailbox);
    let mut b = format!(
        "<publish xmlns=\"{NS_PUBLISH}\"><publications uri=\"{}\">",
        escape(&aor)
    );
    for &slot in slots {
        let (group, category, container, full) = *LAYOUT.get(slot).ok_or(Error::UnsafeUri)?;
        let version = versions.get(slot).copied().unwrap_or(0);
        let expire = if group == Group::WorkingHours {
            "expireType=\"static\"".to_string()
        } else {
            format!("expireType=\"endpoint\" endpointId=\"{endpoint_id}\"")
        };
        if !present(new, group) {
            // Delete what the server has, under the instance it was published with.
            b.push_str(&format!(
                "<publication categoryName=\"{category}\" instance=\"{}\" container=\"{container}\" version=\"{version}\" {expire} expires=\"0\"/>",
                instance_of(group, &old_ids)
            ));
            continue;
        }
        b.push_str(&format!(
            "<publication categoryName=\"{category}\" instance=\"{}\" container=\"{container}\" version=\"{version}\" {expire}>",
            instance_of(group, &ids)
        ));
        match group {
            Group::WorkingHours | Group::FreeBusy if !full => {
                b.push_str(&format!("<calendarData xmlns=\"{NS_CALENDAR_DATA}\"/>"))
            }
            Group::WorkingHours => {
                let wh = checked_working_hours(new.working_hours.as_deref().unwrap_or(""))?;
                b.push_str(&format!(
                    "<calendarData xmlns=\"{NS_CALENDAR_DATA}\" mailboxID=\"{mailbox}\">{wh}</calendarData>"
                ));
            }
            Group::FreeBusy => {
                let (start, value) = new.free_busy.as_ref().ok_or(Error::UnsafeUri)?;
                let ok = value.len() <= MAX_FREE_BUSY
                    && value
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"+/=".contains(&c));
                if !ok {
                    return Err(Error::WrongDocument("freeBusy"));
                }
                let start = xs_datetime(*start).ok_or(Error::WrongDocument("startTime"))?;
                b.push_str(&format!(
                    "<calendarData xmlns=\"{NS_CALENDAR_DATA}\" mailboxID=\"{mailbox}\"><freeBusy startTime=\"{start}\" granularity=\"PT15M\" encodingVersion=\"1\">{value}</freeBusy></calendarData>"
                ));
            }
            Group::Meeting => {
                let m = new.meeting.as_ref().ok_or(Error::UnsafeUri)?;
                let start = xs_datetime(m.start).ok_or(Error::WrongDocument("startTime"))?;
                b.push_str(&format!(
                    "<state xmlns=\"{NS_STATE}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" manual=\"false\" uri=\"{mailbox}\" startTime=\"{start}\" xsi:type=\"calendarState\">\
<availability>6500</availability><activity token=\"in-a-meeting\" minAvailability=\"6500\" maxAvailability=\"8999\"/><endpointLocation/>"
                ));
                if let Some(s) = m.subject.as_deref().filter(|s| !s.is_empty()) {
                    b.push_str(&format!(
                        "<meetingSubject>{}</meetingSubject>",
                        text(s, MAX_TEXT)
                    ));
                }
                if let Some(l) = m.location.as_deref().filter(|l| !l.is_empty()) {
                    b.push_str(&format!(
                        "<meetingLocation>{}</meetingLocation>",
                        text(l, MAX_TEXT)
                    ));
                }
                b.push_str("</state>");
            }
            Group::OofState => b.push_str(&format!(
                "<state xmlns=\"{NS_STATE}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" manual=\"false\" uri=\"{mailbox}\" xsi:type=\"calendarState\">\
<activity token=\"out-of-office\"/></state>"
            )),
            Group::OofNote if !full => b.push_str(&format!("<note xmlns=\"{NS_NOTE}\"/>")),
            Group::OofNote => {
                let t = new.out_of_office.as_deref().unwrap_or("");
                b.push_str(&format!(
                    "<note xmlns=\"{NS_NOTE}\"><body type=\"OOF\" uri=\"\">{}</body></note>",
                    text(t, MAX_NOTE)
                ));
            }
        }
        b.push_str("</publication>");
    }
    b.push_str("</publications></publish>");
    Ok(b)
}
