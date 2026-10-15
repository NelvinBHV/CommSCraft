//! Bounded Microsoft SIP-domain discovery planning and RFC 2782 SRV ordering.
//!
//! MS-CONMGMT §3.1.5 defines four SRV lookups (parallel), specific precedence,
//! and fallback names. This library plans and combines results, but performs
//! NO DNS/network I/O and intentionally returns TLS candidates only.
//! It does not implement DHCP option 120 or HTTPS/mobile autodiscovery.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryPlan {
    pub sip_domain: String,
    /// Mobile/UCWA HTTPS names are informational, not SIP replacements.
    pub https_hosts: Vec<String>,
    pub tls_srv_names: Vec<String>,
    pub tcp_srv_names: Vec<String>,
    pub fallback_hosts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    InvalidSipIdentity,
    InvalidDomain,
    InvalidSrvTarget,
    InvalidPort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service { InternalTls, InternalTcp, ExternalTls, ExternalTcp }

impl Service {
    pub const ALL: [Self; 4] = [Self::InternalTls, Self::InternalTcp, Self::ExternalTls, Self::ExternalTcp];
    pub fn srv_name(self, domain: &str) -> String {
        let prefix = match self {
            Self::InternalTls => "_sipinternaltls._tcp",
            Self::InternalTcp => "_sipinternal._tcp",
            Self::ExternalTls => "_sip._tls",
            Self::ExternalTcp => "_sip._tcp",
        };
        format!("{prefix}.{domain}")
    }
    pub fn is_tls(self) -> bool { matches!(self, Self::InternalTls | Self::ExternalTls) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrvRecord {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    /// Validated ASCII DNS hostname; an isolated '.' denotes unavailable service.
    pub target: String,
}

impl SrvRecord {
    pub fn new(priority: u16, weight: u16, port: u16, target: &str) -> Result<Self, DiscoveryError> {
        if target == "." {
            return Ok(Self { priority, weight, port, target: String::from(".") });
        }
        let target = normalize_domain(target).map_err(|_| DiscoveryError::InvalidSrvTarget)?;
        if port == 0 { return Err(DiscoveryError::InvalidPort); }
        Ok(Self { priority, weight, port, target })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateOrigin { Srv(Service), Fallback }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsCandidate {
    pub host: String,
    pub port: u16,
    pub origin: CandidateOrigin,
}

/// A uniform random draw over 0..=maximum, supplied externally. The caller
/// should use a proper RNG in production; tests can inject deterministic draws.
/// Invalid >maximum values are reduced modulo maximum+1 to avoid panics from
/// hostile/mock sources (production RNG should already obey the requested range).
pub trait SrvRandom { fn choose_inclusive(&mut self, maximum: u32) -> u32; }

/// RFC 2782 §Priority/Weight: ascending priority, zero-weight records placed
/// first, selection on inclusive 0..=sum of weights, then remove and repeat.
/// Note that RFC 2782's inclusive-zero convention gives weight-zero records
/// a small chance of being picked ahead of positive-weight records.
pub fn order_srv(records: &[SrvRecord], rng: &mut impl SrvRandom) -> Vec<SrvRecord> {
    let mut remaining = records.to_vec();
    remaining.sort_by_key(|r| r.priority);
    // Guard the sum of u16 weights against integer overflow and cap the
    // amount of work a hostile DNS answer may trigger.
    remaining.truncate(256);
    let mut ordered = Vec::with_capacity(remaining.len());
    while !remaining.is_empty() {
        let priority = remaining[0].priority;
        let group_size = remaining.iter().take_while(|r| r.priority == priority).count();
        let mut group: Vec<SrvRecord> = remaining.drain(..group_size).collect();
        while !group.is_empty() {
            group.sort_by_key(|r| r.weight != 0); // zero-weight first, stable
            let total: u32 = group.iter().map(|r| u32::from(r.weight)).sum();
            let draw = rng.choose_inclusive(total) % (total + 1);
            let mut running = 0;
            let index = group.iter().position(|r| {
                running += u32::from(r.weight);
                running >= draw
            }).expect("nonempty group has a selection");
            ordered.push(group.remove(index));
        }
    }
    ordered
}

pub fn normalize_domain(name: &str) -> Result<String, DiscoveryError> {
    let name = name.strip_suffix('.').unwrap_or(name);
    if name.is_empty() || name.len() > 253 || !name.contains('.') || !name.is_ascii()
        || name.split('.').any(|label| {
            label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-')
                || !label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        }) {
        return Err(DiscoveryError::InvalidDomain);
    }
    Ok(name.to_ascii_lowercase())
}

impl DiscoveryPlan {
    pub fn from_sip_uri(uri: &str) -> Result<Self, DiscoveryError> {
        let name = uri.strip_prefix("sip:").ok_or(DiscoveryError::InvalidSipIdentity)?;
        let (user, domain) = name.split_once('@').ok_or(DiscoveryError::InvalidSipIdentity)?;
        // Conservative ASCII SIP identity subset, not the complete RFC 3261 grammar.
        if user.is_empty() || name.matches('@').count() != 1
            || !user.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._+".contains(&b)) {
            return Err(DiscoveryError::InvalidSipIdentity);
        }
        Self::from_domain(domain)
    }

    pub fn from_domain(domain: &str) -> Result<Self, DiscoveryError> {
        let domain = normalize_domain(domain)?;
        Ok(Self {
            sip_domain: domain.clone(),
            https_hosts: vec![format!("lyncdiscoverinternal.{domain}"), format!("lyncdiscover.{domain}")],
            tls_srv_names: vec![Service::InternalTls.srv_name(&domain), Service::ExternalTls.srv_name(&domain)],
            tcp_srv_names: vec![Service::InternalTcp.srv_name(&domain), Service::ExternalTcp.srv_name(&domain)],
            fallback_hosts: vec![format!("sipinternal.{domain}"), format!("sip.{domain}"), format!("sipexternal.{domain}")],
        })
    }

    pub fn srv_queries(&self) -> [(Service, String); 4] {
        Service::ALL.map(|service| (service, service.srv_name(&self.sip_domain)))
    }

    /// Returns TLS-only candidates in Microsoft service order. DHCP and TCP
    /// are omitted on purpose; plaintext cannot be enabled by DNS responses.
    /// Note: duplicate endpoints preserve first occurrence (SRV before fallback).
    pub fn tls_candidates(&self, answers: &[(Service, Vec<SrvRecord>)], rng: &mut impl SrvRandom) -> Vec<TlsCandidate> {
        let mut result = Vec::new();
        for service in [Service::InternalTls, Service::ExternalTls] {
            for (_, records) in answers.iter().filter(|(s, _)| *s == service) {
                for record in order_srv(records, rng) {
                    if record.target == "." { continue; }
                    push_unique(&mut result, TlsCandidate {
                        host: record.target, port: record.port, origin: CandidateOrigin::Srv(service),
                    });
                }
            }
        }
        for host in &self.fallback_hosts {
            push_unique(&mut result, TlsCandidate {
                host: host.clone(), port: 443, origin: CandidateOrigin::Fallback,
            });
        }
        result
    }
}

fn push_unique(result: &mut Vec<TlsCandidate>, candidate: TlsCandidate) {
    if !result.iter().any(|c| c.host == candidate.host && c.port == candidate.port) {
        result.push(candidate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Draws(Vec<u32>);
    impl SrvRandom for Draws {
        fn choose_inclusive(&mut self, maximum: u32) -> u32 {
            let v = self.0.pop().unwrap_or(0);
            assert!(v <= maximum);
            v
        }
    }
    fn rr(priority: u16, weight: u16, host: &str) -> SrvRecord {
        SrvRecord::new(priority, weight, 5061, host).unwrap()
    }
    #[test]
    fn microsoft_names_cover_four_srv_services() {
        let p = DiscoveryPlan::from_sip_uri("sip:alice@Example.COM").unwrap();
        assert_eq!(p.sip_domain, "example.com");
        assert_eq!(p.srv_queries().iter().map(|(_,n)| n.as_str()).collect::<Vec<_>>(), vec![
            "_sipinternaltls._tcp.example.com", "_sipinternal._tcp.example.com",
            "_sip._tls.example.com", "_sip._tcp.example.com"]);
        assert_eq!(p.fallback_hosts[2], "sipexternal.example.com");
    }
    #[test]
    fn rejects_invalid_identity_or_hostnames() {
        for s in ["alice@example.com", "sip:alice@-bad.test", "sip:a@bad..test", "sip:x@e.com\r\nVia: x"] {
            assert!(DiscoveryPlan::from_sip_uri(s).is_err(), "{s}");
        }
        assert!(SrvRecord::new(0, 0, 0, "host.example.com").is_err());
        assert!(SrvRecord::new(0, 0, 5061, "bad..example.com").is_err());
        assert_eq!(SrvRecord::new(0, 0, 5061, "Host.EXAMPLE.com.").unwrap().target, "host.example.com");
    }
    #[test]
    fn weight_and_priority_follow_rfc2782() {
        let records = [rr(20, 10, "slow.example.com"), rr(10, 0, "zero.example.com"),
            rr(10, 3, "three.example.com"), rr(10, 1, "one.example.com")];
        // pop order: first draw=4 (last), second draw=1, then 0
        let got = order_srv(&records, &mut Draws(vec![0, 1, 4]));
        assert_eq!(got.iter().map(|r| r.target.as_str()).collect::<Vec<_>>(),
            vec!["one.example.com", "three.example.com", "zero.example.com", "slow.example.com"]);
    }
    #[test]
    fn never_includes_plaintext_dns_candidates() {
        let p = DiscoveryPlan::from_domain("example.com").unwrap();
        let got = p.tls_candidates(&[
            (Service::InternalTcp, vec![rr(0, 1, "insecure.example.com")]),
            (Service::ExternalTls, vec![rr(4, 1, "external.example.com")]),
            (Service::InternalTls, vec![rr(0, 1, "internal.example.com")]),
        ], &mut Draws(vec![]));
        assert_eq!(got[0].host, "internal.example.com");
        assert_eq!(got[1].host, "external.example.com");
        assert!(got.iter().all(|c| c.host != "insecure.example.com"));
        assert_eq!(got[2].port, 443);
    }
    #[test]
    fn duplicate_srv_and_fallback_are_deduplicated() {
        let p = DiscoveryPlan::from_domain("example.com").unwrap();
        let c = p.tls_candidates(&[(Service::InternalTls, vec![SrvRecord::new(0, 0, 443,
            "sipinternal.example.com").unwrap()])], &mut Draws(vec![]));
        assert_eq!(c.iter().filter(|c| c.host == "sipinternal.example.com").count(), 1);
    }
    #[test]
    fn dot_target_is_not_a_connectable_endpoint() {
        let p = DiscoveryPlan::from_domain("example.com").unwrap();
        let c = p.tls_candidates(&[(Service::InternalTls, vec![SrvRecord::new(0, 0, 5061, ".").unwrap()])], &mut Draws(vec![]));
        assert_eq!(c.len(), 3); // static fallback entries only
    }
}
