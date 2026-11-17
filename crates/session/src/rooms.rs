//! Persistent chat (function 20): the SIP dialog with the persistent chat pool and then with
//! the channel server it names, XCCOS in SIP INFO (XC-01, XC-05; [MS-XCCOSIP] 20240820 §1.4,
//! pp. 12–13; §3.1.3–3.1.4, p. 46).
//!
//! The document gives neither the SDP of the INVITE nor the Content-Type of the INFO (L-16);
//! secondary evidence (OB-17) names an IM SDP and `text/plain`, which this follows until an
//! interop capture decides (IT-05). Skype for Business Server 2019 has no persistent chat, so
//! this is `tested-offline` only.

use crate::{response_to, Engine, Event, Output, Purpose, Random};
use commscraft_sip_msg::dialog::Dialog;
use commscraft_sip_msg::Message;
use commscraft_xccos as xccos;

/// The Content-Type of the XCCOS INFO body (secondary evidence OB-17, IT-05).
pub const CONTENT_TYPE: &str = "text/plain";
/// How many rooms getassociations asks for (§3.1.11.3, p. 53; the default is 100).
pub const MAX_ROOMS: u32 = 100;

pub(crate) struct Rooms {
    pub client: xccos::Client,
    pub dialog: Option<Dialog>,
    /// The pool, or after cmd:requri the channel server.
    pub target: String,
    pub on_channel_server: bool,
}

impl<R: Random> Engine<R> {
    /// `ChatRooms`: connect to the persistent chat pool from provisioning
    /// (`DefaultPersistentChatPoolUri`, [MS-SIPREGE] 20250819 §2.2.2.5.11, p. 71).
    pub(crate) fn rooms_connect(&mut self, now: u64, out: &mut Vec<Output>) {
        let Some(pool) = self.persistent_chat_pool.clone() else {
            out.push(Output::Event(Event::Notice(
                "the server offers no persistent chat (persistentChatConfiguration)".into(),
            )));
            return;
        };
        if self.rooms.as_ref().is_some_and(|r| r.dialog.is_some()) {
            out.push(Output::Event(Event::Notice(
                "already connected to the chat rooms".into(),
            )));
            return;
        }
        let mut seed = [0u8; 8];
        self.reg.fill_random(&mut seed);
        let domain = self.user_domain();
        let Ok(client) = xccos::Client::new(
            &self.cfg.aor,
            &domain,
            &self.cfg.user_agent,
            u64::from_be_bytes(seed),
        ) else {
            return;
        };
        self.rooms = Some(Box::new(Rooms {
            client,
            dialog: None,
            target: pool.clone(),
            on_channel_server: false,
        }));
        self.rooms_invite(&pool, now, out);
    }

    fn rooms_invite(&mut self, target: &str, now: u64, out: &mut Vec<Output>) {
        let mut m = self.request("INVITE", target, target);
        m.push_header("Supported", "ms-sender");
        commscraft_im::invite_body(&mut m, self.cfg.local_ip, false);
        self.send_request(m, Purpose::RoomsInvite, now, out);
    }

    /// 200 to our INVITE: ACK; on the pool ask for the channel server (cmd:requri), on the
    /// channel server for its information and our rooms (§3.1.4, §3.1.11.4: "after connection
    /// to the channel server succeeds").
    pub(crate) fn on_rooms_answer(
        &mut self,
        request: &Message,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Ok(d) = Dialog::from_response(request, resp) else {
            return;
        };
        let cseq = request.cseq().ok().flatten().map_or(1, |x| x.number);
        let mut ack = commscraft_im::ack_for_2xx(&d, cseq);
        self.complete(&mut ack);
        self.send_plain(ack, now, out);
        let Some(r) = self.rooms.as_mut() else { return };
        r.dialog = Some(d);
        if r.on_channel_server {
            self.rooms_command(&xccos::Command::ServerInfo, now, out);
            self.rooms_command(
                &xccos::Command::Associations {
                    manager: false,
                    max: MAX_ROOMS,
                },
                now,
                out,
            );
        } else {
            self.rooms_command(&xccos::Command::RequestUri, now, out);
        }
    }

    pub(crate) fn rooms_failed(&mut self, code: u16, out: &mut Vec<Output>) {
        self.rooms = None;
        out.push(Output::Event(Event::Notice(format!(
            "the persistent chat server refused us ({code})"
        ))));
    }

