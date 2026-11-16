//! The session adapter: one worker thread owns the SIP connection, the session engine and the
//! media threads of calls. The interface sends it [`ToWorker`] commands and receives typed
//! [`AppEvent`]s; nothing in the egui frame waits for the network.
//!
//! Security by design: TLS is always verified (system trust store); discovery uses only targets
//! in the user's SIP domain or domains the user trusts in `~/.config/commscraft/trusted-domains`
//! (the desktop client never asks in a dialog yet, so a foreign server is refused with a hint);
//! the password goes into the registrar's wiped container at once; both queues are bounded.
//! Calls go only to SIP addresses chosen in the interface, never to phone numbers, so the
//! emergency call path of the engine is not touched from here.

use commscraft_client::{calls, cues, setup};
use commscraft_net::{OsRandom, SipConnection, TlsClient};
use commscraft_presence as pres;
use commscraft_register::{Config as RegConfig, Password, Registrar};
use commscraft_session::{Command as C, Engine, Event as E, Output};
use commscraft_ui::app::SignIn;
use commscraft_ui::model::{
    AppEvent, Capabilities, Connection, Contact, ConvKey, Gate, Meeting, NoticeKind, Participant,
    Presence, Profile, Reason, Source,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Commands from the interface (conversation keys already resolved where known).
#[derive(Debug)]
pub enum ToWorker {
    SetPresence(Presence),
    StartConversation {
        key: ConvKey,
        peer: String,
    },
    SendText {
        key: ConvKey,
        session: Option<u32>,
        text: String,
    },
    Typing {
        key: ConvKey,
        session: Option<u32>,
        typing: bool,
    },
    EndConversation {
        key: ConvKey,
        session: Option<u32>,
    },
    StartGroup {
        invite: Vec<String>,
    },
    InviteToGroup {
        session: u32,
        peer: String,
    },
    JoinConference {
        focus: String,
    },
    RefreshMeetings,
    CallUser {
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
    /// Address book search on the server (MS-DLX).
    Search {
        text: String,
    },
    SignOut,
}

/// Commands waiting in the queue at most (the interface drops and tells when it is full).
const COMMAND_QUEUE: usize = 256;
/// Events waiting for the interface at most; the worker waits when it is full.
const EVENT_QUEUE: usize = 4096;
/// Commands taken per loop round, so incoming SIP is never starved.
const COMMANDS_PER_ROUND: usize = 32;
/// The registration must succeed within this time.
const SIGN_IN_LIMIT: Duration = Duration::from_secs(120);

pub struct Worker {
    tx: SyncSender<ToWorker>,
    rx: Receiver<AppEvent>,
}

impl Worker {
    /// Starts signing in. The password moves into the worker.
    pub fn start(form: SignIn, ca: Option<Vec<u8>>, repaint: Box<dyn Fn() + Send>) -> Self {
        let (tx, cmd_rx) = mpsc::sync_channel(COMMAND_QUEUE);
        let (ev_tx, rx) = mpsc::sync_channel(EVENT_QUEUE);
        std::thread::spawn(move || {
            let out = Out { tx: ev_tx, repaint };
            out.event(AppEvent::Connection(Connection::Connecting));
            match run(form, ca.as_deref(), &cmd_rx, &out) {
                Ok(()) => out.event(AppEvent::Connection(Connection::SignedOut)),
                Err(Ended::Failed(why)) => out.event(AppEvent::Connection(Connection::Failed(why))),
                Err(Ended::Lost(why)) => {
                    out.event(AppEvent::Connection(Connection::Disconnected(why)))
                }
            }
            out.event(AppEvent::Capabilities(Capabilities::offline()));
        });
        Self { tx, rx }
    }

    /// Hands a command to the worker; false when the queue is full or the worker ended.
    pub fn send(&self, c: ToWorker) -> bool {
        self.tx.try_send(c).is_ok()
    }

    /// The events that arrived (at most `max` per call).
    pub fn events(&self, max: usize) -> Vec<AppEvent> {
        self.rx.try_iter().take(max).collect()
    }
}

struct Out {
    tx: SyncSender<AppEvent>,
    repaint: Box<dyn Fn() + Send>,
}

impl Out {
    fn event(&self, e: AppEvent) {
        let _ = self.tx.send(e);
        (self.repaint)();
    }

    fn notice(&self, kind: NoticeKind, text: impl Into<String>) {
        self.event(AppEvent::Notice(kind, text.into(), now()));
    }
}

enum Ended {
    /// Before the registration: the sign-in failed.
    Failed(String),
    /// After it: the connection or registration was lost.
    Lost(String),
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn verbose() -> bool {
    std::env::var_os("COMMSCRAFT_VERBOSE").is_some()
}

/// With `COMMSCRAFT_VERBOSE`: one line per SIP message – method or status and CSeq, never
/// headers that could carry secrets.
fn trace(dir: &str, m: &commscraft_sip_msg::Message) {
    if !verbose() {
        return;
    }
    let what = match (m.method(), m.status_code()) {
        (Some(method), _) => method.to_string(),
        (None, Some(code)) => code.to_string(),
        _ => "?".into(),
    };
    let cseq = m
        .cseq()
        .ok()
        .flatten()
        .map(|c| format!("{} {}", c.number, c.method))
        .unwrap_or_default();
    eprintln!("{dir} {what} ({cseq})");
}

/// `host:port` (port 5061 when missing), host checked as a DNS name.
pub fn parse_server(v: &str) -> Option<(String, u16)> {
    let v = v.trim();
    let (host, port) = match v.rsplit_once(':') {
        Some((h, p)) => (h, p.parse().ok()?),
        None => (v, 5061),
    };
    let host = commscraft_url::dns_host(host)?;
    (port != 0).then_some((host, port))
}

/// The capabilities once signed in: what the desktop interface offers through the session.
pub fn online_capabilities() -> Capabilities {
    let cli = Gate::Unavailable(Reason::CliOnly);
    let none = Gate::Unavailable(Reason::NotImplemented);
    Capabilities {
        chat: Gate::Available,
        group_chat: Gate::Available,
        presence: Gate::Available,
        directory_search: Gate::Available,
        audio_call: Gate::Available,
        hold: Gate::Available,
        transfer: cli.clone(),
        video_call: cli.clone(),
        meetings: Gate::Available,
        file_transfer: none.clone(),
        rich_text: none.clone(),
        persistent_chat: cli,
        history_search: none.clone(),
        read_receipts: none,
    }
}

pub fn presence_of(class: pres::Availability, activity: Option<&str>) -> Presence {
    use pres::Availability as A;
    let in_call = activity.is_some_and(|a| a == "on-the-phone" || a == "in-a-conference");
    match class {
        A::Online => Presence::Available,
        A::Idle => Presence::Away,
        A::Busy | A::BusyIdle if in_call => Presence::InACall,
        A::Busy | A::BusyIdle => Presence::Busy,
        A::DoNotDisturb => Presence::DoNotDisturb,
        A::Away => Presence::Away,
        A::Offline => Presence::Offline,
        A::Unknown => Presence::Unknown,
    }
}

fn user_status(p: Presence) -> Option<pres::UserStatus> {
    Some(match p {
        Presence::Available => pres::UserStatus::Online,
        Presence::Busy | Presence::InACall => pres::UserStatus::Busy,
        Presence::DoNotDisturb => pres::UserStatus::DoNotDisturb,
        Presence::Away | Presence::BeRightBack => pres::UserStatus::Away,
        Presence::Offline | Presence::Unknown => return None,
    })
}

/// The contact list as interface contacts (group name from the first group).
pub fn contacts_of(l: &pres::ContactList) -> Vec<Contact> {
    let groups: BTreeMap<u32, &str> = l.groups.iter().map(|g| (g.id, g.name.as_str())).collect();
    l.contacts
        .iter()
        .map(|c| {
            let mut k = Contact::new(&c.uri, &c.name, Source::ContactList);
            k.group = c
                .groups
                .first()
                .and_then(|g| groups.get(g))
                .filter(|n| **n != "~")
                .map(|n| n.to_string())
                .unwrap_or_default();
            k
        })
        .collect()
}

fn meeting_of(m: &commscraft_conf::schedule::Meeting) -> Option<Meeting> {
    Some(Meeting {
        subject: m.subject.clone().unwrap_or_default(),
        focus: m.focus.clone()?,
        conference_id: m.conference_id.clone(),
        expires: m
            .expiry
            .as_deref()
            .and_then(commscraft_xml::parse_xs_datetime),
        demo: false,
    })
}

/// Is this the IM MCU of a conference (a group conversation) rather than a person?
fn is_conference_uri(uri: &str) -> bool {
    uri.contains("opaque=app:conf")
}

struct Session {
    engine: Engine<OsRandom>,
    conn: SipConnection,
    local_ip: std::net::IpAddr,
    /// Session conversation → our key (conversations we opened).
    opened: BTreeMap<u32, ConvKey>,
    keys: BTreeMap<ConvKey, u32>,
    groups: BTreeSet<u32>,
    group_titles: BTreeMap<u32, String>,
    services: Option<Box<commscraft_provisioning::ServerConfiguration>>,
    media_tx: mpsc::Sender<calls::FromMedia>,
    media: BTreeMap<u32, calls::MediaHandle>,
    media_of_call: BTreeMap<u32, u32>,
    pending_out: BTreeMap<u32, String>,
    pending_in: BTreeMap<u32, u32>,
    outgoing: BTreeSet<u32>,
    next_media: u32,
    cues: cues::Cues,
    signed_in: bool,
    signing_out: Option<Instant>,
    profile: Profile,
    tls: TlsClient,
    trust: setup::Trust,
}

/// Directory entries with a SIP address as interface contacts.
pub fn directory_contacts(found: &[commscraft_dlx::Entry]) -> Vec<Contact> {
    found
        .iter()
        .filter_map(|e| {
            let uri = e.sip_uri.as_deref()?;
            let mut c = Contact::new(
                uri,
                e.display_name.as_deref().unwrap_or(""),
                Source::Directory,
            );
            c.title = e.title.clone().unwrap_or_default();
            c.department = e.department.clone().unwrap_or_default();
            c.company = e.company.clone().unwrap_or_default();
            c.phone = e.phone.clone().unwrap_or_default();
            Some(c)
        })
        .collect()
}

fn run(form: SignIn, ca: Option<&[u8]>, cmds: &Receiver<ToWorker>, out: &Out) -> Result<(), Ended> {
    let fail = |s: String| Ended::Failed(s);
    let aor = form.address.clone();
    let user_domain = aor
        .rsplit_once('@')
        .map(|(_, d)| d.to_ascii_lowercase())
        .ok_or_else(|| fail("the SIP address has no domain".into()))?;
    let password = Password::new(form.password);
    let mut rng = OsRandom::new().map_err(|e| fail(e.to_string()))?;
    let epid = setup::load_or_create_epid(&mut rng).map_err(fail)?;
    let tls = TlsClient::new(ca).map_err(|e| fail(e.to_string()))?;
    let mut trust = setup::Trust::load(&[]);
    let targets = if form.server.trim().is_empty() {
        let mut foreign = Vec::new();
        let mut log = |line: &str| {
            if verbose() {
                eprintln!(
                    "{}",
                    line.chars().filter(|c| !c.is_control()).collect::<String>()
                );
            }
        };
        let mut confirm = |host: &str, _: &str, _: &str, _: &mut setup::Trust| {
            foreign.push(host.to_string());
            false
        };
        let t = setup::discover(
            &tls,
            &aor,
            &user_domain,
            &mut trust,
            &mut setup::Discovery {
                log: &mut log,
                verbose: false,
                confirm: &mut confirm,
            },
        )
        .map_err(fail)?;
        for host in foreign {
            out.notice(
                NoticeKind::Warning,
                format!(
                    "{host} is outside your SIP domain; to trust it, add {} to ~/.config/commscraft/trusted-domains",
                    setup::Trust::domain_of(&host)
                ),
            );
        }
        t
    } else {
        vec![parse_server(&form.server).ok_or_else(|| fail("server: host:port expected".into()))?]
    };
    if targets.is_empty() {
        return Err(fail("no SIP access point found for the domain".into()));
    }
    let mut conn = None;
    let mut last = String::new();
    for (host, port) in &targets {
        match tls.connect(host, *port, Duration::from_secs(10)) {
            Ok(c) => {
                conn = Some(c);
                break;
            }
            Err(e) => last = format!("{host}:{port}: {e}"),
        }
    }
    let mut conn = conn.ok_or_else(|| fail(format!("no server reachable ({last})")))?;
    let local = conn.local_addr().map_err(|e| fail(e.to_string()))?;
    let host = match local {
        std::net::SocketAddr::V4(v4) => v4.ip().to_string(),
        std::net::SocketAddr::V6(v6) => format!("[{}]", v6.ip()),
    };
    let ua = format!("CommSCraft/{}", env!("CARGO_PKG_VERSION"));
    let cfg = RegConfig {
        aor: aor.clone(),
        registrar_uri: format!("sip:{user_domain}"),
        epid: epid.clone(),
        local_host: host.clone(),
        local_port: local.port(),
        user_agent: ua.clone(),
        user: form.user.trim().to_string(),
        domain: form.domain.trim().to_string(),
        workstation: "COMMSCRAFT".into(),
        subnet: None,
        policy: commscraft_sipae::Policy::default(),
        ntlm_legacy: false,
    };
    let reg =
        Registrar::new(cfg, password, rng).map_err(|e| fail(format!("configuration: {e:?}")))?;
    let ecfg = commscraft_session::Config {
        aor: aor.clone(),
        epid,
        via_host: format!("{host}:{}", local.port()),
        local_ip: local.ip(),
        user_agent: ua,
        machine_name: "CommSCraft".into(),
    };
    conn.set_timeout(Duration::from_millis(100))
        .map_err(|e| fail(e.to_string()))?;
    let (media_tx, media_rx) = mpsc::channel();
    let (cues, cue_notices) = cues::Cues::from_env();
    for n in cue_notices {
        out.notice(NoticeKind::Info, n);
    }
    let mut s = Session {
        engine: Engine::new(ecfg, reg),
        conn,
        local_ip: local.ip(),
        opened: BTreeMap::new(),
        keys: BTreeMap::new(),
        groups: BTreeSet::new(),
        group_titles: BTreeMap::new(),
        services: None,
        media_tx,
        media: BTreeMap::new(),
        media_of_call: BTreeMap::new(),
        pending_out: BTreeMap::new(),
        pending_in: BTreeMap::new(),
        outgoing: BTreeSet::new(),
        next_media: 1,
        cues,
        signed_in: false,
        signing_out: None,
        profile: Profile::default(),
        tls,
        trust,
    };
    let started = Instant::now();
    let mut outputs = s.engine.start(now());
    let mut last_tick = Instant::now();
    loop {
        if let Some(done) = s.handle_outputs(std::mem::take(&mut outputs), out)? {
            s.stop_media();
            return done;
        }
        if !s.signed_in && started.elapsed() > SIGN_IN_LIMIT {
            return Err(fail("no answer to the sign-in within 120 s".into()));
        }
        if s.signing_out
            .is_some_and(|t| t.elapsed() > Duration::from_secs(10))
        {
            s.stop_media();
            return Ok(());
        }
        let mut search = None;
        for _ in 0..COMMANDS_PER_ROUND {
            match cmds.try_recv() {
                // Only the newest search of a round runs (the user is still typing).
                Ok(ToWorker::Search { text }) => search = Some(text),
                Ok(c) => outputs.extend(s.command(c, out)),
                Err(TryRecvError::Empty) => break,
                // The window closed: sign out.
                Err(TryRecvError::Disconnected) => {
                    if s.signing_out.is_none() {
                        outputs.extend(s.command(ToWorker::SignOut, out));
                    }
                    break;
                }
            }
        }
        if let Some(text) = search {
            s.search(&text, out);
        }
        while let Ok(m) = media_rx.try_recv() {
            outputs.extend(s.media_event(m, out));
        }
        match s.conn.recv() {
            Ok(Some(m)) => {
                trace("<<<", &m);
                outputs.extend(s.engine.on_message(&m, now()))
            }
            Ok(None) => {}
            Err(e) => {
                s.stop_media();
                let why = e.to_string();
                return Err(if s.signed_in {
                    Ended::Lost(why)
                } else {
                    fail(why)
                });
            }
        }
        if last_tick.elapsed() >= Duration::from_millis(250) {
            last_tick = Instant::now();
            outputs.extend(s.engine.on_tick(now()));
        }
    }
}

impl Session {
    fn policy(&self) -> commscraft_sdp::Encryption {
        calls::policy(self.services.as_ref().and_then(|s| s.av_encryption))
    }

    fn spawn_media(&mut self, peer: Option<&str>, offer_call: Option<u32>) -> u32 {
        let id = self.next_media;
        self.next_media = self.next_media.wrapping_add(1).max(1);
        let relay = self.engine.media_relay().and_then(calls::relay_from);
        let enabled = self
            .services
            .as_ref()
            .and_then(|s| s.enable_bw_policy_check)
            == Some(true);
        let bwm = match offer_call {
            Some(call) => calls::bandwidth_config(
                enabled,
                self.engine.own_aor(),
                self.engine.call_peer(call),
                self.engine.incoming_offer(call),
            ),
            None => calls::bandwidth_config(enabled, self.engine.own_aor(), peer, None),
        };
        let h = calls::spawn(
            id,
            self.local_ip,
            relay,
            bwm,
            commscraft_audio_alsa::Config::default(),
            self.media_tx.clone(),
        );
        self.media.insert(id, h);
        id
    }

    fn to_media(&self, call: u32, m: calls::ToMedia) {
        if let Some(h) = self
            .media_of_call
            .get(&call)
            .and_then(|id| self.media.get(id))
        {
            let _ = h.tx.send(m);
        }
    }

    fn stop_media(&mut self) {
        for (_, h) in std::mem::take(&mut self.media) {
            let _ = h.tx.send(calls::ToMedia::Stop);
        }
    }

    fn session_of(&self, key: ConvKey, session: Option<u32>) -> Option<u32> {
        session.or_else(|| self.keys.get(&key).copied())
    }

    fn command(&mut self, c: ToWorker, out: &Out) -> Vec<Output> {
        let t = now();
        match c {
            ToWorker::SetPresence(p) => match user_status(p) {
                Some(st) => self.engine.command(C::SetStatus(st), t),
                None => Vec::new(),
            },
            ToWorker::StartConversation { key, peer } => {
                let id = self.engine.next_conversation_id();
                self.opened.insert(id, key);
                self.keys.insert(key, id);
                self.engine.command(C::StartConversation { to: peer }, t)
            }
            ToWorker::SendText { key, session, text } => match self.session_of(key, session) {
                Some(conversation) => self.engine.command(C::SendText { conversation, text }, t),
                None => {
                    out.notice(
                        NoticeKind::Error,
                        "message not sent: the conversation is not open",
                    );
                    Vec::new()
                }
            },
            ToWorker::Typing {
                key,
                session,
                typing,
            } => match self.session_of(key, session) {
                Some(conversation) => self.engine.command(
                    C::Typing {
                        conversation,
                        typing,
                    },
                    t,
                ),
                None => Vec::new(),
            },
            ToWorker::EndConversation { key, session } => match self.session_of(key, session) {
                Some(conversation) => self.engine.command(C::EndConversation { conversation }, t),
                None => Vec::new(),
            },
            ToWorker::StartGroup { invite } => {
                let id = self.engine.next_conversation_id();
                self.groups.insert(id);
                self.engine.command(C::StartGroup { invite }, t)
            }
            ToWorker::InviteToGroup { session, peer } => self.engine.command(
                C::InviteToGroup {
                    conversation: session,
                    to: peer,
                },
                t,
            ),
            ToWorker::JoinConference { focus } => {
                let id = self.engine.next_conversation_id();
                self.groups.insert(id);
                self.engine.command(C::JoinConference { focus }, t)
            }
            ToWorker::RefreshMeetings => self.engine.command(
                C::ListMeetings {
                    static_meetings: false,
                },
                t,
            ),
            ToWorker::CallUser { peer } => {
                let id = self.spawn_media(Some(&peer), None);
                self.pending_out.insert(id, peer);
                Vec::new()
            }
            ToWorker::AnswerCall { id: call } => {
                self.cues.stop_ring(call);
                let id = self.spawn_media(None, Some(call));
                self.pending_in.insert(id, call);
                Vec::new()
            }
            ToWorker::HangUp { id } => {
                self.cues.stop_ring(id);
                self.engine.command(C::Hangup { call: id }, t)
            }
            ToWorker::Hold { id, hold } => {
                let music = self.cues.hold_music(false);
                let with_music = music.is_some();
                self.to_media(id, calls::ToMedia::Hold { hold, music });
                out.event(AppEvent::CallHeld { id, held: hold });
                self.engine.command(
                    C::Hold {
                        call: id,
                        hold,
                        music: with_music,
                    },
                    t,
                )
            }
            ToWorker::Mute { id, muted } => {
                self.to_media(id, calls::ToMedia::Mute(muted));
                out.event(AppEvent::CallMuted { id, muted });
                Vec::new()
            }
            ToWorker::Search { text } => {
                self.search(&text, out);
                Vec::new()
            }
            ToWorker::SignOut => {
                self.signing_out = Some(Instant::now());
                self.stop_media();
                self.engine.command(C::SignOut, t)
            }
        }
    }

    /// The address book search runs here, bounded by the HTTP timeouts (15 s); incoming SIP
    /// waits in the socket meanwhile.
    fn search(&mut self, text: &str, out: &Out) {
        let ua = format!("CommSCraft/{}", env!("CARGO_PKG_VERSION"));
        let engine = &mut self.engine;
        let mut ntlm =
            |n: &[u8], c: &[u8], cert: Option<&[u8]>| engine.http_ntlm(n, c, now(), cert);
        let mut log = |l: &str| {
            if verbose() {
                eprintln!("{l}");
            }
        };
        match commscraft_client::directory::search(
            &self.tls,
            &self.trust,
            self.services.as_deref(),
            text,
            &ua,
            &mut ntlm,
            &mut log,
        ) {
            Ok(found) => out.event(AppEvent::SearchResults(directory_contacts(&found))),
            Err(e) => {
                out.event(AppEvent::SearchResults(Vec::new()));
                out.notice(NoticeKind::Warning, format!("address book search: {e}"));
            }
        }
    }

    fn media_event(&mut self, m: calls::FromMedia, out: &Out) -> Vec<Output> {
        let t = now();
        let policy = self.policy();
        match m {
            calls::FromMedia::Ready(id, local) => {
                if let Some(to) = self.pending_out.remove(&id) {
                    let call = self.engine.next_call_id();
                    self.media_of_call.insert(call, id);
                    self.outgoing.insert(call);
                    out.event(AppEvent::CallStarted {
                        id: call,
                        peer: to.clone(),
                        incoming: false,
                        at: t,
                    });
                    self.engine.command(C::Call { to, local, policy }, t)
                } else if let Some(call) = self.pending_in.remove(&id) {
                    self.media_of_call.insert(call, id);
                    self.engine.command(
                        C::AcceptCall {
                            call,
                            local,
                            policy,
                        },
                        t,
                    )
                } else {
                    Vec::new()
                }
            }
            calls::FromMedia::Selected(id, local) => {
                match self.media_of_call.iter().find(|(_, m)| **m == id) {
                    Some((&call, _)) => self.engine.command(C::UpdateCall { call, local }, t),
                    None => Vec::new(),
                }
            }
            calls::FromMedia::Notice(_, what) => {
                out.notice(NoticeKind::Warning, what);
                Vec::new()
            }
            calls::FromMedia::Failed(id, why) => {
                out.notice(NoticeKind::Error, format!("call media failed: {why}"));
                if let Some(h) = self.media.remove(&id) {
                    let _ = h.tx.send(calls::ToMedia::Stop);
                }
                match self.media_of_call.iter().find(|(_, m)| **m == id) {
                    Some((&call, _)) => self.engine.command(C::Hangup { call }, t),
                    None => {
                        self.pending_out.remove(&id);
                        self.pending_in.remove(&id);
                        Vec::new()
                    }
                }
            }
            calls::FromMedia::PeerBye(_)
            | calls::FromMedia::Dtmf(..)
            | calls::FromMedia::Quality(..) => Vec::new(),
        }
    }

    fn group_title(&self, session: u32) -> String {
        self.group_titles
            .get(&session)
            .cloned()
            .unwrap_or_else(|| "Gruppe".into())
    }

    /// Engine outputs: SIP to the server, events to the interface. `Some` ends the session.
    fn handle_outputs(
        &mut self,
        outputs: Vec<Output>,
        out: &Out,
    ) -> Result<Option<Result<(), Ended>>, Ended> {
        let mut more = Vec::new();
        for o in outputs {
            match o {
                Output::Send(m) => {
                    trace(">>>", &m);
                    if let Err(e) = self.conn.send(&m) {
                        return Err(Ended::Lost(e.to_string()));
                    }
                }
                Output::SendRaw(b) => {
                    if let Err(e) = self.conn.send_raw(b) {
                        return Err(Ended::Lost(e.to_string()));
                    }
                }
                Output::Event(e) => {
                    if let Some(done) = self.event(e, out, &mut more) {
                        return Ok(Some(done));
                    }
                }
            }
        }
        if more.is_empty() {
            Ok(None)
        } else {
            self.handle_outputs(more, out)
        }
    }

    fn event(&mut self, e: E, out: &Out, more: &mut Vec<Output>) -> Option<Result<(), Ended>> {
        let t = now();
        match e {
            E::Registered(_) => {
                if !self.signed_in {
                    self.signed_in = true;
                    let aor = self.engine.aor().to_string();
                    let name = aor
                        .trim_start_matches("sip:")
                        .split('@')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let organisation = aor.rsplit_once('@').map(|(_, d)| d.to_string()).unwrap_or_default();
                    self.profile = Profile {
                        uri: aor,
                        name,
                        presence: Presence::Available,
                        organisation,
                    };
                    out.event(AppEvent::SignedIn(self.profile.clone()));
                    out.event(AppEvent::Capabilities(online_capabilities()));
                    out.event(AppEvent::Connection(Connection::Connected));
                }
            }
            E::SignedOut => return Some(Ok(())),
            E::RegistrationEnded(why) => return Some(Err(Ended::Lost(why))),
            E::CertificateNeeded { .. } => {
                return Some(Err(Ended::Failed(
                    "the server asks for TLS-DSK; the desktop client does not offer it yet (commscraft-cli --tls-dsk does)".into(),
                )))
            }
            E::Provisioned(c) => self.services = Some(c),
            E::ContactList(l) => out.event(AppEvent::Contacts(contacts_of(&l))),
            E::ContactsInExchange => out.notice(
                NoticeKind::Info,
                "the contact list is kept in Exchange; the desktop client does not read it yet",
            ),
            E::Presence(p) => out.event(AppEvent::Presence {
                presence: p
                    .availability
                    .map(|_| presence_of(p.class(), p.activity.as_deref())),
                note: p.note.clone().unwrap_or_default(),
                name: p.display_name.clone(),
                uri: p.uri,
            }),
            E::PresenceUnavailable { uri, .. } => out.event(AppEvent::Presence {
                uri,
                presence: Some(Presence::Unknown),
                note: String::new(),
                name: None,
            }),
            E::OwnStatus(s) => out.event(AppEvent::OwnPresence(presence_of(
                pres::Availability::from_number(s.availability()),
                None,
            ))),
            E::OwnStatusFailed(code) => {
                out.notice(NoticeKind::Error, format!("status not changed ({code})"))
            }
            E::ConversationStarted {
                conversation,
                peer,
                incoming,
            } => {
                if self.groups.contains(&conversation) || is_conference_uri(&peer) {
                    self.groups.insert(conversation);
                    out.event(AppEvent::GroupStarted {
                        session: conversation,
                        title: self.group_title(conversation),
                    });
                } else {
                    out.event(AppEvent::ConversationStarted {
                        session: conversation,
                        peer,
                        incoming,
                    });
                }
            }
            E::ConversationFailed {
                conversation,
                status,
            } => match self.opened.remove(&conversation) {
                Some(key) => out.event(AppEvent::OpeningFailed { key, status }),
                None => out.event(AppEvent::ConversationFailed {
                    session: conversation,
                    status,
                }),
            },
            E::Text {
                conversation,
                sender,
                text,
            } => {
                self.cues.notify(now_ms());
                out.event(AppEvent::Text {
                    session: conversation,
                    sender,
                    text,
                    at: t,
                })
            }
            E::Typing {
                conversation,
                typing,
            } => out.event(AppEvent::Typing {
                session: conversation,
                typing,
            }),
            E::Delivered { conversation } => out.event(AppEvent::Delivered {
                session: conversation,
            }),
            E::NotDelivered {
                conversation,
                status,
            } => out.event(AppEvent::NotDelivered {
                session: conversation,
                status,
            }),
            E::NotDeliveredTo {
                conversation,
                recipient,
                status,
            } => out.event(AppEvent::NotDeliveredTo {
                session: conversation,
                recipient,
                status,
            }),
            E::ConversationEnded { conversation } => {
                self.opened.remove(&conversation);
                self.keys.retain(|_, v| *v != conversation);
                out.event(AppEvent::ConversationEnded {
                    session: conversation,
                })
            }
            E::Participants {
                conversation,
                participants,
            } => {
                let own = self.engine.aor().to_string();
                // Our display name as the conference knows it.
                if let Some(n) = participants
                    .iter()
                    .find(|p| p.uri == own)
                    .and_then(|p| p.name.clone())
                    .filter(|n| !n.trim().is_empty() && *n != self.profile.name)
                {
                    self.profile.name = n;
                    out.event(AppEvent::SignedIn(self.profile.clone()));
                }
                let others: Vec<String> = participants
                    .iter()
                    .filter(|p| p.uri != own && p.in_chat())
                    .map(|p| p.name.clone().unwrap_or_else(|| p.uri.trim_start_matches("sip:").to_string()))
                    .take(3)
                    .collect();
                if !others.is_empty() {
                    self.group_titles.insert(conversation, others.join(", "));
                }
                self.groups.insert(conversation);
                out.event(AppEvent::GroupStarted {
                    session: conversation,
                    title: self.group_title(conversation),
                });
                out.event(AppEvent::Participants {
                    session: conversation,
                    participants: participants
                        .iter()
                        .map(|p| Participant {
                            uri: p.uri.clone(),
                            name: p.name.clone().unwrap_or_default(),
                            presenter: p.roles.iter().any(|r| r == "presenter"),
                        })
                        .collect(),
                });
            }
            E::ConferenceInvitation { from, invitation } => {
                self.cues.notify(now_ms());
                out.event(AppEvent::Invitation(commscraft_ui::model::Invitation {
                    from,
                    focus: invitation.focus,
                    subject: invitation.subject.unwrap_or_default(),
                }))
            }
            E::ConferenceLobby { waiting: true, .. } => {
                out.notice(NoticeKind::Info, "waiting in the lobby until a presenter admits you")
            }
            E::Meetings(list) => out.event(AppEvent::Meetings(
                list.iter().filter_map(meeting_of).collect(),
            )),
            E::MeetingScheduled(m) | E::MeetingDetails(m) => {
                if let Some(m) = meeting_of(&m) {
                    out.notice(NoticeKind::Info, format!("meeting {}", m.conference_id));
                }
            }
            E::IncomingCall { call, from } => {
                self.cues.start_ring(call);
                out.event(AppEvent::CallStarted {
                    id: call,
                    peer: from,
                    incoming: true,
                    at: t,
                })
            }
            E::CallRinging { call } => out.event(AppEvent::CallRinging { id: call }),
            E::CallAnswered {
                call,
                media,
                continues_early,
            } => {
                self.cues.stop_ring(call);
                out.event(AppEvent::CallConnected { id: call, at: t });
                let m = if continues_early {
                    calls::ToMedia::Confirm(media)
                } else {
                    calls::ToMedia::Start {
                        media,
                        controlling: self.outgoing.contains(&call),
                        early: false,
                    }
                };
                self.to_media(call, m);
            }
            E::CallEarlyMedia { call, media } => self.to_media(
                call,
                calls::ToMedia::Start {
                    media,
                    controlling: true,
                    early: true,
                },
            ),
            E::CallEarlyMediaEnded { call } => self.to_media(call, calls::ToMedia::EndEarly),
            E::CallUpdated { call, media } => self.to_media(call, calls::ToMedia::Update(media)),
            E::CallEnded { call, status } => {
                self.cues.stop_ring(call);
                if let Some(id) = self.media_of_call.remove(&call) {
                    if let Some(h) = self.media.remove(&id) {
                        let _ = h.tx.send(calls::ToMedia::Stop);
                    }
                }
                self.outgoing.remove(&call);
                out.event(AppEvent::CallEnded {
                    id: call,
                    status,
                    at: t,
                })
            }
            E::TransferRequested { call, .. } => {
                out.notice(
                    NoticeKind::Info,
                    "the other side wants to transfer the call; the desktop client declines (commscraft-cli can follow)",
                );
                more.extend(self.engine.command(C::DeclineTransfer { call }, t));
            }
            E::IncomingScreenShare { call, .. } => {
                out.notice(
                    NoticeKind::Info,
                    "someone shares a screen; viewing is not in the desktop client yet (commscraft-cli /view)",
                );
                more.extend(self.engine.command(C::Hangup { call }, t));
            }
            E::Diagnostic {
                method,
                status,
                diagnostic,
            } => {
                let why = commscraft_ocer::explain(diagnostic.id)
                    .map(str::to_string)
                    .or(diagnostic.reason)
                    .unwrap_or_default();
                out.notice(
                    NoticeKind::Warning,
                    format!("{method} {status}: {why}"),
                );
            }
            E::Notice(n) => out.notice(NoticeKind::Info, n),
            // Not shown by the desktop interface yet.
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn servers() {
        assert_eq!(
            parse_server("sip.beispiel.invalid:5061"),
            Some(("sip.beispiel.invalid".into(), 5061))
        );
        assert_eq!(
            parse_server("sip.beispiel.invalid"),
            Some(("sip.beispiel.invalid".into(), 5061))
        );
        assert_eq!(parse_server("sip.beispiel.invalid:0"), None);
        assert_eq!(parse_server("bad host:443"), None);
    }

    #[test]
    fn presence_classes() {
        use pres::Availability as A;
        assert_eq!(presence_of(A::Online, None), Presence::Available);
        assert_eq!(
            presence_of(A::Busy, Some("on-the-phone")),
            Presence::InACall
        );
        assert_eq!(presence_of(A::Busy, None), Presence::Busy);
        assert_eq!(presence_of(A::Idle, None), Presence::Away);
        assert_eq!(presence_of(A::Unknown, None), Presence::Unknown);
        assert_eq!(user_status(Presence::Offline), None);
        assert_eq!(
            user_status(Presence::BeRightBack),
            Some(pres::UserStatus::Away)
        );
    }

    #[test]
    fn contact_list_groups() {
        let l = pres::ContactList {
            groups: vec![
                pres::Group {
                    id: 1,
                    name: "~".into(),
                },
                pres::Group {
                    id: 2,
                    name: "Team".into(),
                },
            ],
            contacts: vec![
                pres::Contact {
                    uri: "sip:a@beispiel.invalid".into(),
                    name: "A".into(),
                    groups: vec![2],
                    subscribed: true,
                    extension: false,
                },
                pres::Contact {
                    uri: "sip:b@beispiel.invalid".into(),
                    name: String::new(),
                    groups: vec![1],
                    subscribed: true,
                    extension: false,
                },
            ],
            ..Default::default()
        };
        let c = contacts_of(&l);
        assert_eq!(c[0].group, "Team");
        assert_eq!(c[1].group, "");
        assert_eq!(c[1].display_name(), "b@beispiel.invalid");
    }

    #[test]
    fn conference_peers() {
        assert!(is_conference_uri(
            "sip:a@beispiel.invalid;gruu;opaque=app:conf:chat:id:X"
        ));
        assert!(!is_conference_uri("sip:a@beispiel.invalid"));
    }

    #[test]
    fn the_desktop_gates_match_what_it_wires() {
        let c = online_capabilities();
        assert!(c.chat.usable() && c.audio_call.usable() && c.meetings.usable());
        assert!(!c.file_transfer.usable());
        assert!(!c.read_receipts.usable());
        assert!(!c.video_call.usable());
    }

    #[test]
    fn directory_entries_without_address_are_skipped() {
        let e = |uri: Option<&str>, name: &str| commscraft_dlx::Entry {
            display_name: Some(name.into()),
            sip_uri: uri.map(str::to_string),
            mail: None,
            title: Some("Einkauf".into()),
            department: None,
            company: None,
            phone: None,
            photo: None,
        };
        let c = directory_contacts(&[e(Some("sip:x@beispiel.invalid"), "X"), e(None, "Y")]);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].source, Source::Directory);
        assert_eq!(c[0].title, "Einkauf");
    }
}
