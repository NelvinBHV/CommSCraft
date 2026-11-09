//! Sign-in and registration as a sans-I/O state machine: REGISTER with MS-SIPAE NTLM or Kerberos
//! authentication, verification of the signed 200 OK, GRUU and expiry, refresh and sign-out.
//!
//! Evidence: `docs/protocol/evidence/04-siprege-registration-provisioning.md` (RG-03, RG-05,
//! RG-10, RG-20, RG-21, RG-23, RG-28, RG-29), `03-sipae-authentication.md` (AE-20, AE-23, AE-24,
//! AE-28, AE-32, AE-33), `06-sipre-routing.md` (RE-03, RE-06, RE-07, RE-09).
//! Sources: [MS-SIPAE] 20250520 §3.2.3–3.2.5 (pp. 20–28), example §4.1 (pp. 41–43);
//! [MS-SIPREGE] 20250819 §3.1.1 (pp. 74–77), examples §4.1.1–4.1.2 (pp. 94–95); RFC 3261 §10, §17.
//!
//! Security by design (`docs/security.md`):
//! - a registration counts only after the 200 OK verifies against the security association; an
//!   unauthenticated 200 OK is refused, an invalid signature is dropped (AE-32, AE-33);
//! - responses that do not belong to the current transaction (Call-ID, CSeq, From tag, Via branch)
//!   are dropped; every waiting state has a timeout (32 s, RFC 3261 timer F);
//! - the password lives in a wiped, unprintable container and is only used to answer the NTLM
//!   challenge; Kerberos runs in an adapter the caller supplies ([`KerberosProvider`]), so this
//!   crate links no system library; when Kerberos is offered but fails (no ticket, no KDC), the
//!   sign-in falls back to NTLM and says so; TLS-DSK challenges are ignored;
//! - a security association is renewed before it expires: Kerberos at the ticket's end, NTLM
//!   after 8 hours, each minus 5 minutes (AE-35);
//! - all random values come from the caller's CSPRNG ([`Random`]).
//!
//! Server-initiated deregistration and survivable mode NOTIFYs (RG-30 … RG-32) are accepted only
//! inside the registration dialog and only with a valid signature.
//!
//! Not yet covered: the keep-alive negotiation (RG-22); following a preferred registrar (RG-32) is
//! reported to the caller, which decides after its own domain check.

pub mod keepalive;

use commscraft_ntlm::messages as ntlm;
use commscraft_sip_msg::{parse_name_addr, read_quoted, Message};
use commscraft_sipae::{base64, Challenge, Mic, NtlmMic, Policy, Scheme, SecurityAssociation};
use zeroize::Zeroize;

/// Transaction timeout: 64 × T1 = 32 s (RFC 3261 §17.1.2.2, timer F; MS-SIPAE AE-35 keeps the old
/// SA for this long).
pub const TRANSACTION_TIMEOUT: u64 = 32;
/// Lowest expiry a registrar grants (RG-28).
pub const MIN_EXPIRES: u64 = 30;
/// Seconds between 1601-01-01 and 1970-01-01 (FILETIME epoch).
const FILETIME_UNIX_OFFSET: u64 = 11_644_473_600;

/// Longest life of a security association, and the margin before its end (AE-35: NTLM 8 h,
/// Kerberos min(ticket, 8 h), minus a buffer of at least 5 minutes).
pub const SA_MAX_LIFETIME: u64 = 8 * 3600;
pub const SA_RENEW_MARGIN: u64 = 300;

/// A Kerberos security context for the SIP server, as an adapter crate creates it ([MS-SIPAE]
/// §3.2.5.1 step 5, AE-25): the KRB_AP_REQ token for `gssapi-data`, the context's GSS_GetMIC /
/// GSS_VerifyMIC, and how long its ticket lasts.
pub struct KerberosContext {
    pub token: Vec<u8>,
    pub mic: Box<dyn Mic + Send>,
    /// Seconds the ticket stays valid, if known.
    pub lifetime: Option<u64>,
}

/// Creates Kerberos contexts (implemented by an adapter over the system's GSSAPI library).
pub trait KerberosProvider: Send {
    /// A context for the service principal `target` (the challenge's `targetname`, e.g.
    /// `sip/pool.contoso.com`), with the Integrity flag. An error is a reason to use NTLM.
    fn context(&mut self, target: &str) -> Result<KerberosContext, String>;
}

/// How a TLS-DSK sign-in starts ([MS-SIPAE] §3.2.5.1 step 5, AE-26).
pub enum TlsDskStart {
    /// A TLS handshake with the user certificate is ready.
    Handshake(Box<dyn TlsDskHandshake>),
    /// There is no usable user certificate yet: get one from the certificate provisioning
    /// service ([MS-OCAUTHWS], `sts-uri` of the challenge) and sign in again.
    NeedCertificate,
}

/// One step of the TLS handshake tunnelled in `gssapi-data`.
pub enum TlsDskStep {
    /// Send this token (TLS records) in the next REGISTER.
    Send(Vec<u8>),
    /// The handshake is complete: the signing keys and how long the certificate stays valid.
    Done {
        mic: Box<dyn Mic + Send>,
        lifetime: Option<u64>,
    },
}

/// The client side of the TLS-DSK handshake (implemented by an adapter over a TLS library).
pub trait TlsDskHandshake: Send {
    /// The first token: the client_hello.
    fn hello(&mut self) -> Result<Vec<u8>, String>;
    /// The server's token (server_hello … server_hello_done, or change_cipher_spec and finished).
    fn step(&mut self, token: &[u8]) -> Result<TlsDskStep, String>;
}

/// Creates TLS-DSK handshakes for the server `target` (the challenge's `targetname`).
pub trait TlsDskProvider: Send {
    fn start(&mut self, target: &str) -> Result<TlsDskStart, String>;
}

/// Source of cryptographically secure random bytes (the OS CSPRNG in the adapter layer).
pub trait Random {
    fn fill(&mut self, buf: &mut [u8]);
}

/// The user's password; wiped on drop and never printed.
pub struct Password(String);

impl Password {
    pub fn new(p: String) -> Self {
        Self(p)
    }

    /// The password for a second authentication adapter of the same sign-in (Kerberos with a
    /// password when no ticket is cached). The caller keeps the copy in a wiped container.
    pub fn reveal(&self) -> &str {
        &self.0
    }
}

impl Drop for Password {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Password(<redacted>)")
    }
}

