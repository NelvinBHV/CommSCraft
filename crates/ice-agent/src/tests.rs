use super::*;

fn rng(seed: u8) -> Random {
    // xorshift64: deterministic test randomness without short cycles.
    let mut s = 0x9E37_79B9_7F4A_7C15u64 ^ u64::from(seed);
    Box::new(move |b: &mut [u8]| {
        for x in b.iter_mut() {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            *x = (s >> 24) as u8;
        }
    })
}

fn host(foundation: &str, component: u8, addr: &str) -> Local {
    let a: SocketAddr = addr.parse().unwrap();
    Local {
        foundation: foundation.into(),
        component,
        transport: Transport::Udp,
        kind: Kind::Host,
        address: a,
        base: a,
        priority: commscraft_ice::candidate_priority(126, 0xffff, component),
    }
}

fn remote_of(l: &Local) -> Remote {
    Remote {
        foundation: l.foundation.clone(),
        component: l.component,
        transport: l.transport,
        address: l.address,
        priority: l.priority,
    }
}

fn pair_of_agents() -> (Agent, Agent) {
    let mut a = Agent::new(
        true,
        "AUFR".into(),
        "apassword-0123456789abcdef".into(),
        rng(1),
    );
    let mut b = Agent::new(
        false,
        "BUFR".into(),
        "bpassword-0123456789abcdef".into(),
        rng(2),
    );
    a.add_local(host("1", 1, "10.0.0.1:5000"));
    a.add_local(host("1", 2, "10.0.0.1:5001"));
    b.add_local(host("7", 1, "10.0.0.2:6000"));
    b.add_local(host("7", 2, "10.0.0.2:6001"));
    let (ra, rb): (Vec<Remote>, Vec<Remote>) = (
        a.locals().iter().map(remote_of).collect(),
        b.locals().iter().map(remote_of).collect(),
    );
    a.set_remote("BUFR", "bpassword-0123456789abcdef", rb, 0);
    b.set_remote("AUFR", "apassword-0123456789abcdef", ra, 0);
    (a, b)
}

/// Delivers packets between the two agents by base address.
fn deliver(
    from: &Agent,
    to: &mut Agent,
    packets: Vec<Transmit>,
    now: u64,
    drop: &dyn Fn(&Transmit) -> bool,
) -> (Vec<Transmit>, Vec<Event>) {
    let mut out = Vec::new();
    let mut events = Vec::new();
    for t in packets {
        if drop(&t) {
            continue;
        }
        let src = from.locals()[t.local].base;
        if let Some(i) = to.locals().iter().position(|l| l.base == t.to) {
            let (o, e) = to.on_stun(i, src, &t.data, now);
            out.extend(o);
            events.extend(e);
        }
    }
    (out, events)
}

fn run(
    a: &mut Agent,
    b: &mut Agent,
    until: u64,
    drop: &dyn Fn(&Transmit) -> bool,
) -> (Vec<Event>, Vec<Event>) {
    let (mut ea, mut eb) = (Vec::new(), Vec::new());
    let (mut to_b, mut to_a) = (Vec::new(), Vec::new());
    let mut now = 0;
    while now <= until {
        let (o, e) = a.on_tick(now);
        to_b.extend(o);
        ea.extend(e);
        let (o, e) = b.on_tick(now);
        to_a.extend(o);
        eb.extend(e);
        for _ in 0..4 {
            let (o, e) = deliver(a, b, std::mem::take(&mut to_b), now, drop);
            to_a.extend(o);
            eb.extend(e);
            let (o, e) = deliver(b, a, std::mem::take(&mut to_a), now, drop);
            to_b.extend(o);
            ea.extend(e);
        }
        now += 10;
    }
    (ea, eb)
}

/// ice-19 §5.7.2 with the values of IE-05.
#[test]
fn pair_priority_formula() {
    assert_eq!(
        pair_priority(2130706431, 16648703),
        (16648703u64 << 32) + 2 * 2130706431 + 1
    );
    assert_eq!(pair_priority(1, 2), (1u64 << 32) + 4);
}

