//! The TLS-DSK sign-in (function 6) as an adapter: the user's key pair and certificate request for
//! the certificate provisioning service, the TLS handshake that MS-SIPAE tunnels in
//! `gssapi-data`, and the signing keys derived from it.
//!
//! Evidence: `docs/protocol/evidence/03-sipae-authentication.md` (AE-26, AE-27, AE-30),
//! `09-ocauthws-webticket-certificate.md` (AW-10 … AW-15). Sources: [MS-SIPAE] 20250520
//! §3.2.5.1 steps 3–5 (pp. 24–25), §3.2.4.1 step 3 (HMAC, p. 22), example §4.4 (pp. 47–50);
//! [MS-OCAUTHWS] 20250520 §3.1.4.1 (pp. 25–30).
//!
//! - **Keys:** RSA 2048 (like the certificates the service issues; inference). The `rsa` crate only
//!   generates the key pair; every private key operation – signing the certificate request, the
//!   TLS certificate_verify – runs in `ring`, which works in constant time.
//! - **Certificate request:** PKCS#10 with the subject CN = the entity (the SIP URI without `sip:`,
//!   AW-13), key usage digital signature and key encipherment, extended key usage client
//!   authentication (the service refuses other EKUs: InvalidEKU), signed with SHA-256.
//! - **Handshake:** rustls as the TLS client over buffers instead of a socket, TLS 1.2 only (the
//!   keys come from the TLS 1.2 PRF, AE-27) with SHA-256 cipher suites only, so the HMAC and the key
//!   length are those of SHA-256 (K-AE-3). The server certificate is checked against the same trust
//!   anchors and the same name rules as the SIP connection (the challenge's `targetname`).
//! - **Signing keys:** PRF(master secret, "client EAP encryption", client random ‖ server random)
//!   to 128 bytes – in TLS 1.2 exactly the keying material exporter of RFC 5705 with that label and
//!   no context; client key = bytes 64–95, server key = bytes 96–127; HMAC-SHA256.
//!
//! Security by design: the private key is kept in a wiped buffer and written only with mode 0600
//! in a directory with mode 0700; a stored certificate is used only while it is valid and only if
//! it carries the stored key; keys and tokens are never printed.

mod der;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use commscraft_register::{TlsDskHandshake, TlsDskProvider, TlsDskStart, TlsDskStep};
use commscraft_sipae::{Error as SipaeError, Mic};
use hmac::{Hmac, Mac};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// RSA modulus length of the user key (bits).
pub const KEY_BITS: usize = 2048;
/// The exporter label of the TLS-DSK keys (AE-27).
const KEY_LABEL: &[u8] = b"client EAP encryption";
/// Largest token accepted from the server and size of the TLS output per step (design).
const MAX_TOKEN: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    KeyGeneration,
    Encoding,
    Signing,
    Storage(String),
    Certificate(&'static str),
}

/// The user key and certificate of TLS-DSK.
pub struct Identity {
    pub certificate: Vec<u8>,
    key_pkcs8: Zeroizing<Vec<u8>>,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Identity { certificate, key: <redacted> }")
    }
}

/// A new key pair and its certificate request (DER) for `entity`. The key stays with the caller
/// until the certificate arrives ([`Identity::new`]).
pub struct Request {
    pub csr: Vec<u8>,
    key_pkcs8: Zeroizing<Vec<u8>>,
    public_key_info: Vec<u8>,
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Request { csr, key: <redacted> }")
    }
}

impl Request {
    /// Generates the key pair and the PKCS#10 request.
    pub fn generate(entity: &str) -> Result<Request, Error> {
        use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
        let mut rng = rand_core::OsRng;
        let key = rsa::RsaPrivateKey::new(&mut rng, KEY_BITS).map_err(|_| Error::KeyGeneration)?;
        let key_pkcs8 = Zeroizing::new(
            key.to_pkcs8_der()
                .map_err(|_| Error::Encoding)?
                .as_bytes()
                .to_vec(),
        );
        let public_key_info = key
            .to_public_key()
            .to_public_key_der()
            .map_err(|_| Error::Encoding)?
            .as_bytes()
            .to_vec();
        drop(key);
        let info = der::certification_request_info(entity, &public_key_info);
        let signature = sign_sha256(&key_pkcs8, &info)?;
        Ok(Request {
            csr: der::certification_request(&info, &signature),
            key_pkcs8,
            public_key_info,
        })
    }
}

