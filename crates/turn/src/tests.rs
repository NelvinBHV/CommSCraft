use super::*;

const USER: &str = "edgeuser";
const PASS: &[u8] = b"edgepass";
const REALM: &[u8] = b"SIP Communications Service";
const NONCE: &[u8] = b"0123456789abcdef";

fn counter_random() -> Random {
    let mut n = 0u8;
    Box::new(move |b: &mut [u8]| {
        for x in b.iter_mut() {
            n = n.wrapping_add(17);
            *x = n;
        }
    })
}

fn client(transport: Transport) -> Client {
    Client::new(
        Credentials {
            username: USER.into(),
            password: Zeroizing::new(PASS.to_vec()),
        },
        transport,
        Stream::Audio,
        counter_random(),
    )
}

/// The server side: answers with the given attributes, authenticated with `version` rules.
fn reply(req: &Message, kind: u16, attrs: &[(u16, Vec<u8>)], sha256: Option<bool>) -> Vec<u8> {
    let mut m = Message::new(kind, req.transaction);
    m.push(
        stun::MS_MAGIC_COOKIE,
        stun::MS_MAGIC_COOKIE_VALUE.to_be_bytes().to_vec(),
    );
    for (k, v) in attrs {
        m.push(*k, v.clone());
    }
    let raw = m.encode().unwrap();
    match sha256 {
        None => raw,
        Some(true) => {
            let key = stun::ms_turn_sha256_key(NONCE, PASS, USER.as_bytes(), REALM);
            stun::with_integrity(raw, Integrity::LegacySha256(&key)).unwrap()
        }
        Some(false) => {
            let key = stun::long_term_key(USER.as_bytes(), REALM, PASS);
            stun::with_integrity(raw, Integrity::LegacySha1(&key)).unwrap()
        }
    }
}

fn challenge(req: &Message, version: u32) -> Vec<u8> {
    reply(
        req,
        ALLOCATE_ERROR,
        &[
            (stun::ERROR_CODE, stun::encode_error(401, "Unauthorized")),
            (stun::REALM, REALM.to_vec()),
            (stun::NONCE, NONCE.to_vec()),
            (stun::MS_VERSION, version.to_be_bytes().to_vec()),
        ],
        None,
    )
}

fn allocated(req: &Message, sha256: bool) -> Vec<u8> {
    let mut seq = vec![9u8; 20];
    seq.extend_from_slice(&0u32.to_be_bytes());
    reply(
        req,
        ALLOCATE_RESPONSE,
        &[
            (
                stun::MAPPED_ADDRESS,
                stun::encode_address("192.0.2.20:55667".parse().unwrap()),
            ),
            (
                stun::MS_XOR_MAPPED_ADDRESS,
                stun::encode_xor_address("192.0.2.10:54321".parse().unwrap(), &req.transaction),
            ),
            (stun::LIFETIME, 600u32.to_be_bytes().to_vec()),
            // turn-08 §8.2: the server names the peak bandwidth (kbit/s).
            (stun::BANDWIDTH, 1500u32.to_be_bytes().to_vec()),
            (stun::MS_SEQUENCE_NUMBER, seq),
            (stun::MS_VERSION, 3u32.to_be_bytes().to_vec()),
        ],
        Some(sha256),
    )
}

fn seq_of(m: &Message) -> u32 {
    let v = m.get(stun::MS_SEQUENCE_NUMBER).unwrap();
    assert_eq!(&v[..20], &[9u8; 20]);
    u32::from_be_bytes(v[20..24].try_into().unwrap())
}

