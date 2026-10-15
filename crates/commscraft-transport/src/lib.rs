//! Inbound-only SIP/TLS connection for an explicitly chosen discovered peer.
//!
//! RFC 3261 §§7.5, 18.3; RFC 3263 §7; [MS-CONMGMT] §3.1.5.
//! `native_tls` verifies the system CA chain and the exact TLS peer name.
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
    let name = raw.trim_end_matches('.');
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
            return match tls.connect(peer.host(), tcp) {
                Ok(tls) => Ok(Self { tls, sip: SipInbound::new(), peer }),
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
    fn invalid_timeout_does_not_attempt_network() {
        let target = TlsEndpoint::from_domain_and_host("example.com", "sip.example.com", 5061).unwrap();
        assert!(matches!(TlsSipReceiver::connect(target.clone(), Duration::ZERO),
            Err(TransportError::InvalidEndpoint)));
        assert!(matches!(TlsSipReceiver::connect(target, Duration::from_secs(31)),
            Err(TransportError::InvalidEndpoint)));
    }
}
