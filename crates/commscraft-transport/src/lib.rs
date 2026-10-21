//! Inbound-only SIP/TLS connection for an explicitly chosen discovered peer.
//!
//! RFC 3261 §§7.5, 18.3; RFC 3263 §7; [MS-CONMGMT] §3.1.5.
//! `native_tls` validates the platform CA chain against the *original SIP
//! domain*, not the potentially DNS-controlled SRV target (RFC 5922 §7.3).
//! Strict peer SIP SAN extraction and exact-domain comparison are applied
//! after the platform TLS/PKIX handshake. Legacy CN-only and internationalized
//! U-label identities are intentionally rejected until fully specified.
//! A successful TLS handshake is NOT a Microsoft SIPAE Security Association.
//! No public API exists to send SIP, credentials, or REGISTER requests.

mod idna_policy;
pub mod offline_register_security_handoff;
pub mod offline_session_identity;
mod sip_certificate;
use commscraft_core::inbound::{ReceiveError, ReceiveEvent, SipInbound};
use native_tls::{TlsConnector, TlsStream};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    InvalidEndpoint,
    DnsFailed,
    TcpUnavailable,
    TlsVerificationFailed,
    Read(ReceiveError),
}

/// Restricted endpoint derived from a SIP identity domain and explicitly
/// selected TLS-SRV record. No off-domain redirects or implicit plaintext.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TlsEndpoint {
    domain: String,
    host: String,
    port: u16,
}
fn dns_name(raw: &str) -> Option<String> {
    // Exactly one optional terminal DNS root dot; reject repeated dots.
    let name = raw.strip_suffix('.').unwrap_or(raw);
    // DNS-based SIP identity must not be a numeric IP literal (RFC 5922).
    if name.parse::<std::net::Ipv4Addr>().is_ok() {
        return None;
    }
    if name.len() > 253 || name.is_empty() || !name.is_ascii() || !name.contains('.') {
        return None;
    }
    if !name.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    }) {
        return None;
    }
    // RFC 5890 / RFC 9549 §2.3: validate xn-- A-labels before treating
    // them as DNS targets or verified TLS reference identities. Do not
    // silently transform untrusted hostnames through compatibility mapping.
    let canonical = idna::domain_to_ascii_strict(name).ok()?;
    if !canonical.eq_ignore_ascii_case(name)
        || !crate::idna_policy::rfc9549_a_label_symbols_allowed(&canonical)
    {
        return None;
    }
    Some(canonical)
}
impl TlsEndpoint {
    pub fn from_domain_and_host(
        domain: &str,
        host: &str,
        port: u16,
    ) -> Result<Self, TransportError> {
        let domain = dns_name(domain).ok_or(TransportError::InvalidEndpoint)?;
        let host = dns_name(host).ok_or(TransportError::InvalidEndpoint)?;
        if port == 0 || !(host == domain || host.ends_with(&format!(".{domain}"))) {
            return Err(TransportError::InvalidEndpoint);
        }
        Ok(Self { domain, host, port })
    }
    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn sip_domain(&self) -> &str {
        &self.domain
    }

    /// Identity to pass to TLS certificate verification and SNI. This is
    /// taken from the originating SIP URI, NOT the DNS SRV target hostname.
    /// RFC 5922 §§7.2–7.3: SIP domain identity is a full-domain match.
    pub fn tls_reference_identity(&self) -> &str {
        &self.domain
    }
}

pub struct TlsSipReceiver {
    tls: TlsStream<TcpStream>,
    sip: SipInbound,
    peer: TlsEndpoint,
}

