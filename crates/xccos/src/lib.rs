//! MS-XCCOSIP: the persistent chat (chat room) protocol carried in SIP INFO bodies
//! (function 20). Sans I/O: commands out as XML bodies, received bodies in as events.
//!
//! Evidence: `docs/protocol/evidence/32-xccosip-persistent-chat.md` (XC-01 … XC-07). Sources:
//! [MS-XCCOSIP] 20240820 – envelope §2.2.2.2.1 (p. 35), commands §2.2.2.2.2, replies
//! §2.2.2.2.4 (p. 38), errors §2.2.2.2.6 (p. 41), chat §2.2.2.2.9 (p. 42), transactions
//! §3.1.2.1 (pp. 44–45), the client flows §3.1.3–3.1.14 (pp. 46–56) and the examples §4
//! (pp. 73–76). Skype for Business Server 2019 no longer offers persistent chat, so this is
//! `tested-offline` only.

use commscraft_xml::{escape, parse, Element, Limits};
use std::collections::BTreeMap;

/// The XCCOS namespace (§2.2.1, p. 14).
pub const NS: &str = "urn:parlano:xml:ns:xccos";
/// The transaction timer of a command (§3.1.2.1.2, p. 44).
pub const TRANSACTION_MS: u64 = 10_000;
/// The server information the client asks for (§3.1.4.3, p. 46: serverTime, searchLimit,
/// pingInterval, rootUri, messageSizeLimit, storySizeLimit, serverVersion, displayName,
/// roomManagementUrl; bits of §2.2.2.1.10, p. 23).
pub const INFO_TYPE: u32 = 1 | 2 | 4 | 16 | 32 | 64 | 128 | 512 | 1024;
/// Message size when the server names none (the example's messageSizeLimit, p. 73).
pub const DEFAULT_MESSAGE_LIMIT: usize = 8000;
/// Most pending commands (design).
pub const MAX_PENDING: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Xml(commscraft_xml::Error),
    /// Not an xccos envelope.
    WrongDocument,
    BadValue(&'static str),
    TooManyPending,
}

fn limits() -> Limits {
    Limits {
        max_input: 1024 * 1024,
        max_depth: 16,
        max_nodes: 20_000,
        ..Limits::default()
    }
}

/// The commands the client sends (§2.2.2.2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// cmd:requri – the channel server to use (§3.1.3, p. 46).
    RequestUri,
    /// cmd:getserverinfo (§3.1.4, p. 46).
    ServerInfo,
    /// cmd:getassociations – rooms the user is a member (or manager) of (§3.1.11, p. 53).
    Associations { manager: bool, max: u32 },
    /// cmd:bjoin – join several rooms by GUID (§3.1.6; example §4.2, p. 73).
    BatchJoin { rooms: Vec<String> },
    /// cmd:bccontext – the last messages of a room (§3.1.7; example §4.3, p. 74).
    History { room: String, last: u32 },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::RequestUri => "requri",
            Command::ServerInfo => "getserverinfo",
            Command::Associations { .. } => "getassociations",
            Command::BatchJoin { .. } => "bjoin",
            Command::History { .. } => "bccontext",
        }
    }
}

/// A room as the server describes it (ChannelInformationDataBlock, §2.2.2.1.7, p. 18).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Room {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub topic: String,
    pub participants: Vec<String>,
    pub messages: Vec<ChatMessage>,
}

/// A message in a room (GroupChatDataBlock, §2.2.2.2.9, p. 42).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatMessage {
    pub room: String,
    pub author: String,
    pub author_name: String,
    pub chat_id: u64,
    pub time: String,
    pub text: String,
    pub alert: bool,
    /// Our message coming back (originatingMessageId, §3.1.13, p. 55).
    pub ours: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The pool named the channel server to talk to; end this dialog and open one there
    /// (§3.1.3.5, p. 46).
    ChannelServer(String),
    /// The server's display name, room management URL and message size limit.
    ServerInfo {
        name: String,
        management_url: String,
        message_limit: usize,
    },
    Rooms(Vec<Room>),
    Joined(Vec<Room>),
    History(Vec<Room>),
    Message(ChatMessage),
    /// A command failed (its name and the response code), or timed out (code 408).
    Failed {
        command: String,
        code: u16,
    },
}

