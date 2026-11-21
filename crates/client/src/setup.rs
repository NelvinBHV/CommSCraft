//! What every client does before it signs in: the configuration directory, the persistent
//! endpoint ID, the trusted server domains and the discovery of the SIP access points.
//!
//! Security by design: discovery uses HTTPS only and verified TLS; DNS SRV targets outside the
//! user's SIP domain are dropped (CON-16); an Autodiscover target outside the domain is used only
//! when the domain is trusted or the caller's `confirm` says yes (the password goes there).

use commscraft_discovery as d;
use commscraft_net::{OsRandom, TlsClient};
use commscraft_register::Random;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

/// `$XDG_CONFIG_HOME/commscraft` or `~/.config/commscraft`.
pub fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|b| b.join("commscraft"))
}

fn private_dir(dir: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(|e| e.to_string())
}

/// Persistent EPID (RE-02): 64 random bits, stored with mode 0600 under the user's config dir.
pub fn load_or_create_epid(rng: &mut OsRandom) -> Result<String, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let dir = config_dir().ok_or("no HOME")?;
    private_dir(&dir)?;
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

/// Server domains the user trusts for one SIP account (like `TrustModelData` of the Windows
/// client, Microsoft KB 2531068): given ones and those remembered for this account in
/// `~/.config/commscraft/trusted-domains.d/ACCOUNT` (issue #208: a yes given for one account
/// never applies to another; the earlier user-wide `trusted-domains` file is not read).
#[derive(Clone)]
pub struct Trust {
    domains: Vec<String>,
    file: Option<PathBuf>,
}

impl Trust {
    /// The trust of `account` (a SIP address, with or without `sip:`).
    pub fn load(account: &str, extra: &[String]) -> Self {
        Self::load_in(config_dir().as_deref(), account, extra)
    }

    fn load_in(config: Option<&std::path::Path>, account: &str, extra: &[String]) -> Self {
        let file = Self::file_of(config, account);
        let mut domains: Vec<String> = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .map(|s| s.lines().filter_map(commscraft_url::dns_host).collect())
            .unwrap_or_default();
        domains.extend(extra.iter().filter_map(|d| commscraft_url::dns_host(d)));
        Self { domains, file }
    }

    /// The account's file: its address in lower case, encoded one-to-one as a file name.
    fn file_of(config: Option<&std::path::Path>, account: &str) -> Option<PathBuf> {
        let account = account
            .trim()
            .trim_start_matches("sip:")
            .to_ascii_lowercase();
        if !account.contains('@') {
            return None;
        }
        let name = crate::photos::cache_name(&account)?;
        Some(config?.join("trusted-domains.d").join(name))
    }

    pub fn trusts(&self, host: &str) -> bool {
        self.domains
            .iter()
            .any(|d| commscraft_url::in_domain(host, d))
    }

    /// The parent domain offered for a host (sip.hoster.example → hoster.example).
    pub fn domain_of(host: &str) -> String {
        host.split_once('.')
            .map(|(_, d)| d)
            .filter(|d| d.contains('.'))
            .unwrap_or(host)
            .to_string()
    }

    /// Trusts a domain from now on and remembers it in the file (mode 0600). Returns the file
    /// it was written to.
    pub fn remember(&mut self, domain: &str) -> Option<PathBuf> {
        let domain = commscraft_url::dns_host(domain)?;
        self.domains.push(domain.clone());
        let f = self.file.clone()?;
        use std::os::unix::fs::OpenOptionsExt;
        if let Some(dir) = f.parent() {
            private_dir(dir).ok()?;
        }
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .mode(0o600)
            .open(&f)
            .ok()?;
        writeln!(file, "{domain}").ok()?;
        Some(f)
    }
}

/// How discovery reports and asks.
pub struct Discovery<'a> {
    /// A progress line (already free of secrets).
    pub log: &'a mut dyn FnMut(&str),
    /// Prints the Autodiscover answer bodies too.
    pub verbose: bool,
    /// Asked for an Autodiscover target outside the SIP domain that is not trusted yet: host,
    /// user domain, who named it. `true` uses it (and the caller may remember the domain).
    pub confirm: &'a mut dyn FnMut(&str, &str, &str, &mut Trust) -> bool,
}

