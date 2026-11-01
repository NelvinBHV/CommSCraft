//! `commscraft-cli`: command-line tool for the owner's interop runs.
//!
//! `sign-in` connects to a Skype for Business server, registers with NTLM, fetches in-band
//! provisioning, prints a summary and signs out again. It contacts a server only when started with
//! `--authorized` (rule T-03: interop runs only by the owner, on authorized systems).
//!
//! Security by design (`docs/security.md`): the password is read without echo and kept in a wiped
//! container; verbose output redacts every authentication header; TLS is always verified (system
//! trust store plus an optional enterprise CA file); only TLS targets are used.

// Issue #42: these shadow the standard macros for the whole crate (textual scope comes before the
// prelude), so nothing reaches the terminal without `term::clean`.
macro_rules! println {
    () => { ::std::println!() };
    ($($t:tt)*) => { ::std::println!("{}", $crate::term::clean(&format!($($t)*))) };
}
macro_rules! eprintln {
    () => { ::std::eprintln!() };
    ($($t:tt)*) => { ::std::eprintln!("{}", $crate::term::clean(&format!($($t)*))) };
}
#[allow(unused_macros)]
macro_rules! print {
    ($($t:tt)*) => { ::std::print!("{}", $crate::term::clean(&format!($($t)*))) };
}
macro_rules! eprint {
    ($($t:tt)*) => { ::std::eprint!("{}", $crate::term::clean(&format!($($t)*))) };
}

mod calls;
mod term;
mod video_call;

use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use commscraft_net::{OsRandom, SipConnection, TlsClient};
use commscraft_provisioning as prov;
use commscraft_register::keepalive::KeepAlive;
use commscraft_register::{Config, Event, Password, Random, Registrar};
use commscraft_sip_msg::dialog::Dialog;
use commscraft_sip_msg::Message;
use commscraft_sipae::Policy;

const USAGE: &str =
    "usage: commscraft-cli sign-in|presence|im|interactive --aor sip:USER@DOMAIN --user NTLM_USER --domain NTLM_DOMAIN \
[--server HOST[:PORT]] [--ca-file PEM] [--epid HEX] [--verbose] [--discover-only] [--trust-domain DOMAIN] [--ntlm-legacy] \
[--set-status online|busy|dnd|away] [--max-contacts N] [--to sip:URI --text TEXT [--wait S]] --authorized
       commscraft-cli audio-check [--device ALSA_DEVICE] [--codec g722|pcma]   (local, no network)

sign-in   signs in once, fetches provisioning, prints a summary and signs out.
presence  signs in, reads the contact list, queries the presence of up to N contacts (default 50)
          once, optionally sets the own state for the time of the session (--set-status), signs out.
interactive  signs in and stays signed in: contacts with live presence, own state, several
          conversations. Commands: /contacts  /status online|busy|dnd|away  /chat sip:URI
          /call sip:URI|+491234…|0401234…  /callback N  /transfer N sip:URI|number  /transfer N M (consult)
          /follow N | /decline N (a transfer to a number or another domain)
          /videocall sip:URI (audio and video, camera COMMSCRAFT_CAMERA or /dev/video0)  /answervideo [N]
          /delegate add|remove sip:URI (delegates)  /callfor sip:BOSS sip:URI|number (call as delegate)
          /team sip:A [sip:B …] [SECONDS] | off   /delegates sip:A [SECONDS] [skip] | off (team and delegate ringing)
          /location [lookup | city COUNTRY;STATE;CITY | pick N | COUNTRY;STATE;CITY;STREET;NUMBER;POSTCODE;ROOM;NAME] (emergency location)
          /park [N] (call park; retrieve with /call ORBIT)  /rate N 1-5 [text] (call rating)  /forward off|voicemail|TARGET|noanswer TARGET [SECONDS]  /simring TARGET|off  /answer [N]  /hangup [N]  /dtmf 123#  /mute  /unmute  /hold  /resume (audio calls)
          /group sip:A sip:B … (group conversation)  /invite sip:URI (into the current group)
          /join N | sip:…focus… | https://meeting-link (join a meeting; lobby is shown)
          /schedule MINUTES [OPTIONS] SUBJECT  /schedule static  /meetings [static]  /meeting ID  /cancelmeeting ID [static]
          /changemeeting ID MINUTES [OPTIONS] SUBJECT – OPTIONS: open|closed|anonymous, promote none|company|everyone,
            phone bypass|lobby, locked, presenter sip:URI, attendee sip:URI
          /confvideo [LANGUAGE] (the meeting's audio and video)  /confaudio [LANGUAGE] (join the meeting's audio; announcements in LANGUAGE, default from $LANG)
          /announcements on|off  /dialout NUMBER (presenters: the meeting calls a phone)  /confmute  /confunmute (mute in the meeting)
          presenters: /admit [sip:URI]  /deny sip:URI  /promote sip:URI  /demote sip:URI  /lock  /unlock
          /remove sip:URI  /muteuser sip:URI  /unmuteuser sip:URI  /muteall  /unmuteall
          /options [open|closed|anonymous] [promote none|company|everyone] [phone bypass|lobby]
          /search TEXT (address book)  /to N (switch conversation)  /end [N]  /quit  – any other line is sent as text.
im        signs in, opens a 1:1 conversation with --to, sends --text, shows answers and typing for
          --wait seconds (default 20), ends the conversation and signs out. Plain text only.

--authorized  confirms that you are allowed to test against this server (required).
--server      SIP server; without it the server is discovered: Lync Autodiscover over HTTPS
              (lyncdiscoverinternal, lyncdiscover), then DNS SRV (_sipinternaltls._tcp, _sip._tls),
              then sipinternal/sip/sipexternal.<domain>:443 (MS-OCDISCWS, MS-CONMGMT).
--discover-only  run only the discovery and print the result; no password, no sign-in.
--trust-domain   trust SIP servers in DOMAIN although it differs from your SIP domain (like
              TrustModelData of the Windows client); only for servers named by Autodiscover.
              A yes on the interactive question is remembered in ~/.config/commscraft/trusted-domains.
--ntlm-legacy    diagnostic: NTLMv2 without the optional MIC/channel-binding additions, with LMv2.
--verbose     also prints the non-secret NTLM facts (offered/chosen flags, domain names).
The password is read from the terminal without echo, or as the first line of standard input.";

struct Args {
    aor: String,
    user: String,
    domain: String,
    server: Option<(String, u16)>,
    ca_file: Option<PathBuf>,
    epid: Option<String>,
    verbose: bool,
    authorized: bool,
    discover_only: bool,
    trust_domains: Vec<String>,
    ntlm_legacy: bool,
    command: Command,
    set_status: Option<commscraft_presence::UserStatus>,
    max_contacts: usize,
    to: Option<String>,
    text: Option<String>,
    wait: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    SignIn,
    Presence,
    Im,
    Interactive,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let command = match it.next().as_deref() {
        Some("sign-in") => Command::SignIn,
        Some("presence") => Command::Presence,
        Some("im") => Command::Im,
        Some("interactive") => Command::Interactive,
        _ => return Err(USAGE.into()),
    };
    let mut a = Args {
        aor: String::new(),
        user: String::new(),
        domain: String::new(),
        server: None,
        ca_file: None,
        epid: None,
        verbose: false,
        discover_only: false,
        trust_domains: Vec::new(),
        ntlm_legacy: false,
        command,
        set_status: None,
        max_contacts: 50,
        to: None,
        text: None,
        wait: 20,
        authorized: false,
    };
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--aor" => a.aor = value()?,
            "--user" => a.user = value()?,
            "--domain" => a.domain = value()?,
            "--server" => {
                let v = value()?;
                let (h, p) = match v.rsplit_once(':') {
                    Some((h, p)) => (
                        h.to_string(),
                        p.parse().map_err(|_| "bad port".to_string())?,
                    ),
                    None => (v, 5061),
                };
                a.server = Some((h, p));
            }
            "--ca-file" => a.ca_file = Some(PathBuf::from(value()?)),
            "--epid" => a.epid = Some(value()?),
            "--verbose" => a.verbose = true,
            "--discover-only" => a.discover_only = true,
            "--ntlm-legacy" => a.ntlm_legacy = true,
            "--set-status" => {
                let v = value()?;
                a.set_status = Some(
                    commscraft_presence::UserStatus::parse(&v)
                        .ok_or("--set-status: online, busy, dnd or away")?,
                );
            }
            "--to" => a.to = Some(value()?),
            "--text" => a.text = Some(value()?),
            "--wait" => a.wait = value()?.parse().map_err(|_| "--wait needs seconds")?,
            "--max-contacts" => {
                a.max_contacts = value()?
                    .parse()
                    .map_err(|_| "--max-contacts needs a number")?
            }
            "--trust-domain" => a.trust_domains.push(value()?),
            "--authorized" => a.authorized = true,
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
        }
    }
    if a.aor.is_empty() || (!a.discover_only && (a.user.is_empty() || a.domain.is_empty())) {
        return Err(USAGE.into());
    }
    Ok(a)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Switches terminal echo back on when dropped – on every return path and on a panic (issue
/// #42). A signal such as Ctrl+C ends the process without it; `stty echo` or `reset` repairs the
/// terminal then (documented residual risk: handling signals would need `unsafe` or another
/// dependency).
struct EchoOff;

impl Drop for EchoOff {
    fn drop(&mut self) {
        let _ = std::process::Command::new("stty").arg("echo").status();
    }
}

/// Reads the password without echo (via `stty`), or the first line of a non-terminal stdin.
fn read_password() -> Result<Password, String> {
    let stdin = std::io::stdin();
    let tty = stdin.is_terminal();
    let mut echo_off = None;
    if tty {
        eprint!("Password: ");
        let _ = std::io::stderr().flush();
        let ok = std::process::Command::new("stty")
            .arg("-echo")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        // Restore even if `stty -echo` reported a failure after partly applying it.
        echo_off = Some(EchoOff);
        if !ok {
            return Err(
                "cannot switch off terminal echo; pipe the password on standard input instead"
                    .into(),
            );
        }
    }
    let mut line = String::new();
    let read = stdin.lock().read_line(&mut line);
    if echo_off.take().is_some() {
        eprintln!();
    }
    read.map_err(|e| e.to_string())?;
    let pw = line.trim_end_matches(['\r', '\n']).to_string();
    zeroize::Zeroize::zeroize(&mut line);
    if pw.is_empty() {
        return Err("empty password".into());
    }
    Ok(Password::new(pw))
}

/// Persistent EPID (RE-02): 64 random bits, stored with mode 0600 under the user's config dir.
fn load_or_create_epid(rng: &mut OsRandom) -> Result<String, String> {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok_or("no HOME")?;
    let dir = base.join("commscraft");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)
        .map_err(|e| e.to_string())?;
    let path = dir.join("epid");
    if let Ok(s) = std::fs::read_to_string(&path) {
        let s = s.trim().to_string();
        if !s.is_empty() && s.len() <= 16 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(s);
        }
    }
    let mut b = [0u8; 8];
    rng.fill(&mut b);
    let epid: String = b.iter().map(|x| format!("{x:02x}")).collect();
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| e.to_string())?;
    f.write_all(epid.as_bytes()).map_err(|e| e.to_string())?;
    Ok(epid)
}

/// Keeps the non-secret parameters of an authentication header (realm, targetname, version,
/// opaque, qop, cnum, snum, …) and replaces the tokens and signatures.
fn redact_auth(value: &str) -> String {
    const SECRET_PARAMS: [&str; 4] = ["gssapi-data", "response", "rspauth", "nonce"];
    match commscraft_sipae::parse_auth_params(value) {
        Ok((scheme, params)) => {
            let shown: Vec<String> = params
                .iter()
                .map(|(k, v)| {
                    if SECRET_PARAMS.contains(&k.as_str()) {
                        format!("{k}=<{} chars redacted>", v.len())
                    } else {
                        format!("{k}=\"{v}\"")
                    }
                })
                .collect();
            format!("{scheme} {}", shown.join(", "))
        }
        Err(_) => format!(
            "{} <redacted>",
            value.split_whitespace().next().unwrap_or("")
        ),
    }
}

fn print_ntlm_info(i: &commscraft_register::NtlmInfo) {
    use commscraft_register::NtlmInfo;
    eprintln!("ntlm: MS-SIPAE version {:?}", i.sipae_version);
    eprintln!(
        "ntlm: server offers {}",
        NtlmInfo::flag_names(i.offered_flags).join(" ")
    );
    if let Some(c) = i.chosen_flags {
        eprintln!("ntlm: client chose  {}", NtlmInfo::flag_names(c).join(" "));
    }
    for (k, v) in &i.names {
        eprintln!("ntlm: {k}: {v}");
    }
    eprintln!(
        "ntlm: timestamp in challenge: {}; MIC sent: {}; channel bindings: none (SIP; MS-SIPAE defines none)",
        i.has_timestamp,
        i.modern && i.has_timestamp
    );
}

/// Prints a message with all authentication data replaced.
fn show(prefix: &str, m: &Message) {
    const SECRET: [&str; 6] = [
        "Authorization",
        "Proxy-Authorization",
        "Authentication-Info",
        "Proxy-Authentication-Info",
        "WWW-Authenticate",
        "Proxy-Authenticate",
    ];
    let mut copy = m.clone();
    for h in &mut copy.headers {
        if SECRET.iter().any(|s| h.name.eq_ignore_ascii_case(s)) {
            h.value = redact_auth(&h.value);
        }
    }
    if !copy.body.is_empty() {
        copy.body = format!("<{} bytes body>", m.body.len()).into_bytes();
    }
    match copy.to_bytes() {
        Ok(b) => {
            // The serializer writes the length of the placeholder; show the real one.
            let text = String::from_utf8_lossy(&b).replace(
                &format!("Content-Length: {}\r\n", copy.body.len()),
                &format!("Content-Length: {}\r\n", m.body.len()),
            );
            eprintln!("{prefix}\n{text}")
        }
        Err(_) => eprintln!("{prefix} <unprintable message>"),
    }
}

fn hex(rng: &mut OsRandom, n: usize) -> String {
    let mut b = vec![0u8; n];
    rng.fill(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn print_provisioning(p: &prov::Provisioning, user_domain: &str) {
    println!("provisioning: {} groups", p.groups.len());
    if let Some(g) = p.group("ServerConfiguration") {
        let c = prov::ServerConfiguration::from_group(g, user_domain);
        let url = |name: &str, u: &Option<prov::ServiceUrl>| {
            if let Some(u) = u {
                let note = if u.in_user_domain {
                    ""
                } else {
                    "  (outside your domain)"
                };
                println!("  {name}: {}{note}", u.url);
            }
        };
        url("address book (internal)", &c.abs_internal);
        url("address book (external)", &c.abs_external);
        url("group expansion (internal)", &c.dlx_internal);
        url("group expansion (external)", &c.dlx_external);
        url("location service", &c.lis_internal);
        for (name, v) in [
            ("MRAS", &c.mras_uri),
            ("QoE", &c.qos_uri),
            ("focus factory", &c.focus_factory_uri),
            ("voice mail", &c.voice_mail_uri),
        ] {
            if let Some(v) = v {
                println!("  {name}: {v}");
            }
        }
        if let Some(e) = c.av_encryption {
            println!("  A/V encryption: {e:?}");
        }
        if !c.rejected.is_empty() {
            println!("  rejected (invalid) values: {}", c.rejected.join(", "));
        }
    }
}

/// Server discovery (OB-06): Autodiscover over HTTPS first (D-10), then DNS SRV and the fallback
/// host names. Targets outside the user's SIP domain are not used automatically.
/// Server domains the user trusts for this SIP domain (like `TrustModelData` of the Windows client,
/// Microsoft KB 2531068): from `--trust-domain` and from `~/.config/commscraft/trusted-domains`.
struct Trust {
    domains: Vec<String>,
    file: Option<PathBuf>,
}

fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|b| b.join("commscraft"))
}

impl Trust {
    fn load(extra: &[String]) -> Self {
        let file = config_dir().map(|d| d.join("trusted-domains"));
        let mut domains: Vec<String> = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .map(|s| s.lines().filter_map(commscraft_url::dns_host).collect())
            .unwrap_or_default();
        domains.extend(extra.iter().filter_map(|d| commscraft_url::dns_host(d)));
        Self { domains, file }
    }

    fn trusts(&self, host: &str) -> bool {
        self.domains
            .iter()
            .any(|d| commscraft_url::in_domain(host, d))
    }

