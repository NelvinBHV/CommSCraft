//! Desktop notifications over the session bus: the Desktop Notifications Specification 1.2
//! (`org.freedesktop.Notifications`: `Notify`, `CloseNotification`, the signals `ActionInvoked`
//! and `NotificationClosed`) – the standard notification service of Linux desktops, which a
//! Flatpak reaches with `--talk-name=org.freedesktop.Notifications`. Next to the notification
//! area icon (`tray`).
//!
//! Security by design: two threads of their own; only the title, a short text and fixed action
//! IDs leave the process (the text is the start of the newest message, as the toast of the
//! Windows client shows it); the service's signals are matched to the IDs we got back, anything
//! else is ignored.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use futures_lite::StreamExt;

const DEST: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";
/// The text of a message notification is cut to this many characters (design).
const MAX_BODY: usize = 200;

/// A notification to show.
pub struct Show {
    pub tag: u64,
    pub summary: String,
    pub body: String,
    /// Action ID and label.
    pub actions: Vec<(String, String)>,
    pub urgent: bool,
}

enum Request {
    Show(Show),
    Close(u64),
}

/// What happened to a notification: an action (or `default`), or `closed` by the user.
pub type Reply = (u64, String);

pub struct Notifier {
    tx: Mutex<Sender<Request>>,
    replies: Mutex<Receiver<Reply>>,
}

/// The service's IDs of our notifications, and ours for them.
type Ids = Arc<Mutex<HashMap<u32, u64>>>;

impl Notifier {
    /// Connects to the notification service; `None` without one (the interface then shows its
    /// own window).
    pub fn start() -> Option<Notifier> {
        let conn = futures_lite::future::block_on(async {
            let c = zbus::Connection::session().await.ok()?;
            c.call_method(Some(DEST), PATH, Some(DEST), "GetServerInformation", &())
                .await
                .ok()?;
            Some(c)
        })?;
        let ids: Ids = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = mpsc::channel::<Request>();
        let (reply_tx, replies) = mpsc::channel::<Reply>();
        let (c1, ids1) = (conn.clone(), ids.clone());
        std::thread::Builder::new()
            .name("notify".into())
            .spawn(move || requests(&c1, &ids1, &rx))
            .ok()?;
        std::thread::Builder::new()
            .name("notify-signals".into())
            .spawn(move || futures_lite::future::block_on(signals(&conn, &ids, &reply_tx)))
            .ok()?;
        Some(Notifier {
            tx: Mutex::new(tx),
            replies: Mutex::new(replies),
        })
    }

    pub fn show(&self, s: Show) {
        if let Ok(tx) = self.tx.lock() {
            let _ = tx.send(Request::Show(s));
        }
    }

    pub fn close(&self, tag: u64) {
        if let Ok(tx) = self.tx.lock() {
            let _ = tx.send(Request::Close(tag));
        }
    }

    /// What happened to our notifications since the last call.
    pub fn replies(&self) -> Vec<Reply> {
        self.replies
            .lock()
            .map(|r| r.try_iter().take(16).collect())
            .unwrap_or_default()
    }
}

fn requests(conn: &zbus::Connection, ids: &Ids, rx: &Receiver<Request>) {
    while let Ok(r) = rx.recv() {
        match r {
            Request::Show(s) => {
                let body: String = s.body.chars().take(MAX_BODY).collect();
                let actions: Vec<&str> = s
                    .actions
                    .iter()
                    .flat_map(|(id, label)| [id.as_str(), label.as_str()])
                    .collect();
                let mut hints: HashMap<&str, zbus::Variant<'_>> = HashMap::new();
                // Urgency 2 (critical) keeps a ringing call on screen until it is answered.
                hints.insert(
                    "urgency",
                    zbus::Variant::from(if s.urgent { 2u8 } else { 1u8 }),
                );
                hints.insert(
                    "category",
                    zbus::Variant::from(if s.urgent { "call" } else { "im.received" }),
                );
                hints.insert(
                    "desktop-entry",
                    zbus::Variant::from("io.github.NelvinBHV.CommSCraft"),
                );
                let args = (
                    "CommSCraft",
                    0u32,
                    "io.github.NelvinBHV.CommSCraft",
                    s.summary.as_str(),
                    body.as_str(),
                    actions,
                    hints,
                    -1i32,
                );
                let call = conn.call_method(Some(DEST), PATH, Some(DEST), "Notify", &args);
                let id = futures_lite::future::block_on(call)
                    .ok()
                    .and_then(|m| m.body().deserialize::<u32>().ok());
                if let (Some(id), Ok(mut map)) = (id, ids.lock()) {
                    map.insert(id, s.tag);
                }
            }
            Request::Close(tag) => {
                let id = ids.lock().ok().and_then(|mut map| {
                    let id = map.iter().find(|(_, t)| **t == tag).map(|(i, _)| *i)?;
                    map.remove(&id);
                    Some(id)
                });
                if let Some(id) = id {
                    let call =
                        conn.call_method(Some(DEST), PATH, Some(DEST), "CloseNotification", &id);
                    let _ = futures_lite::future::block_on(call);
                }
            }
        }
    }
}