/// Static configuration of one endpoint.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address of record, e.g. `sip:alice@contoso.com`.
    pub aor: String,
    /// Request-URI of the REGISTER, e.g. `sip:contoso.com`.
    pub registrar_uri: String,
    /// Persistent endpoint ID (RE-02); `+sip.instance` is derived from it (RE-07).
    pub epid: String,
    /// Local address of the TLS connection for Via and Contact.
    pub local_host: String,
    pub local_port: u16,
    pub user_agent: String,
    /// NTLM user and domain (the account, not the SIP URI).
    pub user: String,
    pub domain: String,
    pub workstation: String,
    /// `ms-subnet` (RG-06), if known.
    pub subnet: Option<String>,
    pub policy: Policy,
    /// Leave out the optional MS-NLMP §3.1.5.2.1 additions (MIC, MsvAvFlags, channel bindings,
    /// target name) and send the LMv2 response, as in the MS-NLMP §4.2.4 example. Still NTLMv2.
    /// Diagnostic switch for interop runs.
    pub ntlm_legacy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Aor,
    RegistrarUri,
    Epid,
    LocalHost,
    UserAgent,
    Subnet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The server rejected the credentials (401/407 after the AUTHENTICATE, or 403; AE-33).
    AuthenticationRejected,
    /// No NTLM challenge with an acceptable version was offered.
    NoSupportedChallenge,
    /// The NTLM challenge was malformed or below the security policy (OB-43).
    Ntlm(ntlm::Error),
    /// The challenge or the security association was refused (e.g. version below 3).
    Auth(commscraft_sipae::Error),
    /// A final response other than success or a challenge.
    Status(u16),
    /// The response was formally valid SIP but violates the protocol (reason).
    BadResponse(&'static str),
    /// No valid response within the transaction timeout.
    Timeout,
    /// On refresh the server reported a new or fixed registration: all dialogs are gone (RG-23).
    RegistrationLost,
    /// The server's client version policy blocks this client: 403 with `Warning` 310 or 311 from
    /// `lcs.microsoft.com` (RG-50).
    VersionBlocked(u16),
}

/// A successful registration (RG-20, RG-21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    /// The GRUU assigned by the registrar (without quotes), if any.
    pub gruu: Option<String>,
    pub expires: u64,
    /// When the refresh has to be sent (shorter than the expiry, RG-21).
    pub refresh_at: u64,
    /// Negotiated keep-alive timeout (CON-34), `None` if the negotiation failed (CON-33).
    pub keepalive_timeout: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Send this message (request, or the answer to a NOTIFY) on the registrar connection.
    Send(Message),
    Registered(Registration),
    Unregistered,
    Failed(Failure),
    /// The registrar removed the binding (deregister NOTIFY, RG-30/RG-31). The registration is over.
    Deregistered(Deregistration),
    /// The presence service is unavailable; the registration stays (survivable mode, RG-31).
    SurvivableMode,
    /// The registrar sent a signed 301/302 to the user's home pool (O-RG-1): close the connection
    /// and register there. The caller asks the user first if `in_user_domain` is false.
    Redirected(RedirectTarget),
    /// Which authentication protocol the sign-in uses, and why another one if a stronger one
    /// was wanted.
    Authentication {
        scheme: Scheme,
        fallback: Option<String>,
    },
    /// TLS-DSK is offered but there is no user certificate: get one from `sts_uri` (an HTTPS URL
    /// of the server, checked by the caller like any web service) and call [`Registrar::start`]
    /// again.
    CertificateNeeded {
        sts_uri: String,
    },
}

/// Target of a registrar redirect (validated, TLS only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectTarget {
    pub host: String,
    pub port: u16,
    pub in_user_domain: bool,
}

/// Reason of a deregister NOTIFY (MS-SIPREGE §2.2.1.13, pp. 18–19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deregistration {
    /// 4140: unregistered by policy (e.g. endpoint quota).
    Unregistered,
    /// 4141: user disabled.
    Rejected,
    /// 4142: service temporarily unavailable (user move).
    Deactivated,
    /// 4165: presence service unavailable, no survivable mode.
    UserServicesUnavailable,
    /// 4166/4167: register with the preferred registrar (validated `sip:` URI, if present; RG-32).
    PreferredRegistrarChange(Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Phase {
    Idle,
    /// Unauthenticated REGISTER sent (AE-20).
    AwaitChallenge,
    /// REGISTER with empty gssapi-data sent; waiting for the NTLM CHALLENGE.
    AwaitNtlmChallenge {
        first: Challenge,
    },
    /// A TLS-DSK token sent; waiting for the server's next TLS records.
    AwaitTlsDsk {
        first: Challenge,
    },
    /// AUTHENTICATE sent; waiting for the signed 200 OK.
    AwaitFinal,
    Registered {
        refresh_at: u64,
    },
    Refreshing,
    Unregistering,
    Done,
}

/// Non-secret facts about the NTLM handshake, for diagnostics of interop runs: offered and chosen
/// flags, the names from the CHALLENGE's target information, the MS-SIPAE version. No challenge
/// nonce, no response, no key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NtlmInfo {
    pub sipae_version: Option<u32>,
    pub offered_flags: u32,
    pub chosen_flags: Option<u32>,
    /// (AV_PAIR name, value) for the name pairs 1 … 5.
    pub names: Vec<(&'static str, String)>,
    pub has_timestamp: bool,
    /// MIC and the other §3.1.5.2.1 additions are sent.
    pub modern: bool,
}

impl NtlmInfo {
    fn new(ch: &ntlm::Challenge, version: Option<u32>, modern: bool) -> Self {
        let names = [
            (1u16, "NetBIOS computer"),
            (2, "NetBIOS domain"),
            (3, "DNS computer"),
            (4, "DNS domain"),
            (5, "DNS forest"),
        ]
        .iter()
        .filter_map(|(id, label)| {
            let v = ch.av(*id)?;
            let units: Vec<u16> = v
                .chunks(2)
                .map(|c| u16::from_le_bytes([*c.first().unwrap_or(&0), *c.get(1).unwrap_or(&0)]))
                .collect();
            Some((*label, String::from_utf16_lossy(&units)))
        })
        .collect();
        Self {
            sipae_version: version,
            offered_flags: ch.flags,
            chosen_flags: None,
            names,
            has_timestamp: ch.av(ntlm::av::TIMESTAMP).is_some(),
            modern,
        }
    }

    /// Flag names (MS-NLMP §2.2.2.5) for display.
    pub fn flag_names(flags: u32) -> Vec<&'static str> {
        use ntlm::flags as f;
        [
            (f::UNICODE, "UNICODE"),
            (f::OEM, "OEM"),
            (f::REQUEST_TARGET, "REQUEST_TARGET"),
            (f::SIGN, "SIGN"),
            (f::SEAL, "SEAL"),
            (f::DATAGRAM, "DATAGRAM"),
            (f::LM_KEY, "LM_KEY"),
            (f::NTLM, "NTLM"),
            (f::ANONYMOUS, "ANONYMOUS"),
            (f::ALWAYS_SIGN, "ALWAYS_SIGN"),
            (f::TARGET_TYPE_DOMAIN, "TARGET_TYPE_DOMAIN"),
            (f::TARGET_TYPE_SERVER, "TARGET_TYPE_SERVER"),
            (f::EXTENDED_SESSIONSECURITY, "EXTENDED_SESSIONSECURITY"),
            (f::IDENTIFY, "IDENTIFY"),
            (f::TARGET_INFO, "TARGET_INFO"),
            (f::VERSION, "VERSION"),
            (f::N128, "128"),
            (f::KEY_EXCH, "KEY_EXCH"),
            (f::N56, "56"),
        ]
        .iter()
        .filter(|(bit, _)| flags & bit != 0)
        .map(|(_, n)| *n)
        .collect()
    }
}

