//! The interface shell: the panels, the keyboard shortcuts and the intents for the adapter.

use crate::model::{AppState, Connection, ConvKey, Presence, View};
use crate::text::{Lang, Texts};
use crate::theme::{self, Palette};
use crate::views;
use egui::{Context, Key, KeyboardShortcut, Modifiers, Ui};
use std::collections::{BTreeMap, BTreeSet};

/// What the user asks of the adapter. The interface changes its own state (selection, filters,
/// drafts) itself; everything that needs the session goes out as an intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiIntent {
    StartDemo,
    SignIn(SignIn),
    SignOut,
    SetPresence(Presence),
    /// Back to the automatic state (the manual one removed).
    ResetStatus,
    /// Publish the personal note (empty: remove it).
    SetNote(String),
    /// Open a conversation with one peer under this local key.
    StartConversation {
        key: ConvKey,
        peer: String,
    },
    SendMessage {
        key: ConvKey,
        text: String,
    },
    Typing {
        key: ConvKey,
        typing: bool,
    },
    EndConversation {
        key: ConvKey,
    },
    StartGroup {
        invite: Vec<String>,
    },
    InviteToGroup {
        key: ConvKey,
        peer: String,
    },
    CallUser {
        peer: String,
    },
    /// Blind transfer of a call to an address (REFER).
    Transfer {
        id: u32,
        peer: String,
    },
    AnswerCall {
        id: u32,
    },
    /// A call with audio and video (function 28).
    VideoCall {
        peer: String,
    },
    /// Answer an incoming call with video.
    AnswerVideo {
        id: u32,
    },
    /// View the screen or window someone shares (function 43).
    ViewScreen {
        id: u32,
    },
    HangUp {
        id: u32,
    },
    Hold {
        id: u32,
        hold: bool,
    },
    Mute {
        id: u32,
        muted: bool,
    },
    SearchDirectory {
        text: String,
    },
    JoinMeeting {
        focus: String,
    },
    /// Schedule a meeting (lifetime in minutes); `join`: join it at once (meet now).
    ScheduleMeeting {
        subject: String,
        minutes: u64,
        access: crate::model::MeetingAccess,
        join: bool,
    },
    ChangeMeeting {
        conference_id: String,
        subject: String,
        minutes: u64,
        access: crate::model::MeetingAccess,
    },
    CancelMeeting {
        conference_id: String,
    },
    /// Join a meeting by its https link (the join manager names the focus).
    JoinMeetingLink {
        link: String,
    },
    /// Rate an ended call (1–5 stars).
    RateCall {
        id: u32,
        stars: u8,
    },
    /// Transfer a call to the party of another call (attended transfer).
    TransferToCall {
        id: u32,
        target: u32,
    },
    /// Join the audio of the meeting of this group conversation.
    JoinMeetingAudio {
        key: ConvKey,
    },
    /// Join the meeting with audio and video.
    JoinMeetingVideo {
        key: ConvKey,
    },
    /// View what is shared in the meeting.
    ViewMeetingScreen {
        key: ConvKey,
    },
    /// A presenter's command in the meeting of this conversation.
    ConferenceCommand {
        key: ConvKey,
        command: crate::model::ConfCommand,
    },
    RefreshMeetings,
    AcceptInvitation {
        index: usize,
    },
    DeclineInvitation {
        index: usize,
    },
    /// The local preferences changed (favourites, contrast, scale).
    PrefsChanged,
    /// The answer to a trust question (true: trust the domain and remember it).
    AnswerTrust(bool),
    /// Add a person to the contact list (into this group, else the default group).
    AddContact {
        uri: String,
        name: String,
        group: Option<String>,
    },
    RemoveContact {
        uri: String,
    },
    /// Put a contact into this group (instead of its present groups).
    MoveContact {
        uri: String,
        group: String,
    },
    AddGroup {
        name: String,
    },
    RenameGroup {
        name: String,
        to: String,
    },
    DeleteGroup {
        name: String,
    },
    /// Change a person's privacy relationship (`None`: back to the server's default).
    SetRelation {
        uri: String,
        relation: Option<crate::model::Relation>,
    },
    /// Done with a new subscriber (seen; added or not).
    AcknowledgeSubscriber {
        uri: String,
    },
    /// List the members of a distribution group (address book).
    ExpandGroup {
        email: String,
    },
    /// Remove the password kept in the keyring for the last sign-in address.
    ForgetPassword,
    /// Tones in a call (DTMF).
    Dtmf {
        id: u32,
        digits: String,
    },
    /// Close the application (signs out first).
    Quit,
    /// Minimize the main window.
    Minimize,
    /// Bring the main window forward.
    FocusMain,
    /// The content of the meeting of this group conversation (function 44).
    JoinContent {
        key: ConvKey,
    },
    LeaveContent {
        key: ConvKey,
    },
    /// A vote in a poll of the meeting content.
    Vote {
        key: ConvKey,
        content: i64,
        choice: i64,
    },
    /// Persistent chat: connect to the chat server, list our rooms and join them.
    LoadRooms,
    /// A message to a persistent chat room.
    SendToRoom {
        room: String,
        text: String,
    },
}

