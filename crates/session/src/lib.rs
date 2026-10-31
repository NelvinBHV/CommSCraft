//! The session engine (L4): one signed-in account. It owns the registration, the long-lived
//! subscriptions (contact list, presence of the contacts), the own endpoint publication, the
//! client transactions and the IM conversations, and turns everything into events for a UI.
//!
//! Sans-I/O: the caller feeds received messages, the clock and user commands, and sends what the
//! engine returns ([`Output`]). Nothing here touches sockets, files or the UI.
//!
//! Evidence: MS-OCSPROT §2.5.3 bootstrap (OCS-UC03), MS-PRES (PR-11 … PR-17, PR-31, PR-33),
//! MS-SIP (SIP-04, SIP-07), MS-SIPAE (AE-29, AE-32), MS-CONMGMT (CON-30 … CON-35), RFC 3261
//! (§8.2.6, §12, §13, §17), RFC 3265 (subscriptions, §3.1.4.2 refresh, §3.3.4 unsubscribe),
//! RFC 3428, `docs/protocol/evidence/` files 01, 02, 03, 05, 07, 21.
//!
//! Security by design (`docs/security.md`):
//! - every request of the network must carry a valid server signature (AE-32) and is otherwise
//!   dropped without an answer; every answer to our requests must verify as well;
//! - all our requests and answers are signed with the security association (AE-29);
//! - every request is either matched to one of our dialogs or answered with a final status;
//!   nothing grows without bound: transactions time out (32 s), the number of conversations and
//!   subscriptions is limited;
//! - content from the network reaches the UI only through the checked parsers of `presence` and
//!   `im` (plain text, control characters removed).

use std::collections::HashMap;

use commscraft_avedgea as avedgea;
use commscraft_conf as conf;
use commscraft_im as im;
use commscraft_presence as pres;
use commscraft_provisioning as prov;
use commscraft_register::keepalive::KeepAlive;
use commscraft_register::{Deregistration, Event as RegEvent, Random, Registrar, Registration};
use commscraft_sdp as sdp;
use commscraft_sip_msg::dialog::{response_to, Dialog};
use commscraft_sip_msg::{Header, Message, StartLine};

/// RFC 3261 timer F (64 × T1).
pub const TRANSACTION_TIMEOUT: u64 = 32;
/// Upper bound of simultaneous conversations (design).
pub const MAX_CONVERSATIONS: usize = 64;
/// Upper bound of presence subscriptions (batches and single ones; design).
pub const MAX_SUBSCRIPTIONS: usize = 128;
/// Retry delay after a subscription ended or failed (design).
pub const RESUBSCRIBE_DELAY: u64 = 60;

/// Local parameters of the endpoint (filled by the adapter after the connection is up).
#[derive(Debug, Clone)]
pub struct Config {
    pub aor: String,
    pub epid: String,
    /// `host:port` of the local end of the SIP connection (Via sent-by).
    pub via_host: String,
    pub local_ip: std::net::IpAddr,
    pub user_agent: String,
    pub machine_name: String,
}

pub type ConversationId = u32;

/// Presenter commands (CB-35, CA-20, CA-22).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConferenceAction {
    /// Admit (`true`) or turn away users waiting in the lobby; empty = everyone waiting.
    Lobby {
        users: Vec<String>,
        admit: bool,
    },
    /// Make a participant presenter (`true`) or attendee.
    Role {
        user: String,
        presenter: bool,
    },
    Lock(bool),
    /// Admission policy, automatic promotion and lobby bypass for phone callers (CB-42).
    Options(conf::MeetingOptions),
    Remove {
        user: String,
    },
    /// Mute or unmute another participant's audio.
    MuteUser {
        user: String,
        mute: bool,
    },
    /// Mute or unmute the audience (attendees) in the A/V MCU.
    MuteAudience(bool),
}

/// What the user asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    SetStatus(pres::UserStatus),
    /// Call forwarding and simultaneous ring: publishes the routing preamble (RE-50).
    SetForwarding(pres::Forwarding),
    StartConversation {
        to: String,
    },
    SendText {
        conversation: ConversationId,
        text: String,
    },
    Typing {
        conversation: ConversationId,
        typing: bool,
    },
    EndConversation {
        conversation: ConversationId,
    },
    /// Group IM: create an ad-hoc conference and invite these participants (dial-out).
    StartGroup {
        invite: Vec<String>,
    },
    /// Invite one more participant into a group conversation.
    InviteToGroup {
        conversation: ConversationId,
        to: String,
    },
    /// Join a conference by its focus URI (after an invitation, or from a meeting link).
    JoinConference {
        focus: String,
    },
    /// Schedule a meeting at the focus factory (function 40, PV-09): lifetime in minutes.
    ScheduleMeeting {
        subject: String,
        minutes: u64,
        admission: conf::schedule::Admission,
        /// Presenters, invitees, automatic promotion, lobby bypass, locked (PV-15).
        extras: conf::schedule::Extras,
    },
    /// The organizer's static meeting with the administrator's defaults (PV-16).
    ScheduleStaticMeeting,
    /// The organizer's meetings (getConferences), or the static meeting (PV-16).
    ListMeetings {
        static_meetings: bool,
    },
    /// One meeting's details (getConference).
    ShowMeeting {
        conference_id: String,
    },
    /// Change a meeting's subject, lifetime and admission policy (getConference, then
    /// modifyConference with the current version).
    ChangeMeeting {
        conference_id: String,
        subject: String,
        minutes: u64,
        admission: conf::schedule::Admission,
        extras: conf::schedule::Extras,
    },
    /// Cancel a meeting (deleteConference).
    CancelMeeting {
        conference_id: String,
        /// The static meeting (deleteConference static="true", PV-16).
        static_meeting: bool,
    },
    /// Join the conference's audio: INVITE to the A/V MCU (CA-02, no addUser needed).
    JoinConferenceAudio {
        conversation: ConversationId,
        local: Box<sdp::Local>,
        policy: sdp::Encryption,
    },
    /// A presenter command in a conference (CB-33, CB-35, CA-20, CA-22).
    ConferenceControl {
        conversation: ConversationId,
        action: ConferenceAction,
    },
    /// Mute or unmute ourselves in the A/V MCU (modifyEndpointMedia, CA-20).
    ConferenceMute {
        conversation: ConversationId,
        mute: bool,
    },
    /// Audio call (OCS-UC12): `local` from the media adapter (candidates, key).
    Call {
        to: String,
        local: Box<sdp::Local>,
        policy: sdp::Encryption,
    },
    AcceptCall {
        call: CallId,
        local: Box<sdp::Local>,
        policy: sdp::Encryption,
    },
    /// Final offer with the selected candidates, or hold/resume (re-INVITE).
    UpdateCall {
        call: CallId,
        local: Box<sdp::Local>,
    },
    Hangup {
        call: CallId,
    },
    /// The location for emergency calls: from the location information service, or typed by the
    /// user (`manual`, E9-04).
    SetLocation(Box<commscraft_lis::Location>),
    /// An emergency call to a number of the location policy (function 39, RE-67): time-bound
    /// `e911active` routing, INVITE with Priority and the PIDF-LO of the current location.
    EmergencyCall {
        dialled: String,
        local: Box<sdp::Local>,
        policy: sdp::Encryption,
    },
    /// Send the quality report of an ended call with the measured network figures (function
    /// 45, QE-01/QE-02).
    ReportQuality {
        call: CallId,
        audio: Box<commscraft_qoe::AudioLine>,
    },
    /// Rate a call 1–5 with an optional text (QE-03).
    RateCall {
        call: CallId,
        stars: u8,
        feedback: Option<String>,
    },
    /// Park the active call at the call park server (function 37, CK-01); retrieve it later by
    /// calling the reported orbit.
    ParkCall {
        call: CallId,
    },
    /// Transfer the call (REFER): to an address, or to another call's party (attended).
    Transfer {
        call: CallId,
        to: TransferTarget,
    },
    /// Place the call a REFER of `call`'s peer asked for (after `Event::TransferRequested`).
    FollowTransfer {
        call: CallId,
        local: Box<sdp::Local>,
        policy: sdp::Encryption,
    },
    /// Put the call on hold (`true`) or resume it (MS-SDPEXT §3.1.5.23).
    Hold {
        call: CallId,
        hold: bool,
    },
    SignOut,
}

