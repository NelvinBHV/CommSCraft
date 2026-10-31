//! Call park (function 37; [MS-SIPAPP] 20250520, CK-01 … CK-07): park an active call at the call
//! park server (CPS) from provisioning, keep the parker's dialog with the CPS for its unpark
//! notifications, report the orbit. The parked party's call is replaced by the CPS (INVITE with
//! Replaces, handled as a call replacement); retrieving is an ordinary call to the orbit, which
//! the CPS answers with a REFER with Replaces (followed as a transfer, CK-05).

use commscraft_park as park;
use commscraft_sip_msg::dialog::{response_to, Dialog};
use commscraft_sip_msg::Message;

use crate::call::CallState;
use crate::group::{our_session_timer, SessionTimer};
use crate::{im, CallId, Engine, Event, Output, Purpose, Random};

/// Calls parked by us at the same time (design bound).
pub(crate) const MAX_PARKS: usize = 8;

/// One park request and, once accepted, the parker's dialog with the CPS.
#[derive(Debug, Clone)]
pub(crate) struct Park {
    pub id: u32,
    pub call: CallId,
    pub dialog: Option<Dialog>,
    pub orbit: Option<String>,
    pub timer: Option<SessionTimer>,
}

/// The From and To tags of a call's dialog as its first INVITE carried them (CK-01): the caller's
/// tag is the From tag.
fn call_tags(invite: &Message, d: &Dialog) -> Option<(String, String)> {
    let from_tag = invite.from().ok().flatten()?.param("tag")?.to_string();
    if from_tag == d.local_tag {
        Some((d.local_tag.clone(), d.remote_tag.clone()))
    } else if from_tag == d.remote_tag {
        Some((d.remote_tag.clone(), d.local_tag.clone()))
    } else {
        None
    }
}

impl<R: Random> Engine<R> {
    /// `ParkCall`: INVITE to the CPS with the park request (CK-01; [MS-SIPAPP] §3.1.5.1, p. 15).
    pub(crate) fn park_call(&mut self, call: CallId, now: u64, out: &mut Vec<Output>) {
        let notice = |out: &mut Vec<Output>, t: &str| {
            out.push(Output::Event(Event::Notice(t.to_string())));
        };
        let Some(cps) = self.call_park.clone() else {
            notice(
                out,
                "the server announced no call park service (callParkServerUri)",
            );
            return;
        };
        if self.parks.len() >= MAX_PARKS {
            notice(out, "too many parked calls");
            return;
        }
        let Some(c) = self.calls.get(&call) else {
            notice(out, "no such call");
            return;
        };
        let (Some(d), CallState::Active) = (c.dialog.as_ref(), &c.state) else {
            notice(out, "only an active call can be parked");
            return;
        };
        let Some((from_tag, to_tag)) = call_tags(&c.invite, d) else {
            notice(out, "the call's dialog is not known");
            return;
        };
        let dialog = park::CallDialog {
            call_id: d.call_id.clone(),
            from_tag,
            to_tag,
        };
        // The target is the GRUU of the other party (§3.1.5.1): its Contact in the call dialog.
        let target = d.remote_target.clone();
        self.next_park = self.next_park.wrapping_add(1).max(1);
        let id = self.next_park;
        let body = match park::park_request(id, &dialog, &target) {
            Ok(b) => b,
            Err(e) => {
                notice(out, &format!("call {call}: not parked ({e:?})"));
                return;
            }
        };
        let mut m = self.request("INVITE", &cps, &cps);
        m.push_header("Supported", "timer");
        m.push_header("Content-Type", park::CONTENT_TYPE);
        m.body = body.into_bytes();
        self.parks.push(Park {
            id,
            call,
            dialog: None,
            orbit: None,
            timer: None,
        });
        self.send_request(m, Purpose::Park(id), now, out);
    }

    /// 200 OK of the CPS (CK-02; §3.1.5.2, p. 16): ACK, keep the dialog, report the orbit.
    pub(crate) fn on_parked(
        &mut self,
        id: u32,
        request: &Message,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Ok(d) = Dialog::from_response(request, resp) else {
            self.parks.retain(|p| p.id != id);
            return;
        };
        let cseq = request.cseq().ok().flatten().map_or(1, |x| x.number);
        let mut ack = im::ack_for_2xx(&d, cseq);
        self.complete(&mut ack);
        self.send_plain(ack, now, out);
        let parked = std::str::from_utf8(&resp.body)
            .ok()
            .and_then(|b| park::parse_park_response(b).ok());
        let Some(parked) = parked else {
            // No usable answer: leave the CPS dialog, the call stays as it is.
            let mut d = d;
            let mut bye = d.request("BYE");
            self.complete(&mut bye);
            self.send_request(bye, Purpose::Ignore, now, out);
            self.parks.retain(|p| p.id != id);
            out.push(Output::Event(Event::Notice(
                "unreadable answer of the call park service".into(),
            )));
            return;
        };
        let timer = our_session_timer(resp, true, now);
        let Some(p) = self.parks.iter_mut().find(|p| p.id == id) else {
            return;
        };
        p.dialog = Some(d);
        p.orbit = Some(parked.orbit.clone());
        p.timer = timer;
        let call = p.call;
        out.push(Output::Event(Event::CallParked {
            call,
            orbit: parked.orbit,
        }));
    }