/// The sign-in form. The password leaves the form with the intent; the field is cleared.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct SignIn {
    pub address: String,
    pub user: String,
    pub domain: String,
    pub password: String,
    /// `host:port`; empty means Autodiscover and DNS.
    pub server: String,
    /// Try Kerberos first (the system's ticket cache, else the password), NTLM as fallback.
    pub kerberos: bool,
    /// Sign in with TLS-DSK (a user certificate, got once with the password).
    pub tls_dsk: bool,
    /// Keep the password in the system's keyring after a successful sign-in.
    pub remember: bool,
}

impl std::fmt::Debug for SignIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignIn")
            .field("address", &self.address)
            .field("user", &self.user)
            .field("domain", &self.domain)
            .field("password", &"<hidden>")
            .field("server", &self.server)
            .field("kerberos", &self.kerberos)
            .field("tls_dsk", &self.tls_dsk)
            .field("remember", &self.remember)
            .finish()
    }
}

/// Local preferences: favourites (addresses or group titles), contrast and scale. No chat
/// content is ever part of them.
#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    pub favorites: BTreeSet<String>,
    pub high_contrast: bool,
    pub scale: f32,
    pub context_open: bool,
    pub lang: Lang,
    pub always_on_top: bool,
    /// Contact photos instead of initials.
    pub show_pictures: bool,
    /// Contacts in one line (name only) instead of two (name and status).
    pub compact_contacts: bool,
    pub sort_by_availability: bool,
    /// Away, offline and unknown contacts in a group of their own.
    pub away_group: bool,
    pub favorites_group: bool,
    /// A notification for new messages in conversations that are not open.
    pub message_alerts: bool,
    /// No notifications for messages while the own status is Do Not Disturb.
    pub quiet_when_dnd: bool,
    pub chat_text_size: f32,
    pub sounds: bool,
    pub quiet_sounds_busy: bool,
    /// The camera device; empty: the default.
    pub camera: String,
    /// Show the participant list when joining a meeting.
    pub meeting_participants: bool,
    /// The last sign-in address (no secret; offered in the sign-in form).
    pub last_address: String,
    /// Keep IM conversations in the Conversation History folder in Exchange.
    pub save_history: bool,
    /// Inactive after this many idle minutes (0: never).
    pub inactive_minutes: u32,
    /// Away after this many more minutes of being inactive (0: never).
    pub away_minutes: u32,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            favorites: BTreeSet::new(),
            high_contrast: false,
            scale: 1.0,
            context_open: true,
            lang: Lang::De,
            always_on_top: false,
            show_pictures: true,
            compact_contacts: false,
            sort_by_availability: false,
            away_group: false,
            favorites_group: true,
            message_alerts: true,
            quiet_when_dnd: true,
            chat_text_size: 14.0,
            sounds: true,
            quiet_sounds_busy: false,
            camera: String::new(),
            meeting_participants: false,
            last_address: String::new(),
            save_history: true,
            inactive_minutes: 5,
            away_minutes: 5,
        }
    }
}

