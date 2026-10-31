use super::*;

#[test]
fn bad_inputs_are_refused() {
    assert!(matches!(
        TlsClient::new(Some(b"not a certificate")),
        Err(NetError::BadCaFile(_))
    ));
    assert!(matches!(
        TlsClient::with_roots(RootCertStore::empty()),
        Err(NetError::NoTrustAnchors)
    ));
    if let Ok(client) = TlsClient::new(None) {
        let t = Duration::from_millis(200);
        for host in ["192.0.2.1", "localhost", "a b.example", "[::1]"] {
            assert!(
                matches!(client.connect(host, 5061, t), Err(NetError::BadHostName)),
                "{host}"
            );
        }
        let addr: SocketAddr = "127.0.0.1:9".parse().unwrap();
        assert!(matches!(
            client.connect_addr(addr, "10.0.0.1", t),
            Err(NetError::BadHostName)
        ));
    }
}

#[test]
fn os_random_works() {
    use commscraft_register::Random;
    let mut r = OsRandom::new().unwrap();
    let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
    r.fill(&mut a);
    r.fill(&mut b);
    assert_ne!(a, b);
}
