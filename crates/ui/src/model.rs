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

/// A room URI as the chat server writes it: `ma-chan://<domain>/<id>`.
fn room_uri_ok(uri: &str) -> Option<()> {
    let rest = uri.strip_prefix("ma-chan://")?;
    let (domain, id) = rest.split_once('/')?;
    let ok = !domain.is_empty()
        && !id.is_empty()
        && uri.len() <= MAX_LABEL
        && !uri.chars().any(|c| c.is_control() || c.is_whitespace());
    ok.then_some(())
}

pub(crate) fn label(text: &str) -> String {
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
    /// The content of a meeting (whiteboards, slides, polls; function 44).
    pub meeting_content: Gate,
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
            meetings: off.clone(),
            persistent_chat: off.clone(),
            meeting_content: off,
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
            video_call: Gate::Preview,
            meetings: Gate::Preview,
            persistent_chat: Gate::Preview,
            meeting_content: Gate::Preview,
            ..Self::never()
        }
    }

    /// Functions the project does not have (file transfer, rich text, history search, read
    /// receipts).
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
            persistent_chat: none.clone(),
            meeting_content: none.clone(),
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
    /// Available but idle at keyboard and mouse (the availability class 4500–5999).
    Inactive,
    Away,
    /// Away with the activity "off work".
    OffWork,
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
    /// Further groups of the contact list it is in as well ("copy contact to").
    pub more_groups: Vec<String>,
    pub source: Source,
    /// In the server's favourites group.
    pub pinned: bool,
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
            more_groups: Vec::new(),
            source,
            pinned: false,
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

/// The sender of a line the client writes itself (a call connected or ended), as the Windows
/// client shows them in the conversation; its text is `connected` or `ended <seconds>`.
pub const SYSTEM_SENDER: &str = "\u{1}system";

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
    /// Waits in the meeting's lobby.
    pub waiting: bool,
    /// The audio in the meeting: `Some(muted)`.
    pub muted: Option<bool>,
    /// Takes part in the IM, sends video, shares (the roster's endpoints).
    pub in_chat: bool,
    pub video: bool,
    pub sharing: bool,
}

/// Who becomes a presenter on joining a meeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Presenters {
    Nobody,
    #[default]
    Organisation,
    Everyone,
}

/// A meeting's options as its focus reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeetingView {
    pub locked: bool,
    pub access: MeetingAccess,
    pub presenters: Presenters,
    pub phone_bypass: bool,
    pub announcements: bool,
}

/// A presenter's command in a meeting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfCommand {
    /// Admit these users from the lobby (empty: everyone waiting).
    Admit(Vec<String>),
    /// Turn a user in the lobby away.
    Deny(String),
    Role {
        user: String,
        presenter: bool,
    },
    Remove(String),
    MuteUser {
        user: String,
        mute: bool,
    },
    MuteAudience(bool),
    Lock(bool),
    /// Who skips the lobby, who becomes a presenter, whether phone callers skip the lobby.
    Options {
        access: MeetingAccess,
        presenters: Presenters,
        phone_bypass: bool,
    },
    /// Announce people entering and leaving (the meeting's audio).
    Announcements(bool),
    /// The meeting's audio calls this phone number ("call me at").
    DialOut(String),
    /// End the meeting for everyone.
    End,
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
    /// Who shares in this meeting right now.
    pub sharer: Option<String>,
    /// The meeting's options, once its focus reported them.
    pub meeting: Option<MeetingView>,
    /// The conference URI of the meeting's focus, once known.
    pub focus: Option<String>,
    pub messages: VecDeque<Message>,
    pub unread: usize,
    pub typing: bool,
    pub state: ConvState,
    pub last_activity: u64,
    /// Set by the demo for illustrative content (shown with a demo mark).
    pub demo: bool,
    /// A persistent chat room instead of a conversation of the session.
    pub room: Option<RoomInfo>,
    /// The content of the meeting this conversation belongs to, once asked for.
    pub content: Option<crate::content::MeetingContent>,
}

/// A persistent chat room (function 20): a room of the chat server, not an ad-hoc group.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoomInfo {
    /// `ma-chan://…`
    pub uri: String,
    pub topic: String,
    /// The highest message number seen (a message comes again with the history).
    pub last_chat_id: u64,
}