/// The values of the schedule dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingDialog {
    /// `Some(conference ID)`: changing that meeting.
    pub change: Option<String>,
    pub subject: String,
    pub minutes: u64,
    pub access: crate::model::MeetingAccess,
}

/// The tabs of the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainTab {
    #[default]
    Contacts,
    Conversations,
    Meetings,
    /// Persistent chat rooms (only when the server offers them).
    Rooms,
}

/// How the contacts tab groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContactsView {
    #[default]
    Groups,
    Status,
    Relationships,
    New,
}

/// The filter of the conversations tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConvFilter {
    #[default]
    All,
    Missed,
    Calls,
}

/// The windows besides the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Win {
    Conv(ConvKey),
    Call(u32),
    Options,
    About,
    Card,
}

/// What the address dialog is for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AddressFor {
    #[default]
    NewChat,
    /// Invite into this group conversation.
    Invite(ConvKey),
    /// Transfer this call (blind transfer).
    Transfer(u32),
    /// Add a person to the contact list.
    AddContact,
}

/// The address dialog: a new conversation or group, an invitation, a transfer target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewChat {
    pub text: String,
    pub purpose: AddressFor,
}

/// The whole interface: state from the adapter plus the interface's own state.
pub struct CommsUi {
    pub state: AppState,
    pub prefs: Prefs,
    pub t: &'static Texts,
    /// Local time offset to UTC in seconds.
    pub utc_offset: i64,
    /// Unix seconds of this frame.
    pub now: u64,
    pub demo: bool,
    pub logo: Option<egui::TextureHandle>,
    pub(crate) view: View,
    pub(crate) selected_conv: Option<ConvKey>,
    pub(crate) selected_contact: Option<String>,
    pub(crate) selected_meeting: Option<usize>,
    pub(crate) search: String,
    pub(crate) focus_search: bool,
    pub(crate) drafts: BTreeMap<ConvKey, String>,
    pub(crate) focus_composer: bool,
    pub(crate) new_chat: Option<NewChat>,
    pub(crate) sign_in: SignIn,
    pub(crate) typing_since: BTreeMap<ConvKey, u64>,
    pub(crate) last_search: String,
    /// The search text waiting to settle before it goes to the server, and since when (s).
    pub(crate) search_pending: (String, f64),
    pub(crate) intents: Vec<UiIntent>,
    pub(crate) window_width: f32,
    /// The next group conversation that appears is selected (the user just started or joined
    /// one).
    pub(crate) select_next_group: bool,
    pub(crate) tab: MainTab,
    pub(crate) contacts_view: ContactsView,
    pub(crate) conv_filter: ConvFilter,
    pub(crate) note: String,
    /// The note as last published (or received).
    pub(crate) published_note: String,
    /// The own note as the server last showed it (taken into the field when it changes).
    pub(crate) seen_note: String,
    /// The newest warning the user closed (its time).
    pub(crate) dismissed_alert: u64,
    pub(crate) collapsed: BTreeSet<String>,
    /// The name dialog: a new group, or renaming one (its old name).
    pub(crate) group_dialog: Option<(Option<String>, String)>,
    /// The meeting link typed in the meetings tab.
    pub(crate) meeting_link: String,
    /// The star under the pointer in the call rating (0: none).
    pub(crate) rating_hover: u8,
    /// The schedule dialog: the meeting changed (its ID) or a new one, subject, minutes, access.
    pub(crate) meeting_dialog: Option<MeetingDialog>,
    /// Distribution groups whose members were asked for.
    pub(crate) expanded_groups: BTreeSet<String>,
    pub(crate) windows: Vec<Win>,
    pub(crate) focus_window: Option<Win>,
    pub(crate) options_page: usize,
    pub(crate) prefs_draft: Option<Prefs>,
    pub(crate) card: Option<String>,
    pub(crate) participants_open: BTreeSet<ConvKey>,
    /// Message notifications the user ignored: conversation and its message count then.
    pub(crate) toasts_seen: BTreeSet<(ConvKey, usize)>,
    /// Calls whose window was opened once (closing it does not open it again).
    pub(crate) call_windows_opened: BTreeSet<u32>,
    /// A screenshot of a window of its own, taken in that window's own frame (tests and
    /// documentation).
    pub screenshot: Option<(Win, egui::ScreenshotCallback)>,
    /// The keyring holds a password for this sign-in address (set by the application).
    pub stored_password_for: Option<String>,
    /// Textures of the contact photos, made when first shown.
    pub(crate) textures: std::cell::RefCell<BTreeMap<String, egui::TextureHandle>>,
}

