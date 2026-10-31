//! Audio calls in the engine: INVITE with the audio offer (multipart per MS-SIPRE §3.15),
//! ringing, answer, final offer / re-INVITE, hold, CANCEL, BYE; incoming calls with
//! ringing, accept, reject and timeout.
//!
//! Evidence: files 01 (OCS-UC12), 06 (RE-65), 14 (SD-02 … SD-13, SD-41, SD-51), 13 (IC-07).
//! Sources: [MS-OCSPROT] 20250520 §2.5.12 (call flow, pp. 38–41); [MS-SIPRE] 20250520 §3.15
//! (multipart SDP, pp. 76–78), example §4.12.1 (pp. 89–90); [MS-SDPEXT] §3.1.5.2.2/§3.1.5.14
//! (no renegotiation of keys); RFC 3261 §9 (CANCEL), §13–§15 (INVITE, re-INVITE, BYE);
//! draft-ietf-mmusic-ice-19 §9.1.2.2 (final offer with remote-candidates).
//!
//! Scope: the media (sockets, TURN, ICE, SRTP, codec, devices) runs in the adapter; the engine
//! carries the SDP. Not offered: ICE-06 SDP (OCS 2007, RE-65 fallback), reliable provisional
//! responses (no `Supported: 100rel`, SD-51), early media.
//!
//! Security by design: every SDP goes through the bounded `sdp` parser and the encryption
//! policy (no silent plaintext); keys are never renegotiated in a re-INVITE (SD-13) – an update
//! must carry the key of the call; calls are bounded; incoming calls ring at most 60 s; nothing
//! is accepted without the user's command.

use super::*;
use commscraft_sdp as sdp;

pub type CallId = u32;
pub const MAX_CALLS: usize = 4;
/// An incoming call rings at most this long before 480 (design).
pub const RING_TIMEOUT: u64 = 60;
const BOUNDARY: &str = "CommSCraft_audio_offer";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallState {
    /// Our INVITE is out; `cancelling` once the user hung up before the answer.
    Inviting {
        cancelling: bool,
    },
    /// Incoming INVITE, 180 sent, waiting for the user until `deadline`.
    Ringing {
        deadline: u64,
    },
    Active,
}

#[derive(Debug, Clone)]
pub(crate) struct Call {
    pub peer: String,
    pub state: CallState,
    /// Our INVITE (outgoing) or the peer's INVITE (incoming, until answered).
    pub invite: Message,
    pub dialog: Option<Dialog>,
    /// Our SDP parameters; the key stays the same for the whole call (SD-13).
    pub local: Option<sdp::Local>,
    pub policy: sdp::Encryption,
    /// The SDP of the incoming offer (the ICE-19 part, RE-65).
    pub offer: Vec<u8>,
    pub ringing_reported: bool,
}

/// The audio SDP of a message: plain `application/sdp`, or the last `application/sdp` part of a
/// `multipart/alternative` / `multipart/mixed` body (MS-SIPRE §3.15.5.1: ICE-19 is last). Only
/// SDPs with an `m=audio` line count.
pub(crate) fn audio_sdp(m: &Message) -> Option<Vec<u8>> {
    let ct = m.single("Content-Type").ok().flatten()?.to_string();
    let lower = ct.to_ascii_lowercase();
    let has_audio = |b: &[u8]| {
        sdp::parse(b)
            .ok()
            .is_some_and(|s| s.media.iter().any(|x| x.kind == "audio"))
    };
    if lower.starts_with("application/sdp") {
        return has_audio(&m.body).then(|| m.body.clone());
    }
    if !lower.starts_with("multipart/") {
        return None;
    }
    let body = std::str::from_utf8(&m.body).ok()?;
    let parts = pres::split_multipart(body, &pres::boundary(&ct)?).ok()?;
    let mut found = None;
    for p in parts.iter().take(8) {
        let pct = p.content_type.to_ascii_lowercase();
        if pct.starts_with("application/sdp") && has_audio(p.body.as_bytes()) {
            found = Some(p.body.as_bytes().to_vec());
        } else if pct.starts_with("multipart/alternative") {
            // 3-level structure: multipart/mixed ⊃ multipart/alternative ⊃ SDPs.
            let inner = pres::boundary(&p.content_type)
                .and_then(|b| pres::split_multipart(&p.body, &b).ok())
                .unwrap_or_default();
            for q in inner.iter().take(8) {
                if q.content_type
                    .to_ascii_lowercase()
                    .starts_with("application/sdp")
                    && has_audio(q.body.as_bytes())
                {
                    found = Some(q.body.as_bytes().to_vec());
                }
            }
        }
    }
    found
}