/// The registration state machine of one endpoint.
pub struct Registrar<R: Random> {
    cfg: Config,
    instance: String,
    password: Password,
    rng: R,
    call_id: String,
    from_tag: String,
    cseq: u32,
    branch: String,
    deadline: Option<u64>,
    phase: Phase,
    sa: Option<SecurityAssociation<Box<dyn Mic + Send>>>,
    /// When the security association must be renewed (AE-35).
    sa_renew_at: Option<u64>,
    ntlm_info: Option<NtlmInfo>,
    kerberos: Option<Box<dyn KerberosProvider>>,
    tls_dsk: Option<Box<dyn TlsDskProvider>>,
    /// The TLS-DSK handshake in progress.
    tls_handshake: Option<Box<dyn TlsDskHandshake>>,
    /// TLS-DSK failed in this sign-in: the next challenge is answered with Kerberos or NTLM.
    tls_dsk_failed: Option<String>,
}

impl<R: Random> std::fmt::Debug for Registrar<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registrar")
            .field("aor", &self.cfg.aor)
            .field("phase", &self.phase)
            .field("cseq", &self.cseq)
            .finish_non_exhaustive()
    }
}

fn safe_token(s: &str, extra: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
}

fn validate(cfg: &Config) -> Result<String, ConfigError> {
    let uri_ok = |u: &str| {
        u.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("sip:"))
            && safe_token(u.get(4..).unwrap_or(""), "@.-_+")
    };
    if !uri_ok(&cfg.aor) || !cfg.aor.contains('@') {
        return Err(ConfigError::Aor);
    }
    if !uri_ok(&cfg.registrar_uri) {
        return Err(ConfigError::RegistrarUri);
    }
    if !safe_token(&cfg.local_host, ".-:[]") {
        return Err(ConfigError::LocalHost);
    }
    if !safe_token(&cfg.user_agent, "./-_ ()") {
        return Err(ConfigError::UserAgent);
    }
    if cfg.subnet.as_deref().is_some_and(|s| !safe_token(s, ".:")) {
        return Err(ConfigError::Subnet);
    }
    commscraft_sip_identity::instance_uuid_from_epid(&cfg.epid).map_err(|_| ConfigError::Epid)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Header value without surrounding quotes (a quoted-string parameter as kept by `sip-msg`).
fn unquote(v: &str) -> String {
    match read_quoted(v) {
        Some((s, "")) => s,
        _ => v.to_string(),
    }
}

impl<R: Random> Registrar<R> {
    pub fn new(cfg: Config, password: Password, mut rng: R) -> Result<Self, ConfigError> {
        let instance = validate(&cfg)?;
        let mut call_id = [0u8; 16];
        rng.fill(&mut call_id);
        let mut tag = [0u8; 5];
        rng.fill(&mut tag);
        Ok(Self {
            cfg,
            instance,
            password,
            rng,
            call_id: hex(&call_id),
            from_tag: hex(&tag),
            cseq: 0,
            branch: String::new(),
            deadline: None,
            phase: Phase::Idle,
            sa: None,
            sa_renew_at: None,
            ntlm_info: None,
            kerberos: None,
            tls_dsk: None,
            tls_handshake: None,
            tls_dsk_failed: None,
        })
    }

    /// Signs in with TLS-DSK and the user certificate when the server offers it ([MS-SIPAE]
    /// §3.2.5.1, AE-26/AE-27), before Kerberos and NTLM.
    pub fn with_tls_dsk(mut self, provider: Box<dyn TlsDskProvider>) -> Self {
        self.tls_dsk = Some(provider);
        self
    }

    /// Signs in with Kerberos when the server offers it ([MS-SIPAE] §3.2.5.1, AE-25), else NTLM.
    pub fn with_kerberos(mut self, provider: Box<dyn KerberosProvider>) -> Self {
        self.kerberos = Some(provider);
        self
    }

    /// The `+sip.instance` value derived from the EPID (RE-07).
    pub fn instance(&self) -> &str {
        &self.instance
    }

    /// The AUTHENTICATE_MESSAGE for connection-oriented NTLM over HTTP (MS-NTHT; web services
    /// such as MS-DLX) with the account's credentials, which never leave the registrar.
    /// `negotiate` is the NEGOTIATE_MESSAGE sent first, `challenge` the server's CHALLENGE,
    /// `server_certificate` the DER certificate of the TLS connection the exchange runs on: it
    /// binds the authentication to that connection (channel bindings, issue #42).
    pub fn http_ntlm(
        &mut self,
        negotiate: &[u8],
        challenge: &[u8],
        now: u64,
        server_certificate: Option<&[u8]>,
    ) -> Result<Vec<u8>, Failure> {
        let ch = ntlm::Challenge::parse(challenge).map_err(Failure::Ntlm)?;
        let flags = ntlm::choose_flags_http(ch.flags).map_err(Failure::Ntlm)?;
        let mut client_challenge = [0u8; 8];
        self.rng.fill(&mut client_challenge);
        let mut random_session_key = [0u8; 16];
        self.rng.fill(&mut random_session_key);
        let result = ntlm::authenticate(
            &ch,
            &ntlm::AuthenticateInput {
                user: &self.cfg.user,
                domain: &self.cfg.domain,
                password: &self.password.0,
                workstation: &self.cfg.workstation,
                flags,
                version: [0; 8],
                client_challenge,
                random_session_key,
                now_filetime: now
                    .saturating_add(FILETIME_UNIX_OFFSET)
                    .saturating_mul(10_000_000),
                modern: true,
                negotiate: Some(negotiate),
                channel_bindings: server_certificate
                    .and_then(commscraft_ntlm::channel_binding::for_certificate),
            },
        );
        random_session_key.zeroize();
        result.map(|(token, _key)| token).map_err(Failure::Ntlm)
    }

    /// Random bytes from the registrar's CSPRNG, for tags, Call-IDs and branches of the other
    /// requests of this endpoint.
    pub fn fill_random(&mut self, buf: &mut [u8]) {
        self.rng.fill(buf);
    }