impl CommsUi {
    pub fn new(lang: Lang, utc_offset: i64) -> Self {
        Self {
            state: AppState::default(),
            prefs: Prefs::default(),
            t: lang.texts(),
            utc_offset,
            now: 0,
            demo: false,
            logo: None,
            view: View::Contacts,
            selected_conv: None,
            selected_contact: None,
            selected_meeting: None,
            search: String::new(),
            focus_search: false,
            drafts: BTreeMap::new(),
            focus_composer: false,
            new_chat: None,
            sign_in: SignIn::default(),
            typing_since: BTreeMap::new(),
            last_search: String::new(),
            search_pending: (String::new(), 0.0),
            intents: Vec::new(),
            window_width: 1440.0,
            select_next_group: false,
            tab: MainTab::Contacts,
            contacts_view: ContactsView::Groups,
            conv_filter: ConvFilter::All,
            note: String::new(),
            published_note: String::new(),
            seen_note: String::new(),
            dismissed_alert: 0,
            collapsed: BTreeSet::new(),
            group_dialog: None,
            meeting_link: String::new(),
            rating_hover: 0,
            meeting_dialog: None,
            expanded_groups: BTreeSet::new(),
            windows: Vec::new(),
            focus_window: None,
            options_page: 0,
            prefs_draft: None,
            card: None,
            participants_open: BTreeSet::new(),
            toasts_seen: BTreeSet::new(),
            call_windows_opened: BTreeSet::new(),
            screenshot: None,
            stored_password_for: None,
            textures: std::cell::RefCell::new(BTreeMap::new()),
        }
    }

    pub fn palette(&self) -> Palette {
        if self.prefs.high_contrast {
            theme::HIGH_CONTRAST
        } else {
            theme::LIGHT
        }
    }

    /// Puts text into the global search (as typed by the user).
    pub fn set_search(&mut self, text: &str) {
        self.search = text.chars().take(128).collect();
    }

    pub fn view(&self) -> View {
        self.view
    }

    /// Selects what the main window shows (the views of the command line map to its tabs).
    pub fn set_view(&mut self, v: View) {
        self.view = v;
        self.tab = match v {
            View::Chats => MainTab::Conversations,
            View::Contacts => MainTab::Contacts,
            // No telephony in this interface: calls are listed with the conversations.
            View::Calls => {
                self.conv_filter = ConvFilter::Calls;
                MainTab::Conversations
            }
            View::Meetings => MainTab::Meetings,
            View::Settings => {
                self.open_options(0);
                self.tab
            }
        };
    }

    pub fn selected_conversation(&self) -> Option<ConvKey> {
        self.selected_conv
    }

    /// Opens (or brings forward) the window of a conversation.
    pub fn select_conversation(&mut self, key: ConvKey) {
        self.view = View::Chats;
        self.selected_conv = Some(key);
        self.focus_composer = true;
        if let Some(c) = self.state.conversation_mut(key) {
            c.unread = 0;
        }
        let open_participants = self.prefs.meeting_participants
            && self.state.conversation(key).is_some_and(|c| c.is_group());
        if open_participants {
            self.participants_open.insert(key);
        }
        self.open_window(Win::Conv(key));
    }