    /// Asks on the terminal (never when stdin is not a terminal) and remembers a yes.
    fn ask(&mut self, host: &str, user_domain: &str, named_by: &str) -> bool {
        // Offer the parent domain of the host (sip.hoster.example -> hoster.example).
        let domain = host
            .split_once('.')
            .map(|(_, d)| d)
            .filter(|d| d.contains('.'))
            .unwrap_or(host)
            .to_string();
        if !std::io::stdin().is_terminal() {
            eprintln!("  {host} is outside your SIP domain {user_domain}; to trust it, add --trust-domain {domain}");
            return false;
        }
        eprint!(
            "The server {host} is not in your SIP domain {user_domain}.\n\
             It was named by {named_by} over a verified HTTPS connection.\n\
             Trust servers in {domain} for signing in (your password goes there)? [y/N] "
        );
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        if std::io::stdin().lock().read_line(&mut line).is_err()
            || !line.trim().eq_ignore_ascii_case("y")
        {
            return false;
        }
        self.domains.push(domain.clone());
        if let Some(f) = &self.file {
            use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
            if let Some(dir) = f.parent() {
                let _ = std::fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(dir);
            }
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .mode(0o600)
                .open(f)
            {
                let _ = writeln!(file, "{domain}");
                eprintln!("  remembered in {}", f.display());
            }
        }
        true
    }
}

fn discover(
    tls: &TlsClient,
    aor: &str,
    user_domain: &str,
    verbose: bool,
    trust: &mut Trust,
) -> Result<Vec<(String, u16)>, String> {
    use commscraft_discovery as d;
    let ua = format!("CommSCraft/{}", env!("CARGO_PKG_VERSION"));
    let timeout = Duration::from_secs(10);
    let mut ad = d::Autodiscover::new(&d::Config {
        aor: aor.to_string(),
        allow_plain_http: false,
    })
    .map_err(|e| format!("discovery: {e:?}"))?;
    let mut step = ad.start();
    let mut outcome = None;
    loop {
        match step {
            d::Step::Fetch(url) => {
                eprintln!("autodiscover: GET {url}");
                let response = match commscraft_url::parse_https(&url) {
                    None => d::Response::Failed,
                    Some(u) => match commscraft_net::http::get(tls, &u, d::ACCEPT, &ua, timeout) {
                        Err(e) => {
                            eprintln!("  {e}");
                            d::Response::Failed
                        }
                        Ok(r) => {
                            let header = |n: &str| r.header(n).ok().flatten().map(str::to_string);
                            // A relative Location is resolved against the requested URL.
                            let location = header("location").map(|l| {
                                if l.starts_with('/') {
                                    let port = if u.port == 443 {
                                        String::new()
                                    } else {
                                        format!(":{}", u.port)
                                    };
                                    format!("https://{}{port}{l}", u.host)
                                } else {
                                    l
                                }
                            });
                            eprintln!("  HTTP {}", r.status);
                            match String::from_utf8(r.body.clone()) {
                                Ok(body) => {
                                    if verbose && r.status == 200 {
                                        eprintln!("{}", term::quoted(&body));
                                    }
                                    d::Response::Http {
                                        status: r.status,
                                        location,
                                        content_type: header("content-type"),
                                        body,
                                    }
                                }
                                Err(_) => d::Response::Failed,
                            }
                        }
                    },
                };
                step = ad.on_response(response);
            }
            d::Step::Done(o) => {
                eprintln!("autodiscover: access location {:?}", o.access_location);
                for t in &o.targets {
                    eprintln!(
                        "  SIP access point {}:{}{}",
                        t.host,
                        t.port,
                        if t.in_user_domain {
                            ""
                        } else if trust.trusts(&t.host) {
                            " (outside your domain, trusted)"
                        } else {
                            " (outside your domain)"
                        }
                    );
                }
                if o.targets.is_empty() {
                    eprintln!(
                        "  no SIP access point in the Autodiscover answer (interop test IT-16)"
                    );
                }
                outcome = Some(o);
                break;
            }
            d::Step::Failed(e) => {
                eprintln!("autodiscover: {e:?}");
                break;
            }
        }
    }
    // DNS SRV (CON-11, TLS names only).
    let servers = std::fs::read_to_string("/etc/resolv.conf")
        .map(|c| commscraft_net::dns::nameservers(&c))
        .unwrap_or_default();
    let mut rng = OsRandom::new().map_err(|e| e.to_string())?;
    let mut lookup = |name: &str| -> Vec<d::SrvRecord> {
        let mut id = [0u8; 2];
        rng.fill(&mut id);
        match commscraft_net::dns::lookup_srv(
            &servers,
            name,
            u16::from_be_bytes(id),
            Duration::from_secs(5),
        ) {
            Ok(r) => {
                eprintln!("dns: {name}: {} record(s)", r.len());
                for s in &r {
                    eprintln!(
                        "  priority {} weight {} -> {}:{}",
                        s.priority, s.weight, s.target, s.port
                    );
                }
                r.into_iter()
                    .map(|s| d::SrvRecord {
                        priority: s.priority,
                        weight: s.weight,
                        port: s.port,
                        target: s.target,
                    })
                    .collect()
            }
            Err(e) => {
                eprintln!("dns: {name}: {e}");
                Vec::new()
            }
        }
    };
    let [internal_name, external_name] = d::srv_names(user_domain);
    let internal = lookup(&internal_name);
    let external = lookup(&external_name);
    let all = d::candidates(
        outcome.as_ref(),
        d::srv_targets(user_domain, &internal, &external),
    );
    let mut out = Vec::new();
    let named_by = format!("lyncdiscover[internal].{user_domain}");
    for t in all {
        // Only Autodiscover targets can be trusted across domains: they were named by a host of
        // the user's domain over verified TLS. Foreign DNS SRV targets are already dropped (CON-16).
        if t.in_user_domain
            || trust.trusts(&t.host)
            || (t.source == d::Source::Autodiscover && trust.ask(&t.host, user_domain, &named_by))
        {
            out.push((t.host, t.port));
        } else {
            eprintln!(
                "skipping {}:{} (outside your SIP domain, not trusted)",
                t.host, t.port
            );
        }
    }
    Ok(out)
}

struct Session {
    conn: SipConnection,
    keepalive: Option<KeepAlive>,
    verbose: bool,
    aor: String,
    epid: String,
    gruu: String,
    via_host: String,
    rng: OsRandom,
}

enum Pumped {
    Nothing,
    Registered,
    Done,
    /// A message that is not for the registrar (a response to one of our own requests).
    Message(Message),
}

impl Session {
    fn send(&mut self, m: &Message) -> Result<(), String> {
        if self.verbose {
            show(">>>", m);
        }
        self.conn.send(m).map_err(|e| e.to_string())?;
        if let Some(k) = &mut self.keepalive {
            k.on_data_sent(now());
        }
        Ok(())
    }

    fn hex(&mut self, n: usize) -> String {
        hex(&mut self.rng, n)
    }

    fn via(&mut self) -> String {
        let branch = self.hex(8);
        format!("SIP/2.0/TLS {};branch=z9hG4bK{branch}", self.via_host)
    }

    /// A new out-of-dialog request from this endpoint (RE-03: epid in From; GRUU as Contact).
    fn request(&mut self, method: &str, uri: &str) -> Message {
        let mut m = Message::request(method, uri);
        let via = self.via();
        m.push_header("Via", &via);
        m.push_header("Max-Forwards", "70");
        let tag = self.hex(5);
        m.push_header(
            "From",
            &format!("<{}>;tag={tag};epid={}", self.aor, self.epid),
        );
        m.push_header("To", &format!("<{uri}>"));
        let call_id = self.hex(16);
        m.push_header("Call-ID", &call_id);
        m.push_header("CSeq", &format!("1 {method}"));
        m.push_header("Contact", &format!("<{}>", self.gruu));
        m.push_header("User-Agent", &user_agent());
        m
    }

    /// Receives one message (or a timeout) and lets the registrar handle what is its business.
    fn pump(&mut self, reg: &mut Registrar<OsRandom>, verbose: bool) -> Result<Pumped, String> {
        let msg = self.conn.recv().map_err(|e| e.to_string())?;
        let events = match &msg {
            Some(m) => {
                if verbose {
                    show("<<<", m);
                }
                reg.on_message(m, now())
            }
            None => reg.on_tick(now()),
        };
        if let Some(k) = &mut self.keepalive {
            if let Some(bytes) = k.on_tick(now()) {
                self.conn.send_raw(bytes).map_err(|e| e.to_string())?;
            }
        }
        let mut result = Pumped::Nothing;
        let handled = !events.is_empty();
        for e in events {
            match e {
                Event::Send(m) => {
                    let is_authenticate = m
                        .header_values("Authorization")
                        .any(|v| v.contains("gssapi-data") && !v.contains("gssapi-data=\"\""));
                    if verbose && is_authenticate {
                        if let Some(i) = reg.ntlm_info() {
                            print_ntlm_info(i);
                        }
                    }
                    self.send(&m)?
                }
                Event::Registered(r) => {
                    println!(
                        "registered: expires {} s, GRUU {}",
                        r.expires,
                        r.gruu.as_deref().unwrap_or("-")
                    );
                    if let Some(g) = &r.gruu {
                        self.gruu = g.clone();
                    }
                    if self.keepalive.is_none() {
                        match r.keepalive_timeout {
                            Some(t) => {
                                println!("keep-alive: every {} s", t * 2 / 3);
                                self.keepalive = Some(KeepAlive::new(t, now()));
                            }
                            None => println!("keep-alive: not negotiated"),
                        }
                    }
                    result = Pumped::Registered;
                }
                Event::Redirected(t) => {
                    println!(
                        "redirected to home pool {}:{}; run again with --server {}:{}",
                        t.host, t.port, t.host, t.port
                    );
                    result = Pumped::Done;
                }
                Event::Unregistered => {
                    println!("signed out");
                    result = Pumped::Done;
                }
                Event::Deregistered(d) => {
                    println!("deregistered by the server: {d:?}");
                    result = Pumped::Done;
                }
                Event::SurvivableMode => println!("server reports survivable mode"),
                Event::Failed(f) => return Err(format!("sign-in failed: {f:?}")),
            }
        }
        if !handled {
            if let Some(m) = msg {
                return Ok(Pumped::Message(m));
            }
        }
        Ok(result)
    }

    /// Waits up to `secs` seconds for a request of the peer in `dialog` (e.g. NOTIFY). It must
    /// carry a valid signature (AE-32), otherwise it is dropped; it is answered with a signed
    /// 200 OK.
    fn wait_peer_request(
        &mut self,
        reg: &mut Registrar<OsRandom>,
        dialog: &Dialog,
        secs: u64,
    ) -> Result<Option<Message>, String> {
        let Some(m) = self.next_peer_request(reg, dialog, secs)? else {
            return Ok(None);
        };
        let ok = commscraft_sip_msg::dialog::response_to(&m, 200, "OK");
        self.send_signed(reg, ok)?;
        Ok(Some(m))
    }

    /// The next request of the peer in `dialog` within `secs` seconds, with a valid signature
    /// (AE-32); unsigned ones are dropped. Not answered yet.
    fn next_peer_request(
        &mut self,
        reg: &mut Registrar<OsRandom>,
        dialog: &Dialog,
        secs: u64,
    ) -> Result<Option<Message>, String> {
        let started = now();
        while now().saturating_sub(started) < secs {
            if let Pumped::Message(m) = self.pump(reg, self.verbose)? {
                if !dialog.is_peer_request(&m) {
                    continue;
                }
                if !reg.verify_message(&m) {
                    eprintln!(
                        "dropped a {} without a valid signature",
                        m.method().unwrap_or("request")
                    );
                    continue;
                }
                return Ok(Some(m));
            }
        }
        Ok(None)
    }

    fn send_signed(&mut self, reg: &mut Registrar<OsRandom>, mut m: Message) -> Result<(), String> {
        reg.sign_request(&mut m).map_err(|e| format!("{e:?}"))?;
        self.send(&m)
    }

    /// Adds Via (new branch), Max-Forwards, Contact (GRUU) and User-Agent to an in-dialog request.
    fn complete(&mut self, m: &mut Message) {
        let via = self.via();
        m.headers.insert(
            0,
            commscraft_sip_msg::Header {
                name: "Via".into(),
                value: via,
            },
        );
        m.push_header("Max-Forwards", "70");
        let contact = format!("<{}>", self.gruu);
        m.push_header("Contact", &contact);
        m.push_header("User-Agent", &user_agent());
    }

    /// Sends a signed request and waits for its final answer, which must carry a valid signature
    /// (AE-32); provisional answers are skipped, other traffic goes to the registrar.
    fn transact(
        &mut self,
        reg: &mut Registrar<OsRandom>,
        req: &Message,
    ) -> Result<Message, String> {
        self.transact_within(reg, req, 32)
    }

    /// Like `transact`, with an own timeout (an INVITE may ring longer than a transaction).
    fn transact_within(
        &mut self,
        reg: &mut Registrar<OsRandom>,
        req: &Message,
        secs: u64,
    ) -> Result<Message, String> {
        self.send(req)?;
        let call_id = req.call_id().ok().flatten().unwrap_or("").to_string();
        let cseq = req
            .cseq()
            .ok()
            .flatten()
            .map(|c| (c.number, c.method.to_string()));
        let started = now();
        loop {
            if now().saturating_sub(started) > secs {
                return Err(format!(
                    "no answer to {}",
                    req.method().unwrap_or("request")
                ));
            }
            match self.pump(reg, self.verbose)? {
                Pumped::Message(m) => {
                    let ours = m.is_response()
                        && m.call_id().ok().flatten() == Some(call_id.as_str())
                        && m.cseq()
                            .ok()
                            .flatten()
                            .map(|c| (c.number, c.method.to_string()))
                            == cseq;
                    if !ours || m.status_code().is_some_and(|c| c < 200) {
                        continue;
                    }
                    if !reg.verify_message(&m) {
                        return Err(format!(
                            "answer {} without a valid signature - not trusted",
                            m.status_code().unwrap_or(0)
                        ));
                    }
                    return Ok(m);
                }
                Pumped::Done => return Err("the registration ended".into()),
                Pumped::Nothing | Pumped::Registered => {}
            }
        }
    }
}