/// RSASSA-PKCS1-v1_5 with SHA-256 by `ring`.
fn sign_sha256(key_pkcs8: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    let pair = ring::rsa::KeyPair::from_pkcs8(key_pkcs8).map_err(|_| Error::Signing)?;
    let mut sig = vec![0u8; pair.public().modulus_len()];
    pair.sign(
        &ring::signature::RSA_PKCS1_SHA256,
        &ring::rand::SystemRandom::new(),
        data,
        &mut sig,
    )
    .map_err(|_| Error::Signing)?;
    Ok(sig)
}

/// Seconds since 1970 now.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl Identity {
    /// The issued certificate for a request: accepted only if it carries the request's public key
    /// and is valid now.
    pub fn new(request: Request, certificate: Vec<u8>) -> Result<Identity, Error> {
        let c = der::certificate_facts(&certificate).ok_or(Error::Certificate("unreadable"))?;
        if c.public_key_info != request.public_key_info {
            return Err(Error::Certificate("another public key"));
        }
        if !(c.not_before..c.not_after).contains(&now()) {
            return Err(Error::Certificate("not valid now"));
        }
        Ok(Identity {
            certificate,
            key_pkcs8: request.key_pkcs8,
        })
    }

    /// Seconds the certificate stays valid.
    pub fn remaining(&self) -> Option<u64> {
        der::certificate_facts(&self.certificate).map(|c| c.not_after.saturating_sub(now()))
    }

    /// Writes certificate and key to `dir` (created 0700; files 0600).
    pub fn store(&self, dir: &Path) -> Result<(), Error> {
        use std::io::Write;
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|e| Error::Storage(e.to_string()))?;
        for (name, data) in [
            ("certificate.der", &self.certificate[..]),
            ("key.pk8", &self.key_pkcs8[..]),
        ] {
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(dir.join(name))
                .and_then(|mut f| f.write_all(data))
                .map_err(|e| Error::Storage(e.to_string()))?;
        }
        Ok(())
    }

    /// The stored identity, if it is still valid (at least `margin` seconds) and the key fits.
    pub fn load(dir: &Path, margin: u64) -> Option<Identity> {
        let certificate = std::fs::read(dir.join("certificate.der")).ok()?;
        let key_pkcs8 = Zeroizing::new(std::fs::read(dir.join("key.pk8")).ok()?);
        let c = der::certificate_facts(&certificate)?;
        let pair = ring::rsa::KeyPair::from_pkcs8(&key_pkcs8).ok()?;
        let fits = der::rsa_public_key_of(&c.public_key_info)
            .is_some_and(|k| k == ring::signature::KeyPair::public_key(&pair).as_ref());
        (fits && now() >= c.not_before && now().saturating_add(margin) < c.not_after).then_some(
            Identity {
                certificate,
                key_pkcs8,
            },
        )
    }
}

/// The registrar's TLS-DSK provider: the stored identity of the account, the trust anchors of
/// the SIP connection.
pub struct TlsDsk {
    dir: PathBuf,
    roots: Arc<RootCertStore>,
}

impl TlsDsk {
    pub fn new(dir: PathBuf, roots: Arc<RootCertStore>) -> TlsDsk {
        TlsDsk { dir, roots }
    }
}

/// Renew the certificate when less than this is left (design: a day).
pub const RENEW_MARGIN: u64 = 86_400;

impl TlsDskProvider for TlsDsk {
    fn start(&mut self, target: &str) -> Result<TlsDskStart, String> {
        let Some(identity) = Identity::load(&self.dir, RENEW_MARGIN) else {
            return Ok(TlsDskStart::NeedCertificate);
        };
        Ok(TlsDskStart::Handshake(Box::new(Handshake::new(
            target,
            identity,
            self.roots.clone(),
        )?)))
    }
}

/// The TLS client of one TLS-DSK handshake.
pub struct Handshake {
    conn: ClientConnection,
    lifetime: Option<u64>,
}

