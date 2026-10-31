//! Network adapter: TLS connections for SIP, the OS random source and a blocking SIP connection.
//!
//! This is the first adapter crate (decision D-03): protocol crates stay sans-I/O, this crate owns
//! sockets and TLS. Evidence: MS-CONMGMT CON-01 … CON-04 (`docs/protocol/evidence/02-conmgmt-connection.md`).
//!
//! Security by design (`docs/security.md`, rule "TLS"):
//! - rustls with the `ring` provider; TLS 1.2 and 1.3 only (no SSL 3.0/TLS 1.0/1.1), AEAD cipher
//!   suites with forward secrecy only, as rustls offers them;
//! - certificates are always verified against the system trust store, optionally plus an
//!   explicitly given enterprise CA file; there is no way to switch verification off;
//! - the server name is the discovered host name (a checked DNS name), never an IP address;
//! - connect, read and write have timeouts; incoming data goes through the bounded, poisoning
//!   `StreamDecoder`;
//! - random numbers come from the OS CSPRNG; if it fails, the process aborts instead of continuing
//!   with predictable values.

pub mod dns;
pub mod http;

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use commscraft_sip_msg::{Limits, Message, StreamDecoder};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

#[derive(Debug)]
pub enum NetError {
    /// No usable root certificate (system store empty and no CA file).
    NoTrustAnchors,
    /// The CA file could not be read or contains no certificate.
    BadCaFile(String),
    /// The host is not a DNS name acceptable as TLS server name.
    BadHostName,
    Resolve(std::io::Error),
    Connect(std::io::Error),
    /// TLS handshake or record error (certificate validation failures included).
    Tls(std::io::Error),
    Io(std::io::Error),
    Sip(commscraft_sip_msg::Error),
    Closed,
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::NoTrustAnchors => f.write_str("no trusted root certificates"),
            NetError::BadCaFile(e) => write!(f, "CA file: {e}"),
            NetError::BadHostName => f.write_str("host name is not an acceptable DNS name"),
            NetError::Resolve(e) => write!(f, "name resolution: {e}"),
            NetError::Connect(e) => write!(f, "connect: {e}"),
            NetError::Tls(e) => write!(f, "TLS: {e}"),
            NetError::Io(e) => write!(f, "I/O: {e}"),
            NetError::Sip(e) => write!(f, "SIP framing: {e:?}"),
            NetError::Closed => f.write_str("connection closed by peer"),
        }
    }
}

impl std::error::Error for NetError {}

/// Builds TLS client configurations.
#[derive(Clone)]
pub struct TlsClient {
    config: Arc<ClientConfig>,
}

impl std::fmt::Debug for TlsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TlsClient")
    }
}

impl TlsClient {
    /// System trust store, plus the certificates of `extra_ca_pem` if given (an enterprise CA that
    /// is not installed system-wide).
    pub fn new(extra_ca_pem: Option<&[u8]>) -> Result<Self, NetError> {
        let mut roots = RootCertStore::empty();
        let native = rustls_native_certs::load_native_certs();
        for cert in native.certs {
            // Single unparsable system certificates are skipped, like other TLS stacks do.
            let _ = roots.add(cert);
        }
        if let Some(pem) = extra_ca_pem {
            let mut n = 0;
            for cert in CertificateDer::pem_slice_iter(pem) {
                let cert = cert.map_err(|e| NetError::BadCaFile(format!("{e:?}")))?;
                roots
                    .add(cert)
                    .map_err(|e| NetError::BadCaFile(e.to_string()))?;
                n += 1;
            }
            if n == 0 {
                return Err(NetError::BadCaFile("no certificate in file".into()));
            }
        }
        Self::with_roots(roots)
    }