fn run(a: Args) -> Result<(), String> {
    if !a.authorized {
        return Err("refusing to contact a server without --authorized (interop runs only on systems you are allowed to test)".into());
    }
    let user_domain = a
        .aor
        .rsplit_once('@')
        .map(|(_, d)| d.to_ascii_lowercase())
        .ok_or("bad --aor")?;
    let mut rng = OsRandom::new().map_err(|e| e.to_string())?;
    let epid = match a.epid.clone() {
        Some(e) => e,
        None => load_or_create_epid(&mut rng)?,
    };
    let ca = match &a.ca_file {
        Some(p) => Some(std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?),
        None => None,
    };
    let tls = TlsClient::new(ca.as_deref()).map_err(|e| e.to_string())?;
    let mut trust = Trust::load(&a.trust_domains);
    let targets: Vec<(String, u16)> = match &a.server {
        Some(s) => vec![s.clone()],
        None => discover(&tls, &a.aor, &user_domain, a.verbose, &mut trust)?,
    };
    if a.discover_only {
        println!("discovery result (in order):");
        for (h, p) in &targets {
            println!("  {h}:{p}");
        }
        return Ok(());
    }
    let password = read_password()?;
    let timeout = Duration::from_secs(10);
    let mut conn = None;
    for (host, port) in &targets {
        eprintln!("connecting to {host}:{port} (TLS) ...");
        match tls.connect(host, *port, timeout) {
            Ok(c) => {
                eprintln!("connected, {}", c.protocol_version().unwrap_or_default());
                conn = Some(c);
                break;
            }
            Err(e) => eprintln!("  {e}"),
        }
    }
    let conn = conn.ok_or("no server reachable")?;
    let local = conn.local_addr().map_err(|e| e.to_string())?;
    let host = match local {
        std::net::SocketAddr::V4(v4) => v4.ip().to_string(),
        std::net::SocketAddr::V6(v6) => format!("[{}]", v6.ip()),
    };
    let cfg = Config {
        aor: a.aor.clone(),
        registrar_uri: format!("sip:{user_domain}"),
        epid: epid.clone(),
        local_host: host.clone(),
        local_port: local.port(),
        user_agent: format!("CommSCraft/{}", env!("CARGO_PKG_VERSION")),
        user: a.user.clone(),
        domain: a.domain.clone(),
        workstation: "COMMSCRAFT".into(),
        subnet: None,
        policy: Policy::default(),
        ntlm_legacy: a.ntlm_legacy,
    };
    let reg = Registrar::new(cfg, password, rng).map_err(|e| format!("configuration: {e:?}"))?;
    if a.command == Command::Interactive {
        let ecfg = commscraft_session::Config {
            aor: a.aor.clone(),
            epid: epid.clone(),
            via_host: format!("{host}:{}", local.port()),
            local_ip: local.ip(),
            user_agent: user_agent(),
            machine_name: "CommSCraft".into(),
        };
        return interactive(
            conn,
            commscraft_session::Engine::new(ecfg, reg),
            &tls,
            &trust,
            a.verbose,
        );
    }
    let mut reg = reg;
    let mut s = Session {
        conn,
        keepalive: None,
        verbose: a.verbose,
        aor: a.aor.clone(),
        epid: epid.clone(),
        gruu: a.aor.clone(),
        via_host: format!("{host}:{}", local.port()),
        rng: OsRandom::new().map_err(|e| e.to_string())?,
    };
    s.conn
        .set_timeout(Duration::from_secs(1))
        .map_err(|e| e.to_string())?;
    s.send(&reg.start(now()))?;

    // Phase 1: sign in.
    let started = now();
    loop {
        if now().saturating_sub(started) > 120 {
            return Err("gave up after 120 s".into());
        }
        match s.pump(&mut reg, a.verbose)? {
            Pumped::Registered => break,
            Pumped::Done => return Ok(()),
            Pumped::Nothing | Pumped::Message(_) => {}
        }
    }

    // Phase 2: the requested actions, each as a signed request with a verified answer.
    match a.command {
        Command::SignIn => {
            let mut sub = prov::subscribe(&prov::Request {
                aor: &a.aor,
                gruu: &s.gruu.clone(),
                epid: &epid,
                from_tag: &s.hex(5),
                call_id: &s.hex(16),
                cseq: 1,
                via: &s.via(),
                user_agent: &user_agent(),
                subnet: None,
            });
            reg.sign_request(&mut sub).map_err(|e| format!("{e:?}"))?;
            let answer = s.transact(&mut reg, &sub)?;
            match prov::from_message(&answer) {
                Ok(Some(p)) => print_provisioning(&p, &user_domain),
                Ok(None) => {
                    println!("provisioning: no piggybacked document (NOTIFY not handled yet)")
                }
                Err(e) => println!("provisioning: unusable document ({e:?})"),
            }
        }
        Command::Presence => presence_actions(&mut s, &mut reg, &a)?,
        Command::Im => im_actions(&mut s, &mut reg, &a, local.ip())?,
        // Handled above, before the phases.
        Command::Interactive => {}
    }

    // Phase 3: sign out.
    let m = reg.unregister(now()).map_err(|e| format!("{e:?}"))?;
    s.send(&m)?;
    let started = now();
    loop {
        if now().saturating_sub(started) > 40 {
            return Err("no answer to the sign-out".into());
        }
        if let Pumped::Done = s.pump(&mut reg, a.verbose)? {
            return Ok(());
        }
    }
}

/// The focus URI of a meeting link from the join manager (CB-40/CB-41, [MS-CONFBAS] §2.2.6):
/// a GET over verified TLS with the client Accept header, no credentials, no redirects followed.
/// Inference: our own User-Agent is sent, not the product string of §2.2.6.1.
fn meeting_focus(tls: &TlsClient, link: &str) -> Result<String, String> {
    let url = commscraft_url::parse_https(link).ok_or("not a usable https address")?;
    let r = commscraft_net::http::get(
        tls,
        &url,
        commscraft_conf::MEETING_ACCEPT,
        &user_agent(),
        Duration::from_secs(15),
    )
    .map_err(|e| e.to_string())?;
    if r.status != 200 {
        return Err(format!("HTTP {}", r.status));
    }
    let body = String::from_utf8(r.body).map_err(|_| "answer is not UTF-8")?;
    commscraft_conf::parse_meeting(&body).map_err(|e| format!("{e:?}"))
}

fn user_agent() -> String {
    format!("CommSCraft/{}", env!("CARGO_PKG_VERSION"))
}

/// `im`: a 1:1 IM session (OCS-UC09): INVITE, ACK, MESSAGE, show answers and typing for
/// `--wait` seconds, BYE. Plain text only.
fn im_actions(
    s: &mut Session,
    reg: &mut Registrar<OsRandom>,
    a: &Args,
    local_ip: std::net::IpAddr,
) -> Result<(), String> {
    let to = commscraft_presence::checked_sip_uri(
        a.to.as_deref().ok_or("im needs --to sip:user@domain")?,
    )
    .map_err(|_| "--to is not a usable SIP URI")?;
    let text = a.text.clone().ok_or("im needs --text")?;
    let mut invite = s.request("INVITE", &to);
    commscraft_im::invite_body(&mut invite, local_ip, false);
    reg.sign_request(&mut invite)
        .map_err(|e| format!("{e:?}"))?;
    println!("inviting {to} ...");
    let answer = s.transact_within(reg, &invite, 60)?;
    if answer.status_code() != Some(200) {
        let ack = commscraft_im::ack_for_failure(&invite, &answer);
        s.send_signed(reg, ack)?;
        println!(
            "the session was not accepted: {}",
            answer.status_code().unwrap_or(0)
        );
        return Ok(());
    }
    let mut dialog =
        Dialog::from_response(&invite, &answer).map_err(|e| format!("no usable dialog: {e:?}"))?;
    let invite_cseq = invite.cseq().ok().flatten().map_or(1, |c| c.number);
    let mut ack = commscraft_im::ack_for_2xx(&dialog, invite_cseq);
    s.complete(&mut ack);
    s.send_signed(reg, ack)?;
    if let Err(e) = commscraft_im::check_sdp(&answer.body) {
        println!("the peer answered with an unusable session description ({e:?}); ending");
    } else {
        let mut m = commscraft_im::message(&mut dialog, &text).map_err(|e| format!("{e:?}"))?;
        s.complete(&mut m);
        reg.sign_request(&mut m).map_err(|e| format!("{e:?}"))?;
        let r = s.transact(reg, &m)?;
        println!(
            "message: {}",
            if r.status_code() == Some(200) {
                "delivered".to_string()
            } else {
                format!("answer {}", r.status_code().unwrap_or(0))
            }
        );
        let until = now() + a.wait;
        while now() < until {
            let Some(req) = s.next_peer_request(reg, &dialog, until.saturating_sub(now()))? else {
                break;
            };
            let (incoming, answer) = commscraft_im::handle_request(&req);
            s.send_signed(reg, answer)?;
            match incoming {
                commscraft_im::Incoming::Text(t) => println!("{to}: {t}"),
                commscraft_im::Incoming::Typing(true) => println!("({to} is typing)"),
                commscraft_im::Incoming::Typing(false) => {}
                commscraft_im::Incoming::Bye => {
                    println!("{to} ended the conversation");
                    return Ok(());
                }
                commscraft_im::Incoming::Refused(code) => {
                    if s.verbose {
                        eprintln!(
                            "refused a {} with {code}",
                            req.method().unwrap_or("request")
                        );
                    }
                }
            }
        }
    }
    let mut bye = dialog.request("BYE");
    s.complete(&mut bye);
    reg.sign_request(&mut bye).map_err(|e| format!("{e:?}"))?;
    match s.transact(reg, &bye) {
        Ok(r) => println!("conversation ended ({})", r.status_code().unwrap_or(0)),
        Err(e) => eprintln!("ending the conversation: {e}"),
    }
    Ok(())
}

/// Address book search (MS-DLX SearchAbEntry, OB-44): the endpoint comes from in-band
/// provisioning; only hosts in the user's SIP domain or a trusted domain get credentials. NTLM
/// over HTTP per MS-NTHT (NEGOTIATE in the first request, AUTHENTICATE after the 401 with the
/// CHALLENGE, all on one connection). Blocks the loop for at most the HTTP timeout.
fn search(
    engine: &mut commscraft_session::Engine<OsRandom>,
    tls: &TlsClient,
    trust: &Trust,
    services: Option<&commscraft_provisioning::ServerConfiguration>,
    query: &str,
    verbose: bool,
) -> Result<Vec<commscraft_dlx::Entry>, String> {
    use commscraft_sipae::base64;
    let body = commscraft_dlx::search_request(query).map_err(|e| format!("{e:?}"))?;
    let c = services.ok_or("no provisioning yet (address book service unknown)")?;
    if c.dlx_enabled == Some(false) {
        return Err("the address book web search is disabled by the server".into());
    }
    let candidates: Vec<&commscraft_provisioning::ServiceUrl> = [
        &c.dlx_internal,
        &c.dlx_external,
        &c.abwq_internal,
        &c.abwq_external,
    ]
    .into_iter()
    .flatten()
    .collect();
    if candidates.is_empty() {
        return Err("the server announced no address book web service".into());
    }
    let mut last = String::from("no usable service");
    for s in candidates {
        if !s.in_user_domain && !trust.trusts(&s.host) {
            last = format!(
                "{} is outside your SIP domain; add --trust-domain to use it",
                s.host
            );
            continue;
        }
        let Some(url) = commscraft_url::parse_https(&s.url) else {
            continue;
        };
        if verbose {
            println!("  POST {} (SearchAbEntry)", s.url);
        }
        let negotiate =
            commscraft_ntlm::messages::negotiate_message(commscraft_ntlm::messages::HTTP_NEGOTIATE);
        let mut auth_err: Option<String> = None;
        let result = commscraft_net::http::post_with_auth(
            tls,
            &url,
            &[
                ("Content-Type", commscraft_dlx::CONTENT_TYPE),
                ("SOAPAction", commscraft_dlx::SOAP_ACTION),
            ],
            body.as_bytes(),
            &user_agent(),
            Duration::from_secs(15),
            &mut |challenge, cert| match challenge {
                None => Ok(format!("NTLM {}", base64::encode(&negotiate))),
                Some(ch) => {
                    let token = ch
                        .strip_prefix("NTLM ")
                        .and_then(|b| base64::decode(b.trim()).ok());
                    let answer = token
                        .ok_or_else(|| "no NTLM challenge".to_string())
                        .and_then(|t| engine.http_ntlm(&negotiate, &t, now(), cert));
                    match answer {
                        Ok(a) => Ok(format!("NTLM {}", base64::encode(&a))),
                        Err(e) => {
                            auth_err = Some(e);
                            Err(commscraft_net::NetError::Io(std::io::Error::other("NTLM")))
                        }
                    }
                }
            },
        );
        match result {
            Ok(r) if r.status == 200 => {
                let text = String::from_utf8(r.body).map_err(|_| "answer is not UTF-8")?;
                return commscraft_dlx::parse_response(&text).map_err(|e| format!("{e:?}"));
            }
            Ok(r) if r.status == 401 => return Err("not authorized (401)".into()),
            Ok(r) => {
                // A SOAP fault comes with 500.
                let text = String::from_utf8_lossy(&r.body).to_string();
                return match commscraft_dlx::parse_response(&text) {
                    Err(commscraft_dlx::Error::Fault(f)) => Err(format!("server fault: {f}")),
                    _ => Err(format!("HTTP {}", r.status)),
                };
            }
            Err(e) => last = auth_err.take().unwrap_or_else(|| e.to_string()),
        }
    }
    Err(last)
}

/// The local IPv4 subnet of `ip` from the kernel's routing table (`/proc/net/route`), for the
/// location request (E9-03).
fn subnet_of(ip: std::net::IpAddr) -> Option<std::net::IpAddr> {
    let table = std::fs::read_to_string("/proc/net/route").ok()?;
    subnet_in(&table, ip)
}

/// The most specific non-default route of `table` that contains `ip`. Destination and mask are
/// the kernel's network-order values printed as a native integer in hex, so the native byte
/// order gives back the address bytes.
fn subnet_in(table: &str, ip: std::net::IpAddr) -> Option<std::net::IpAddr> {
    let std::net::IpAddr::V4(v4) = ip else {
        return None;
    };
    let addr = u32::from(v4);
    let field = |s: &str| {
        u32::from_str_radix(s, 16)
            .ok()
            .map(|v| u32::from_be_bytes(v.to_ne_bytes()))
    };
    let mut best: Option<(u32, u32)> = None;
    for line in table.lines().skip(1).take(1024) {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(dest), Some(mask)) = (
            f.get(1).and_then(|s| field(s)),
            f.get(7).and_then(|s| field(s)),
        ) else {
            continue;
        };
        if mask != 0
            && addr & mask == dest
            && best.is_none_or(|(_, m)| mask.count_ones() > m.count_ones())
        {
            best = Some((dest, mask));
        }
    }
    best.map(|(d, _)| std::net::IpAddr::V4(std::net::Ipv4Addr::from(d)))
}

/// The location from the location information service (function 39; [MS-E911WS] GetLocations
/// over HTTPS with NTLM, E9-01 … E9-04): the first civic address for our IP and subnet.
/// One SOAP request to the location information service over HTTPS with NTLM (E9-02); the
/// answer body on HTTP 200.
fn lis_post(
    engine: &mut commscraft_session::Engine<OsRandom>,
    tls: &TlsClient,
    trust: &Trust,
    services: Option<&commscraft_provisioning::ServerConfiguration>,
    action: &str,
    body: &str,
    verbose: bool,
) -> Result<String, String> {
    use commscraft_sipae::base64;
    let s = services
        .and_then(|c| c.lis_internal.as_ref())
        .ok_or("the server announced no location information service")?;
    if !s.in_user_domain && !trust.trusts(&s.host) {
        return Err(format!(
            "{} is outside your SIP domain; add --trust-domain to use it",
            s.host
        ));
    }
    let url = commscraft_url::parse_https(&s.url).ok_or("the location service URL is not https")?;
    if verbose {
        println!("  POST {} ({action})", s.url);
    }
    let negotiate =
        commscraft_ntlm::messages::negotiate_message(commscraft_ntlm::messages::HTTP_NEGOTIATE);
    let mut auth_err: Option<String> = None;
    let result = commscraft_net::http::post_with_auth(
        tls,
        &url,
        &[
            ("Content-Type", commscraft_lis::CONTENT_TYPE),
            ("SOAPAction", action),
        ],
        body.as_bytes(),
        &user_agent(),
        Duration::from_secs(15),
        &mut |challenge, cert| match challenge {
            None => Ok(format!("NTLM {}", base64::encode(&negotiate))),
            Some(ch) => {
                let answer = ch
                    .strip_prefix("NTLM ")
                    .and_then(|b| base64::decode(b.trim()).ok())
                    .ok_or_else(|| "no NTLM challenge".to_string())
                    .and_then(|t| engine.http_ntlm(&negotiate, &t, now(), cert));
                match answer {
                    Ok(a) => Ok(format!("NTLM {}", base64::encode(&a))),
                    Err(e) => {
                        auth_err = Some(e);
                        Err(commscraft_net::NetError::Io(std::io::Error::other("NTLM")))
                    }
                }
            }
        },
    );
    match result {
        Ok(r) if r.status == 200 => {
            String::from_utf8(r.body).map_err(|_| "answer is not UTF-8".to_string())
        }
        Ok(r) => Err(format!("HTTP {}", r.status)),
        Err(e) => Err(auth_err.take().unwrap_or_else(|| e.to_string())),
    }
}

/// GetLocations for this network position (E9-03/E9-04).
fn lis_lookup(
    engine: &mut commscraft_session::Engine<OsRandom>,
    tls: &TlsClient,
    trust: &Trust,
    services: Option<&commscraft_provisioning::ServerConfiguration>,
    local_ip: std::net::IpAddr,
    verbose: bool,
) -> Result<Option<commscraft_lis::Location>, String> {
    let ids = commscraft_lis::NetworkIds {
        mac: None,
        subnet: subnet_of(local_ip),
        ip: Some(local_ip),
    };
    let body = commscraft_lis::get_locations_request(engine.own_aor(), &ids)
        .map_err(|e| format!("{e:?}"))?;
    let text = lis_post(
        engine,
        tls,
        trust,
        services,
        commscraft_lis::SOAP_ACTION,
        &body,
        verbose,
    )?;
    let (code, mut locs) =
        commscraft_lis::parse_get_locations(&text).map_err(|e| format!("{e:?}"))?;
    match code {
        200 if !locs.is_empty() => Ok(Some(locs.remove(0))),
        200 | 404 => Ok(None),
        c => Err(format!("location service answered {c}")),
    }
}

