//! Quality reports and call rating (function 45; [MS-QoE] 20250520, QE-01 … QE-06): when a call
//! becomes active the engine keeps what the report needs about its dialog; after the call the UI
//! hands in the measured network figures (`ReportQuality`) and, if the user wants, a rating
//! (`RateCall`); both go in a SERVICE to the QoE URI from provisioning (`qosUri`). A 202 is the
//! normal answer (accepted, not yet processed, QE-01).

use commscraft_qoe as qoe;
use commscraft_sip_msg::Message;

use crate::{CallId, Engine, Event, Output, Purpose, Random};

/// Calls whose dialog data are kept for reports and ratings (design bound; the oldest goes).
pub(crate) const MAX_KEPT: usize = 16;

/// What the report needs about a call's dialog (DialogInfo, §2.2.1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Kept {
    pub call_id: String,
    pub from_tag: String,
    pub to_tag: String,
    pub from_uri: String,
    pub to_uri: String,
    pub caller: bool,
    pub remote_contact: String,
    pub remote_user_agent: String,
    pub start: u64,
}

impl<R: Random> Engine<R> {
    /// The call `id` is active now: keep its dialog data. `peer`: the peer's message that carried
    /// its user agent (our INVITE's 200 OK, or the peer's INVITE).
    pub(crate) fn keep_for_quality(&mut self, id: CallId, peer: &Message, now: u64) {
        let Some(c) = self.calls.get(&id) else { return };
        let Some(d) = c.dialog.as_ref() else { return };
        let invite = &c.invite;
        let (Ok(Some(from)), Ok(Some(to))) = (invite.from(), invite.to()) else {
            return;
        };
        let Some(from_tag) = from.param("tag").map(str::to_string) else {
            return;
        };
        let caller = from_tag == d.local_tag;
        let to_tag = if caller {
            d.remote_tag.clone()
        } else {
            d.local_tag.clone()
        };
        let remote_user_agent = peer
            .single("User-Agent")
            .ok()
            .flatten()
            .or_else(|| peer.single("Server").ok().flatten())
            .unwrap_or("")
            .to_string();
        let kept = Kept {
            call_id: d.call_id.clone(),
            from_tag,
            to_tag,
            from_uri: from.uri.clone(),
            to_uri: to.uri.clone(),
            caller,
            remote_contact: d.remote_target.clone(),
            remote_user_agent,
            start: now,
        };
        self.quality_calls.retain(|(c, _)| *c != id);
        if self.quality_calls.len() >= MAX_KEPT {
            self.quality_calls.remove(0);
        }
        self.quality_calls.push((id, kept));
    }

    fn kept(&mut self, id: CallId, out: &mut Vec<Output>) -> Option<(String, Kept)> {
        let Some(uri) = self.qos_uri.clone() else {
            out.push(Output::Event(Event::Notice(
                "the server announced no quality service (qosUri)".into(),
            )));
            return None;
        };
        match self.quality_calls.iter().find(|(c, _)| *c == id) {
            Some((_, k)) => Some((uri, k.clone())),
            None => {
                out.push(Output::Event(Event::Notice(format!(
                    "call {id}: no call data for a quality report"
                ))));
                None
            }
        }
    }

    fn send_quality(&mut self, uri: &str, ct: &str, body: String, now: u64, out: &mut Vec<Output>) {
        let mut m = self.request("SERVICE", uri, uri);
        m.push_header("Content-Type", ct);
        m.body = body.into_bytes();
        self.send_request(m, Purpose::Quality, now, out);
    }

    /// `ReportQuality` (QE-01, QE-02): the VQReportEvent with LocalPAI = our address (QE-04).
    pub(crate) fn report_quality(
        &mut self,
        id: CallId,
        audio: qoe::AudioLine,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some((uri, k)) = self.kept(id, out) else {
            return;
        };
        let report = qoe::Report {
            endpoint: self.cfg.machine_name.clone(),
            dialog: qoe::DialogInfo {
                call_id: k.call_id,
                from_tag: k.from_tag,
                to_tag: k.to_tag,
                start: k.start,
                end: now,
                from_uri: k.from_uri,
                to_uri: k.to_uri,
                caller: k.caller,
                local_contact: self.gruu.clone(),
                remote_contact: k.remote_contact,
                local_user_agent: self.cfg.user_agent.clone(),
                remote_user_agent: k.remote_user_agent,
                local_pai: self.cfg.aor.clone(),
            },
            // We made the offer of the media session when we placed the call.
            audio: qoe::AudioLine {
                offerer: k.caller,
                ..audio
            },
        };
        match qoe::report(&report) {
            Ok(body) => self.send_quality(&uri, qoe::REPORT_TYPE, body, now, out),
            Err(e) => out.push(Output::Event(Event::Notice(format!(
                "call {id}: no quality report ({e:?})"
            )))),
        }
    }

    /// `RateCall` (QE-03): rating 1–5 and an optional text.
    pub(crate) fn rate_call(
        &mut self,
        id: CallId,
        stars: u8,
        feedback: Option<&str>,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some((uri, k)) = self.kept(id, out) else {
            return;
        };
        let aor = self.cfg.aor.clone();
        match qoe::rating(&k.call_id, &k.from_tag, &k.to_tag, &aor, stars, feedback) {
            Ok(body) => self.send_quality(&uri, qoe::RATING_TYPE, body, now, out),
            Err(e) => out.push(Output::Event(Event::Notice(format!(
                "call {id}: rating not sent ({e:?})"
            )))),
        }
    }
}
