//! Group IM in the engine: create an ad-hoc conference, join it through the focus, follow the
//! roster, join the IM MCU, invite participants by dial-out, delivery notifications, leave.
//!
//! Evidence: files 18, 19, 21 (CB-10 … CB-37, PV-01 … PV-11, CI-01 … CI-12). Flow:
//! [MS-CONFBAS] 20250520 §1.3.2 (pp. 17–19); [MS-CONFIM] 20250520 §4.1.1 (join, pp. 28–40),
//! §4.1.2 (dial-out, pp. 41–51), §4.2 (MESSAGE, 202 and IMDN, pp. 52–58), §4.3.2 (leave,
//! pp. 67–70); session refresh: [MS-CONFBAS] §4.2.2 (pp. 124–125), RFC 4028 §10.
//!
//! Security by design: only checked SIP URIs reach requests and XML; a conference is joined only
//! on the user's command; the roster is bounded (`conf`); requests in the focus dialog are
//! answered (INFO, UPDATE, BYE) or refused; delivery notifications are matched to our own
//! message IDs and time out after 10 s (CI-05).

use super::*;

/// RFC 4028 session timer of a dialog that we refresh: interval and the next refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionTimer {
    pub interval: u64,
    pub next: u64,
}

/// Session-Expires of a message: we refresh if the refresher is our side (`uac` when we sent the
/// INVITE, `uas` when we received it; RFC 4028 §7.4, §9). Interval bounded to 90 s … 1 day.
pub(crate) fn our_session_timer(m: &Message, we_are_uac: bool, now: u64) -> Option<SessionTimer> {
    let v = m.single("Session-Expires").ok().flatten()?;
    let mut parts = v.split(';');
    let interval: u64 = parts.next()?.trim().parse().ok()?;
    let refresher = parts
        .filter_map(|p| p.trim().split_once('='))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("refresher"))
        .map(|(_, v)| v.trim().to_ascii_lowercase());
    let ours = match refresher.as_deref() {
        Some("uac") => we_are_uac,
        Some("uas") => !we_are_uac,
        _ => false,
    };
    ours.then(|| {
        let interval = interval.clamp(90, 86_400);
        SessionTimer {
            interval,
            next: now + interval / 2,
        }
    })
}

/// State of a group conversation.
#[derive(Debug, Clone, Default)]
pub(crate) struct Group {
    /// Conference URI of the focus (known after addConference or from the invitation).
    pub focus: Option<String>,
    pub organizer: bool,
    /// The focus dialog was created by the IM MCU calling us (dial-out to us, CI-01): no focus
    /// dialog, no roster.
    pub dialed_out: bool,
    pub focus_dialog: Option<Dialog>,
    pub focus_timer: Option<SessionTimer>,
    pub roster_dialog: Option<Dialog>,
    pub roster_refresh_at: Option<u64>,
    pub roster: conf::Roster,
    /// The INVITE to the IM MCU was sent.
    pub chat_requested: bool,
    /// Participants to invite once the IM MCU session is up.
    pub invitees: Vec<String>,
    /// Message-Ids answered with 202 and waiting for their IMDN (CI-05), with the deadline.
    pub imdn: Vec<(u32, u64)>,
    /// Since when we wait in the lobby (CB-23).
    pub lobby_since: Option<u64>,
    /// The call to the A/V MCU (conference audio).
    pub av_call: Option<CallId>,
}

/// How long we wait in the lobby before leaving (CB-23 recommends an own timeout; design).
pub const LOBBY_TIMEOUT: u64 = 15 * 60;

/// Upper bound of participants invited at once (design).
pub const MAX_INVITEES: usize = 32;

impl<R: Random> Engine<R> {
    fn new_conversation(&mut self, peer: String, group: Option<Group>) -> Option<ConversationId> {
        if self.convs.len() >= MAX_CONVERSATIONS {
            return None;
        }
        let id = self.next_conv;
        self.next_conv = self.next_conv.wrapping_add(1).max(1);
        self.convs.insert(
            id,
            Conversation {
                peer,
                dialog: None,
                queue: Vec::new(),
                group: group.map(Box::new),
                timer: None,
            },
        );
        Some(id)
    }