/// GetLocationsInCity (E9-08): the addresses the administrator entered for a city.
fn lis_city(
    engine: &mut commscraft_session::Engine<OsRandom>,
    tls: &TlsClient,
    trust: &Trust,
    services: Option<&commscraft_provisioning::ServerConfiguration>,
    (country, state, city): (&str, &str, &str),
    verbose: bool,
) -> Result<Vec<commscraft_lis::Location>, String> {
    let body =
        commscraft_lis::get_locations_in_city_request(engine.own_aor(), country, state, city)
            .map_err(|e| format!("{e:?}"))?;
    let text = lis_post(
        engine,
        tls,
        trust,
        services,
        commscraft_lis::SOAP_ACTION_IN_CITY,
        &body,
        verbose,
    )?;
    let (code, locs) =
        commscraft_lis::parse_get_locations_in_city(&text).map_err(|e| format!("{e:?}"))?;
    match code {
        200 | 404 => Ok(locs),
        c => Err(format!("location service answered {c}")),
    }
}

/// The interactive mode on top of the session engine. Lines from the terminal arrive through a
/// thread and a channel; network input and the clock are handled in this loop. Everything shown
/// comes from the engine's checked events (control characters already removed).
fn interactive(
    conn: SipConnection,
    mut engine: commscraft_session::Engine<OsRandom>,
    tls: &TlsClient,
    trust: &Trust,
    verbose: bool,
) -> Result<(), String> {
    use commscraft_session::{Command as C, Event as E, Output};
    use std::collections::BTreeMap;
    use std::sync::mpsc;

    let mut conn = conn;
    let local_ip = conn.local_addr().map_err(|e| e.to_string())?.ip();
    let (media_tx, media_rx) = mpsc::channel::<calls::FromMedia>();
    let mut handles: BTreeMap<u32, calls::MediaHandle> = BTreeMap::new();
    // Function 28: video streams (own ids), the audio media id each belongs to, the video id of
    // each call, and the descriptions waiting until both streams are ready.
    let (video_tx, video_rx) = mpsc::channel::<video_call::FromVideo>();
    let mut video_handles: BTreeMap<u32, video_call::VideoHandle> = BTreeMap::new();
    let mut video_for_audio: BTreeMap<u32, u32> = BTreeMap::new();
    let mut video_of_call: BTreeMap<u32, u32> = BTreeMap::new();
    let mut ready_audio: BTreeMap<u32, Box<commscraft_sdp::Local>> = BTreeMap::new();
    let mut ready_video: BTreeMap<u32, Box<commscraft_sdp::video::VideoLocal>> = BTreeMap::new();
    let camera = std::env::var("COMMSCRAFT_CAMERA").unwrap_or_else(|_| "/dev/video0".into());
    // Meetings with video: conversation → (video id, the source we watch).
    let mut conf_video: BTreeMap<u32, (u32, Option<u32>)> = BTreeMap::new();
    let mut next_media: u32 = 1;
    let mut pending_out: BTreeMap<u32, String> = BTreeMap::new();
    // AE-38: media ids of calls placed as delegate, with the boss.
    let mut pending_behalf: BTreeMap<u32, String> = BTreeMap::new();
    // Media ids of calls being prepared that are emergency calls (function 39).
    let mut pending_emergency: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    // The location policy enables emergency services: ask the location service (E9-03).
    let mut lis_wanted = false;
    // RG-51: the location policy and the next periodic location lookup (LocationRefreshInterval).
    let mut location_policy: Option<commscraft_provisioning::LocationPolicy> = None;
    let mut lis_next: Option<u64> = None;
    // E9-08: the addresses of the last /location city, to pick from.
    let mut city_choices: Vec<commscraft_lis::Location> = Vec::new();
    let mut pending_in: BTreeMap<u32, u32> = BTreeMap::new();
    // Media prepared for a conference's audio: media id → conversation.
    // Media handle → (conversation, announcement language, CA-24).
    let mut pending_conf: BTreeMap<u32, (u32, Option<String>)> = BTreeMap::new();
    // Conference audio calls whose media id is known before the engine names the call.
    let mut conf_media: BTreeMap<u32, u32> = BTreeMap::new();
    let mut media_of_call: BTreeMap<u32, u32> = BTreeMap::new();
    // Media of ended calls whose quality figures are still to come (bounded).
    let mut ended_media: BTreeMap<u32, u32> = BTreeMap::new();
    let mut outgoing: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    let mut current_call: Option<u32> = None;
    conn.set_timeout(Duration::from_millis(250))
        .map_err(|e| e.to_string())?;
    let (tx, rx) = mpsc::channel::<String>();
    // Commands the client gives itself (answering a call that replaces another, RFC 3891).
    let self_tx = tx.clone();
    // Media prepared for a transfer we follow: media id → the transferred call.
    let mut pending_transfer: BTreeMap<u32, u32> = BTreeMap::new();
    // Issue #67: transfers waiting for the user's decision (call → target).
    let mut transfer_offers: BTreeMap<u32, String> = BTreeMap::new();
    // The routing rules last asked for (forwarding and simultaneous ring are combined).
    let mut forwarding = commscraft_presence::Forwarding::default();
    // CB-43: the last meeting settings per conversation, so /options can change one part.
    let mut meeting_settings: BTreeMap<_, commscraft_conf::Settings> = BTreeMap::new();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut line = String::new();
        loop {
            line.clear();
            match stdin.lock().read_line(&mut line) {
                Ok(0) | Err(_) => {
                    let _ = tx.send("/quit".into());
                    break;
                }
                Ok(_) => {
                    if tx
                        .send(line.trim_end_matches(['\r', '\n']).to_string())
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    });

    let mut presence: BTreeMap<String, (String, String)> = BTreeMap::new(); // uri -> (name, state)
    let mut peers: BTreeMap<u32, String> = BTreeMap::new();
    let mut current: Option<u32> = None;
    let mut invitations: Vec<String> = Vec::new();
    let mut quitting = false;
    let mut services: Option<Box<commscraft_provisioning::ServerConfiguration>> = None;
    let mut outputs = engine.start(now());
    // Commands that events lead to (sent after the events are shown).
    let mut outputs_extra: Vec<C> = Vec::new();
    loop {
        for o in outputs.drain(..) {
            match o {
                Output::Send(m) => {
                    if verbose {
                        show(">>>", &m);
                    }
                    conn.send(&m).map_err(|e| e.to_string())?;
                }
                Output::SendRaw(b) => conn.send_raw(b).map_err(|e| e.to_string())?,
                Output::Event(ev) => match ev {
                    E::Registered(r) => {
                        println!("* signed in (registration valid {} s)", r.expires)
                    }
                    E::SignedOut => {
                        println!("* signed out");
                        return Ok(());
                    }
                    E::RegistrationEnded(why) => return Err(format!("registration ended: {why}")),
                    E::Provisioned(c) => {
                        services = Some(c);
                        if lis_wanted {
                            let r = lis_lookup(
                                &mut engine,
                                tls,
                                trust,
                                services.as_deref(),
                                local_ip,
                                verbose,
                            );
                            for line in lis_messages(&r, location_policy.as_ref()) {
                                println!("{line}");
                            }
                            if let Ok(Some(loc)) = r {
                                outputs_extra.push(C::SetLocation(Box::new(loc)));
                            }
                            lis_next = lis_refresh_at(location_policy.as_ref(), now());
                        }
                    }
                    E::LocationPolicy(p) => {
                        lis_wanted = p.enabled;
                        location_policy = Some((*p).clone());
                        if p.enabled {
                            let numbers: Vec<String> = p
                                .numbers
                                .iter()
                                .map(|n| {
                                    if n.masks.is_empty() {
                                        n.dial.clone()
                                    } else {
                                        format!("{} ({})", n.dial, n.masks.join(", "))
                                    }
                                })
                                .collect();
                            println!("* emergency numbers: {}", numbers.join("; "));
                        }
                    }
                    E::Location(l) => println!(
                        "* emergency location: {}{}",
                        l.summary(),
                        if l.manual {
                            " (entered by hand, not verified by the network)"
                        } else {
                            ""
                        }
                    ),
                    // Calls: signalling is in the engine; the media path (and with it /call and
                    // /answer) follows. An incoming call rings and ends after 60 s with 480.
                    E::CallRinging { call } => println!("* call {call} ringing"),
                    E::CallVideoControl { call, control } => {
                        // PL-50: a freeze comes only from an MCU (XMLMC §1.6) – ignored in 1:1
                        // calls; a fast update always means a key frame (and resuming).
                        let in_meeting = conf_video
                            .values()
                            .any(|(v, _)| video_of_call.get(&call) == Some(v));
                        let freeze =
                            control == commscraft_session::media_control::VideoControl::Freeze;
                        if let Some(h) = video_of_call.get(&call).and_then(|v| video_handles.get(v))
                        {
                            if !freeze || in_meeting {
                                let _ = h.tx.send(video_call::ToVideo::Control(control));
                            }
                        }
                    }
                    E::CallVideo { call, video } => {
                        println!("* call {call}: video negotiated");
                        if let Some(h) = video_of_call.get(&call).and_then(|v| video_handles.get(v))
                        {
                            let _ = h.tx.send(video_call::ToVideo::Start {
                                video,
                                controlling: outgoing.contains(&call),
                            });
                        }
                    }
                    E::IncomingCall { call, from } => {
                        // PS-02/PS-04/PS-07: number, gateway and anonymous callers.
                        // RE-72 / AE-38: meant for someone else, or a delegate calling for
                        // someone (then the server's Referred-By names the same person).
                        let for_whom = match engine.caller(call) {
                            Some(c) => match (c.acting_for, c.on_behalf_of) {
                                (Some(a), _) => format!(" on behalf of {a}"),
                                (None, Some(b)) => format!(" for {b}"),
                                (None, None) => String::new(),
                            },
                            None => String::new(),
                        };
                        let who = match engine.caller(call) {
                            Some(c) if c.anonymous => "an anonymous caller".to_string(),
                            Some(c) => match (c.number, c.gateway) {
                                (Some(n), true) => format!("{n} (phone line)"),
                                (Some(n), false) => format!("{n} ({})", c.uri),
                                (None, _) => c.uri,
                            },
                            None => from,
                        } + &for_whom;
                        if let Some(old) = engine.replaces(call) {
                            // RFC 3891: an attended transfer replaces call `old` – answer it.
                            println!("* call {old} is being replaced by call {call} from {who}");
                            let _ = self_tx.send(format!("/answer {call}"));
                        } else {
                            println!(
                                "* incoming call {call} from {who} – /answer {call} or /hangup {call}"
                            );
                        }
                        current_call = Some(call);
                    }
                    E::CallAnswered {
                        call,
                        media,
                        continues_early,
                    } => {
                        println!("* call {call} connected ({:?})", media.codec);
                        current_call = Some(call);
                        if let Some(h) = media_of_call.get(&call).and_then(|m| handles.get(m)) {
                            let _ = h.tx.send(if continues_early {
                                calls::ToMedia::Confirm(media)
                            } else {
                                calls::ToMedia::Start {
                                    media,
                                    controlling: outgoing.contains(&call),
                                    early: false,
                                }
                            });
                        }
                    }
                    // MS-SDPEXT §3.1.5.12: play what the far end sends before it answers
                    // (announcements, ringback from a gateway); speech only after the answer.
                    E::CallEarlyMedia { call, media } => {
                        println!("* call {call}: early media ({:?})", media.codec);
                        if let Some(h) = media_of_call.get(&call).and_then(|m| handles.get(m)) {
                            let _ = h.tx.send(calls::ToMedia::Start {
                                media,
                                controlling: true,
                                early: true,
                            });
                        }
                    }
                    E::CallEarlyMediaEnded { call } => {
                        println!("* call {call}: early media ended");
                        if let Some(h) = media_of_call.get(&call).and_then(|m| handles.get(m)) {
                            let _ = h.tx.send(calls::ToMedia::EndEarly);
                        }
                    }
                    E::CallUpdated { call, media } => {
                        // MS-SDPEXT §3.1.5.23/§3.1.5.27: the peer's hold or music on hold.
                        match (media.direction, media.music_on_hold) {
                            (commscraft_sdp::Direction::SendOnly, true) => {
                                println!("* call {call}: on hold (music)")
                            }
                            (
                                commscraft_sdp::Direction::SendOnly
                                | commscraft_sdp::Direction::Inactive,
                                _,
                            ) => {
                                println!("* call {call}: on hold by the peer")
                            }
                            _ => println!("* call {call}: media updated"),
                        }
                        if let Some(h) = media_of_call.get(&call).and_then(|m| handles.get(m)) {
                            let _ = h.tx.send(calls::ToMedia::Update(media));
                        }
                    }
                    E::TransferRequested { call, to } if !auto_follow(&to, engine.own_aor()) => {
                        // Issue #67: a phone number or another domain may cost money or reach
                        // someone unexpected – the user decides.
                        println!("* call {call}: the other side transfers you to {to} – /follow {call} or /decline {call}");
                        transfer_offers.insert(call, to);
                    }
                    E::TransferRequested { call, to } => {
                        // RFC 3515: the peer transfers us within our own domain; place the new
                        // call.
                        println!("* call {call}: transferred to {to} – connecting …");
                        let relay = engine.media_relay().and_then(calls::relay_from);
                        let id = next_media;
                        next_media = next_media.wrapping_add(1).max(1);
                        let h = calls::spawn(
                            id,
                            local_ip,
                            relay,
                            None,
                            commscraft_audio_alsa::Config::default(),
                            media_tx.clone(),
                        );
                        handles.insert(id, h);
                        pending_transfer.insert(id, call);
                    }
                    E::TransferResult { call, status } => {
                        if (200..300).contains(&status) {
                            println!("* call {call} transferred");
                        } else {
                            println!("* transfer of call {call} failed ({status})");
                        }
                    }
                    E::CallParked { call, orbit } => {
                        println!(
                            "* call {call} parked at orbit {orbit} – retrieve with /call {orbit}"
                        )
                    }
                    E::ParkNotification {
                        orbit,
                        reason,
                        target,
                    } => {
                        use commscraft_park::UnparkReason as U;
                        let what = match reason {
                            U::Retrieval => "retrieved",
                            U::HangUp => "the parked party hung up",
                            U::Ringback => "rings back to you",
                            U::Fallback => "went to the fallback destination",
                            U::Drop => "was dropped",
                        };
                        println!(
                            "* parked call at orbit {orbit}: {what}{}",
                            target.map(|t| format!(" ({t})")).unwrap_or_default()
                        );
                    }
                    E::CallEnded { call, status } => {
                        println!("* call {call} ended ({status})");
                        if let Some(v) = video_of_call.remove(&call) {
                            if let Some(h) = video_handles.remove(&v) {
                                let _ = h.tx.send(video_call::ToVideo::Stop);
                            }
                        }
                        if let Some(m) = media_of_call.remove(&call) {
                            if ended_media.len() >= 16 {
                                ended_media.pop_first();
                            }
                            ended_media.insert(m, call);
                            if let Some(h) = handles.remove(&m) {
                                let _ = h.tx.send(calls::ToMedia::Stop);
                            }
                        }
                        outgoing.remove(&call);
                        if current_call == Some(call) {
                            current_call = None;
                        }
                    }
                    E::MediaRelayReady { relays, minutes } => {
                        println!("* media relay ready ({relays} relays, valid {minutes} min)")
                    }
                    E::ContactList(l) => {
                        for c in &l.contacts {
                            presence
                                .entry(c.uri.clone())
                                .or_insert_with(|| (c.name.clone(), "?".into()));
                        }
                        println!("* {} contacts", l.contacts.len());
                    }
                    E::Presence(p) => {
                        let state = format!("{:?}", p.class());
                        let entry = presence
                            .entry(p.uri.clone())
                            .or_insert_with(|| (String::new(), String::new()));
                        if entry.0.is_empty() {
                            entry.0 = p.display_name.clone().unwrap_or_default();
                        }
                        if entry.1 != state && entry.1 != "?" {
                            println!(
                                "* {} is now {state}",
                                if entry.0.is_empty() { &p.uri } else { &entry.0 }
                            );
                        }
                        entry.1 = state;
                    }
                    E::PresenceUnavailable { uri, state } => {
                        if let Some(e) = presence.get_mut(&uri) {
                            e.1 = state;
                        }
                    }
                    E::OwnStatus(s) => println!("* your state: {s:?} (until you sign out)"),
                    E::OwnStatusFailed(c) => println!("* setting your state failed ({c})"),
                    E::Delegates(list) if list.is_empty() => println!("* delegates: none"),
                    E::Delegates(list) => println!("* delegates: {}", list.join(", ")),
                    E::Forwarding(f) => {
                        // PR-13: the server's rules – also those set by another client – are
                        // the base for the next /forward, /team or /delegates.
                        forwarding = f.clone();
                        let mut parts = Vec::new();
                        match (&f.forward_to, f.immediate) {
                            (Some(t), true) => parts.push(format!("all calls go to {t}")),
                            (Some(t), false) => parts.push(format!(
                                "unanswered calls go to {t} after {} s",
                                f.ring_seconds.unwrap_or(20)
                            )),
                            (None, true) => parts.push("all calls go to voice mail".into()),
                            (None, false) => parts.push("no forwarding".into()),
                        }
                        if let Some(s) = &f.simultaneous_ring {
                            parts.push(format!("{s} rings too"));
                        }
                        if f.team_ring {
                            parts.push(format!(
                                "the team ({}) rings after {} s",
                                f.team.join(", "),
                                f.user_seconds.unwrap_or(10)
                            ));
                        }
                        if f.delegate_ring {
                            parts.push(format!(
                                "the delegates ({}) ring{}",
                                f.delegates.join(", "),
                                if f.skip_primary {
                                    " instead of you".to_string()
                                } else {
                                    format!(" after {} s", f.user_seconds.unwrap_or(10))
                                }
                            ));
                        }
                        println!("* call routing: {}", parts.join("; "));
                    }
                    E::ConversationStarted {
                        conversation,
                        peer,
                        incoming,
                    } => {
                        println!(
                            "* [{conversation}] {} {peer}",
                            if incoming {
                                "conversation from"
                            } else {
                                "connected with"
                            }
                        );
                        peers.insert(conversation, peer);
                        current = Some(conversation);
                    }
                    E::ConversationFailed {
                        conversation,
                        status,
                    } => {
                        println!("* [{conversation}] not accepted ({status})");
                        peers.remove(&conversation);
                    }
                    E::Text {
                        conversation,
                        sender,
                        text,
                    } => {
                        let who = sender
                            .map(|s| {
                                presence
                                    .get(&s)
                                    .map(|(n, _)| n.clone())
                                    .filter(|n| !n.is_empty())
                                    .unwrap_or(s)
                            })
                            .or_else(|| peers.get(&conversation).cloned())
                            .unwrap_or_default();
                        println!("[{conversation}] {who}: {text}");
                    }
                    E::MeetingSettings {
                        conversation,
                        settings,
                    } => {
                        let mut parts = Vec::new();
                        if let Some(l) = settings.locked {
                            parts.push(if l { "locked" } else { "not locked" }.to_string());
                        }
                        if let Some(a) = settings.admission {
                            parts.push(format!("admission {}", a.as_str()));
                        }
                        if let Some(p) = settings.autopromote {
                            parts.push(format!("promoted on joining: {}", promote_name(p)));
                        }
                        if let Some(b) = settings.pstn_lobby_bypass {
                            parts.push(
                                if b {
                                    "phone callers bypass the lobby"
                                } else {
                                    "phone callers wait in the lobby"
                                }
                                .to_string(),
                            );
                        }
                        if let Some(a) = settings.announcements {
                            parts.push(
                                if a {
                                    "entry/exit announcements on"
                                } else {
                                    "entry/exit announcements off"
                                }
                                .to_string(),
                            );
                        }
                        if settings.lobby_capable == Some(false) {
                            parts.push("options cannot be changed".into());
                        }
                        if !parts.is_empty() {
                            println!("* [{conversation}] meeting: {}", parts.join(", "));
                        }
                        meeting_settings.insert(conversation, settings);
                    }
                    E::Participants {
                        conversation,
                        participants,
                    } => {
                        // CA-13: the client names the video to watch – the first other
                        // participant that sends video.
                        if let Some((vid, current)) = conf_video.get_mut(&conversation) {
                            let me = engine.own_aor().to_ascii_lowercase();
                            let pick = participants
                                .iter()
                                .filter(|p| p.uri.to_ascii_lowercase() != me)
                                .find_map(|p| p.video_source().map(|m| (m, p.uri.clone())));
                            if let Some((msi, who)) = pick {
                                if *current != Some(msi) {
                                    *current = Some(msi);
                                    if let Some(h) = video_handles.get(vid) {
                                        let _ = h.tx.send(video_call::ToVideo::Source(msi));
                                    }
                                    println!("* [{conversation}] watching the video of {who}");
                                }
                            }
                        }
                        let names: Vec<String> = participants
                            .iter()
                            .filter(|p| p.in_chat() || p.audio_muted().is_some() || p.waiting())
                            .map(|p| {
                                let n = p.name.clone().unwrap_or_else(|| p.uri.clone());
                                let n = if p.waiting() {
                                    format!("{n} <{}> (in the lobby)", p.uri)
                                } else {
                                    n
                                };
                                match p.audio_muted() {
                                    Some(true) => format!("{n} (audio, muted)"),
                                    Some(false) => format!("{n} (audio)"),
                                    None => n,
                                }
                            })
                            .collect();
                        println!(
                            "* [{conversation}] in the conversation: {}",
                            names.join(", ")
                        );
                    }
                    E::ConferenceLobby {
                        conversation,
                        waiting,
                    } => {
                        if waiting {
                            println!("* [{conversation}] waiting in the lobby for admission …");
                        } else {
                            println!("* [{conversation}] admitted to the meeting");
                        }
                    }
                    E::ConferenceAudio { conversation, call } => {
                        if let Some((vid, _)) = conf_video.get(&conversation) {
                            video_of_call.insert(call, *vid);
                        }
                        if let Some(m) = conf_media.remove(&conversation) {
                            media_of_call.insert(call, m);
                            outgoing.insert(call);
                            current_call = Some(call);
                        }
                        println!("* [{conversation}] joining the meeting audio (call {call})");
                    }
                    E::MeetingScheduled(m) => {
                        println!(
                            "* meeting {} scheduled{} – join with /join {}",
                            m.conference_id,
                            m.expiry
                                .as_deref()
                                .map(|x| format!(" until {x}"))
                                .unwrap_or_default(),
                            m.focus.as_deref().unwrap_or("(conference address unknown)")
                        );
                    }
                    E::Meetings(list) if list.is_empty() => println!("* no meetings"),
                    E::Meetings(list) => {
                        for m in list {
                            println!(
                                "  {:<34} {:<20} {}{}",
                                m.conference_id,
                                m.admission.map(|a| a.as_str()).unwrap_or(""),
                                if m.static_meeting { "(static) " } else { "" },
                                m.subject.as_deref().unwrap_or("")
                            );
                        }
                    }
                    E::MeetingDetails(m) => {
                        println!(
                            "* meeting {}: {}{}{}",
                            m.conference_id,
                            m.subject.as_deref().unwrap_or("(no subject)"),
                            m.expiry
                                .as_deref()
                                .map(|x| format!(", until {x}"))
                                .unwrap_or_default(),
                            m.join_url
                                .as_deref()
                                .map(|u| format!(", join link {u}"))
                                .unwrap_or_default()
                        );
                    }
                    E::MeetingCancelled { conference_id } => {
                        println!("* meeting {conference_id} cancelled")
                    }
                    E::ConferenceInvitation { from, invitation } => {
                        invitations.push(invitation.focus.clone());
                        println!(
                            "* invitation {} from {from}{} – join with /join {}",
                            invitations.len(),
                            invitation
                                .subject
                                .map(|s| format!(" ({s})"))
                                .unwrap_or_default(),
                            invitations.len()
                        );
                    }
                    E::NotDeliveredTo {
                        conversation,
                        recipient,
                        status,
                    } => println!("* [{conversation}] not delivered to {recipient} ({status})"),
                    E::Typing {
                        conversation,
                        typing,
                    } => {
                        if typing {
                            println!("[{conversation}] (typing)");
                        }
                    }
                    E::Delivered { .. } => {}
                    E::NotDelivered {
                        conversation,
                        status,
                    } => println!("* [{conversation}] message not delivered ({status})"),
                    E::ConversationEnded { conversation } => {
                        println!("* [{conversation}] ended");
                        peers.remove(&conversation);
                        if current == Some(conversation) {
                            current = peers.keys().next().copied();
                        }
                    }
                    E::Notice(n) => println!("* {n}"),
                },
            }
        }
        for c in std::mem::take(&mut outputs_extra) {
            outputs.extend(engine.command(c, now()));
        }
        let t = now();
        match conn.recv() {
            Ok(Some(m)) => {
                if verbose {
                    show("<<<", &m);
                }
                outputs.extend(engine.on_message(&m, t));
            }
            Ok(None) => {}
            Err(e) => return Err(e.to_string()),
        }
        outputs.extend(engine.on_tick(t));
        // RG-51: LocationRefreshInterval – look the location up again; a location found
        // replaces the current one, none found keeps it.
        if lis_wanted && lis_next.is_some_and(|n| t >= n) {
            lis_next = lis_refresh_at(location_policy.as_ref(), t);
            if let Ok(Some(loc)) = lis_lookup(
                &mut engine,
                tls,
                trust,
                services.as_deref(),
                local_ip,
                verbose,
            ) {
                outputs.extend(engine.command(C::SetLocation(Box::new(loc)), t));
            }
        }
        // Video streams: ready, selected, notices (function 28).
        while let Ok(v) = video_rx.try_recv() {
            match v {
                video_call::FromVideo::Ready(vid, local) => {
                    ready_video.insert(vid, local);
                }
                video_call::FromVideo::Selected(vid, local) => {
                    if let Some((&call, _)) = video_of_call.iter().find(|(_, &v)| v == vid) {
                        outputs
                            .extend(engine.command(C::SetCallVideo { call, video: local }, now()));
                    }
                }
                video_call::FromVideo::Notice(vid, n) => println!("* [video {vid}] {n}"),
                video_call::FromVideo::PictureUpdate(vid) => {
                    if let Some((&call, _)) = video_of_call.iter().find(|(_, &v)| v == vid) {
                        outputs.extend(engine.command(C::RequestPictureUpdate { call }, now()));
                    }
                }
                video_call::FromVideo::Failed(vid, why) => {
                    println!("* [video {vid}] video ended: {why}");
                    video_handles.remove(&vid);
                }
            }
        }
        // A video call goes out (or is answered) once both of its streams are ready.
        let both: Vec<(u32, u32)> = video_for_audio
            .iter()
            .filter(|(a, v)| ready_audio.contains_key(a) && ready_video.contains_key(v))
            .map(|(a, v)| (*a, *v))
            .collect();
        for (aid, vid) in both {
            video_for_audio.remove(&aid);
            let (Some(local), Some(video)) = (ready_audio.remove(&aid), ready_video.remove(&vid))
            else {
                continue;
            };
            let policy = calls::policy(services.as_ref().and_then(|s| s.av_encryption));
            if let Some(to) = pending_out.remove(&aid) {
                let call = engine.next_call_id();
                media_of_call.insert(call, aid);
                video_of_call.insert(call, vid);
                outgoing.insert(call);
                outputs.extend(engine.command(
                    C::VideoCall {
                        to,
                        local,
                        video,
                        policy,
                    },
                    now(),
                ));
            } else if let Some((conversation, language)) = pending_conf.remove(&aid) {
                conf_media.insert(conversation, aid);
                outputs.extend(engine.command(
                    C::JoinConferenceVideo {
                        conversation,
                        local,
                        video,
                        policy,
                        language,
                    },
                    now(),
                ));
            } else if let Some(call) = pending_in.remove(&aid) {
                media_of_call.insert(call, aid);
                video_of_call.insert(call, vid);
                outputs.extend(engine.command(
                    C::AcceptVideoCall {
                        call,
                        local,
                        video,
                        policy,
                    },
                    now(),
                ));
            }
        }
        while let Ok(m) = media_rx.try_recv() {
            let policy = calls::policy(services.as_ref().and_then(|s| s.av_encryption));
            match m {
                calls::FromMedia::Ready(id, local) if video_for_audio.contains_key(&id) => {
                    // A video call waits for its video stream too.
                    ready_audio.insert(id, local);
                }
                calls::FromMedia::Ready(id, local) => {
                    if let Some(to) = pending_out.remove(&id) {
                        let call = engine.next_call_id();
                        media_of_call.insert(call, id);
                        outgoing.insert(call);
                        let command = if pending_emergency.remove(&id) {
                            C::EmergencyCall {
                                dialled: to,
                                local,
                                policy,
                            }
                        } else if let Some(boss) = pending_behalf.remove(&id) {
                            C::CallOnBehalf {
                                to,
                                boss,
                                local,
                                policy,
                            }
                        } else {
                            C::Call { to, local, policy }
                        };
                        outputs.extend(engine.command(command, now()));
                    } else if let Some(old) = pending_transfer.remove(&id) {
                        let call = engine.next_call_id();
                        media_of_call.insert(call, id);
                        outgoing.insert(call);
                        outputs.extend(engine.command(
                            C::FollowTransfer {
                                call: old,
                                local,
                                policy,
                            },
                            now(),
                        ));
                    } else if let Some((conversation, language)) = pending_conf.remove(&id) {
                        conf_media.insert(conversation, id);
                        outputs.extend(engine.command(
                            C::JoinConferenceAudio {
                                conversation,
                                local,
                                policy,
                                language,
                            },
                            now(),
                        ));
                    } else if let Some(call) = pending_in.remove(&id) {
                        media_of_call.insert(call, id);
                        outputs.extend(engine.command(
                            C::AcceptCall {
                                call,
                                local,
                                policy,
                            },
                            now(),
                        ));
                    }
                }
                calls::FromMedia::Selected(id, local) => {
                    if let Some((&call, _)) = media_of_call.iter().find(|(_, &m)| m == id) {
                        outputs.extend(engine.command(C::UpdateCall { call, local }, now()));
                    }
                }
                calls::FromMedia::Dtmf(id, d) => println!("* [media {id}] DTMF {d}"),
                calls::FromMedia::PeerBye(id) => {
                    println!("* [media {id}] the other side left the media session")
                }
                calls::FromMedia::Quality(id, q) => {
                    // QE-01: the report after every call with media.
                    if let Some(call) = ended_media.remove(&id) {
                        outputs.extend(engine.command(
                            C::ReportQuality {
                                call,
                                audio: Box::new(calls::audio_line(&q)),
                            },
                            now(),
                        ));
                    }
                }
                calls::FromMedia::Notice(id, what) => {
                    match media_of_call.iter().find(|(_, &m)| m == id) {
                        Some((call, _)) => println!("* call {call}: {what}"),
                        None => println!("* [media {id}] {what}"),
                    }
                }
                calls::FromMedia::Failed(id, why) => {
                    println!("* media problem: {why}");
                    if let Some((&call, _)) = media_of_call.iter().find(|(_, &m)| m == id) {
                        outputs.extend(engine.command(C::Hangup { call }, now()));
                    } else if let Some(h) = handles.remove(&id) {
                        let _ = h.tx.send(calls::ToMedia::Stop);
                        pending_out.remove(&id);
                        pending_behalf.remove(&id);
                        pending_in.remove(&id);
                    }
                }
            }
        }
        while let Ok(line) = rx.try_recv() {
            let line = line.trim().to_string();
            let mut words = line.splitn(2, ' ');
            let cmd = words.next().unwrap_or("");
            let arg = words.next().unwrap_or("").trim().to_string();
            let command = match cmd {
                "" => None,
                "/quit" => {
                    if quitting {
                        return Ok(());
                    }
                    quitting = true;
                    Some(C::SignOut)
                }
                "/contacts" => {
                    for (uri, (name, state)) in &presence {
                        println!("  {state:<14} {uri:<40} {name}");
                    }
                    None
                }
                "/status" => match commscraft_presence::UserStatus::parse(&arg) {
                    Some(s) => Some(C::SetStatus(s)),
                    None => {
                        println!("* /status online|busy|dnd|away");
                        None
                    }
                },
                "/chat" => Some(C::StartConversation { to: arg }),
                "/call" | "/callback" | "/callfor" | "/videocall" => {
                    // AE-38: /callfor sip:BOSS TARGET calls as the boss's delegate.
                    let (boss, arg) = if cmd == "/callfor" {
                        let mut w = arg.splitn(2, ' ');
                        let boss = w.next().unwrap_or("").to_string();
                        let rest = w.next().unwrap_or("").trim().to_string();
                        if !boss.starts_with("sip:") || rest.is_empty() {
                            println!("* /callfor sip:BOSS sip:URI|number");
                            continue;
                        }
                        (Some(boss), rest)
                    } else {
                        (None, arg)
                    };
                    // A phone number becomes a SIP URI with the dial plan (PS-01); /callback N
                    // calls the asserted identity of call N (PS-02).
                    // Function 39: a number of the location policy is an emergency call.
                    let emergency = (cmd == "/call")
                        .then(|| engine.emergency_dial(&arg))
                        .flatten();
                    let target = if emergency.is_some() {
                        Some(arg.trim().to_string())
                    } else if cmd == "/callback" {
                        arg.parse::<u32>().ok().and_then(|n| engine.callback_uri(n))
                    } else if arg.starts_with("sip:") {
                        Some(arg.clone())
                    } else {
                        engine.phone_uri(&arg)
                    };
                    let Some(arg) = target else {
                        println!(
                            "* /call sip:URI or a phone number; /callback N needs a known caller"
                        );
                        continue;
                    };
                    if arg.contains(";user=phone") {
                        println!("* calling {arg}");
                    }
                    let relay = engine.media_relay().and_then(calls::relay_from);
                    let bwm = calls::bandwidth_config(
                        services.as_ref().and_then(|s| s.enable_bw_policy_check) == Some(true),
                        engine.own_aor(),
                        Some(arg.as_str()),
                        None,
                    );
                    let id = next_media;
                    next_media = next_media.wrapping_add(1).max(1);
                    let h = calls::spawn(
                        id,
                        local_ip,
                        relay,
                        bwm,
                        commscraft_audio_alsa::Config::default(),
                        media_tx.clone(),
                    );
                    handles.insert(id, h);
                    pending_out.insert(id, arg.clone());
                    if cmd == "/videocall" && emergency.is_none() {
                        let vid = next_media;
                        next_media = next_media.wrapping_add(1).max(1);
                        let relay = engine.media_relay().and_then(calls::relay_from);
                        video_handles.insert(
                            vid,
                            video_call::spawn(
                                vid,
                                local_ip,
                                relay,
                                camera.clone(),
                                video_tx.clone(),
                            ),
                        );
                        video_for_audio.insert(id, vid);
                    }
                    if let Some(b) = boss {
                        pending_behalf.insert(id, b);
                    }
                    if let Some(dial) = &emergency {
                        pending_emergency.insert(id);
                        println!("* EMERGENCY CALL to {dial} – preparing …");
                    }
                    println!("* preparing the call (gathering media addresses) …");
                    None
                }
                "/answer" | "/answervideo" => match arg.parse::<u32>().ok().or(current_call) {
                    Some(call) => {
                        let relay = engine.media_relay().and_then(calls::relay_from);
                        // The callee is the bandwidth-managing endpoint ([MS-ICE2BWM] §3.1.4).
                        let bwm = calls::bandwidth_config(
                            services.as_ref().and_then(|s| s.enable_bw_policy_check) == Some(true),
                            engine.own_aor(),
                            engine.call_peer(call),
                            engine.incoming_offer(call),
                        );
                        let id = next_media;
                        next_media = next_media.wrapping_add(1).max(1);
                        let h = calls::spawn(
                            id,
                            local_ip,
                            relay,
                            bwm,
                            commscraft_audio_alsa::Config::default(),
                            media_tx.clone(),
                        );
                        handles.insert(id, h);
                        pending_in.insert(id, call);
                        if cmd == "/answervideo" {
                            if engine.incoming_video(call) {
                                let vid = next_media;
                                next_media = next_media.wrapping_add(1).max(1);
                                let relay = engine.media_relay().and_then(calls::relay_from);
                                video_handles.insert(
                                    vid,
                                    video_call::spawn(
                                        vid,
                                        local_ip,
                                        relay,
                                        camera.clone(),
                                        video_tx.clone(),
                                    ),
                                );
                                video_for_audio.insert(id, vid);
                            } else {
                                println!("* call {call} offers no video – answering with audio");
                            }
                        }
                        None
                    }
                    None => {
                        println!("* no call to answer");
                        None
                    }
                },
                "/forward" | "/simring" => {
                    // RE-50/RE-51: TARGET is a SIP address or a phone number.
                    let mut w = arg.split_whitespace();
                    let first = w.next().unwrap_or("");
                    let target = |s: &str| {
                        if s.starts_with("sip:") {
                            Some(s.to_string())
                        } else {
                            engine.phone_uri(s)
                        }
                    };
                    let mut next = forwarding.clone();
                    let ok = match (cmd, first) {
                        ("/simring", "off") => {
                            next.simultaneous_ring = None;
                            true
                        }
                        ("/simring", t) => {
                            next.simultaneous_ring = target(t);
                            next.simultaneous_ring.is_some()
                        }
                        (_, "off") => {
                            next.forward_to = None;
                            next.immediate = false;
                            next.ring_seconds = None;
                            true
                        }
                        (_, "voicemail") => {
                            next.forward_to = None;
                            next.immediate = true;
                            true
                        }
                        (_, "noanswer") => {
                            next.forward_to = w.next().and_then(target);
                            next.immediate = false;
                            next.ring_seconds = w.next().and_then(|s| s.parse().ok());
                            next.forward_to.is_some()
                        }
                        (_, t) => {
                            next.forward_to = target(t);
                            next.immediate = true;
                            next.forward_to.is_some()
                        }
                    };
                    if ok {
                        forwarding = next.clone();
                        Some(C::SetForwarding(next))
                    } else {
                        println!("* /forward off|voicemail|TARGET|noanswer TARGET [SECONDS]  /simring TARGET|off");
                        None
                    }
                }
                "/follow" | "/decline" => {
                    let call = arg.trim().parse::<u32>().ok();
                    match call.filter(|c| transfer_offers.contains_key(c)) {
                        Some(call) if cmd == "/decline" => {
                            transfer_offers.remove(&call);
                            Some(C::DeclineTransfer { call })
                        }
                        Some(call) => {
                            transfer_offers.remove(&call);
                            println!("* call {call}: following the transfer – connecting …");
                            let relay = engine.media_relay().and_then(calls::relay_from);
                            let id = next_media;
                            next_media = next_media.wrapping_add(1).max(1);
                            let h = calls::spawn(
                                id,
                                local_ip,
                                relay,
                                None,
                                commscraft_audio_alsa::Config::default(),
                                media_tx.clone(),
                            );
                            handles.insert(id, h);
                            pending_transfer.insert(id, call);
                            None
                        }
                        None => {
                            println!("* /follow N or /decline N – for a transfer that asks");
                            None
                        }
                    }
                }
                "/delegate" => {
                    // PR-21: who may call for us and take our calls.
                    let mut w = arg.split_whitespace();
                    match (w.next(), w.next()) {
                        (Some(a @ ("add" | "remove")), Some(u)) if u.starts_with("sip:") => {
                            Some(C::SetDelegate {
                                uri: u.to_string(),
                                add: a == "add",
                            })
                        }
                        _ => {
                            println!("* /delegate add|remove sip:URI");
                            None
                        }
                    }
                }
                "/team" | "/delegates" => {
                    // RE-71: team or delegate ringing (routing version 2). Members are SIP
                    // addresses; SECONDS after which they ring; `skip` (delegates only): only the
                    // delegates ring.
                    let words: Vec<&str> = arg.split_whitespace().collect();
                    let mut next = forwarding.clone();
                    let team = cmd == "/team";
                    if words.first() == Some(&"off") || words.is_empty() {
                        if team {
                            next.team.clear();
                            next.team_ring = false;
                        } else {
                            next.delegates.clear();
                            next.delegate_ring = false;
                            next.skip_primary = false;
                        }
                    } else {
                        let members: Vec<String> = words
                            .iter()
                            .filter(|w| w.starts_with("sip:"))
                            .map(|w| w.to_string())
                            .collect();
                        let seconds = words.iter().find_map(|w| w.parse::<u32>().ok());
                        if team {
                            next.team = members;
                            next.team_ring = true;
                            next.delegate_ring = false;
                            next.skip_primary = false;
                        } else {
                            next.delegates = members;
                            next.delegate_ring = true;
                            next.team_ring = false;
                            next.skip_primary = words.contains(&"skip");
                        }
                        next.user_seconds = seconds;
                    }
                    match next.preamble() {
                        Ok(_) => {
                            forwarding = next.clone();
                            Some(C::SetForwarding(next))
                        }
                        Err(_) => {
                            println!(
                                "* {cmd} sip:A [sip:B …] [SECONDS]{} | off",
                                if team { "" } else { " [skip]" }
                            );
                            None
                        }
                    }
                }
                "/transfer" => {
                    let mut w = arg.split_whitespace();
                    let call = w.next().and_then(|n| n.parse::<u32>().ok());
                    let target = w.next().unwrap_or("");
                    let to = if let Ok(other) = target.parse::<u32>() {
                        Some(commscraft_session::TransferTarget::Call(other))
                    } else if target.starts_with("sip:") {
                        Some(commscraft_session::TransferTarget::Address(
                            target.to_string(),
                        ))
                    } else {
                        engine
                            .phone_uri(target)
                            .map(commscraft_session::TransferTarget::Address)
                    };
                    match (call, to) {
                        (Some(call), Some(to)) => Some(C::Transfer { call, to }),
                        _ => {
                            println!("* /transfer N sip:URI|number, or /transfer N M to connect call N with call M");
                            None
                        }
                    }
                }
                "/hangup" => arg
                    .parse::<u32>()
                    .ok()
                    .or(current_call)
                    .map(|call| C::Hangup { call }),
                "/dtmf" => {
                    match current_call
                        .and_then(|c| media_of_call.get(&c))
                        .and_then(|m| handles.get(m))
                    {
                        Some(h) => {
                            for d in arg
                                .chars()
                                .filter(|c| commscraft_rtp::dtmf_event(*c).is_some())
                            {
                                let _ = h.tx.send(calls::ToMedia::Dtmf(d));
                            }
                        }
                        None => println!("* no active call"),
                    }
                    None
                }
                "/rate" => {
                    let mut words = arg.split_whitespace();
                    let call = words.next().and_then(|c| c.parse::<u32>().ok());
                    let stars = words.next().and_then(|s| s.parse::<u8>().ok());
                    let text: Vec<&str> = words.collect();
                    match (call, stars) {
                        (Some(call), Some(stars)) if (1..=5).contains(&stars) => {
                            Some(C::RateCall {
                                call,
                                stars,
                                feedback: (!text.is_empty()).then(|| text.join(" ")),
                            })
                        }
                        _ => {
                            println!("* /rate CALL 1-5 [text]");
                            None
                        }
                    }
                }
                "/location" if arg.trim().starts_with("city ") => {
                    let f: Vec<&str> = arg.trim()["city ".len()..]
                        .split(';')
                        .map(str::trim)
                        .collect();
                    match f.as_slice() {
                        [c, s, city] => {
                            match lis_city(
                                &mut engine,
                                tls,
                                trust,
                                services.as_deref(),
                                (&c.to_ascii_uppercase(), s, city),
                                verbose,
                            ) {
                                Ok(l) if l.is_empty() => {
                                    println!("* the location service knows no addresses there")
                                }
                                Ok(l) => {
                                    for (i, loc) in l.iter().enumerate() {
                                        println!("  {} {}", i + 1, loc.summary());
                                    }
                                    println!("* choose one with /location pick N");
                                    city_choices = l;
                                }
                                Err(e) => println!("* location service: {e}"),
                            }
                        }
                        _ => println!("* /location city COUNTRY;STATE(2 characters);CITY"),
                    }
                    None
                }
                "/location" if arg.trim().starts_with("pick ") => {
                    let n = arg.trim()["pick ".len()..].trim().parse::<usize>().ok();
                    match n
                        .and_then(|n| n.checked_sub(1))
                        .and_then(|i| city_choices.get(i))
                    {
                        // Chosen by the user, not found for the network position: Manual (E9-08,
                        // inference).
                        Some(loc) => Some(C::SetLocation(Box::new(commscraft_lis::Location {
                            manual: true,
                            ..loc.clone()
                        }))),
                        None => {
                            println!("* /location pick N – after /location city");
                            None
                        }
                    }
                }
                "/location" => {
                    let a = arg.trim();
                    if a.is_empty() || a == "lookup" {
                        match lis_lookup(
                            &mut engine,
                            tls,
                            trust,
                            services.as_deref(),
                            local_ip,
                            verbose,
                        ) {
                            Ok(Some(loc)) => Some(C::SetLocation(Box::new(loc))),
                            Ok(None) => {
                                println!("* no location known here; /location COUNTRY;STATE;CITY;STREET;NUMBER;POSTCODE;ROOM;NAME");
                                None
                            }
                            Err(e) => {
                                println!("* location service: {e}");
                                None
                            }
                        }
                    } else {
                        // Typed by the user: method Manual (not verified by the network, E9-04).
                        let f: Vec<String> = a
                            .split(';')
                            .map(|x| x.trim().chars().take(128).collect())
                            .collect();
                        let g = |i: usize| f.get(i).cloned().unwrap_or_default();
                        if f.len() < 5 || g(0).len() != 2 {
                            println!("* /location COUNTRY(2 letters);STATE;CITY;STREET;NUMBER[;POSTCODE;ROOM;NAME]");
                            None
                        } else {
                            Some(C::SetLocation(Box::new(commscraft_lis::Location {
                                country: g(0).to_ascii_uppercase(),
                                a1: g(1),
                                a3: g(2),
                                rd: g(3),
                                hno: g(4),
                                pc: g(5),
                                loc: g(6),
                                nam: g(7),
                                manual: true,
                                ..Default::default()
                            })))
                        }
                    }
                }
                "/park" => {
                    let call = arg.trim().parse::<u32>().ok().or(current_call);
                    match call {
                        Some(call) => Some(C::ParkCall { call }),
                        None => {
                            println!("* no active call");
                            None
                        }
                    }
                }
                "/hold" | "/resume" => match current_call {
                    Some(call) => {
                        let hold = cmd == "/hold";
                        if let Some(h) = media_of_call.get(&call).and_then(|m| handles.get(m)) {
                            let _ = h.tx.send(calls::ToMedia::Hold(hold));
                        }
                        if let Some(h) = video_of_call.get(&call).and_then(|v| video_handles.get(v))
                        {
                            let _ = h.tx.send(video_call::ToVideo::Hold(hold));
                        }
                        Some(C::Hold { call, hold })
                    }
                    None => {
                        println!("* no active call");
                        None
                    }
                },
                "/mute" | "/unmute" => {
                    if let Some(h) = current_call
                        .and_then(|c| media_of_call.get(&c))
                        .and_then(|m| handles.get(m))
                    {
                        let _ = h.tx.send(calls::ToMedia::Mute(cmd == "/mute"));
                    }
                    None
                }
                "/group" => Some(C::StartGroup {
                    invite: arg.split_whitespace().map(str::to_string).collect(),
                }),
                "/invite" => match current {
                    Some(c) => Some(C::InviteToGroup {
                        conversation: c,
                        to: arg,
                    }),
                    None => {
                        println!("* no conversation");
                        None
                    }
                },
                "/join" => {
                    let invited = arg
                        .parse::<usize>()
                        .ok()
                        .and_then(|n| n.checked_sub(1))
                        .and_then(|i| invitations.get(i))
                        .cloned();
                    let focus = if let Some(f) = invited {
                        Some(f)
                    } else if arg.starts_with("sip:") {
                        Some(arg.clone())
                    } else if arg.starts_with("https://") {
                        match meeting_focus(tls, &arg) {
                            Ok(f) => Some(f),
                            Err(e) => {
                                println!("* meeting link not usable: {e}");
                                None
                            }
                        }
                    } else {
                        println!("* /join N, a conference address or a https meeting link");
                        None
                    };
                    focus.map(|focus| C::JoinConference { focus })
                }
                "/confaudio" | "/confvideo" => match current {
                    Some(conversation) => {
                        let relay = engine.media_relay().and_then(calls::relay_from);
                        let id = next_media;
                        next_media = next_media.wrapping_add(1).max(1);
                        let h = calls::spawn(
                            id,
                            local_ip,
                            relay,
                            None,
                            commscraft_audio_alsa::Config::default(),
                            media_tx.clone(),
                        );
                        handles.insert(id, h);
                        // CA-24: the language given, else the one of the environment.
                        let language = Some(arg.trim())
                            .filter(|l| !l.is_empty())
                            .map(str::to_string)
                            .or_else(language_from_env)
                            .filter(|l| commscraft_conf::valid_language_tag(l));
                        pending_conf.insert(id, (conversation, language));
                        if cmd == "/confvideo" {
                            let vid = next_media;
                            next_media = next_media.wrapping_add(1).max(1);
                            let relay = engine.media_relay().and_then(calls::relay_from);
                            video_handles.insert(
                                vid,
                                video_call::spawn(
                                    vid,
                                    local_ip,
                                    relay,
                                    camera.clone(),
                                    video_tx.clone(),
                                ),
                            );
                            video_for_audio.insert(id, vid);
                            conf_video.insert(conversation, (vid, None));
                        }
                        println!("* preparing the meeting audio …");
                        None
                    }
                    None => {
                        println!("* no meeting");
                        None
                    }
                },
                "/announcements" | "/dialout" => {
                    use commscraft_session::ConferenceAction as A;
                    let a = arg.trim();
                    let action = match (cmd, a) {
                        ("/announcements", "on" | "off") => Some(A::Announcements(a == "on")),
                        ("/dialout", n) if !n.is_empty() => Some(A::DialOut {
                            number: n.to_string(),
                        }),
                        _ => None,
                    };
                    match action {
                        Some(action) => current.map(|conversation| C::ConferenceControl {
                            conversation,
                            action,
                        }),
                        None => {
                            println!("* /announcements on|off  /dialout NUMBER");
                            None
                        }
                    }
                }
                "/admit" | "/deny" | "/promote" | "/demote" | "/lock" | "/unlock" | "/remove"
                | "/muteuser" | "/unmuteuser" | "/muteall" | "/unmuteall" => {
                    use commscraft_session::ConferenceAction as A;
                    let user = arg.trim().to_string();
                    let needs_user = !matches!(
                        cmd,
                        "/admit" | "/lock" | "/unlock" | "/muteall" | "/unmuteall"
                    );
                    if needs_user && !user.starts_with("sip:") {
                        println!("* {cmd} sip:URI");
                        None
                    } else {
                        let action = match cmd {
                            "/admit" => A::Lobby {
                                users: if user.is_empty() {
                                    Vec::new()
                                } else {
                                    vec![user]
                                },
                                admit: true,
                            },
                            "/deny" => A::Lobby {
                                users: vec![user],
                                admit: false,
                            },
                            "/promote" | "/demote" => A::Role {
                                user,
                                presenter: cmd == "/promote",
                            },
                            "/lock" | "/unlock" => A::Lock(cmd == "/lock"),
                            "/remove" => A::Remove { user },
                            "/muteuser" | "/unmuteuser" => A::MuteUser {
                                user,
                                mute: cmd == "/muteuser",
                            },
                            _ => A::MuteAudience(cmd == "/muteall"),
                        };
                        current.map(|conversation| C::ConferenceControl {
                            conversation,
                            action,
                        })
                    }
                }
                "/options" => {
                    use commscraft_conf::schedule::Admission;
                    let known = current
                        .and_then(|c| meeting_settings.get(&c))
                        .and_then(|s| s.options());
                    let mut o = known.unwrap_or(commscraft_conf::MeetingOptions {
                        admission: Admission::OpenAuthenticated,
                        autopromote: commscraft_conf::AUTOPROMOTE_COMPANY,
                        pstn_lobby_bypass: false,
                    });
                    let words: Vec<&str> = arg.split_whitespace().collect();
                    let mut ok = !words.is_empty();
                    let mut i = 0;
                    while ok && i < words.len() {
                        match (words.get(i).copied(), words.get(i + 1).copied()) {
                            (Some(a), _) if Admission::parse(&admission_word(a)).is_some() => {
                                o.admission =
                                    Admission::parse(&admission_word(a)).unwrap_or(o.admission);
                                i += 1;
                            }
                            (Some("promote"), Some(p)) => {
                                match promote_value(p) {
                                    Some(v) => o.autopromote = v,
                                    None => ok = false,
                                }
                                i += 2;
                            }
                            (Some("phone"), Some(p @ ("bypass" | "lobby"))) => {
                                o.pstn_lobby_bypass = p == "bypass";
                                i += 2;
                            }
                            _ => ok = false,
                        }
                    }
                    if !ok {
                        println!("* /options [open|closed|anonymous] [promote none|company|everyone] [phone bypass|lobby]");
                        None
                    } else {
                        current.map(|conversation| C::ConferenceControl {
                            conversation,
                            action: commscraft_session::ConferenceAction::Options(o),
                        })
                    }
                }
                "/schedule" if arg.trim() == "static" => Some(C::ScheduleStaticMeeting),
                "/schedule" => {
                    let mut words = arg.split_whitespace();
                    let minutes = words.next().and_then(|m| m.parse::<u64>().ok());
                    let rest: Vec<&str> = words.collect();
                    match (minutes, meeting_words(&rest)) {
                        (Some(minutes), Some((admission, extras, subject))) => {
                            Some(C::ScheduleMeeting {
                                subject,
                                minutes,
                                admission,
                                extras,
                            })
                        }
                        _ => {
                            println!("* /schedule MINUTES [OPTIONS] SUBJECT  |  /schedule static\n  {MEETING_OPTIONS}");
                            None
                        }
                    }
                }
                "/changemeeting" => {
                    let mut words = arg.split_whitespace();
                    let id = words.next().map(str::to_string);
                    let minutes = words.next().and_then(|m| m.parse::<u64>().ok());
                    let rest: Vec<&str> = words.collect();
                    match (id, minutes, meeting_words(&rest)) {
                        (
                            Some(conference_id),
                            Some(minutes),
                            Some((admission, extras, subject)),
                        ) => Some(C::ChangeMeeting {
                            conference_id,
                            subject,
                            minutes,
                            admission,
                            extras,
                        }),
                        _ => {
                            println!("* /changemeeting ID MINUTES [OPTIONS] SUBJECT\n  {MEETING_OPTIONS}");
                            None
                        }
                    }
                }
                "/meetings" => Some(C::ListMeetings {
                    static_meetings: arg.trim() == "static",
                }),
                "/meeting" | "/cancelmeeting" => {
                    let mut w = arg.split_whitespace();
                    let conference_id = w.next().unwrap_or("").to_string();
                    let static_meeting = w.next() == Some("static");
                    if conference_id.is_empty() {
                        println!(
                            "* {cmd} CONFERENCE-ID (see /meetings){}",
                            if cmd == "/cancelmeeting" {
                                " [static]"
                            } else {
                                ""
                            }
                        );
                        None
                    } else if cmd == "/meeting" {
                        Some(C::ShowMeeting { conference_id })
                    } else {
                        Some(C::CancelMeeting {
                            conference_id,
                            static_meeting,
                        })
                    }
                }
                "/confmute" | "/confunmute" => current.map(|conversation| C::ConferenceMute {
                    conversation,
                    mute: cmd == "/confmute",
                }),
                "/search" => {
                    match search(&mut engine, tls, trust, services.as_deref(), &arg, verbose) {
                        Ok(found) if found.is_empty() => println!("* nothing found"),
                        Ok(found) => {
                            for f in found {
                                println!(
                                    "  {:<40} {}  {}",
                                    f.sip_uri.as_deref().unwrap_or("(no SIP address)"),
                                    f.display_name.as_deref().unwrap_or(""),
                                    [f.title, f.department, f.company, f.phone]
                                        .into_iter()
                                        .flatten()
                                        .filter(|s| !s.is_empty())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                );
                            }
                        }
                        Err(e) => println!("* search failed: {e}"),
                    }
                    None
                }
                "/to" => {
                    match arg.parse::<u32>().ok().filter(|n| peers.contains_key(n)) {
                        Some(n) => current = Some(n),
                        None => println!("* no such conversation"),
                    }
                    None
                }
                "/end" => arg
                    .parse::<u32>()
                    .ok()
                    .or(current)
                    .map(|c| C::EndConversation { conversation: c }),
                c if c.starts_with('/') => {
                    println!("* unknown command");
                    None
                }
                _ => match current {
                    Some(c) => Some(C::SendText {
                        conversation: c,
                        text: line.clone(),
                    }),
                    None => {
                        println!("* no conversation – start one with /chat sip:user@domain");
                        None
                    }
                },
            };
            if let Some(c) = command {
                outputs.extend(engine.command(c, now()));
            }
        }
    }
}

/// One one-time presence query (PR-17). `target` is the Request-URI: the own AOR or a pool
/// GRUU for a batch, the contact itself for a single SUBSCRIBE (`single`, To = contact).
fn query_presence(
    s: &mut Session,
    reg: &mut Registrar<OsRandom>,
    a: &Args,
    target: &str,
    uris: &[String],
    single: bool,
) -> Result<
    (
        Vec<commscraft_presence::ResourceState>,
        Vec<commscraft_presence::ContactPresence>,
    ),
    String,
> {
    use commscraft_presence as pres;
    let to = if single {
        target.to_string()
    } else {
        a.aor.clone()
    };
    let mut req = s.request("SUBSCRIBE", &to);
    if !single {
        // A pool GRUU (checked when parsed) as Request-URI, To stays the own AOR (§3.4.5.1.3.1).
        if let commscraft_sip_msg::StartLine::Request { uri, .. } = &mut req.start {
            *uri = target.to_string();
        }
        pres::batch_headers(&mut req, true);
    } else {
        pres::single_headers(&mut req, true);
    }
    let mut id = [0u8; 4];
    s.rng.fill(&mut id);
    req.body = pres::batch_body(&a.aor, uris, u32::from_be_bytes(id) & 0x7fff_ffff)
        .map_err(|e| format!("{e:?}"))?
        .into_bytes();
    reg.sign_request(&mut req).map_err(|e| format!("{e:?}"))?;
    let answer = s.transact(reg, &req)?;
    if answer.status_code() != Some(200) {
        println!(
            "presence query to {target}: answer {}",
            answer.status_code().unwrap_or(0)
        );
        return Ok((Vec::new(), Vec::new()));
    }
    if answer.body.is_empty() {
        println!("presence query to {target}: no document");
        return Ok((Vec::new(), Vec::new()));
    }
    match pres::parse_batch(&answer) {
        Ok(r) => Ok(r),
        Err(e) => {
            println!("presence query to {target}: unusable answer ({e:?})");
            Ok((Vec::new(), Vec::new()))
        }
    }
}

/// `presence`: contact list, presence of the contacts, optionally the own state (M2).
fn presence_actions(
    s: &mut Session,
    reg: &mut Registrar<OsRandom>,
    a: &Args,
) -> Result<(), String> {
    use commscraft_presence as pres;
    // Contact list (SIP-07): a real subscription, as in MS-PRES §4.14 and MS-SIP §3.7 – the
    // list comes piggybacked in the 200 OK or in the first NOTIFY; afterwards the subscription is
    // ended again (in-dialog SUBSCRIBE with Expires: 0, RFC 3265 §3.3.4).
    let mut req = s.request("SUBSCRIBE", &a.aor.clone());
    pres::contacts_headers(&mut req, false);
    reg.sign_request(&mut req).map_err(|e| format!("{e:?}"))?;
    let answer = s.transact(reg, &req)?;
    let mut document = None;
    let mut dialog = None;
    if answer.status_code() == Some(200) {
        if !answer.body.is_empty() {
            document = Some(answer.body.clone());
        }
        match Dialog::from_response(&req, &answer) {
            Ok(d) => dialog = Some(d),
            Err(e) => println!("contact list: no usable dialog ({e:?})"),
        }
    } else {
        println!("contact list: answer {}", answer.status_code().unwrap_or(0));
    }
    if let Some(d) = &dialog {
        if document.is_none() {
            if let Some(notify) = s.wait_peer_request(reg, d, 10)? {
                if !notify.body.is_empty() {
                    document = Some(notify.body.clone());
                }
            }
        }
    }
    if let Some(mut d) = dialog {
        let mut end = d.request("SUBSCRIBE");
        let via = s.via();
        end.headers.insert(
            0,
            commscraft_sip_msg::Header {
                name: "Via".into(),
                value: via,
            },
        );
        end.push_header("Max-Forwards", "70");
        end.push_header("Contact", &format!("<{}>", s.gruu));
        end.push_header("User-Agent", &user_agent());
        end.push_header("Event", "vnd-microsoft-roaming-contacts");
        end.push_header("Expires", "0");
        reg.sign_request(&mut end).map_err(|e| format!("{e:?}"))?;
        match s.transact(reg, &end) {
            Ok(r) => {
                if s.verbose {
                    eprintln!(
                        "contact list subscription ended ({})",
                        r.status_code().unwrap_or(0)
                    );
                }
            }
            Err(e) => eprintln!("ending the contact list subscription: {e}"),
        }
        // A final NOTIFY (subscription-state: terminated) may follow; it is answered if it comes.
        let _ = s.wait_peer_request(reg, &d, 2)?;
    }
    let list = match document {
        Some(body) => {
            let body = std::str::from_utf8(&body).map_err(|_| "contact list is not UTF-8")?;
            pres::parse_contact_list(body).map_err(|e| format!("contact list: {e:?}"))?
        }
        None => {
            println!("contact list: no document received");
            pres::ContactList::default()
        }
    };
    println!(
        "contact list: {} contacts in {} groups{}",
        list.contacts.len(),
        list.groups.len(),
        list.ucs_mode
            .as_deref()
            .map(|m| format!(" (unified contact store: {m})"))
            .unwrap_or_default()
    );
    if !list.skipped.is_empty() {
        println!(
            "  skipped {} entries with unusable URIs",
            list.skipped.len()
        );
    }

    // Presence of the contacts (PR-14, PR-17): one-time batch queries of at most MAX_BATCH to the
    // own AOR; contacts the server answers with "resubscribe" are asked again at the pool named
    // in poolFqdn (batch) or, without poolFqdn, one by one (§3.4.5.1.3.1, p. 102; §4.6).
    let uris: Vec<String> = list
        .contacts
        .iter()
        .map(|c| c.uri.clone())
        .take(a.max_contacts)
        .collect();
    let mut results: Vec<pres::ContactPresence> = Vec::new();
    let mut open_states: Vec<pres::ResourceState> = Vec::new();
    for chunk in uris.chunks(pres::MAX_BATCH) {
        let (states, contacts) = query_presence(s, reg, a, &a.aor.clone(), chunk, false)?;
        results.extend(contacts);
        open_states.extend(states.into_iter().filter(|st| st.state != "active"));
    }
    let mut pools: Vec<(String, Vec<String>)> = Vec::new();
    let mut singles: Vec<String> = Vec::new();
    let mut remaining: Vec<pres::ResourceState> = Vec::new();
    for st in open_states {
        if st.state != "resubscribe" {
            remaining.push(st);
            continue;
        }
        let Ok(uri) = pres::checked_sip_uri(&st.uri) else {
            continue;
        };
        match &st.pool_fqdn {
            Some(pool) => match pools.iter_mut().find(|(p, _)| p == pool) {
                Some((_, v)) => v.push(uri),
                None => pools.push((pool.clone(), vec![uri])),
            },
            None => singles.push(uri),
        }
    }
    for (pool, members) in &pools {
        if s.verbose {
            eprintln!("asking pool {pool} for {} contacts", members.len());
        }
        for chunk in members.chunks(pres::MAX_BATCH) {
            let (states, contacts) = query_presence(s, reg, a, pool, chunk, false)?;
            results.extend(contacts);
            remaining.extend(states.into_iter().filter(|st| st.state != "active"));
        }
    }
    for uri in &singles {
        let (states, contacts) = query_presence(s, reg, a, uri, std::slice::from_ref(uri), true)?;
        results.extend(contacts);
        remaining.extend(states.into_iter().filter(|st| st.state != "active"));
    }
    for c in &results {
        let name = list
            .contacts
            .iter()
            .find(|x| x.uri.eq_ignore_ascii_case(&c.uri))
            .map(|x| x.name.clone())
            .filter(|n| !n.is_empty())
            .or_else(|| c.display_name.clone())
            .unwrap_or_default();
        println!(
            "  {:<14} {:<40} {}{}",
            format!("{:?}", c.class()),
            c.uri,
            name,
            c.note
                .as_deref()
                .map(|n| format!("  \"{n}\""))
                .unwrap_or_default()
        );
    }
    for st in &remaining {
        println!(
            "  {:<14} {:<40} ({})",
            st.state,
            st.uri,
            st.reason.as_deref().unwrap_or("")
        );
    }

    // Own state (PR-11), only on request and scoped to this endpoint (PR-02).
    if let Some(status) = a.set_status {
        let endpoint = reg.instance().to_string();
        let mut versions = [0u32, 0u32];
        for attempt in 0..2 {
            let mut req = s.request("SERVICE", &a.aor.clone());
            req.push_header("Content-Type", "application/msrtc-category-publish+xml");
            req.body = pres::publish_state_body(&a.aor, status, versions, Some(&endpoint))
                .map_err(|e| format!("{e:?}"))?
                .into_bytes();
            reg.sign_request(&mut req).map_err(|e| format!("{e:?}"))?;
            let answer = s.transact(reg, &req)?;
            match answer.status_code() {
                Some(200) => {
                    println!("own state set to {status:?} (until this client signs out)");
                    break;
                }
                // PR-05: version conflict, retry once with the server's versions.
                Some(409) if attempt == 0 => {
                    let body = std::str::from_utf8(&answer.body).unwrap_or("");
                    let conflicts = pres::parse_fault(body).map_err(|e| format!("409: {e:?}"))?;
                    for c in conflicts {
                        if let Some(v) = versions.get_mut((c.index as usize).saturating_sub(1)) {
                            *v = c.cur_version;
                        }
                    }
                }
                other => {
                    println!("publishing the state failed: {}", other.unwrap_or(0));
                    break;
                }
            }
        }
    }
    Ok(())
}

/// `audio-check`: local test of microphone and speaker through the codecs, no network.
/// Plays a 1020 Hz tone for one second, records for three seconds and plays the recording back;
/// all audio goes through encoder and decoder of the chosen codec.
fn audio_check() -> Result<(), String> {
    use commscraft_audio_alsa::{Capture, Config, Playback};
    let mut it = std::env::args().skip(2);
    let mut cfg = Config::default();
    let mut g711 = false;
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--device" => cfg.device = it.next().ok_or("--device needs a name")?,
            "--codec" => match it.next().as_deref() {
                Some("g722") => g711 = false,
                Some("pcma") => g711 = true,
                _ => return Err("--codec g722 or pcma".into()),
            },
            other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
        }
    }
    if g711 {
        cfg.rate = 8000;
    }
    let codec_round_trip = |pcm: &[i16],
                            out: &mut Vec<i16>,
                            enc: &mut commscraft_g722::Encoder,
                            dec: &mut commscraft_g722::Decoder| {
        let mut payload = Vec::new();
        if g711 {
            commscraft_g711::Law::Alaw.encode(pcm, &mut payload);
            commscraft_g711::Law::Alaw.decode(&payload, out);
        } else {
            enc.encode(pcm, &mut payload);
            dec.decode(&payload, out);
        }
    };
    let (mut enc, mut dec) = (
        commscraft_g722::Encoder::new(),
        commscraft_g722::Decoder::new(),
    );
    let frame = cfg.frame_samples();
    let frames_per_second = (1000 / cfg.frame_ms) as usize;
    println!(
        "audio device {} at {} Hz, codec {}",
        cfg.device,
        cfg.rate,
        if g711 { "PCMA (G.711)" } else { "G.722" }
    );
    let mut speaker = Playback::open(&cfg).map_err(|e| e.to_string())?;
    println!("1/3 tone (1 s) …");
    let mut out = Vec::new();
    for f in 0..frames_per_second {
        let pcm: Vec<i16> = (0..frame)
            .map(|i| {
                let t = (f * frame + i) as f64 / f64::from(cfg.rate);
                (6000.0 * (2.0 * std::f64::consts::PI * 1020.0 * t).sin()) as i16
            })
            .collect();
        out.clear();
        codec_round_trip(&pcm, &mut out, &mut enc, &mut dec);
        speaker.write(&out).map_err(|e| e.to_string())?;
    }
    speaker.drain().map_err(|e| e.to_string())?;
    println!("2/3 speak now (3 s) …");
    let mut mic = Capture::open(&cfg).map_err(|e| e.to_string())?;
    let mut recording = Vec::new();
    let mut buf = Vec::new();
    let mut peak = 0i16;
    for _ in 0..frames_per_second * 3 {
        mic.read_frame(&mut buf).map_err(|e| e.to_string())?;
        peak = buf.iter().fold(peak, |p, s| p.max(s.saturating_abs()));
        codec_round_trip(&buf, &mut recording, &mut enc, &mut dec);
    }
    drop(mic);
    println!(
        "   input peak {:.1} dBFS",
        20.0 * (f64::from(peak.max(1)) / 32768.0).log10()
    );
    println!("3/3 playback (3 s) …");
    speaker.write(&recording).map_err(|e| e.to_string())?;
    speaker.drain().map_err(|e| e.to_string())?;
    println!("done");
    Ok(())
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("audio-check") {
        let code = match audio_check() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                2
            }
        };
        std::process::exit(code);
    }
    let code = match parse_args().and_then(run) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            2
        }
    };
    std::process::exit(code);
}