    /// A person's picture: the photo when there is one (and pictures are shown), else initials.
    pub(crate) fn avatar(
        &self,
        ui: &mut Ui,
        uri: Option<&str>,
        name: &str,
        diameter: f32,
        presence: Option<Presence>,
        pal: &Palette,
    ) {
        let photo = uri
            .filter(|_| self.prefs.show_pictures)
            .and_then(|u| self.state.photos.get(u).map(|p| (u.to_string(), p.clone())));
        if let Some((u, p)) = photo {
            let tex = self
                .textures
                .borrow_mut()
                .entry(u.clone())
                .or_insert_with(|| {
                    let img = egui::ColorImage::from_rgba_unmultiplied([p.side, p.side], &p.rgba);
                    ui.ctx()
                        .load_texture(format!("photo:{u}"), img, egui::TextureOptions::LINEAR)
                })
                .clone();
            crate::widgets::avatar_photo(ui, &tex, diameter, presence, pal);
        } else {
            crate::widgets::avatar(ui, name, diameter, presence, pal);
        }
    }

    /// Puts an address into the sign-in form (the last one used).
    pub fn set_sign_in_address(&mut self, address: &str) {
        self.sign_in.address = address.chars().take(256).collect();
    }

    /// The viewport of a window of its own (screenshots).
    pub fn viewport_of(&self, w: Win) -> egui::ViewportId {
        views::conv_window::viewport_of(w)
    }

    /// The open windows besides the main window.
    pub fn windows(&self) -> &[Win] {
        &self.windows
    }

    pub fn open_window(&mut self, w: Win) {
        if !self.windows.contains(&w) {
            self.windows.push(w);
        }
        self.focus_window = Some(w);
    }

    pub fn close_window(&mut self, w: Win) {
        self.windows.retain(|x| *x != w);
        if w == Win::Options {
            self.prefs_draft = None;
        }
        if let Win::Conv(k) = w {
            if self.selected_conv == Some(k) {
                self.selected_conv = None;
            }
        }
    }

    pub fn open_options(&mut self, page: usize) {
        self.options_page = page.min(12);
        self.open_window(Win::Options);
    }

    pub fn open_card(&mut self, uri: &str) {
        self.card = Some(uri.to_string());
        self.open_window(Win::Card);
    }

    /// Brings the main window forward (dialogs that live there, like the address dialog).
    pub fn focus_main(&mut self) {
        self.focus_window = None;
        self.intents.push(UiIntent::FocusMain);
    }

    /// A call (with video when asked); its window opens when the session reports it.
    pub fn call(&mut self, peer: &str, video: bool) {
        let peer = peer.to_string();
        self.emit(if video {
            UiIntent::VideoCall { peer }
        } else {
            UiIntent::CallUser { peer }
        });
    }

    pub fn select_contact(&mut self, uri: &str) {
        self.view = View::Contacts;
        self.selected_contact = Some(uri.to_string());
    }

    /// The intents emitted outside a frame (by `chat_with` and the like).
    pub fn take_intents(&mut self) -> Vec<UiIntent> {
        std::mem::take(&mut self.intents)
    }

    /// The next group conversation that appears is the one the user asked for: select it.
    pub fn expect_group(&mut self) {
        self.select_next_group = true;
    }

    /// Applies an adapter event; a group the user just started or joined is selected.
    pub fn apply(&mut self, e: crate::model::AppEvent) {
        let new_group = match &e {
            crate::model::AppEvent::GroupStarted { session, .. } => {
                self.select_next_group && self.state.key_of_session(*session).is_none()
            }
            _ => false,
        };
        let session = match &e {
            crate::model::AppEvent::GroupStarted { session, .. } => Some(*session),
            _ => None,
        };
        self.state.apply(e);
        if new_group {
            self.select_next_group = false;
            if let Some(k) = session.and_then(|s| self.state.key_of_session(s)) {
                self.select_conversation(k);
            }
        }
    }

    pub fn emit(&mut self, i: UiIntent) {
        if matches!(
            i,
            UiIntent::StartGroup { .. }
                | UiIntent::JoinMeeting { .. }
                | UiIntent::AcceptInvitation { .. }
        ) {
            self.select_next_group = true;
        }
        self.intents.push(i);
    }