    fn next_c3p(&mut self) -> u32 {
        self.c3p_request = self.c3p_request.wrapping_add(1);
        self.c3p_request
    }

    fn group_mut(&mut self, id: ConversationId) -> Option<&mut Group> {
        self.convs.get_mut(&id).and_then(|c| c.group.as_deref_mut())
    }

    /// `StartGroup`: addConference to the focus factory (PV-02, PV-09).
    pub(crate) fn start_group(&mut self, invite: &[String], now: u64, out: &mut Vec<Output>) {
        let Some(factory) = self.focus_factory.clone() else {
            out.push(Output::Event(Event::Notice(
                "the server announced no conferencing service (focusFactoryUri)".into(),
            )));
            return;
        };
        let mut invitees = Vec::new();
        for u in invite.iter().take(MAX_INVITEES) {
            match pres::checked_sip_uri(u) {
                Ok(u) if !u.eq_ignore_ascii_case(&self.cfg.aor) => invitees.push(u),
                Ok(_) => {}
                Err(_) => out.push(Output::Event(Event::Notice(format!(
                    "not a usable SIP address: {}",
                    pres::printable(u)
                )))),
            }
        }
        let group = Group {
            organizer: true,
            invitees,
            ..Group::default()
        };
        let Some(id) = self.new_conversation("conference".into(), Some(group)) else {
            out.push(Output::Event(Event::Notice(
                "too many open conversations".into(),
            )));
            return;
        };
        let mut random = [0u8; 16];
        self.reg.fill_random(&mut random);
        let request_id = self.next_c3p();
        let body = match conf::add_conference(
            request_id,
            &self.cfg.aor,
            &factory,
            &conf::conference_id(&random),
        ) {
            Ok(b) => b,
            Err(e) => {
                self.convs.remove(&id);
                out.push(Output::Event(Event::Notice(format!(
                    "conference not created ({e:?})"
                ))));
                return;
            }
        };
        let mut m = self.request("SERVICE", &factory, &factory);
        m.push_header("Content-Type", conf::CONTENT_TYPE);
        m.body = body.into_bytes();
        self.send_request(m, Purpose::CreateConference(id), now, out);
    }

    /// `JoinConference`: join an existing conference (invitation) as attendee.
    pub(crate) fn join_conference(&mut self, focus: &str, now: u64, out: &mut Vec<Output>) {
        if !conf::is_focus_uri(focus) {
            out.push(Output::Event(Event::Notice(
                "not a conference address".into(),
            )));
            return;
        }
        let group = Group {
            focus: Some(focus.to_string()),
            ..Group::default()
        };
        match self.new_conversation(focus.to_string(), Some(group)) {
            Some(id) => self.join_focus(id, now, out),
            None => out.push(Output::Event(Event::Notice(
                "too many open conversations".into(),
            ))),
        }
    }

    /// INVITE to the focus with addUser (CB-10, CB-11).
    fn join_focus(&mut self, id: ConversationId, now: u64, out: &mut Vec<Output>) {
        let Some(g) = self.group_mut(id) else { return };
        let (Some(focus), organizer) = (g.focus.clone(), g.organizer) else {
            return;
        };
        let role = if organizer {
            conf::Role::Presenter
        } else {
            conf::Role::Attendee
        };
        let mut random = [0u8; 16];
        self.reg.fill_random(&mut random);
        let request_id = self.next_c3p();
        let Ok(body) = conf::add_user_join(
            request_id,
            &self.cfg.aor,
            &focus,
            &conf::guid(&random),
            role,
        ) else {
            self.fail_group(id, 400, out);
            return;
        };
        let mut m = self.request("INVITE", &focus, &focus);
        m.push_header("Supported", "timer");
        m.push_header("Supported", "ms-sender");
        m.push_header("Content-Type", conf::CONTENT_TYPE);
        m.body = body.into_bytes();
        self.send_request(m, Purpose::JoinFocus(id), now, out);
    }