async fn signals(conn: &zbus::Connection, ids: &Ids, tx: &Sender<Reply>) {
    let Ok(rule) = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface(DEST)
        .path(PATH)
        .build()
    else {
        return;
    };
    let Ok(mut stream) = zbus::MessageStream::for_match_rule(rule, conn, None).await else {
        return;
    };
    while let Some(Ok(m)) = stream.next().await {
        let member = m
            .header()
            .member()
            .map(|n| n.to_string())
            .unwrap_or_default();
        match member.as_str() {
            "ActionInvoked" => {
                let Ok((id, action)) = m.body().deserialize::<(u32, String)>() else {
                    continue;
                };
                let tag = ids.lock().ok().and_then(|map| map.get(&id).copied());
                if let Some(tag) = tag {
                    let _ = tx.send((tag, action));
                }
            }
            "NotificationClosed" => {
                let Ok((id, reason)) = m.body().deserialize::<(u32, u32)>() else {
                    continue;
                };
                let tag = ids.lock().ok().and_then(|mut map| map.remove(&id));
                // Reason 2: dismissed by the user (the others: expired, closed by us, other).
                if let (Some(tag), 2) = (tag, reason) {
                    let _ = tx.send((tag, "closed".into()));
                }
            }
            _ => {}
        }
    }
}

/// What the interface tells the session worker for its notifications: when it last drew a frame
/// (a hidden, minimised or locked window gets none under Wayland) and whether message
/// notifications are off (option, or quiet while Do Not Disturb).
#[derive(Clone)]
pub struct UiState {
    pub last_frame_ms: Arc<std::sync::atomic::AtomicU64>,
    pub messages_quiet: Arc<std::sync::atomic::AtomicBool>,
    /// "Notifications off" in the notification area icon's menu: no notifications for
    /// messages and invitations (calls still ring and show).
    pub muted: Arc<std::sync::atomic::AtomicBool>,
}

/// A window that drew no frame for this long is taken as not seen (design).
const UNSEEN_MS: u64 = 3000;

/// What a notification of the worker is about.
#[derive(Clone)]
enum About {
    Call(u32),
    Invitation(String),
    Text(u32),
}

/// What the user chose in a notification, for the worker to carry out.
pub enum Chosen {
    AnswerCall(u32),
    AnswerVideo(u32),
    ViewScreen(u32),
    HangUp(u32),
    Join(String),
    /// Show the conversation of this session.
    Open(u32),
}

/// The notifications of the session worker: it sees every event, so incoming calls and
/// invitations are shown – and answered – even while the window gets no frames; new messages
/// while the window is not seen.
pub struct WorkerNotes {
    notifier: Arc<Notifier>,
    ui: UiState,
    t: &'static commscraft_ui::text::Texts,
    shown: HashMap<u64, About>,
    next: u64,
    /// Calls that offer video or share a screen (set after `CallStarted`).
    kinds: HashMap<u32, (bool, bool)>,
    /// The callers of ringing calls.
    callers: HashMap<u32, String>,
}

fn short(uri: &str) -> String {
    uri.strip_prefix("sip:")
        .unwrap_or(uri)
        .chars()
        .take(80)
        .collect()
}

impl WorkerNotes {
    pub fn new(
        notifier: Arc<Notifier>,
        ui: UiState,
        t: &'static commscraft_ui::text::Texts,
    ) -> Self {
        WorkerNotes {
            notifier,
            ui,
            t,
            shown: HashMap::new(),
            next: 1,
            kinds: HashMap::new(),
            callers: HashMap::new(),
        }
    }

    fn ui_seen(&self) -> bool {
        let last = self
            .ui
            .last_frame_ms
            .load(std::sync::atomic::Ordering::Relaxed);
        now_ms().saturating_sub(last) < UNSEEN_MS
    }

    fn show(
        &mut self,
        about: About,
        summary: String,
        body: String,
        actions: Vec<(&str, &str)>,
        urgent: bool,
    ) {
        if !matches!(about, About::Call(_))
            && self.ui.muted.load(std::sync::atomic::Ordering::Relaxed)
        {
            return;
        }
        let tag = self.next;
        self.next += 1;
        self.notifier.show(Show {
            tag,
            summary,
            body,
            actions: actions
                .into_iter()
                .map(|(a, l)| (a.to_string(), l.to_string()))
                .collect(),
            urgent,
        });
        self.shown.insert(tag, about);
    }