/// One message of a room from the chat server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoomPost {
    pub author: String,
    pub name: String,
    pub text: String,
    /// Unix seconds.
    pub at: u64,
    /// The server's message number (0: none).
    pub chat_id: u64,
    /// Our own message coming back from the server.
    pub ours: bool,
}

impl Conversation {
    /// An ad-hoc group conversation (a room is not one).
    pub fn is_group(&self) -> bool {
        self.peer.is_none() && self.room.is_none()
    }

    pub fn is_room(&self) -> bool {
        self.room.is_some()
    }

    /// The last message written by a person (not a line of the client's own).
    pub fn last_text(&self) -> Option<&Message> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.sender.as_deref() != Some(SYSTEM_SENDER))
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

/// What a call carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CallKind {
    #[default]
    Audio,
    /// Audio and video (function 28).
    Video,
    /// Someone shares a screen or window with us (function 43, receive only).
    Screen,
}

/// The received picture of a call: shown in the sandboxed video window, never in this process.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum VideoState {
    #[default]
    None,
    /// Negotiating or connecting the video stream.
    Starting,
    /// The video window shows the picture.
    Window,
    /// The video ended or could not start (the reason); the call goes on.
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub id: u32,
    pub peer: String,
    pub incoming: bool,
    pub kind: CallKind,
    /// An incoming call offers video (it can be answered with video).
    pub offers_video: bool,
    pub video: VideoState,
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
    /// The https join link, when the server names it.
    pub join_url: Option<String>,
}

/// Added to a call's ID for its self view (our own camera picture) in [`AppState::video`].
pub const SELF_VIEW: u32 = 0x8000_0000;

/// The latest received picture of a call (RGBA), shown in its conversation window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: std::sync::Arc<Vec<u8>>,
    /// Counts the pictures (a new one replaces the texture).
    pub seq: u64,
}

/// An entry of the Conversation History folder in Exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: String,
    pub subject: String,
    pub at: Option<u64>,
    pub with: String,
}

/// Dial-in numbers of the conferencing service (getConferencingCapabilities): the region to
/// show first, its numbers, and a page with all numbers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DialIn {
    pub region: String,
    /// Number and whether it is toll-free.
    pub numbers: Vec<(String, bool)>,
    /// Further regions' names (all numbers are on the page).
    pub other_regions: Vec<String>,
    pub page: Option<String>,
}