    fn fail_group(&mut self, id: ConversationId, status: u16, out: &mut Vec<Output>) {
        self.convs.remove(&id);
        out.push(Output::Event(Event::ConversationFailed {
            conversation: id,
            status,
        }));
    }

    pub(crate) fn on_conference_created(
        &mut self,
        id: ConversationId,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let r = std::str::from_utf8(&resp.body)
            .ok()
            .and_then(|b| conf::parse_response(b).ok());
        match r {
            Some(r) if r.code == conf::Code::Success && r.command == "addConference" => {
                match r.conference.filter(|c| conf::is_focus_uri(c)) {
                    Some(focus) => {
                        if let Some(c) = self.convs.get_mut(&id) {
                            c.peer = focus.clone();
                        }
                        if let Some(g) = self.group_mut(id) {
                            g.focus = Some(focus);
                        }
                        self.join_focus(id, now, out);
                    }
                    None => self.fail_group(id, 500, out),
                }
            }
            Some(r) => {
                out.push(Output::Event(Event::Notice(format!(
                    "conference not created ({})",
                    r.reason.unwrap_or_else(|| "failure".into())
                ))));
                self.fail_group(id, 500, out);
            }
            None => self.fail_group(id, 500, out),
        }
    }

    /// 200 OK of the focus: dialog, ACK, granted role, session timer, then the roster
    /// subscription (CB-12, CB-20).
    pub(crate) fn on_focus_joined(
        &mut self,
        id: ConversationId,
        request: &Message,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Ok(d) = Dialog::from_response(request, resp) else {
            self.fail_group(id, 500, out);
            return;
        };
        let invite_cseq = request.cseq().ok().flatten().map_or(1, |x| x.number);
        let mut ack = im::ack_for_2xx(&d, invite_cseq);
        self.complete(&mut ack);
        self.send_plain(ack, now, out);
        let r = std::str::from_utf8(&resp.body)
            .ok()
            .and_then(|b| conf::parse_response(b).ok());
        if r.as_ref().is_some_and(|r| r.code == conf::Code::Failure) {
            let mut bye = d.clone();
            let mut m = bye.request("BYE");
            self.complete(&mut m);
            self.send_request(m, Purpose::Ignore, now, out);
            self.fail_group(id, 403, out);
            return;
        }
        let timer = our_session_timer(resp, true, now);
        let Some(g) = self.group_mut(id) else { return };
        g.focus_dialog = Some(d);
        g.focus_timer = timer;
        let Some(focus) = g.focus.clone() else { return };
        let mut m = self.request("SUBSCRIBE", &focus, &focus);
        m.push_header("Event", "conference");
        m.push_header("Accept", conf::ROSTER_TYPE);
        m.push_header("Supported", "com.microsoft.autoextend");
        m.push_header("Supported", "ms-benotify");
        m.push_header("Proxy-Require", "ms-benotify");
        m.push_header("Supported", "ms-piggyback-first-notify");
        self.send_request(m, Purpose::Roster(id), now, out);
    }

    pub(crate) fn on_roster_subscribed(
        &mut self,
        id: ConversationId,
        request: &Message,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let expires = expires_of(resp).unwrap_or(3600).max(60);
        let Some(g) = self.group_mut(id) else { return };
        if g.roster_dialog.is_none() {
            g.roster_dialog = Dialog::from_response(request, resp).ok();
        }
        g.roster_refresh_at = Some(now + expires * 4 / 5);
        if !resp.body.is_empty() {
            self.on_roster_document(id, resp, now, out);
        }
    }