struct Pending {
    command: &'static str,
    deadline: u64,
}

/// The client state of one persistent chat dialog.
pub struct Client {
    domain: String,
    aor: String,
    client_version: String,
    next_envid: u64,
    pending: BTreeMap<(u64, u64), Pending>,
    users: BTreeMap<String, String>,
    message_limit: usize,
    last_chat_id: BTreeMap<String, u64>,
    /// The last bjoin answered: its rpl:bccontext follows under the same commandid.
    answered_bjoin: Option<(u64, u64)>,
}

fn dns_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 253
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// The GUID of a room URI `ma-chan://<domain>/<guid>` (§2.2.2.1.9, p. 23; example p. 73).
pub fn room_guid(uri: &str) -> Option<&str> {
    let guid = uri.strip_prefix("ma-chan://")?.split_once('/')?.1;
    let ok = !guid.is_empty()
        && guid.len() <= 64
        && guid.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    ok.then_some(guid)
}

fn attr(e: &Element, name: &str) -> String {
    e.attr(name).unwrap_or_default().to_string()
}

impl Client {
    /// `envid_base` is a random start: envids then grow by one (XC-02, "monotonic").
    pub fn new(
        aor: &str,
        domain: &str,
        client_version: &str,
        envid_base: u64,
    ) -> Result<Self, Error> {
        if !dns_ok(domain) {
            return Err(Error::BadValue("domain"));
        }
        if !aor.starts_with("sip:") || aor.len() > 256 {
            return Err(Error::BadValue("aor"));
        }
        Ok(Self {
            domain: domain.to_string(),
            aor: aor.to_string(),
            client_version: client_version.to_string(),
            next_envid: envid_base >> 1,
            pending: BTreeMap::new(),
            users: BTreeMap::new(),
            message_limit: DEFAULT_MESSAGE_LIMIT,
            last_chat_id: BTreeMap::new(),
            answered_bjoin: None,
        })
    }

    fn envid(&mut self) -> u64 {
        self.next_envid = self.next_envid.wrapping_add(1);
        self.next_envid
    }

    /// The body of a command in its own envelope, seqid 1; the command is pending until its
    /// reply or the transaction timer (§3.1.2.1.3, p. 45).
    pub fn command(&mut self, cmd: &Command, now_ms: u64) -> Result<String, Error> {
        if self.pending.len() >= MAX_PENDING {
            return Err(Error::TooManyPending);
        }
        let d = escape(&self.domain);
        let data = match cmd {
            Command::RequestUri => String::new(),
            Command::ServerInfo => format!(
                "<sib domain=\"{d}\" infoType=\"{INFO_TYPE}\" clientVersion=\"{}\" />",
                escape(&self.client_version)
            ),
            Command::Associations { manager, max } => format!(
                "<association domain=\"{d}\" type=\"{}\" maxResult=\"{max}\" />",
                if *manager { "MANAGER" } else { "MEMBER" }
            ),
            Command::BatchJoin { rooms } => {
                if rooms.is_empty()
                    || rooms.len() > 100
                    || rooms
                        .iter()
                        .any(|g| room_guid(&format!("ma-chan://x/{g}")).is_none())
                {
                    return Err(Error::BadValue("room"));
                }
                // key "Currently not used … SHOULD be 0" (§2.2.2.1.9, p. 23).
                format!(
                    "<chanid key=\"0\" value=\"{}\" domain=\"{d}\" />",
                    rooms.join(",")
                )
            }
            Command::History { room, last } => {
                if room_guid(room).is_none() {
                    return Err(Error::BadValue("room"));
                }
                format!(
                    "<chanib uri=\"{}\" overridemembers=\"false\" behavior=\"UNSET\" disabled=\"false\" /><bcq><last cnt=\"{}\" /></bcq>",
                    escape(room),
                    (*last).min(1000)
                )
            }
        };
        let envid = self.envid();
        self.pending.insert(
            (envid, 1),
            Pending {
                command: cmd.id(),
                deadline: now_ms.saturating_add(TRANSACTION_MS),
            },
        );
        Ok(format!(
            "<xccos ver=\"1\" envid=\"{envid}\" xmlns=\"{NS}\"><cmd id=\"cmd:{}\" seqid=\"1\"><data>{data}</data></cmd></xccos>",
            cmd.id()
        ))
    }