    /// One XCCOS command in an INFO of the dialog.
    pub(crate) fn rooms_command(&mut self, cmd: &xccos::Command, now: u64, out: &mut Vec<Output>) {
        let Some(r) = self.rooms.as_mut() else {
            out.push(Output::Event(Event::Notice(
                "not connected to the chat rooms – /rooms".into(),
            )));
            return;
        };
        let Some(mut d) = r.dialog.take() else {
            return;
        };
        let body = r.client.command(cmd, now.saturating_mul(1000));
        let mut m = d.request("INFO");
        r.dialog = Some(d);
        match body {
            Ok(body) => {
                self.complete(&mut m);
                m.push_header("Content-Type", CONTENT_TYPE);
                m.body = body.into_bytes();
                self.send_request(m, Purpose::RoomsInfo, now, out);
            }
            Err(e) => out.push(Output::Event(Event::Notice(format!(
                "chat room command not possible ({e:?})"
            )))),
        }
    }

    /// A message to a room (§3.1.13, p. 55).
    pub(crate) fn rooms_send(&mut self, room: &str, text: &str, now: u64, out: &mut Vec<Output>) {
        let time = commscraft_xml::xs_datetime(now).unwrap_or_default();
        let not_sent = |out: &mut Vec<Output>, why: String| {
            out.push(Output::Event(Event::Notice(why)));
            out.push(Output::Event(Event::RoomNotSent {
                room: room.to_string(),
                status: 0,
            }));
        };
        let Some(r) = self.rooms.as_mut() else {
            not_sent(out, "not connected to the chat rooms – /rooms".into());
            return;
        };
        let Some(mut d) = r.dialog.take() else {
            not_sent(out, "the chat room server is not connected yet".into());
            return;
        };
        let body = r.client.chat(room, text, &time);
        let mut m = d.request("INFO");
        r.dialog = Some(d);
        match body {
            Ok(body) => {
                self.complete(&mut m);
                m.push_header("Content-Type", CONTENT_TYPE);
                m.body = body.into_bytes();
                self.send_request(m, Purpose::RoomsSend(room.to_string()), now, out);
            }
            Err(e) => not_sent(out, format!("not sent to the room ({e:?})")),
        }
    }

    /// Leaves the chat rooms: BYE.
    pub(crate) fn rooms_leave(&mut self, now: u64, out: &mut Vec<Output>) {
        if let Some(mut d) = self.rooms.take().and_then(|r| r.dialog) {
            let mut bye = d.request("BYE");
            self.complete(&mut bye);
            self.send_request(bye, Purpose::Ignore, now, out);
        }
    }

    pub(crate) fn rooms_of(&self, req: &Message) -> bool {
        self.rooms
            .as_ref()
            .and_then(|r| r.dialog.as_ref())
            .is_some_and(|d| d.is_peer_request(req))
    }

    /// A request in the chat dialog: INFO carries XCCOS replies and room messages (200 is the
    /// delivery status, XC-01); a BYE ends the connection.
    pub(crate) fn on_rooms_request(&mut self, req: &Message, now: u64, out: &mut Vec<Output>) {
        let method = req.method().unwrap_or("").to_ascii_uppercase();
        let ok = response_to(req, 200, "OK");
        match method.as_str() {
            "INFO" => {
                self.send_plain(ok, now, out);
                let events = match (self.rooms.as_mut(), std::str::from_utf8(&req.body)) {
                    (Some(r), Ok(body)) => r.client.on_body(body).unwrap_or_default(),
                    _ => Vec::new(),
                };
                self.rooms_events(events, now, out);
            }
            "BYE" => {
                self.send_plain(ok, now, out);
                self.rooms = None;
                out.push(Output::Event(Event::Notice(
                    "the chat rooms were closed".into(),
                )));
            }
            _ => {
                let r = response_to(req, 405, "Method Not Allowed");
                self.send_plain(r, now, out);
            }
        }
    }

    /// The transaction timers of pending XCCOS commands (§3.1.2.1.6, p. 45).
    pub(crate) fn rooms_tick(&mut self, now: u64, out: &mut Vec<Output>) {
        let events = self
            .rooms
            .as_mut()
            .map(|r| r.client.tick(now.saturating_mul(1000)))
            .unwrap_or_default();
        self.rooms_events(events, now, out);
    }

    fn rooms_events(&mut self, events: Vec<xccos::Event>, now: u64, out: &mut Vec<Output>) {
        for e in events {
            if let xccos::Event::ChannelServer(uri) = &e {
                // §3.1.3.5 (p. 46): BYE on the pool, then a dialog with the channel server.
                let Ok(uri) = crate::pres::checked_sip_uri(uri) else {
                    continue;
                };
                if let Some(r) = self.rooms.as_mut() {
                    if let Some(mut d) = r.dialog.take() {
                        let mut bye = d.request("BYE");
                        self.complete(&mut bye);
                        self.send_request(bye, Purpose::Ignore, now, out);
                    }
                }
                if let Some(r) = self.rooms.as_mut() {
                    r.target = uri.clone();
                    r.on_channel_server = true;
                }
                self.rooms_invite(&uri, now, out);
                continue;
            }
            out.push(Output::Event(Event::ChatRooms(e)));
        }
    }
}
