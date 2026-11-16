//! The interface's state and the typed events that change it.
//!
//! Adapters (the demo fixtures, the session worker) never touch widgets: they produce
//! [`AppEvent`]s, and [`AppState::apply`] is the only place that changes the state. Everything
//! from the network was already parsed and bounded by the protocol crates; the state bounds it
//! again (lengths, counts) and strips control and invisible characters before it is shown.

use std::collections::{BTreeMap, VecDeque};

/// Messages kept per conversation (older ones are dropped, oldest first).
pub const MAX_MESSAGES: usize = 500;
/// Conversations kept (ended ones are dropped first).
pub const MAX_CONVERSATIONS: usize = 200;
/// Characters of one message shown; the rest is cut and marked.
pub const MAX_TEXT: usize = 8 * 1024;
/// Characters of a name, title or address shown.
pub const MAX_LABEL: usize = 256;
/// Contacts kept.
pub const MAX_CONTACTS: usize = 10_000;
/// Calls kept in the history.
pub const MAX_CALL_HISTORY: usize = 200;
/// Notices kept for the status bar.
pub const MAX_NOTICES: usize = 50;

/// Removes control characters (except line breaks in messages), bidirectional overrides and
/// other invisible format characters, and cuts the text at `max` characters.
pub fn clean(text: &str, max: usize, keep_newlines: bool) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    for c in text.chars() {
        let invisible = matches!(c,
            '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2069}'
            | '\u{FEFF}' | '\u{FFF9}'..='\u{FFFB}');
        if invisible || (c.is_control() && !(keep_newlines && c == '\n')) {
            if c == '\t' {
                out.push(' ');
                count += 1;
            }
            continue;
        }
        if count >= max {
            out.push('…');
            break;
        }
        out.push(c);
        count += 1;
    }
    out
}

fn label(text: &str) -> String {
    clean(text, MAX_LABEL, false)
}

/// The rail's views (no alarm or situation view: out of scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Chats,
    Contacts,
    Calls,
    Meetings,
    Settings,
}

/// The connection as the adapter reports it. `Demo` means no server at all.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Connection {
    #[default]
    SignedOut,
    Demo,
    Connecting,
    Connected,
    /// Signed in, but something does not work (a service, the keep-alive, …).
    Degraded(String),
    Disconnected(String),
    Failed(String),
}

impl Connection {
    pub fn is_online(&self) -> bool {
        matches!(self, Connection::Connected | Connection::Degraded(_))
    }
}

/// Whether a function can be used, for this user and server, right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    /// Supported by the current code and enabled.
    Available,
    /// Only a design preview with demo data; it never has a real effect.
    Preview,
    /// Disabled, with the reason shown to the user.
    Unavailable(Reason),
}

impl Gate {
    pub fn usable(&self) -> bool {
        matches!(self, Gate::Available | Gate::Preview)
    }
}

/// Why a function is unavailable (translated in [`crate::text`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    NotConnected,
    NotImplemented,
    /// The protocol core has it, the desktop interface does not offer it yet (the command-line
    /// tool does).
    CliOnly,
    ServerDisabled,
    NoDevice,
    DemoOnly,
}

/// The capability matrix: one gate per function the interface shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    pub chat: Gate,
    pub group_chat: Gate,
    pub presence: Gate,
    pub directory_search: Gate,
    pub audio_call: Gate,
    pub hold: Gate,
    pub transfer: Gate,
    pub video_call: Gate,
    pub meetings: Gate,
    pub file_transfer: Gate,
    pub rich_text: Gate,
    pub persistent_chat: Gate,
    pub history_search: Gate,
    pub read_receipts: Gate,
}

impl Capabilities {
    /// Nothing works without a connection; what is not built is unavailable either way.
    pub fn offline() -> Self {
        let off = Gate::Unavailable(Reason::NotConnected);
        Self {
            chat: off.clone(),
            group_chat: off.clone(),
            presence: off.clone(),
            directory_search: off.clone(),
            audio_call: off.clone(),
            hold: off.clone(),
            transfer: off.clone(),
            video_call: off.clone(),
            meetings: off,
            ..Self::never()
        }
    }