impl TlsSipReceiver {
    /// No external CA or hostname verification bypass; never retries without TLS.
    /// Only TCP-connect failures are retried across A/AAAA addresses. The first
    /// TLS failure is terminal in accordance with [MS-CONMGMT] §3.1.5.
    /// The original SIP domain is used for TLS SNI and platform certificate
    /// validation. SRV hostnames are used ONLY for DNS address resolution;
    /// that distinction is essential under RFC 5922 §7.3.
    /// Platform validation can accept wildcard/CN names, so we additionally
    /// require a strict RFC 5922 SIP-URI/DNS SAN identity from the real leaf
    /// certificate. This remains an inbound-only diagnostic (not sign-in).
    pub fn connect(peer: TlsEndpoint, timeout: Duration) -> Result<Self, TransportError> {
        if timeout.is_zero() || timeout > Duration::from_secs(30) {
            return Err(TransportError::InvalidEndpoint);
        }
        let addr_iter = (peer.host.as_str(), peer.port)
            .to_socket_addrs()
            .map_err(|_| TransportError::DnsFailed)?;
        let addresses: Vec<_> = addr_iter.take(16).collect();
        if addresses.is_empty() {
            return Err(TransportError::DnsFailed);
        }
        let tls = TlsConnector::new().map_err(|_| TransportError::TlsVerificationFailed)?;
        for address in addresses {
            let tcp = match TcpStream::connect_timeout(&address, timeout) {
                Ok(tcp) => tcp,
                Err(_) => continue,
            };
            if tcp.set_read_timeout(Some(timeout)).is_err()
                || tcp.set_write_timeout(Some(timeout)).is_err()
            {
                return Err(TransportError::TcpUnavailable);
            }
            return match tls.connect(peer.tls_reference_identity(), tcp) {
                Ok(tls) => {
                    // Even with the platform verifier, require an actual
                    // X.509 peer certificate before accepting the diagnostic
                    // connection. This is not complete RFC 5922 SAN parsing.
                    let cert = tls
                        .peer_certificate()
                        .map_err(|_| TransportError::TlsVerificationFailed)?;
                    let cert = cert.ok_or(TransportError::TlsVerificationFailed)?;
                    let der = cert
                        .to_der()
                        .map_err(|_| TransportError::TlsVerificationFailed)?;
                    if !sip_certificate::matches_sip_identity(&der, peer.tls_reference_identity()) {
                        return Err(TransportError::TlsVerificationFailed);
                    }
                    Ok(Self {
                        tls,
                        sip: SipInbound::new(),
                        peer,
                    })
                }
                Err(_) => Err(TransportError::TlsVerificationFailed),
            };
        }
        Err(TransportError::TcpUnavailable)
    }

    pub fn endpoint(&self) -> &TlsEndpoint {
        &self.peer
    }