    pub(crate) fn on_roster_document(
        &mut self,
        id: ConversationId,
        m: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        if header_token(m, "Content-Type").as_deref() != Some(conf::ROSTER_TYPE) {
            return;
        }
        let Ok(body) = std::str::from_utf8(&m.body) else {
            return;
        };
        let aor = self.cfg.aor.clone();
        let Some(g) = self.group_mut(id) else { return };
        let Some(focus) = g.focus.clone() else { return };
        match g.roster.apply(body, &focus) {
            Ok(true) => {
                // CB-23: our focus endpoint on-hold = lobby; connected = admitted.
                let waiting = g.roster.in_lobby(&aor);
                if waiting != g.lobby_since.is_some() {
                    g.lobby_since = waiting.then_some(now);
                    out.push(Output::Event(Event::ConferenceLobby {
                        conversation: id,
                        waiting,
                    }));
                }
                let Some(g) = self.group_mut(id) else { return };
                let participants = g.roster.participants.clone();
                let join_chat = !g.chat_requested && g.roster.chat_uri.is_some();
                out.push(Output::Event(Event::Participants {
                    conversation: id,
                    participants,
                }));
                if join_chat {
                    self.join_chat(id, now, out);
                }
            }
            Ok(false) => {}
            Err(e) => out.push(Output::Event(Event::Notice(format!(
                "unusable conference state ({e:?})"
            )))),
        }
    }

    /// INVITE to the IM MCU (CI-01, CI-02, CI-03; [MS-CONFIM] §4.1.1 step 10, p. 36).
    fn join_chat(&mut self, id: ConversationId, now: u64, out: &mut Vec<Output>) {
        let Some(g) = self.group_mut(id) else { return };
        let Some(chat) = g.roster.chat_uri.clone() else {
            return;
        };
        g.chat_requested = true;
        let mut m = self.request("INVITE", &chat, &chat);
        m.push_header("Supported", "timer");
        m.push_header("Supported", "ms-sender");
        im::invite_body(&mut m, self.cfg.local_ip);
        self.send_request(m, Purpose::Invite(id), now, out);
    }

    /// The IM MCU session is up: invite the waiting participants.
    pub(crate) fn on_chat_joined(&mut self, id: ConversationId, now: u64, out: &mut Vec<Output>) {
        let invitees = match self.group_mut(id) {
            Some(g) if !g.dialed_out => std::mem::take(&mut g.invitees),
            _ => return,
        };
        for u in invitees {
            self.dial_out(id, &u, now, out);
        }
    }

    /// addUser dial-out over INFO in the focus dialog (CB-37).
    pub(crate) fn dial_out(
        &mut self,
        id: ConversationId,
        invitee: &str,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Ok(invitee) = pres::checked_sip_uri(invitee) else {
            out.push(Output::Event(Event::Notice(
                "not a usable SIP address".into(),
            )));
            return;
        };
        let mut random = [0u8; 16];
        self.reg.fill_random(&mut random);
        let request_id = self.next_c3p();
        let aor = self.cfg.aor.clone();
        let Some(g) = self.group_mut(id) else {
            out.push(Output::Event(Event::Notice("no such conversation".into())));
            return;
        };
        if g.dialed_out {
            out.push(Output::Event(Event::Notice(
                "invitations are only possible in conferences joined through the focus".into(),
            )));
            return;
        }
        let (Some(focus), Some(chat), Some(mut d)) = (
            g.focus.clone(),
            g.roster.chat_uri.clone(),
            g.focus_dialog.take(),
        ) else {
            if g.invitees.len() < MAX_INVITEES {
                g.invitees.push(invitee);
            }
            return;
        };
        let body = conf::add_user_dial_out(
            request_id,
            &aor,
            &focus,
            &chat,
            &invitee,
            &conf::guid(&random),
        );
        let mut m = d.request("INFO");
        g.focus_dialog = Some(d);
        let Ok(body) = body else { return };
        self.complete(&mut m);
        m.push_header("Supported", "timer");
        m.push_header("Content-Type", conf::CONTENT_TYPE);
        m.body = body.into_bytes();
        self.send_request(m, Purpose::DialOut(id), now, out);
    }