/// The announcement language from the environment (CA-24): `de_DE.UTF-8` → `de-DE`; `C` and
/// `POSIX` mean none.
fn language_from_env() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .and_then(|v| posix_language(&v))
}

fn posix_language(v: &str) -> Option<String> {
    let base = v.split(['.', '@']).next().unwrap_or("");
    if base.is_empty() || base == "C" || base == "POSIX" {
        return None;
    }
    let tag = base.replace('_', "-");
    commscraft_conf::valid_language_tag(&tag).then_some(tag)
}

/// What the user is told after a location lookup (RG-51, [MS-SIPREGE] §2.2.2.5.7, p. 57):
/// without a location, `LocationRequired` "yes" asks for one, "disclaimer" also shows the
/// administrator's EmergencyServiceDisclaimer (a terminal cannot be clicked through, so it
/// follows at once – inference), "no" only notes it.
fn lis_messages(
    r: &Result<Option<commscraft_lis::Location>, String>,
    policy: Option<&commscraft_provisioning::LocationPolicy>,
) -> Vec<String> {
    use commscraft_provisioning::LocationRequired as Req;
    let mut lines = match r {
        Ok(Some(_)) => return Vec::new(),
        Ok(None) => vec!["* the location service knows no location here".to_string()],
        Err(e) => vec![format!("* location service: {e}")],
    };
    let required = policy.map(|p| p.location_required).unwrap_or_default();
    if required != Req::No {
        lines.push(
            "! emergency services need your location: set it with /location COUNTRY;STATE;CITY;STREET;NUMBER;POSTCODE;ROOM;NAME"
                .to_string(),
        );
    }
    if required == Req::Disclaimer {
        if let Some(d) = policy.and_then(|p| p.disclaimer.as_deref()) {
            lines.push(format!("! {d}"));
        }
    }
    lines
}