impl<R: Random> Engine<R> {
    fn call_event(out: &mut Vec<Output>, e: Event) {
        out.push(Output::Event(e));
    }

    /// `Call`: INVITE with our offer as a 2-level multipart/alternative whose last (and here
    /// only) part is the ICE-19 SDP (MS-SIPRE §3.15.4.1).
    pub(crate) fn start_call(
        &mut self,
        to: &str,
        local: sdp::Local,
        policy: sdp::Encryption,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Ok(peer) = pres::checked_sip_uri(to) else {
            out.push(Output::Event(Event::Notice(
                "not a usable SIP address".into(),
            )));
            return;
        };
        if self.calls.len() >= MAX_CALLS {
            out.push(Output::Event(Event::Notice("too many calls".into())));
            return;
        }
        if policy != sdp::Encryption::Off && local.key.is_none() {
            out.push(Output::Event(Event::Notice(
                "no media key for an encrypted call".into(),
            )));
            return;
        }
        let offer = sdp::offer(&local, policy);
        let mut m = self.request("INVITE", &peer, &peer);
        m.push_header(
            "Content-Type",
            &format!("multipart/alternative;boundary=\"{BOUNDARY}\""),
        );
        m.body = format!(
            "--{BOUNDARY}\r\nContent-Type: application/sdp\r\nContent-Transfer-Encoding: 7bit\r\nContent-Disposition: session; handling=optional\r\n\r\n{offer}\r\n--{BOUNDARY}--\r\n"
        )
        .into_bytes();
        let id = self.next_call;
        self.next_call = self.next_call.wrapping_add(1).max(1);
        self.calls.insert(
            id,
            Call {
                peer: peer.clone(),
                state: CallState::Inviting { cancelling: false },
                invite: m.clone(),
                dialog: None,
                local: Some(local),
                policy,
                offer: Vec::new(),
                ringing_reported: false,
            },
        );
        self.send_request(m, Purpose::CallInvite(id), now, out);
        // The callee may ring for minutes (RFC 3261 §13.3.1.1, timer C ≥ 3 min).
        if let Some(p) = self
            .pending
            .values_mut()
            .find(|p| p.purpose == Purpose::CallInvite(id))
        {
            p.deadline = now + 180;
            // The INVITE as signed (for CANCEL with the same Via).
            if let Some(c) = self.calls.get_mut(&id) {
                c.invite = p.request.clone();
            }
        }
    }

    /// A provisional response to our INVITE: report ringing once.
    pub(crate) fn on_call_provisional(
        &mut self,
        purpose: &Purpose,
        code: u16,
        out: &mut Vec<Output>,
    ) {
        let Purpose::CallInvite(id) = *purpose else {
            return;
        };
        if !(180..=183).contains(&code) {
            return;
        }
        if let Some(c) = self.calls.get_mut(&id) {
            if !c.ringing_reported {
                c.ringing_reported = true;
                Self::call_event(out, Event::CallRinging { call: id });
            }
        }
    }

