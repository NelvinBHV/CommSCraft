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
        ssrtp: false,
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

/// Issue #42: the TCP relay's write buffer never grows beyond its bound; media is dropped, a
/// control message that does not fit gives the relay up.
#[test]
fn tcp_write_buffer_bounded() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let stream = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let relay = Relay {
        server: "127.0.0.1:9".parse().unwrap(),
        tcp_server: None,
        username: "u".into(),
        password: zeroize::Zeroizing::new(b"p".to_vec()),
    };
    let mut t = TcpRelay {
        stream,
        client: Box::new(turn_client(&relay, turn::Transport::Tcp)),
        frames: turn::Deframer::default(),
        out: Vec::new(),
        allocated: None,
        active: None,
        failed: false,
        peers: HashMap::new(),
    };
    for _ in 0..100 {
        t.queue_media(&[0u8; 1500]);
    }
    assert!(
        t.out.len() <= MAX_WRITE_BUFFER && !t.failed,
        "media dropped, relay kept"
    );
    t.queue_control(&[0u8; 2000]);
    assert!(
        t.failed && t.out.is_empty(),
        "control that does not fit: relay given up"
    );
    t.queue_control(&[0u8; 10]);
    assert!(t.out.is_empty());
}

/// SD-50 over real sockets: during early media A (the caller) plays B but sends no speech; after
/// the 200 of the same fork (`confirm`) A's speech arrives.
#[test]
fn early_media_then_confirm() {
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    let t0 = now_ms();
    let (mut a, _) = CallMedia::prepare(ip, None, t0).unwrap();
    let (mut b, _) = CallMedia::prepare(ip, None, t0).unwrap();
    let (na, nb) = (negotiated_from(&b), negotiated_from(&a));
    a.start_early(&na, "a@test", t0).unwrap();
    b.start(&nb, false, "b@test", t0).unwrap();
    let frame: Vec<i16> = (0..320)
        .map(|i| {
            (8000.0 * (2.0 * std::f64::consts::PI * 1020.0 * f64::from(i) / 16000.0).sin()) as i16
        })
        .collect();
    let energy = |v: Vec<i16>| v.iter().map(|&s| f64::from(s).powi(2)).sum::<f64>();
    let run = |a: &mut CallMedia, b: &mut CallMedia, rounds: usize| {
        let (mut at_a, mut at_b) = (0f64, 0f64);
        for _ in 0..rounds {
            let now = now_ms();
            a.send_frame(&frame, now);
            b.send_frame(&frame, now);
            a.poll(now);
            b.poll(now);
            at_a += energy(a.playout());
            at_b += energy(b.playout());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        (at_a, at_b)
    };
    let (at_a, at_b) = run(&mut a, &mut b, 400);
    assert!(at_a > 1e9, "A hears B's early media: {at_a}");
    assert!(at_b < 1e6, "A sends no speech before the answer: {at_b}");
    a.confirm(&na, "a@test", now_ms()).unwrap();
    let (_, at_b) = run(&mut a, &mut b, 60);
    assert!(at_b > 1e9, "speech after the answer: {at_b}");
}

/// What the UDP test TURN server answers to a bandwidth check: (local, local relay, remote) V
/// flags.
type CheckAnswer = (bool, bool, bool);

/// A UDP TURN server for the bandwidth tests: challenge, SHA-256 allocation (relayed address
/// `relayed`), and the bandwidth requests of MS-TURNBWM. Returns the BAC message types it saw,
/// with the amount's maximum send value when one was present.
fn mock_udp_turn_bwm(
    socket: UdpSocket,
    relayed: SocketAddr,
    answer: CheckAnswer,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> std::thread::JoinHandle<Vec<(u8, Option<u32>)>> {
    use commscraft_stun as stun;
    std::thread::spawn(move || {
        socket
            .set_read_timeout(Some(std::time::Duration::from_millis(50)))
            .unwrap();
        let key = stun::ms_turn_sha256_key(b"nonce", b"pw", b"user", b"realm");
        let mut seen = Vec::new();
        let mut buf = [0u8; 2048];
        while !stop.load(std::sync::atomic::Ordering::Relaxed) {
            let Ok((n, from)) = socket.recv_from(&mut buf) else {
                continue;
            };
            let Some(req) = turn::parse(&buf[..n]) else {
                continue;
            };
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
                if sign {
                    stun::with_integrity(raw, stun::Integrity::LegacySha256(&key)).unwrap()
                } else {
                    raw
                }
            };
            let site = |valid: bool| -> Vec<u8> {
                let flags: u32 = u32::from(valid) << 31;
                [flags, 85, 85]
                    .iter()
                    .flat_map(|v| v.to_be_bytes())
                    .collect()
            };
            let out = if req.get(stun::MESSAGE_INTEGRITY).is_none() {
                reply(
                    turn::ALLOCATE_ERROR,
                    vec![
                        (stun::ERROR_CODE, stun::encode_error(401, "Unauthorized")),
                        (stun::REALM, b"realm".to_vec()),
                        (stun::NONCE, b"nonce".to_vec()),
                        (stun::MS_VERSION, 3u32.to_be_bytes().to_vec()),
                    ],
                    false,
                )
            } else if let Some(bac) = req.get(turn_bwm::attr::BAC_MESSAGE) {
                let kind = bac[3];
                let max_send = req
                    .get(turn_bwm::attr::RESERVATION_AMOUNT)
                    .map(|a| u32::from_be_bytes([a[4], a[5], a[6], a[7]]));
                seen.push((kind, max_send));
                let attrs = match kind {
                    0 => {
                        let mut v = vec![
                            (turn_bwm::attr::REMOTE_SITE_ADDRESS_RESPONSE, site(answer.2)),
                            (turn_bwm::attr::LOCAL_SITE_ADDRESS_RESPONSE, site(answer.0)),
                        ];
                        if req.get(turn_bwm::attr::REMOTE_RELAY_SITE_ADDRESS).is_some() {
                            v.push((
                                turn_bwm::attr::REMOTE_RELAY_SITE_ADDRESS_RESPONSE,
                                site(answer.2),
                            ));
                        }
                        if req.get(turn_bwm::attr::LOCAL_RELAY_SITE_ADDRESS).is_some() {
                            v.push((
                                turn_bwm::attr::LOCAL_RELAY_SITE_ADDRESS_RESPONSE,
                                site(answer.1),
                            ));
                        }
                        v
                    }
                    _ => vec![
                        (turn_bwm::attr::RESERVATION_ID, vec![0x42; 16]),
                        (turn_bwm::attr::RESERVATION_AMOUNT, vec![0; 16]),
                    ],
                };
                reply(turn::ALLOCATE_RESPONSE, attrs, true)
            } else {
                reply(
                    turn::ALLOCATE_RESPONSE,
                    vec![
                        (stun::MAPPED_ADDRESS, stun::encode_address(relayed)),
                        (stun::LIFETIME, 600u32.to_be_bytes().to_vec()),
                    ],
                    true,
                )
            };
            let _ = socket.send_to(&out, from);
        }
        seen
    })
}

/// The media under test, the server's stop flag and its log of bandwidth requests.
type BwmSetup = (
    CallMedia,
    std::sync::Arc<std::sync::atomic::AtomicBool>,
    std::thread::JoinHandle<Vec<(u8, Option<u32>)>>,
);

fn bwm_media(answer: CheckAnswer, peer: Option<sdp::Ice>) -> BwmSetup {
    let server = UdpSocket::bind("127.0.0.1:0").unwrap();
    let relay = Relay {
        server: server.local_addr().unwrap(),
        tcp_server: None,
        username: "user".into(),
        password: zeroize::Zeroizing::new(b"pw".to_vec()),
    };
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let handle = mock_udp_turn_bwm(
        server,
        "127.0.0.1:40500".parse().unwrap(),
        answer,
        stop.clone(),
    );
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    let (mut m, first) = CallMedia::prepare(ip, Some(&relay), now_ms()).unwrap();
    m.enable_bandwidth_management(BwmConfig {
        call_id: "call-1".into(),
        profile: turn_bwm::LocationProfile {
            peer: turn_bwm::Location::Unknown,
            own: turn_bwm::Location::Unknown,
            federation: turn_bwm::Federation::None,
        },
        peer,
    });
    m.send_initial(first);
    (m, stop, handle)
}

fn peer_offer() -> sdp::Ice {
    let cand = |kind, port| sdp::Candidate {
        foundation: "1".into(),
        component: 1,
        transport: sdp::Transport::Udp,
        priority: 1,
        address: "198.51.100.7".parse().unwrap(),
        port,
        kind,
        related: None,
    };
    sdp::Ice {
        ufrag: "PEER".into(),
        pwd: "peerpassword0123456789ab".into(),
        candidates: vec![
            cand(sdp::CandidateType::Host, 7000),
            cand(sdp::CandidateType::Relay, 7100),
        ],
    }
}

/// [MS-ICE2BWM] §3.1.4.2 (BW-02, BW-03): the callee checks while gathering; a refused local
/// relay is not offered; a policy without any path blocks the call.
#[test]
fn callee_bandwidth_check() {
    let (mut m, stop, handle) = bwm_media((true, false, true), Some(peer_offer()));
    let t0 = now_ms();
    while !m.gathered(now_ms()) && now_ms() < t0 + 5000 {
        m.poll(now_ms());
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let d = m.local_description();
    let cands = &d.ice.as_ref().unwrap().candidates;
    assert!(
        cands.iter().all(|c| c.kind != sdp::CandidateType::Relay),
        "relay refused: {cands:?}"
    );
    assert!(!m.bandwidth_blocked());
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let seen = handle.join().unwrap();
    assert_eq!(
        seen.first(),
        Some(&(0, Some(bwm::AUDIO_KBPS))),
        "a check with the audio amount"
    );

    let (mut m, stop, handle) = bwm_media((false, false, true), Some(peer_offer()));
    let t0 = now_ms();
    while !m.gathered(now_ms()) && now_ms() < t0 + 5000 {
        m.poll(now_ms());
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(m.bandwidth_blocked(), "no path out of the local site");
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    handle.join().unwrap();
}

/// §3.1.4.4.2, §3.1.4.8.2.2–3 and MS-TURNBWM §2.2.3: towards a peer without ICE the caller
/// commits, updates, and cancels the reservation at the end.
#[test]
fn commit_update_cancel_without_ice() {
    let (mut m, stop, handle) = bwm_media((true, true, true), None);
    let t0 = now_ms();
    while !m.gathered(now_ms()) && now_ms() < t0 + 5000 {
        m.poll(now_ms());
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let mut n = negotiated_from(&m);
    n.remote_ice = None;
    n.remote_key = Some(m.key().clone());
    n.remote_rtp = ("198.51.100.7".parse().unwrap(), 7000);
    n.remote_rtcp = ("198.51.100.7".parse().unwrap(), 7001);
    m.start(&n, true, "a@test", now_ms()).unwrap();
    // Commit, then (with a simulated clock) an update after 19 s.
    let t = now_ms();
    for k in 0..40u64 {
        m.poll(t + k * 10);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    m.poll(t + bwm::UPDATE_MS + 1000);
    std::thread::sleep(std::time::Duration::from_millis(50));
    m.poll(t + bwm::UPDATE_MS + 1100);
    m.stop(t + bwm::UPDATE_MS + 2000);
    std::thread::sleep(std::time::Duration::from_millis(100));
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let seen = handle.join().unwrap();
    assert_eq!(
        seen,
        vec![
            (1, Some(bwm::AUDIO_KBPS)),
            (2, Some(bwm::AUDIO_KBPS)),
            (2, Some(0))
        ],
        "commit, update, cancel"
    );
}
