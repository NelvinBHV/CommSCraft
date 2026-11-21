//! Network adapter: TLS connections for SIP, the OS random source and a blocking SIP connection.
//!
//! This is the first adapter crate: protocol crates stay sans-I/O, this crate owns
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
    roots: Arc<RootCertStore>,
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
        let roots = Arc::new(roots);
        let config = ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
            .map_err(|e| NetError::Tls(std::io::Error::other(e)))?
            .with_root_certificates(roots.clone())
            .with_no_client_auth();
        Ok(Self {
            config: Arc::new(config),
            roots,
        })
    }

    /// The trust anchors, for TLS run elsewhere with the same trust (the TLS-DSK handshake).
    pub fn roots(&self) -> Arc<RootCertStore> {
        self.roots.clone()
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
        Ok(SipConnection {
            tls: self.handshake(addr, server_name, timeout)?,
            decoder: StreamDecoder::new(Limits::default()),
            compression: None,
        })
    }

    /// A TLS connection for raw bytes (the PSOM connection of the web conferencing MCU):
    /// resolves `host`, connects to the first address that accepts, and verifies the
    /// certificate for `certificate_name` – the `alternativeName` the server names in place of
    /// the host ([MS-PSOM] 20250520 §3.2.3.1.1.1, p. 53).
    pub fn connect_stream(
        &self,
        host: &str,
        port: u16,
        certificate_name: &str,
        timeout: Duration,
    ) -> Result<TlsStream, NetError> {
        let host = commscraft_url::dns_host(host).ok_or(NetError::BadHostName)?;
        let addrs: Vec<SocketAddr> = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(NetError::Resolve)?
            .collect();
        let mut last = None;
        for addr in addrs {
            match self.handshake(addr, certificate_name, timeout) {
                Ok(tls) => return Ok(TlsStream { tls }),
                Err(e @ NetError::Connect(_)) => last = Some(e),
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap_or(NetError::Resolve(std::io::Error::other("no address"))))
    }

    fn handshake(
        &self,
        addr: SocketAddr,
        server_name: &str,
        timeout: Duration,
    ) -> Result<StreamOwned<ClientConnection, TcpStream>, NetError> {
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
        Ok(tls)
    }
}

/// A TLS connection carrying raw bytes (blocking).
pub struct TlsStream {
    tls: StreamOwned<ClientConnection, TcpStream>,
}

impl TlsStream {
    pub fn send(&mut self, bytes: &[u8]) -> Result<(), NetError> {
        use std::io::Write;
        self.tls.write_all(bytes).map_err(NetError::Io)?;
        self.tls.flush().map_err(NetError::Io)
    }

    /// Reads what is there into `buf`: `Ok(0)` when the peer closed, `Ok(None)` on a timeout.
    pub fn recv(&mut self, buf: &mut [u8]) -> Result<Option<usize>, NetError> {
        use std::io::Read;
        match self.tls.read(buf) {
            Ok(n) => Ok(Some(n)),
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(NetError::Io(e)),
        }
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), NetError> {
        self.tls
            .sock
            .set_read_timeout(Some(timeout))
            .map_err(NetError::Io)
    }
}

/// A TLS connection carrying SIP (blocking).
pub struct SipConnection {
    tls: StreamOwned<ClientConnection, TcpStream>,
    decoder: StreamDecoder,
    /// SIP compression, once negotiated ([MS-SIPCOMP], function 3).
    compression: Option<Compression>,
}

#[derive(Default)]
struct Compression {
    inbound: commscraft_sipcomp::Inbound,
    outbound: commscraft_sipcomp::Outbound,
}

/// Packet counts of a compressed connection (diagnostics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressionStats {
    pub sent: u64,
    pub sent_compressed: u64,
    pub received: u64,
    pub received_compressed: u64,
}

/// Timer F for the NEGOTIATE transaction: 5 s in place of 32 s ([MS-SIPCOMP] 20250520 §3.1,
/// p. 11, CP-03).
const NEGOTIATE_TIMEOUT: Duration = Duration::from_secs(5);

impl SipConnection {
    /// One read that ends by `deadline` at the latest (issue #210): the socket's read timeout is
    /// lowered to what is left, and past the deadline the read fails with `TimedOut` – a peer
    /// that trickles a few octets before each socket timeout cannot hold the reader longer.
    pub(crate) fn read_by(
        &mut self,
        buf: &mut [u8],
        deadline: std::time::Instant,
    ) -> std::io::Result<usize> {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() {
            return Err(std::io::Error::new(
                ErrorKind::TimedOut,
                "the response took longer than allowed",
            ));
        }
        let per_read = self.tls.sock.read_timeout()?.unwrap_or(left).min(left);
        self.tls.sock.set_read_timeout(Some(per_read))?;
        self.tls.read(buf)
    }