    /// Non-secret facts about the last NTLM handshake (for `--verbose` diagnostics).
    pub fn ntlm_info(&self) -> Option<&NtlmInfo> {
        self.ntlm_info.as_ref()
    }

    pub fn is_registered(&self) -> bool {
        matches!(self.phase, Phase::Registered { .. })
    }

    /// A new REGISTER of this registration dialog: same Call-ID and From tag, CSeq + 1, new branch
    /// (RFC 3261 §10.2.4, RG-10).
    fn register_request(&mut self, now: u64, expires_zero: bool) -> Message {
        self.cseq = self.cseq.saturating_add(1);
        let mut b = [0u8; 8];
        self.rng.fill(&mut b);
        self.branch = format!("z9hG4bK{}", hex(&b));
        self.deadline = Some(now.saturating_add(TRANSACTION_TIMEOUT));
        let c = &self.cfg;
        let mut m = Message::request("REGISTER", &c.registrar_uri);
        m.push_header(
            "Via",
            &format!(
                "SIP/2.0/TLS {}:{};branch={}",
                c.local_host, c.local_port, self.branch
            ),
        );
        m.push_header("Max-Forwards", "70");
        m.push_header(
            "From",
            &format!("<{}>;tag={};epid={}", c.aor, self.from_tag, c.epid),
        );
        m.push_header("To", &format!("<{}>", c.aor));
        m.push_header("Call-ID", &self.call_id);
        m.push_header("CSeq", &format!("{} REGISTER", self.cseq));
        m.push_header(
            "Contact",
            &format!(
                "<sip:{}:{};transport=tls>;proxy=replace;+sip.instance=\"<urn:uuid:{}>\"",
                c.local_host, c.local_port, self.instance
            ),
        );
        m.push_header("User-Agent", &c.user_agent);
        // PR-10: enhanced presence needs msrtc-event-categories (together with gruu-10, RG-27);
        // adhoclist for batch subscriptions (RG-03, example MS-SIPREGE §4.1.1).
        m.push_header("Supported", "gruu-10, adhoclist, msrtc-event-categories");
        // CON-30: hop-by-hop keep-alive negotiation on the REGISTER (MS-CONMGMT §4.1).
        m.push_header("ms-keep-alive", keepalive::REQUEST_VALUE);
        if let Some(s) = &c.subnet {
            m.push_header("ms-subnet", s);
        }
        if expires_zero {
            m.push_header("Expires", "0");
        }
        m
    }

    /// Starts (or restarts) the sign-in with an unauthenticated REGISTER (AE-20).
    pub fn start(&mut self, now: u64) -> Message {
        self.sa = None;
        self.phase = Phase::AwaitChallenge;
        self.register_request(now, false)
    }

    /// Sign-out: a signed REGISTER with `Expires: 0` (RG-29).
    pub fn unregister(&mut self, now: u64) -> Result<Message, Failure> {
        if !self.is_registered() {
            return Err(Failure::BadResponse("not registered"));
        }
        let mut m = self.register_request(now, true);
        self.sign(&mut m)?;
        self.phase = Phase::Unregistering;
        Ok(m)
    }

    /// Signs another request of this endpoint (e.g. the provisioning SUBSCRIBE) with the
    /// established security association (AE-29).
    pub fn sign_request(&mut self, m: &mut Message) -> Result<(), Failure> {
        if !self.is_registered() {
            return Err(Failure::BadResponse("not registered"));
        }
        self.sign(m)
    }

    /// Verifies a message from the registrar outside the REGISTER transaction (AE-32). `false`:
    /// drop it.
    pub fn verify_message(&mut self, m: &Message) -> bool {
        self.is_registered() && self.verified(m).is_some()
    }

    fn sign(&mut self, m: &mut Message) -> Result<(), Failure> {
        let mut r = [0u8; 4];
        self.rng.fill(&mut r);
        let sa = self
            .sa
            .as_mut()
            .ok_or(Failure::BadResponse("no security association"))?;
        sa.sign(m, u32::from_be_bytes(r)).map_err(Failure::Auth)
    }

    /// Timers: refresh before expiry (RG-21) and transaction timeouts.
    pub fn on_tick(&mut self, now: u64) -> Vec<Event> {
        match self.phase {
            // AE-35: the security association ends before the next refresh would be due –
            // authenticate anew instead of refreshing.
            Phase::Registered { refresh_at }
                if now >= refresh_at && self.sa_renew_at.is_some_and(|r| now >= r) =>
            {
                vec![Event::Send(self.start(now))]
            }
            Phase::Registered { refresh_at } if now >= refresh_at => {
                let mut m = self.register_request(now, false);
                match self.sign(&mut m) {
                    Ok(()) => {
                        self.phase = Phase::Refreshing;
                        vec![Event::Send(m)]
                    }
                    Err(f) => self.fail(f),
                }
            }
            Phase::Idle | Phase::Done | Phase::Registered { .. } => Vec::new(),
            _ if self.deadline.is_some_and(|d| now >= d) => self.fail(Failure::Timeout),
            _ => Vec::new(),
        }
    }

    /// RG-50: a 403 with a client version warning ([MS-SIPREGE] §3.4.2.5, pp. 88–92).
    fn version_warning(msg: &Message) -> Option<u16> {
        msg.header_values("Warning").find_map(|v| {
            let mut parts = v.split_whitespace();
            let code: u16 = parts.next()?.parse().ok()?;
            let agent = parts.next()?;
            (matches!(code, 310 | 311) && agent.eq_ignore_ascii_case("lcs.microsoft.com"))
                .then_some(code)
        })
    }

    fn fail(&mut self, f: Failure) -> Vec<Event> {
        self.phase = Phase::Done;
        self.sa = None;
        self.deadline = None;
        vec![Event::Failed(f)]
    }

    /// True if `msg` is a response to the REGISTER transaction in flight.
    fn belongs_to_us(&self, msg: &Message) -> bool {
        let Some(_) = msg.status_code() else {
            return false;
        };
        let ours = || -> Result<bool, commscraft_sip_msg::Error> {
            let call_id = msg.call_id()? == Some(self.call_id.as_str());
            let cseq = msg.cseq()?.is_some_and(|c| {
                c.number == self.cseq && c.method.eq_ignore_ascii_case("REGISTER")
            });
            let tag = msg
                .from()?
                .is_some_and(|f| f.param("tag") == Some(self.from_tag.as_str()));
            let via = msg
                .header_values("Via")
                .next()
                .and_then(|v| v.split(',').next())
                .and_then(|v| v.split(';').find_map(|p| p.trim().strip_prefix("branch=")))
                == Some(self.branch.as_str());
            Ok(call_id && cseq && tag && via)
        };
        ours().unwrap_or(false)
    }