/// When to look the location up again: LocationRefreshInterval hours, kept within 1 … 168
/// (design); none without an interval.
fn lis_refresh_at(
    policy: Option<&commscraft_provisioning::LocationPolicy>,
    now: u64,
) -> Option<u64> {
    let hours = policy.and_then(|p| p.refresh_hours).filter(|h| *h > 0)?;
    Some(now.saturating_add(u64::from(hours.min(168)) * 3600))
}

/// Issue #67: a transfer is followed without asking only to a SIP address (no phone number) in
/// the user's own SIP domain. Issue #70: the user part must contain a letter – a numeric user
/// (`sip:900123@…`) may be a phone number in the server's dial plan.
fn auto_follow(target: &str, own_aor: &str) -> bool {
    let domain = |u: &str| {
        u.split_once('@').map(|(_, d)| {
            d.split([';', '?', '>', ':'])
                .next()
                .unwrap_or("")
                .to_ascii_lowercase()
        })
    };
    let lower = target.to_ascii_lowercase();
    let user = lower
        .trim_start_matches("sip:")
        .split('@')
        .next()
        .unwrap_or("");
    lower.starts_with("sip:")
        && user.chars().any(|c| c.is_ascii_alphabetic())
        && !lower.contains(";user=phone")
        && !lower.trim_start_matches("sip:").starts_with('+')
        && domain(target).is_some()
        && domain(target) == domain(own_aor)
}