    /// The demo: the functions the code has are previews, the rest stays unavailable.
    pub fn demo() -> Self {
        Self {
            chat: Gate::Preview,
            group_chat: Gate::Preview,
            presence: Gate::Preview,
            directory_search: Gate::Preview,
            audio_call: Gate::Preview,
            hold: Gate::Preview,
            transfer: Gate::Unavailable(Reason::DemoOnly),
            video_call: Gate::Unavailable(Reason::CliOnly),
            meetings: Gate::Preview,
            ..Self::never()
        }
    }

    /// Functions the project does not have (file transfer, rich text, history search, read
    /// receipts) or that the desktop interface does not offer (persistent chat rooms).
    fn never() -> Self {
        let none = Gate::Unavailable(Reason::NotImplemented);
        Self {
            chat: none.clone(),
            group_chat: none.clone(),
            presence: none.clone(),
            directory_search: none.clone(),
            audio_call: none.clone(),
            hold: none.clone(),
            transfer: none.clone(),
            video_call: none.clone(),
            meetings: none.clone(),
            file_transfer: none.clone(),
            rich_text: none.clone(),
            persistent_chat: Gate::Unavailable(Reason::CliOnly),
            history_search: none.clone(),
            read_receipts: none,
        }
    }
}

/// Presence as shown: a word and a colour, never the colour alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
pub enum Presence {
    Available,
    Busy,
    InACall,
    DoNotDisturb,
    BeRightBack,
    Away,
    Offline,
    #[default]
    Unknown,
}

/// Where a contact entry comes from (shown with search results).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    ContactList,
    AddressBook,
    Directory,
    Demo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    pub uri: String,
    pub name: String,
    pub title: String,
    pub department: String,
    pub company: String,
    pub phone: String,
    pub presence: Presence,
    pub note: String,
    pub group: String,
    pub source: Source,
}

impl Contact {
    pub fn new(uri: &str, name: &str, source: Source) -> Self {
        Self {
            uri: label(uri),
            name: label(name),
            title: String::new(),
            department: String::new(),
            company: String::new(),
            phone: String::new(),
            presence: Presence::Unknown,
            note: String::new(),
            group: String::new(),
            source,
        }
    }

    /// The name, or the address without `sip:` when there is none.
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            self.uri.strip_prefix("sip:").unwrap_or(&self.uri)
        } else {
            &self.name
        }
    }

    fn cleaned(mut self) -> Self {
        self.uri = label(&self.uri);
        self.name = label(&self.name);
        self.title = label(&self.title);
        self.department = label(&self.department);
        self.company = label(&self.company);
        self.phone = label(&self.phone);
        self.note = clean(&self.note, 1024, false);
        self.group = label(&self.group);
        self
    }
}