    /// Only the given roots (tests, or a user who wants to pin the enterprise CA).
    pub fn with_roots(roots: RootCertStore) -> Result<Self, NetError> {
        if roots.is_empty() {
            return Err(NetError::NoTrustAnchors);
        }
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
            .map_err(|e| NetError::Tls(std::io::Error::other(e)))?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Resolves `host` and connects to the first address that accepts the connection.
    pub fn connect(
        &self,
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<SipConnection, NetError> {
        let host = commscraft_url::dns_host(host).ok_or(NetError::BadHostName)?;
        let addrs: Vec<SocketAddr> = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(NetError::Resolve)?
            .collect();
        let mut last = None;
        for addr in addrs {
            match self.connect_addr(addr, &host, timeout) {
                Ok(c) => return Ok(c),
                Err(e @ NetError::Connect(_)) => last = Some(e),
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap_or(NetError::Resolve(std::io::Error::other("no address"))))
    }

    /// Connects to `addr` and verifies the certificate for `server_name`.
    pub fn connect_addr(
        &self,
        addr: SocketAddr,
        server_name: &str,
        timeout: Duration,
    ) -> Result<SipConnection, NetError> {
        let name = commscraft_url::dns_host(server_name).ok_or(NetError::BadHostName)?;
        let name = ServerName::try_from(name).map_err(|_| NetError::BadHostName)?;
        let tcp = TcpStream::connect_timeout(&addr, timeout).map_err(NetError::Connect)?;
        tcp.set_read_timeout(Some(timeout)).map_err(NetError::Io)?;
        tcp.set_write_timeout(Some(timeout)).map_err(NetError::Io)?;
        tcp.set_nodelay(true).map_err(NetError::Io)?;
        let conn = ClientConnection::new(self.config.clone(), name)
            .map_err(|e| NetError::Tls(std::io::Error::other(e)))?;
        let mut tls = StreamOwned::new(conn, tcp);
        // Complete the handshake now, so certificate errors surface here.
        while tls.conn.is_handshaking() {
            tls.conn.complete_io(&mut tls.sock).map_err(NetError::Tls)?;
        }
        Ok(SipConnection {
            tls,
            decoder: StreamDecoder::new(Limits::default()),
        })
    }
}

/// A TLS connection carrying SIP (blocking).
pub struct SipConnection {
    tls: StreamOwned<ClientConnection, TcpStream>,
    decoder: StreamDecoder,
}

impl std::fmt::Debug for SipConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SipConnection")
    }
}

impl SipConnection {
    /// Negotiated TLS version, for diagnostics.
    pub fn protocol_version(&self) -> Option<String> {
        self.tls.conn.protocol_version().map(|v| format!("{v:?}"))
    }

    /// Local address of the TCP connection (for Via and Contact).
    pub fn local_addr(&self) -> Result<SocketAddr, NetError> {
        self.tls.sock.local_addr().map_err(NetError::Io)
    }

    pub fn send(&mut self, msg: &Message) -> Result<(), NetError> {
        let bytes = msg.to_bytes().map_err(NetError::Sip)?;
        self.send_raw(&bytes)
    }

    pub fn send_raw(&mut self, bytes: &[u8]) -> Result<(), NetError> {
        self.tls.write_all(bytes).map_err(NetError::Io)?;
        self.tls.flush().map_err(NetError::Io)
    }

    /// Waits up to the socket timeout for the next complete message; `Ok(None)` on timeout.
    pub fn recv(&mut self) -> Result<Option<Message>, NetError> {
        loop {
            if let Some(m) = self.decoder.next_message().map_err(NetError::Sip)? {
                return Ok(Some(m));
            }
            let mut buf = [0u8; 16 * 1024];
            match self.tls.read(&mut buf) {
                Ok(0) => return Err(NetError::Closed),
                Ok(n) => {
                    let data = buf.get(..n).unwrap_or_default();
                    self.decoder.push(data).map_err(NetError::Sip)?;
                }
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    return Ok(None)
                }
                Err(e) => return Err(NetError::Io(e)),
            }
        }
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), NetError> {
        self.tls
            .sock
            .set_read_timeout(Some(timeout))
            .map_err(NetError::Io)
    }
}

/// The OS CSPRNG for the protocol crates (`commscraft_register::Random`).
#[derive(Debug, Default, Clone, Copy)]
pub struct OsRandom;

impl OsRandom {
    /// Checks once that the OS random source works.
    pub fn new() -> Result<Self, NetError> {
        let mut probe = [0u8; 16];
        getrandom::fill(&mut probe)
            .map_err(|e| NetError::Io(std::io::Error::other(e.to_string())))?;
        Ok(Self)
    }
}

impl commscraft_register::Random for OsRandom {
    fn fill(&mut self, buf: &mut [u8]) {
        if getrandom::fill(buf).is_err() {
            // Fail closed: never continue with predictable nonces or keys.
            std::process::abort();
        }
    }
}

#[cfg(test)]
mod tests;