    /// The body of a chat message: chatId 0 and an empty authdisp when sending, the
    /// originatingMessageId naming the envelope (§3.1.13, p. 55; example §4.6, p. 76).
    pub fn chat(&mut self, room: &str, text: &str, time: &str) -> Result<String, Error> {
        if room_guid(room).is_none() {
            return Err(Error::BadValue("room"));
        }
        if text.is_empty() || text.len() > self.message_limit {
            return Err(Error::BadValue("text"));
        }
        let envid = self.envid();
        Ok(format!(
            "<xccos ver=\"1\" envid=\"{envid}\" xmlns=\"{NS}\"><grpchat id=\"grpchat\" seqid=\"1\" chanUri=\"{}\" author=\"{}\" authdisp=\"\" alert=\"false\" chatId=\"0\" ts=\"{}\"><originatingMessageId seqid=\"1\" envid=\"{envid}\" /><chat>{}</chat></grpchat></xccos>",
            escape(room),
            escape(&self.aor),
            escape(time),
            escape(text)
        ))
    }

    /// Commands whose transaction timer ran out (§3.1.2.1.6, p. 45).
    pub fn tick(&mut self, now_ms: u64) -> Vec<Event> {
        let late: Vec<(u64, u64)> = self
            .pending
            .iter()
            .filter(|(_, p)| p.deadline <= now_ms)
            .map(|(k, _)| *k)
            .collect();
        late.into_iter()
            .filter_map(|k| self.pending.remove(&k))
            .map(|p| Event::Failed {
                command: p.command.to_string(),
                code: 408,
            })
            .collect()
    }

    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// A received INFO body.
    pub fn on_body(&mut self, body: &str) -> Result<Vec<Event>, Error> {
        let root = parse(body, &limits()).map_err(Error::Xml)?;
        if root.name != "xccos" || root.namespace.as_deref() != Some(NS) {
            return Err(Error::WrongDocument);
        }
        let mut events = Vec::new();
        for p in root.elements() {
            match p.name.as_str() {
                "rpl" | "err" => self.on_reply(p, &mut events),
                "grpchat" => {
                    if let Some(m) = self.message(p) {
                        events.push(Event::Message(m));
                    }
                }
                // Notices, system messages: nothing the client shows yet.
                _ => {}
            }
        }
        Ok(events)
    }

