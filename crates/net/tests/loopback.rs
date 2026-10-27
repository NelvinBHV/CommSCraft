//! TLS over the loopback interface with a throw-away test CA (generated with the `openssl` command
//! at test time; no key material is committed). No external server is contacted.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)] // test helpers outside #[test] functions

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use commscraft_net::{NetError, TlsClient};
use commscraft_sip_msg::{Limits, Message, StreamDecoder};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::RootCertStore;

const NAME: &str = "pool.contoso.test";

fn openssl(dir: &Path, args: &[&str]) -> bool {
    Command::new("openssl")
        .current_dir(dir)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Creates CA and leaf certificate; `None` if the openssl command is not available.
fn make_certs(tag: &str) -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("tls-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    std::fs::write(
        dir.join("ext.cnf"),
        format!("subjectAltName=DNS:{NAME}\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth\n"),
    )
    .ok()?;
    let ok = openssl(
        &dir,
        &[
            "req",
            "-x509",
            "-newkey",
            "ec",
            "-pkeyopt",
            "ec_paramgen_curve:P-256",
            "-nodes",
            "-keyout",
            "ca.key",
            "-out",
            "ca.pem",
            "-days",
            "2",
            "-subj",
            "/CN=CommSCraft Test CA",
            "-addext",
            "basicConstraints=critical,CA:TRUE",
            "-addext",
            "keyUsage=critical,keyCertSign",
        ],
    ) && openssl(
        &dir,
        &[
            "req",
            "-newkey",
            "ec",
            "-pkeyopt",
            "ec_paramgen_curve:P-256",
            "-nodes",
            "-keyout",
            "leaf.key",
            "-out",
            "leaf.csr",
            "-subj",
            &format!("/CN={NAME}"),
        ],
    ) && openssl(
        &dir,
        &[
            "x509",
            "-req",
            "-in",
            "leaf.csr",
            "-CA",
            "ca.pem",
            "-CAkey",
            "ca.key",
            "-CAcreateserial",
            "-out",
            "leaf.pem",
            "-days",
            "2",
            "-extfile",
            "ext.cnf",
        ],
    );
    ok.then_some(dir)
}

fn server(
    dir: &Path,
) -> (
    std::net::SocketAddr,
    std::thread::JoinHandle<Option<Vec<u8>>>,
) {
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(dir.join("leaf.pem"))
        .unwrap()
        .map(|c| c.unwrap())
        .collect();
    let key = PrivateKeyDer::from_pem_file(dir.join("leaf.key")).unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (tcp, _) = listener.accept().ok()?;
        tcp.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
        let conn = rustls::ServerConnection::new(Arc::new(config)).ok()?;
        let mut tls = rustls::StreamOwned::new(conn, tcp);
        let mut decoder = StreamDecoder::new(Limits::default());
        let mut buf = [0u8; 4096];
        loop {
            if let Ok(Some(m)) = decoder.next_message() {
                let mut ok = Message::response(200, "OK");
                ok.push_header("Call-ID", m.call_id().ok()??);
                tls.write_all(&ok.to_bytes().ok()?).ok()?;
                tls.flush().ok()?;
                return m.to_bytes().ok();
            }
            let n = tls.read(&mut buf).ok()?;
            if n == 0 {
                return None;
            }
            decoder.push(&buf[..n]).ok()?;
        }
    });
    (addr, handle)
}

fn roots(dir: &Path) -> RootCertStore {
    let mut r = RootCertStore::empty();
    for c in CertificateDer::pem_file_iter(dir.join("ca.pem")).unwrap() {
        r.add(c.unwrap()).unwrap();
    }
    r
}

#[test]
fn verified_tls_round_trip() {
    let Some(dir) = make_certs("ok") else {
        eprintln!("openssl not available; skipping");
        return;
    };
    let (addr, handle) = server(&dir);
    let client = TlsClient::with_roots(roots(&dir)).unwrap();
    let mut conn = client
        .connect_addr(addr, NAME, Duration::from_secs(5))
        .unwrap();
    assert!(conn
        .protocol_version()
        .is_some_and(|v| v.contains("TLSv1_3") || v.contains("TLSv1_2")));
    let mut req = Message::request("OPTIONS", "sip:pool.contoso.test");
    req.push_header("Call-ID", "abc");
    conn.send_raw(b"\r\n\r\n").unwrap(); // a keep-alive before the request is ignored
    conn.send(&req).unwrap();
    let resp = conn.recv().unwrap().unwrap();
    assert_eq!(resp.status_code(), Some(200));
    assert_eq!(resp.call_id().unwrap(), Some("abc"));
    assert_eq!(handle.join().unwrap(), Some(req.to_bytes().unwrap()));
}

