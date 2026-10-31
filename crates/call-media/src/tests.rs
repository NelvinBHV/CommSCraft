use super::*;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn negotiated_from(other: &CallMedia) -> sdp::Negotiated {
    let d = other.local_description();
    sdp::Negotiated {
        codec: sdp::Codec::G722,
        payload_type: 9,
        dtmf_payload_type: Some(101),
        remote_rtp: (d.address, d.rtp_port),
        remote_rtcp: (d.address, d.rtcp_port),
        remote_key: Some(other.key().clone()),
        remote_ice: d.ice.clone(),
        direction: sdp::Direction::SendRecv,
        music_on_hold: false,
    }
}

/// Two call halves over loopback UDP: ICE selects the host pairs, then encrypted G.722 and a
/// DTMF digit travel from A to B; the final offer lists only the selected candidates.
#[test]
fn loopback_call_media() {
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    let t0 = now_ms();
    let (mut a, fa) = CallMedia::prepare(ip, None, t0).unwrap();
    let (mut b, fb) = CallMedia::prepare(ip, None, t0).unwrap();
    assert!(fa.is_empty() && fb.is_empty(), "no relay, no TURN");
    assert!(a.gathered(t0));
    let desc = a.local_description();
    assert_eq!(desc.ice.as_ref().unwrap().candidates.len(), 2);
    assert_eq!(desc.ice.as_ref().unwrap().ufrag.len(), 8);
    let (na, nb) = (negotiated_from(&b), negotiated_from(&a));
    a.start(&na, true, "a@test", t0).unwrap();
    b.start(&nb, false, "b@test", t0).unwrap();

    let mut selected = (false, false);
    let deadline = t0 + 15_000;
    while !(selected.0 && selected.1) && now_ms() < deadline {
        let now = now_ms();
        selected.0 |= a.poll(now).contains(&Event::Selected);
        selected.1 |= b.poll(now).contains(&Event::Selected);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(selected, (true, true), "ICE completed on both sides");
    let fin = a.final_description().unwrap();
    assert_eq!(fin.ice.as_ref().unwrap().candidates.len(), 2);
    assert_eq!(fin.remote_candidates.len(), 2);

    // 40 frames of tone A → B, one DTMF digit.
    let frame: Vec<i16> = (0..320)
        .map(|i| {
            (8000.0 * (2.0 * std::f64::consts::PI * 1020.0 * f64::from(i) / 16000.0).sin()) as i16
        })
        .collect();
    assert!(a.send_dtmf('#'));
    let mut digits = Vec::new();
    let mut energy = 0f64;
    for _ in 0..60 {
        let now = now_ms();
        a.send_frame(&frame, now);
        a.poll(now);
        for e in b.poll(now) {
            if let Event::Dtmf(d) = e {
                digits.push(d);
            }
        }
        energy += b
            .playout()
            .iter()
            .map(|&s| f64::from(s).powi(2))
            .sum::<f64>();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(digits, vec!['#']);
    assert!(energy > 1e9, "audio arrived: {energy}");

    // Hold: A stops sending RTP at once; B plays silence once its buffer is empty.
    a.set_hold(true);
    assert!(!a.sending());
    for _ in 0..20 {
        a.send_frame(&frame, now_ms());
        a.poll(now_ms());
        b.poll(now_ms());
        // Drains the jitter buffer; concealment then decays to silence.
        let _ = b.playout();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let mut held = 0f64;
    for _ in 0..10 {
        a.send_frame(&frame, now_ms());
        b.poll(now_ms());
        held += b
            .playout()
            .iter()
            .map(|&s| f64::from(s).powi(2))
            .sum::<f64>();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(held < 1e7, "no audio on hold: {held}");
    a.set_hold(false);
    assert!(a.sending());
    a.stop(now_ms());
    let mut bye = false;
    for _ in 0..20 {
        bye |= b.poll(now_ms()).contains(&Event::PeerBye);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(bye, "RTCP BYE received");
}

/// Datagrams from unknown addresses never reach the media core.
#[test]
fn strangers_are_ignored() {
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    let t0 = now_ms();
    let (mut a, _) = CallMedia::prepare(ip, None, t0).unwrap();
    let (b, _) = CallMedia::prepare(ip, None, t0).unwrap();
    a.start(&negotiated_from(&b), true, "a@test", t0).unwrap();
    let stranger = UdpSocket::bind("127.0.0.1:0").unwrap();
    let target = a.host_address(0).unwrap();
    let mut rtp_like = vec![0x80, 9, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1];
    rtp_like.extend_from_slice(&[0u8; 160]);
    for _ in 0..5 {
        stranger.send_to(&rtp_like, target).unwrap();
    }
    std::thread::sleep(std::time::Duration::from_millis(20));
    a.poll(now_ms());
    assert_eq!(
        a.audio.as_ref().unwrap().stats.received + a.audio.as_ref().unwrap().stats.dropped,
        0
    );
}

#[test]
fn rfc4571_framing() {
    let mut d = Deframer4571::default();
    let mut stream = frame_4571(b"abc");
    stream.extend(frame_4571(b""));
    stream.extend(frame_4571(&[7u8; 1500]));
    assert!(d.push(&stream[..4]).unwrap().is_empty());
    let got = d.push(&stream[4..]).unwrap();
    assert_eq!(got, vec![b"abc".to_vec(), Vec::new(), vec![7u8; 1500]]);
    assert!(
        Deframer4571::default().push(&[0x05, 0xdd]).is_err(),
        "1501 octets"
    );
}

/// A TURN server over TCP for the test: pseudo-TLS, MS-TURN framing, digest challenge, allocation
/// with HMAC-SHA256; then it returns the first Send request it receives.
fn mock_tcp_turn(
    listener: std::net::TcpListener,
    relayed: SocketAddr,
) -> std::thread::JoinHandle<Vec<u8>> {
    use commscraft_stun as stun;
    std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let mut hello = [0u8; 50];
        std::io::Read::read_exact(&mut s, &mut hello).unwrap();
        assert_eq!(&hello[..5], &[0x16, 3, 1, 0, 0x2d]);
        let mut answer = vec![0x16, 3, 1, 0, 0x4e, 2, 0, 0, 0x46, 3, 1];
        answer.extend_from_slice(&[0; 32]);
        answer.push(0x20);
        answer.extend_from_slice(&[1; 32]);
        answer.extend_from_slice(&[0, 0x18, 0, 0x0e, 0, 0, 0]);
        std::io::Write::write_all(&mut s, &answer).unwrap();
        let mut frames = turn::Deframer::default();
        let mut buf = [0u8; 4096];
        let key = stun::ms_turn_sha256_key(b"nonce", b"pw", b"user", b"realm");
        loop {
            let n = std::io::Read::read(&mut s, &mut buf).unwrap();
            assert!(n > 0, "connection closed");
            for (kind, p) in frames.push(&buf[..n]).unwrap() {
                assert_eq!(kind, 2);
                let req = turn::parse(&p).unwrap();
                let reply = |kind: u16, attrs: Vec<(u16, Vec<u8>)>, sign: bool| {
                    let mut m = stun::Message::new(kind, req.transaction);
                    m.push(
                        stun::MS_MAGIC_COOKIE,
                        stun::MS_MAGIC_COOKIE_VALUE.to_be_bytes().to_vec(),
                    );
                    for (k, v) in attrs {
                        m.push(k, v);
                    }
                    let raw = m.encode().unwrap();
                    let raw = if sign {
                        stun::with_integrity(raw, stun::Integrity::LegacySha256(&key)).unwrap()
                    } else {
                        raw
                    };
                    turn::frame(2, &raw)
                };
                let out = match (req.kind, req.get(stun::MESSAGE_INTEGRITY).is_some()) {
                    (turn::ALLOCATE, false) => reply(
                        turn::ALLOCATE_ERROR,
                        vec![
                            (stun::ERROR_CODE, stun::encode_error(401, "Unauthorized")),
                            (stun::REALM, b"realm".to_vec()),
                            (stun::NONCE, b"nonce".to_vec()),
                            (stun::MS_VERSION, 3u32.to_be_bytes().to_vec()),
                        ],
                        false,
                    ),
                    (turn::ALLOCATE, true) => reply(
                        turn::ALLOCATE_RESPONSE,
                        vec![
                            (stun::MAPPED_ADDRESS, stun::encode_address(relayed)),
                            (stun::LIFETIME, 600u32.to_be_bytes().to_vec()),
                        ],
                        true,
                    ),
                    (turn::SEND, _) => return req.get(stun::DATA).unwrap().to_vec(),
                    _ => Vec::new(),
                };
                std::io::Write::write_all(&mut s, &out).unwrap();
            }
        }
    })
}

/// MS-ICE2 regular mode over the TCP relay (port 443 in production): pseudo-TLS, allocation,
/// TCP-ACT/TCP-PASS relayed candidates (never the default), and an ICE check towards a passive
/// peer candidate sent as a TURN Send request carrying an RFC 4571-framed STUN request.
#[test]
fn tcp_relay_allocation_and_check() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let relayed: SocketAddr = "127.0.0.1:40443".parse().unwrap();
    let relay = Relay {
        // An unused UDP port: the UDP allocation fails silently, only TCP matters here.
        server: "127.0.0.1:9".parse().unwrap(),
        tcp_server: Some(listener.local_addr().unwrap()),
        username: "user".into(),
        password: zeroize::Zeroizing::new(b"pw".to_vec()),
    };
    let server = mock_tcp_turn(listener, relayed);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    let t0 = now_ms();
    let (mut a, first) = CallMedia::prepare(ip, Some(&relay), t0).unwrap();
    a.send_initial(first);
    let deadline = t0 + 5000;
    while now_ms() < deadline
        && !a
            .candidates()
            .iter()
            .any(|(_, c)| c.transport == ice::Transport::TcpActive)
    {
        a.poll(now_ms());
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let d = a.local_description();
    let cands = &d.ice.as_ref().unwrap().candidates;
    let tcp: Vec<_> = cands
        .iter()
        .filter(|c| c.transport != sdp::Transport::Udp)
        .collect();
    assert_eq!(tcp.len(), 4, "active and passive, RTP and RTCP: {cands:?}");
    assert!(tcp
        .iter()
        .all(|c| c.kind == sdp::CandidateType::Relay
            && SocketAddr::new(c.address, c.port) == relayed));
    assert_ne!(
        SocketAddr::new(d.address, d.rtp_port),
        relayed,
        "SD-45: default never TCP"
    );

    // The peer offers only a TCP-passive candidate: our active relay candidate checks it.
    let peer: SocketAddr = "127.0.0.1:50000".parse().unwrap();
    let mut n = negotiated_from(&a);
    n.remote_ice = Some(sdp::Ice {
        ufrag: "PEER".into(),
        pwd: "peerpassword0123456789ab".into(),
        candidates: vec![sdp::Candidate {
            foundation: "9".into(),
            component: 1,
            transport: sdp::Transport::TcpPassive,
            priority: 1,
            address: peer.ip(),
            port: peer.port(),
            kind: sdp::CandidateType::Relay,
            related: None,
        }],
    });
    a.start(&n, true, "a@test", now_ms()).unwrap();
    for _ in 0..50 {
        a.poll(now_ms());
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let data = server.join().unwrap();
    let len = usize::from(u16::from_be_bytes([data[0], data[1]]));
    assert_eq!(len, data.len() - 2, "RFC 4571 length prefix");
    let stun = commscraft_stun::Message::decode(&data[2..]).unwrap();
    assert_eq!(stun.kind, 0x0001, "Binding Request");
    assert!(
        stun.get(commscraft_stun::USERNAME)
            .is_some_and(|u| u.starts_with(b"PEER:")),
        "USERNAME = remote:local"
    );
}