    /// The key a conversation is remembered by as favourite.
    pub fn favorite_id(&self, key: ConvKey) -> Option<String> {
        let c = self.state.conversation(key)?;
        Some(match (&c.peer, &c.room) {
            (Some(p), _) => p.clone(),
            (None, Some(r)) => format!("room:{}", r.uri),
            (None, None) => format!("group:{}", c.title),
        })
    }

    pub fn toggle_favorite(&mut self, id: String) {
        if !self.prefs.favorites.remove(&id) {
            self.prefs.favorites.insert(id);
        }
        self.emit(UiIntent::PrefsChanged);
    }

    /// Opens (or selects) the conversation with a peer.
    pub fn chat_with(&mut self, peer: &str) {
        let key = match self.state.conversation_with(peer) {
            Some(k) => k,
            None => {
                let key = self.state.new_key();
                self.state
                    .apply(crate::model::AppEvent::ConversationOpening {
                        key,
                        peer: peer.to_string(),
                    });
                self.emit(UiIntent::StartConversation {
                    key,
                    peer: peer.to_string(),
                });
                key
            }
        };
        self.select_conversation(key);
    }

    fn shortcuts(&mut self, ctx: &Context) {
        let ctrl = |k| KeyboardShortcut::new(Modifiers::COMMAND, k);
        if ctx.input_mut(|i| i.consume_shortcut(&ctrl(Key::K))) {
            self.focus_search = true;
        }
        if self.state.capabilities.chat.usable()
            && ctx.input_mut(|i| i.consume_shortcut(&ctrl(Key::N)))
        {
            self.new_chat = Some(NewChat::default());
        }
        let digits = [
            (Key::Num1, View::Chats),
            (Key::Num2, View::Contacts),
            (Key::Num3, View::Calls),
            (Key::Num4, View::Meetings),
        ];
        for (k, v) in digits {
            if ctx.input_mut(|i| i.consume_shortcut(&ctrl(k))) {
                self.set_view(v);
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.new_chat.is_some() {
                self.new_chat = None;
            } else if !self.search.is_empty() {
                self.search.clear();
            }
        }
    }

    /// Draws one frame and returns what the user asked for.
    pub fn show(&mut self, ui: &mut Ui, now: u64) -> Vec<UiIntent> {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        self.now = now;
        let pal = self.palette();
        theme::apply(ctx, &pal);
        ctx.set_zoom_factor(self.prefs.scale.clamp(1.0, 2.0));
        self.window_width = ctx.content_rect().width();
        self.shortcuts(ctx);
        let signed_out = matches!(
            self.state.connection,
            Connection::SignedOut | Connection::Connecting | Connection::Failed(_)
        ) && !self.demo;
        views::signin::trust_dialog(self, ctx, &pal);
        if signed_out {
            self.windows.clear();
            views::shell::statusbar(self, ui, &pal);
            views::signin::show(self, ui, &pal);
        } else {
            self.follow_calls();
            views::main_window::show(self, ui, &pal);
            views::chats::new_chat_dialog(self, ctx, &pal);
            views::main_window::group_dialog(self, ctx);
            views::meetings::meeting_dialog(self, ctx);
            views::conv_window::show_all(self, ctx, &pal);
        }
        std::mem::take(&mut self.intents)
    }

    /// Every call that is not ringing in gets its window once; ended calls lose it.
    fn follow_calls(&mut self) {
        let ids: Vec<u32> = self
            .state
            .calls
            .iter()
            .filter(|c| c.state != crate::model::CallState::Incoming)
            .map(|c| c.id)
            .collect();
        for id in ids {
            if self.call_windows_opened.insert(id) {
                self.open_window(Win::Call(id));
            }
        }
        let live: BTreeSet<u32> = self.state.calls.iter().map(|c| c.id).collect();
        self.windows
            .retain(|w| !matches!(w, Win::Call(id) if !live.contains(id)));
    }
}
