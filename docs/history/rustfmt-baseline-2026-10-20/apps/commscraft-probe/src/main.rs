//! Diagnostic probes ONLY. Does not send SIP requests or credentials.
//! All live network operations require an explicit CLI flag.
use hickory_resolver::Resolver;
use rand::{rngs::OsRng, Rng};
use commscraft_core::discovery::{DiscoveryPlan, Service, SrvRandom, SrvRecord, TlsCandidate};
use std::error::Error;
use std::time::Duration;
use commscraft_transport::{TlsEndpoint, TlsSipReceiver, TransportError};

struct SecureRandom(OsRng);
impl SrvRandom for SecureRandom {
    fn choose_inclusive(&mut self, maximum: u32) -> u32 { self.0.gen_range(0..=maximum) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { Plan, Dns, Tls }

fn arguments() -> Result<(DiscoveryPlan, Mode), String> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 || a.len() > 4 || a[1] != "--domain" {
        return Err("Usage: commscraft-probe --domain example.com [--dns | --tls-probe]\n\
            Default: offline plan only. --dns sends 4 DNS SRV queries.\n\
            --tls-probe also connects using validated TLS, but sends NO SIP messages or credentials.\n\
            Only test domains/deployments you are authorized to contact.".into());
    }
    let plan = DiscoveryPlan::from_domain(&a[2]).map_err(|e| format!("Invalid DNS domain: {e:?}"))?;
    let mode = match a.get(3).map(String::as_str) {
        None => Mode::Plan,
        Some("--dns") => Mode::Dns,
        Some("--tls-probe") => Mode::Tls,
        _ => return Err("Unknown flag. Use --dns or --tls-probe".into()),
    };
    Ok((plan, mode))
}

/// Four lookups are started concurrently as required by [MS-CONMGMT] §3.1.5.
/// Resolver errors are reported rather than indistinguishably counted as zero.
fn lookup_srv(plan: &DiscoveryPlan) -> Vec<(Service, Vec<SrvRecord>)> {
    let jobs: Vec<_> = plan.srv_queries().into_iter().map(|(service, name)| {
        std::thread::spawn(move || {
            let result = (|| -> Result<Vec<SrvRecord>, String> {
                let resolver = Resolver::from_system_conf().map_err(|e| e.to_string())?;
                let found = resolver.srv_lookup(format!("{name}."))
                    .map_err(|e| e.to_string())?;
                found.iter().map(|r| {
                    SrvRecord::new(r.priority(), r.weight(), r.port(), &r.target().to_utf8())
                        .map_err(|e| format!("invalid SRV answer: {e:?}"))
                }).collect()
            })();
            (service, name, result)
        })
    }).collect();
    let mut answers = Vec::new();
    for job in jobs {
        match job.join() {
            Ok((service, name, Ok(records))) => {
                println!("  {name}: {} record(s)", records.len());
                answers.push((service, records));
            }
            Ok((service, name, Err(_))) => {
                eprintln!("  {name}: no usable DNS response (NXDOMAIN, no data, or resolver error)");
                answers.push((service, Vec::new()));
            }
            Err(_) => eprintln!("  a DNS worker exited unexpectedly; no results from that worker"),
        }
    }
    answers
}

/// Constrain unauthenticated diagnostics to the SIP identity's DNS subtree.
/// Real deployments can point at other domains, but need separate policy and
/// certificate/service-identity binding before a production client can trust
/// such a redirection (RFC 3263 §7).
fn authorized_target(domain: &str, host: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

#[derive(Debug, PartialEq, Eq)]
enum ProbeResult { Connected, CannotConnect, InvalidTls }

fn try_tls(candidate: &TlsCandidate, sip_domain: &str) -> ProbeResult {
    // Transport crate accepts only explicit SIP-domain-bound, validated TLS
    // endpoints and exposes receive-only access after the OS TLS handshake.
    let peer = match TlsEndpoint::from_domain_and_host(
        sip_domain, &candidate.host, candidate.port,
    ) {
        Ok(peer) => peer,
        Err(_) => return ProbeResult::InvalidTls,
    };
    match TlsSipReceiver::connect(peer, Duration::from_secs(4)) {
        Ok(_) => ProbeResult::Connected,
        Err(TransportError::DnsFailed | TransportError::TcpUnavailable) => ProbeResult::CannotConnect,
        Err(_) => ProbeResult::InvalidTls,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let (plan, mode) = arguments()?;
    println!("CommSCraft SIP discovery for {} (no registration or auth)", plan.sip_domain);
    for (service, name) in plan.srv_queries() {
        println!("  {name} [{}]", if service.is_tls() { "TLS" } else { "TCP: NEVER PROBED" });
    }
    if mode == Mode::Plan {
        println!("Offline plan only. Pass --dns for explicit DNS queries.");
        return Ok(());
    }
    println!("DNS SRV queries (parallel):");
    let answers = lookup_srv(&plan);
    let candidates = plan.tls_candidates(&answers, &mut SecureRandom(OsRng));
    println!("TLS candidates in priority order:");
    for candidate in &candidates {
        println!("  {}:{} ({:?})", candidate.host, candidate.port, candidate.origin);
    }
    if mode == Mode::Dns { return Ok(()); }
    println!("TLS-only diagnostic handshake; no SIP/REGISTER packets will be sent.");
    for candidate in candidates.iter().filter(|c| authorized_target(&plan.sip_domain, &c.host)).take(8) {
        match try_tls(candidate, &plan.sip_domain) {
            ProbeResult::Connected => {
                println!("TLS platform-verified for SIP domain {} via {}:{}; RFC 5922 SAN/wildcard rules not fully verified; NOT signed in.", plan.sip_domain, candidate.host, candidate.port);
                return Ok(());
            }
            ProbeResult::CannotConnect => {
                println!("No TCP connectivity to {}:{}; trying next candidate.", candidate.host, candidate.port);
            }
            ProbeResult::InvalidTls => {
                // Per MS-CONMGMT §3.1.5, fail on other than DNS/TCP failure;
                // never bypass certificate checks or downgrade to TCP.
                return Err("TLS handshake/certificate verification failed; stopping without fallback".into());
            }
        }
    }
    Err("No allowed TLS endpoint was verified; no SIP request was sent".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suffix_policy_is_label_bound() {
        assert!(authorized_target("example.com", "sip.example.com"));
        assert!(authorized_target("example.com", "example.com"));
        assert!(!authorized_target("example.com", "evil-example.com"));
        assert!(!authorized_target("example.com", "example.com.attacker.org"));
    }
    #[test]
    fn no_network_for_plan_mode() {
        // No live DNS or socket request is needed to produce deterministic plan.
        let p = DiscoveryPlan::from_domain("example.com").unwrap();
        assert_eq!(p.srv_queries().len(), 4);
    }
}