    /// 2xx to our INVITE: dialog, ACK, answer evaluated with our key and policy.
    pub(crate) fn on_call_answered(
        &mut self,
        id: CallId,
        request: &Message,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Ok(d) = Dialog::from_response(request, resp) else {
            self.calls.remove(&id);
            Self::call_event(
                out,
                Event::CallEnded {
                    call: id,
                    status: 500,
                },
            );
            return;
        };
        let cseq = request.cseq().ok().flatten().map_or(1, |x| x.number);
        let mut ack = im::ack_for_2xx(&d, cseq);
        self.complete(&mut ack);
        self.send_plain(ack, now, out);
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        if c.state == (CallState::Inviting { cancelling: true }) {
            // The answer crossed our CANCEL: end with BYE.
            let mut d2 = d;
            let mut bye = d2.request("BYE");
            self.complete(&mut bye);
            self.send_request(bye, Purpose::Ignore, now, out);
            self.calls.remove(&id);
            Self::call_event(
                out,
                Event::CallEnded {
                    call: id,
                    status: 487,
                },
            );
            return;
        }
        let key = c.local.as_ref().and_then(|l| l.key.clone());
        let negotiated = audio_sdp(resp)
            .ok_or(sdp::Error::NoAudio)
            .and_then(|s| sdp::answer_to_offer(&s, key.as_ref(), c.policy));
        match negotiated {
            Ok(n) => {
                c.dialog = Some(d);
                c.state = CallState::Active;
                Self::call_event(
                    out,
                    Event::CallAnswered {
                        call: id,
                        media: Box::new(n),
                    },
                );
            }
            Err(e) => {
                // An answer we cannot use (e.g. plaintext against our policy): hang up.
                let mut d2 = d;
                let mut bye = d2.request("BYE");
                self.complete(&mut bye);
                self.send_request(bye, Purpose::Ignore, now, out);
                self.calls.remove(&id);
                out.push(Output::Event(Event::Notice(format!(
                    "call {id}: unusable answer ({e:?})"
                ))));
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 488,
                    },
                );
            }
        }
    }

    pub(crate) fn on_call_failed(&mut self, id: CallId, code: u16, out: &mut Vec<Output>) {
        if self.calls.remove(&id).is_some() {
            Self::call_event(
                out,
                Event::CallEnded {
                    call: id,
                    status: code,
                },
            );
        }
    }

    /// `UpdateCall`: re-INVITE with a new local description (final offer with the selected
    /// candidates, IC-07; hold/resume, SD-06). The key of the call is kept (SD-13).
    pub(crate) fn update_call(
        &mut self,
        id: CallId,
        mut local: sdp::Local,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        let (Some(mut d), CallState::Active) = (c.dialog.take(), c.state.clone()) else {
            out.push(Output::Event(Event::Notice(
                "the call is not active".into(),
            )));
            return;
        };
        local.key = c.local.as_ref().and_then(|l| l.key.clone());
        let body = sdp::offer(&local, c.policy);
        c.local = Some(local);
        let mut m = d.request("INVITE");
        c.dialog = Some(d);
        self.complete(&mut m);
        m.push_header("Content-Type", "application/sdp");
        m.body = body.into_bytes();
        self.send_request(m, Purpose::CallReinvite(id), now, out);
    }

    pub(crate) fn on_call_reinvite_answered(
        &mut self,
        id: CallId,
        request: &Message,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(c) = self.calls.get(&id) else { return };
        let Some(d) = c.dialog.clone() else { return };
        let key = c.local.as_ref().and_then(|l| l.key.clone());
        let policy = c.policy;
        let cseq = request.cseq().ok().flatten().map_or(1, |x| x.number);
        let mut ack = im::ack_for_2xx(&d, cseq);
        self.complete(&mut ack);
        self.send_plain(ack, now, out);
        match audio_sdp(resp)
            .ok_or(sdp::Error::NoAudio)
            .and_then(|s| sdp::answer_to_offer(&s, key.as_ref(), policy))
        {
            Ok(n) => Self::call_event(
                out,
                Event::CallUpdated {
                    call: id,
                    media: Box::new(n),
                },
            ),
            Err(e) => out.push(Output::Event(Event::Notice(format!(
                "call {id}: unusable re-INVITE answer ({e:?})"
            )))),
        }
    }

    /// `Hangup`: BYE, CANCEL while inviting, 603 while ringing (RFC 3261 §9.1, §15.1).
    pub(crate) fn hangup(&mut self, id: CallId, now: u64, out: &mut Vec<Output>) {
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        match c.state.clone() {
            CallState::Active => {
                if let Some(mut d) = c.dialog.take() {
                    let mut bye = d.request("BYE");
                    self.complete(&mut bye);
                    self.send_request(bye, Purpose::Ignore, now, out);
                }
                self.calls.remove(&id);
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 0,
                    },
                );
            }
            CallState::Inviting { cancelling: false } => {
                c.state = CallState::Inviting { cancelling: true };
                let cancel = cancel_for(&c.invite);
                // CANCEL has its own transaction; its answer is not tracked (the 487 to the
                // INVITE ends the call).
                self.send_plain(cancel, now, out);
            }
            CallState::Inviting { cancelling: true } => {}
            CallState::Ringing { .. } => {
                let invite = c.invite.clone();
                self.calls.remove(&id);
                self.answer_call_request(&invite, 603, "Decline", now, out);
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 603,
                    },
                );
            }
        }
    }

    fn answer_call_request(
        &mut self,
        invite: &Message,
        code: u16,
        reason: &str,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let r = response_to(invite, code, reason);
        self.send_plain(r, now, out);
    }

    /// An incoming INVITE with an audio offer: 180 Ringing, report it, wait for the user.
    pub(crate) fn on_incoming_call(
        &mut self,
        req: &Message,
        offer: Vec<u8>,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        if self.calls.len() >= MAX_CALLS {
            let r = response_to(req, 486, "Busy Here");
            self.send_plain(r, now, out);
            return;
        }
        let tag = self.hex(5);
        let Ok(dialog) = Dialog::from_request(req, &tag) else {
            let r = response_to(req, 400, "Bad Request");
            self.send_plain(r, now, out);
            return;
        };
        let mut invite = req.clone();
        // Our To tag goes into every response of this transaction.
        for h in invite
            .headers
            .iter_mut()
            .filter(|h| h.name.eq_ignore_ascii_case("To"))
        {
            h.value = format!("{};tag={tag}", h.value);
        }
        let mut ringing = response_to(&invite, 180, "Ringing");
        ringing.push_header("Contact", &format!("<{}>", self.gruu));
        self.send_plain(ringing, now, out);
        let id = self.next_call;
        self.next_call = self.next_call.wrapping_add(1).max(1);
        let from = dialog.remote_uri.clone();
        self.calls.insert(
            id,
            Call {
                peer: from.clone(),
                state: CallState::Ringing {
                    deadline: now + RING_TIMEOUT,
                },
                invite,
                dialog: Some(dialog),
                local: None,
                policy: sdp::Encryption::Required,
                offer,
                ringing_reported: true,
            },
        );
        Self::call_event(out, Event::IncomingCall { call: id, from });
    }

    /// `AcceptCall`: answer the offer with our parameters (200 OK with SDP).
    pub(crate) fn accept_call(
        &mut self,
        id: CallId,
        local: sdp::Local,
        policy: sdp::Encryption,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(c) = self.calls.get(&id) else { return };
        if !matches!(c.state, CallState::Ringing { .. }) {
            return;
        }
        let invite = c.invite.clone();
        let fresh = local
            .key
            .clone()
            .unwrap_or_else(|| sdp::Key::new(0, [0; 30]));
        let answered = if policy != sdp::Encryption::Off && local.key.is_none() {
            Err(sdp::Error::Crypto("no media key"))
        } else {
            sdp::answer(&c.offer, &local, fresh, policy)
        };
        match answered {
            Ok((body, n)) => {
                let mut ok = response_to(&invite, 200, "OK");
                for rr in invite.header_values("Record-Route") {
                    ok.push_header("Record-Route", rr);
                }
                ok.push_header("Contact", &format!("<{}>", self.gruu));
                ok.push_header("Content-Type", "application/sdp");
                ok.body = body.into_bytes();
                self.send_plain(ok, now, out);
                let mut kept = local;
                // Keep the key under the tag the peer offered (it was answered with it).
                if let (Some(k), Some(tag)) =
                    (kept.key.as_mut(), n.remote_key.as_ref().map(|r| r.tag))
                {
                    k.tag = tag;
                }
                if let Some(c) = self.calls.get_mut(&id) {
                    c.state = CallState::Active;
                    c.local = Some(kept);
                    c.policy = policy;
                }
                Self::call_event(
                    out,
                    Event::CallAnswered {
                        call: id,
                        media: Box::new(n),
                    },
                );
            }
            Err(e) => {
                self.calls.remove(&id);
                self.answer_call_request(&invite, 488, "Not Acceptable Here", now, out);
                out.push(Output::Event(Event::Notice(format!(
                    "call {id}: offer not acceptable ({e:?})"
                ))));
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 488,
                    },
                );
            }
        }
    }

    /// Requests of the peer in a call dialog: BYE, re-INVITE (final offer, hold), ACK.
    pub(crate) fn on_call_request(
        &mut self,
        id: CallId,
        req: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let method = req.method().unwrap_or("").to_ascii_uppercase();
        match method.as_str() {
            "ACK" => {}
            "BYE" => {
                let r = response_to(req, 200, "OK");
                self.send_plain(r, now, out);
                self.calls.remove(&id);
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 0,
                    },
                );
            }
            "INVITE" => {
                let Some(c) = self.calls.get(&id) else { return };
                let (Some(local), Some(offer)) = (c.local.clone(), audio_sdp(req)) else {
                    let r = response_to(req, 488, "Not Acceptable Here");
                    self.send_plain(r, now, out);
                    return;
                };
                let key = local
                    .key
                    .clone()
                    .unwrap_or_else(|| sdp::Key::new(0, [0; 30]));
                match sdp::answer(&offer, &local, key, c.policy) {
                    // SD-13: the peer must keep its key as well.
                    Ok((body, n)) if c.policy == sdp::Encryption::Off || n.remote_key.is_some() => {
                        let mut ok = response_to(req, 200, "OK");
                        ok.push_header("Contact", &format!("<{}>", self.gruu));
                        ok.push_header("Content-Type", "application/sdp");
                        ok.body = body.into_bytes();
                        self.send_plain(ok, now, out);
                        Self::call_event(
                            out,
                            Event::CallUpdated {
                                call: id,
                                media: Box::new(n),
                            },
                        );
                    }
                    _ => {
                        let r = response_to(req, 488, "Not Acceptable Here");
                        self.send_plain(r, now, out);
                    }
                }
            }
            "UPDATE" | "INFO" | "OPTIONS" => {
                let r = response_to(req, 200, "OK");
                self.send_plain(r, now, out);
            }
            _ => {
                let r = response_to(req, 405, "Method Not Allowed");
                self.send_plain(r, now, out);
            }
        }
    }

    /// CANCEL of a ringing incoming call (RFC 3261 §9.2): 200 to the CANCEL, 487 to the INVITE.
    pub(crate) fn on_cancel(&mut self, req: &Message, now: u64, out: &mut Vec<Output>) {
        let call_id = req.call_id().ok().flatten().map(str::to_string);
        let from_tag = req
            .from()
            .ok()
            .flatten()
            .and_then(|f| f.param("tag").map(str::to_string));
        let found = self.calls.iter().find(|(_, c)| {
            matches!(c.state, CallState::Ringing { .. })
                && c.invite.call_id().ok().flatten().map(str::to_string) == call_id
                && c.invite
                    .from()
                    .ok()
                    .flatten()
                    .and_then(|f| f.param("tag").map(str::to_string))
                    == from_tag
        });
        let Some((&id, c)) = found else {
            let r = response_to(req, 481, "Call/Transaction Does Not Exist");
            self.send_plain(r, now, out);
            return;
        };
        let invite = c.invite.clone();
        let ok = response_to(req, 200, "OK");
        self.send_plain(ok, now, out);
        self.calls.remove(&id);
        self.answer_call_request(&invite, 487, "Request Terminated", now, out);
        Self::call_event(
            out,
            Event::CallEnded {
                call: id,
                status: 487,
            },
        );
    }

    /// Ringing timeout of incoming calls.
    pub(crate) fn call_tick(&mut self, now: u64, out: &mut Vec<Output>) {
        let expired: Vec<CallId> = self
            .calls
            .iter()
            .filter(|(_, c)| matches!(c.state, CallState::Ringing { deadline } if now >= deadline))
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            if let Some(c) = self.calls.remove(&id) {
                self.answer_call_request(&c.invite, 480, "Temporarily Unavailable", now, out);
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 480,
                    },
                );
            }
        }
    }

    pub(crate) fn call_of(&self, req: &Message) -> Option<CallId> {
        self.calls
            .iter()
            .find(|(_, c)| {
                matches!(c.state, CallState::Active)
                    && c.dialog.as_ref().is_some_and(|d| d.is_peer_request(req))
            })
            .map(|(id, _)| *id)
    }

    /// The peer of a call (for the user interface).
    pub fn call_peer(&self, id: CallId) -> Option<&str> {
        self.calls.get(&id).map(|c| c.peer.as_str())
    }

    /// Ends all calls (sign-out).
    pub(crate) fn end_all_calls(&mut self, now: u64, out: &mut Vec<Output>) {
        let ids: Vec<CallId> = self.calls.keys().copied().collect();
        for id in ids {
            self.hangup(id, now, out);
        }
    }
}

/// CANCEL for an INVITE (RFC 3261 §9.1): same Request-URI, Call-ID, To, From, CSeq number, top
/// Via and Route set.
fn cancel_for(invite: &Message) -> Message {
    let uri = match &invite.start {
        StartLine::Request { uri, .. } => uri.clone(),
        StartLine::Response { .. } => String::new(),
    };
    let mut m = Message::request("CANCEL", &uri);
    if let Some(v) = invite.header_values("Via").next() {
        m.push_header("Via", v);
    }
    for r in invite.header_values("Route") {
        m.push_header("Route", r);
    }
    m.push_header("Max-Forwards", "70");
    for name in ["From", "To", "Call-ID"] {
        if let Some(v) = invite.header_values(name).next() {
            m.push_header(name, v);
        }
    }
    let n = invite.cseq().ok().flatten().map_or(1, |c| c.number);
    m.push_header("CSeq", &format!("{n} CANCEL"));
    m
}