/// [MS-TURN] §3.1.12 / §4 flow over UDP with HMAC-SHA256 (both sides version 3).
#[test]
fn allocate_send_activate_refresh_teardown() {
    let mut c = client(Transport::Udp);
    let first = c.start(0).unwrap();
    let req = parse(&first).expect("magic cookie first (§3.1.8)");
    assert_eq!(req.kind, ALLOCATE);
    assert!(
        req.get(stun::MESSAGE_INTEGRITY).is_none(),
        "§3.2.4.1: no MI"
    );
    assert_eq!(req.get(stun::MS_SERVICE_QUALITY), Some(&[0, 1, 0, 0][..]));

    let (w, e) = c.on_packet(&challenge(&req, 3), 10);
    assert!(e.is_empty());
    let auth = w[0].clone();
    let key = stun::ms_turn_sha256_key(NONCE, PASS, USER.as_bytes(), REALM);
    stun::verify_integrity(&auth, Integrity::LegacySha256(&key)).expect("SHA-256 after version 3");
    let req2 = parse(&auth).unwrap();
    assert_eq!(req2.get(stun::USERNAME), Some(USER.as_bytes()));

    let (_, e) = c.on_packet(&allocated(&req2, true), 20);
    assert_eq!(
        e,
        vec![Event::Allocated {
            relayed: "192.0.2.20:55667".parse().unwrap(),
            reflexive: Some("192.0.2.10:54321".parse().unwrap()),
            lifetime: 600
        }]
    );
    assert_eq!(c.bandwidth(), Some(1500));

    let peer: SocketAddr = "192.0.2.30:44556".parse().unwrap();
    let send = parse(&c.send(peer, b"rtp").unwrap()).unwrap();
    assert_eq!(send.kind, SEND);
    assert_eq!(seq_of(&send), 1, "sequence number echoed and incremented");
    assert_eq!(
        stun::decode_address(send.get(stun::DESTINATION_ADDRESS).unwrap()).unwrap(),
        peer
    );

    // Data Indication from the permitted peer only.
    let di = |from: SocketAddr| {
        reply(
            &send,
            DATA_INDICATION,
            &[
                (stun::REMOTE_ADDRESS, stun::encode_address(from)),
                (stun::DATA, b"pong".to_vec()),
            ],
            None,
        )
    };
    assert_eq!(
        c.on_packet(&di(peer), 30).1,
        vec![Event::Data {
            from: Some(peer),
            data: b"pong".to_vec()
        }]
    );
    assert!(c
        .on_packet(&di("198.51.100.9:1".parse().unwrap()), 30)
        .1
        .is_empty());

    let sad = parse(&c.set_active(peer, 40).unwrap()).unwrap();
    assert_eq!(seq_of(&sad), 2);
    let (_, e) = c.on_packet(
        &reply(&sad, SET_ACTIVE_DESTINATION_RESPONSE, &[], Some(true)),
        50,
    );
    assert_eq!(e, vec![Event::ActiveDestination(peer)]);
    assert_eq!(c.send_raw(b"raw").unwrap(), b"raw");
    assert_eq!(
        c.on_packet(b"\x80\x00media", 60).1,
        vec![Event::Data {
            from: Some(peer),
            data: b"\x80\x00media".to_vec()
        }]
    );

    // Refresh at 3/4 of 600 s.
    let (w, _) = c.on_tick(20 + 449_000);
    assert!(w.is_empty());
    let (w, _) = c.on_tick(20 + 450_000);
    let refresh = parse(&w[0]).unwrap();
    assert_eq!((refresh.kind, seq_of(&refresh)), (ALLOCATE, 3));
    let (_, e) = c.on_packet(&allocated(&refresh, true), 20 + 450_100);
    assert!(e.is_empty(), "a refresh is silent");

    let bye = parse(&c.close(500_000).unwrap()).unwrap();
    assert_eq!(bye.get(stun::LIFETIME), Some(&[0, 0, 0, 0][..]));
    c.on_packet(&allocated(&bye, true), 500_100);
    assert_eq!(c.relayed(), None);
}

#[test]
fn security_checks() {
    let mut c = client(Transport::Udp);
    let req = parse(&c.start(0).unwrap()).unwrap();
    let (w, _) = c.on_packet(&challenge(&req, 2), 1);
    let req2 = parse(&w[0]).unwrap();
    // Version 2 server: SHA-1 (TU-14).
    let key = stun::long_term_key(USER.as_bytes(), REALM, PASS);
    stun::verify_integrity(&w[0], Integrity::LegacySha1(&key)).unwrap();
    // A forged (wrongly authenticated or unauthenticated) success response is ignored.
    assert!(c.on_packet(&allocated(&req2, true), 2).1.is_empty());
    let mut unauth = Message::new(ALLOCATE_RESPONSE, req2.transaction);
    unauth.push(
        stun::MS_MAGIC_COOKIE,
        stun::MS_MAGIC_COOKIE_VALUE.to_be_bytes().to_vec(),
    );
    unauth.push(
        stun::MAPPED_ADDRESS,
        stun::encode_address("6.6.6.6:6".parse().unwrap()),
    );
    assert!(c.on_packet(&unauth.encode().unwrap(), 2).1.is_empty());
    assert_eq!(c.relayed(), None);
    // Messages without the MS magic cookie are malformed; no data without an active peer.
    assert_eq!(c.on_packet(&[0u8; 20], 3).1, vec![Event::Malformed]);
    // Endless challenges end after three rounds.
    let mut c = client(Transport::Udp);
    let mut req = parse(&c.start(0).unwrap()).unwrap();
    let mut failed = false;
    for i in 0..5 {
        let (w, e) = c.on_packet(&challenge(&req, 3), i);
        if e.contains(&Event::Failed(401)) {
            failed = true;
            break;
        }
        req = parse(&w[0]).unwrap();
    }
    assert!(failed);
    // Responses to unknown transactions are ignored.
    let mut c = client(Transport::Udp);
    let req = parse(&c.start(0).unwrap()).unwrap();
    let mut other = req.clone();
    other.transaction[0] ^= 1;
    assert!(c.on_packet(&challenge(&other, 3), 1).0.is_empty());
}