const MEETING_OPTIONS: &str = "OPTIONS: open|closed|anonymous, promote none|company|everyone, phone bypass|lobby, locked, presenter sip:URI, attendee sip:URI";

/// Meeting options before the subject of /schedule and /changemeeting (PV-15): admission policy,
/// automatic promotion, lobby bypass for phone callers, locked, invitees with role. The first
/// word that is no option starts the subject; a malformed option or an empty subject is `None`.
fn meeting_words(
    words: &[&str],
) -> Option<(
    commscraft_conf::schedule::Admission,
    commscraft_conf::schedule::Extras,
    String,
)> {
    use commscraft_conf::schedule::{Admission, Extras};
    let mut admission = Admission::OpenAuthenticated;
    let mut x = Extras::default();
    let mut i = 0;
    loop {
        match (words.get(i).copied(), words.get(i + 1).copied()) {
            (Some(w @ ("open" | "closed" | "anonymous")), _) => {
                admission = Admission::parse(&admission_word(w))?;
                i += 1;
            }
            (Some("promote"), Some(p)) => {
                x.autopromote = Some(promote_value(p)?);
                i += 2;
            }
            (Some("phone"), Some(p @ ("bypass" | "lobby"))) => {
                x.pstn_lobby_bypass = Some(p == "bypass");
                i += 2;
            }
            (Some("locked"), _) => {
                x.locked = Some(true);
                i += 1;
            }
            (Some(r @ ("presenter" | "attendee")), Some(u)) if u.starts_with("sip:") => {
                x.users.push((u.to_string(), r == "presenter"));
                i += 2;
            }
            (Some("promote" | "phone" | "presenter" | "attendee"), _) => return None,
            _ => break,
        }
    }
    let subject = words.get(i..)?.join(" ");
    (!subject.is_empty()).then_some((admission, x, subject))
}