/// Controlling and controlled agent find, check and nominate the host pairs for RTP and RTCP.
#[test]
fn two_agents_select_both_components() {
    let (mut a, mut b) = pair_of_agents();
    let (ea, eb) = run(&mut a, &mut b, 12_000, &|_| false);
    let rtp_a: SocketAddr = "10.0.0.2:6000".parse().unwrap();
    assert!(
        ea.contains(&Event::Usable {
            local: 0,
            remote: rtp_a
        }),
        "{ea:?}"
    );
    assert!(
        ea.contains(&Event::Selected {
            rtp: (0, rtp_a),
            rtcp: Some((1, "10.0.0.2:6001".parse().unwrap()))
        }),
        "{ea:?}"
    );
    assert!(
        eb.contains(&Event::Selected {
            rtp: (0, "10.0.0.1:5000".parse().unwrap()),
            rtcp: Some((1, "10.0.0.1:5001".parse().unwrap()))
        }),
        "{eb:?}"
    );
    // Both announced version 3: the peers switched to RFC 5389 and consent freshness runs.
    assert_eq!(a.format, PeerFormat::Rfc5389);
    let (o, e) = a.on_tick(12_000 + CONSENT_INTERVAL_MS);
    assert!(e.is_empty());
    assert!(o
        .iter()
        .all(|t| t.data.get(4..8) == Some(&stun::MAGIC_COOKIE.to_be_bytes()[..])));
}

#[test]
fn no_peer_means_failure_after_the_check_phase() {
    let (mut a, mut b) = pair_of_agents();
    let (ea, _) = run(&mut a, &mut b, 11_000, &|_| true);
    assert_eq!(ea, vec![Event::Failed("no valid candidate pair")]);
}

/// The first checks go out in STUN-02 format plus an RFC 5389 duplicate (§3.1.5.2, IC-03), with
/// the MS-ICE2 attributes.
#[test]
fn first_checks_in_both_formats() {
    let (mut a, _) = pair_of_agents();
    let (o, _) = a.on_tick(0);
    assert_eq!(o.len(), 2);
    let legacy = Message::decode(&o[0].data).unwrap();
    let modern = Message::decode(&o[1].data).unwrap();
    assert_eq!(
        (legacy.format(), modern.format()),
        (stun::Format::Legacy, stun::Format::Rfc5389)
    );
    assert_eq!(legacy.get(stun::USERNAME), Some(&b"BUFR:AUFR"[..]));
    assert_eq!(
        legacy.get(stun::CANDIDATE_IDENTIFIER),
        Some(&b"1\0\0\0"[..])
    );
    assert!(legacy.get(stun::ICE_CONTROLLING).is_some() && legacy.get(stun::PRIORITY).is_some());
    let key = b"bpassword-0123456789abcdef";
    stun::verify_integrity(&o[0].data, Integrity::LegacySha1(key)).unwrap();
    stun::verify_fingerprint(&o[0].data).unwrap();
    stun::verify_integrity(&o[1].data, Integrity::Rfc5389(key)).unwrap();
}

fn request_from_peer(tid: [u8; 16], user: &[u8], with_mi: Option<&[u8]>, version: u32) -> Vec<u8> {
    let mut m = Message::new(BINDING_REQUEST, tid);
    m.push(stun::USERNAME, user.to_vec());
    m.push(stun::PRIORITY, 1u32.to_be_bytes().to_vec());
    m.push(stun::IMPLEMENTATION_VERSION, version.to_be_bytes().to_vec());
    let mut raw = m.encode().unwrap();
    if let Some(k) = with_mi {
        raw = stun::with_integrity(raw, Integrity::LegacySha1(k)).unwrap();
    }
    stun::with_fingerprint(raw).unwrap()
}

