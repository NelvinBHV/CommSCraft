//! `commscraft-cli`: command-line tool for the owner's interop runs.
//!
//! `sign-in` connects to a Skype for Business server, registers with NTLM, fetches in-band
//! provisioning, prints a summary and signs out again. It contacts a server only when started with
//! `--authorized` (rule T-03: interop runs only by the owner, on authorized systems).
//!
//! Security by design (`docs/security.md`): the password is read without echo and kept in a wiped
//! container; verbose output redacts every authentication header; TLS is always verified (system
//! trust store plus an optional enterprise CA file); only TLS targets are used.

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
          /group sip:A sip:B … (group conversation)  /invite sip:URI (into the current group)
          /join N (accept invitation N)  /search TEXT (address book)  /to N (switch conversation)  /end [N]  /quit  – any other line is sent as text.
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

/// Reads the password without echo (via `stty`), or the first line of a non-terminal stdin.
fn read_password() -> Result<Password, String> {
    let stdin = std::io::stdin();
    let tty = stdin.is_terminal();
    if tty {
        eprint!("Password: ");
        let _ = std::io::stderr().flush();
        let ok = std::process::Command::new("stty")
            .arg("-echo")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return Err(
                "cannot switch off terminal echo; pipe the password on standard input instead"
                    .into(),
            );
        }
    }
    let mut line = String::new();
    let read = stdin.lock().read_line(&mut line);
    if tty {
        let _ = std::process::Command::new("stty").arg("echo").status();
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
        "ntlm: timestamp in challenge: {}; MIC/channel bindings sent: {}",
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
                                        eprintln!("{body}");
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
    commscraft_im::invite_body(&mut invite, local_ip);
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
            &mut |challenge| match challenge {
                None => Ok(format!("NTLM {}", base64::encode(&negotiate))),
                Some(ch) => {
                    let token = ch
                        .strip_prefix("NTLM ")
                        .and_then(|b| base64::decode(b.trim()).ok());
                    let answer = token
                        .ok_or_else(|| "no NTLM challenge".to_string())
                        .and_then(|t| engine.http_ntlm(&negotiate, &t, now()));
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
    conn.set_timeout(Duration::from_millis(250))
        .map_err(|e| e.to_string())?;
    let (tx, rx) = mpsc::channel::<String>();
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
                    E::Provisioned(c) => services = Some(c),
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
                    E::Participants {
                        conversation,
                        participants,
                    } => {
                        let names: Vec<String> = participants
                            .iter()
                            .filter(|p| p.in_chat())
                            .map(|p| p.name.clone().unwrap_or_else(|| p.uri.clone()))
                            .collect();
                        println!(
                            "* [{conversation}] in the conversation: {}",
                            names.join(", ")
                        );
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
                "/join" => match arg
                    .parse::<usize>()
                    .ok()
                    .and_then(|n| n.checked_sub(1))
                    .and_then(|i| invitations.get(i))
                {
                    Some(f) => Some(C::JoinConference { focus: f.clone() }),
                    None => {
                        println!("* no such invitation");
                        None
                    }
                },
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