/// TU-27: 650 ms × 9, then the transaction fails.
#[test]
fn retransmission() {
    let mut c = client(Transport::Udp);
    let first = c.start(0).unwrap();
    let mut sent = 0;
    let mut now = 0;
    let mut events = Vec::new();
    while events.is_empty() && now < 20_000 {
        now += 50;
        let (w, e) = c.on_tick(now);
        for x in &w {
            assert_eq!(x, &first, "same transaction");
        }
        sent += w.len();
        events = e;
    }
    assert_eq!((sent, events), (9, vec![Event::Failed(0)]));
    assert_eq!(now, 650 * 10);
}

#[test]
fn tcp_framing_and_pseudo_tls() {
    let mut c = client(Transport::Tcp);
    let w = c.start(0).unwrap();
    assert_eq!(&w[..2], &[2, 0]);
    let mut d = Deframer::default();
    let mut frames = d.push(&w[..7]).unwrap();
    assert!(frames.is_empty());
    frames = d.push(&w[7..]).unwrap();
    assert_eq!(frames.len(), 1);
    assert!(parse(&frames[0].1).is_some());
    assert!(Deframer::default().push(&[9, 0, 0, 1, 0]).is_err());
    assert!(Deframer::default().push(&[2, 0, 0xff, 0xff]).is_err());
    let hello = pseudo_tls_client_hello(0x5f00_0000, [7; 28]);
    assert_eq!(hello.len(), 50);
    assert_eq!(&hello[..11], &[0x16, 3, 1, 0, 0x2d, 1, 0, 0, 0x29, 3, 1]);
    assert_eq!(&hello[43..], &[0, 0, 2, 0, 0x18, 1, 0]);
    let mut server = vec![0x16, 3, 1, 0, 0x4e, 2, 0, 0, 0x46, 3, 1];
    server.extend_from_slice(&[0; 32]);
    server.push(0x20);
    server.extend_from_slice(&[5; 32]);
    server.extend_from_slice(&[0, 0x18, 0, 0x0e, 0, 0, 0]);
    assert!(check_pseudo_tls_server_hello(&server));
    server[77] = 0x2f;
    assert!(!check_pseudo_tls_server_hello(&server));
}

/// [MS-TURNBWM] §3.1: a bandwidth request is an authenticated Allocate on the allocation with
/// the caller's attributes (built for its transaction ID); only one request at a time; the
/// authenticated answer is reported with its attributes, an error or timeout as `BacFailed`
/// while the allocation stays.
#[test]
fn bandwidth_admission_requests() {
    let mut c = client(Transport::Udp);
    assert!(c.bac(|_| Vec::new(), 0).is_none(), "no allocation yet");
    let req = parse(&c.start(0).unwrap()).unwrap();
    let (w, _) = c.on_packet(&challenge(&req, 3), 10);
    let req2 = parse(&w[0]).unwrap();
    c.on_packet(&allocated(&req2, true), 20);

    let mut seen = None;
    let wire = c
        .bac(
            |t| {
                seen = Some(*t);
                vec![(0x8056, vec![0, 0, 0, 0])]
            },
            30,
        )
        .unwrap();
    let bac = parse(&wire).unwrap();
    assert_eq!(
        Some(bac.transaction),
        seen,
        "attributes built for this transaction"
    );
    assert_eq!(bac.kind, ALLOCATE);
    assert_eq!(bac.get(0x8056), Some(&[0u8, 0, 0, 0][..]));
    assert!(bac.get(stun::MESSAGE_INTEGRITY).is_some());
    assert!(c.bac(|_| Vec::new(), 31).is_none(), "one request at a time");

    // An unauthenticated answer is ignored; the authenticated one is reported.
    let site = vec![0x80, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 100];
    let unsigned = reply(&bac, ALLOCATE_RESPONSE, &[(0x805d, site.clone())], None);
    assert!(c.on_packet(&unsigned, 40).1.is_empty());
    let (_, e) = c.on_packet(
        &reply(
            &bac,
            ALLOCATE_RESPONSE,
            &[(0x805d, site.clone())],
            Some(true),
        ),
        41,
    );
    match &e[..] {
        [Event::Bac {
            attributes,
            transaction,
        }] => {
            assert_eq!(*transaction, bac.transaction);
            assert!(attributes.contains(&(0x805d, site)));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(c.relayed(), Some("192.0.2.20:55667".parse().unwrap()));

    // Error and timeout: BacFailed, the allocation stays.
    let bac2 = parse(&c.bac(|_| Vec::new(), 50).unwrap()).unwrap();
    let err = reply(
        &bac2,
        ALLOCATE_ERROR,
        &[(stun::ERROR_CODE, stun::encode_error(403, "Forbidden"))],
        Some(true),
    );
    assert_eq!(c.on_packet(&err, 60).1, vec![Event::BacFailed(403)]);
    c.bac(|_| Vec::new(), 70).unwrap();
    let mut events = Vec::new();
    for k in 0..20 {
        events.extend(c.on_tick(70 + k * 1000).1);
    }
    assert_eq!(events, vec![Event::BacFailed(0)]);
    assert!(c.relayed().is_some());
}