/// §3.1.5.2.2 / §3.1.5.2.3: validation of incoming requests and the response.
#[test]
fn incoming_request_validation() {
    let (_, mut b) = pair_of_agents();
    let from: SocketAddr = "10.0.0.1:5000".parse().unwrap();
    let pw = b"bpassword-0123456789abcdef";
    // Wrong ufrag or no fingerprint: discarded silently.
    assert!(b
        .on_stun(
            0,
            from,
            &request_from_peer([1; 16], b"XXXX:AUFR", Some(pw), 2),
            0
        )
        .0
        .is_empty());
    let mut no_fp = Message::new(BINDING_REQUEST, [2; 16]);
    no_fp.push(stun::USERNAME, b"BUFR:AUFR".to_vec());
    assert!(b.on_stun(0, from, &no_fp.encode().unwrap(), 0).0.is_empty());
    // No MI → 401, wrong MI → 431, both with USERNAME.
    for (mi, code) in [(None, 401), (Some(&b"wrong"[..]), 431)] {
        let (o, _) = b.on_stun(0, from, &request_from_peer([3; 16], b"BUFR:AUFR", mi, 2), 0);
        let r = Message::decode(&o[0].data).unwrap();
        assert_eq!(r.kind, BINDING_ERROR);
        assert_eq!(
            stun::decode_error(r.get(stun::ERROR_CODE).unwrap())
                .unwrap()
                .0,
            code
        );
        assert_eq!(r.get(stun::USERNAME), Some(&b"BUFR:AUFR"[..]));
    }
    // Valid legacy request (peer version 2): legacy response with the required attributes.
    let (o, _) = b.on_stun(
        0,
        from,
        &request_from_peer([4; 16], b"BUFR:AUFR", Some(pw), 2),
        0,
    );
    let r = Message::decode(&o[0].data).unwrap();
    assert_eq!(
        (r.kind, r.format()),
        (BINDING_RESPONSE, stun::Format::Legacy)
    );
    assert_eq!(
        stun::decode_xor_address(r.get(stun::XOR_MAPPED_ADDRESS).unwrap(), &r.transaction).unwrap(),
        from
    );
    stun::verify_integrity(&o[0].data, Integrity::LegacySha1(pw)).unwrap();
    stun::verify_fingerprint(&o[0].data).unwrap();
    assert_eq!(
        b.format,
        PeerFormat::Legacy,
        "IMPLEMENTATION-VERSION 2 ⇒ STUN-02 only"
    );
    // From now on only legacy checks, no RFC 5389 duplicates.
    let (o, _) = b.on_tick(10);
    assert!(o
        .iter()
        .all(|t| Message::decode(&t.data).unwrap().format() == stun::Format::Legacy));
}

/// §3.1.5.3.1 and ice-19 §7.1.2.1: forged or misrouted responses are ignored.
#[test]
fn response_validation() {
    let (mut a, _) = pair_of_agents();
    let (o, _) = a.on_tick(0);
    let req = Message::decode(&o[0].data).unwrap();
    let respond = |mapped: &str, key: &[u8]| {
        let mut r = Message::new(BINDING_RESPONSE, req.transaction);
        r.push(
            stun::XOR_MAPPED_ADDRESS,
            stun::encode_xor_address(mapped.parse().unwrap(), &req.transaction),
        );
        r.push(stun::USERNAME, b"BUFR:AUFR".to_vec());
        let raw = stun::with_integrity(r.encode().unwrap(), Integrity::LegacySha1(key)).unwrap();
        stun::with_fingerprint(raw).unwrap()
    };
    let key = b"bpassword-0123456789abcdef";
    let peer: SocketAddr = "10.0.0.2:6000".parse().unwrap();
    // Wrong password, wrong source address, 0.0.0.0: nothing happens.
    assert!(a
        .on_stun(0, peer, &respond("10.0.0.1:5000", b"nope"), 1)
        .1
        .is_empty());
    assert!(a
        .on_stun(
            0,
            "10.0.0.9:6000".parse().unwrap(),
            &respond("10.0.0.1:5000", key),
            1
        )
        .1
        .is_empty());
    assert!(a
        .on_stun(0, peer, &respond("0.0.0.0:5000", key), 1)
        .1
        .is_empty());
    assert!(a.pairs.iter().all(|p| p.state != State::Succeeded));
    // The genuine response makes the pair usable.
    let (_, e) = a.on_stun(0, peer, &respond("10.0.0.1:5000", key), 1);
    assert_eq!(
        e,
        vec![Event::Usable {
            local: 0,
            remote: peer
        }]
    );
}