/// What the UI is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Registered(Registration),
    SignedOut,
    /// The registration ended without the user asking (deregistered by the server, lost, failed).
    RegistrationEnded(String),
    /// In-band provisioning arrived (RG-41): checked ServerConfiguration (web service URLs …).
    Provisioned(Box<prov::ServerConfiguration>),
    /// The location policy of provisioning (emergency numbers, RG-51).
    LocationPolicy(Box<prov::LocationPolicy>),
    /// The current location for emergency calls (E9-04).
    Location(Box<commscraft_lis::Location>),
    /// Media relay credentials are available (kept in the engine, see
    /// [`Engine::media_relay`]); number of relays and lifetime in minutes (AV-04).
    MediaRelayReady {
        relays: usize,
        minutes: u32,
    },
    ContactList(pres::ContactList),
    Presence(pres::ContactPresence),
    PresenceUnavailable {
        uri: String,
        state: String,
    },
    OwnStatus(pres::UserStatus),
    /// The routing preamble was published (forwarding is in effect on the server).
    Forwarding(pres::Forwarding),
    OwnStatusFailed(u16),
    ConversationStarted {
        conversation: ConversationId,
        peer: String,
        incoming: bool,
    },
    ConversationFailed {
        conversation: ConversationId,
        status: u16,
    },
    Text {
        conversation: ConversationId,
        /// In group conversations: the sender (`Ms-Sender`, CI-06), checked SIP URI.
        sender: Option<String>,
        text: String,
    },
    /// We wait in the lobby of a conference (`true`) or were admitted (`false`) (CB-23).
    ConferenceLobby {
        conversation: ConversationId,
        waiting: bool,
    },
    /// A meeting was scheduled (function 40); join it with its focus URI.
    MeetingScheduled(Box<conf::schedule::Meeting>),
    /// The organizer's meetings.
    Meetings(Vec<conf::schedule::Meeting>),
    /// One meeting's details (join URL, version, expiry).
    MeetingDetails(Box<conf::schedule::Meeting>),
    MeetingCancelled {
        conference_id: String,
    },
    /// The conference's audio call (an ordinary call to the A/V MCU) got this call ID.
    ConferenceAudio {
        conversation: ConversationId,
        call: CallId,
    },
    /// The lock state or meeting options of a conference changed (CB-43).
    MeetingSettings {
        conversation: ConversationId,
        settings: conf::Settings,
    },
    /// The roster of a group conversation changed (CB-24).
    Participants {
        conversation: ConversationId,
        participants: Vec<conf::Participant>,
    },
    /// An invitation to a conference arrived; joining is the user's decision.
    ConferenceInvitation {
        from: String,
        invitation: conf::Invitation,
    },
    /// The IM MCU could not deliver a message to this participant (IMDN, CI-05).
    NotDeliveredTo {
        conversation: ConversationId,
        recipient: String,
        status: u16,
    },
    Typing {
        conversation: ConversationId,
        typing: bool,
    },
    Delivered {
        conversation: ConversationId,
    },
    NotDelivered {
        conversation: ConversationId,
        status: u16,
    },
    ConversationEnded {
        conversation: ConversationId,
    },
    CallRinging {
        call: CallId,
    },
    IncomingCall {
        call: CallId,
        from: String,
    },
    /// The call is established; the adapter starts ICE and media with these parameters.
    CallAnswered {
        call: CallId,
        media: Box<sdp::Negotiated>,
        /// The answer comes from the fork whose early media is running: the media continues
        /// and takes over the differences of this answer (MS-SDPEXT §3.1.5.12.3). Otherwise the
        /// media starts (again) with this answer.
        continues_early: bool,
    },
    /// An SDP answer in a provisional response (early media, MS-SDPEXT §3.1.5.12): ICE may start,
    /// incoming media may be played, only DTMF may be sent (after the first received packet),
    /// speech only after `CallAnswered`.
    CallEarlyMedia {
        call: CallId,
        media: Box<sdp::Negotiated>,
    },
    /// The early media's fork ended (199, MS-SDPEXT §3.1.5.13): stop it, keep waiting.
    CallEarlyMediaEnded {
        call: CallId,
    },
    /// The peer of `call` transfers us to `to` (REFER): prepare media and send `FollowTransfer`.
    TransferRequested {
        call: CallId,
        to: String,
    },
    /// Our transfer of `call` ended: 2xx = done (our call ends), otherwise failed (it stays).
    TransferResult {
        call: CallId,
        status: u16,
    },
    /// New media parameters after a re-INVITE (final offer, hold).
    CallUpdated {
        call: CallId,
        media: Box<sdp::Negotiated>,
    },
    /// The call ended or failed (status code; 0 = normal hang-up).
    /// The call is parked; dial the orbit to retrieve it (CK-02).
    CallParked {
        call: CallId,
        orbit: String,
    },
    /// A parked call left its orbit (CK-04): retrieved (by `target`), hung up, rang back, went to
    /// the fallback destination or was dropped.
    ParkNotification {
        orbit: String,
        reason: commscraft_park::UnparkReason,
        target: Option<String>,
    },
    CallEnded {
        call: CallId,
        status: u16,
    },
    /// Something the user should know but that is not an error of theirs.
    Notice(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Send(Message),
    /// Keep-alive bytes (CRLFCRLF, CON-35).
    SendRaw(&'static [u8]),
    Event(Event),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SubKind {
    ContactList,
    /// Batch subscription to the presence of `uris` at `target` (own AOR or a pool GRUU), or a
    /// single one (`single`: Request-URI = To = the contact).
    Presence {
        target: String,
        uris: Vec<String>,
        single: bool,
    },
}

#[derive(Debug, Clone)]
struct Subscription {
    kind: SubKind,
    dialog: Option<Dialog>,
    refresh_at: Option<u64>,
    /// Retry at this time (after failure or termination).
    retry_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Purpose {
    Subscribe(usize),
    Refresh(usize),
    PublishEndpoint,
    Provisioning,
    PublishStatus(pres::UserStatus),
    PublishRouting(Box<pres::Forwarding>),
    Invite(ConversationId),
    Message(ConversationId),
    CreateConference(ConversationId),
    JoinFocus(ConversationId),
    Roster(ConversationId),
    DialOut(ConversationId),
    MediaRelay(u32),
    CallInvite(CallId),
    CallReinvite(CallId),
    Refer(CallId),
    Schedule(schedule::ScheduleOp),
    Park(u32),
    Quality,
    Ignore,
}

#[derive(Debug, Clone)]
struct Pending {
    purpose: Purpose,
    request: Message,
    deadline: u64,
}

#[derive(Debug, Clone)]
struct Conversation {
    peer: String,
    dialog: Option<Dialog>,
    /// Texts the user wrote before the session was accepted.
    queue: Vec<String>,
    /// Group conversation state (conference); `None` for 1:1.
    group: Option<Box<group::Group>>,
    /// RFC 4028 session timer of `dialog` if we are the refresher.
    timer: Option<group::SessionTimer>,
}

/// The engine of one account.
pub struct Engine<R: Random> {
    cfg: Config,
    reg: Registrar<R>,
    gruu: String,
    registered: bool,
    signing_out: bool,
    keepalive: Option<KeepAlive>,
    pending: HashMap<(String, u32), Pending>,
    subs: Vec<Subscription>,
    convs: HashMap<ConversationId, Conversation>,
    next_conv: ConversationId,
    endpoint_versions: [u32; 2],
    status_versions: [u32; 2],
    /// Versions of the routing publications per container (PR-05).
    routing_versions: [u32; 5],
    /// The routing rules the user published last (restored after an emergency call).
    routing_rules: pres::Forwarding,
    /// The rules above were published by this session (only then they are restored).
    routing_known: bool,
    /// locationPolicy from provisioning (RG-51) and the current location (E9-04).
    location_policy: Option<commscraft_provisioning::LocationPolicy>,
    location: Option<commscraft_lis::Location>,
    /// The emergency call in progress (RE-67).
    emergency_call: Option<CallId>,
    /// Focus factory URI from provisioning (PV-01).
    focus_factory: Option<String>,
    /// Call park server from provisioning (callParkServerUri, CK-01).
    call_park: Option<String>,
    parks: Vec<park::Park>,
    next_park: u32,
    /// QoE URI from provisioning (qosUri, QE-01) and the dialog data of recent calls.
    qos_uri: Option<String>,
    quality_calls: Vec<(CallId, qoe::Kept)>,
    c3p_request: u32,
    /// MRAS URI from provisioning (AV-01) and the current credentials with their refresh time.
    mras_uri: Option<String>,
    /// Location profile for dial strings (RE-40, PS-01).
    location_profile: Option<String>,
    media_relay: Option<avedgea::Credentials>,
    media_relay_refresh_at: Option<u64>,
    calls: HashMap<CallId, call::Call>,
    next_call: CallId,
}

impl<R: Random> std::fmt::Debug for Engine<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("aor", &self.cfg.aor)
            .field("registered", &self.registered)
            .field("subscriptions", &self.subs.len())
            .field("conversations", &self.convs.len())
            .finish_non_exhaustive()
    }
}

fn header_token(m: &Message, name: &'static str) -> Option<String> {
    m.single(name)
        .ok()
        .flatten()
        .and_then(|v| v.split(';').next())
        .map(|v| v.trim().to_ascii_lowercase())
}

fn expires_of(m: &Message) -> Option<u64> {
    m.expires()
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse().ok())
}