    /// The final C3P answer of a command, in a 200 OK or an INFO of the focus (CB-31).
    pub(crate) fn on_c3p_answer(&mut self, id: ConversationId, body: &[u8], out: &mut Vec<Output>) {
        let Some(r) = std::str::from_utf8(body)
            .ok()
            .and_then(|b| conf::parse_response(b).ok())
        else {
            return;
        };
        if r.command == "modifyEndpointMedia" && r.code == conf::Code::Failure {
            out.push(Output::Event(Event::Notice(format!(
                "[{id}] muting refused ({})",
                r.reason.clone().unwrap_or_else(|| "failure".into())
            ))));
        }
        if r.command == "addUser" && r.code == conf::Code::Failure {
            out.push(Output::Event(Event::Notice(format!(
                "[{id}] invitation of {} failed ({})",
                r.user.unwrap_or_default(),
                r.reason.unwrap_or_else(|| "failure".into())
            ))));
        }
    }

    /// 202 Accepted of the IM MCU: wait for the IMDN of this Message-Id (CI-05).
    pub(crate) fn on_message_accepted(&mut self, id: ConversationId, resp: &Message, now: u64) {
        let mid = conf::message_id(resp.single("Message-Id").ok().flatten());
        if let Some(g) = self.group_mut(id) {
            if g.imdn.len() < 64 {
                g.imdn.push((mid, now + conf::IMDN_TIMEOUT));
            }
        }
    }

    /// IMDN in a BENOTIFY of the IM MCU (CI-05).
    pub(crate) fn on_imdn(&mut self, id: ConversationId, m: &Message, out: &mut Vec<Output>) {
        if header_token(m, "Content-Type").as_deref() != Some(conf::IMDN_TYPE) {
            return;
        }
        let Some(imdn) = std::str::from_utf8(&m.body)
            .ok()
            .and_then(|b| conf::parse_imdn(b).ok())
        else {
            return;
        };
        let Some(g) = self.group_mut(id) else { return };
        let Some(pos) = g.imdn.iter().position(|(mid, _)| *mid == imdn.message_id) else {
            return;
        };
        g.imdn.remove(pos);
        if imdn.failures.is_empty() {
            out.push(Output::Event(Event::Delivered { conversation: id }));
        }
        for (recipient, status) in imdn.failures {
            out.push(Output::Event(Event::NotDeliveredTo {
                conversation: id,
                recipient,
                status,
            }));
        }
    }

    /// A request of the focus in our focus dialog: C3P INFO, session refresh, ejection.
    pub(crate) fn on_focus_request(
        &mut self,
        id: ConversationId,
        req: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let method = req.method().unwrap_or("").to_ascii_uppercase();
        match method.as_str() {
            "ACK" => {}
            "INFO" => {
                let r = response_to(req, 200, "OK");
                self.send_plain(r, now, out);
                self.on_c3p_answer(id, &req.body, out);
            }
            "UPDATE" => {
                let mut r = response_to(req, 200, "OK");
                r.push_header("Supported", "timer");
                self.send_plain(r, now, out);
            }
            "BYE" => {
                let r = response_to(req, 200, "OK");
                self.send_plain(r, now, out);
                self.leave_group(id, now, out, false);
            }
            _ => {
                let r = response_to(req, 405, "Method Not Allowed");
                self.send_plain(r, now, out);
            }
        }
    }

    /// Leave: BYE to the IM MCU, unsubscribe the roster, BYE to the focus ([MS-CONFIM] §4.3.2).
    /// `by_us`: false when the focus ended the dialog (no BYE to the focus then).
    pub(crate) fn leave_group(
        &mut self,
        id: ConversationId,
        now: u64,
        out: &mut Vec<Output>,
        by_us: bool,
    ) {
        let Some(c) = self.convs.remove(&id) else {
            return;
        };
        if let Some(mut d) = c.dialog {
            let mut m = d.request("BYE");
            self.complete(&mut m);
            self.send_request(m, Purpose::Ignore, now, out);
        }
        if let Some(g) = c.group {
            if let Some(call) = g.av_call {
                self.hangup(call, now, out);
            }
            if let Some(mut d) = g.roster_dialog {
                let mut m = d.request("SUBSCRIBE");
                self.complete(&mut m);
                m.push_header("Event", "conference");
                m.push_header("Expires", "0");
                self.send_request(m, Purpose::Ignore, now, out);
            }
            if let (Some(mut d), true) = (g.focus_dialog, by_us) {
                let mut m = d.request("BYE");
                self.complete(&mut m);
                self.send_request(m, Purpose::Ignore, now, out);
            }
        }
        out.push(Output::Event(Event::ConversationEnded { conversation: id }));
    }