impl Handshake {
    pub fn new(
        target: &str,
        identity: Identity,
        roots: Arc<RootCertStore>,
    ) -> Result<Self, String> {
        let lifetime = identity.remaining();
        let mut provider = rustls::crypto::ring::default_provider();
        // TLS 1.2 suites with SHA-256 only (AE-27, K-AE-3).
        provider.cipher_suites.retain(|s| {
            matches!(
                s.suite(),
                rustls::CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
                    | rustls::CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
                    | rustls::CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256
                    | rustls::CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256
            )
        });
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(identity.key_pkcs8.to_vec()));
        let config = ClientConfig::builder_with_provider(Arc::new(provider))
            .with_protocol_versions(&[&rustls::version::TLS12])
            .map_err(|e| e.to_string())?
            .with_root_certificates(roots)
            .with_client_auth_cert(vec![CertificateDer::from(identity.certificate)], key)
            .map_err(|e| e.to_string())?;
        let name = ServerName::try_from(target.to_string()).map_err(|e| e.to_string())?;
        let conn = ClientConnection::new(Arc::new(config), name).map_err(|e| e.to_string())?;
        Ok(Handshake { conn, lifetime })
    }

    /// What TLS wants to send now.
    fn output(&mut self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        while self.conn.wants_write() {
            self.conn.write_tls(&mut out).map_err(|e| e.to_string())?;
            if out.len() > MAX_TOKEN {
                return Err("TLS output too large".into());
            }
        }
        Ok(out)
    }

    /// The signing keys once the handshake is complete (AE-27).
    fn keys(&self) -> Result<TlsDskMic, String> {
        let mut block = Zeroizing::new([0u8; 128]);
        self.conn
            .export_keying_material(&mut block[..], KEY_LABEL, None)
            .map_err(|e| e.to_string())?;
        let mut mic = TlsDskMic {
            client: Zeroizing::new([0u8; 32]),
            server: Zeroizing::new([0u8; 32]),
        };
        let (Some(client), Some(server)) = (block.get(64..96), block.get(96..128)) else {
            return Err("key block".into());
        };
        mic.client.copy_from_slice(client);
        mic.server.copy_from_slice(server);
        Ok(mic)
    }
}

impl TlsDskHandshake for Handshake {
    fn hello(&mut self) -> Result<Vec<u8>, String> {
        self.output()
    }

    fn step(&mut self, token: &[u8]) -> Result<TlsDskStep, String> {
        if token.len() > MAX_TOKEN {
            return Err("TLS token too large".into());
        }
        let mut input = token;
        while !input.is_empty() {
            let n = self.conn.read_tls(&mut input).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            self.conn.process_new_packets().map_err(|e| e.to_string())?;
        }
        if self.conn.is_handshaking() {
            let out = self.output()?;
            if out.is_empty() {
                return Err("the TLS handshake stalled".into());
            }
            return Ok(TlsDskStep::Send(out));
        }
        Ok(TlsDskStep::Done {
            mic: Box::new(self.keys()?),
            lifetime: self.lifetime,
        })
    }
}

/// HMAC-SHA256 signatures with the TLS-DSK keys (AE-30).
pub struct TlsDskMic {
    client: Zeroizing<[u8; 32]>,
    server: Zeroizing<[u8; 32]>,
}

impl std::fmt::Debug for TlsDskMic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TlsDskMic { keys: <redacted> }")
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, SipaeError> {
    let mut m = Hmac::<Sha256>::new_from_slice(key).map_err(|_| SipaeError::WrongState)?;
    m.update(data);
    Ok(m.finalize().into_bytes().to_vec())
}

impl Mic for TlsDskMic {
    fn get_mic(&mut self, buffer: &[u8]) -> Result<Vec<u8>, SipaeError> {
        hmac_sha256(&self.client[..], buffer)
    }

    fn verify_mic(&mut self, buffer: &[u8], token: &[u8]) -> bool {
        hmac_sha256(&self.server[..], buffer)
            .is_ok_and(|e| e.len() == token.len() && bool::from(e.ct_eq(token)))
    }
}

#[cfg(test)]
mod tests;
