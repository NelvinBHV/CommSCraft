//! Server discovery: Lync Autodiscover first, then DNS SRV and fallback host names (OB-06), as a
//! sans-I/O state machine. The caller performs HTTPS requests and DNS lookups.
//!
//! Evidence: `docs/protocol/evidence/08-ocdiscws-autodiscover.md` (AD-01 … AD-13, O-AD-1/2,
//! K-AD-1), `02-conmgmt-connection.md` (CON-10 … CON-19), file 39 OB-06/OB-07.
//! Sources: [MS-OCDISCWS] 20250520 §2.2.4–2.2.5 (pp. 10–17), §3.2.1 (pp. 22–23), example §4.1
//! (pp. 24–26); [MS-CONMGMT] 20250520 §3.1 (pp. 13–15); [SFB-DNS] (OB-06).
//!
//! Security by design (`docs/security.md`, rule "Discovery"):
//! - HTTPS only by default: MS-OCDISCWS §3.2.1.1 also asks for `http://` requests, which would send
//!   the SIP URI in plain text and accept spoofable answers; that is decision proposal D-10
//!   (`Config::allow_plain_http`, default off);
//! - discovery sends no credentials: it uses the unauthenticated Domain resource (AD-10), never the
//!   User or OAuth resources;
//! - every URL and host from a response is checked (`commscraft-url`); at most 10 redirects and no
//!   loops (AD-12); SIP targets and web links outside the user's SIP domain are marked, so the
//!   caller asks the user before connecting or sending credentials there; DNS SRV targets outside
//!   the domain are dropped (CON-16);
//! - TLS only: plain TCP SRV records and DHCP results are not used (MS-CONMGMT allows them).

use commscraft_url::{dns_host, in_domain, parse_https, HttpsUrl};
use commscraft_xml::{Element, Limits};

pub const ACCEPT: &str = "application/vnd.microsoft.rtc.autodiscover+xml;v=1";
/// AD-12.
pub const MAX_REDIRECTS: usize = 10;
/// Upper bound for an Autodiscover answer (inference: the examples are below 2 KiB).
pub const MAX_BODY: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The address of record is not `sip:user@domain`.
    BadAor,
    /// Neither lyncdiscoverinternal nor lyncdiscover answered usefully: use DNS SRV.
    NotFound,
    TooManyRedirects,
    RedirectLoop,
    /// A redirect or link that is not an acceptable HTTPS URL.
    UnsafeUrl,
    /// Not an Autodiscover document, or not the expected content.
    BadResponse(&'static str),
    Xml(commscraft_xml::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLocation {
    Internal,
    External,
}

/// What the HTTPS adapter got back for a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Http {
        status: u16,
        location: Option<String>,
        content_type: Option<String>,
        body: String,
    },
    /// DNS, connection or TLS failure (certificate validation failures included).
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Autodiscover,
    SrvInternalTls,
    SrvTls,
    Fallback,
}