    /// Clock for groups and session timers: dialog refresh by UPDATE, roster refresh, IMDN
    /// timeouts (CI-05: treat as 408 for all).
    pub(crate) fn group_tick(&mut self, now: u64, out: &mut Vec<Output>) {
        let ids: Vec<ConversationId> = self.convs.keys().copied().collect();
        let mut lobby_expired = Vec::new();
        for id in ids {
            let Some(c) = self.convs.get_mut(&id) else {
                continue;
            };
            let mut sends: Vec<(Message, Purpose)> = Vec::new();
            if let (Some(t), Some(d)) = (c.timer.as_mut(), c.dialog.as_mut()) {
                if now >= t.next {
                    t.next = now + t.interval / 2;
                    let mut m = d.request("UPDATE");
                    m.push_header("Supported", "timer");
                    sends.push((m, Purpose::Ignore));
                }
            }
            let mut timed_out = 0usize;
            if let Some(g) = c.group.as_deref_mut() {
                if let (Some(t), Some(d)) = (g.focus_timer.as_mut(), g.focus_dialog.as_mut()) {
                    if now >= t.next {
                        t.next = now + t.interval / 2;
                        let mut m = d.request("UPDATE");
                        m.push_header("Supported", "timer");
                        sends.push((m, Purpose::Ignore));
                    }
                }
                if let (Some(at), Some(d)) = (g.roster_refresh_at, g.roster_dialog.as_mut()) {
                    if now >= at {
                        g.roster_refresh_at = Some(now + 3600);
                        let mut m = d.request("SUBSCRIBE");
                        m.push_header("Event", "conference");
                        m.push_header("Accept", conf::ROSTER_TYPE);
                        m.push_header("Supported", "ms-benotify");
                        m.push_header("Proxy-Require", "ms-benotify");
                        sends.push((m, Purpose::Roster(id)));
                    }
                }
                let before = g.imdn.len();
                g.imdn.retain(|(_, deadline)| now < *deadline);
                timed_out = before - g.imdn.len();
                if g.lobby_since.is_some_and(|t| now >= t + LOBBY_TIMEOUT) {
                    lobby_expired.push(id);
                }
            }
            for _ in 0..timed_out {
                out.push(Output::Event(Event::NotDelivered {
                    conversation: id,
                    status: 408,
                }));
            }
            for (mut m, purpose) in sends {
                self.complete(&mut m);
                self.send_request(m, purpose, now, out);
            }
        }
        for id in lobby_expired {
            out.push(Output::Event(Event::Notice(format!(
                "[{id}] not admitted from the lobby in time"
            ))));
            self.leave_group(id, now, out, true);
        }
    }

    /// `JoinConferenceAudio`: an ordinary audio call to the A/V MCU of the conference (CA-01,
    /// CA-02: the INVITE itself joins, no addUser dial-in); the call belongs to the conversation
    /// and ends with it.
    pub(crate) fn join_conference_audio(
        &mut self,
        id: ConversationId,
        local: sdp::Local,
        policy: sdp::Encryption,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let av = match self.group_mut(id) {
            Some(g) if g.lobby_since.is_some() => {
                out.push(Output::Event(Event::Notice("still in the lobby".into())));
                return;
            }
            Some(g) if g.av_call.is_some() => {
                out.push(Output::Event(Event::Notice("audio already joined".into())));
                return;
            }
            Some(g) => g.roster.av_uri.clone(),
            None => None,
        };
        let Some(av) = av else {
            out.push(Output::Event(Event::Notice(
                "this conference has no audio".into(),
            )));
            return;
        };
        let call = self.next_call_id();
        let before = self.calls.len();
        // SD-12: offer SRTP and client SSRTP to the MCU; it answers with one of them.
        let mut local = local;
        local.ssrtp = policy != sdp::Encryption::Off;
        self.start_call(&av, local, policy, now, out);
        if self.calls.len() > before {
            if let Some(g) = self.group_mut(id) {
                g.av_call = Some(call);
            }
            out.push(Output::Event(Event::ConferenceAudio {
                conversation: id,
                call,
            }));
        }
    }

