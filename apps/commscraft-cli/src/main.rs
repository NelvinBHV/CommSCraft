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
[--server HOST[:PORT]] [--ca-file PEM] [--epid HEX] [--verbose] --authorized

Signs in once, fetches provisioning, prints a summary and signs out.
--authorized  confirms that you are allowed to test against this server (required).
--server      SIP server; without it the fallback hosts sipinternal/sip/sipexternal.<domain>:443
              are tried (MS-CONMGMT). Autodiscover and DNS SRV lookups are not wired up yet.
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
            "--authorized" => a.authorized = true,
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
        }
    }
    if a.aor.is_empty() || a.user.is_empty() || a.domain.is_empty() {
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
            let scheme = h.value.split_whitespace().next().unwrap_or("").to_string();
            h.value = format!("{scheme} <redacted>");
        }
    }
    if !copy.body.is_empty() {
        copy.body = format!("<{} bytes body>", m.body.len()).into_bytes();
    }
    match copy.to_bytes() {
        Ok(b) => eprintln!("{prefix}\n{}", String::from_utf8_lossy(&b)),
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
    let targets: Vec<(String, u16)> = match &a.server {
        Some(s) => vec![s.clone()],
        None => commscraft_discovery::srv_targets(&user_domain, &[], &[])
            .into_iter()
            .map(|t| (t.host, t.port))
            .collect(),
    };
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
                Event::Send(m) => s.send(&m)?,
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
                    let note = if t.in_user_domain {
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