#[test]
fn bounds() {
    let mut a = Agent::new(
        true,
        "AUFR".into(),
        "apassword-0123456789abcdef".into(),
        rng(3),
    );
    for i in 0..50 {
        a.add_local(host(
            &i.to_string(),
            1,
            &format!("10.0.1.{}:{}", i, 5000 + i),
        ));
    }
    assert_eq!(a.locals().len(), MAX_CANDIDATES);
    let remotes: Vec<Remote> = (0..50)
        .map(|i| Remote {
            foundation: format!("r{i}"),
            component: 1,
            transport: Transport::Udp,
            address: format!("10.0.2.{i}:7000").parse().unwrap(),
            priority: 1,
        })
        .collect();
    a.set_remote("BUFR", "bpassword-0123456789abcdef", remotes, 0);
    assert_eq!(a.pairs.len(), MAX_PAIRS);
    // Unusable remote addresses are not paired.
    let mut c = Agent::new(
        true,
        "AUFR".into(),
        "apassword-0123456789abcdef".into(),
        rng(4),
    );
    c.add_local(host("1", 1, "10.0.0.1:5000"));
    c.set_remote(
        "X",
        "y",
        vec![Remote {
            foundation: "z".into(),
            component: 1,
            transport: Transport::Udp,
            address: "0.0.0.0:1".parse().unwrap(),
            priority: 1,
        }],
        0,
    );
    assert!(c.pairs.is_empty());
}

/// ice-tcp-07 §4.2: TCP pairing rules and pruning of passive local candidates.
#[test]
fn tcp_pairing_rules() {
    let mut a = Agent::new(
        true,
        "AUFR".into(),
        "apassword-0123456789abcdef".into(),
        rng(5),
    );
    let mut act = host("2", 1, "10.0.0.1:443");
    act.transport = Transport::TcpActive;
    let mut pass = host("2", 1, "10.0.0.1:443");
    pass.transport = Transport::TcpPassive;
    let udp = host("1", 1, "10.0.0.1:5000");
    a.add_local(act);
    a.add_local(pass);
    a.add_local(udp);
    let remote = |t: Transport, port: u16| Remote {
        foundation: format!("r{port}"),
        component: 1,
        transport: t,
        address: format!("10.0.0.2:{port}").parse().unwrap(),
        priority: 1,
    };
    a.set_remote(
        "BUFR",
        "bpassword-0123456789abcdef",
        vec![
            remote(Transport::TcpPassive, 1),
            remote(Transport::TcpActive, 2),
            remote(Transport::Udp, 3),
        ],
        0,
    );
    let pairs: Vec<(usize, u16)> = a
        .pairs
        .iter()
        .map(|p| (p.local, a.remotes[p.remote].address.port()))
        .collect();
    assert!(pairs.contains(&(0, 1)), "tcp-act → tcp-pass");
    assert!(pairs.contains(&(2, 3)), "udp → udp");
    assert_eq!(
        pairs.len(),
        2,
        "no tcp-pass local pairs, no mixed transports: {pairs:?}"
    );
}

/// Issue #42: a peer cannot point checks at this host or the local link.
#[test]
fn loopback_and_link_local_remotes_ignored() {
    let mut a = Agent::new(
        true,
        "AUFR".into(),
        "apassword-0123456789abcdef".into(),
        rng(9),
    );
    a.add_local(host("1", 1, "10.0.0.1:5000"));
    let remote = |addr: &str| Remote {
        foundation: addr.into(),
        component: 1,
        transport: Transport::Udp,
        address: addr.parse().unwrap(),
        priority: 1,
    };
    a.set_remote(
        "X",
        "y",
        vec![
            remote("127.0.0.1:7000"),
            remote("169.254.3.4:7000"),
            remote("[::1]:7000"),
            remote("192.0.2.9:7000"),
        ],
        0,
    );
    assert_eq!(a.pairs.len(), 1, "only the routable candidate is paired");
}
