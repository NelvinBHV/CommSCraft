//! The interface shell: the panels, the keyboard shortcuts and the intents for the adapter.

use crate::model::{AppState, Connection, ConvKey, Presence, View};
use crate::text::{Lang, Texts};
use crate::theme::{self, size, Palette};
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
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatFilter {
    #[default]
    All,
    Unread,
    Groups,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContactFilter {
    #[default]
    All,
    Online,
    Favorites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CallFilter {
    #[default]
    All,
    Missed,
}

/// Local preferences: favourites (addresses or group titles), contrast and scale. No chat
/// content is ever part of them.
#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    pub favorites: BTreeSet<String>,
    pub high_contrast: bool,
    pub scale: f32,
    pub context_open: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            favorites: BTreeSet::new(),
            high_contrast: false,
            scale: 1.0,
            context_open: true,
        }
    }
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
    pub(crate) chat_filter: ChatFilter,
    pub(crate) contact_filter: ContactFilter,
    pub(crate) call_filter: CallFilter,
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
            view: View::Chats,
            selected_conv: None,
            selected_contact: None,
            selected_meeting: None,
            chat_filter: ChatFilter::All,
            contact_filter: ContactFilter::All,
            call_filter: CallFilter::All,
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

    pub fn set_view(&mut self, v: View) {
        self.view = v;
    }

    pub fn selected_conversation(&self) -> Option<ConvKey> {
        self.selected_conv
    }

    pub fn select_conversation(&mut self, key: ConvKey) {
        self.view = View::Chats;
        self.selected_conv = Some(key);
        self.focus_composer = true;
        if let Some(c) = self.state.conversation_mut(key) {
            c.unread = 0;
        }
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
        Some(match &c.peer {
            Some(p) => p.clone(),
            None => format!("group:{}", c.title),
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
                self.view = v;
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
        // What is on screen is read: the open conversation has no unread messages.
        if self.view == View::Chats {
            if let Some(c) = self
                .selected_conv
                .and_then(|k| self.state.conversation_mut(k))
            {
                c.unread = 0;
            }
        }
        self.shortcuts(ctx);
        views::shell::statusbar(self, ui, &pal);
        views::shell::topbar(self, ui, &pal);
        let signed_out = matches!(
            self.state.connection,
            Connection::SignedOut | Connection::Connecting | Connection::Failed(_)
        ) && !self.demo;
        views::signin::trust_dialog(self, ctx, &pal);
        if signed_out {
            views::signin::show(self, ui, &pal);
        } else {
            views::shell::rail(self, ui, &pal);
            views::shell::list_panel(self, ui, &pal);
            let show_context = self.prefs.context_open
                && self.window_width >= size::CONTEXT_MIN_WINDOW
                && self.view != View::Settings;
            if show_context {
                views::shell::context_panel(self, ui, &pal);
            }
            views::shell::main_panel(self, ui, &pal);
            views::chats::new_chat_dialog(self, ctx, &pal);
        }
        std::mem::take(&mut self.intents)
    }
}
