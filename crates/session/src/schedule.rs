//! Scheduling meetings at the focus factory (function 40; [MS-CONFPRO] 20250520, PV-06, PV-09,
//! PV-13, PV-14): create a meeting with subject, lifetime and admission policy, list, show and
//! cancel the organizer's meetings. Joining a scheduled meeting is the ordinary join by focus URI
//! (CB-11). Every request is a SERVICE to the focus factory; every answer is matched by its
//! transaction, parsed with the bounded reader and reported as an event.

use commscraft_conf as conf;
use conf::schedule::{self as sch, MeetingSpec};

use crate::{Engine, Event, Output, Purpose, Random};

/// The focus factory request a pending SERVICE belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScheduleOp {
    Add(Box<MeetingSpec>),
    List,
    Get,
    /// getConference before a change: the answer's version goes into modifyConference (PV-09).
    GetForChange(Box<MeetingSpec>),
    Change(Box<MeetingSpec>),
    Delete(String),
}

impl<R: Random> Engine<R> {
    fn factory(&mut self, out: &mut Vec<Output>) -> Option<String> {
        let f = self.focus_factory.clone();
        if f.is_none() {
            out.push(Output::Event(Event::Notice(
                "the server announced no conferencing service (focusFactoryUri)".into(),
            )));
        }
        f
    }

    fn send_factory(
        &mut self,
        factory: &str,
        body: Result<String, conf::Error>,
        op: ScheduleOp,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        match body {
            Ok(b) => {
                let mut m = self.request("SERVICE", factory, factory);
                m.push_header("Content-Type", conf::CONTENT_TYPE);
                m.body = b.into_bytes();
                self.send_request(m, Purpose::Schedule(op), now, out);
            }
            Err(e) => out.push(Output::Event(Event::Notice(format!(
                "meeting request not sent ({e:?})"
            )))),
        }
    }

    /// `ScheduleMeeting`: a new conference ID (128 random bits, PV-04) and addConference with
    /// the IM and A/V MCU; the lifetime is bounded by [`sch::MAX_LIFETIME`] (design).
    pub(crate) fn schedule_meeting(
        &mut self,
        subject: &str,
        minutes: u64,
        admission: sch::Admission,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(factory) = self.factory(out) else {
            return;
        };
        let lifetime = minutes.saturating_mul(60);
        if lifetime == 0 || lifetime > sch::MAX_LIFETIME {
            out.push(Output::Event(Event::Notice(
                "a meeting lasts between one minute and one year".into(),
            )));
            return;
        }
        let mut random = [0u8; 16];
        self.reg.fill_random(&mut random);
        let spec = MeetingSpec {
            conference_id: conf::conference_id(&random),
            subject: subject.trim().to_string(),
            expiry: now.saturating_add(lifetime),
            admission,
        };
        let request_id = self.next_c3p();
        let body = sch::schedule_conference(request_id, &self.cfg.aor, &factory, &spec);
        self.send_factory(&factory, body, ScheduleOp::Add(Box::new(spec)), now, out);
    }

    /// `ListMeetings`: getConferences.
    pub(crate) fn list_meetings(&mut self, now: u64, out: &mut Vec<Output>) {
        let Some(factory) = self.factory(out) else {
            return;
        };
        let request_id = self.next_c3p();
        let body = sch::get_conferences(request_id, &self.cfg.aor, &factory);
        self.send_factory(&factory, body, ScheduleOp::List, now, out);
    }

    /// `ShowMeeting`: getConference.
    pub(crate) fn show_meeting(&mut self, conference_id: &str, now: u64, out: &mut Vec<Output>) {
        let Some(factory) = self.factory(out) else {
            return;
        };
        let request_id = self.next_c3p();
        let body = sch::get_conference(request_id, &self.cfg.aor, &factory, conference_id);
        self.send_factory(&factory, body, ScheduleOp::Get, now, out);
    }