    fn close_where(&mut self, f: impl Fn(&About) -> bool) {
        let tags: Vec<u64> = self
            .shown
            .iter()
            .filter(|(_, a)| f(a))
            .map(|(t, _)| *t)
            .collect();
        for tag in tags {
            self.shown.remove(&tag);
            self.notifier.close(tag);
        }
    }

    /// Looks at one event of the worker.
    pub fn observe(&mut self, e: &commscraft_ui::model::AppEvent) {
        use commscraft_ui::model::{AppEvent, CallKind};
        let t = self.t;
        match e {
            AppEvent::CallStarted {
                id,
                peer,
                incoming: true,
                ..
            } => {
                let (video, screen) = self.kinds.get(id).copied().unwrap_or((false, false));
                self.callers.insert(*id, peer.clone());
                self.ring(*id, peer, video, screen);
            }
            AppEvent::CallOffersVideo { id } => {
                self.kinds.entry(*id).or_default().0 = true;
                self.reshow_call(*id);
            }
            AppEvent::CallKind { id, kind } => {
                self.kinds.entry(*id).or_default().1 = *kind == CallKind::Screen;
                self.reshow_call(*id);
            }
            AppEvent::CallConnected { id, .. } | AppEvent::CallEnded { id, .. } => {
                let id = *id;
                self.kinds.remove(&id);
                self.callers.remove(&id);
                self.close_where(|a| matches!(a, About::Call(c) if *c == id));
            }
            AppEvent::Invitation(inv) => {
                let body = if inv.subject.is_empty() {
                    t.invites_to_meeting.to_string()
                } else {
                    format!("{}: {}", t.invites_to_meeting, inv.subject)
                };
                self.show(
                    About::Invitation(inv.focus.clone()),
                    short(&inv.from),
                    body,
                    vec![("join", t.join), ("decline", t.decline)],
                    false,
                );
            }
            AppEvent::Text {
                session,
                sender: Some(sender),
                text,
                ..
            } if !self.ui_seen()
                && !self
                    .ui
                    .messages_quiet
                    .load(std::sync::atomic::Ordering::Relaxed) =>
            {
                let session = *session;
                // One notification per conversation: the newest message replaces the last.
                self.close_where(|a| matches!(a, About::Text(s) if *s == session));
                self.show(
                    About::Text(session),
                    short(sender),
                    text.clone(),
                    vec![(DEFAULT_ACTION, t.open)],
                    false,
                );
            }
            _ => {}
        }
    }

    fn ring(&mut self, id: u32, peer: &str, video: bool, screen: bool) {
        let t = self.t;
        let what = if screen {
            t.screen_offered
        } else if video {
            t.incoming_video_call
        } else {
            t.incoming_call
        };
        let mut actions = Vec::new();
        if screen {
            actions.push(("screen", t.view_screen));
        } else {
            actions.push(("answer", t.answer));
            if video {
                actions.push(("video", t.answer_video));
            }
        }
        actions.push(("decline", t.decline));
        self.show(
            About::Call(id),
            short(peer),
            what.to_string(),
            actions,
            true,
        );
    }

    /// The call's kind became known after it started ringing: show it with the right buttons.
    fn reshow_call(&mut self, id: u32) {
        let shown = self
            .shown
            .values()
            .any(|a| matches!(a, About::Call(c) if *c == id));
        let Some(summary) = self.callers.get(&id).cloned().filter(|_| shown) else {
            return;
        };
        let (video, screen) = self.kinds.get(&id).copied().unwrap_or((false, false));
        self.close_where(|a| matches!(a, About::Call(c) if *c == id));
        self.ring(id, &summary, video, screen);
    }

    /// The actions chosen since the last call.
    pub fn chosen(&mut self) -> Vec<Chosen> {
        let replies = self.notifier.replies();
        let mut out = Vec::new();
        for (tag, action) in replies {
            let Some(about) = self.shown.get(&tag).cloned() else {
                continue;
            };
            if action == "closed" {
                self.shown.remove(&tag);
                continue;
            }
            let c = match (about, action.as_str()) {
                (About::Call(id), "answer") => Some(Chosen::AnswerCall(id)),
                (About::Call(id), "video") => Some(Chosen::AnswerVideo(id)),
                (About::Call(id), "screen") => Some(Chosen::ViewScreen(id)),
                (About::Call(id), "decline") => Some(Chosen::HangUp(id)),
                (About::Invitation(focus), "join") => Some(Chosen::Join(focus)),
                (About::Text(s), DEFAULT_ACTION) => Some(Chosen::Open(s)),
                _ => None,
            };
            self.shown.remove(&tag);
            out.extend(c);
        }
        out
    }
}

/// Milliseconds since the Unix epoch (for the last frame of the interface).
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The action of clicking a notification itself (Desktop Notifications Specification).
pub const DEFAULT_ACTION: &str = "default";