    /// The server's end-entity certificate (DER) as sent in the TLS handshake.
    pub fn server_certificate(&self) -> Option<Vec<u8>> {
        self.tls
            .conn
            .peer_certificates()?
            .first()
            .map(|c| c.as_ref().to_vec())
    }
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
        if let Some(c) = &mut self.compression {
            // CP-04: compressed only after the server's first compressed packet.
            let framed = c
                .outbound
                .frame(bytes, c.inbound.peer_compressed)
                .map_err(compression_error)?;
            self.tls.write_all(&framed).map_err(NetError::Io)?;
        } else {
            self.tls.write_all(bytes).map_err(NetError::Io)?;
        }
        self.tls.flush().map_err(NetError::Io)
    }

    /// Asks the server for SIP compression ([MS-SIPCOMP] 20250520 §3.1.4.1, p. 11): a NEGOTIATE
    /// as the first SIP message after the TLS handshake (CP-01, CP-02), answered within 5 s.
    /// `Ok(true)`: compression is on for the rest of the connection. `Ok(false)`: no or another
    /// answer in time – the connection goes on uncompressed (CP-03). A 200 without `LZ77-8K` is
    /// an error, the caller closes the connection (CP-03). Call it right after connecting.
    pub fn negotiate_compression(&mut self, from: &str, tag: &str) -> Result<bool, NetError> {
        // Injection-safe: both go into header lines unquoted.
        if from
            .chars()
            .chain(tag.chars())
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '<' | '>' | '"'))
        {
            return Err(NetError::Io(std::io::Error::new(
                ErrorKind::InvalidInput,
                "unusable From for NEGOTIATE",
            )));
        }
        let local = self.local_addr()?;
        let peer = self.tls.sock.peer_addr().map_err(NetError::Io)?;
        let mut id = [0u8; 16];
        getrandom::fill(&mut id).map_err(|e| NetError::Io(std::io::Error::other(e.to_string())))?;
        let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
        let (branch, call_id) = hex.split_at(12);
        let request = format!(
            "NEGOTIATE sip:{peer} SIP/2.0\r\nVia: SIP/2.0/TLS {local};branch=z9hG4bK{branch}\r\n\
             Max-Forwards: 0\r\nFrom: <{from}>;tag={tag}\r\nTo: <sip:{peer}>\r\n\
             Call-ID: {call_id}\r\nCSeq: 1 NEGOTIATE\r\nCompression: LZ77-8K\r\n\
             Content-Length: 0\r\n\r\n"
        );
        let before = self.tls.sock.read_timeout().map_err(NetError::Io)?;
        self.tls
            .sock
            .set_read_timeout(Some(NEGOTIATE_TIMEOUT))
            .map_err(NetError::Io)?;
        self.send_raw(request.as_bytes())?;
        let deadline = std::time::Instant::now() + NEGOTIATE_TIMEOUT;
        let mut answer = None;
        while std::time::Instant::now() < deadline {
            match self.recv()? {
                Some(m)
                    if m.is_response()
                        && m.cseq()
                            .ok()
                            .flatten()
                            .is_some_and(|c| c.method == "NEGOTIATE") =>
                {
                    answer = Some(m);
                    break;
                }
                Some(_) => {}
                None => break,
            }
        }
        self.tls
            .sock
            .set_read_timeout(before)
            .map_err(NetError::Io)?;
        let Some(m) = answer.filter(|m| m.status_code() == Some(200)) else {
            return Ok(false);
        };
        let lz = m
            .header_values("Compression")
            .flat_map(|v| v.split(','))
            .any(|v| v.trim().eq_ignore_ascii_case("LZ77-8K"));
        if !lz {
            return Err(NetError::Io(std::io::Error::other(
                "the server accepted compression without naming LZ77-8K",
            )));
        }
        self.compression = Some(Compression::default());
        Ok(true)
    }

    /// Packet counts while SIP compression is on.
    pub fn compression_stats(&self) -> Option<CompressionStats> {
        self.compression.as_ref().map(|c| CompressionStats {
            sent: c.outbound.packets,
            sent_compressed: c.outbound.compressed,
            received: c.inbound.packets,
            received_compressed: c.inbound.compressed,
        })
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
                    match &mut self.compression {
                        Some(c) => {
                            let plain = c.inbound.push(data).map_err(compression_error)?;
                            self.decoder.push(&plain).map_err(NetError::Sip)?;
                        }
                        None => self.decoder.push(data).map_err(NetError::Sip)?,
                    }
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

/// A compression error closes the connection ([MS-SIPCOMP] §3.2.5.2, CP-08).
fn compression_error(e: commscraft_sipcomp::Error) -> NetError {
    NetError::Io(std::io::Error::new(
        ErrorKind::InvalidData,
        format!("SIP compression: {e:?}"),
    ))
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