/// The technical delivery state of an own message. Delivered is never "read".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    /// Handed to the session, no answer yet.
    Sending,
    /// The server or peer accepted it (a 200 to the MESSAGE).
    Delivered,
    /// Refused with this status.
    Failed(u16),
    /// In a group: these recipients did not get it (IMDN).
    PartlyFailed(Vec<(String, u16)>),
    /// Demo data or no answer expected.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// `None`: our own message.
    pub sender: Option<String>,
    pub text: String,
    /// Unix seconds.
    pub at: u64,
    pub delivery: Delivery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConvKey(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConvState {
    /// Opening (INVITE sent); messages wait in the session.
    Opening,
    Open,
    Ended,
    Failed(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    pub uri: String,
    pub name: String,
    pub presenter: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversation {
    pub key: ConvKey,
    /// The session's conversation ID once it is known.
    pub session: Option<u32>,
    pub title: String,
    /// One peer, or `None` for a group.
    pub peer: Option<String>,
    pub participants: Vec<Participant>,
    pub messages: VecDeque<Message>,
    pub unread: usize,
    pub typing: bool,
    pub state: ConvState,
    pub last_activity: u64,
    /// Set by the demo for illustrative content (shown with a demo mark).
    pub demo: bool,
}

impl Conversation {
    pub fn is_group(&self) -> bool {
        self.peer.is_none()
    }

    pub fn last_text(&self) -> Option<&Message> {
        self.messages.back()
    }

    fn push(&mut self, m: Message) {
        self.last_activity = self.last_activity.max(m.at);
        self.messages.push_back(m);
        while self.messages.len() > MAX_MESSAGES {
            self.messages.pop_front();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallState {
    /// INVITE sent, no answer yet.
    Dialing,
    /// The peer's phone rings (180).
    Ringing,
    /// Someone calls us.
    Incoming,
    /// Media negotiated (`CallAnswered`): since this time.
    Connected {
        since: u64,
    },
    Ended {
        status: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub id: u32,
    pub peer: String,
    pub incoming: bool,
    pub state: CallState,
    pub muted: bool,
    pub held: bool,
    pub started: u64,
    /// The conference this call belongs to.
    pub conference: Option<ConvKey>,
    pub demo: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection {
    Incoming,
    Outgoing,
    Missed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallRecord {
    pub peer: String,
    pub direction: CallDirection,
    pub at: u64,
    pub seconds: u64,
    pub status: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meeting {
    pub subject: String,
    pub focus: String,
    pub conference_id: String,
    pub expires: Option<u64>,
    pub demo: bool,
}

/// A server outside the user's SIP domain, named over verified HTTPS by `named_by`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustQuestion {
    pub host: String,
    /// The domain that would be trusted from now on.
    pub domain: String,
    pub user_domain: String,
    pub named_by: String,
}

/// An invitation to a group conversation or meeting; joining is the user's decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invitation {
    pub from: String,
    pub focus: String,
    pub subject: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub text: String,
    pub at: u64,
}

/// Our own identity once signed in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Profile {
    pub uri: String,
    pub name: String,
    pub presence: Presence,
    pub organisation: String,
}

/// Typed changes from an adapter. Session IDs are the engine's conversation and call IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Connection(Connection),
    Capabilities(Capabilities),
    SignedIn(Profile),
    OwnPresence(Presence),
    /// The contact list (replaces the list's members, keeps other sources).
    Contacts(Vec<Contact>),
    Presence {
        uri: String,
        /// `None`: the notification carried no state (only a note or a name): keep the state.
        presence: Option<Presence>,
        note: String,
        /// The display name the contact publishes (fills an empty name of the contact list).
        name: Option<String>,
    },
    /// Results of a directory or address book search (shown separately, not kept as contacts).
    SearchResults(Vec<Contact>),
    /// We opened a conversation; the session names it later.
    ConversationOpening {
        key: ConvKey,
        peer: String,
    },
    ConversationStarted {
        session: u32,
        peer: String,
        incoming: bool,
    },
    /// A group conversation with this session ID (ours or one we joined).
    GroupStarted {
        session: u32,
        title: String,
    },
    ConversationFailed {
        session: u32,
        status: u16,
    },
    /// An opening conversation failed before the session named it.
    OpeningFailed {
        key: ConvKey,
        status: u16,
    },
    Participants {
        session: u32,
        participants: Vec<Participant>,
    },
    Text {
        session: u32,
        sender: Option<String>,
        text: String,
        at: u64,
    },
    /// Our own message, shown at once with `Sending`.
    OwnText {
        key: ConvKey,
        text: String,
        at: u64,
    },
    Delivered {
        session: u32,
    },
    NotDelivered {
        session: u32,
        status: u16,
    },
    NotDeliveredTo {
        session: u32,
        recipient: String,
        status: u16,
    },
    Typing {
        session: u32,
        typing: bool,
    },
    ConversationEnded {
        session: u32,
    },
    CallStarted {
        id: u32,
        peer: String,
        incoming: bool,
        at: u64,
    },
    CallRinging {
        id: u32,
    },
    CallConnected {
        id: u32,
        at: u64,
    },
    CallHeld {
        id: u32,
        held: bool,
    },
    CallMuted {
        id: u32,
        muted: bool,
    },
    CallEnded {
        id: u32,
        status: u16,
        at: u64,
    },
    Meetings(Vec<Meeting>),
    Invitation(Invitation),
    /// Discovery named a server outside the SIP domain: may the password go there?
    TrustQuestion(TrustQuestion),
    Notice(NoticeKind, String, u64),
}

/// Everything the interface shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub connection: Connection,
    pub capabilities: Capabilities,
    pub me: Profile,
    pub contacts: BTreeMap<String, Contact>,
    pub search_results: Vec<Contact>,
    pub conversations: Vec<Conversation>,
    pub calls: Vec<Call>,
    pub call_history: VecDeque<CallRecord>,
    pub meetings: Vec<Meeting>,
    pub invitations: Vec<Invitation>,
    pub notices: VecDeque<Notice>,
    /// A trust question waiting for the user's answer.
    pub trust_question: Option<TrustQuestion>,
    next_key: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connection: Connection::SignedOut,
            capabilities: Capabilities::offline(),
            me: Profile::default(),
            contacts: BTreeMap::new(),
            search_results: Vec::new(),
            conversations: Vec::new(),
            calls: Vec::new(),
            call_history: VecDeque::new(),
            meetings: Vec::new(),
            invitations: Vec::new(),
            notices: VecDeque::new(),
            trust_question: None,
            next_key: 1,
        }
    }
}

impl AppState {
    /// A new local conversation key.
    pub fn new_key(&mut self) -> ConvKey {
        let k = ConvKey(self.next_key);
        self.next_key = self.next_key.wrapping_add(1).max(1);
        k
    }

    pub fn conversation(&self, key: ConvKey) -> Option<&Conversation> {
        self.conversations.iter().find(|c| c.key == key)
    }

    pub fn conversation_mut(&mut self, key: ConvKey) -> Option<&mut Conversation> {
        self.conversations.iter_mut().find(|c| c.key == key)
    }

    fn by_session(&mut self, session: u32) -> Option<&mut Conversation> {
        self.conversations
            .iter_mut()
            .find(|c| c.session == Some(session))
    }

    pub fn key_of_session(&self, session: u32) -> Option<ConvKey> {
        self.conversations
            .iter()
            .find(|c| c.session == Some(session))
            .map(|c| c.key)
    }

    /// The name of an address: from the contacts, else the address without `sip:`.
    pub fn name_of(&self, uri: &str) -> String {
        if uri == self.me.uri && !self.me.name.is_empty() {
            return self.me.name.clone();
        }
        match self.contacts.get(uri) {
            Some(c) => c.display_name().to_string(),
            None => uri.strip_prefix("sip:").unwrap_or(uri).to_string(),
        }
    }

    /// The open conversation with one peer, if any.
    pub fn conversation_with(&self, peer: &str) -> Option<ConvKey> {
        self.conversations
            .iter()
            .find(|c| {
                c.peer.as_deref() == Some(peer)
                    && matches!(c.state, ConvState::Open | ConvState::Opening)
            })
            .map(|c| c.key)
    }

    pub fn active_call(&self) -> Option<&Call> {
        self.calls
            .iter()
            .find(|c| !matches!(c.state, CallState::Ended { .. }))
    }

    pub fn unread_total(&self) -> usize {
        self.conversations.iter().map(|c| c.unread).sum()
    }

    /// Adds a conversation (dropping the oldest ended one when full) and returns its key.
    fn new_conversation(
        &mut self,
        key: Option<ConvKey>,
        peer: Option<String>,
        title: String,
        session: Option<u32>,
    ) -> ConvKey {
        if self.conversations.len() >= MAX_CONVERSATIONS {
            let drop = self
                .conversations
                .iter()
                .enumerate()
                .filter(|(_, c)| matches!(c.state, ConvState::Ended | ConvState::Failed(_)))
                .min_by_key(|(_, c)| c.last_activity)
                .or_else(|| {
                    self.conversations
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, c)| c.last_activity)
                })
                .map(|(i, _)| i);
            if let Some(i) = drop {
                self.conversations.remove(i);
            }
        }
        let key = key.unwrap_or_else(|| self.new_key());
        self.conversations.push(Conversation {
            key,
            session,
            title,
            peer,
            participants: Vec::new(),
            messages: VecDeque::new(),
            unread: 0,
            typing: false,
            state: if session.is_some() {
                ConvState::Open
            } else {
                ConvState::Opening
            },
            last_activity: 0,
            demo: false,
        });
        key
    }

    fn notice(&mut self, kind: NoticeKind, text: &str, at: u64) {
        self.notices.push_back(Notice {
            kind,
            text: clean(text, 512, false),
            at,
        });
        while self.notices.len() > MAX_NOTICES {
            self.notices.pop_front();
        }
    }

    fn end_call(&mut self, id: u32, status: u16, at: u64) {
        let Some(c) = self.calls.iter_mut().find(|c| c.id == id) else {
            return;
        };
        let (direction, seconds) = match c.state {
            CallState::Connected { since } => (
                if c.incoming {
                    CallDirection::Incoming
                } else {
                    CallDirection::Outgoing
                },
                at.saturating_sub(since),
            ),
            _ if c.incoming => (CallDirection::Missed, 0),
            _ => (CallDirection::Outgoing, 0),
        };
        c.state = CallState::Ended { status };
        let record = CallRecord {
            peer: c.peer.clone(),
            direction,
            at: c.started,
            seconds,
            status,
        };
        self.calls.retain(|c| c.id != id);
        self.call_history.push_front(record);
        self.call_history.truncate(MAX_CALL_HISTORY);
    }

    /// Applies one event. Unknown session IDs are ignored (a late event after the end).
    pub fn apply(&mut self, e: AppEvent) {
        match e {
            AppEvent::Connection(c) => {
                if !c.is_online() && c != Connection::Demo {
                    // Without a connection nothing is open any more.
                    for conv in &mut self.conversations {
                        if matches!(conv.state, ConvState::Open | ConvState::Opening) {
                            conv.state = ConvState::Ended;
                        }
                        conv.typing = false;
                    }
                    let ids: Vec<u32> = self.calls.iter().map(|c| c.id).collect();
                    for id in ids {
                        self.end_call(id, 0, 0);
                    }
                    for contact in self.contacts.values_mut() {
                        contact.presence = Presence::Unknown;
                    }
                }
                self.connection = c;
            }
            AppEvent::Capabilities(c) => self.capabilities = c,
            AppEvent::SignedIn(p) => {
                self.me = Profile {
                    uri: label(&p.uri),
                    name: label(&p.name),
                    presence: p.presence,
                    organisation: label(&p.organisation),
                }
            }
            AppEvent::OwnPresence(p) => self.me.presence = p,
            AppEvent::Contacts(list) => {
                let mut presence: BTreeMap<String, (Presence, String)> = BTreeMap::new();
                for (uri, c) in &self.contacts {
                    presence.insert(uri.clone(), (c.presence, c.note.clone()));
                }
                self.contacts.retain(|_, c| c.source != Source::ContactList);
                for c in list.into_iter().take(MAX_CONTACTS) {
                    let mut c = c.cleaned();
                    if c.presence == Presence::Unknown {
                        if let Some((p, n)) = presence.get(&c.uri) {
                            c.presence = *p;
                            c.note.clone_from(n);
                        }
                    }
                    self.contacts.insert(c.uri.clone(), c);
                }
            }
            AppEvent::Presence {
                uri,
                presence,
                note,
                name,
            } => {
                if let Some(c) = self.contacts.get_mut(&uri) {
                    if let Some(p) = presence {
                        c.presence = p;
                    }
                    c.note = clean(&note, 1024, false);
                    if let Some(n) = name.filter(|n| !n.trim().is_empty()) {
                        if c.name.trim().is_empty() {
                            c.name = label(&n);
                            // Conversations titled with the bare address take the name.
                            let bare = uri.strip_prefix("sip:").unwrap_or(&uri).to_string();
                            for conv in &mut self.conversations {
                                if conv.peer.as_deref() == Some(uri.as_str()) && conv.title == bare
                                {
                                    conv.title = label(&n);
                                }
                            }
                        }
                    }
                }
                if let (true, Some(p)) = (uri == self.me.uri, presence) {
                    self.me.presence = p;
                }
            }
            AppEvent::SearchResults(r) => {
                self.search_results = r.into_iter().take(200).map(Contact::cleaned).collect();
            }
            AppEvent::ConversationOpening { key, peer } => {
                if self.conversation(key).is_some() {
                    return;
                }
                let title = self.name_of(&peer);
                self.new_conversation(Some(key), Some(label(&peer)), title, None);
            }
            AppEvent::ConversationStarted {
                session,
                peer,
                incoming,
            } => {
                let peer = label(&peer);
                if self.by_session(session).is_some() {
                    return;
                }
                // Our own opening conversation with this peer gets the session ID.
                if !incoming {
                    if let Some(c) = self.conversations.iter_mut().find(|c| {
                        c.session.is_none()
                            && c.state == ConvState::Opening
                            && c.peer.as_deref() == Some(peer.as_str())
                    }) {
                        c.session = Some(session);
                        c.state = ConvState::Open;
                        return;
                    }
                }
                let title = self.name_of(&peer);
                self.new_conversation(None, Some(peer), title, Some(session));
            }
            AppEvent::GroupStarted { session, title } => {
                if let Some(c) = self.by_session(session) {
                    c.peer = None;
                    if !title.is_empty() {
                        c.title = label(&title);
                    }
                    return;
                }
                self.new_conversation(None, None, label(&title), Some(session));
            }
            AppEvent::ConversationFailed { session, status } => {
                if let Some(c) = self.by_session(session) {
                    c.state = ConvState::Failed(status);
                    c.typing = false;
                    for m in c.messages.iter_mut() {
                        if m.delivery == Delivery::Sending {
                            m.delivery = Delivery::Failed(status);
                        }
                    }
                }
            }
            AppEvent::OpeningFailed { key, status } => {
                if let Some(c) = self.conversation_mut(key) {
                    c.state = ConvState::Failed(status);
                    for m in c.messages.iter_mut() {
                        if m.delivery == Delivery::Sending {
                            m.delivery = Delivery::Failed(status);
                        }
                    }
                }
            }
            AppEvent::Participants {
                session,
                participants,
            } => {
                if let Some(c) = self.by_session(session) {
                    c.participants = participants
                        .into_iter()
                        .take(1000)
                        .map(|p| Participant {
                            uri: label(&p.uri),
                            name: label(&p.name),
                            presenter: p.presenter,
                        })
                        .collect();
                }
            }
            AppEvent::Text {
                session,
                sender,
                text,
                at,
            } => {
                if let Some(c) = self.by_session(session) {
                    c.typing = false;
                    c.unread = c.unread.saturating_add(1);
                    c.push(Message {
                        sender: Some(label(
                            sender
                                .as_deref()
                                .unwrap_or_else(|| c.peer.as_deref().unwrap_or_default()),
                        )),
                        text: clean(&text, MAX_TEXT, true),
                        at,
                        delivery: Delivery::Unknown,
                    });
                }
            }
            AppEvent::OwnText { key, text, at } => {
                if let Some(c) = self.conversation_mut(key) {
                    c.push(Message {
                        sender: None,
                        text: clean(&text, MAX_TEXT, true),
                        at,
                        delivery: Delivery::Sending,
                    });
                }
            }
            AppEvent::Delivered { session } => {
                // The session answers the MESSAGEs in order: the oldest one still sending.
                if let Some(m) = self.by_session(session).and_then(|c| {
                    c.messages
                        .iter_mut()
                        .find(|m| m.sender.is_none() && m.delivery == Delivery::Sending)
                }) {
                    m.delivery = Delivery::Delivered;
                }
            }
            AppEvent::NotDelivered { session, status } => {
                if let Some(m) = self.by_session(session).and_then(|c| {
                    c.messages
                        .iter_mut()
                        .find(|m| m.sender.is_none() && m.delivery == Delivery::Sending)
                }) {
                    m.delivery = Delivery::Failed(status);
                }
            }
            AppEvent::NotDeliveredTo {
                session,
                recipient,
                status,
            } => {
                // IMDN refers to the last own message (CI-05).
                if let Some(m) = self
                    .by_session(session)
                    .and_then(|c| c.messages.iter_mut().rev().find(|m| m.sender.is_none()))
                {
                    let entry = (label(&recipient), status);
                    match &mut m.delivery {
                        Delivery::PartlyFailed(list) => {
                            if list.len() < 100 {
                                list.push(entry)
                            }
                        }
                        d => *d = Delivery::PartlyFailed(vec![entry]),
                    }
                }
            }
            AppEvent::Typing { session, typing } => {
                if let Some(c) = self.by_session(session) {
                    c.typing = typing;
                }
            }
            AppEvent::ConversationEnded { session } => {
                if let Some(c) = self.by_session(session) {
                    c.state = ConvState::Ended;
                    c.typing = false;
                }
            }
            AppEvent::CallStarted {
                id,
                peer,
                incoming,
                at,
            } => {
                if self.calls.iter().any(|c| c.id == id) {
                    return;
                }
                self.calls.push(Call {
                    id,
                    peer: label(&peer),
                    incoming,
                    state: if incoming {
                        CallState::Incoming
                    } else {
                        CallState::Dialing
                    },
                    muted: false,
                    held: false,
                    started: at,
                    conference: None,
                    demo: false,
                });
            }
            AppEvent::CallRinging { id } => {
                if let Some(c) = self.calls.iter_mut().find(|c| c.id == id) {
                    if c.state == CallState::Dialing {
                        c.state = CallState::Ringing;
                    }
                }
            }
            AppEvent::CallConnected { id, at } => {
                if let Some(c) = self.calls.iter_mut().find(|c| c.id == id) {
                    if !matches!(c.state, CallState::Connected { .. }) {
                        c.state = CallState::Connected { since: at };
                    }
                }
            }
            AppEvent::CallHeld { id, held } => {
                if let Some(c) = self.calls.iter_mut().find(|c| c.id == id) {
                    c.held = held;
                }
            }
            AppEvent::CallMuted { id, muted } => {
                if let Some(c) = self.calls.iter_mut().find(|c| c.id == id) {
                    c.muted = muted;
                }
            }
            AppEvent::CallEnded { id, status, at } => self.end_call(id, status, at),
            AppEvent::Meetings(m) => {
                self.meetings = m
                    .into_iter()
                    .take(500)
                    .map(|m| Meeting {
                        subject: label(&m.subject),
                        focus: label(&m.focus),
                        conference_id: label(&m.conference_id),
                        ..m
                    })
                    .collect();
            }
            AppEvent::Invitation(i) => {
                if self.invitations.len() < 20 {
                    self.invitations.push(Invitation {
                        from: label(&i.from),
                        focus: label(&i.focus),
                        subject: label(&i.subject),
                    });
                }
            }
            AppEvent::Notice(kind, text, at) => self.notice(kind, &text, at),
            AppEvent::TrustQuestion(q) => {
                self.trust_question = Some(TrustQuestion {
                    host: label(&q.host),
                    domain: label(&q.domain),
                    user_domain: label(&q.user_domain),
                    named_by: label(&q.named_by),
                })
            }
        }
    }
}