    /// Feeds a received message. Messages that do not belong to the current transaction, and
    /// responses whose signature does not verify, are dropped (no event).
    pub fn on_message(&mut self, msg: &Message, now: u64) -> Vec<Event> {
        if msg
            .method()
            .is_some_and(|m| m.eq_ignore_ascii_case("NOTIFY"))
        {
            return self.on_notify(msg);
        }
        if !self.belongs_to_us(msg) {
            return Vec::new();
        }
        let Some(code) = msg.status_code() else {
            return Vec::new();
        };
        if code < 200 {
            return Vec::new();
        }
        if code == 403 {
            if let Some(w) = Self::version_warning(msg) {
                return self.fail(Failure::VersionBlocked(w));
            }
        }
        match self.phase.clone() {
            Phase::AwaitChallenge => self.on_first_response(msg, code, now),
            Phase::AwaitNtlmChallenge { first } => self.on_ntlm_challenge(msg, code, &first, now),
            Phase::AwaitTlsDsk { first } => self.on_tls_dsk_challenge(msg, code, &first, now),
            Phase::AwaitFinal => self.on_final(msg, code, now),
            Phase::Refreshing => self.on_refresh(msg, code, now),
            Phase::Unregistering => self.on_unregister(msg, code),
            Phase::Idle | Phase::Registered { .. } | Phase::Done => Vec::new(),
        }
    }

