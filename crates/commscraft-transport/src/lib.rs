//! Inbound-only SIP/TLS connection for an explicitly chosen discovered peer.
//!
//! RFC 3261 §§7.5, 18.3; RFC 3263 §7; [MS-CONMGMT] §3.1.5.
//! `native_tls` validates the platform CA chain against the *original SIP
//! domain*, not the potentially DNS-controlled SRV target (RFC 5922 §7.3).
//! Platform hostname matching is NOT a full RFC 5922 SAN/URI/wildcard verifier.
//! A successful TLS handshake is NOT a Microsoft SIPAE Security Association.
//! No public API exists to send SIP, credentials, or REGISTER requests.

use native_tls::{TlsConnector, TlsStream};
use commscraft_core::inbound::{ReceiveError, ReceiveEvent, SipInbound};
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
    if name.len() > 253 || name.is_empty() || !name.is_ascii() || !name.contains('.') {
        return None;
    }
    if !name.split('.').all(|label| !label.is_empty() && label.len() <= 63
        && !label.starts_with('-') && !label.ends_with('-')
        && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')) {
        return None;
    }
    Some(name.to_ascii_lowercase())
}
impl TlsEndpoint {
    pub fn from_domain_and_host(domain: &str, host: &str, port: u16) -> Result<Self, TransportError> {
        let domain = dns_name(domain).ok_or(TransportError::InvalidEndpoint)?;
        let host = dns_name(host).ok_or(TransportError::InvalidEndpoint)?;
        if port == 0 || !(host == domain || host.ends_with(&format!(".{domain}"))) {
            return Err(TransportError::InvalidEndpoint);
        }
        Ok(Self { domain, host, port })
    }
    pub fn host(&self) -> &str { &self.host }
    pub fn port(&self) -> u16 { self.port }
    pub fn sip_domain(&self) -> &str { &self.domain }

    /// Identity to pass to TLS certificate verification and SNI. This is
    /// taken from the originating SIP URI, NOT the DNS SRV target hostname.
    /// RFC 5922 §§7.2–7.3: SIP domain identity is a full-domain match.
    pub fn tls_reference_identity(&self) -> &str { &self.domain }
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
    /// Note: platform validation may allow wildcard/CN rules beyond RFC 5922.
    /// This remains a restricted diagnostic, NOT proof of full SIP identity.
    pub fn connect(peer: TlsEndpoint, timeout: Duration) -> Result<Self, TransportError> {
        if timeout.is_zero() || timeout > Duration::from_secs(30) {
            return Err(TransportError::InvalidEndpoint);
        }
        let addr_iter = (peer.host.as_str(), peer.port)
            .to_socket_addrs().map_err(|_| TransportError::DnsFailed)?;
        let addresses: Vec<_> = addr_iter.take(16).collect();
        if addresses.is_empty() { return Err(TransportError::DnsFailed); }
        let tls = TlsConnector::new().map_err(|_| TransportError::TlsVerificationFailed)?;
        for address in addresses {
            let tcp = match TcpStream::connect_timeout(&address, timeout) {
                Ok(tcp) => tcp,
                Err(_) => continue,
            };
            if tcp.set_read_timeout(Some(timeout)).is_err()
                || tcp.set_write_timeout(Some(timeout)).is_err() {
                return Err(TransportError::TcpUnavailable);
            }
            return match tls.connect(peer.tls_reference_identity(), tcp) {
                Ok(tls) => {
                    // Even with the platform verifier, require an actual
                    // X.509 peer certificate before accepting the diagnostic
                    // connection. This is not complete RFC 5922 SAN parsing.
                    let cert = tls.peer_certificate()
                        .map_err(|_| TransportError::TlsVerificationFailed)?;
                    if cert.is_none() {
                        return Err(TransportError::TlsVerificationFailed);
                    }
                    Ok(Self { tls, sip: SipInbound::new(), peer })
                }
                Err(_) => Err(TransportError::TlsVerificationFailed),
            };
        }
        Err(TransportError::TcpUnavailable)
    }

    pub fn endpoint(&self) -> &TlsEndpoint { &self.peer }

    /// Receive-only: SIP messages are not considered authenticated here.
    /// No untrusted Contact/GRUU data is promoted to authenticated state.
    pub fn receive(&mut self) -> Result<ReceiveEvent, TransportError> {
        self.sip.receive(&mut self.tls).map_err(TransportError::Read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_subdomain_boundaries_and_ports() {
        assert!(TlsEndpoint::from_domain_and_host("example.com", "sip.EXAMPLE.com.", 5061).is_ok());
        assert!(TlsEndpoint::from_domain_and_host("example.com", "example.com", 443).is_ok());
        for host in ["otherexample.com", "example.com.attacker.test", "sip.example.com.evil",
            "sip..example.com", "[::1]", "sip.example.com:5061"] {
            assert_eq!(TlsEndpoint::from_domain_and_host("example.com", host, 5061),
                Err(TransportError::InvalidEndpoint));
        }
        assert_eq!(TlsEndpoint::from_domain_and_host("example.com", "sip.example.com", 0),
            Err(TransportError::InvalidEndpoint));
    }
    #[test]
    fn rejects_host_injection_and_invalid_domains() {
        for invalid in ["bad\r\n.example.com", "example.com ", "foo_.example.com", "-foo.example.com", "singlelabel"] {
            assert_eq!(TlsEndpoint::from_domain_and_host(invalid, "sip.example.com", 5061),
                Err(TransportError::InvalidEndpoint));
        }
    }
    #[test]
    fn tls_certificate_identity_is_original_sip_domain_not_srv_hostname() {
        let endpoint = TlsEndpoint::from_domain_and_host(
            "ExAmPlE.COM.", "sipedge01.EXAMPLE.COM.", 5061
        ).unwrap();
        assert_eq!(endpoint.host(), "sipedge01.example.com");
        assert_eq!(endpoint.sip_domain(), "example.com");
        assert_eq!(endpoint.tls_reference_identity(), "example.com");
        // DNS SRV targets are NOT permitted to replace the origin identity.
        assert_ne!(endpoint.tls_reference_identity(), endpoint.host());
        let direct = TlsEndpoint::from_domain_and_host(
            "example.com", "example.com", 5061
        ).unwrap();
        assert_eq!(direct.tls_reference_identity(), direct.host());
    }
    #[test]
    fn dns_root_dot_and_forged_suffixes_fail_closed() {
        for host in ["sip.example.com..", "sip.example.com...", "sip.example.com.evil.",
                     "sip..example.com", "*.example.com", "sip.example.com\\r\\n"] {
            assert_eq!(TlsEndpoint::from_domain_and_host("example.com", host, 5061),
                Err(TransportError::InvalidEndpoint), "{host:?}");
        }
        for domain in ["example.com..", "example.com...", "*.example.com"] {
            assert_eq!(TlsEndpoint::from_domain_and_host(domain, "sip.example.com", 5061),
                Err(TransportError::InvalidEndpoint), "{domain:?}");
        }
    }
    #[test]
    fn invalid_timeout_does_not_attempt_network() {
        let target = TlsEndpoint::from_domain_and_host("example.com", "sip.example.com", 5061).unwrap();
        assert!(matches!(TlsSipReceiver::connect(target.clone(), Duration::ZERO),
            Err(TransportError::InvalidEndpoint)));
        assert!(matches!(TlsSipReceiver::connect(target, Duration::from_secs(31)),
            Err(TransportError::InvalidEndpoint)));
    }
}
