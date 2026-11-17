//! The session adapter: one worker thread owns the SIP connection, the session engine and the
//! media threads of calls. The interface sends it [`ToWorker`] commands and receives typed
//! [`AppEvent`]s; nothing in the egui frame waits for the network.
//!
//! Security by design: TLS is always verified (system trust store); discovery uses only targets
//! in the user's SIP domain, domains the user trusts in `~/.config/commscraft/trusted-domains`,
//! or an Autodiscover target the user trusts in the dialog (remembered there);
//! the password goes into the registrar's wiped container at once; both queues are bounded.
//! Calls go only to SIP addresses chosen in the interface, never to phone numbers, so the
//! emergency call path of the engine is not touched from here.

use commscraft_client::{calls, cues, meeting_content, rdp_call, setup, video_call};
use commscraft_net::{OsRandom, SipConnection, TlsClient};
use commscraft_presence as pres;
use commscraft_register::{Config as RegConfig, Password, Registrar};
use commscraft_session::{Command as C, Engine, Event as E, Output};
use commscraft_ui::app::SignIn;
use commscraft_ui::model::{
    AppEvent, Appointment, Busy, CallKind, Capabilities, Connection, Contact, ConvKey, Gate,
    Meeting, NoticeKind, Participant, Presence, Profile, Reason, RoomPost, Source, VideoState,
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
    /// The user's answer to a trust question of discovery.
    Trust(bool),
    /// Blind transfer (REFER) to an address.
    Transfer {
        id: u32,
        peer: String,
    },
    /// Address book search on the server (MS-DLX).
    Search {
        text: String,
    },
    /// A call with audio and video (function 28).
    VideoCall {
        peer: String,
    },
    AnswerVideo {
        id: u32,
    },
    /// View the screen or window someone shares (function 43).
    ViewScreen {
        id: u32,
    },
    /// The content of a meeting conversation (function 44).
    JoinContent {
        session: u32,
    },
    LeaveContent {
        session: u32,
    },
    Vote {
        session: u32,
        content: i64,
        choice: i64,
    },
    /// Local settings the worker follows: the camera (empty: the default), sounds.
    Settings {
        camera: String,
        sounds: bool,
        quiet_busy: bool,
    },
    /// Tones in a call (DTMF, RFC 4733).
    Dtmf {
        id: u32,
        digits: String,
    },
    /// The members of a distribution group (address book).
    ExpandGroup {
        email: String,
    },
    /// A change of the contact list or of a privacy relationship, by names as the interface
    /// knows them.
    Contacts(ContactChange),
    /// Persistent chat: connect, list our rooms, join them.
    LoadRooms,
    SendToRoom {
        room: String,
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

/// The capabilities once signed in: what the desktop interface offers through the session;
/// persistent chat only when provisioning named a chat pool.
pub fn online_capabilities(rooms: bool) -> Capabilities {
    let none = Gate::Unavailable(Reason::NotImplemented);
    Capabilities {
        chat: Gate::Available,
        group_chat: Gate::Available,
        presence: Gate::Available,
        directory_search: Gate::Available,
        audio_call: Gate::Available,
        hold: Gate::Available,
        transfer: Gate::Available,
        video_call: Gate::Available,
        meetings: Gate::Available,
        file_transfer: none.clone(),
        rich_text: none.clone(),
        meeting_content: Gate::Available,
        persistent_chat: if rooms {
            Gate::Available
        } else {
            Gate::Unavailable(Reason::ServerDisabled)
        },
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
    let pinned: BTreeSet<u32> = l
        .groups
        .iter()
        .filter(|g| pres::group_extension(&g.external) == pres::GroupExtension::Pinned)
        .map(|g| g.id)
        .collect();
    let groups: BTreeMap<u32, &str> = l
        .groups
        .iter()
        .filter(|g| pres::group_extension(&g.external) == pres::GroupExtension::Plain)
        .map(|g| (g.id, g.name.as_str()))
        .collect();
    l.contacts
        .iter()
        .map(|c| {
            let mut k = Contact::new(&c.uri, &c.name, Source::ContactList);
            k.pinned = c.groups.iter().any(|g| pinned.contains(g));
            k.group = c
                .groups
                .iter()
                .find_map(|g| groups.get(g))
                .filter(|n| **n != "~")
                .map(|n| n.to_string())
                .unwrap_or_default();
            k
        })
        .collect()
}

/// The distribution groups of the contact list: name and mail address (interop, see
/// `presence::group_extension`).
pub fn distribution_groups_of(l: &pres::ContactList) -> Vec<(String, String)> {
    l.groups
        .iter()
        .filter_map(|g| match pres::group_extension(&g.external) {
            pres::GroupExtension::Distribution { email } => Some((g.name.clone(), email)),
            _ => None,
        })
        .collect()
}

/// Members of a distribution group as interface contacts (those with a SIP address).
pub fn members_of(x: &commscraft_dlx::Expansion) -> Vec<Contact> {
    x.users
        .iter()
        .filter_map(|m| {
            let uri = m.sip_uri.as_deref()?;
            Some(Contact::new(
                uri,
                m.display_name.as_deref().unwrap_or(""),
                Source::Directory,
            ))
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
    certificate_tried: bool,
    /// The sign-in came with a password (a certificate can be got with it).
    password_given: bool,
    /// Rooms whose history was asked for (once per room and sign-in).
    room_history: BTreeSet<String>,
    v: Videos,
    /// Ringing and alert sounds (the options); fewer while busy.
    sounds: bool,
    quiet_busy: bool,
    content_tx: mpsc::Sender<meeting_content::FromContent>,
    /// Meeting content connections by conversation, with the users they named.
    content: BTreeMap<u32, (meeting_content::ContentHandle, BTreeMap<i64, String>)>,
    /// The thread that fetches contact photos (started once signed in).
    photos: Option<crate::photos::Photos>,
    photo_ntlm: mpsc::Sender<crate::photos::NtlmAsk>,
    photo_done: mpsc::Sender<crate::photos::Done>,
    /// When the appointments were last asked for.
    calendar_at: Option<Instant>,
    /// The contact list's addresses (their photos are asked for once provisioning is known).
    contact_uris: Vec<String>,
    /// The last contact list (group IDs for changes from the interface).
    contact_list: Option<pres::ContactList>,
    /// The Exchange identifiers behind a contact list kept in Exchange.
    ucs_ids: commscraft_client::exchange::UcsIds,
    /// Contact lists from Exchange handed to the engine and not yet seen back as its event.
    exchange_lists: u32,
    tls: TlsClient,
    trust: setup::Trust,
}

/// A contact list change asked for by the interface.
#[derive(Debug)]
pub enum ContactChange {
    Add {
        uri: String,
        name: String,
        group: Option<String>,
    },
    Remove {
        uri: String,
    },
    Move {
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
    Relation {
        uri: String,
        relation: Option<commscraft_ui::model::Relation>,
    },
    Acknowledge {
        uri: String,
    },
}

/// The engine's change for an interface change, with the group IDs of the last contact list.
pub fn contact_command(l: Option<&pres::ContactList>, c: ContactChange) -> Result<C, String> {
    use commscraft_ui::model::Relation as R;
    let plain = |name: &str| {
        l.and_then(|l| {
            l.groups
                .iter()
                .find(|g| {
                    g.name == name
                        && pres::group_extension(&g.external) == pres::GroupExtension::Plain
                })
                .map(|g| g.id)
        })
    };
    let default_group = || {
        l.and_then(|l| l.groups.iter().find(|g| g.name == "~").or(l.groups.first()))
            .map_or(1, |g| g.id)
    };
    Ok(match c {
        ContactChange::Add { uri, name, group } => C::EditContacts(pres::ContactEdit::Set {
            uri,
            name,
            groups: vec![group
                .as_deref()
                .and_then(plain)
                .unwrap_or_else(default_group)],
            subscribed: true,
        }),
        ContactChange::Remove { uri } => C::EditContacts(pres::ContactEdit::Delete { uri }),
        ContactChange::Move { uri, group } => {
            let l = l.ok_or("the contact list is not loaded yet")?;
            let c = l
                .contacts
                .iter()
                .find(|c| c.uri == uri)
                .ok_or("not in the contact list")?;
            let target = plain(&group).ok_or("no such group")?;
            // The favourites (and other special groups) stay; the plain groups are replaced.
            let mut groups: Vec<u32> = c
                .groups
                .iter()
                .copied()
                .filter(|g| {
                    l.groups.iter().any(|x| {
                        x.id == *g
                            && pres::group_extension(&x.external) != pres::GroupExtension::Plain
                    })
                })
                .collect();
            groups.push(target);
            C::EditContacts(pres::ContactEdit::Set {
                uri,
                name: c.name.clone(),
                groups,
                subscribed: c.subscribed,
            })
        }
        ContactChange::AddGroup { name } => C::EditContacts(pres::ContactEdit::AddGroup { name }),
        ContactChange::RenameGroup { name, to } => {
            C::EditContacts(pres::ContactEdit::RenameGroup {
                id: plain(&name).ok_or("no such group")?,
                name: to,
            })
        }
        ContactChange::DeleteGroup { name } => C::EditContacts(pres::ContactEdit::DeleteGroup {
            id: plain(&name).ok_or("no such group")?,
        }),
        ContactChange::Relation { uri, relation } => C::SetRelationship {
            uri,
            relationship: relation.map(|r| match r {
                R::External => pres::Relationship::External,
                R::Colleagues => pres::Relationship::Colleagues,
                R::Workgroup => pres::Relationship::Workgroup,
                R::Family => pres::Relationship::Family,
                R::Blocked => pres::Relationship::Blocked,
            }),
        },
        ContactChange::Acknowledge { uri } => C::AcknowledgeSubscriber { uri },
    })
}

/// The video streams and RDP viewers of the calls (function 28, function 43).
struct Videos {
    video_tx: mpsc::Sender<video_call::FromVideo>,
    rdp_tx: mpsc::Sender<rdp_call::FromRdp>,
    /// The camera device (`COMMSCRAFT_CAMERA`, else `/dev/video0`); opened only once a video
    /// stream is negotiated.
    camera: String,
    handles: BTreeMap<u32, video_call::VideoHandle>,
    of_call: BTreeMap<u32, u32>,
    /// Audio stream → video stream of a video call that is not sent yet.
    for_audio: BTreeMap<u32, u32>,
    ready_audio: BTreeMap<u32, Box<commscraft_sdp::Local>>,
    ready_video: BTreeMap<u32, Box<commscraft_sdp::video::VideoLocal>>,
    /// Video stream → the screen sharing call it will view.
    pending_screen: BTreeMap<u32, u32>,
    /// Offered screen shares: call → only RDP offered.
    screen_offers: BTreeMap<u32, bool>,
    rdp: BTreeMap<u32, rdp_call::RdpHandle>,
    pending_rdp: BTreeMap<u32, u32>,
    rdp_call_of: BTreeMap<u32, u32>,
    /// Screen shares we view whose stream is not negotiated yet: they have no audio answer, so
    /// the negotiated stream is what connects them.
    viewing: BTreeSet<u32>,
}

impl Videos {
    fn new(
        video_tx: mpsc::Sender<video_call::FromVideo>,
        rdp_tx: mpsc::Sender<rdp_call::FromRdp>,
    ) -> Self {
        Self {
            video_tx,
            rdp_tx,
            camera: std::env::var("COMMSCRAFT_CAMERA").unwrap_or_else(|_| "/dev/video0".into()),
            handles: BTreeMap::new(),
            of_call: BTreeMap::new(),
            for_audio: BTreeMap::new(),
            ready_audio: BTreeMap::new(),
            ready_video: BTreeMap::new(),
            pending_screen: BTreeMap::new(),
            screen_offers: BTreeMap::new(),
            rdp: BTreeMap::new(),
            pending_rdp: BTreeMap::new(),
            rdp_call_of: BTreeMap::new(),
            viewing: BTreeSet::new(),
        }
    }
}

/// The slide number of a slide location (`<PptLocation><SlideNumber>…`, [MS-PSOM] §3.1.4.1.11,
/// p. 41) as "slide N", else the text cut short.
pub fn slide_of(location: &str) -> String {
    let number = location
        .split_once("SlideNumber>")
        .and_then(|(_, rest)| rest.split_once('<'))
        .map(|(n, _)| n.trim())
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    match number {
        Some(n) => format!("slide {n}"),
        None => location.chars().take(60).collect(),
    }
}

/// An appointment for the interface (subject and location only where the event is not private).
pub fn appointment_of(e: &commscraft_ews::calendar::CalendarEvent) -> Appointment {
    use commscraft_ews::calendar::FreeBusy as F;
    Appointment {
        start: e.start,
        end: e.end,
        busy: match e.busy {
            F::Free | F::NoData => Busy::Free,
            F::Tentative => Busy::Tentative,
            F::Busy => Busy::Busy,
            F::Oof => Busy::OutOfOffice,
        },
        subject: e.subject.clone().unwrap_or_default(),
        location: e.location.clone().unwrap_or_default(),
    }
}

/// One line of the trace (`COMMSCRAFT_VERBOSE`).
fn trace_line(l: &str) {
    if verbose() {
        eprintln!("{l}");
    }
}

/// A room of the chat server as an interface event.
pub fn room_event(r: &commscraft_xccos::Room, joined: bool) -> AppEvent {
    AppEvent::Room {
        uri: r.uri.clone(),
        name: r.name.clone(),
        topic: if r.topic.is_empty() {
            r.description.clone()
        } else {
            r.topic.clone()
        },
        participants: r.participants.clone(),
        joined,
    }
}

/// A room message for the interface; the time is the server's (xs:dateTime), else now.
pub fn room_post(m: &commscraft_xccos::ChatMessage) -> RoomPost {
    RoomPost {
        author: m.author.clone(),
        name: m.author_name.clone(),
        text: m.text.clone(),
        at: commscraft_xml::parse_xs_datetime(&m.time).unwrap_or_else(now),
        chat_id: m.chat_id,
        ours: m.ours,
    }
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
        let mut log = |line: &str| {
            if verbose() {
                eprintln!(
                    "{}",
                    line.chars().filter(|c| !c.is_control()).collect::<String>()
                );
            }
        };
        // A server outside the SIP domain: the user decides in a dialog (as the CLI asks on the
        // terminal); without an answer within two minutes it is not trusted.
        let mut confirm =
            |host: &str, user_domain: &str, named_by: &str, trust: &mut setup::Trust| {
                let domain = setup::Trust::domain_of(host);
                out.event(AppEvent::TrustQuestion(
                    commscraft_ui::model::TrustQuestion {
                        host: host.to_string(),
                        domain: domain.clone(),
                        user_domain: user_domain.to_string(),
                        named_by: named_by.to_string(),
                    },
                ));
                let until = Instant::now() + Duration::from_secs(120);
                let yes = loop {
                    let left = until.saturating_duration_since(Instant::now());
                    match cmds.recv_timeout(left) {
                        Ok(ToWorker::Trust(a)) => break a,
                        Ok(ToWorker::SignOut) | Err(_) => break false,
                        Ok(_) => {}
                    }
                };
                if yes {
                    trust.remember(&domain);
                }
                yes
            };
        setup::discover(
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
        .map_err(fail)?
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
    // Without a user name the sign-in address is the account, as the Windows client does it:
    // the user principal name `name@domain` with an empty NTLM domain (interop: Active
    // Directory takes the UPN as NTLM user name, `tested-in-lab`).
    let user = match form.user.trim() {
        "" => aor.trim_start_matches("sip:").to_string(),
        u => u.to_string(),
    };
    let ntlm_domain = if form.user.trim().is_empty() {
        String::new()
    } else {
        form.domain.trim().to_string()
    };
    let password_given = !password.reveal().is_empty();
    let cfg = RegConfig {
        aor: aor.clone(),
        registrar_uri: format!("sip:{user_domain}"),
        epid: epid.clone(),
        local_host: host.clone(),
        local_port: local.port(),
        user_agent: ua.clone(),
        user: user.clone(),
        domain: ntlm_domain,
        workstation: "COMMSCRAFT".into(),
        subnet: None,
        policy: commscraft_sipae::Policy::default(),
        ntlm_legacy: false,
    };
    // Function 5: the principal is the account in the realm of the SIP domain (realm = DNS
    // domain in capitals, the usual Active Directory layout; inference, as in the CLI); the
    // ticket cache is used when it holds a ticket for it, else the password.
    let kerberos = form.kerberos.then(|| {
        Box::new(commscraft_gssapi::Kerberos::new(
            commscraft_gssapi::Credentials::Password {
                principal: match user.split_once('@') {
                    // A UPN: the realm is its domain in capitals.
                    Some((name, realm)) => format!("{name}@{}", realm.to_ascii_uppercase()),
                    None => format!("{user}@{}", user_domain.to_ascii_uppercase()),
                },
                password: zeroize::Zeroizing::new(password.reveal().to_string()),
            },
        ))
    });
    let mut reg =
        Registrar::new(cfg, password, rng).map_err(|e| fail(format!("configuration: {e:?}")))?;
    if let Some(k) = kerberos {
        reg = reg.with_kerberos(k);
    }
    // Function 6: the account's user certificate and the trust anchors of the connection.
    if form.tls_dsk {
        let dir = commscraft_client::tlsdsk::identity_dir(&aor)
            .ok_or_else(|| fail("no place for the TLS-DSK certificate".into()))?;
        reg = reg.with_tls_dsk(Box::new(commscraft_tlsdsk::TlsDsk::new(dir, tls.roots())));
    }
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
    let (video_tx, video_rx) = mpsc::channel();
    let (rdp_tx, rdp_rx) = mpsc::channel();
    let (content_tx, content_rx) = mpsc::channel();
    let (ntlm_tx, ntlm_rx) = mpsc::channel::<crate::photos::NtlmAsk>();
    let (thumb_tx, thumb_rx) = mpsc::channel::<crate::photos::Done>();
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
        certificate_tried: false,
        room_history: BTreeSet::new(),
        password_given,
        v: Videos::new(video_tx, rdp_tx),
        sounds: true,
        quiet_busy: false,
        content_tx,
        content: BTreeMap::new(),
        photos: None,
        photo_ntlm: ntlm_tx,
        photo_done: thumb_tx,
        calendar_at: None,
        contact_uris: Vec::new(),
        contact_list: None,
        ucs_ids: Default::default(),
        exchange_lists: 0,
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
        while let Ok(v) = video_rx.try_recv() {
            outputs.extend(s.video_event(v, out));
        }
        while let Ok(r) = rdp_rx.try_recv() {
            outputs.extend(s.rdp_event(r, out));
        }
        while let Ok(c) = content_rx.try_recv() {
            s.content_event(c, out);
        }
        // The photo thread's NTLM steps, with the signed-in registrar.
        while let Ok(a) = ntlm_rx.try_recv() {
            let r = s
                .engine
                .http_ntlm(&a.negotiate, &a.challenge, now(), a.cert.as_deref());
            let _ = a.reply.send(r);
        }
        while let Ok(d) = thumb_rx.try_recv() {
            match d {
                crate::photos::Done::Thumb(t) => out.event(AppEvent::ContactPhoto {
                    uri: t.uri,
                    side: t.side,
                    rgba: t.rgba,
                }),
                crate::photos::Done::Calendar(Ok((events, oof))) => {
                    let t = now();
                    trace_line(&format!(
                        "calendar: {} appointments, automatic replies {}",
                        events.as_ref().map_or(0, Vec::len),
                        if oof.as_ref().is_some_and(|o| o.active(t)) {
                            "on"
                        } else {
                            "off"
                        }
                    ));
                    out.event(AppEvent::Calendar {
                        appointments: events.map(|l| l.iter().map(appointment_of).collect()),
                        out_of_office: oof.filter(|o| o.active(t)).map(|o| o.message),
                    })
                }
                crate::photos::Done::Calendar(Err(e)) => trace_line(&format!("calendar: {e}")),
                crate::photos::Done::Contacts(Ok((list, ids))) => {
                    s.ucs_ids = ids;
                    s.exchange_lists += 1;
                    outputs.extend(s.engine.set_exchange_contacts(list, now()));
                }
                crate::photos::Done::Contacts(Err(e)) => out.notice(
                    NoticeKind::Warning,
                    format!("the contact list in Exchange: {e}"),
                ),
                crate::photos::Done::Expanded(email, found) => {
                    use commscraft_dlx::ExpandStatus as X;
                    let (members, partial) = match found {
                        Ok(x) => match x.status {
                            X::Success => (Ok(members_of(&x)), false),
                            X::MemberCountLimitExceeded => (Ok(members_of(&x)), true),
                            X::NotAuthorized => (Err("not authorized".to_string()), false),
                            X::NotFound => (Err("no such list".to_string()), false),
                            X::SimultaneousRequestLimitExceeded => {
                                (Err("the server is busy".to_string()), false)
                            }
                            X::Invalid => {
                                (Err("the server could not expand it".to_string()), false)
                            }
                        },
                        Err(e) => (Err(e), false),
                    };
                    // Like the contacts: their presence and photos.
                    if let Ok(list) = &members {
                        let uris: Vec<String> = list.iter().map(|c| c.uri.clone()).collect();
                        for u in &uris {
                            s.want_photo(u);
                        }
                        outputs.extend(s.engine.command(C::WatchPresence { uris }, now()));
                    }
                    out.event(AppEvent::DistributionMembers {
                        email,
                        members,
                        partial,
                    });
                }
            }
        }
        // The appointments: once provisioning is known, then every 15 minutes.
        if s.calendar_at
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(15 * 60))
        {
            if let (Some(p), Some(c)) = (&s.photos, s.services.as_deref()) {
                p.calendar(&c.calendar);
                s.calendar_at = Some(Instant::now());
            }
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
        self.spawn_media_with_video(peer, offer_call).0
    }

    /// The audio stream of a call, and the bandwidth settings for its video stream.
    fn spawn_media_with_video(
        &mut self,
        peer: Option<&str>,
        offer_call: Option<u32>,
    ) -> (u32, Option<commscraft_call_media::BwmConfig>) {
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
        let video_bwm = calls::video_bandwidth_config(
            bwm.as_ref(),
            offer_call.and_then(|c| self.engine.incoming_offer(c).map(|(_, o)| o)),
        );
        let h = calls::spawn(
            id,
            self.local_ip,
            relay,
            bwm,
            commscraft_audio_alsa::Config::default(),
            self.media_tx.clone(),
        );
        self.media.insert(id, h);
        (id, video_bwm)
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_media;
        self.next_media = self.next_media.wrapping_add(1).max(1);
        id
    }

    /// A video stream (function 28; without a camera for a shared screen).
    fn spawn_video(&mut self, bwm: Option<commscraft_call_media::BwmConfig>, camera: bool) -> u32 {
        let vid = self.next_id();
        let relay = self.engine.media_relay().and_then(calls::relay_from);
        let h = video_call::spawn(
            vid,
            self.local_ip,
            relay,
            bwm,
            if camera {
                self.v.camera.clone()
            } else {
                String::new()
            },
            self.v.video_tx.clone(),
        );
        self.v.handles.insert(vid, h);
        vid
    }

    fn to_video(&self, call: u32, m: video_call::ToVideo) {
        if let Some(h) = self
            .v
            .of_call
            .get(&call)
            .and_then(|vid| self.v.handles.get(vid))
        {
            let _ = h.tx.send(m);
        }
    }

    /// Ends the video and the RDP viewer of a call.
    fn stop_video(&mut self, call: u32) {
        if let Some(h) = self
            .v
            .of_call
            .remove(&call)
            .and_then(|v| self.v.handles.remove(&v))
        {
            let _ = h.tx.send(video_call::ToVideo::Stop);
        }
        if let Some(h) = self.v.rdp.remove(&call) {
            let _ = h.tx.send(rdp_call::ToRdp::Stop);
        }
        self.v.screen_offers.remove(&call);
        self.v.viewing.remove(&call);
    }

    /// A video call goes out (or is answered) once its audio and its video stream are ready.
    fn start_video_calls(&mut self, out: &Out) -> Vec<Output> {
        let t = now();
        let policy = self.policy();
        let both: Vec<(u32, u32)> = self
            .v
            .for_audio
            .iter()
            .filter(|(a, v)| {
                self.v.ready_audio.contains_key(a) && self.v.ready_video.contains_key(v)
            })
            .map(|(a, v)| (*a, *v))
            .collect();
        let mut o = Vec::new();
        for (aid, vid) in both {
            self.v.for_audio.remove(&aid);
            let (Some(local), Some(video)) = (
                self.v.ready_audio.remove(&aid),
                self.v.ready_video.remove(&vid),
            ) else {
                continue;
            };
            if let Some(to) = self.pending_out.remove(&aid) {
                let call = self.engine.next_call_id();
                self.media_of_call.insert(call, aid);
                self.v.of_call.insert(call, vid);
                self.outgoing.insert(call);
                out.event(AppEvent::CallStarted {
                    id: call,
                    peer: to.clone(),
                    incoming: false,
                    at: t,
                });
                out.event(AppEvent::CallKind {
                    id: call,
                    kind: CallKind::Video,
                });
                out.event(AppEvent::CallVideo {
                    id: call,
                    state: VideoState::Starting,
                });
                o.extend(self.engine.command(
                    C::VideoCall {
                        to,
                        local,
                        video,
                        policy,
                    },
                    t,
                ));
            } else if let Some(call) = self.pending_in.remove(&aid) {
                self.media_of_call.insert(call, aid);
                self.v.of_call.insert(call, vid);
                out.event(AppEvent::CallKind {
                    id: call,
                    kind: CallKind::Video,
                });
                out.event(AppEvent::CallVideo {
                    id: call,
                    state: VideoState::Starting,
                });
                o.extend(self.engine.command(
                    C::AcceptVideoCall {
                        call,
                        local,
                        video,
                        policy,
                    },
                    t,
                ));
            } else if let Some(h) = self.v.handles.remove(&vid) {
                let _ = h.tx.send(video_call::ToVideo::Stop);
            }
        }
        o
    }

    fn call_of_video(&self, vid: u32) -> Option<u32> {
        self.v
            .of_call
            .iter()
            .find(|(_, v)| **v == vid)
            .map(|(c, _)| *c)
    }

    fn video_event(&mut self, e: video_call::FromVideo, out: &Out) -> Vec<Output> {
        use video_call::FromVideo as V;
        let t = now();
        match e {
            V::Ready(vid, local) => match self.v.pending_screen.remove(&vid) {
                Some(call) => {
                    self.v.of_call.insert(call, vid);
                    let policy = self.policy();
                    self.engine.command(
                        C::AcceptScreenShare {
                            call,
                            video: local,
                            policy,
                        },
                        t,
                    )
                }
                None => {
                    self.v.ready_video.insert(vid, local);
                    self.start_video_calls(out)
                }
            },
            V::Selected(vid, local) => match self.call_of_video(vid) {
                Some(call) => self
                    .engine
                    .command(C::SetCallVideo { call, video: local }, t),
                None => Vec::new(),
            },
            V::Notice(_, n) => {
                out.notice(NoticeKind::Info, format!("video: {n}"));
                Vec::new()
            }
            // Only meetings switch sources.
            V::Stalled(_) => Vec::new(),
            V::PictureUpdate(vid) => match self.call_of_video(vid) {
                Some(call) => self.engine.command(C::RequestPictureUpdate { call }, t),
                None => Vec::new(),
            },
            V::Failed(vid, why) => {
                self.v.handles.remove(&vid);
                self.v.ready_video.remove(&vid);
                if let Some(call) = self.v.pending_screen.remove(&vid) {
                    out.event(AppEvent::CallVideo {
                        id: call,
                        state: VideoState::Failed(why),
                    });
                    return self.engine.command(C::Hangup { call }, t);
                }
                if let Some(call) = self.call_of_video(vid) {
                    self.v.of_call.remove(&call);
                    out.event(AppEvent::CallVideo {
                        id: call,
                        state: VideoState::Failed(why),
                    });
                }
                Vec::new()
            }
        }
    }

    fn rdp_event(&mut self, e: rdp_call::FromRdp, out: &Out) -> Vec<Output> {
        use rdp_call::FromRdp as R;
        let t = now();
        match e {
            R::Ready(rid, local) => match self.v.pending_rdp.remove(&rid) {
                Some(call) => {
                    self.v.rdp_call_of.insert(rid, call);
                    let policy = self.policy();
                    self.engine.command(
                        C::AcceptRdpShare {
                            call,
                            local,
                            policy,
                        },
                        t,
                    )
                }
                None => Vec::new(),
            },
            R::Selected(rid, local) => match self.v.rdp_call_of.get(&rid) {
                Some(&call) => self.engine.command(C::UpdateCall { call, local }, t),
                None => Vec::new(),
            },
            R::Notice(rid, n) => {
                if let Some(&call) = self.v.rdp_call_of.get(&rid) {
                    if n.starts_with("viewing") {
                        out.event(AppEvent::CallVideo {
                            id: call,
                            state: VideoState::Window,
                        });
                    }
                }
                out.notice(NoticeKind::Info, n);
                Vec::new()
            }
            R::Ended(rid, why) => {
                let call = self
                    .v
                    .pending_rdp
                    .remove(&rid)
                    .or_else(|| self.v.rdp_call_of.remove(&rid));
                match call {
                    Some(call) if self.v.rdp.remove(&call).is_some() => {
                        out.event(AppEvent::CallVideo {
                            id: call,
                            state: VideoState::Failed(why),
                        });
                        // Viewing was all the call was for.
                        self.engine.command(C::Hangup { call }, t)
                    }
                    _ => Vec::new(),
                }
            }
        }
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
        for (_, h) in std::mem::take(&mut self.v.handles) {
            let _ = h.tx.send(video_call::ToVideo::Stop);
        }
        for (_, h) in std::mem::take(&mut self.v.rdp) {
            let _ = h.tx.send(rdp_call::ToRdp::Stop);
        }
        for (_, (h, _)) in std::mem::take(&mut self.content) {
            let _ = h.tx.send(meeting_content::ToContent::Stop);
        }
    }

    /// Asks the photo thread for the photos of the user and the contacts, once provisioning
    /// (the photo policy, the address book service) is known; the thread fetches each once.
    /// The contact list is kept in Exchange (the server said so, or marks its copy migrated).
    fn in_exchange(&self) -> bool {
        self.engine.contacts_in_exchange()
            || self
                .contact_list
                .as_ref()
                .is_some_and(|l| l.ucs_mode.as_deref() == Some("migrated"))
    }

    fn want_photos(&self) {
        let (Some(p), Some(services)) = (&self.photos, self.services.as_deref()) else {
            return;
        };
        p.want(self.engine.aor(), Some(services));
        for u in &self.contact_uris {
            p.want(u, Some(services));
        }
    }

    /// One person's photo (a search hit, a conversation partner).
    fn want_photo(&self, uri: &str) {
        if let (Some(p), Some(services)) = (&self.photos, self.services.as_deref()) {
            p.want(uri, Some(services));
        }
    }

    /// Alert sounds: on, unless switched off or kept to a minimum while busy.
    fn alert_sounds(&self) -> bool {
        self.sounds && !(self.quiet_busy && self.profile.presence == Presence::Busy)
    }

    fn stop_content(&mut self, conversation: u32) {
        if let Some((h, _)) = self.content.remove(&conversation) {
            let _ = h.tx.send(meeting_content::ToContent::Stop);
        }
    }

    /// What the meeting content reports, as changes for the interface.
    fn content_event(&mut self, c: meeting_content::FromContent, out: &Out) {
        use commscraft_psom::client::Event as P;
        use commscraft_ui::content::{ContentChange as Ch, ContentKind};
        let (session, e) = match c {
            meeting_content::FromContent::Line(..) => return,
            meeting_content::FromContent::Ended(session, reason) => {
                self.content.remove(&session);
                out.event(AppEvent::ContentEnded { session, reason });
                return;
            }
            meeting_content::FromContent::Event(session, e) => (session, *e),
        };
        let Some((_, users)) = self.content.get_mut(&session) else {
            return;
        };
        let change = match e {
            P::Connected => Ch::Connected,
            P::UsersAdded(list) => {
                for (id, uri, name) in list {
                    if users.len() < 2000 {
                        users.insert(id, if name.is_empty() { uri } else { name });
                    }
                }
                return;
            }
            P::UsersRemoved(ids) => {
                for id in ids {
                    users.remove(&id);
                }
                return;
            }
            P::ContentAdded { content, kind } => Ch::Added {
                id: content,
                kind: ContentKind::of(&kind),
            },
            P::ContentRemoved(id) => Ch::Removed(id),
            P::Title { content, title } => Ch::Title { id: content, title },
            P::ActiveContent(id) => Ch::Presented(id),
            P::ActivePresenter(0) => Ch::Presenter(String::new()),
            P::ActivePresenter(u) => {
                Ch::Presenter(users.get(&u).cloned().unwrap_or_else(|| u.to_string()))
            }
            P::FileInfo {
                content,
                name,
                size,
            } => Ch::File {
                id: content,
                name,
                size,
            },
            P::PollQuestion {
                content,
                question,
                choices,
            } => Ch::PollQuestion {
                id: content,
                question,
                choices,
            },
            P::PollResults { content, results } => Ch::PollResults {
                id: content,
                results,
            },
            P::PollOpen { content, open } => Ch::PollOpen { id: content, open },
            P::PollLocalVote { content, choice } => Ch::Voted {
                id: content,
                choice,
            },
            P::Annotations { content, count } => Ch::Annotations { id: content, count },
            P::Slides { content, count } => Ch::Slides {
                id: content,
                count: usize::try_from(count).unwrap_or(0),
            },
            P::SlideLocation { content, location } => Ch::SlideAt {
                id: content,
                at: slide_of(&location),
            },
            P::Owner { .. }
            | P::Presented { .. }
            | P::Ready
            | P::UrlBase(_)
            | P::PollResultsVisible { .. }
            | P::NotesTitles { .. }
            | P::Closed(_) => return,
        };
        out.event(AppEvent::Content { session, change });
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
            ToWorker::VideoCall { peer } => {
                let (aid, bwm) = self.spawn_media_with_video(Some(&peer), None);
                self.pending_out.insert(aid, peer);
                let vid = self.spawn_video(bwm, true);
                self.v.for_audio.insert(aid, vid);
                Vec::new()
            }
            ToWorker::AnswerVideo { id: call } => {
                self.cues.stop_ring(call);
                let video = self.engine.incoming_video(call);
                let (aid, bwm) = self.spawn_media_with_video(None, Some(call));
                self.pending_in.insert(aid, call);
                if video {
                    let vid = self.spawn_video(bwm, true);
                    self.v.for_audio.insert(aid, vid);
                } else {
                    out.notice(
                        NoticeKind::Info,
                        "the call offers no video – answered with audio",
                    );
                }
                Vec::new()
            }
            ToWorker::ViewScreen { id: call } => {
                self.cues.stop_ring(call);
                match self.v.screen_offers.remove(&call) {
                    // Only RDP offered: a shared window.
                    Some(true) => {
                        let rid = self.next_id();
                        let h = rdp_call::spawn(
                            rid,
                            self.local_ip,
                            self.engine.own_aor().to_string(),
                            self.v.rdp_tx.clone(),
                            self.v.video_tx.clone(),
                        );
                        self.v.rdp.insert(call, h);
                        self.v.pending_rdp.insert(rid, call);
                    }
                    Some(false) => {
                        let vid = self.spawn_video(None, false);
                        self.v.pending_screen.insert(vid, call);
                    }
                    None => return Vec::new(),
                }
                self.v.viewing.insert(call);
                out.event(AppEvent::CallVideo {
                    id: call,
                    state: VideoState::Starting,
                });
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
                self.to_video(id, video_call::ToVideo::Hold(hold));
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
            // Only asked during discovery; a late answer has nothing to decide.
            ToWorker::Trust(_) => Vec::new(),
            ToWorker::Transfer { id, peer } => self.engine.command(
                C::Transfer {
                    call: id,
                    to: commscraft_session::TransferTarget::Address(peer),
                },
                t,
            ),
            ToWorker::Search { text } => {
                self.search(&text, out);
                Vec::new()
            }
            ToWorker::JoinContent { session } => self.engine.command(
                C::ConferenceControl {
                    conversation: session,
                    action: commscraft_session::ConferenceAction::JoinContent,
                },
                t,
            ),
            ToWorker::LeaveContent { session } => {
                self.stop_content(session);
                out.event(AppEvent::ContentEnded {
                    session,
                    reason: "closed".into(),
                });
                Vec::new()
            }
            ToWorker::Vote {
                session,
                content,
                choice,
            } => {
                if let Some((h, _)) = self.content.get(&session) {
                    let _ =
                        h.tx.send(meeting_content::ToContent::Vote { content, choice });
                }
                Vec::new()
            }
            ToWorker::Settings {
                camera,
                sounds,
                quiet_busy,
            } => {
                self.v.camera = if camera.is_empty() {
                    std::env::var("COMMSCRAFT_CAMERA").unwrap_or_else(|_| "/dev/video0".into())
                } else {
                    camera
                };
                self.sounds = sounds;
                self.quiet_busy = quiet_busy;
                if !sounds {
                    self.cues.stop_all();
                }
                Vec::new()
            }
            ToWorker::Dtmf { id, digits } => {
                for d in digits
                    .chars()
                    .filter(|c| commscraft_rtp::dtmf_event(*c).is_some())
                {
                    self.to_media(id, calls::ToMedia::Dtmf(d));
                }
                Vec::new()
            }
            ToWorker::Contacts(c) => match contact_command(self.contact_list.as_ref(), c) {
                // The list is in Exchange: the change goes there (the relationships stay SIP).
                Ok(C::EditContacts(edit)) if self.in_exchange() => {
                    match (&self.photos, self.engine.contact_list()) {
                        (Some(p), Some(l)) => {
                            p.contacts(Some((edit, l.clone(), self.ucs_ids.clone())))
                        }
                        _ => out.notice(
                            NoticeKind::Warning,
                            "the contact list in Exchange is not reachable",
                        ),
                    }
                    Vec::new()
                }
                Ok(cmd) => self.engine.command(cmd, t),
                Err(e) => {
                    out.notice(NoticeKind::Warning, format!("contact list: {e}"));
                    Vec::new()
                }
            },
            ToWorker::ExpandGroup { email } => {
                if let Some(p) = &self.photos {
                    p.expand(&email, self.services.as_deref());
                }
                Vec::new()
            }
            ToWorker::LoadRooms => {
                self.room_history.clear();
                self.engine.command(C::ChatRooms, t)
            }
            ToWorker::SendToRoom { room, text } => {
                self.engine.command(C::SendToRoom { room, text }, t)
            }
            ToWorker::SignOut => {
                self.signing_out = Some(Instant::now());
                self.stop_media();
                let mut o = self.engine.command(C::LeaveRooms, t);
                o.extend(self.engine.command(C::SignOut, t));
                o
            }
        }
    }

    /// Persistent chat events: the room list is joined at once, each joined room's history is
    /// asked for once.
    fn rooms_event(&mut self, x: commscraft_xccos::Event, out: &Out) -> Vec<Output> {
        use commscraft_xccos::Event as X;
        let t = now();
        match x {
            X::ChannelServer(_) => Vec::new(),
            X::ServerInfo { name, .. } => {
                let shown = if name.trim().is_empty() {
                    "persistent chat".to_string()
                } else {
                    name
                };
                out.notice(NoticeKind::Info, format!("connected to {shown}"));
                Vec::new()
            }
            X::Rooms(rooms) => {
                if rooms.is_empty() {
                    out.notice(NoticeKind::Info, "you are a member of no chat room");
                }
                let uris: Vec<String> = rooms.iter().map(|r| r.uri.clone()).collect();
                for r in rooms {
                    out.event(room_event(&r, false));
                }
                if uris.is_empty() {
                    Vec::new()
                } else {
                    self.engine.command(C::JoinRooms { rooms: uris }, t)
                }
            }
            X::Joined(rooms) => {
                let mut o = Vec::new();
                for r in rooms {
                    out.event(room_event(&r, true));
                    if self.room_history.len() < 200 && self.room_history.insert(r.uri.clone()) {
                        o.extend(self.engine.command(C::RoomHistory { room: r.uri }, t));
                    }
                }
                o
            }
            X::History(rooms) => {
                for r in rooms {
                    let posts = r.messages.iter().map(room_post).collect();
                    out.event(AppEvent::RoomPosts {
                        room: r.uri,
                        posts,
                        history: true,
                    });
                }
                Vec::new()
            }
            X::Message(m) => {
                out.event(AppEvent::RoomPosts {
                    room: m.room.clone(),
                    posts: vec![room_post(&m)],
                    history: false,
                });
                Vec::new()
            }
            X::Failed { command, code } => {
                out.notice(
                    NoticeKind::Warning,
                    format!("chat room command {command} failed ({code})"),
                );
                Vec::new()
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
            Ok(found) => {
                let list = directory_contacts(&found);
                for c in list.iter().take(20) {
                    self.want_photo(&c.uri);
                }
                out.event(AppEvent::SearchResults(list));
            }
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
            calls::FromMedia::Ready(id, local) if self.v.for_audio.contains_key(&id) => {
                self.v.ready_audio.insert(id, local);
                self.start_video_calls(out)
            }
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
                        self.v.ready_audio.remove(&id);
                        if let Some(h) = self
                            .v
                            .for_audio
                            .remove(&id)
                            .and_then(|vid| self.v.handles.remove(&vid))
                        {
                            let _ = h.tx.send(video_call::ToVideo::Stop);
                        }
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
                    let organisation = aor
                        .rsplit_once('@')
                        .map(|(_, d)| d.to_string())
                        .unwrap_or_default();
                    self.profile = Profile {
                        uri: aor,
                        name,
                        presence: Presence::Available,
                        organisation,
                    };
                    out.event(AppEvent::SignedIn(self.profile.clone()));
                    self.photos = Some(crate::photos::Photos::start(
                        self.tls.clone(),
                        self.trust.clone(),
                        self.engine.aor().to_string(),
                        self.photo_ntlm.clone(),
                        self.photo_done.clone(),
                    ));
                    self.want_photos();
                    out.event(AppEvent::Capabilities(online_capabilities(
                        self.engine.has_persistent_chat(),
                    )));
                    out.event(AppEvent::Connection(Connection::Connected));
                }
            }
            E::SignedOut => return Some(Ok(())),
            E::RegistrationEnded(why) => return Some(Err(Ended::Lost(why))),
            E::CertificateNeeded { sts_uri } => {
                // Function 6: get the user certificate once, then start the sign-in again.
                if self.certificate_tried {
                    return Some(Err(Ended::Failed(
                        "the server still asks for a certificate".into(),
                    )));
                }
                // Getting a certificate needs the password: without one (a sign-in with the
                // stored certificate, which is missing or expired now) the user is asked for it.
                if !self.password_given {
                    return Some(Err(Ended::Failed(
                        "password needed: the stored certificate is missing or has expired".into(),
                    )));
                }
                self.certificate_tried = true;
                out.notice(NoticeKind::Info, "getting a user certificate for TLS-DSK …");
                let Some(dir) = commscraft_client::tlsdsk::identity_dir(self.engine.aor()) else {
                    return Some(Err(Ended::Failed("no place for the certificate".into())));
                };
                let aor = self.engine.aor().to_string();
                let epid = self.engine.epid().to_string();
                let ua = format!("CommSCraft/{}", env!("CARGO_PKG_VERSION"));
                let engine = &mut self.engine;
                let mut ntlm =
                    |n: &[u8], c: &[u8], cert: Option<&[u8]>| engine.http_ntlm(n, c, now(), cert);
                let mut log = |l: &str| {
                    if verbose() {
                        eprintln!("{l}");
                    }
                };
                match commscraft_client::tlsdsk::provision_certificate(
                    &self.tls,
                    &self.trust,
                    &sts_uri,
                    &aor,
                    &epid,
                    &dir,
                    &ua,
                    &mut ntlm,
                    &mut log,
                ) {
                    Ok(days) => {
                        out.notice(
                            NoticeKind::Info,
                            format!("user certificate received (valid for {days} more days)"),
                        );
                        more.extend(self.engine.start(now()));
                    }
                    Err(e) => return Some(Err(Ended::Failed(format!("TLS-DSK: {e}")))),
                }
            }
            E::Provisioned(c) => {
                self.services = Some(c);
                self.want_photos();
                if self.signed_in {
                    out.event(AppEvent::Capabilities(online_capabilities(
                        self.engine.has_persistent_chat(),
                    )));
                }
            }
            E::ChatRooms(x) => more.extend(self.rooms_event(x, out)),
            E::RoomNotSent { room, status } => out.event(AppEvent::RoomNotSent { room, status }),
            E::ContactList(l) => {
                trace_line(&format!(
                    "contact list: {} contacts, groups {:?}",
                    l.contacts.len(),
                    l.groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>()
                ));
                // A list the server marks as migrated to Exchange is its copy ([MS-OXWSCOS],
                // interop: the server still sends it); changes and their identifiers come from
                // Exchange, so the list is read there – and kept in Exchange's numbering.
                if self.exchange_lists > 0 {
                    self.exchange_lists -= 1;
                } else if l.ucs_mode.as_deref() == Some("migrated") {
                    if let Some(p) = &self.photos {
                        p.contacts(None);
                    }
                }
                let list = contacts_of(&l);
                out.event(AppEvent::DistributionGroups(distribution_groups_of(&l)));
                out.event(AppEvent::Groups(
                    l.groups
                        .iter()
                        .filter(|g| {
                            g.name != "~"
                                && pres::group_extension(&g.external) == pres::GroupExtension::Plain
                        })
                        .map(|g| g.name.clone())
                        .collect(),
                ));
                self.contact_list = Some(l.clone());
                self.contact_uris = list.iter().map(|c| c.uri.clone()).take(2000).collect();
                out.event(AppEvent::Contacts(list));
                self.want_photos();
            }
            // Function 11: the list lives in Exchange – read it there (EX-04/EX-05).
            E::ContactsInExchange => match &self.photos {
                Some(p) => p.contacts(None),
                None => out.notice(
                    NoticeKind::Info,
                    "the contact list is kept in Exchange, which is not reachable",
                ),
            },
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
                    self.want_photo(&peer);
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
                if self.alert_sounds() {
                    self.cues.notify(now_ms());
                }
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
            E::MeetingContent {
                conversation,
                connection,
            } => {
                self.stop_content(conversation);
                out.event(AppEvent::ContentConnecting {
                    session: conversation,
                });
                let h = meeting_content::spawn(
                    conversation,
                    connection,
                    self.tls.clone(),
                    self.content_tx.clone(),
                );
                self.content.insert(conversation, (h, BTreeMap::new()));
            }
            E::ConversationEnded { conversation } => {
                self.stop_content(conversation);
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
                    .map(|p| {
                        p.name
                            .clone()
                            .unwrap_or_else(|| p.uri.trim_start_matches("sip:").to_string())
                    })
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
                if self.alert_sounds() {
                    self.cues.notify(now_ms());
                }
                out.notice(
                    NoticeKind::Info,
                    format!(
                        "{} invites you to a meeting (Meetings)",
                        from.trim_start_matches("sip:")
                    ),
                );
                out.event(AppEvent::Invitation(commscraft_ui::model::Invitation {
                    from,
                    focus: invitation.focus,
                    subject: invitation.subject.unwrap_or_default(),
                }))
            }
            E::ConferenceLobby { waiting: true, .. } => out.notice(
                NoticeKind::Info,
                "waiting in the lobby until a presenter admits you",
            ),
            E::Meetings(list) => out.event(AppEvent::Meetings(
                list.iter().filter_map(meeting_of).collect(),
            )),
            E::MeetingScheduled(m) | E::MeetingDetails(m) => {
                if let Some(m) = meeting_of(&m) {
                    out.notice(NoticeKind::Info, format!("meeting {}", m.conference_id));
                }
            }
            E::IncomingCall { call, from } => {
                if self.sounds {
                    self.cues.start_ring(call);
                }
                out.event(AppEvent::CallStarted {
                    id: call,
                    peer: from,
                    incoming: true,
                    at: t,
                });
                if self.engine.incoming_video(call) {
                    out.event(AppEvent::CallOffersVideo { id: call });
                }
            }
            E::CallVideo { call, video } => {
                self.to_video(
                    call,
                    video_call::ToVideo::Start {
                        video,
                        controlling: self.outgoing.contains(&call),
                    },
                );
                if self.v.of_call.contains_key(&call) {
                    if self.v.viewing.remove(&call) {
                        out.event(AppEvent::CallConnected { id: call, at: t });
                    }
                    out.event(AppEvent::CallVideo {
                        id: call,
                        state: VideoState::Window,
                    });
                }
            }
            // PL-50: a freeze comes only from an MCU – ignored in 1:1 calls.
            E::CallVideoControl { call, control } => {
                if control != commscraft_session::media_control::VideoControl::Freeze {
                    self.to_video(call, video_call::ToVideo::Control(control));
                }
            }
            E::ScreenRdp {
                call,
                rdp,
                controlling,
            } => {
                if let Some(h) = self.v.rdp.get(&call) {
                    let _ = h.tx.send(rdp_call::ToRdp::Start { rdp, controlling });
                    if self.v.viewing.remove(&call) {
                        out.event(AppEvent::CallConnected { id: call, at: t });
                    }
                }
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
                self.stop_video(call);
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
            E::TransferResult { call, status } => {
                if (200..300).contains(&status) {
                    out.notice(NoticeKind::Info, "the call was transferred");
                } else {
                    out.notice(
                        NoticeKind::Error,
                        format!("transfer of the call failed ({status}); the call stays"),
                    );
                }
                let _ = call;
            }
            E::TransferRequested { call, .. } => {
                out.notice(
                    NoticeKind::Info,
                    "the other side wants to transfer the call; the desktop client declines (commscraft-cli can follow)",
                );
                more.extend(self.engine.command(C::DeclineTransfer { call }, t));
            }
            E::IncomingScreenShare { call, from, rdp } => {
                if self.sounds {
                    self.cues.start_ring(call);
                }
                self.v.screen_offers.insert(call, rdp);
                out.event(AppEvent::CallStarted {
                    id: call,
                    peer: from,
                    incoming: true,
                    at: t,
                });
                out.event(AppEvent::CallKind {
                    id: call,
                    kind: CallKind::Screen,
                });
            }
            E::NewSubscriber { uri, name } => {
                out.event(AppEvent::NewSubscriber(commscraft_ui::model::Subscriber {
                    uri,
                    name,
                }))
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
                out.notice(NoticeKind::Warning, format!("{method} {status}: {why}"));
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
                    external: String::new(),
                },
                pres::Group {
                    id: 2,
                    name: "Team".into(),
                    external: String::new(),
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
    fn favourites_and_distribution_groups_of_the_contact_list() {
        let g = |id, name: &str, ext: &str| pres::Group {
            id,
            name: name.into(),
            external: ext.into(),
        };
        let l = pres::ContactList {
            groups: vec![
                g(1, "~", ""),
                g(2, "Pinned Contacts", "<groupExtension groupType=\"pinnedGroup\"><email/></groupExtension>"),
                g(3, "Lab Team", "<groupExtension groupType=\"dg\"><email>team@beispiel.invalid</email></groupExtension>"),
                g(4, "Team", ""),
            ],
            contacts: vec![pres::Contact {
                uri: "sip:bob@beispiel.invalid".into(),
                name: "Bob".into(),
                groups: vec![2, 4],
                subscribed: true,
                extension: false,
            }],
            ..pres::ContactList::default()
        };
        let c = contacts_of(&l);
        assert!(c[0].pinned);
        assert_eq!(c[0].group, "Team");
        assert_eq!(
            distribution_groups_of(&l),
            vec![("Lab Team".to_string(), "team@beispiel.invalid".to_string())]
        );
    }

    #[test]
    fn interface_changes_become_engine_commands_with_group_ids() {
        let g = |id, name: &str, ext: &str| pres::Group {
            id,
            name: name.into(),
            external: ext.into(),
        };
        let l = pres::ContactList {
            groups: vec![
                g(1, "~", ""),
                g(
                    2,
                    "Pinned Contacts",
                    "<groupExtension groupType=\"pinnedGroup\"/>",
                ),
                g(3, "Team", ""),
            ],
            contacts: vec![pres::Contact {
                uri: "sip:bob@beispiel.invalid".into(),
                name: "Bob".into(),
                groups: vec![1, 2],
                subscribed: true,
                extension: false,
            }],
            ..pres::ContactList::default()
        };
        let add = |group: Option<&str>| {
            contact_command(
                Some(&l),
                ContactChange::Add {
                    uri: "sip:eve@beispiel.invalid".into(),
                    name: String::new(),
                    group: group.map(String::from),
                },
            )
        };
        assert!(
            matches!(add(Some("Team")), Ok(C::EditContacts(pres::ContactEdit::Set { groups, .. })) if groups == vec![3])
        );
        // An unknown group, or none: the default group.
        assert!(
            matches!(add(None), Ok(C::EditContacts(pres::ContactEdit::Set { groups, .. })) if groups == vec![1])
        );
        assert!(
            matches!(add(Some("Pinned Contacts")), Ok(C::EditContacts(pres::ContactEdit::Set { groups, .. })) if groups == vec![1])
        );
        // Moving keeps the favourites and replaces the plain group.
        let moved = contact_command(
            Some(&l),
            ContactChange::Move {
                uri: "sip:bob@beispiel.invalid".into(),
                group: "Team".into(),
            },
        );
        assert!(
            matches!(moved, Ok(C::EditContacts(pres::ContactEdit::Set { groups, name, .. })) if groups == vec![2, 3] && name == "Bob")
        );
        assert!(contact_command(
            Some(&l),
            ContactChange::DeleteGroup {
                name: "Pinned Contacts".into()
            }
        )
        .is_err());
        assert!(matches!(
            contact_command(
                Some(&l),
                ContactChange::Relation {
                    uri: "sip:eve@beispiel.invalid".into(),
                    relation: Some(commscraft_ui::model::Relation::Blocked),
                }
            ),
            Ok(C::SetRelationship {
                relationship: Some(pres::Relationship::Blocked),
                ..
            })
        ));
        assert!(contact_command(
            None,
            ContactChange::Move {
                uri: "sip:bob@beispiel.invalid".into(),
                group: "Team".into()
            }
        )
        .is_err());
    }

    #[test]
    fn the_desktop_gates_match_what_it_wires() {
        let c = online_capabilities(false);
        assert!(c.chat.usable() && c.audio_call.usable() && c.meetings.usable());
        assert!(!c.file_transfer.usable());
        assert!(!c.read_receipts.usable());
        assert!(c.video_call.usable());
        assert!(c.transfer.usable() && c.directory_search.usable());
        // Chat rooms only with a persistent chat pool from provisioning.
        assert!(!c.persistent_chat.usable());
        assert!(online_capabilities(true).persistent_chat.usable());
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
