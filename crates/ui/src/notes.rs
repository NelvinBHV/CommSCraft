//! Desktop notifications of the interface: what the Windows client shows as toasts – incoming
//! calls, meeting invitations, new messages in a conversation that is not open – as items the
//! application hands to the desktop's notification service (`org.freedesktop.Notifications`),
//! and the actions chosen there. Without such a service the interface shows them in its own
//! small window instead (`CommsUi::system_notifications`).

use crate::app::{CommsUi, UiIntent, Win};
use crate::model::{CallKind, CallState, ConvKey, Presence};

/// What a notification is about; it stays the same while the item is pending.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NoteKey {
    Call(u32),
    /// A meeting invitation, by inviter and focus.
    Invitation(String, String),
    /// New messages of a conversation, up to that message count.
    Message(ConvKey, usize),
}

/// One notification: title, text and the buttons (action ID, label).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub key: NoteKey,
    pub summary: String,
    pub body: String,
    pub actions: Vec<(&'static str, String)>,
    /// A call: shown as urgent until answered.
    pub urgent: bool,
}

/// The action of clicking the notification itself.
pub const DEFAULT: &str = "default";

impl CommsUi {
    /// The notifications that are due now (the same items as the interface's own window).
    pub fn pending_notes(&self) -> Vec<Note> {
        let t = self.t;
        let caps = &self.state.capabilities;
        let mut out = Vec::new();
        for c in self
            .state
            .calls
            .iter()
            .filter(|c| c.state == CallState::Incoming)
        {
            let what = match c.kind {
                CallKind::Screen => t.screen_offered,
                _ if c.offers_video => t.incoming_video_call,
                _ => t.incoming_call,
            };
            let mut actions = Vec::new();
            if c.kind == CallKind::Screen {
                if caps.video_call.usable() {
                    actions.push(("screen", t.view_screen.to_string()));
                }
            } else {
                if caps.audio_call.usable() {
                    actions.push(("answer", t.answer.to_string()));
                }
                if c.offers_video && caps.video_call.usable() {
                    actions.push(("video", t.answer_video.to_string()));
                }
            }
            actions.push(("decline", t.decline.to_string()));
            out.push(Note {
                key: NoteKey::Call(c.id),
                summary: self.state.name_of(&c.peer),
                body: what.to_string(),
                actions,
                urgent: true,
            });
        }
        for inv in &self.state.invitations {
            out.push(Note {
                key: NoteKey::Invitation(inv.from.clone(), inv.focus.clone()),
                summary: self.state.name_of(&inv.from),
                body: if inv.subject.is_empty() {
                    t.invites_to_meeting.to_string()
                } else {
                    format!("{}: {}", t.invites_to_meeting, inv.subject)
                },
                actions: vec![
                    ("join", t.join.to_string()),
                    ("decline", t.decline.to_string()),
                ],
                urgent: false,
            });
        }
        let quiet = self.quiet_for_messages();
        for c in self.state.conversations.iter().filter(|c| {
            !quiet
                && c.unread > 0
                && !self.windows.contains(&Win::Conv(c.key))
                && !self.toasts_seen.contains(&(c.key, c.messages.len()))
        }) {
            out.push(Note {
                key: NoteKey::Message(c.key, c.messages.len()),
                summary: c.title.clone(),
                body: c.last_text().map(|m| m.text.clone()).unwrap_or_default(),
                actions: vec![(DEFAULT, t.open.to_string())],
                urgent: false,
            });
        }
        out
    }

    /// An action chosen in a notification (or `DEFAULT` for a click on it).
    pub fn note_action(&mut self, key: &NoteKey, action: &str) {
        match (key, action) {
            (NoteKey::Call(id), "answer") => self.emit(UiIntent::AnswerCall { id: *id }),
            (NoteKey::Call(id), "video") => self.emit(UiIntent::AnswerVideo { id: *id }),
            (NoteKey::Call(id), "screen") => self.emit(UiIntent::ViewScreen { id: *id }),
            (NoteKey::Call(id), "decline") => self.emit(UiIntent::HangUp { id: *id }),
            (NoteKey::Invitation(from, focus), a) if a == "join" || a == "decline" => {
                let index = self
                    .state
                    .invitations
                    .iter()
                    .position(|i| &i.from == from && &i.focus == focus);
                if let Some(index) = index {
                    self.emit(if a == "join" {
                        UiIntent::AcceptInvitation { index }
                    } else {
                        UiIntent::DeclineInvitation { index }
                    });
                }
            }
            (NoteKey::Message(k, _), DEFAULT) => self.select_conversation(*k),
            _ => {}
        }
    }

    /// Message notifications are off (option), or quiet while the own status is Do Not Disturb.
    pub fn quiet_for_messages(&self) -> bool {
        !self.prefs.message_alerts
            || (self.prefs.quiet_when_dnd && self.state.me.presence == Presence::DoNotDisturb)
    }

    /// A message notification the user closed: not shown again for these messages.
    pub fn note_dismissed(&mut self, key: &NoteKey) {
        if let NoteKey::Message(k, n) = key {
            self.toasts_seen.insert((*k, *n));
        }
    }
}