/// `open`/`closed`/`anonymous` as admission policy names (CB-42).
fn admission_word(w: &str) -> String {
    match w {
        "open" => "openAuthenticated".into(),
        "closed" => "closedAuthenticated".into(),
        other => other.into(),
    }
}

/// Automatic promotion by name (CB-42, [MS-CONFBAS] §2.2.2.3).
fn promote_value(w: &str) -> Option<u32> {
    match w {
        "none" => Some(0),
        "company" => Some(commscraft_conf::AUTOPROMOTE_COMPANY),
        "everyone" => Some(commscraft_conf::AUTOPROMOTE_EVERYONE),
        _ => None,
    }
}

fn promote_name(v: u32) -> &'static str {
    if v & commscraft_conf::AUTOPROMOTE_EVERYONE != 0 {
        "everyone"
    } else if v & commscraft_conf::AUTOPROMOTE_COMPANY != 0 {
        "people from the company"
    } else {
        "nobody"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// E9-03: the subnet for the location request from `/proc/net/route` (values in native byte
    /// order, here as printed on a little-endian machine); the most specific route wins, the
    /// default route never matches.
    #[test]
    #[cfg(target_endian = "little")]
    fn subnet_from_route_table() {
        let table =
            "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n\
eth0\t00000000\t016433C6\t0003\t0\t0\t100\t00000000\t0\t0\t0\n\
eth0\t006433C6\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0\n\
wlan0\t007100CB\t00000000\t0001\t0\t0\t426\t00FFFFFF\t0\t0\t0\n\
tun0\t000000CB\t00000000\t0001\t0\t0\t500\t000000FF\t0\t0\t0\n";
        let ip = |s: &str| s.parse::<std::net::IpAddr>().unwrap();
        assert_eq!(subnet_in(table, ip("203.0.113.7")), Some(ip("203.0.113.0")));
        assert_eq!(
            subnet_in(table, ip("198.51.100.40")),
            Some(ip("198.51.100.0"))
        );
        assert_eq!(subnet_in(table, ip("203.7.7.7")), Some(ip("203.0.0.0")));
        assert_eq!(
            subnet_in(table, ip("192.0.2.1")),
            None,
            "only the default route"
        );
        assert_eq!(subnet_in(table, ip("2001:db8::1")), None);
        assert_eq!(subnet_in("garbage", ip("203.0.113.7")), None);
    }

    /// PV-15: options before the subject; the first other word starts the subject.
    #[test]
    fn meeting_option_words() {
        use commscraft_conf::schedule::Admission;
        let w: Vec<&str> = "closed promote company phone bypass locked presenter sip:bob@contoso.com attendee sip:carol@contoso.com Plan for open questions".split(' ').collect();
        let (a, x, subject) = meeting_words(&w).unwrap();
        assert_eq!(a, Admission::ClosedAuthenticated);
        assert_eq!(x.autopromote, Some(commscraft_conf::AUTOPROMOTE_COMPANY));
        assert_eq!((x.pstn_lobby_bypass, x.locked), (Some(true), Some(true)));
        assert_eq!(x.users.len(), 2);
        assert_eq!(subject, "Plan for open questions");
        assert!(meeting_words(&["promote", "boss", "Plan"]).is_none());
        assert!(meeting_words(&["presenter", "bob", "Plan"]).is_none());
        assert!(meeting_words(&["open"]).is_none());
        assert_eq!(
            meeting_words(&["Plan"]).unwrap().0,
            Admission::OpenAuthenticated
        );
    }

    /// CA-24: POSIX locale names become language tags.
    #[test]
    fn language_from_locale() {
        assert_eq!(posix_language("de_DE.UTF-8").as_deref(), Some("de-DE"));
        assert_eq!(posix_language("en_US@euro").as_deref(), Some("en-US"));
        assert_eq!(posix_language("C.UTF-8"), None);
        assert_eq!(posix_language("POSIX"), None);
        assert_eq!(posix_language("x;q=1"), None);
    }

    /// RG-51: messages after a lookup per LocationRequired; the refresh interval is bounded.
    #[test]
    fn location_lookup_messages() {
        use commscraft_provisioning::{LocationPolicy, LocationRequired};
        let mut p = LocationPolicy {
            enabled: true,
            location_required: LocationRequired::Disclaimer,
            disclaimer: Some("Without a location emergency calls may be delayed.".into()),
            refresh_hours: Some(4),
            ..LocationPolicy::default()
        };
        let none: Result<Option<commscraft_lis::Location>, String> = Ok(None);
        let m = lis_messages(&none, Some(&p));
        assert_eq!(m.len(), 3);
        assert!(m[2].contains("may be delayed"));
        p.location_required = LocationRequired::Yes;
        assert_eq!(lis_messages(&none, Some(&p)).len(), 2);
        p.location_required = LocationRequired::No;
        assert_eq!(lis_messages(&none, Some(&p)).len(), 1);
        let found = Ok(Some(commscraft_lis::Location::default()));
        assert!(lis_messages(&found, Some(&p)).is_empty());
        assert_eq!(lis_refresh_at(Some(&p), 100), Some(100 + 4 * 3600));
        p.refresh_hours = Some(100_000);
        assert_eq!(lis_refresh_at(Some(&p), 0), Some(168 * 3600));
        p.refresh_hours = Some(0);
        assert_eq!(lis_refresh_at(Some(&p), 0), None);
    }

    /// Issue #67: only own-domain SIP targets are followed without asking.
    #[test]
    fn transfer_auto_follow() {
        let me = "sip:alice@contoso.com";
        assert!(auto_follow("sip:bob@contoso.com", me));
        assert!(auto_follow(
            "sip:bob@CONTOSO.com;opaque=user:epid:x;gruu",
            me
        ));
        assert!(!auto_follow("sip:+4940123456@contoso.com;user=phone", me));
        assert!(!auto_follow("sip:+4940123456@contoso.com", me));
        assert!(!auto_follow("sip:eve@fabrikam.com", me));
        assert!(!auto_follow("sip:eve@contoso.com.evil.example", me));
        assert!(!auto_follow("tel:+4940123456", me));
        // Issue #70: numeric users may be dial-plan numbers.
        assert!(!auto_follow("sip:900123@contoso.com", me));
        assert!(!auto_follow("sip:0049-40-123@contoso.com", me));
        assert!(auto_follow("sip:room42@contoso.com", me));
    }
}