impl<R: Random> Engine<R> {
    pub fn new(cfg: Config, reg: Registrar<R>) -> Self {
        let gruu = cfg.aor.clone();
        Self {
            cfg,
            reg,
            gruu,
            registered: false,
            signing_out: false,
            keepalive: None,
            pending: HashMap::new(),
            subs: Vec::new(),
            convs: HashMap::new(),
            next_conv: 1,
            endpoint_versions: [0, 0],
            status_versions: [0, 0],
            routing_versions: [0; 5],
            routing_rules: pres::Forwarding::default(),
            routing_known: false,
            location_policy: None,
            location: None,
            emergency_call: None,
            focus_factory: None,
            call_park: None,
            parks: Vec::new(),
            next_park: 0,
            qos_uri: None,
            quality_calls: Vec::new(),
            c3p_request: 0,
            mras_uri: None,
            location_profile: None,
            media_relay: None,
            media_relay_refresh_at: None,
            calls: HashMap::new(),
            next_call: 1,
        }
    }

    /// Starts the sign-in.
    pub fn start(&mut self, now: u64) -> Vec<Output> {
        vec![Output::Send(self.reg.start(now))]
    }

    pub fn is_registered(&self) -> bool {
        self.registered
    }

    fn hex(&mut self, n: usize) -> String {
        let mut b = vec![0u8; n];
        self.reg.fill_random(&mut b);
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    fn via(&mut self) -> String {
        let branch = self.hex(8);
        format!("SIP/2.0/TLS {};branch=z9hG4bK{branch}", self.cfg.via_host)
    }

    /// A new out-of-dialog request of this endpoint (RE-03, RE-09).
    fn request(&mut self, method: &str, uri: &str, to: &str) -> Message {
        let mut m = Message::request(method, uri);
        let via = self.via();
        m.push_header("Via", &via);
        m.push_header("Max-Forwards", "70");
        let tag = self.hex(5);
        m.push_header(
            "From",
            &format!("<{}>;tag={tag};epid={}", self.cfg.aor, self.cfg.epid),
        );
        m.push_header("To", &format!("<{to}>"));
        let call_id = self.hex(16);
        m.push_header("Call-ID", &call_id);
        m.push_header("CSeq", &format!("1 {method}"));
        m.push_header("Contact", &format!("<{}>", self.gruu));
        m.push_header("User-Agent", &self.cfg.user_agent);
        m
    }

    /// Via, Max-Forwards, Contact and User-Agent for an in-dialog request.
    fn complete(&mut self, m: &mut Message) {
        let via = self.via();
        m.headers.insert(
            0,
            Header {
                name: "Via".into(),
                value: via,
            },
        );
        m.push_header("Max-Forwards", "70");
        m.push_header("Contact", &format!("<{}>", self.gruu));
        m.push_header("User-Agent", &self.cfg.user_agent);
    }

    /// Signs and sends a request and remembers it as a client transaction.
    fn send_request(&mut self, mut m: Message, purpose: Purpose, now: u64, out: &mut Vec<Output>) {
        if self.reg.sign_request(&mut m).is_err() {
            out.push(Output::Event(Event::Notice(
                "could not sign a request".into(),
            )));
            return;
        }
        if let (Ok(Some(call_id)), Ok(Some(cseq))) = (m.call_id(), m.cseq()) {
            if purpose != Purpose::Ignore || !m.method().is_some_and(|x| x == "ACK") {
                self.pending.insert(
                    (call_id.to_string(), cseq.number),
                    Pending {
                        purpose,
                        request: m.clone(),
                        deadline: now + TRANSACTION_TIMEOUT,
                    },
                );
            }
        }
        if let Some(k) = &mut self.keepalive {
            k.on_data_sent(now);
        }
        out.push(Output::Send(m));
    }

    /// Signs and sends an answer or an ACK (no transaction to track).
    fn send_plain(&mut self, mut m: Message, now: u64, out: &mut Vec<Output>) {
        if self.reg.sign_request(&mut m).is_ok() {
            if let Some(k) = &mut self.keepalive {
                k.on_data_sent(now);
            }
            out.push(Output::Send(m));
        }
    }

    // ----- registration -------------------------------------------------------------------

    fn on_registrar_events(&mut self, events: Vec<RegEvent>, now: u64, out: &mut Vec<Output>) {
        for e in events {
            match e {
                RegEvent::Send(m) => {
                    if let Some(k) = &mut self.keepalive {
                        k.on_data_sent(now);
                    }
                    out.push(Output::Send(m));
                }
                RegEvent::Registered(r) => {
                    let first = !self.registered;
                    self.registered = true;
                    if let Some(g) = &r.gruu {
                        self.gruu = g.clone();
                    }
                    if self.keepalive.is_none() {
                        if let Some(t) = r.keepalive_timeout {
                            self.keepalive = Some(KeepAlive::new(t, now));
                        }
                    }
                    out.push(Output::Event(Event::Registered(r)));
                    if first {
                        self.bootstrap(now, out);
                    }
                }
                RegEvent::Unregistered => {
                    self.registered = false;
                    out.push(Output::Event(Event::SignedOut));
                }
                RegEvent::Deregistered(d) => self.ended(
                    &format!("deregistered by the server: {d:?}"),
                    out,
                    matches!(d, Deregistration::Unregistered),
                ),
                RegEvent::Redirected(t) => {
                    self.ended(&format!("redirected to {}:{}", t.host, t.port), out, false)
                }
                RegEvent::SurvivableMode => out.push(Output::Event(Event::Notice(
                    "the server works in survivable mode".into(),
                ))),
                RegEvent::Failed(f) => self.ended(&format!("{f:?}"), out, false),
            }
        }
    }

    fn ended(&mut self, why: &str, out: &mut Vec<Output>, _by_policy: bool) {
        self.registered = false;
        self.pending.clear();
        self.subs.clear();
        for id in self.convs.keys().copied().collect::<Vec<_>>() {
            out.push(Output::Event(Event::ConversationEnded { conversation: id }));
        }
        self.convs.clear();
        out.push(Output::Event(Event::RegistrationEnded(why.to_string())));
    }

    /// OCS-UC03: after the first registration publish the endpoint and subscribe to the contact
    /// list (the presence subscriptions follow when the list is known).
    fn bootstrap(&mut self, now: u64, out: &mut Vec<Output>) {
        self.fetch_provisioning(now, out);
        self.publish_endpoint(now, out);
        self.subs.push(Subscription {
            kind: SubKind::ContactList,
            dialog: None,
            refresh_at: None,
            retry_at: None,
        });
        self.subscribe(0, now, out);
    }

    /// In-band provisioning (RG-40 … RG-43), piggybacked answer (Expires: 0).
    fn fetch_provisioning(&mut self, now: u64, out: &mut Vec<Output>) {
        let via = self.via();
        let tag = self.hex(5);
        let call_id = self.hex(16);
        let m = prov::subscribe(&prov::Request {
            aor: &self.cfg.aor,
            gruu: &self.gruu,
            epid: &self.cfg.epid,
            from_tag: &tag,
            call_id: &call_id,
            cseq: 1,
            via: &via,
            user_agent: &self.cfg.user_agent,
            subnet: None,
        });
        self.send_request(m, Purpose::Provisioning, now, out);
    }

    /// SERVICE to the Media Relay Authentication Service (MS-AVEDGEA §3.1.5, AV-01 … AV-03).
    fn fetch_media_relay(&mut self, now: u64, out: &mut Vec<Output>) {
        let Some(mras) = self.mras_uri.clone() else {
            return;
        };
        let mut id = [0u8; 4];
        self.reg.fill_random(&mut id);
        let request_id = u32::from_be_bytes(id) & 0x7fff_ffff;
        let Ok(body) = avedgea::request(request_id, &self.cfg.aor, &mras) else {
            return;
        };
        let mut m = self.request("SERVICE", &mras, &mras);
        m.push_header("Content-Type", avedgea::CONTENT_TYPE);
        m.body = body.into_bytes();
        self.send_request(m, Purpose::MediaRelay(request_id), now, out);
    }

    /// The current media relay credentials (for the TURN client of a call).
    pub fn media_relay(&self) -> Option<&avedgea::Credentials> {
        self.media_relay.as_ref()
    }

    /// The user's SIP domain (host part of the AOR).
    pub fn user_domain(&self) -> String {
        self.cfg
            .aor
            .rsplit_once('@')
            .map(|(_, d)| d.to_ascii_lowercase())
            .unwrap_or_default()
    }

    /// The AUTHENTICATE_MESSAGE for NTLM over HTTP (web services); the password stays in the
    /// registrar.
    pub fn http_ntlm(
        &mut self,
        negotiate: &[u8],
        challenge: &[u8],
        now: u64,
        server_certificate: Option<&[u8]>,
    ) -> Result<Vec<u8>, String> {
        self.reg
            .http_ntlm(negotiate, challenge, now, server_certificate)
            .map_err(|e| format!("{e:?}"))
    }

    fn publish_endpoint(&mut self, now: u64, out: &mut Vec<Output>) {
        let endpoint = self.reg.instance().to_string();
        let Ok(body) = pres::publish_endpoint_body(
            &self.cfg.aor,
            &endpoint,
            &self.cfg.machine_name,
            3500,
            self.endpoint_versions,
        ) else {
            return;
        };
        let aor = self.cfg.aor.clone();
        let mut m = self.request("SERVICE", &aor, &aor);
        m.push_header("Content-Type", "application/msrtc-category-publish+xml");
        m.body = body.into_bytes();
        self.send_request(m, Purpose::PublishEndpoint, now, out);
    }

    /// RE-50: the routing preamble into the routing category (private, [MS-PRES] §2.2.2.7.7).
    fn publish_routing(&mut self, rules: pres::Forwarding, now: u64, out: &mut Vec<Output>) {
        // RE-67: during an emergency call the preamble with e911active is time-bound.
        let body = if rules.e911_active {
            pres::publish_routing_for_body(
                &self.cfg.aor,
                &rules,
                self.routing_versions,
                emergency::E911_ROUTING_SECONDS,
            )
        } else {
            pres::publish_routing_body(&self.cfg.aor, &rules, self.routing_versions)
        };
        let body = match body {
            Ok(b) => b,
            Err(e) => {
                out.push(Output::Event(Event::Notice(format!(
                    "forwarding not usable ({e:?})"
                ))));
                return;
            }
        };
        let aor = self.cfg.aor.clone();
        let mut m = self.request("SERVICE", &aor, &aor);
        m.push_header("Content-Type", "application/msrtc-category-publish+xml");
        m.body = body.into_bytes();
        self.send_request(m, Purpose::PublishRouting(Box::new(rules)), now, out);
    }

    fn publish_status(&mut self, status: pres::UserStatus, now: u64, out: &mut Vec<Output>) {
        let endpoint = self.reg.instance().to_string();
        let Ok(body) =
            pres::publish_state_body(&self.cfg.aor, status, self.status_versions, Some(&endpoint))
        else {
            return;
        };
        let aor = self.cfg.aor.clone();
        let mut m = self.request("SERVICE", &aor, &aor);
        m.push_header("Content-Type", "application/msrtc-category-publish+xml");
        m.body = body.into_bytes();
        self.send_request(m, Purpose::PublishStatus(status), now, out);
    }

    // ----- subscriptions -------------------------------------------------------------------

    fn subscribe(&mut self, index: usize, now: u64, out: &mut Vec<Output>) {
        let Some(sub) = self.subs.get(index).cloned() else {
            return;
        };
        let aor = self.cfg.aor.clone();
        let m = match &sub.kind {
            SubKind::ContactList => {
                let mut m = self.request("SUBSCRIBE", &aor, &aor);
                pres::contacts_headers(&mut m, false);
                m
            }
            SubKind::Presence {
                target,
                uris,
                single,
            } => {
                let to = if *single { target.clone() } else { aor.clone() };
                let mut m = self.request("SUBSCRIBE", target, &to);
                if *single {
                    pres::single_headers(&mut m, false);
                } else {
                    pres::batch_headers(&mut m, false);
                }
                let mut id = [0u8; 4];
                self.reg.fill_random(&mut id);
                match pres::batch_body(&aor, uris, u32::from_be_bytes(id) & 0x7fff_ffff) {
                    Ok(b) => m.body = b.into_bytes(),
                    Err(_) => return,
                }
                m
            }
        };
        if let Some(s) = self.subs.get_mut(index) {
            s.retry_at = None;
        }
        self.send_request(m, Purpose::Subscribe(index), now, out);
    }

    fn refresh(&mut self, index: usize, now: u64, out: &mut Vec<Output>) {
        let Some(sub) = self.subs.get_mut(index) else {
            return;
        };
        let Some(mut d) = sub.dialog.take() else {
            return;
        };
        let mut m = d.request("SUBSCRIBE");
        let kind = sub.kind.clone();
        sub.dialog = Some(d);
        sub.refresh_at = None;
        self.complete(&mut m);
        match &kind {
            SubKind::ContactList => {
                m.push_header("Event", "vnd-microsoft-roaming-contacts");
                m.push_header("Accept", "application/vnd-microsoft-roaming-contacts+xml");
            }
            SubKind::Presence { uris, single, .. } => {
                m.push_header("Event", "presence");
                m.push_header("Accept", "application/msrtc-event-categories+xml, application/rlmi+xml, multipart/related");
                if !*single {
                    m.push_header("Require", "adhoclist, categoryList");
                    m.push_header("Supported", "eventlist");
                }
                m.push_header("Content-Type", "application/msrtc-adrl-categorylist+xml");
                let mut id = [0u8; 4];
                self.reg.fill_random(&mut id);
                if let Ok(b) =
                    pres::batch_body(&self.cfg.aor, uris, u32::from_be_bytes(id) & 0x7fff_ffff)
                {
                    m.body = b.into_bytes();
                }
            }
        }
        self.send_request(m, Purpose::Refresh(index), now, out);
    }

    fn schedule_retry(&mut self, index: usize, now: u64) {
        if let Some(s) = self.subs.get_mut(index) {
            s.dialog = None;
            s.refresh_at = None;
            s.retry_at = Some(now + RESUBSCRIBE_DELAY);
        }
    }

    /// A 2xx to a SUBSCRIBE: dialog, refresh time, piggybacked document (SIP-04).
    fn on_subscribed(
        &mut self,
        index: usize,
        req: &Message,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let expires = expires_of(resp).unwrap_or(0);
        if let Some(s) = self.subs.get_mut(index) {
            if s.dialog.is_none() {
                s.dialog = Dialog::from_response(req, resp).ok();
            }
            s.refresh_at = (expires > 0).then(|| now + (expires * 4 / 5).max(1));
        }
        if !resp.body.is_empty() {
            self.on_document(index, resp, now, out);
        }
        if expires == 0 {
            self.schedule_retry(index, now);
        }
    }

    fn on_document(&mut self, index: usize, m: &Message, now: u64, out: &mut Vec<Output>) {
        let Some(kind) = self.subs.get(index).map(|s| s.kind.clone()) else {
            return;
        };
        match kind {
            SubKind::ContactList => {
                let Ok(body) = std::str::from_utf8(&m.body) else {
                    return;
                };
                match pres::contact_document_kind(body) {
                    pres::ContactDocument::Full => match pres::parse_contact_list(body) {
                        Ok(list) => {
                            let uris: Vec<String> =
                                list.contacts.iter().map(|c| c.uri.clone()).collect();
                            out.push(Output::Event(Event::ContactList(list)));
                            self.replace_presence_subscriptions(uris, now, out);
                        }
                        Err(e) => out.push(Output::Event(Event::Notice(format!(
                            "unusable contact list ({e:?})"
                        )))),
                    },
                    // A change: fetch the full list again with a new subscription (MS-SIP §3.7.5.2).
                    pres::ContactDocument::Delta => self.resubscribe_now(index, now, out),
                    pres::ContactDocument::Other => {}
                }
            }
            SubKind::Presence { .. } => match pres::parse_batch(m) {
                Ok((states, contacts)) => {
                    for c in contacts {
                        out.push(Output::Event(Event::Presence(c)));
                    }
                    self.on_resource_states(states, now, out);
                }
                Err(e) => out.push(Output::Event(Event::Notice(format!(
                    "unusable presence document ({e:?})"
                )))),
            },
        }
    }

    fn resubscribe_now(&mut self, index: usize, now: u64, out: &mut Vec<Output>) {
        if let Some(s) = self.subs.get_mut(index) {
            s.dialog = None;
            s.refresh_at = None;
        }
        self.subscribe(index, now, out);
    }

    /// The contact list changed: one batch subscription per 100 contacts to the own AOR.
    fn replace_presence_subscriptions(
        &mut self,
        uris: Vec<String>,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        // Existing presence subscriptions are ended (RFC 3265 §3.3.4) and replaced.
        let old: Vec<usize> = (0..self.subs.len())
            .filter(|i| {
                matches!(
                    self.subs.get(*i).map(|s| &s.kind),
                    Some(SubKind::Presence { .. })
                )
            })
            .collect();
        for i in old.iter().rev() {
            self.unsubscribe(*i, now, out);
            self.subs.remove(*i);
        }
        self.pending.retain(
            |_, p| !matches!(p.purpose, Purpose::Subscribe(i) | Purpose::Refresh(i) if i >= 1),
        );
        let aor = self.cfg.aor.clone();
        for chunk in uris.chunks(pres::MAX_BATCH) {
            if self.subs.len() >= MAX_SUBSCRIPTIONS {
                break;
            }
            self.subs.push(Subscription {
                kind: SubKind::Presence {
                    target: aor.clone(),
                    uris: chunk.to_vec(),
                    single: false,
                },
                dialog: None,
                refresh_at: None,
                retry_at: None,
            });
            let i = self.subs.len() - 1;
            self.subscribe(i, now, out);
        }
    }

    /// RLMI states: follow `resubscribe` to the pool or with single subscriptions
    /// (§3.4.5.1.3.1); report everything else.
    fn on_resource_states(
        &mut self,
        states: Vec<pres::ResourceState>,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let mut pools: Vec<(String, Vec<String>)> = Vec::new();
        for st in states {
            if st.state == "active" || st.state == "pending" {
                continue;
            }
            let Ok(uri) = pres::checked_sip_uri(&st.uri) else {
                continue;
            };
            if st.state == "resubscribe" {
                match st.pool_fqdn {
                    Some(pool) => match pools.iter_mut().find(|(p, _)| *p == pool) {
                        Some((_, v)) => v.push(uri),
                        None => pools.push((pool, vec![uri])),
                    },
                    None => self.add_presence_sub(uri.clone(), vec![uri], true, now, out),
                }
            } else {
                out.push(Output::Event(Event::PresenceUnavailable {
                    uri,
                    state: st.state,
                }));
            }
        }
        for (pool, uris) in pools {
            for chunk in uris.chunks(pres::MAX_BATCH) {
                self.add_presence_sub(pool.clone(), chunk.to_vec(), false, now, out);
            }
        }
    }

    fn add_presence_sub(
        &mut self,
        target: String,
        uris: Vec<String>,
        single: bool,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let exists = self.subs.iter().any(|s| matches!(&s.kind, SubKind::Presence { target: t, uris: u, .. } if *t == target && *u == uris));
        if exists || self.subs.len() >= MAX_SUBSCRIPTIONS {
            return;
        }
        self.subs.push(Subscription {
            kind: SubKind::Presence {
                target,
                uris,
                single,
            },
            dialog: None,
            refresh_at: None,
            retry_at: None,
        });
        let i = self.subs.len() - 1;
        self.subscribe(i, now, out);
    }

    fn unsubscribe(&mut self, index: usize, now: u64, out: &mut Vec<Output>) {
        let Some(sub) = self.subs.get_mut(index) else {
            return;
        };
        let Some(mut d) = sub.dialog.take() else {
            return;
        };
        let event = match sub.kind {
            SubKind::ContactList => "vnd-microsoft-roaming-contacts",
            SubKind::Presence { .. } => "presence",
        };
        let mut m = d.request("SUBSCRIBE");
        self.complete(&mut m);
        m.push_header("Event", event);
        m.push_header("Expires", "0");
        self.send_request(m, Purpose::Ignore, now, out);
    }

    /// A NOTIFY in one of our subscription dialogs (RFC 3265 §3.2.4; SIP-02).
    fn on_notify(&mut self, index: usize, req: &Message, now: u64, out: &mut Vec<Output>) {
        let ok = response_to(req, 200, "OK");
        self.send_plain(ok, now, out);
        if !req.body.is_empty() {
            self.on_document(index, req, now, out);
        }
        let state = header_token(req, "subscription-state").unwrap_or_default();
        if state == "terminated" {
            self.schedule_retry(index, now);
        }
    }

    // ----- conversations -------------------------------------------------------------------

    fn start_conversation(&mut self, to: &str, now: u64, out: &mut Vec<Output>) {
        let Ok(peer) = pres::checked_sip_uri(to) else {
            out.push(Output::Event(Event::Notice(
                "not a usable SIP address".into(),
            )));
            return;
        };
        if self.convs.len() >= MAX_CONVERSATIONS {
            out.push(Output::Event(Event::Notice(
                "too many open conversations".into(),
            )));
            return;
        }
        let id = self.next_conv;
        self.next_conv = self.next_conv.wrapping_add(1).max(1);
        let mut m = self.request("INVITE", &peer, &peer);
        im::invite_body(&mut m, self.cfg.local_ip, false);
        self.convs.insert(
            id,
            Conversation {
                peer,
                dialog: None,
                queue: Vec::new(),
                group: None,
                timer: None,
            },
        );
        self.send_request(m, Purpose::Invite(id), now, out);
        if let Some(p) = self
            .pending
            .values_mut()
            .find(|p| p.purpose == Purpose::Invite(id))
        {
            // An INVITE may ring longer than a transaction (RFC 3261 §13.3.1.1, timer C ≥ 3 min).
            p.deadline = now + 180;
        }
    }

    fn send_text(&mut self, id: ConversationId, text: String, now: u64, out: &mut Vec<Output>) {
        let Some(c) = self.convs.get_mut(&id) else {
            return;
        };
        let Some(mut d) = c.dialog.take() else {
            c.queue.push(text);
            return;
        };
        let msg = im::message(&mut d, &text);
        c.dialog = Some(d);
        match msg {
            Ok(mut m) => {
                self.complete(&mut m);
                self.send_request(m, Purpose::Message(id), now, out);
            }
            Err(e) => out.push(Output::Event(Event::Notice(format!(
                "message not sent ({e:?})"
            )))),
        }
    }

    fn in_dialog(
        &mut self,
        id: ConversationId,
        f: impl FnOnce(&mut Dialog) -> Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(c) = self.convs.get_mut(&id) else {
            return;
        };
        let Some(mut d) = c.dialog.take() else { return };
        let mut m = f(&mut d);
        c.dialog = Some(d);
        self.complete(&mut m);
        self.send_request(m, Purpose::Ignore, now, out);
    }

    /// An INVITE of a peer (out of dialog): accept IM sessions, refuse everything else.
    fn on_invite(&mut self, req: &Message, now: u64, out: &mut Vec<Output>) {
        if header_token(req, "Content-Type").as_deref() == Some(conf::INVITATION_TYPE) {
            self.on_invitation(req, now, out);
            return;
        }
        if let Some(offer) = call::audio_sdp(req) {
            self.on_incoming_call(req, offer, now, out);
            return;
        }
        let is_sdp = header_token(req, "Content-Type").as_deref() == Some("application/sdp");
        if !is_sdp || im::check_sdp(&req.body).is_err() {
            // Audio/video and other sessions come with M3; 488 = not acceptable here.
            let r = response_to(req, 488, "Not Acceptable Here");
            self.send_plain(r, now, out);
            return;
        }
        if self.convs.len() >= MAX_CONVERSATIONS {
            let r = response_to(req, 486, "Busy Here");
            self.send_plain(r, now, out);
            return;
        }
        let tag = self.hex(5);
        let Ok(dialog) = Dialog::from_request(req, &tag) else {
            let r = response_to(req, 400, "Bad Request");
            self.send_plain(r, now, out);
            return;
        };
        let group = Self::dialed_out_group(req);
        let peer = match &group {
            Some(g) => g.focus.clone().unwrap_or_default(),
            None => dialog.remote_uri.clone(),
        };
        let mut ok = response_to(req, 200, "OK");
        for h in ok.headers.iter_mut().filter(|h| h.name == "To") {
            h.value = format!("{};tag={tag}", h.value);
        }
        for rr in req.header_values("Record-Route") {
            ok.push_header("Record-Route", rr);
        }
        ok.push_header("Contact", &format!("<{}>", self.gruu));
        // CI-03: ms-sender, so that the IM MCU tells us the sender of every message.
        ok.push_header("Supported", "ms-sender");
        // RFC 4028 §9: confirm the session timer of the offer.
        let timer = group::our_session_timer(req, false, now);
        if let Some(se) = req.single("Session-Expires").ok().flatten() {
            let refresher = if timer.is_some() { "uas" } else { "uac" };
            if let Some(n) = se.split(';').next().map(str::trim) {
                if n.parse::<u64>().is_ok() {
                    ok.push_header("Session-Expires", &format!("{n};refresher={refresher}"));
                    ok.push_header("Require", "timer");
                }
            }
        }
        ok.push_header("Content-Type", "application/sdp");
        ok.body = im::sdp(self.cfg.local_ip, group.is_some()).into_bytes();
        self.send_plain(ok, now, out);
        let id = self.next_conv;
        self.next_conv = self.next_conv.wrapping_add(1).max(1);
        self.convs.insert(
            id,
            Conversation {
                peer: peer.clone(),
                dialog: Some(dialog),
                queue: Vec::new(),
                group: group.map(Box::new),
                timer,
            },
        );
        out.push(Output::Event(Event::ConversationStarted {
            conversation: id,
            peer,
            incoming: true,
        }));
        // CI-13: the IM MCU dialled out to us; join the focus of the same conference too, for
        // the roster (participants, the A/V MCU for conference audio). Only if the announced
        // focus belongs to the calling MCU's conference.
        let focus = self
            .convs
            .get(&id)
            .and_then(|c| c.group.as_ref())
            .and_then(|g| g.focus.clone());
        let mcu = self
            .convs
            .get(&id)
            .and_then(|c| c.dialog.as_ref())
            .map(|d| d.remote_uri.clone());
        if let (Some(focus), Some(mcu)) = (focus, mcu) {
            if conf::same_conference(&focus, &mcu) {
                self.join_focus(id, now, out);
            }
        }
    }

    fn on_conversation_request(
        &mut self,
        id: ConversationId,
        req: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        if req.method().is_some_and(|m| m.eq_ignore_ascii_case("ACK")) {
            return;
        }
        if req
            .method()
            .is_some_and(|m| m.eq_ignore_ascii_case("UPDATE"))
        {
            // Session refresh by the peer (RFC 4028).
            let mut r = response_to(req, 200, "OK");
            r.push_header("Supported", "timer");
            self.send_plain(r, now, out);
            return;
        }
        let is_group = self.convs.get(&id).is_some_and(|c| c.group.is_some());
        let (incoming, answer) = im::handle_request(req);
        self.send_plain(answer, now, out);
        match incoming {
            im::Incoming::Text(text) => {
                let sender = if is_group {
                    req.single("Ms-Sender")
                        .ok()
                        .flatten()
                        .and_then(conf::ms_sender)
                        .map(|(_, uri)| uri)
                } else {
                    None
                };
                out.push(Output::Event(Event::Text {
                    conversation: id,
                    sender,
                    text,
                }))
            }
            im::Incoming::Typing(typing) => out.push(Output::Event(Event::Typing {
                conversation: id,
                typing,
            })),
            im::Incoming::Bye => {
                if is_group {
                    // The IM MCU ended the session (ejection or end, CI-09): leave the conference.
                    if let Some(c) = self.convs.get_mut(&id) {
                        c.dialog = None;
                    }
                    self.leave_group(id, now, out, true);
                } else {
                    self.convs.remove(&id);
                    out.push(Output::Event(Event::ConversationEnded { conversation: id }));
                }
            }
            im::Incoming::Refused(_) => {}
        }
    }

    // ----- input ---------------------------------------------------------------------------

    /// A user command.
    pub fn command(&mut self, cmd: Command, now: u64) -> Vec<Output> {
        let mut out = Vec::new();
        if !self.registered {
            out.push(Output::Event(Event::Notice("not signed in".into())));
            return out;
        }
        match cmd {
            Command::SetStatus(s) => self.publish_status(s, now, &mut out),
            Command::SetForwarding(f) => self.publish_routing(f, now, &mut out),
            Command::StartConversation { to } => self.start_conversation(&to, now, &mut out),
            Command::SendText { conversation, text } => {
                self.send_text(conversation, text, now, &mut out)
            }
            Command::Typing {
                conversation,
                typing,
            } => self.in_dialog(conversation, |d| im::typing(d, typing), now, &mut out),
            Command::EndConversation { conversation } => {
                if self
                    .convs
                    .get(&conversation)
                    .is_some_and(|c| c.group.is_some())
                {
                    self.leave_group(conversation, now, &mut out, true);
                } else {
                    self.in_dialog(conversation, |d| d.request("BYE"), now, &mut out);
                    self.convs.remove(&conversation);
                    out.push(Output::Event(Event::ConversationEnded { conversation }));
                }
            }
            Command::StartGroup { invite } => self.start_group(&invite, now, &mut out),
            Command::InviteToGroup { conversation, to } => {
                self.dial_out(conversation, &to, now, &mut out)
            }
            Command::JoinConference { focus } => self.join_conference(&focus, now, &mut out),
            Command::ScheduleMeeting {
                subject,
                minutes,
                admission,
                extras,
            } => self.schedule_meeting(
                schedule::Planned {
                    subject,
                    minutes,
                    admission,
                    extras,
                },
                now,
                &mut out,
            ),
            Command::ScheduleStaticMeeting => self.schedule_static_meeting(now, &mut out),
            Command::ListMeetings { static_meetings } => {
                self.list_meetings(static_meetings, now, &mut out)
            }
            Command::ShowMeeting { conference_id } => {
                self.show_meeting(&conference_id, now, &mut out)
            }
            Command::ChangeMeeting {
                conference_id,
                subject,
                minutes,
                admission,
                extras,
            } => self.change_meeting(
                &conference_id,
                schedule::Planned {
                    subject,
                    minutes,
                    admission,
                    extras,
                },
                now,
                &mut out,
            ),
            Command::CancelMeeting {
                conference_id,
                static_meeting,
            } => self.cancel_meeting(&conference_id, static_meeting, now, &mut out),
            Command::JoinConferenceAudio {
                conversation,
                local,
                policy,
            } => self.join_conference_audio(conversation, *local, policy, now, &mut out),
            Command::ConferenceControl {
                conversation,
                action,
            } => self.conference_control(conversation, action, now, &mut out),
            Command::ConferenceMute { conversation, mute } => {
                self.conference_mute(conversation, mute, now, &mut out)
            }
            Command::Call { to, local, policy } => {
                self.start_call(&to, *local, policy, now, &mut out)
            }
            Command::AcceptCall {
                call,
                local,
                policy,
            } => self.accept_call(call, *local, policy, now, &mut out),
            Command::UpdateCall { call, local } => self.update_call(call, *local, now, &mut out),
            Command::Hangup { call } => self.hangup(call, now, &mut out),
            Command::ParkCall { call } => self.park_call(call, now, &mut out),
            Command::SetLocation(loc) => self.set_location(*loc, &mut out),
            Command::EmergencyCall {
                dialled,
                local,
                policy,
            } => self.emergency_call(&dialled, *local, policy, now, &mut out),
            Command::ReportQuality { call, audio } => {
                self.report_quality(call, *audio, now, &mut out)
            }
            Command::RateCall {
                call,
                stars,
                feedback,
            } => self.rate_call(call, stars, feedback.as_deref(), now, &mut out),
            Command::Transfer { call, to } => self.transfer(call, to, now, &mut out),
            Command::FollowTransfer {
                call,
                local,
                policy,
            } => self.follow_transfer(call, *local, policy, now, &mut out),
            Command::Hold { call, hold } => self.hold_call(call, hold, now, &mut out),
            Command::SignOut => {
                self.signing_out = true;
                self.end_all_calls(now, &mut out);
                self.leave_parks(now, &mut out);
                for i in (0..self.subs.len()).rev() {
                    self.unsubscribe(i, now, &mut out);
                }
                self.subs.clear();
                let ids: Vec<ConversationId> = self.convs.keys().copied().collect();
                for id in ids {
                    if self.convs.get(&id).is_some_and(|c| c.group.is_some()) {
                        self.leave_group(id, now, &mut out, true);
                    } else {
                        self.in_dialog(id, |d| d.request("BYE"), now, &mut out);
                        out.push(Output::Event(Event::ConversationEnded { conversation: id }));
                    }
                }
                self.convs.clear();
                match self.reg.unregister(now) {
                    Ok(m) => out.push(Output::Send(m)),
                    Err(_) => out.push(Output::Event(Event::SignedOut)),
                }
            }
        }
        out
    }

    /// The clock: registration refresh, keep-alive, subscription refresh and retries,
    /// transaction timeouts.
    pub fn on_tick(&mut self, now: u64) -> Vec<Output> {
        let mut out = Vec::new();
        let events = self.reg.on_tick(now);
        self.on_registrar_events(events, now, &mut out);
        if let Some(k) = &mut self.keepalive {
            if let Some(bytes) = k.on_tick(now) {
                out.push(Output::SendRaw(bytes));
            }
        }
        if !self.registered {
            return out;
        }
        for i in 0..self.subs.len() {
            let Some(s) = self.subs.get(i) else { continue };
            if s.refresh_at.is_some_and(|t| now >= t) {
                self.refresh(i, now, &mut out);
            } else if s.retry_at.is_some_and(|t| now >= t) {
                self.subscribe(i, now, &mut out);
            }
        }
        self.group_tick(now, &mut out);
        self.call_tick(now, &mut out);
        self.park_tick(now, &mut out);
        self.emergency_tick(now, &mut out);
        if self.media_relay_refresh_at.is_some_and(|t| now >= t) {
            self.media_relay_refresh_at = None;
            self.fetch_media_relay(now, &mut out);
        }
        let expired: Vec<(String, u32)> = self
            .pending
            .iter()
            .filter(|(_, p)| now >= p.deadline)
            .map(|(k, _)| k.clone())
            .collect();
        for k in expired {
            if let Some(p) = self.pending.remove(&k) {
                self.on_failure(p, 408, now, &mut out);
            }
        }
        out
    }

    /// A message from the connection.
    pub fn on_message(&mut self, msg: &Message, now: u64) -> Vec<Output> {
        let mut out = Vec::new();
        let events = self.reg.on_message(msg, now);
        if !events.is_empty() {
            self.on_registrar_events(events, now, &mut out);
            return out;
        }
        if !self.registered {
            return out;
        }
        // AE-32: everything else must be signed by the server; unsigned messages are dropped.
        if !self.reg.verify_message(msg) {
            return out;
        }
        if msg.is_response() {
            self.on_response(msg, now, &mut out);
        } else {
            self.on_request(msg, now, &mut out);
        }
        out
    }

    fn on_response(&mut self, resp: &Message, now: u64, out: &mut Vec<Output>) {
        let (Ok(Some(call_id)), Ok(Some(cseq))) = (resp.call_id(), resp.cseq()) else {
            return;
        };
        let key = (call_id.to_string(), cseq.number);
        let Some(code) = resp.status_code() else {
            return;
        };
        if code < 200 {
            if let Some((purpose, request)) = self
                .pending
                .get(&key)
                .map(|p| (p.purpose.clone(), p.request.clone()))
            {
                self.on_call_provisional(&purpose, &request, resp, now, out);
            }
            return;
        }
        let Some(p) = self.pending.remove(&key) else {
            return;
        };
        if !p
            .request
            .method()
            .is_some_and(|m| m.eq_ignore_ascii_case(cseq.method))
        {
            self.pending.insert(key, p);
            return;
        }
        if (200..300).contains(&code) {
            self.on_success(p, resp, now, out);
        } else {
            if p.request.method() == Some("INVITE") {
                let ack = im::ack_for_failure(&p.request, resp);
                self.send_plain(ack, now, out);
            }
            if code == 409 {
                if let Some(retry) = self.on_conflict(&p, resp) {
                    match retry {
                        Purpose::PublishEndpoint => self.publish_endpoint(now, out),
                        Purpose::PublishStatus(s) => self.publish_status(s, now, out),
                        Purpose::PublishRouting(f) => self.publish_routing(*f, now, out),
                        _ => {}
                    }
                    return;
                }
            }
            // PV-11: an error response of the focus factory may carry the C3P answer with the
            // reason (example §4.3: 404 with conferenceDoesNotExist).
            if let Purpose::Schedule(op) = &p.purpose {
                if !resp.body.is_empty() {
                    self.on_schedule_answer(op.clone(), &resp.body, now, out);
                    return;
                }
            }
            self.on_failure(p, code, now, out);
        }
    }

    /// PR-05: a 409 with `curVersion` – remember the versions and retry once.
    fn on_conflict(&mut self, p: &Pending, resp: &Message) -> Option<Purpose> {
        let body = std::str::from_utf8(&resp.body).ok()?;
        let conflicts = pres::parse_fault(body).ok()?;
        let target = match p.purpose {
            Purpose::PublishEndpoint => &mut self.endpoint_versions,
            Purpose::PublishStatus(_) => &mut self.status_versions[..],
            Purpose::PublishRouting(_) => &mut self.routing_versions[..],
            _ => return None,
        };
        let mut changed = false;
        for c in conflicts {
            if let Some(v) = target.get_mut((c.index as usize).saturating_sub(1)) {
                if *v != c.cur_version {
                    *v = c.cur_version;
                    changed = true;
                }
            }
        }
        changed.then(|| p.purpose.clone())
    }

    fn on_success(&mut self, p: Pending, resp: &Message, now: u64, out: &mut Vec<Output>) {
        match p.purpose {
            Purpose::Subscribe(i) | Purpose::Refresh(i) => {
                self.on_subscribed(i, &p.request, resp, now, out)
            }
            Purpose::PublishEndpoint => {
                self.endpoint_versions = self.endpoint_versions.map(|v| v + 1)
            }
            Purpose::Provisioning => match prov::from_message(resp) {
                Ok(Some(p)) => {
                    // RG-51: the location policy decides about emergency numbers.
                    if let Some(g) = p.group("locationPolicy") {
                        let policy = prov::LocationPolicy::from_group(g);
                        out.push(Output::Event(Event::LocationPolicy(Box::new(
                            policy.clone(),
                        ))));
                        self.location_policy = Some(policy);
                    }
                    if let Some(g) = p.group("ServerConfiguration") {
                        let mut c = prov::ServerConfiguration::from_group(g, &self.user_domain());
                        if let Some(u) = p.group("userSetting") {
                            c.apply_user_setting(u);
                        }
                        self.location_profile = c.location_profile.clone();
                        self.focus_factory = c
                            .focus_factory_uri
                            .as_deref()
                            .and_then(|u| pres::checked_sip_uri(u).ok());
                        self.call_park = c
                            .call_park_server_uri
                            .as_deref()
                            .and_then(|u| pres::checked_sip_uri(u).ok());
                        self.qos_uri = c
                            .qos_uri
                            .as_deref()
                            .and_then(|u| pres::checked_sip_uri(u).ok());
                        self.mras_uri = c
                            .mras_uri
                            .as_deref()
                            .and_then(|u| pres::checked_sip_uri(u).ok());
                        out.push(Output::Event(Event::Provisioned(Box::new(c))));
                        // AV-09: fetch relay credentials during bootstrap.
                        self.fetch_media_relay(now, out);
                    }
                }
                Ok(None) => out.push(Output::Event(Event::Notice(
                    "provisioning without document".into(),
                ))),
                Err(e) => out.push(Output::Event(Event::Notice(format!(
                    "unusable provisioning ({e:?})"
                )))),
            },
            Purpose::PublishStatus(s) => {
                self.status_versions = self.status_versions.map(|v| v + 1);
                out.push(Output::Event(Event::OwnStatus(s)));
            }
            Purpose::PublishRouting(f) => {
                self.routing_versions = self.routing_versions.map(|v| v + 1);
                // The user's own rules are kept to restore them after an emergency call.
                if !f.e911_active {
                    self.routing_rules = (*f).clone();
                    self.routing_known = true;
                    out.push(Output::Event(Event::Forwarding(*f)));
                }
            }
            Purpose::Invite(id) => {
                let Some(c) = self.convs.get_mut(&id) else {
                    return;
                };
                let (Ok(d), Ok(())) = (
                    Dialog::from_response(&p.request, resp),
                    im::check_sdp(&resp.body),
                ) else {
                    out.push(Output::Event(Event::ConversationFailed {
                        conversation: id,
                        status: 488,
                    }));
                    return;
                };
                let peer = c.peer.clone();
                let queue = std::mem::take(&mut c.queue);
                c.dialog = Some(d.clone());
                c.timer = group::our_session_timer(resp, true, now);
                let is_group = c.group.is_some();
                let invite_cseq = p.request.cseq().ok().flatten().map_or(1, |x| x.number);
                let mut ack = im::ack_for_2xx(&d, invite_cseq);
                self.complete(&mut ack);
                self.send_plain(ack, now, out);
                out.push(Output::Event(Event::ConversationStarted {
                    conversation: id,
                    peer,
                    incoming: false,
                }));
                for text in queue {
                    self.send_text(id, text, now, out);
                }
                if is_group {
                    self.on_chat_joined(id, now, out);
                }
            }
            Purpose::Message(id) => {
                if resp.status_code() == Some(202) {
                    self.on_message_accepted(id, resp, now);
                } else {
                    out.push(Output::Event(Event::Delivered { conversation: id }))
                }
            }
            Purpose::CreateConference(id) => self.on_conference_created(id, resp, now, out),
            Purpose::Schedule(op) => self.on_schedule_answer(op, &resp.body, now, out),
            Purpose::Park(id) => self.on_parked(id, &p.request, resp, now, out),
            Purpose::Quality => {}
            Purpose::CallInvite(id) => self.on_call_answered(id, &p.request, resp, now, out),
            Purpose::CallReinvite(id) => {
                self.on_call_reinvite_answered(id, &p.request, resp, now, out)
            }
            Purpose::Refer(id) => {
                let code = resp.status_code().unwrap_or(500);
                self.on_refer_answered(id, code, now, out)
            }
            Purpose::MediaRelay(request_id) => {
                let parsed = std::str::from_utf8(&resp.body)
                    .map_err(|_| avedgea::Error::BadValue("UTF-8"))
                    .and_then(|b| avedgea::parse_response(b, request_id));
                match parsed {
                    Ok(c) => {
                        // Refresh at 90 % of the lifetime (design; AV-06 token lifetime).
                        self.media_relay_refresh_at =
                            Some(now + u64::from(c.duration_minutes) * 60 * 9 / 10);
                        out.push(Output::Event(Event::MediaRelayReady {
                            relays: c.relays.len(),
                            minutes: c.duration_minutes,
                        }));
                        self.media_relay = Some(c);
                    }
                    Err(e) => {
                        self.media_relay_refresh_at = Some(now + 300);
                        out.push(Output::Event(Event::Notice(format!(
                            "unusable media relay answer ({e:?})"
                        ))));
                    }
                }
            }
            Purpose::JoinFocus(id) => self.on_focus_joined(id, &p.request, resp, now, out),
            Purpose::Roster(id) => self.on_roster_subscribed(id, &p.request, resp, now, out),
            Purpose::DialOut(id) => {
                if resp.status_code() == Some(200) && !resp.body.is_empty() {
                    self.on_c3p_answer(id, &resp.body, out);
                }
            }
            Purpose::Ignore => {}
        }
    }

    fn on_failure(&mut self, p: Pending, code: u16, now: u64, out: &mut Vec<Output>) {
        match p.purpose {
            Purpose::Subscribe(i) | Purpose::Refresh(i) => {
                // 481: the dialog is gone – subscribe again at once; otherwise retry later.
                if code == 481 {
                    self.resubscribe_now(i, now, out);
                } else {
                    self.schedule_retry(i, now);
                }
            }
            Purpose::PublishStatus(_) => out.push(Output::Event(Event::OwnStatusFailed(code))),
            Purpose::PublishRouting(_) => out.push(Output::Event(Event::Notice(format!(
                "forwarding not set ({code})"
            )))),
            Purpose::PublishEndpoint => out.push(Output::Event(Event::Notice(format!(
                "endpoint publication failed ({code})"
            )))),
            Purpose::MediaRelay(_) => {
                // Retry later; calls then work only without the relay.
                self.media_relay_refresh_at = Some(now + 300);
                out.push(Output::Event(Event::Notice(format!(
                    "media relay credentials not available ({code})"
                ))));
            }
            Purpose::Provisioning => out.push(Output::Event(Event::Notice(format!(
                "provisioning failed ({code})"
            )))),
            Purpose::Invite(id) => {
                if self.convs.get(&id).is_some_and(|c| c.group.is_some()) {
                    out.push(Output::Event(Event::Notice(format!(
                        "the IM conference refused us ({code})"
                    ))));
                    self.leave_group(id, now, out, true);
                } else {
                    self.convs.remove(&id);
                    out.push(Output::Event(Event::ConversationFailed {
                        conversation: id,
                        status: code,
                    }));
                }
            }
            // CI-13: the IM session the MCU dialled out to us goes on without the roster.
            Purpose::JoinFocus(id)
                if self
                    .convs
                    .get(&id)
                    .and_then(|c| c.group.as_ref())
                    .is_some_and(|g| g.dialed_out) =>
            {
                out.push(Output::Event(Event::Notice(format!(
                    "conference state not available ({code}); the chat goes on"
                ))));
            }
            Purpose::CreateConference(id) | Purpose::JoinFocus(id) => {
                self.convs.remove(&id);
                out.push(Output::Event(Event::ConversationFailed {
                    conversation: id,
                    status: code,
                }));
            }
            Purpose::Roster(id) => {
                // Without roster no chat URI: give up unless the chat session is already up.
                let joined = self.convs.get(&id).is_some_and(|c| c.dialog.is_some());
                if !joined {
                    out.push(Output::Event(Event::Notice(format!(
                        "conference state not available ({code})"
                    ))));
                    self.leave_group(id, now, out, true);
                }
            }
            Purpose::CallInvite(id) => {
                self.transferred_call_result(id, code, now, out);
                self.on_call_failed(id, code, out)
            }
            Purpose::Refer(id) => self.on_refer_answered(id, code, now, out),
            Purpose::CallReinvite(id) => out.push(Output::Event(Event::Notice(format!(
                "call {id}: update refused ({code})"
            )))),
            Purpose::DialOut(id) => out.push(Output::Event(Event::Notice(format!(
                "[{id}] conference command failed ({code})"
            )))),
            Purpose::Schedule(_) => out.push(Output::Event(Event::Notice(format!(
                "meeting request failed ({code})"
            )))),
            Purpose::Park(id) => self.on_park_failed(id, code, out),
            Purpose::Quality => out.push(Output::Event(Event::Notice(format!(
                "quality report not accepted ({code})"
            )))),
            Purpose::Message(id) => out.push(Output::Event(Event::NotDelivered {
                conversation: id,
                status: code,
            })),
            Purpose::Ignore => {}
        }
    }

    fn on_request(&mut self, req: &Message, now: u64, out: &mut Vec<Output>) {
        let method = req.method().unwrap_or("").to_ascii_uppercase();
        // SIP-05: a BENOTIFY is never answered (we do not offer it, but a server may send it).
        if method == "BENOTIFY" {
            if let Some(i) = self
                .subs
                .iter()
                .position(|s| s.dialog.as_ref().is_some_and(|d| d.is_peer_request(req)))
            {
                if !req.body.is_empty() {
                    self.on_document(i, req, now, out);
                }
            } else if let Some(id) = self.roster_of(req) {
                self.on_roster_document(id, req, now, out);
            } else if let Some(id) = self.chat_of(req) {
                self.on_imdn(id, req, out);
            }
            return;
        }
        // SD-51 / RFC 3262 §4, RFC 3261 §8.2.2.3: we send no reliable provisional responses, so
        // an INVITE that requires them is refused with 420 (MS clients never require 100rel).
        if method == "INVITE"
            && req
                .header_values("Require")
                .flat_map(|v| v.split(','))
                .any(|t| t.trim().eq_ignore_ascii_case("100rel"))
        {
            let mut r = response_to(req, 420, "Bad Extension");
            r.push_header("Unsupported", "100rel");
            self.send_plain(r, now, out);
            return;
        }
        if let Some(id) = self.roster_of(req) {
            if method == "NOTIFY" {
                let ok = response_to(req, 200, "OK");
                self.send_plain(ok, now, out);
                self.on_roster_document(id, req, now, out);
                if header_token(req, "subscription-state").as_deref() == Some("terminated") {
                    self.leave_group(id, now, out, true);
                }
            } else {
                let r = response_to(req, 405, "Method Not Allowed");
                self.send_plain(r, now, out);
            }
            return;
        }
        if let Some(id) = self.focus_of(req) {
            self.on_focus_request(id, req, now, out);
            return;
        }
        if let Some(i) = self
            .subs
            .iter()
            .position(|s| s.dialog.as_ref().is_some_and(|d| d.is_peer_request(req)))
        {
            if method == "NOTIFY" {
                self.on_notify(i, req, now, out);
            } else {
                let r = response_to(req, 405, "Method Not Allowed");
                self.send_plain(r, now, out);
            }
            return;
        }
        if let Some(id) = self.park_of(req) {
            self.on_park_request(id, req, now, out);
            return;
        }
        if let Some(id) = self.call_of(req) {
            self.on_call_request(id, req, now, out);
            return;
        }
        let conv = self
            .convs
            .iter()
            .find(|(_, c)| c.dialog.as_ref().is_some_and(|d| d.is_peer_request(req)))
            .map(|(id, _)| *id);
        if let Some(id) = conv {
            self.on_conversation_request(id, req, now, out);
            return;
        }
        let has_to_tag = req
            .to()
            .ok()
            .flatten()
            .is_some_and(|t| t.param("tag").is_some());
        match (method.as_str(), has_to_tag) {
            ("INVITE", false) => self.on_invite(req, now, out),
            ("ACK", _) => {}
            ("CANCEL", false) => self.on_cancel(req, now, out),
            ("OPTIONS", false) => {
                let mut r = response_to(req, 200, "OK");
                r.push_header("Accept", "application/sdp");
                self.send_plain(r, now, out);
            }
            (_, true) => {
                // A request in a dialog we do not know (RFC 3261 §12.2.2).
                let r = response_to(req, 481, "Call/Transaction Does Not Exist");
                self.send_plain(r, now, out);
            }
            _ => {
                let r = response_to(req, 501, "Not Implemented");
                self.send_plain(r, now, out);
            }
        }
    }

    fn roster_of(&self, req: &Message) -> Option<ConversationId> {
        self.convs
            .iter()
            .find(|(_, c)| {
                c.group
                    .as_ref()
                    .and_then(|g| g.roster_dialog.as_ref())
                    .is_some_and(|d| d.is_peer_request(req))
            })
            .map(|(id, _)| *id)
    }

    fn focus_of(&self, req: &Message) -> Option<ConversationId> {
        self.convs
            .iter()
            .find(|(_, c)| {
                c.group
                    .as_ref()
                    .and_then(|g| g.focus_dialog.as_ref())
                    .is_some_and(|d| d.is_peer_request(req))
            })
            .map(|(id, _)| *id)
    }

    /// The group conversation whose IM MCU dialog `req` belongs to; an IMDN BENOTIFY may come
    /// outside the dialog, then the sender (the MCU conference URI) decides (inference, CI-05).
    fn chat_of(&self, req: &Message) -> Option<ConversationId> {
        let from = req
            .from()
            .ok()
            .flatten()
            .map(|f| f.uri.to_ascii_lowercase());
        self.convs
            .iter()
            .find(|(_, c)| {
                c.group.is_some()
                    && c.dialog.as_ref().is_some_and(|d| {
                        d.is_peer_request(req)
                            || from
                                .as_deref()
                                .is_some_and(|f| f == d.remote_uri.to_ascii_lowercase())
                    })
            })
            .map(|(id, _)| *id)
    }

    /// For tests and diagnostics: the request line of a sent message.
    pub fn request_uri(m: &Message) -> Option<&str> {
        match &m.start {
            StartLine::Request { uri, .. } => Some(uri),
            StartLine::Response { .. } => None,
        }
    }
}

mod call;
mod emergency;
mod group;
mod park;
mod qoe;
mod schedule;
mod transfer;

pub use call::{CallId, Caller};
pub use transfer::TransferTarget;

#[cfg(test)]
mod tests;