    /// Receive-only: SIP messages are not considered authenticated here.
    /// No untrusted Contact/GRUU data is promoted to authenticated state.
    pub fn receive(&mut self) -> Result<ReceiveEvent, TransportError> {
        self.sip
            .receive(&mut self.tls)
            .map_err(TransportError::Read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_subdomain_boundaries_and_ports() {
        assert!(TlsEndpoint::from_domain_and_host("example.com", "sip.EXAMPLE.com.", 5061).is_ok());
        assert!(TlsEndpoint::from_domain_and_host("example.com", "example.com", 443).is_ok());
        for host in [
            "otherexample.com",
            "example.com.attacker.test",
            "sip.example.com.evil",
            "sip..example.com",
            "[::1]",
            "sip.example.com:5061",
        ] {
            assert_eq!(
                TlsEndpoint::from_domain_and_host("example.com", host, 5061),
                Err(TransportError::InvalidEndpoint)
            );
        }
        assert_eq!(
            TlsEndpoint::from_domain_and_host("example.com", "sip.example.com", 0),
            Err(TransportError::InvalidEndpoint)
        );
    }
    #[test]
    fn numeric_ipv4_hosts_are_not_dns_sip_tls_identities() {
        for ip in ["127.0.0.1", "192.0.2.5", "255.255.255.255"] {
            assert_eq!(
                TlsEndpoint::from_domain_and_host(ip, ip, 5061),
                Err(TransportError::InvalidEndpoint),
            );
            assert_eq!(
                TlsEndpoint::from_domain_and_host("example.com", ip, 5061),
                Err(TransportError::InvalidEndpoint),
            );
        }
        assert!(
            TlsEndpoint::from_domain_and_host("example.com", "192.0.2.5.example.com", 5061).is_ok()
        );
    }
    #[test]
    fn rejects_host_injection_and_invalid_domains() {
        for invalid in [
            "bad\r\n.example.com",
            "example.com ",
            "foo_.example.com",
            "-foo.example.com",
            "singlelabel",
        ] {
            assert_eq!(
                TlsEndpoint::from_domain_and_host(invalid, "sip.example.com", 5061),
                Err(TransportError::InvalidEndpoint)
            );
        }
    }
    #[test]
    fn idna_a_label_validation_is_required_for_tls_sni_and_srv_target() {
        let valid = TlsEndpoint::from_domain_and_host(
            "XN--BCHER-KVA.EXAMPLE",
            "edge.XN--BCHER-KVA.example",
            5061,
        )
        .expect("valid canonical punycode A-label");
        assert_eq!(valid.tls_reference_identity(), "xn--bcher-kva.example");
        assert_eq!(valid.host(), "edge.xn--bcher-kva.example");
        for invalid in ["xn--a.example", "xn--ab.example", "xn--p.example"] {
            assert_eq!(
                TlsEndpoint::from_domain_and_host(invalid, invalid, 5061),
                Err(TransportError::InvalidEndpoint),
                "{invalid}"
            );
            assert_eq!(
                TlsEndpoint::from_domain_and_host(
                    "example.com",
                    &format!("edge.{invalid}.com"),
                    5061
                ),
                Err(TransportError::InvalidEndpoint),
                "bad SRV target {invalid}"
            );
        }
        // Deliberate fail-closed subset: no U-label ingress conversion.
        assert_eq!(
            TlsEndpoint::from_domain_and_host("bücher.example", "edge.xn--bcher-kva.example", 5061),
            Err(TransportError::InvalidEndpoint)
        );
    }

    #[test]
    fn rfc9549_rejects_other_symbol_a_labels_before_tls_dns_or_sni() {
        for invalid in ["xn--45h.example.com", "XN--45H.EXAMPLE.COM"] {
            assert_eq!(
                TlsEndpoint::from_domain_and_host(invalid, invalid, 5061),
                Err(TransportError::InvalidEndpoint),
                "forbidden Unicode Other Symbol A-label: {invalid}"
            );
            assert_eq!(
                TlsEndpoint::from_domain_and_host("example.com", &format!("edge.{invalid}"), 5061),
                Err(TransportError::InvalidEndpoint),
                "forbidden Unicode Other Symbol in DNS SRV destination: {invalid}"
            );
        }
        let accepted = TlsEndpoint::from_domain_and_host(
            "xn--bcher-kva.example.com",
            "edge.xn--bcher-kva.example.com",
            5061,
        )
        .expect("valid IDNA A-label");
        assert_eq!(
            accepted.tls_reference_identity(),
            "xn--bcher-kva.example.com"
        );
        assert_eq!(accepted.host(), "edge.xn--bcher-kva.example.com");
    }

    #[test]
    fn tls_certificate_identity_is_original_sip_domain_not_srv_hostname() {
        let endpoint =
            TlsEndpoint::from_domain_and_host("ExAmPlE.COM.", "sipedge01.EXAMPLE.COM.", 5061)
                .unwrap();
        assert_eq!(endpoint.host(), "sipedge01.example.com");
        assert_eq!(endpoint.sip_domain(), "example.com");
        assert_eq!(endpoint.tls_reference_identity(), "example.com");
        // DNS SRV targets are NOT permitted to replace the origin identity.
        assert_ne!(endpoint.tls_reference_identity(), endpoint.host());
        let direct = TlsEndpoint::from_domain_and_host("example.com", "example.com", 5061).unwrap();
        assert_eq!(direct.tls_reference_identity(), direct.host());
    }
    #[test]
    fn dns_root_dot_and_forged_suffixes_fail_closed() {
        for host in [
            "sip.example.com..",
            "sip.example.com...",
            "sip.example.com.evil.",
            "sip..example.com",
            "*.example.com",
            "sip.example.com\r\n",
        ] {
            assert_eq!(
                TlsEndpoint::from_domain_and_host("example.com", host, 5061),
                Err(TransportError::InvalidEndpoint),
                "{host:?}"
            );
        }
        for domain in ["example.com..", "example.com...", "*.example.com"] {
            assert_eq!(
                TlsEndpoint::from_domain_and_host(domain, "sip.example.com", 5061),
                Err(TransportError::InvalidEndpoint),
                "{domain:?}"
            );
        }
    }
    #[test]
    fn invalid_timeout_does_not_attempt_network() {
        let target =
            TlsEndpoint::from_domain_and_host("example.com", "sip.example.com", 5061).unwrap();
        assert!(matches!(
            TlsSipReceiver::connect(target.clone(), Duration::ZERO),
            Err(TransportError::InvalidEndpoint)
        ));
        assert!(matches!(
            TlsSipReceiver::connect(target, Duration::from_secs(31)),
            Err(TransportError::InvalidEndpoint)
        ));
    }
}
