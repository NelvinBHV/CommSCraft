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
use commscraft_sip_msg::Message;
use commscraft_sipae::Policy;

const USAGE: &str =
    "usage: commscraft-cli sign-in --aor sip:USER@DOMAIN --user NTLM_USER --domain NTLM_DOMAIN \
[--server HOST[:PORT]] [--ca-file PEM] [--epid HEX] [--verbose] [--discover-only] [--trust-domain DOMAIN] [--ntlm-legacy] --authorized

Signs in once, fetches provisioning, prints a summary and signs out.
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
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    if it.next().as_deref() != Some("sign-in") {
        return Err(USAGE.into());
    }
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
    let mut reg =
        Registrar::new(cfg, password, rng).map_err(|e| format!("configuration: {e:?}"))?;
    let mut s = Session {
        conn,
        keepalive: None,
        verbose: a.verbose,
    };
    s.conn
        .set_timeout(Duration::from_secs(1))
        .map_err(|e| e.to_string())?;
    s.send(&reg.start(now()))?;

    let mut prov_call_id: Option<String> = None;
    let mut done_provisioning = false;
    let mut signing_out = false;
    let started = now();
    loop {
        if now().saturating_sub(started) > 120 {
            return Err("gave up after 120 s".into());
        }
        let msg = s.conn.recv().map_err(|e| e.to_string())?;
        let mut events = Vec::new();
        match msg {
            Some(m) => {
                if a.verbose {
                    show("<<<", &m);
                }
                if prov_call_id.is_some()
                    && m.call_id().ok().flatten() == prov_call_id.as_deref()
                    && m.is_response()
                {
                    if m.status_code().is_some_and(|c| c >= 200) {
                        if !reg.verify_message(&m) {
                            eprintln!("provisioning answer not correctly signed - ignored");
                        } else {
                            match prov::from_message(&m) {
                                Ok(Some(p)) => print_provisioning(&p, &user_domain),
                                Ok(None) => println!("provisioning: no piggybacked document (NOTIFY not handled yet)"),
                                Err(e) => println!("provisioning: unusable document ({e:?})"),
                            }
                        }
                        done_provisioning = true;
                    }
                } else {
                    events = reg.on_message(&m, now());
                }
            }
            None => events = reg.on_tick(now()),
        }
        if let Some(k) = &mut s.keepalive {
            if let Some(bytes) = k.on_tick(now()) {
                s.conn.send_raw(bytes).map_err(|e| e.to_string())?;
            }
        }
        for e in events {
            match e {
                Event::Send(m) => {
                    let is_authenticate = m
                        .header_values("Authorization")
                        .any(|v| v.contains("gssapi-data") && !v.contains("gssapi-data=\"\""));
                    if a.verbose && is_authenticate {
                        if let Some(i) = reg.ntlm_info() {
                            print_ntlm_info(i);
                        }
                    }
                    s.send(&m)?
                }
                Event::Registered(r) => {
                    println!(
                        "registered: expires {} s, GRUU {}",
                        r.expires,
                        r.gruu.as_deref().unwrap_or("-")
                    );
                    match r.keepalive_timeout {
                        Some(t) => {
                            println!("keep-alive: every {} s", t * 2 / 3);
                            s.keepalive = Some(KeepAlive::new(t, now()));
                        }
                        None => println!("keep-alive: not negotiated"),
                    }
                    if prov_call_id.is_none() {
                        let mut r2 = OsRandom::new().map_err(|e| e.to_string())?;
                        let call_id = hex(&mut r2, 16);
                        let gruu = r.gruu.clone().unwrap_or_else(|| a.aor.clone());
                        let via = format!(
                            "SIP/2.0/TLS {host}:{};branch=z9hG4bK{}",
                            local.port(),
                            hex(&mut r2, 8)
                        );
                        let tag = hex(&mut r2, 5);
                        let ua = format!("CommSCraft/{}", env!("CARGO_PKG_VERSION"));
                        let mut sub = prov::subscribe(&prov::Request {
                            aor: &a.aor,
                            gruu: &gruu,
                            epid: &epid,
                            from_tag: &tag,
                            call_id: &call_id,
                            cseq: 1,
                            via: &via,
                            user_agent: &ua,
                            subnet: None,
                        });
                        reg.sign_request(&mut sub).map_err(|e| format!("{e:?}"))?;
                        s.send(&sub)?;
                        prov_call_id = Some(call_id);
                    }
                }
                Event::Redirected(t) => {
                    let note = if t.in_user_domain || trust.trusts(&t.host) {
                        ""
                    } else {
                        " (outside your domain: not followed automatically)"
                    };
                    println!(
                        "redirected to home pool {}:{}{note}; run again with --server {}:{}",
                        t.host, t.port, t.host, t.port
                    );
                    return Ok(());
                }
                Event::Unregistered => {
                    println!("signed out");
                    return Ok(());
                }
                Event::Deregistered(d) => {
                    println!("deregistered by the server: {d:?}");
                    return Ok(());
                }
                Event::SurvivableMode => println!("server reports survivable mode"),
                Event::Failed(f) => return Err(format!("sign-in failed: {f:?}")),
            }
        }
        if done_provisioning && !signing_out {
            signing_out = true;
            let m = reg.unregister(now()).map_err(|e| format!("{e:?}"))?;
            s.send(&m)?;
        }
    }
}

fn main() {
    let code = match parse_args().and_then(run) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            2
        }
    };
    std::process::exit(code);
}