/// Who may join a meeting without waiting in the lobby.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeetingAccess {
    /// Only the people invited.
    Invited,
    /// Everyone from the organisation (and federated users).
    #[default]
    Organisation,
    /// Also anonymous people with the link.
    Anyone,
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
    /// The own personal note as the server shows it.
    pub note: String,
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
    /// What the call carries (set after `CallStarted`).
    CallKind {
        id: u32,
        kind: CallKind,
    },
    /// The incoming call offers video.
    CallOffersVideo {
        id: u32,
    },
    CallVideo {
        id: u32,
        state: VideoState,
    },
    Meetings(Vec<Meeting>),
    /// This call is the audio of the meeting of a group conversation (session ID).
    /// Someone shares in the meeting of a group conversation (their name), or nobody does.
    ConferenceSharing {
        session: u32,
        sharer: Option<String>,
    },
    /// Calls can be rated (the server has a quality service).
    QualityRating(bool),
    /// The focus (conference URI) of a group conversation.
    ConferenceFocus {
        session: u32,
        focus: String,
    },
    /// The microphone's level during its test, 0–100; `None`: the test is over.
    MicLevel(Option<u8>),
    /// The options of the meeting of a group conversation.
    MeetingSettings {
        session: u32,
        view: MeetingView,
    },
    /// A received picture of a call (RGBA).
    VideoFrame {
        call: u32,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    /// The entries of the Conversation History folder, or why they are not there.
    History(Result<Vec<HistoryEntry>, String>),
    /// One history entry's text.
    HistoryText {
        id: String,
        text: Result<String, String>,
    },
    ConferenceCall {
        id: u32,
        session: u32,
    },
    /// One meeting's details (subject, expiry, link) into its entry of the list.
    MeetingDetails(Meeting),
    MeetingCancelled(String),
    /// The dial-in numbers of the conferencing service (`None`: no dial-in).
    DialIn(Option<DialIn>),
    Invitation(Invitation),
    /// Discovery named a server outside the SIP domain: may the password go there?
    TrustQuestion(TrustQuestion),
    /// A persistent chat room: listed (not joined yet) or joined, with its participants.
    Room {
        uri: String,
        name: String,
        topic: String,
        participants: Vec<String>,
        joined: bool,
    },
    /// Messages of a room; `history` ones are not counted as unread.
    RoomPosts {
        room: String,
        posts: Vec<RoomPost>,
        history: bool,
    },
    /// A message to a room was not accepted.
    RoomNotSent {
        room: String,
        status: u16,
    },
    /// The distribution groups of the contact list: name and mail address.
    DistributionGroups(Vec<(String, String)>),
    /// The plain groups of the contact list.
    Groups(Vec<String>),
    /// Someone added the user to their contact list.
    NewSubscriber(Subscriber),
    /// The members of a distribution group, or why there are none.
    DistributionMembers {
        email: String,
        members: Result<Vec<Contact>, String>,
        partial: bool,
    },
    /// The user's appointments and automatic reply from Exchange.
    Calendar {
        appointments: Option<Vec<Appointment>>,
        out_of_office: Option<String>,
    },
    /// A contact's photo (side × side RGBA).
    ContactPhoto {
        uri: String,
        side: usize,
        rgba: Vec<u8>,
    },
    /// Connecting to the content of the meeting of this conversation.
    ContentConnecting {
        session: u32,
    },
    Content {
        session: u32,
        change: crate::content::ContentChange,
    },
    ContentEnded {
        session: u32,
        reason: String,
    },
    Notice(NoticeKind, String, u64),
}

/// A contact's photo as a square RGBA thumbnail.
#[derive(Clone, PartialEq, Eq)]
pub struct Photo {
    pub side: usize,
    pub rgba: std::sync::Arc<Vec<u8>>,
}

impl std::fmt::Debug for Photo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Photo({0}x{0})", self.side)
    }
}

/// How an appointment shows the user's time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Busy {
    Free,
    Tentative,
    Busy,
    OutOfOffice,
}

/// An appointment of the user's calendar (subject and location absent for a private one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Appointment {
    pub start: u64,
    pub end: u64,
    pub busy: Busy,
    pub subject: String,
    pub location: String,
}

/// Appointments kept at most.
pub const MAX_APPOINTMENTS: usize = 200;

/// A distribution group the user added to the contact list; its members come from the address
/// book when it is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistGroup {
    pub name: String,
    pub email: String,
    pub members: Option<Vec<Contact>>,
    /// Why the members could not be listed.
    pub error: Option<String>,
    /// The server lists only part of the members.
    pub partial: bool,
}

/// A privacy relationship (who sees how much of the user's presence; blocked: nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    External,
    Colleagues,
    Workgroup,
    Family,
    Blocked,
}

/// Someone who added the user to their contact list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscriber {
    pub uri: String,
    pub name: String,
}

/// Members of a distribution group kept at most.
pub const MAX_DG_MEMBERS: usize = 1000;