    /// `ChangeMeeting` (PV-09: "first getConference, compare version, then modifyConference"):
    /// new subject, lifetime from now and admission policy for an existing meeting.
    pub(crate) fn change_meeting(
        &mut self,
        conference_id: &str,
        subject: &str,
        minutes: u64,
        admission: sch::Admission,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(factory) = self.factory(out) else {
            return;
        };
        let lifetime = minutes.saturating_mul(60);
        if lifetime == 0 || lifetime > sch::MAX_LIFETIME {
            out.push(Output::Event(Event::Notice(
                "a meeting lasts between one minute and one year".into(),
            )));
            return;
        }
        let spec = MeetingSpec {
            conference_id: conference_id.to_string(),
            subject: subject.trim().to_string(),
            expiry: now.saturating_add(lifetime),
            admission,
        };
        let request_id = self.next_c3p();
        let body = sch::get_conference(request_id, &self.cfg.aor, &factory, conference_id);
        self.send_factory(
            &factory,
            body,
            ScheduleOp::GetForChange(Box::new(spec)),
            now,
            out,
        );
    }

    /// `CancelMeeting`: deleteConference.
    pub(crate) fn cancel_meeting(&mut self, conference_id: &str, now: u64, out: &mut Vec<Output>) {
        let Some(factory) = self.factory(out) else {
            return;
        };
        let request_id = self.next_c3p();
        let body = sch::delete_conference(request_id, &self.cfg.aor, &factory, conference_id);
        self.send_factory(
            &factory,
            body,
            ScheduleOp::Delete(conference_id.to_string()),
            now,
            out,
        );
    }

    /// 200 OK of the focus factory (PV-07, PV-11: a 200 carries the C3P answer).
    pub(crate) fn on_schedule_answer(
        &mut self,
        op: ScheduleOp,
        body: &[u8],
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let parsed = std::str::from_utf8(body)
            .ok()
            .and_then(|b| sch::parse_meetings(b).ok());
        let Some((r, meetings)) = parsed else {
            out.push(Output::Event(Event::Notice(
                "unreadable answer of the conferencing service".into(),
            )));
            return;
        };
        if r.code != conf::Code::Success {
            out.push(Output::Event(Event::Notice(format!(
                "meeting request refused ({})",
                r.reason.unwrap_or_else(|| "failure".into())
            ))));
            return;
        }
        match op {
            ScheduleOp::Add(spec) => {
                // PV-07: the conference URI is conference-info/@entity (K-PV-1).
                let meeting = sch::Meeting {
                    focus: r.conference.filter(|c| conf::is_focus_uri(c)),
                    conference_id: spec.conference_id.clone(),
                    subject: Some(spec.subject.clone()),
                    expiry: sch::xs_datetime(spec.expiry),
                    admission: Some(spec.admission),
                    join_url: None,
                    version: Some(1),
                };
                out.push(Output::Event(Event::MeetingScheduled(Box::new(meeting))));
                // The details may name the join URL (join-url, §2.2.1.2).
                self.show_meeting(&spec.conference_id, now, out);
            }
            ScheduleOp::List => out.push(Output::Event(Event::Meetings(meetings))),
            ScheduleOp::Get => {
                if let Some(m) = meetings.into_iter().next() {
                    out.push(Output::Event(Event::MeetingDetails(Box::new(m))));
                }
            }
            ScheduleOp::GetForChange(spec) => {
                let version = meetings
                    .first()
                    .filter(|m| m.conference_id == spec.conference_id)
                    .and_then(|m| m.version);
                let (Some(version), Some(factory)) = (version, self.focus_factory.clone()) else {
                    out.push(Output::Event(Event::Notice(
                        "the meeting's version is not known; not changed".into(),
                    )));
                    return;
                };
                let request_id = self.next_c3p();
                let body =
                    sch::modify_conference(request_id, &self.cfg.aor, &factory, version, &spec);
                self.send_factory(&factory, body, ScheduleOp::Change(spec), now, out);
            }
            ScheduleOp::Change(spec) => {
                // PV-07: a modified conference gets the next version; read the details again.
                self.show_meeting(&spec.conference_id, now, out);
            }
            ScheduleOp::Delete(conference_id) => {
                out.push(Output::Event(Event::MeetingCancelled { conference_id }))
            }
        }
    }
}