    /// The matching rules of §3.1.2.1.5 (p. 45): commandid with seqid and envid of a pending
    /// command, a response code, and a reply id that fits the command. The bjoin reply comes
    /// with rpl:bccontext under the same commandid (example §4.2, p. 74), taken as history.
    fn on_reply(&mut self, p: &Element, events: &mut Vec<Event>) {
        let Some(cid) = p.child("commandid") else {
            return;
        };
        let (Some(seq), Some(env)) = (
            cid.attr("seqid").and_then(|v| v.trim().parse::<u64>().ok()),
            cid.attr("envid").and_then(|v| v.trim().parse::<u64>().ok()),
        ) else {
            return;
        };
        let Some(code) = p
            .child("resp")
            .and_then(|r| r.attr("code"))
            .and_then(|c| c.trim().parse::<u16>().ok())
        else {
            return;
        };
        let id = p.attr("id").unwrap_or_default();
        let command = match self.pending.get(&(env, seq)) {
            Some(x) => x.command,
            None if self.answered_bjoin == Some((env, seq)) && id == "rpl:bccontext" => "bjoin",
            None => return,
        };
        let extra_history = command == "bjoin" && id == "rpl:bccontext";
        if p.name == "rpl" && id.strip_prefix("rpl:") != Some(command) && !extra_history {
            return;
        }
        if !extra_history {
            self.pending.remove(&(env, seq));
            if command == "bjoin" {
                self.answered_bjoin = Some((env, seq));
            }
        }
        if p.name == "err" || !(200..300).contains(&code) {
            events.push(Event::Failed {
                command: command.to_string(),
                code,
            });
            return;
        }
        let data = p.child("data");
        if extra_history {
            events.push(Event::History(self.rooms(data)));
            return;
        }
        match command {
            "requri" => {
                if let Some(uri) = data
                    .and_then(|d| d.child("uib"))
                    .and_then(|u| u.attr("uri"))
                    .filter(|u| u.starts_with("sip:") && u.len() <= 512)
                {
                    events.push(Event::ChannelServer(uri.to_string()));
                }
            }
            "getserverinfo" => {
                let sib = data.and_then(|d| d.child("sib"));
                if let Some(limit) = sib
                    .and_then(|s| s.attr("messageSizeLimit"))
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .filter(|l| (1..=1_000_000).contains(l))
                {
                    self.message_limit = limit;
                }
                events.push(Event::ServerInfo {
                    name: sib.map(|s| attr(s, "displayName")).unwrap_or_default(),
                    management_url: sib
                        .map(|s| attr(s, "roomManagementUrl"))
                        .unwrap_or_default(),
                    message_limit: self.message_limit,
                });
            }
            // The rooms come in an association element (§2.2.2.1.20, p. 30), or directly.
            "getassociations" => {
                let holder = data.and_then(|d| d.child("association")).or(data);
                events.push(Event::Rooms(self.rooms(holder)));
            }
            "bjoin" => events.push(Event::Joined(self.rooms(data))),
            "bccontext" => events.push(Event::History(self.rooms(data))),
            _ => {}
        }
    }

    /// Rooms of a reply: users (uib id → uri) first, then each chanib with its participants
    /// from the aib value (indexes into the users; §3.1.5.5, p. 47) and its messages.
    fn rooms(&mut self, data: Option<&Element>) -> Vec<Room> {
        let Some(data) = data else {
            return Vec::new();
        };
        for u in data.children_named("uib").take(10_000) {
            if let (Some(id), Some(uri)) = (u.attr("id"), u.attr("uri")) {
                if self.users.len() < 10_000 {
                    self.users.insert(id.to_string(), uri.to_string());
                }
            }
        }
        let mut rooms = Vec::new();
        for c in data.children_named("chanib").take(1000) {
            let mut room = Room {
                uri: attr(c, "uri"),
                name: attr(c, "name"),
                description: attr(c, "description"),
                topic: attr(c, "topic"),
                ..Room::default()
            };
            for a in c.children_named("aib") {
                for idx in attr(a, "value").split(',') {
                    if let Some(u) = self.users.get(idx.trim()) {
                        if !room.participants.contains(u) {
                            room.participants.push(u.clone());
                        }
                    }
                }
            }
            for m in c.children_named("msg").take(1000) {
                if let Some(msg) = self.message(m) {
                    room.messages.push(msg);
                }
            }
            rooms.push(room);
        }
        rooms
    }

    /// A group chat message; the LastReceivedMessageId of its room grows (XC-07; §3.1.14.5,
    /// p. 56).
    fn message(&mut self, m: &Element) -> Option<ChatMessage> {
        let room = attr(m, "chanUri");
        room_guid(&room)?;
        let chat_id = m
            .attr("chatId")
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(0);
        if self.last_chat_id.len() < 10_000 || self.last_chat_id.contains_key(&room) {
            let last = self.last_chat_id.entry(room.clone()).or_insert(0);
            *last = (*last).max(chat_id);
        }
        Some(ChatMessage {
            room,
            author: attr(m, "author"),
            author_name: attr(m, "authdisp"),
            chat_id,
            time: attr(m, "ts"),
            text: m.child("chat").map(|c| c.text()).unwrap_or_default(),
            alert: m.attr("alert") == Some("true"),
            ours: m.child("originatingMessageId").is_some(),
        })
    }

    /// The largest chatId seen in a room.
    pub fn last_chat_id(&self, room: &str) -> Option<u64> {
        self.last_chat_id.get(room).copied()
    }
}

#[cfg(test)]
mod tests;