    /// Deregister and survivable mode NOTIFY (RG-30, RG-31; MS-SIPREGE §2.2.1.11–2.2.1.13,
    /// §3.2.2.4.1): same Call-ID as the REGISTER, our From tag as To tag, `Event:
    /// registration-notify`, `Content-Type: text/registration-event`, valid MS-SIPAE signature.
    /// Anything else is dropped without an answer.
    fn on_notify(&mut self, msg: &Message) -> Vec<Event> {
        if !matches!(self.phase, Phase::Registered { .. } | Phase::Refreshing) {
            return Vec::new();
        }
        let in_dialog = || -> Result<bool, commscraft_sip_msg::Error> {
            Ok(msg.call_id()? == Some(self.call_id.as_str())
                && msg
                    .to()?
                    .is_some_and(|t| t.param("tag") == Some(self.from_tag.as_str()))
                && msg.from()?.is_some_and(|f| f.param("tag").is_some()))
        };
        let header_is = |name: &'static str, value: &str| {
            msg.single(name).ok().flatten().is_some_and(|v| {
                v.split(';')
                    .next()
                    .is_some_and(|x| x.trim().eq_ignore_ascii_case(value))
            })
        };
        if !in_dialog().unwrap_or(false)
            || !header_is("Event", "registration-notify")
            || !header_is("Content-Type", "text/registration-event")
        {
            return Vec::new();
        }
        if self.verified(msg).is_none() {
            return Vec::new();
        }
        let Ok(body) = std::str::from_utf8(&msg.body) else {
            return Vec::new();
        };
        let event = match body.trim().to_ascii_lowercase().as_str() {
            "registered;event=userservices-unavailable" => Event::SurvivableMode,
            "deregistered;event=unregistered" => Event::Deregistered(Deregistration::Unregistered),
            "deregistered;event=rejected" => Event::Deregistered(Deregistration::Rejected),
            "deregistered;event=deactivated" => Event::Deregistered(Deregistration::Deactivated),
            "deregistered;event=userservices-unavailable" => {
                Event::Deregistered(Deregistration::UserServicesUnavailable)
            }
            "deregistered;event=preferred-registrar-change" => {
                let target = msg
                    .name_addrs("P-Preferred-Registrar")
                    .ok()
                    .and_then(|v| v.into_iter().next())
                    .map(|a| a.uri)
                    .filter(|u| {
                        u.get(..4).is_some_and(|s| s.eq_ignore_ascii_case("sip:"))
                            && safe_token(u.get(4..).unwrap_or(""), "@.-_:;=")
                    });
                Event::Deregistered(Deregistration::PreferredRegistrarChange(target))
            }
            _ => return Vec::new(),
        };
        let mut ok = Message::response(200, "OK");
        for name in ["Via", "From", "To", "Call-ID", "CSeq"] {
            for v in msg.header_values(name) {
                ok.push_header(name, v);
            }
        }
        let mut out = Vec::new();
        if self.sign(&mut ok).is_ok() {
            out.push(Event::Send(ok));
        }
        if matches!(event, Event::Deregistered(_)) {
            self.phase = Phase::Done;
            self.sa = None;
            self.deadline = None;
        }
        out.push(event);
        out
    }

    fn challenges(&self, msg: &Message, code: u16) -> Vec<Challenge> {
        let (header, proxy) = if code == 407 {
            ("Proxy-Authenticate", true)
        } else {
            ("WWW-Authenticate", false)
        };
        msg.header_values(header)
            .filter_map(|v| Challenge::parse(v, proxy).ok().flatten())
            .filter(|c| matches!(c.scheme, Scheme::Ntlm | Scheme::Kerberos | Scheme::TlsDsk))
            .collect()
    }

    /// Authorization during the handshake (AE-28; parameter order as in MS-SIPAE §4.1, p. 42).
    fn auth_header(c: &Challenge, gssapi: &str) -> (&'static str, String) {
        let mut params = Vec::new();
        if let Some(o) = &c.opaque {
            params.push(format!("opaque=\"{o}\""));
        }
        params.push("qop=\"auth\"".to_string());
        params.push(format!("realm=\"{}\"", c.realm));
        params.push(format!("targetname=\"{}\"", c.targetname));
        params.push(format!("gssapi-data=\"{gssapi}\""));
        // §3.2.5.1 steps 6.7–6.8: answer with 3 or 4, never more than the client implements.
        if let Some(n) = c.version {
            params.push(format!(
                "version={}",
                n.min(commscraft_sipae::IMPLEMENTED_VERSION)
            ));
        }
        let name = if c.proxy {
            "Proxy-Authorization"
        } else {
            "Authorization"
        };
        (name, format!("{} {}", c.scheme.name(), params.join(", ")))
    }

    /// The TLS-DSK sign-in, first step: the client_hello in `gssapi-data` (§4.4 step 3, p. 47).
    /// `Err` with the reason: use Kerberos or NTLM.
    fn tls_dsk_hello(&mut self, c: &Challenge, now: u64) -> Result<Vec<Event>, String> {
        let provider = self.tls_dsk.as_mut().ok_or("no TLS-DSK")?;
        let mut handshake = match provider.start(&c.targetname)? {
            TlsDskStart::Handshake(h) => h,
            TlsDskStart::NeedCertificate => {
                let sts_uri = c
                    .sts_uri
                    .clone()
                    .filter(|u| {
                        u.get(..8)
                            .is_some_and(|p| p.eq_ignore_ascii_case("https://"))
                    })
                    .ok_or("no certificate and no certificate provisioning service")?;
                self.phase = Phase::Idle;
                self.deadline = None;
                return Ok(vec![Event::CertificateNeeded { sts_uri }]);
            }
        };
        let hello = handshake.hello()?;
        self.tls_handshake = Some(handshake);
        let mut m = self.register_request(now, false);
        let (name, value) = Self::auth_header(c, &base64::encode(&hello));
        m.push_header(name, &value);
        self.phase = Phase::AwaitTlsDsk { first: c.clone() };
        Ok(vec![
            Event::Authentication {
                scheme: Scheme::TlsDsk,
                fallback: None,
            },
            Event::Send(m),
        ])
    }

    /// The server's TLS records (§4.4 steps 4–7, pp. 48–49): the next token, or – after the
    /// server's finished – the keys and a signed REGISTER without a token (step 8). A TLS failure
    /// starts the sign-in again with Kerberos or NTLM.
    fn on_tls_dsk_challenge(
        &mut self,
        msg: &Message,
        code: u16,
        first: &Challenge,
        now: u64,
    ) -> Vec<Event> {
        if !matches!(code, 401 | 407) {
            self.tls_handshake = None;
            return if (200..300).contains(&code) {
                self.fail(Failure::BadResponse("registration without authentication"))
            } else {
                self.fail(Failure::Status(code))
            };
        }
        let step = self
            .challenges(msg, code)
            .into_iter()
            .find(|c| {
                c.scheme == Scheme::TlsDsk
                    && c.realm == first.realm
                    && c.targetname == first.targetname
                    && c.gssapi_data.as_deref().is_some_and(|g| !g.is_empty())
            })
            .ok_or_else(|| "the server ended the TLS handshake".to_string())
            .and_then(|c| {
                let token = base64::decode(c.gssapi_data.as_deref().unwrap_or(""))
                    .map_err(|_| "gssapi-data is not base64".to_string())?;
                let h = self
                    .tls_handshake
                    .as_mut()
                    .ok_or_else(|| "no TLS handshake".to_string())?;
                Ok((c, h.step(&token)?))
            });
        match step {
            Ok((c, TlsDskStep::Send(token))) => {
                let mut m = self.register_request(now, false);
                let (name, value) = Self::auth_header(&c, &base64::encode(&token));
                m.push_header(name, &value);
                vec![Event::Send(m)]
            }
            Ok((c, TlsDskStep::Done { mic, lifetime })) => {
                self.tls_handshake = None;
                let sa = match SecurityAssociation::new(c.clone(), mic, &self.cfg.policy) {
                    Ok(sa) => sa,
                    Err(e) => return self.fail(Failure::Auth(e)),
                };
                self.sa = Some(sa);
                let lifetime = lifetime.unwrap_or(SA_MAX_LIFETIME).min(SA_MAX_LIFETIME);
                self.sa_renew_at = Some(now + lifetime.saturating_sub(SA_RENEW_MARGIN));
                let mut m = self.register_request(now, false);
                let mut r = [0u8; 4];
                self.rng.fill(&mut r);
                let signed = match self.sa.as_mut() {
                    Some(sa) => sa.tls_dsk_signature(&m, u32::from_be_bytes(r)),
                    None => Err(commscraft_sipae::Error::WrongState),
                };
                let (crand, cnum, response) = match signed {
                    Ok(v) => v,
                    Err(e) => return self.fail(Failure::Auth(e)),
                };
                let mut params = Vec::new();
                if let Some(o) = &c.opaque {
                    params.push(format!("opaque=\"{o}\""));
                }
                params.push("qop=\"auth\"".to_string());
                params.push(format!("realm=\"{}\"", c.realm));
                params.push(format!("targetname=\"{}\"", c.targetname));
                if let Some(v) = c.version {
                    params.push(format!(
                        "version={}",
                        v.min(commscraft_sipae::IMPLEMENTED_VERSION)
                    ));
                }
                params.push(format!(
                    "crand=\"{crand}\", cnum=\"{cnum}\", response=\"{response}\""
                ));
                let header = if c.proxy {
                    "Proxy-Authorization"
                } else {
                    "Authorization"
                };
                m.push_header(header, &format!("TLS-DSK {}", params.join(", ")));
                self.phase = Phase::AwaitFinal;
                vec![Event::Send(m)]
            }
            Err(e) => {
                // AE-26: a failed handshake – start again without TLS-DSK.
                self.tls_handshake = None;
                self.tls_dsk_failed = Some(e);
                vec![Event::Send(self.start(now))]
            }
        }
    }

    /// The Kerberos sign-in: one REGISTER with the KRB_AP_REQ (§4.2 step 3, p. 44), signed from
    /// version 4 on (§4.3 steps 3–4, pp. 45–46). `Err` with the reason: use NTLM.
    fn kerberos_register(&mut self, c: &Challenge, now: u64) -> Result<Vec<Event>, String> {
        let provider = self.kerberos.as_mut().ok_or("no Kerberos")?;
        let ctx = provider.context(&c.targetname)?;
        if ctx.token.is_empty() {
            return Err("no Kerberos token".into());
        }
        let sa = SecurityAssociation::new(c.clone(), ctx.mic, &self.cfg.policy)
            .map_err(|e| format!("{e:?}"))?;
        self.sa = Some(sa);
        let lifetime = ctx.lifetime.unwrap_or(SA_MAX_LIFETIME).min(SA_MAX_LIFETIME);
        self.sa_renew_at = Some(now + lifetime.saturating_sub(SA_RENEW_MARGIN));
        let mut m = self.register_request(now, false);
        let (name, mut value) = Self::auth_header(c, &base64::encode(&ctx.token));
        if c.version.is_some_and(|v| v >= 4) {
            let mut r = [0u8; 4];
            self.rng.fill(&mut r);
            let (crand, cnum, response) = self
                .sa
                .as_mut()
                .ok_or("no security association")?
                .handshake_signature(&m, u32::from_be_bytes(r))
                .map_err(|e| format!("{e:?}"))?;
            value.push_str(&format!(
                ", crand=\"{crand}\", cnum=\"{cnum}\", response=\"{response}\""
            ));
        }
        m.push_header(name, &value);
        self.phase = Phase::AwaitFinal;
        Ok(vec![
            Event::Authentication {
                scheme: Scheme::Kerberos,
                fallback: None,
            },
            Event::Send(m),
        ])
    }

    fn on_first_response(&mut self, msg: &Message, code: u16, now: u64) -> Vec<Event> {
        match code {
            401 | 407 => {
                let challenges = self.challenges(msg, code);
                let tls_dsk = challenges.iter().find(|c| {
                    c.scheme == Scheme::TlsDsk
                        && c.version.is_some_and(|v| v >= self.cfg.policy.min_version)
                });
                let mut tls_fallback = self.tls_dsk_failed.take();
                if self.tls_dsk.is_some() && tls_fallback.is_none() {
                    match tls_dsk.cloned() {
                        Some(t) => match self.tls_dsk_hello(&t, now) {
                            Ok(events) => return events,
                            Err(e) => tls_fallback = Some(e),
                        },
                        None => tls_fallback = Some("the server offers no TLS-DSK".into()),
                    }
                }
                let kerberos = challenges.iter().find(|c| {
                    c.scheme == Scheme::Kerberos
                        && c.version.is_some_and(|v| v >= self.cfg.policy.min_version)
                });
                let mut fallback = tls_fallback.map(|e| format!("TLS-DSK: {e}"));
                if let (Some(k), true) = (kerberos.cloned(), self.kerberos.is_some()) {
                    match self.kerberos_register(&k, now) {
                        Ok(mut events) => {
                            if let (Some(f), Some(Event::Authentication { fallback, .. })) =
                                (fallback, events.first_mut())
                            {
                                *fallback = Some(f);
                            }
                            return events;
                        }
                        Err(e) => {
                            fallback = Some(match fallback {
                                Some(f) => format!("{f}; Kerberos: {e}"),
                                None => e,
                            })
                        }
                    }
                } else if self.kerberos.is_some() {
                    fallback = Some(match fallback {
                        Some(f) => format!("{f}; the server offers no Kerberos"),
                        None => "the server offers no Kerberos".into(),
                    });
                }
                // Reported as "<why> – <protocol used>": name what was not used.
                let fallback = fallback.map(|f| {
                    if f.starts_with("TLS-DSK") {
                        f
                    } else {
                        format!("Kerberos not used ({f})")
                    }
                });
                let Some(c) = challenges.into_iter().find(|c| c.scheme == Scheme::Ntlm) else {
                    return self.fail(Failure::NoSupportedChallenge);
                };
                if c.version.is_none_or(|v| v < self.cfg.policy.min_version) {
                    return self.fail(Failure::Auth(commscraft_sipae::Error::VersionTooLow(
                        c.version,
                    )));
                }
                // AE-24: connectionless NTLM sends an empty first token.
                let mut m = self.register_request(now, false);
                let (name, value) = Self::auth_header(&c, "");
                m.push_header(name, &value);
                self.phase = Phase::AwaitNtlmChallenge { first: c };
                // The protocol is worth reporting only when a stronger one was wanted.
                let mut events = Vec::new();
                if self.kerberos.is_some() || self.tls_dsk.is_some() {
                    events.push(Event::Authentication {
                        scheme: Scheme::Ntlm,
                        fallback,
                    });
                }
                events.push(Event::Send(m));
                events
            }
            200..=299 => self.fail(Failure::BadResponse("registration without authentication")),
            _ => self.fail(Failure::Status(code)),
        }
    }

    fn on_ntlm_challenge(
        &mut self,
        msg: &Message,
        code: u16,
        first: &Challenge,
        now: u64,
    ) -> Vec<Event> {
        if !matches!(code, 401 | 407) {
            return if (200..300).contains(&code) {
                self.fail(Failure::BadResponse("registration without authentication"))
            } else {
                self.fail(Failure::Status(code))
            };
        }
        let Some(c) = self.challenges(msg, code).into_iter().find(|c| {
            c.scheme == Scheme::Ntlm
                && c.realm == first.realm
                && c.targetname == first.targetname
                && c.gssapi_data.as_deref().is_some_and(|g| !g.is_empty())
        }) else {
            return self.fail(Failure::NoSupportedChallenge);
        };
        let Ok(raw) = base64::decode(c.gssapi_data.as_deref().unwrap_or("")) else {
            return self.fail(Failure::BadResponse("gssapi-data is not base64"));
        };
        let ch = match ntlm::Challenge::parse(&raw) {
            Ok(ch) => ch,
            Err(e) => return self.fail(Failure::Ntlm(e)),
        };
        self.ntlm_info = Some(NtlmInfo::new(&ch, c.version, !self.cfg.ntlm_legacy));
        let flags = match ntlm::choose_flags(ch.flags) {
            Ok(f) => f,
            Err(e) => return self.fail(Failure::Ntlm(e)),
        };
        if let Some(i) = &mut self.ntlm_info {
            i.chosen_flags = Some(flags);
        }
        let mut client_challenge = [0u8; 8];
        self.rng.fill(&mut client_challenge);
        let mut random_session_key = [0u8; 16];
        self.rng.fill(&mut random_session_key);
        let result = ntlm::authenticate(
            &ch,
            &ntlm::AuthenticateInput {
                user: &self.cfg.user,
                domain: &self.cfg.domain,
                password: &self.password.0,
                workstation: &self.cfg.workstation,
                flags,
                version: [0; 8],
                client_challenge,
                random_session_key,
                now_filetime: now
                    .saturating_add(FILETIME_UNIX_OFFSET)
                    .saturating_mul(10_000_000),
                modern: !self.cfg.ntlm_legacy,
                negotiate: None,
                // [MS-SIPAE] defines no channel bindings for SIP: Z(16).
                channel_bindings: None,
            },
        );
        random_session_key.zeroize();
        let (token, key) = match result {
            Ok(v) => v,
            Err(e) => return self.fail(Failure::Ntlm(e)),
        };
        let mic: Box<dyn Mic + Send> = Box::new(NtlmMic::client(key.as_bytes()));
        let sa = match SecurityAssociation::new(c.clone(), mic, &self.cfg.policy) {
            Ok(sa) => sa,
            Err(e) => return self.fail(Failure::Auth(e)),
        };
        drop(key);
        self.sa = Some(sa);
        self.sa_renew_at = Some(now + SA_MAX_LIFETIME - SA_RENEW_MARGIN);
        let mut m = self.register_request(now, false);
        let (name, mut value) = Self::auth_header(&c, &base64::encode(&token));
        // Version 4: this request is already signed (MS-SIPAE §3.2.5.1 step 6.8, p. 26; AE-06).
        if c.version.is_some_and(|v| v >= 4) {
            let mut r = [0u8; 4];
            self.rng.fill(&mut r);
            let signed = match self.sa.as_mut() {
                Some(sa) => sa.handshake_signature(&m, u32::from_be_bytes(r)),
                None => Err(commscraft_sipae::Error::WrongState),
            };
            match signed {
                Ok((crand, cnum, response)) => {
                    value.push_str(&format!(
                        ", crand=\"{crand}\", cnum=\"{cnum}\", response=\"{response}\""
                    ));
                }
                Err(e) => return self.fail(Failure::Auth(e)),
            }
        }
        m.push_header(name, &value);
        self.phase = Phase::AwaitFinal;
        vec![Event::Send(m)]
    }

    /// Verifies a 2xx with the SA (AE-32). `Ok(None)`: drop the message.
    fn verified(&mut self, msg: &Message) -> Option<()> {
        self.sa.as_mut()?.verify(msg).ok().map(|_| ())
    }

    fn registration(&self, msg: &Message, now: u64) -> Result<Registration, Failure> {
        let contacts = msg
            .name_addrs("Contact")
            .map_err(|_| Failure::BadResponse("malformed Contact"))?;
        let ours = contacts.iter().find(|c| {
            c.param("+sip.instance").is_some_and(|v| {
                let v = unquote(v);
                v.trim_start_matches('<')
                    .trim_end_matches('>')
                    .strip_prefix("urn:uuid:")
                    .is_some_and(|u| {
                        u.replace('-', "")
                            .eq_ignore_ascii_case(&self.instance.replace('-', ""))
                    })
            })
        });
        let Some(contact) = ours else {
            return Err(Failure::BadResponse("own Contact missing"));
        };
        // The Expires header is covered by the MS-SIPAE signature (field 15), the Contact is not
        // (§3.2.4.1 step 2): prefer the signed value (security by design, inference).
        let signed = msg
            .expires()
            .map_err(|_| Failure::BadResponse("several Expires"))?;
        let expires_text = match (signed, contact.param("expires")) {
            (Some(e), _) | (None, Some(e)) => e.to_string(),
            (None, None) => return Err(Failure::BadResponse("no expiry")),
        };
        let expires: u64 = expires_text
            .parse()
            .ok()
            .filter(|e| *e >= MIN_EXPIRES && *e <= 86_400 * 7)
            .ok_or(Failure::BadResponse("expiry out of range"))?;
        let gruu = contact.param("gruu").map(unquote);
        if let Some(g) = &gruu {
            // RE-09/RE-10: a GRUU is a SIP URI ending in the gruu parameter.
            if parse_name_addr(g).is_err() || !g.to_ascii_lowercase().starts_with("sip:") {
                return Err(Failure::BadResponse("malformed GRUU"));
            }
        }
        Ok(Registration {
            gruu,
            expires,
            // Inference: refresh after 4/5 of the expiry ("shorter", RG-21).
            refresh_at: now.saturating_add(expires * 4 / 5),
            keepalive_timeout: keepalive::negotiated_timeout(msg),
        })
    }

    fn registered(&mut self, msg: &Message, now: u64) -> Vec<Event> {
        match self.registration(msg, now) {
            Ok(r) => {
                self.phase = Phase::Registered {
                    refresh_at: r.refresh_at,
                };
                self.deadline = None;
                vec![Event::Registered(r)]
            }
            Err(f) => self.fail(f),
        }
    }

    /// O-RG-1 (secondary evidence: SIPE `sip-transport.c` l. 1154–1196; RFC 3261 §8.1.3.4,
    /// §21.3.2): after the authentication handshake the registrar can answer 301 with the home
    /// pool in the Contact. Accepted only when signed; host checked; only TLS.
    fn on_redirect(&mut self, msg: &Message) -> Vec<Event> {
        if self.verified(msg).is_none() {
            return Vec::new();
        }
        let target = msg
            .name_addrs("Contact")
            .ok()
            .and_then(|c| c.into_iter().next())
            .and_then(|c| {
                let rest = c
                    .uri
                    .get(..4)
                    .filter(|s| s.eq_ignore_ascii_case("sip:"))
                    .and(c.uri.get(4..))?;
                let mut parts = rest.split(';');
                let hostport = parts.next()?;
                if hostport.contains('@') {
                    return None;
                }
                let tls = parts.all(|p| match p.split_once('=') {
                    Some((k, v)) if k.eq_ignore_ascii_case("transport") => {
                        v.eq_ignore_ascii_case("tls")
                    }
                    _ => true,
                });
                if !tls {
                    return None;
                }
                let (host, port) = match hostport.rsplit_once(':') {
                    Some((h, p)) => (h, p.parse::<u16>().ok().filter(|p| *p != 0)?),
                    None => (hostport, 5061),
                };
                let host = commscraft_url::dns_host(host)?;
                let domain = self.cfg.aor.rsplit_once('@').map(|(_, d)| d.to_string())?;
                Some(RedirectTarget {
                    in_user_domain: commscraft_url::in_domain(&host, &domain),
                    host,
                    port,
                })
            });
        match target {
            Some(t) => {
                self.phase = Phase::Done;
                self.sa = None;
                self.deadline = None;
                vec![Event::Redirected(t)]
            }
            None => self.fail(Failure::BadResponse("unusable redirect")),
        }
    }

    fn on_final(&mut self, msg: &Message, code: u16, now: u64) -> Vec<Event> {
        match code {
            301 | 302 => self.on_redirect(msg),
            200..=299 => {
                if self.verified(msg).is_none() {
                    return Vec::new();
                }
                self.registered(msg, now)
            }
            401 | 407 | 403 => self.fail(Failure::AuthenticationRejected),
            _ => self.fail(Failure::Status(code)),
        }
    }

    fn on_refresh(&mut self, msg: &Message, code: u16, now: u64) -> Vec<Event> {
        match code {
            200..=299 => {
                if self.verified(msg).is_none() {
                    return Vec::new();
                }
                // RG-23: "added" or "fixed" on a refresh means the registration was lost.
                let action = msg.header_values("presence-state").find_map(|v| {
                    v.split(';').find_map(|p| {
                        p.trim()
                            .strip_prefix("register-action=")
                            .map(|a| a.trim_matches('"').to_ascii_lowercase())
                    })
                });
                if matches!(action.as_deref(), Some("added" | "fixed")) {
                    return self.fail(Failure::RegistrationLost);
                }
                self.registered(msg, now)
            }
            301 | 302 => self.on_redirect(msg),
            // AE-23: a signed request challenged again ⇒ discard the SA and authenticate anew.
            401 | 407 => vec![Event::Send(self.start(now))],
            _ => self.fail(Failure::Status(code)),
        }
    }

    fn on_unregister(&mut self, msg: &Message, code: u16) -> Vec<Event> {
        match code {
            200..=299 => {
                if self.verified(msg).is_none() {
                    return Vec::new();
                }
                self.phase = Phase::Done;
                self.sa = None;
                self.deadline = None;
                vec![Event::Unregistered]
            }
            _ => self.fail(Failure::Status(code)),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub mod testing;
#[cfg(test)]
mod tests;