/// A SIP server to try, always over TLS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SipTarget {
    pub host: String,
    pub port: u16,
    pub source: Source,
    pub in_user_domain: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebLink {
    pub token: String,
    pub url: HttpsUrl,
    pub in_user_domain: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub access_location: Option<AccessLocation>,
    /// In the order to try: the access point for the reported location first.
    pub targets: Vec<SipTarget>,
    pub links: Vec<WebLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// GET this URL with `Accept: ACCEPT`.
    Fetch(String),
    Done(Outcome),
    Failed(Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Internal,
    External,
    Following,
    Finished,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub aor: String,
    /// D-10 (proposal): also try `http://` as MS-OCDISCWS §3.2.1.1 asks. Default: off.
    pub allow_plain_http: bool,
}

/// The Autodiscover part (AD-02 … AD-13).
#[derive(Debug, Clone)]
pub struct Autodiscover {
    domain: String,
    sipuri: String,
    allow_plain_http: bool,
    stage: Stage,
    pending: Vec<String>,
    visited: Vec<String>,
    redirects: usize,
}

/// Domain of a `sip:user@domain` address of record (CON-10).
pub fn sip_domain(aor: &str) -> Option<(String, String)> {
    let rest = aor
        .get(..4)
        .filter(|s| s.eq_ignore_ascii_case("sip:"))
        .and(aor.get(4..))?;
    let (user, domain) = rest.split_once('@')?;
    if user.is_empty()
        || !user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-+".contains(c))
    {
        return None;
    }
    Some((user.to_string(), dns_host(domain)?))
}

fn token_value(value: &str, wanted: &str) -> bool {
    value
        .split(';')
        .next()
        .is_some_and(|v| v.trim().eq_ignore_ascii_case(wanted))
}

impl Autodiscover {
    pub fn new(cfg: &Config) -> Result<Self, Error> {
        let (user, domain) = sip_domain(&cfg.aor).ok_or(Error::BadAor)?;
        Ok(Self {
            // AD-03: Skype for Business omits the "sip:" prefix in the sipuri parameter.
            sipuri: format!("{user}@{domain}"),
            domain,
            allow_plain_http: cfg.allow_plain_http,
            stage: Stage::Internal,
            pending: Vec::new(),
            visited: Vec::new(),
            redirects: 0,
        })
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    fn initial(&self, host: &str) -> Vec<String> {
        let mut v = Vec::new();
        if self.allow_plain_http {
            v.push(format!(
                "http://{host}.{}/?sipuri={}",
                self.domain, self.sipuri
            ));
        }
        v.push(format!(
            "https://{host}.{}/?sipuri={}",
            self.domain, self.sipuri
        ));
        v
    }

    /// The first request (AD-02: lyncdiscoverinternal before lyncdiscover).
    pub fn start(&mut self) -> Step {
        self.stage = Stage::Internal;
        self.pending = self.initial("lyncdiscoverinternal");
        self.next_pending()
    }

    fn next_pending(&mut self) -> Step {
        if self.pending.is_empty() {
            return match self.stage {
                Stage::Internal => {
                    self.stage = Stage::External;
                    self.pending = self.initial("lyncdiscover");
                    self.next_pending()
                }
                _ => self.finish(Step::Failed(Error::NotFound)),
            };
        }
        let url = self.pending.remove(0);
        self.visited.push(url.to_ascii_lowercase());
        Step::Fetch(url)
    }

    fn finish(&mut self, step: Step) -> Step {
        self.stage = Stage::Finished;
        step
    }

    fn follow(&mut self, target: &str) -> Step {
        if parse_https(target).is_none() {
            return self.finish(Step::Failed(Error::UnsafeUrl));
        }
        if self.visited.iter().any(|v| v.eq_ignore_ascii_case(target)) {
            return self.finish(Step::Failed(Error::RedirectLoop));
        }
        // Every step after the initial requests counts as a redirect (AD-12).
        self.redirects += 1;
        if self.redirects > MAX_REDIRECTS {
            return self.finish(Step::Failed(Error::TooManyRedirects));
        }
        self.stage = Stage::Following;
        self.pending.clear();
        self.visited.push(target.to_ascii_lowercase());
        Step::Fetch(target.to_string())
    }

    /// Feeds the response to the last `Fetch`.
    pub fn on_response(&mut self, response: Response) -> Step {
        if self.stage == Stage::Finished {
            return Step::Failed(Error::BadResponse("discovery already finished"));
        }
        let (status, location, content_type, body) = match response {
            Response::Http {
                status,
                location,
                content_type,
                body,
            } => (status, location, content_type, body),
            Response::Failed => return self.failed_request(),
        };
        match status {
            300..=399 => match location {
                Some(l) => self.follow(&l),
                None => self.failed_request(),
            },
            200 => {
                if !content_type.as_deref().is_some_and(|t| {
                    token_value(t, "application/vnd.microsoft.rtc.autodiscover+xml")
                }) {
                    return self.finish(Step::Failed(Error::BadResponse("content type")));
                }
                if body.len() > MAX_BODY {
                    return self.finish(Step::Failed(Error::BadResponse("too large")));
                }
                match self.document(&body) {
                    Ok(step) => step,
                    Err(e) => self.finish(Step::Failed(e)),
                }
            }
            _ => self.failed_request(),
        }
    }

    fn failed_request(&mut self) -> Step {
        match self.stage {
            Stage::Internal | Stage::External => self.next_pending(),
            _ => self.finish(Step::Failed(Error::NotFound)),
        }
    }

    fn document(&mut self, body: &str) -> Result<Step, Error> {
        let limits = Limits {
            max_input: MAX_BODY,
            max_depth: 8,
            ..Limits::default()
        };
        let root = commscraft_xml::parse(body, &limits).map_err(Error::Xml)?;
        // The schema has no target namespace (§6, p. 28).
        if root.name != "AutodiscoverResponse" || root.namespace.is_some() {
            return Err(Error::BadResponse("not an AutodiscoverResponse"));
        }
        // K-AD-1: §2.2.5.1 requires "internal"/"external", the examples write "Internal".
        let access = match root.attr("AccessLocation") {
            None => None,
            Some(v) if v.eq_ignore_ascii_case("internal") => Some(AccessLocation::Internal),
            Some(v) if v.eq_ignore_ascii_case("external") => Some(AccessLocation::External),
            Some(_) => return Err(Error::BadResponse("AccessLocation")),
        };
        let kinds: Vec<&Element> = root
            .elements()
            .filter(|e| matches!(e.name.as_str(), "Root" | "User" | "Domain"))
            .collect();
        let [kind] = kinds.as_slice() else {
            return Err(Error::BadResponse("exactly one of Root, User, Domain"));
        };
        let link = |token: &str| {
            kind.children_named("Link")
                .find(|l| {
                    l.attr("token")
                        .is_some_and(|t| t.eq_ignore_ascii_case(token))
                })
                .and_then(|l| l.attr("href"))
                .map(str::to_string)
        };
        if let Some(r) = link("Redirect") {
            return Ok(self.follow(&r));
        }
        if kind.name == "Root" {
            // AD-10: the Domain resource needs no authentication and carries the SIP access points.
            let d = link("Domain").ok_or(Error::BadResponse("Root without Domain link"))?;
            return Ok(self.follow(&d));
        }
        let mut out = Outcome {
            access_location: access,
            ..Outcome::default()
        };
        let order: &[&str] = match access {
            Some(AccessLocation::External) => {
                &["SipClientExternalAccess", "SipClientInternalAccess"]
            }
            _ => &["SipClientInternalAccess", "SipClientExternalAccess"],
        };
        for name in order {
            if let Some(a) = kind.child(name) {
                let host = a
                    .attr("fqdn")
                    .and_then(dns_host)
                    .ok_or(Error::BadResponse("SipAccess fqdn"))?;
                let port: u16 = a
                    .attr("port")
                    .and_then(|p| p.parse().ok())
                    .filter(|p| *p != 0)
                    .ok_or(Error::BadResponse("SipAccess port"))?;
                if !out.targets.iter().any(|t| t.host == host && t.port == port) {
                    out.targets.push(SipTarget {
                        in_user_domain: in_domain(&host, &self.domain),
                        host,
                        port,
                        source: Source::Autodiscover,
                    });
                }
            }
        }
        for l in kind.children_named("Link") {
            let (Some(token), Some(href)) = (l.attr("token"), l.attr("href")) else {
                continue;
            };
            // Links that are not acceptable HTTPS URLs are left out (fail closed per link).
            if let Some(url) = parse_https(href) {
                out.links.push(WebLink {
                    token: token.to_string(),
                    in_user_domain: in_domain(&url.host, &self.domain),
                    url,
                });
            }
        }
        Ok(self.finish(Step::Done(out)))
    }
}

/// A DNS SRV answer (RFC 2782).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrvRecord {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: String,
}

/// SRV names the caller has to look up (TLS only; CON-11 without the TCP names).
pub fn srv_names(domain: &str) -> [String; 2] {
    [
        format!("_sipinternaltls._tcp.{domain}"),
        format!("_sip._tls.{domain}"),
    ]
}

/// Ordered TLS targets from DNS: `_sipinternaltls._tcp`, then `_sip._tls`, each sorted by priority
/// (CON-12, CON-14), targets outside the domain dropped (CON-16), then the fallback hosts
/// `sipinternal`, `sip`, `sipexternal` on port 443 (CON-18). Weight is not used for ordering
/// (inference: MS-CONMGMT only asks to sort by priority).
pub fn srv_targets(
    domain: &str,
    internal_tls: &[SrvRecord],
    sip_tls: &[SrvRecord],
) -> Vec<SipTarget> {
    let mut out: Vec<SipTarget> = Vec::new();
    for (records, source) in [
        (internal_tls, Source::SrvInternalTls),
        (sip_tls, Source::SrvTls),
    ] {
        let mut sorted: Vec<&SrvRecord> = records.iter().collect();
        sorted.sort_by_key(|r| r.priority);
        for r in sorted {
            let Some(host) = dns_host(&r.target) else {
                continue;
            };
            if r.port == 0 || !in_domain(&host, domain) {
                continue;
            }
            if !out.iter().any(|t| t.host == host && t.port == r.port) {
                out.push(SipTarget {
                    host,
                    port: r.port,
                    source,
                    in_user_domain: true,
                });
            }
        }
    }
    for prefix in ["sipinternal", "sip", "sipexternal"] {
        let host = format!("{prefix}.{domain}");
        if !out.iter().any(|t| t.host == host && t.port == 443) {
            out.push(SipTarget {
                host,
                port: 443,
                source: Source::Fallback,
                in_user_domain: true,
            });
        }
    }
    out
}

/// The complete candidate list: Autodiscover targets first, then DNS (OB-06: "The Autodiscover
/// service is always favored … the others are fallback methods").
pub fn candidates(autodiscover: Option<&Outcome>, dns: Vec<SipTarget>) -> Vec<SipTarget> {
    let mut out: Vec<SipTarget> = autodiscover.map(|o| o.targets.clone()).unwrap_or_default();
    for t in dns {
        if !out.iter().any(|o| o.host == t.host && o.port == t.port) {
            out.push(t);
        }
    }
    out
}

#[cfg(test)]
mod tests;