/// The Autodiscover answer for `aor` (HTTPS only): SIP access points and web links, or `None`
/// when neither lyncdiscoverinternal nor lyncdiscover answered usefully.
pub fn autodiscover(
    tls: &TlsClient,
    aor: &str,
    log: &mut dyn FnMut(&str),
    verbose: bool,
) -> Option<d::Outcome> {
    let ua = format!("CommSCraft/{}", env!("CARGO_PKG_VERSION"));
    let timeout = Duration::from_secs(10);
    let mut ad = d::Autodiscover::new(&d::Config {
        aor: aor.to_string(),
        allow_plain_http: false,
    })
    .ok()?;
    let mut step = ad.start();
    let mut outcome = None;
    loop {
        match step {
            d::Step::Fetch(url) => {
                log(&format!("autodiscover: GET {url}"));
                let response = match commscraft_url::parse_https(&url) {
                    None => d::Response::Failed,
                    Some(u) => match commscraft_net::http::get(tls, &u, d::ACCEPT, &ua, timeout) {
                        Err(e) => {
                            log(&format!("  {e}"));
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
                            log(&format!("  HTTP {}", r.status));
                            match String::from_utf8(r.body.clone()) {
                                Ok(body) => {
                                    if verbose && r.status == 200 {
                                        log(&body);
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
                log(&format!(
                    "autodiscover: access location {:?}",
                    o.access_location
                ));
                outcome = Some(o);
                break;
            }
            d::Step::Failed(e) => {
                log(&format!("autodiscover: {e:?}"));
                break;
            }
        }
    }
    outcome
}

/// The SIP access points in order: Autodiscover (HTTPS only), then DNS SRV (CON-11, TLS only).
pub fn discover(
    tls: &TlsClient,
    aor: &str,
    user_domain: &str,
    trust: &mut Trust,
    io: &mut Discovery<'_>,
) -> Result<Vec<(String, u16)>, String> {
    // An address Autodiscover cannot work with is an error, as before.
    d::Autodiscover::new(&d::Config {
        aor: aor.to_string(),
        allow_plain_http: false,
    })
    .map_err(|e| format!("discovery: {e:?}"))?;
    let outcome = autodiscover(tls, aor, io.log, io.verbose);
    if let Some(o) = &outcome {
        for t in &o.targets {
            let note = if t.in_user_domain {
                ""
            } else if trust.trusts(&t.host) {
                " (outside your domain, trusted)"
            } else {
                " (outside your domain)"
            };
            (io.log)(&format!("  SIP access point {}:{}{note}", t.host, t.port));
        }
        if o.targets.is_empty() {
            (io.log)("  no SIP access point in the Autodiscover answer (interop test IT-16)");
        }
    }
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
                (io.log)(&format!("dns: {name}: {} record(s)", r.len()));
                for s in &r {
                    (io.log)(&format!(
                        "  priority {} weight {} -> {}:{}",
                        s.priority, s.weight, s.target, s.port
                    ));
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
                (io.log)(&format!("dns: {name}: {e}"));
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
            || (t.source == d::Source::Autodiscover
                && (io.confirm)(&t.host, user_domain, &named_by, trust))
        {
            out.push((t.host, t.port));
        } else {
            (io.log)(&format!(
                "skipping {}:{} (outside your SIP domain, not trusted)",
                t.host, t.port
            ));
        }
    }
    Ok(out)
}

/// The focus URI of a meeting link from the join manager (CB-40/CB-41, [MS-CONFBAS] §2.2.6):
/// a GET over verified TLS with the client Accept header, no credentials, no redirects followed.
/// Inference: our own User-Agent is sent, not the product string of §2.2.6.1.
pub fn meeting_focus(
    tls: &commscraft_net::TlsClient,
    link: &str,
    user_agent: &str,
) -> Result<String, String> {
    let url = commscraft_url::parse_https(link).ok_or("not a usable https address")?;
    let r = commscraft_net::http::get(
        tls,
        &url,
        commscraft_conf::MEETING_ACCEPT,
        user_agent,
        std::time::Duration::from_secs(15),
    )
    .map_err(|e| e.to_string())?;
    if r.status != 200 {
        return Err(format!("HTTP {}", r.status));
    }
    let body = String::from_utf8(r.body).map_err(|_| "answer is not UTF-8")?;
    commscraft_conf::parse_meeting(&body).map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod trust_tests {
    use super::Trust;

    /// Issue #208: a domain trusted while signed in as one account is not trusted for another.
    #[test]
    fn trust_is_kept_per_account() {
        let dir = std::env::temp_dir().join(format!("commscraft-trust-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut a = Trust::load_in(Some(&dir), "sip:Alice@contoso.test", &[]);
        assert!(!a.trusts("sip.hoster.example"));
        assert!(a.remember("hoster.example").is_some());
        assert!(a.trusts("sip.hoster.example"));
        // The same account later (other spelling): still trusted.
        let again = Trust::load_in(Some(&dir), "alice@contoso.test", &[]);
        assert!(again.trusts("sip.hoster.example"));
        // Another account of the same Unix user: not trusted.
        let b = Trust::load_in(Some(&dir), "sip:bob@fabrikam.test", &[]);
        assert!(!b.trusts("sip.hoster.example"));
        // Given on the command line: trusted for this run only.
        let c = Trust::load_in(
            Some(&dir),
            "sip:bob@fabrikam.test",
            &["hoster.example".into()],
        );
        assert!(c.trusts("sip.hoster.example"));
        // The earlier user-wide file is not read.
        std::fs::write(dir.join("trusted-domains"), "evil.example\n").unwrap();
        assert!(!Trust::load_in(Some(&dir), "sip:bob@fabrikam.test", &[]).trusts("evil.example"));
        // No account address: nothing is remembered.
        assert!(Trust::load_in(Some(&dir), "nobody", &[])
            .remember("x.example")
            .is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod tests {
    use super::Trust;

    #[test]
    fn parent_domain_of_a_host() {
        assert_eq!(Trust::domain_of("sip.hoster.example"), "hoster.example");
        assert_eq!(Trust::domain_of("hoster.example"), "hoster.example");
        assert_eq!(Trust::domain_of("localhost"), "localhost");
    }
}