    /// `ConferenceMute`: modifyEndpointMedia over INFO to the focus with our audio's keys from
    /// the roster (CA-20). Unmuting is refused by the MCU if the role may not (mayModifyOwnFilters).
    pub(crate) fn conference_mute(
        &mut self,
        id: ConversationId,
        mute: bool,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let request_id = self.next_c3p();
        let (aor, gruu) = (self.cfg.aor.clone(), self.gruu.clone());
        let Some(g) = self.group_mut(id) else { return };
        let (Some(focus), Some(av)) = (g.focus.clone(), g.roster.av_uri.clone()) else {
            out.push(Output::Event(Event::Notice("no conference audio".into())));
            return;
        };
        let Some(keys) = g.roster.own_audio(&aor, &gruu) else {
            out.push(Output::Event(Event::Notice(
                "our audio is not in the conference yet".into(),
            )));
            return;
        };
        let Some(mut d) = g.focus_dialog.take() else {
            return;
        };
        let body = conf::modify_endpoint_media(request_id, &aor, &focus, &av, &keys, mute);
        let mut m = d.request("INFO");
        if let Some(g) = self.group_mut(id) {
            g.focus_dialog = Some(d);
        }
        let Ok(body) = body else { return };
        self.complete(&mut m);
        m.push_header("Supported", "timer");
        m.push_header("Content-Type", conf::CONTENT_TYPE);
        m.body = body.into_bytes();
        self.send_request(m, Purpose::DialOut(id), now, out);
    }

    /// An INVITE of the IM MCU calling us (dial-out to us, CI-01) carries `Ms-Focus-Uri`.
    pub(crate) fn dialed_out_group(req: &Message) -> Option<Group> {
        let v = req.single("Ms-Focus-Uri").ok().flatten()?;
        let focus = commscraft_sip_msg::parse_name_addr(v).ok()?.uri;
        conf::is_focus_uri(&focus).then(|| Group {
            focus: Some(focus),
            dialed_out: true,
            chat_requested: true,
            ..Group::default()
        })
    }

    /// An invitation INVITE (`application/ms-conf-invite+xml`, secondary evidence K-CI-3):
    /// accepted as delivered (200 OK, no session), the user decides about joining.
    pub(crate) fn on_invitation(&mut self, req: &Message, now: u64, out: &mut Vec<Output>) {
        let inv = std::str::from_utf8(&req.body)
            .ok()
            .and_then(|b| conf::parse_invitation(b).ok());
        let from = req
            .from()
            .ok()
            .flatten()
            .and_then(|f| pres::checked_sip_uri(f.uri.split(';').next().unwrap_or("")).ok());
        let (Some(invitation), Some(from)) = (inv, from) else {
            let r = response_to(req, 488, "Not Acceptable Here");
            self.send_plain(r, now, out);
            return;
        };
        let tag = self.hex(5);
        let mut ok = response_to(req, 200, "OK");
        for h in ok.headers.iter_mut().filter(|h| h.name == "To") {
            h.value = format!("{};tag={tag}", h.value);
        }
        ok.push_header("Contact", &format!("<{}>", self.gruu));
        self.send_plain(ok, now, out);
        out.push(Output::Event(Event::ConferenceInvitation {
            from,
            invitation,
        }));
    }
}
