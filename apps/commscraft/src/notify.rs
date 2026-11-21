//! Desktop notifications over the session bus: the Desktop Notifications Specification 1.2
//! (`org.freedesktop.Notifications`: `Notify`, `CloseNotification`, the signals `ActionInvoked`
//! and `NotificationClosed`) – the standard notification service of Linux desktops, which a
//! Flatpak reaches with `--talk-name=org.freedesktop.Notifications`. In place of a notification
//! area icon (the GNOME desktops in use have no StatusNotifierItem host).
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
    tx: Sender<Request>,
    pub replies: Receiver<Reply>,
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
        Some(Notifier { tx, replies })
    }

    pub fn show(&self, s: Show) {
        let _ = self.tx.send(Request::Show(s));
    }

    pub fn close(&self, tag: u64) {
        let _ = self.tx.send(Request::Close(tag));
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