/// Photos kept at most.
pub const MAX_PHOTOS: usize = 2000;

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
    /// Contact photos by address.
    pub photos: BTreeMap<String, Photo>,
    /// The distribution groups of the contact list.
    pub distribution: Vec<DistGroup>,
    /// The plain groups of the contact list (names; the default group is not among them).
    pub groups: Vec<String>,
    /// People who added the user and were not acknowledged yet.
    pub subscribers: Vec<Subscriber>,
    /// The user's appointments from Exchange (`None`: not loaded or switched off by the server).
    pub appointments: Option<Vec<Appointment>>,
    /// The user's automatic reply while it is on.
    pub out_of_office: Option<String>,
    next_key: u64,
    /// Calls can be rated (quality service of the server).
    pub quality_rating: bool,
    /// The ended call offered for rating: call ID and peer.
    pub rate_call: Option<(u32, String)>,
    /// The Conversation History folder, once asked for.
    /// The dial-in numbers of the conferencing service, once reported.
    pub dial_in: Option<DialIn>,
    /// The microphone's level while it is tested (0–100).
    pub mic_level: Option<u8>,
    pub history: Option<Result<Vec<HistoryEntry>, String>>,
    /// The history entry being read: its ID and, once there, its text.
    pub history_open: Option<(String, Option<Result<String, String>>)>,
    /// The latest received picture per call.
    pub video: BTreeMap<u32, VideoFrame>,
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
            photos: BTreeMap::new(),
            distribution: Vec::new(),
            groups: Vec::new(),
            subscribers: Vec::new(),
            quality_rating: false,
            mic_level: None,
            dial_in: None,
            history: None,
            video: BTreeMap::new(),
            history_open: None,
            rate_call: None,
            appointments: None,
            out_of_office: None,
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

    /// A person's name as known from the contacts, search results or distribution groups;
    /// empty if only the address is known (a new contact then takes the server's name).
    pub fn known_name(&self, uri: &str) -> String {
        let bare = uri.strip_prefix("sip:").unwrap_or(uri);
        self.contacts
            .get(uri)
            .into_iter()
            .chain(self.search_results.iter().filter(|c| c.uri == uri))
            .chain(
                self.distribution
                    .iter()
                    .flat_map(|g| g.members.iter().flatten())
                    .filter(|c| c.uri == uri),
            )
            .map(|c| c.name.trim().to_string())
            .find(|n| !n.is_empty() && n != bare)
            .unwrap_or_default()
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
            sharer: None,
            meeting: None,
            focus: None,
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
            room: None,
            content: None,
        });
        key
    }

    /// The conversation of a room.
    pub fn room_key(&self, uri: &str) -> Option<ConvKey> {
        self.conversations
            .iter()
            .find(|c| c.room.as_ref().is_some_and(|r| r.uri == uri))
            .map(|c| c.key)
    }

    fn room_mut(&mut self, uri: &str) -> Option<&mut Conversation> {
        self.conversations
            .iter_mut()
            .find(|c| c.room.as_ref().is_some_and(|r| r.uri == uri))
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

    /// The conversation a call belongs to: its meeting, or the conversation with its peer.
    pub fn conversation_of_call(&self, call: &Call) -> Option<ConvKey> {
        call.conference.or_else(|| {
            (!call.peer.is_empty())
                .then(|| self.conversation_with(&call.peer))
                .flatten()
        })
    }

    /// A line of the client's own in the conversation of a call.
    fn system_line(&mut self, call: &Call, text: String, at: u64) {
        let Some(key) = self.conversation_of_call(call) else {
            return;
        };
        if let Some(c) = self.conversation_mut(key) {
            c.messages.push_back(Message {
                sender: Some(SYSTEM_SENDER.into()),
                text,
                at,
                delivery: Delivery::Unknown,
            });
            while c.messages.len() > MAX_MESSAGES {
                c.messages.pop_front();
            }
        }
    }

    fn end_call(&mut self, id: u32, status: u16, at: u64) {
        self.video.remove(&id);
        self.video.remove(&(id | SELF_VIEW));
        if let Some(call) = self
            .calls
            .iter()
            .find(|c| c.id == id && c.conference.is_none())
            .cloned()
        {
            if let CallState::Connected { since } = call.state {
                self.system_line(&call, format!("ended {}", at.saturating_sub(since)), at);
            }
        }
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
        // A connected call (not a meeting's audio) may be rated afterwards.
        if self.quality_rating && seconds > 0 && c.conference.is_none() {
            self.rate_call = Some((id, c.peer.clone()));
        }
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
                    note: clean(&p.note, 1024, false),
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
                // Members of an opened distribution group show their presence too. SIP URIs
                // compare without regard to case (the directory may spell them otherwise than
                // the presence document).
                for g in &mut self.distribution {
                    for m in g.members.iter_mut().flatten() {
                        if m.uri.eq_ignore_ascii_case(&uri) {
                            if let Some(p) = presence {
                                m.presence = p;
                            }
                            m.note = clean(&note, 1024, false);
                        }
                    }
                }
                let key = if self.contacts.contains_key(&uri) {
                    Some(uri.clone())
                } else {
                    self.contacts
                        .keys()
                        .find(|k| k.eq_ignore_ascii_case(&uri))
                        .cloned()
                };
                if let Some(c) = key.and_then(|k| self.contacts.get_mut(&k)) {
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
                if uri.eq_ignore_ascii_case(&self.me.uri) {
                    if let Some(p) = presence {
                        self.me.presence = p;
                    }
                    self.me.note = clean(&note, 1024, false);
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
                            waiting: p.waiting,
                            muted: p.muted,
                            in_chat: p.in_chat,
                            video: p.video,
                            sharing: p.sharing,
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
                    kind: CallKind::Audio,
                    offers_video: false,
                    video: VideoState::None,
                    muted: false,
                    held: false,
                    started: at,
                    conference: None,
                    demo: false,
                });
            }
            AppEvent::CallKind { id, kind } => {
                if let Some(c) = self.calls.iter_mut().find(|c| c.id == id) {
                    c.kind = kind;
                }
            }
            AppEvent::CallOffersVideo { id } => {
                if let Some(c) = self.calls.iter_mut().find(|c| c.id == id) {
                    c.offers_video = c.incoming;
                }
            }
            AppEvent::CallVideo { id, state } => {
                if let Some(c) = self.calls.iter_mut().find(|c| c.id == id) {
                    c.video = match state {
                        VideoState::Failed(why) => VideoState::Failed(label(&why)),
                        s => s,
                    };
                }
            }
            AppEvent::CallRinging { id } => {
                if let Some(c) = self.calls.iter_mut().find(|c| c.id == id) {
                    if c.state == CallState::Dialing {
                        c.state = CallState::Ringing;
                    }
                }
            }
            AppEvent::CallConnected { id, at } => {
                let mut newly = None;
                if let Some(c) = self.calls.iter_mut().find(|c| c.id == id) {
                    if !matches!(c.state, CallState::Connected { .. }) {
                        c.state = CallState::Connected { since: at };
                        newly = Some(c.clone());
                    }
                }
                if let Some(call) = newly.filter(|c| c.conference.is_none()) {
                    self.system_line(&call, "connected".into(), at);
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
                        join_url: m
                            .join_url
                            .filter(|u| u.starts_with("https://") && u.len() <= 2048)
                            .map(|u| label(&u)),
                        ..m
                    })
                    .collect();
            }
            AppEvent::MeetingDetails(m) => {
                if let Some(e) = self
                    .meetings
                    .iter_mut()
                    .find(|e| e.conference_id == m.conference_id)
                {
                    if !m.subject.trim().is_empty() {
                        e.subject = label(&m.subject);
                    }
                    e.expires = m.expires.or(e.expires);
                    if let Some(u) = m
                        .join_url
                        .filter(|u| u.starts_with("https://") && u.len() <= 2048)
                    {
                        e.join_url = Some(label(&u));
                    }
                }
            }
            AppEvent::MeetingCancelled(id) => self.meetings.retain(|m| m.conference_id != id),
            AppEvent::QualityRating(on) => self.quality_rating = on,
            AppEvent::MicLevel(l) => self.mic_level = l.map(|l| l.min(100)),
            AppEvent::DialIn(d) => {
                self.dial_in = d.map(|d| DialIn {
                    region: label(&d.region),
                    numbers: d
                        .numbers
                        .into_iter()
                        .take(32)
                        .map(|(n, free)| (label(&n), free))
                        .collect(),
                    other_regions: d.other_regions.iter().take(64).map(|r| label(r)).collect(),
                    page: d.page,
                })
            }
            AppEvent::ConferenceFocus { session, focus } => {
                if let Some(c) = self.by_session(session) {
                    c.focus = Some(label(&focus));
                }
            }
            AppEvent::MeetingSettings { session, view } => {
                if let Some(c) = self.by_session(session) {
                    c.meeting = Some(view);
                }
            }
            AppEvent::VideoFrame {
                call,
                width,
                height,
                rgba,
            } => {
                let fits = width > 0
                    && height > 0
                    && width <= 2048
                    && height <= 2048
                    && rgba.len() == width as usize * height as usize * 4;
                if fits && self.calls.iter().any(|c| c.id == call & !SELF_VIEW) {
                    let seq = self.video.get(&call).map_or(1, |f| f.seq.wrapping_add(1));
                    self.video.insert(
                        call,
                        VideoFrame {
                            width,
                            height,
                            rgba: std::sync::Arc::new(rgba),
                            seq,
                        },
                    );
                }
            }
            AppEvent::History(r) => {
                self.history = Some(r.map(|l| {
                    l.into_iter()
                        .take(200)
                        .map(|e| HistoryEntry {
                            id: label(&e.id),
                            subject: label(&e.subject),
                            with: label(&e.with),
                            ..e
                        })
                        .collect()
                }));
            }
            AppEvent::HistoryText { id, text } => {
                if self.history_open.as_ref().is_some_and(|(o, _)| *o == id) {
                    self.history_open = Some((
                        id,
                        Some(text.map(|t| {
                            t.chars()
                                .filter(|c| *c == '\n' || !c.is_control())
                                .take(256 * 1024)
                                .collect()
                        })),
                    ));
                }
            }
            AppEvent::ConferenceSharing { session, sharer } => {
                if let Some(c) = self.by_session(session) {
                    c.sharer = sharer.map(|s| label(&s));
                }
            }
            AppEvent::ConferenceCall { id, session } => {
                let key = self.by_session(session).map(|c| c.key);
                if let Some(c) = self.calls.iter_mut().find(|c| c.id == id) {
                    c.conference = key;
                }
            }
            AppEvent::Invitation(i) => {
                // The same conference again (the inviting client may send it twice): once.
                let focus = label(&i.focus);
                if self.invitations.len() < 20 && !self.invitations.iter().any(|v| v.focus == focus)
                {
                    self.invitations.push(Invitation {
                        from: label(&i.from),
                        focus: label(&i.focus),
                        subject: label(&i.subject),
                    });
                }
            }
            AppEvent::Notice(kind, text, at) => self.notice(kind, &text, at),
            AppEvent::Room {
                uri,
                name,
                topic,
                participants,
                joined,
            } => {
                if room_uri_ok(&uri).is_none() {
                    return;
                }
                let uri = label(&uri);
                let title = if name.trim().is_empty() {
                    uri.clone()
                } else {
                    label(&name)
                };
                let key = match self.room_key(&uri) {
                    Some(k) => k,
                    None => {
                        let k = self.new_conversation(None, None, title.clone(), None);
                        if let Some(c) = self.conversation_mut(k) {
                            c.room = Some(RoomInfo {
                                uri: uri.clone(),
                                ..RoomInfo::default()
                            });
                        }
                        k
                    }
                };
                let names: Vec<Participant> = participants
                    .iter()
                    .take(1000)
                    .map(|p| Participant {
                        uri: label(p),
                        name: self.name_of(p),
                        presenter: false,
                        waiting: false,
                        muted: None,
                        in_chat: true,
                        video: false,
                        sharing: false,
                    })
                    .collect();
                if let Some(c) = self.conversation_mut(key) {
                    c.title = title;
                    if let Some(r) = c.room.as_mut() {
                        if !topic.is_empty() || joined {
                            r.topic = label(&topic);
                        }
                    }
                    if joined {
                        c.state = ConvState::Open;
                        c.participants = names;
                    }
                }
            }
            AppEvent::RoomPosts {
                room,
                posts,
                history,
            } => {
                let me = self.me.uri.clone();
                let names: Vec<String> = posts.iter().map(|p| self.name_of(&p.author)).collect();
                let Some(c) = self.room_mut(&room) else {
                    return;
                };
                for (p, known) in posts.into_iter().zip(names) {
                    let last = c.room.as_ref().map_or(0, |r| r.last_chat_id);
                    if p.chat_id != 0 && p.chat_id <= last {
                        continue;
                    }
                    if let Some(r) = c.room.as_mut() {
                        r.last_chat_id = r.last_chat_id.max(p.chat_id);
                    }
                    let own = p.ours || (!me.is_empty() && p.author == me);
                    // Our message coming back: the server took it.
                    if p.ours {
                        if let Some(m) = c
                            .messages
                            .iter_mut()
                            .find(|m| m.sender.is_none() && m.delivery == Delivery::Sending)
                        {
                            m.delivery = Delivery::Delivered;
                            continue;
                        }
                    }
                    let sender = if own {
                        None
                    } else if p.name.trim().is_empty() {
                        Some(label(&p.author))
                    } else {
                        // The room's display name, unless the contact list knows better.
                        Some(label(if known != p.author { &known } else { &p.name }))
                    };
                    if !own && !history {
                        c.unread = c.unread.saturating_add(1);
                    }
                    c.push(Message {
                        sender,
                        text: clean(&p.text, MAX_TEXT, true),
                        at: p.at,
                        delivery: if own {
                            Delivery::Delivered
                        } else {
                            Delivery::Unknown
                        },
                    });
                }
            }
            AppEvent::DistributionGroups(list) => {
                let old = std::mem::take(&mut self.distribution);
                self.distribution = list
                    .into_iter()
                    .take(200)
                    .map(|(name, email)| {
                        let email = label(&email);
                        let kept = old.iter().find(|g| g.email == email);
                        DistGroup {
                            name: label(&name),
                            members: kept.and_then(|g| g.members.clone()),
                            error: None,
                            partial: kept.is_some_and(|g| g.partial),
                            email,
                        }
                    })
                    .collect();
            }
            AppEvent::Groups(list) => {
                self.groups = list.iter().take(200).map(|g| label(g)).collect();
            }
            AppEvent::NewSubscriber(s) => {
                let uri = label(&s.uri);
                if self.subscribers.len() < 200 && !self.subscribers.iter().any(|x| x.uri == uri) {
                    self.subscribers.push(Subscriber {
                        uri,
                        name: label(&s.name),
                    });
                }
            }
            AppEvent::DistributionMembers {
                email,
                members,
                partial,
            } => {
                if let Some(g) = self.distribution.iter_mut().find(|g| g.email == email) {
                    match members {
                        Ok(list) => {
                            g.members = Some(
                                list.into_iter()
                                    .take(MAX_DG_MEMBERS)
                                    .map(Contact::cleaned)
                                    .collect(),
                            );
                            g.error = None;
                            g.partial = partial;
                        }
                        Err(e) => g.error = Some(label(&e)),
                    }
                }
            }
            AppEvent::Calendar {
                appointments,
                out_of_office,
            } => {
                self.appointments = appointments.map(|list| {
                    let mut list: Vec<Appointment> = list
                        .into_iter()
                        .filter(|a| a.end >= a.start)
                        .take(MAX_APPOINTMENTS)
                        .map(|a| Appointment {
                            subject: label(&a.subject),
                            location: label(&a.location),
                            ..a
                        })
                        .collect();
                    list.sort_by_key(|a| a.start);
                    list
                });
                // Some(""): the replies are on, without a text.
                self.out_of_office = out_of_office.map(|t| clean(&t, 500, false));
            }
            AppEvent::ContactPhoto { uri, side, rgba } => {
                let ok = (16..=256).contains(&side) && rgba.len() == side * side * 4;
                if ok && (self.photos.len() < MAX_PHOTOS || self.photos.contains_key(&uri)) {
                    self.photos.insert(
                        label(&uri),
                        Photo {
                            side,
                            rgba: std::sync::Arc::new(rgba),
                        },
                    );
                }
            }
            AppEvent::ContentConnecting { session } => {
                if let Some(c) = self.by_session(session) {
                    c.content = Some(crate::content::MeetingContent::default());
                }
            }
            AppEvent::Content { session, change } => {
                if let Some(m) = self.by_session(session).and_then(|c| c.content.as_mut()) {
                    m.apply(change);
                }
            }
            AppEvent::ContentEnded { session, reason } => {
                if let Some(m) = self.by_session(session).and_then(|c| c.content.as_mut()) {
                    m.state = crate::content::ContentState::Ended(label(&reason));
                }
            }
            AppEvent::RoomNotSent { room, status } => {
                if let Some(m) = self.room_mut(&room).and_then(|c| {
                    c.messages
                        .iter_mut()
                        .find(|m| m.sender.is_none() && m.delivery == Delivery::Sending)
                }) {
                    m.delivery = Delivery::Failed(status);
                }
            }
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