    /// An error response of the CPS (CK-03; §3.1.5.1, §3.1.5.3, p. 16).
    pub(crate) fn on_park_failed(&mut self, id: u32, code: u16, out: &mut Vec<Output>) {
        let Some(i) = self.parks.iter().position(|p| p.id == id) else {
            return;
        };
        let p = self.parks.remove(i);
        let why = match code {
            500 => "no free orbit",
            408 => "the other party could not be moved",
            415 | 488 => "the call park service refused the request",
            _ => "refused",
        };
        out.push(Output::Event(Event::Notice(format!(
            "call {}: not parked ({code}, {why})",
            p.call
        ))));
    }

    pub(crate) fn park_of(&self, req: &Message) -> Option<u32> {
        self.parks
            .iter()
            .find(|p| p.dialog.as_ref().is_some_and(|d| d.is_peer_request(req)))
            .map(|p| p.id)
    }

    /// Requests of the CPS on the parker's dialog: INFO with an unpark notification (CK-04), BYE
    /// (the parked call left its orbit), session timer refreshes.
    pub(crate) fn on_park_request(
        &mut self,
        id: u32,
        req: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let method = req.method().unwrap_or("").to_ascii_uppercase();
        match method.as_str() {
            "INFO" => {
                let ct = req.single("Content-Type").ok().flatten().map(|v| {
                    v.split(';')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_ascii_lowercase()
                });
                let notice = (ct.as_deref() == Some(park::CONTENT_TYPE))
                    .then(|| std::str::from_utf8(&req.body).ok())
                    .flatten()
                    .and_then(|b| park::parse_unpark(b).ok());
                let code = if notice.is_some() || req.body.is_empty() {
                    200
                } else {
                    415
                };
                let r = response_to(
                    req,
                    code,
                    if code == 200 {
                        "OK"
                    } else {
                        "Unsupported Media Type"
                    },
                );
                self.send_plain(r, now, out);
                if let Some(n) = notice {
                    let orbit = self
                        .parks
                        .iter()
                        .find(|p| p.id == id)
                        .and_then(|p| p.orbit.clone())
                        .unwrap_or_default();
                    out.push(Output::Event(Event::ParkNotification {
                        orbit,
                        reason: n.reason,
                        target: n.target,
                    }));
                }
            }
            "BYE" => {
                let r = response_to(req, 200, "OK");
                self.send_plain(r, now, out);
                self.parks.retain(|p| p.id != id);
            }
            "UPDATE" | "OPTIONS" => {
                let r = response_to(req, 200, "OK");
                self.send_plain(r, now, out);
            }
            _ => {
                let r = response_to(req, 405, "Method Not Allowed");
                self.send_plain(r, now, out);
            }
        }
    }

    /// Session timer of the parker's dialogs (RFC 4028; the CPS asks the parker to refresh).
    pub(crate) fn park_tick(&mut self, now: u64, out: &mut Vec<Output>) {
        let mut sends = Vec::new();
        for p in &mut self.parks {
            if let (Some(t), Some(d)) = (p.timer.as_mut(), p.dialog.as_mut()) {
                if now >= t.next {
                    t.next = now + t.interval / 2;
                    let mut m = d.request("UPDATE");
                    m.push_header("Supported", "timer");
                    sends.push(m);
                }
            }
        }
        for mut m in sends {
            self.complete(&mut m);
            self.send_request(m, Purpose::Ignore, now, out);
        }
    }

    /// Signing out: leave the CPS dialogs (only ends the notifications, §3.1.5.4).
    pub(crate) fn leave_parks(&mut self, now: u64, out: &mut Vec<Output>) {
        let dialogs: Vec<Dialog> = self.parks.drain(..).filter_map(|p| p.dialog).collect();
        for mut d in dialogs {
            let mut bye = d.request("BYE");
            self.complete(&mut bye);
            self.send_request(bye, Purpose::Ignore, now, out);
        }
    }
}