#[test]
fn certificate_problems_fail_closed() {
    let Some(dir) = make_certs("bad") else {
        eprintln!("openssl not available; skipping");
        return;
    };
    // Wrong host name.
    let (addr, _h) = server(&dir);
    let client = TlsClient::with_roots(roots(&dir)).unwrap();
    assert!(matches!(
        client.connect_addr(addr, "other.contoso.test", Duration::from_secs(5)),
        Err(NetError::Tls(_))
    ));
    // Unknown CA (system trust store only).
    let (addr, _h) = server(&dir);
    if let Ok(system) = TlsClient::new(None) {
        assert!(matches!(
            system.connect_addr(addr, NAME, Duration::from_secs(5)),
            Err(NetError::Tls(_))
        ));
    }
    // The CA given as enterprise CA file works together with the system store.
    let (addr, h) = server(&dir);
    let pem = std::fs::read(dir.join("ca.pem")).unwrap();
    let with_ca = TlsClient::new(Some(&pem)).unwrap();
    let mut conn = with_ca
        .connect_addr(addr, NAME, Duration::from_secs(5))
        .unwrap();
    let mut req = Message::request("OPTIONS", "sip:pool.contoso.test");
    req.push_header("Call-ID", "x");
    conn.send(&req).unwrap();
    assert!(conn.recv().unwrap().is_some());
    drop(h);
}

/// HTTPS GET as used for Autodiscover: request line and headers, chunked answer.
#[test]
fn https_get_for_autodiscover() {
    let Some(dir) = make_certs("http") else {
        eprintln!("openssl not available; skipping");
        return;
    };
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(dir.join("leaf.pem"))
        .unwrap()
        .map(|c| c.unwrap())
        .collect();
    let key = PrivateKeyDer::from_pem_file(dir.join("leaf.key")).unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(certs, key)
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (tcp, _) = listener.accept().unwrap();
        let conn = rustls::ServerConnection::new(Arc::new(config)).unwrap();
        let mut tls = rustls::StreamOwned::new(conn, tcp);
        let mut req = Vec::new();
        let mut b = [0u8; 1];
        while !req.ends_with(b"\r\n\r\n") {
            tls.read_exact(&mut b).unwrap();
            req.push(b[0]);
        }
        let body =
            "<AutodiscoverResponse AccessLocation=\"internal\"><Domain/></AutodiscoverResponse>";
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.microsoft.rtc.autodiscover+xml; v=1\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{body}\r\n0\r\n\r\n",
            body.len()
        );
        tls.write_all(resp.as_bytes()).unwrap();
        tls.conn.send_close_notify();
        let _ = tls.flush();
        String::from_utf8(req).unwrap()
    });
    let client = TlsClient::with_roots(roots(&dir)).unwrap();
    let url = commscraft_url::parse_https(&format!(
        "https://{NAME}:{}/?sipuri=john@contoso.test",
        addr.port()
    ))
    .unwrap();
    let r = commscraft_net::http::get_at(
        &client,
        addr,
        &url,
        "application/vnd.microsoft.rtc.autodiscover+xml;v=1",
        "CommSCraft/test",
        Duration::from_secs(5),
    )
    .unwrap();
    assert_eq!(r.status, 200);
    assert!(String::from_utf8(r.body)
        .unwrap()
        .starts_with("<AutodiscoverResponse"));
    let req = handle.join().unwrap();
    assert!(
        req.starts_with("GET /?sipuri=john@contoso.test HTTP/1.1\r\n"),
        "{req}"
    );
    assert!(req.contains(&format!("Host: {NAME}:{}\r\n", addr.port())));
    assert!(req.contains("Accept: application/vnd.microsoft.rtc.autodiscover+xml;v=1\r\n"));
    assert!(req.contains("Connection: close\r\n"));
}
