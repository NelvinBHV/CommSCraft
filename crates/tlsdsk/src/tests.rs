use super::*;
use rustls::server::WebPkiClientVerifier;
use rustls::{ServerConfig, ServerConnection};

#[test]
fn certificate_times() {
    assert_eq!(
        der::time_for_test(0x17, b"260101000000Z"),
        Some(1_767_225_600)
    );
    assert_eq!(
        der::time_for_test(0x18, b"20000301120000Z"),
        Some(951_912_000)
    );
    assert_eq!(der::time_for_test(0x17, b"261301000000Z"), None);
    assert_eq!(der::time_for_test(0x17, b"2601010000Z"), None);
}

struct Pki {
    ca: rcgen::Certificate,
    server: rcgen::Certificate,
    server_key: rcgen::KeyPair,
    user: Vec<u8>,
}

/// A CA, a server certificate for `fe.contoso.test` and the user certificate issued for our
/// request – as the certificate provisioning service would.
fn pki(request: &Request) -> Pki {
    use rcgen::{
        BasicConstraints, CertificateParams, CertificateSigningRequestParams, IsCa, Issuer, KeyPair,
    };
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca = ca_params.self_signed(&ca_key).unwrap();
    let issuer = Issuer::new(ca_params, ca_key);
    let server_key = KeyPair::generate().unwrap();
    let server = CertificateParams::new(vec!["fe.contoso.test".to_string()])
        .unwrap()
        .signed_by(&server_key, &issuer)
        .unwrap();
    // A real parser reads our request and checks its signature.
    let csr = CertificateSigningRequestParams::from_der(&request.csr.clone().into()).unwrap();
    let user = csr.signed_by(&issuer).unwrap().der().to_vec();
    Pki {
        ca,
        server,
        server_key,
        user,
    }
}

/// AE-26/AE-27: the full handshake against a TLS 1.2 server that asks for the user certificate;
/// both sides derive the same signing keys from the exporter "client EAP encryption", and each
/// side's signature verifies at the other.
#[test]
fn handshake_and_signing_keys() {
    let request = Request::generate("alice@contoso.test").unwrap();
    let p = pki(&request);
    let identity = Identity::new(request, p.user.clone()).unwrap();
    assert!(identity.remaining().unwrap() > 86_400);

    let mut roots = RootCertStore::empty();
    roots.add(p.ca.der().clone()).unwrap();
    let roots = Arc::new(roots);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = WebPkiClientVerifier::builder_with_provider(roots.clone(), provider.clone())
        .build()
        .unwrap();
    let server_config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS12])
        .unwrap()
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![p.server.der().clone()],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(p.server_key.serialize_der())),
        )
        .unwrap();
    let mut server = ServerConnection::new(Arc::new(server_config)).unwrap();
    let mut client = Handshake::new("fe.contoso.test", identity, roots).unwrap();

    let mut to_server = client.hello().unwrap();
    assert_eq!(to_server.first(), Some(&0x16), "a TLS handshake record");
    let mut mic = None;
    for _ in 0..4 {
        let mut input = &to_server[..];
        while !input.is_empty() {
            server.read_tls(&mut input).unwrap();
            server.process_new_packets().unwrap();
        }
        let mut to_client = Vec::new();
        while server.wants_write() {
            server.write_tls(&mut to_client).unwrap();
        }
        match client.step(&to_client).unwrap() {
            TlsDskStep::Send(t) => to_server = t,
            TlsDskStep::Done { mic: m, lifetime } => {
                assert!(lifetime.is_some());
                mic = Some(m);
                break;
            }
        }
    }
    let mut mic = mic.expect("handshake complete");
    assert!(!server.is_handshaking());
    let mut block = [0u8; 128];
    server
        .export_keying_material(&mut block[..], b"client EAP encryption", None)
        .unwrap();
    let buffer = b"<TLS-DSK><1d7d4ecf><1><SIP Communications Service><fe.contoso.test>";
    // The client signs with bytes 64–95: the server checks with the same key.
    let client_sig = mic.get_mic(buffer).unwrap();
    assert_eq!(client_sig, hmac_sha256(&block[64..96], buffer).unwrap());
    // The server signs with bytes 96–127: the client verifies it.
    let server_sig = hmac_sha256(&block[96..128], buffer).unwrap();
    assert!(mic.verify_mic(buffer, &server_sig));
    assert!(!mic.verify_mic(buffer, &client_sig));
}

/// A server certificate for another name ends the handshake (no TLS bypass).
#[test]
fn wrong_server_name_is_refused() {
    let request = Request::generate("alice@contoso.test").unwrap();
    let p = pki(&request);
    let identity = Identity::new(request, p.user.clone()).unwrap();
    let mut roots = RootCertStore::empty();
    roots.add(p.ca.der().clone()).unwrap();
    let roots = Arc::new(roots);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server_config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS12])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![p.server.der().clone()],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(p.server_key.serialize_der())),
        )
        .unwrap();
    let mut server = ServerConnection::new(Arc::new(server_config)).unwrap();
    let mut client = Handshake::new("other.contoso.test", identity, roots).unwrap();
    let hello = client.hello().unwrap();
    server.read_tls(&mut &hello[..]).unwrap();
    server.process_new_packets().unwrap();
    let mut to_client = Vec::new();
    while server.wants_write() {
        server.write_tls(&mut to_client).unwrap();
    }
    assert!(client.step(&to_client).is_err());
}

/// A certificate for another key is not taken over; the stored identity comes back only with
/// its key, and files are private.
#[test]
fn identity_checks_and_storage() {
    use std::os::unix::fs::PermissionsExt;
    let request = Request::generate("alice@contoso.test").unwrap();
    let other = Request::generate("alice@contoso.test").unwrap();
    let p = pki(&other);
    assert_eq!(
        Identity::new(request, p.user.clone()).unwrap_err(),
        Error::Certificate("another public key")
    );
    let identity = Identity::new(other, p.user.clone()).unwrap();
    let dir = std::env::temp_dir().join(format!("commscraft-tlsdsk-{}", std::process::id()));
    identity.store(&dir).unwrap();
    let mode = |f: &str| std::fs::metadata(dir.join(f)).unwrap().permissions().mode() & 0o777;
    assert_eq!((mode("certificate.der"), mode("key.pk8")), (0o600, 0o600));
    assert!(Identity::load(&dir, 60).is_some());
    // A validity margin beyond the certificate's end: renew instead.
    assert!(Identity::load(&dir, u64::MAX).is_none());
    // A key that does not fit the certificate.
    let foreign = Request::generate("alice@contoso.test").unwrap();
    std::fs::write(dir.join("key.pk8"), &foreign.key_pkcs8[..]).unwrap();
    assert!(Identity::load(&dir, 60).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
