use super::*;

fn key(b: u8) -> sdp::Key {
    sdp::Key::new(1, [b; 30])
}

/// The two halves of a call: A sends with key 1, B with key 2.
fn pair(codec: sdp::Codec, pt: u8) -> (Audio, Audio) {
    let n = |remote: u8| sdp::Negotiated {
        codec,
        payload_type: pt,
        dtmf_payload_type: Some(101),
        remote_rtp: ("192.0.2.1".parse().unwrap(), 5000),
        remote_rtcp: ("192.0.2.1".parse().unwrap(), 5001),
        remote_key: Some(key(remote)),
        remote_ice: None,
        direction: sdp::Direction::SendRecv,
        music_on_hold: false,
        ssrtp: false,
    };
    let a = Audio::new(&n(2), Some(&key(1)), [1; 10], "a@client").unwrap();
    let b = Audio::new(&n(1), Some(&key(2)), [2; 10], "b@client").unwrap();
    (a, b)
}

fn tone(n: usize, rate: f64, offset: usize) -> Vec<i16> {
    (0..n)
        .map(|i| {
            (8000.0 * (2.0 * std::f64::consts::PI * 1020.0 * (i + offset) as f64 / rate).sin())
                as i16
        })
        .collect()
}

fn energy(s: &[i16]) -> f64 {
    s.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>() / s.len().max(1) as f64
}

#[test]
fn classification() {
    assert_eq!(
        classify(&[0x00, 0x01, 0, 0, 0x21, 0x12, 0xa4, 0x42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        Kind::Stun
    );
    assert_eq!(
        classify(&[0x80, 0x09, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1]),
        Kind::Rtp
    );
    assert_eq!(classify(&[0x80, 200, 0, 6]), Kind::Rtcp);
    assert_eq!(classify(&[0x80, 201]), Kind::Rtcp);
    assert_eq!(classify(&[0x40, 0]), Kind::Other);
    assert_eq!(classify(&[]), Kind::Other);
}

/// Encrypted G.722 end to end, with reordering and one lost packet concealed.
#[test]
fn g722_call_path() {
    let (mut a, mut b) = pair(sdp::Codec::G722, 9);
    assert_eq!((a.sample_rate(), a.frame_samples()), (16000, 320));
    let mut packets = Vec::new();
    for f in 0..20 {
        packets.push(a.on_capture(&tone(320, 16000.0, f * 320)).unwrap());
    }
    assert!(
        packets.iter().all(|p| p.len() == 12 + 160 + 1 + 10),
        "header + G.722 + MKI + tag"
    );
    packets.swap(4, 5);
    packets.remove(10);
    for (i, p) in packets.iter().enumerate() {
        assert_eq!(b.on_rtp(p, i as u32 * 160), None);
    }
    let mut played = Vec::new();
    for _ in 0..20 {
        played.push(b.playout());
    }
    assert!(played.iter().all(|f| f.len() == 320));
    assert!(energy(&played[12]) > 1e6, "tone arrives");
    assert_eq!(b.stats.concealed, 1);
    assert_eq!(b.stats.received, 19);
}

#[test]
fn g711_and_mute() {
    let (mut a, mut b) = pair(sdp::Codec::Pcma, 8);
    assert_eq!(a.frame_samples(), 160);
    a.set_muted(true);
    for f in 0..5 {
        let p = a.on_capture(&tone(160, 8000.0, f * 160)).unwrap();
        b.on_rtp(&p, 0);
    }
    for _ in 0..5 {
        assert!(energy(&b.playout()) < 100.0, "muted ⇒ silence");
    }
}

/// RFC 4733 / MS-DTMF: one digit, reported once although the end packet comes three times.
#[test]
fn dtmf_digits() {
    let (mut a, mut b) = pair(sdp::Codec::G722, 9);
    assert!(a.send_dtmf('5'));
    assert!(!a.send_dtmf('6'), "one digit at a time");
    let mut digits = Vec::new();
    for f in 0..10 {
        let p = a.on_capture(&tone(320, 16000.0, f * 320)).unwrap();
        if let Some(d) = b.on_rtp(&p, 0) {
            digits.push(d);
        }
    }
    assert_eq!(digits, vec!['5']);
}

/// Security by design: wrong keys, foreign SSRCs, other payload types and oversized packets are
/// dropped before decoding.
#[test]
fn hostile_media() {
    let (mut a, mut b) = pair(sdp::Codec::G722, 9);
    let (mut evil, _) = pair(sdp::Codec::G722, 9);
    // `evil` encrypts with the same key 1 but another SSRC: accepted keys, foreign stream.
    let first = a.on_capture(&tone(320, 16000.0, 0)).unwrap();
    b.on_rtp(&first, 0);
    let mut forged = first.clone();
    forged[20] ^= 0xff;
    b.on_rtp(&forged, 0);
    evil.sender = rtp::Sender::new([9; 10]);
    let foreign = evil.on_capture(&tone(320, 16000.0, 0)).unwrap();
    b.on_rtp(&foreign, 0);
    b.on_rtp(&vec![0x80; 1600], 0);
    assert_eq!(b.stats.received, 1);
    assert_eq!(b.stats.dropped, 3);
}

#[test]
fn rtcp_reports_and_bye() {
    let (mut a, mut b) = pair(sdp::Codec::G722, 9);
    let p = a.on_capture(&tone(320, 16000.0, 0)).unwrap();
    b.on_rtp(&p, 0);
    let due = a.rtcp_due(0, 1 << 32);
    assert_eq!(due.len(), 2, "probe and report");
    assert!(due.iter().all(|p| classify(p) == Kind::Rtcp));
    assert!(a.rtcp_due(4999, 0).is_empty(), "5 s interval");
    for p in &due {
        assert!(!b.on_rtcp(p, 0));
    }
    assert!(b.on_rtcp(&a.bye().unwrap(), 0), "BYE of the peer");
}

/// Hold (RFC 3264 §8.4, MS-SDPEXT §3.1.5.23): no RTP while not sending, the timestamp keeps
/// running, and sending resumes with the next frame.
#[test]
fn hold_stops_rtp() {
    let (mut a, mut b) = pair(sdp::Codec::Pcmu, 0);
    let first = a.on_capture(&tone(160, 8000.0, 0)).unwrap();
    b.on_rtp(&first, 0);
    a.set_sending(false);
    for f in 1..=10 {
        assert!(a.on_capture(&tone(160, 8000.0, f * 160)).is_none());
    }
    a.set_sending(true);
    let p = a.on_capture(&tone(160, 8000.0, 11 * 160)).unwrap();
    assert!(b.on_rtp(&p, 0).is_none());
}

/// Without keys, to look into the RTCP packets.
fn plain_pair() -> (Audio, Audio) {
    let n = sdp::Negotiated {
        codec: sdp::Codec::Pcmu,
        payload_type: 0,
        dtmf_payload_type: None,
        remote_rtp: ("192.0.2.1".parse().unwrap(), 5000),
        remote_rtcp: ("192.0.2.1".parse().unwrap(), 5001),
        remote_key: None,
        remote_ice: None,
        direction: sdp::Direction::SendRecv,
        music_on_hold: false,
        ssrtp: false,
    };
    let a = Audio::new(&n, None, [1; 10], "a@client").unwrap();
    let b = Audio::new(&n, None, [2; 10], "b@client").unwrap();
    (a, b)
}

fn report_of(p: &[u8]) -> (Vec<rtp::ReportBlock>, Vec<rtp::Extension>) {
    match rtp::parse_rtcp(p).unwrap().into_iter().next().unwrap() {
        rtp::Rtcp::SenderReport {
            reports,
            extensions,
            ..
        }
        | rtp::Rtcp::ReceiverReport {
            reports,
            extensions,
            ..
        } => (reports, extensions),
        other => panic!("{other:?}"),
    }
}

/// [MS-RTP] §2.2.10.1, §2.2.11, §3.2–3.2.6 and RFC 3550 §6.4.1 (LSR/DLSR).
#[test]
fn rtcp_profile() {
    let (mut a, mut b) = plain_pair();
    // Three frames fill the jitter buffer; three 20 ms frames played = six 10 ms healer frames.
    for f in 0..3 {
        b.on_rtp(&a.on_capture(&tone(160, 8000.0, f * 160)).unwrap(), 0);
    }
    for _ in 0..3 {
        b.playout();
    }
    // A's SR reaches B at 1000 ms; B reports 500 ms later.
    let ntp = 0x0001_2345_6789_abcd_u64;
    for p in a.rtcp_due(0, ntp) {
        b.on_rtcp(&p, 1000);
    }
    let due = b.rtcp_due(1500, 0);
    assert_eq!(due.len(), 2);
    assert_eq!(
        rtp::parse_rtcp(&due[0]).unwrap().len(),
        1,
        "probe: a simple SR"
    );
    let (blocks, ext) = report_of(&due[1]);
    assert_eq!(blocks[0].ssrc, a.sender.ssrc);
    assert_eq!((blocks[0].lsr, blocks[0].dlsr), (0x2345_6789, 32768));
    assert!(
        matches!(ext[..], [rtp::Extension::AudioHealer { ssrc, total: 6, .. }] if ssrc == a.sender.ssrc)
    );
    assert!(rtp::parse_rtcp(&due[1])
        .unwrap()
        .contains(&rtp::Rtcp::MediaQuality {
            ssrc: b.sender.ssrc,
            quality: rtp::Quality::default()
        }));

    // B's report about A's stream: A switches to the packet pair fast rate (250 ms × 40).
    for p in &due {
        a.on_rtcp(p, 1600);
    }
    assert_eq!(a.rate, Rate::PairFast);
    let mut t = 5000;
    let mut fast = 0;
    while a.rate == Rate::PairFast {
        assert_eq!(a.rtcp_due(t, 0).len(), 2);
        assert!(a.rtcp_due(t + PAIR_FAST_MS - 1, 0).is_empty());
        t += PAIR_FAST_MS;
        fast += 1;
    }
    assert_eq!(fast, PAIR_FAST_COUNT);
    // Another report does not restart it.
    a.on_rtcp(&due[1], t);
    assert_eq!(a.rate, Rate::Normal);

    // A positive estimate ends the fast rate early.
    let (mut c, _) = plain_pair();
    let c_ssrc = c.sender.ssrc;
    let about_c = |bps| {
        let block = rtp::ReportBlock {
            ssrc: c_ssrc,
            fraction_lost: 0,
            cumulative_lost: 0,
            highest_sequence: 0,
            jitter: 0,
            lsr: 0,
            dlsr: 0,
        };
        let ext = [rtp::Extension::Bandwidth {
            ssrc: c_ssrc,
            bps,
            confidence: None,
        }];
        rtp::Report {
            ssrc: 77,
            info: None,
            reports: &[block],
            extensions: &ext,
            cname: "x",
            quality: None,
        }
        .build()
    };
    c.on_rtcp(&about_c(rtp::BANDWIDTH_PACKET_PAIR), 0);
    assert_eq!(c.rate, Rate::PairFast, "−3 is no estimate");
    c.on_rtcp(&about_c(1_000_000), 0);
    assert_eq!(
        (c.rate, c.peer.estimated_bps),
        (Rate::Normal, Some(1_000_000))
    );

    // −6: packet trains, 1 s × 21; pair packet and five train packets of the same size.
    c.on_rtcp(&about_c(rtp::BANDWIDTH_SEND_TRAIN), 10_000);
    assert_eq!(c.rate, Rate::TrainFast);
    let train = c.rtcp_due(11_000, 0);
    assert_eq!(train.len(), 2 + usize::from(TRAIN_PACKETS));
    assert!(
        train[1..].iter().all(|p| p.len() == train[1].len()),
        "equal sizes"
    );
    let (_, last) = report_of(train.last().unwrap());
    assert!(matches!(
        last[0],
        rtp::Extension::PacketTrain {
            last: true,
            index: 4,
            count: 5,
            ..
        }
    ));
    assert!(c.rtcp_due(11_999, 0).is_empty());

    // TURN bandwidth: in the next five reports, then no more.
    c.set_turn_bandwidth(2_000_000);
    let mut seen = 0;
    for k in 0..10u64 {
        let p = c.rtcp_due(20_000 + k * 6000, 0);
        let (_, ext) = report_of(&p[1]);
        seen += ext
            .iter()
            .filter(|e| **e == rtp::Extension::TurnBandwidth(2_000_000))
            .count();
    }
    assert_eq!(seen, 5);
}

/// SD-50 (MS-SDPEXT §3.1.5.12.2): early media sends no speech; DTMF only after the first
/// received packet; after the answer speech flows again.
#[test]
fn early_media_sends_only_dtmf_after_reception() {
    let (mut a, mut b) = pair(sdp::Codec::Pcmu, 0);
    a.set_early(true);
    assert!(a.on_capture(&tone(160, 8000.0, 0)).is_none(), "no speech");
    assert!(a.send_dtmf('5'));
    assert!(
        a.on_capture(&tone(160, 8000.0, 0)).is_none(),
        "no DTMF before a packet arrived"
    );
    // A packet from the far end arrives (e.g. an announcement).
    a.on_rtp(&b.on_capture(&tone(160, 8000.0, 0)).unwrap(), 0);
    let mut digits = Vec::new();
    let mut sent = 0;
    while let Some(p) = a.on_capture(&tone(160, 8000.0, 0)) {
        sent += 1;
        digits.extend(b.on_rtp(&p, 0));
    }
    assert!(sent > 0);
    assert_eq!(digits, vec!['5'], "only the DTMF packets went out");
    // Answered: speech.
    a.set_early(false);
    assert!(a.on_capture(&tone(160, 8000.0, 0)).is_some());
}

/// [MS-RTP] §3.2: the policy server's bandwidth goes to the peer in five reports.
#[test]
fn policy_bandwidth_five_times() {
    let (mut a, _) = plain_pair();
    a.set_policy_bandwidth(85_000);
    let mut seen = 0;
    for k in 0..8u64 {
        let p = a.rtcp_due(k * 6000, 0);
        let (_, ext) = report_of(&p[1]);
        seen += ext
            .iter()
            .filter(|e| **e == rtp::Extension::PolicyBandwidth(85_000))
            .count();
    }
    assert_eq!(seen, 5);
}

/// SD-12 / [MS-SSRTP]: with an SSRTP answer, packets of the MCU are taken as SSRTP; a packet in
/// SRTP format is refused.
#[test]
fn receives_ssrtp_from_an_mcu() {
    let mcu = key(2);
    let n = sdp::Negotiated {
        codec: sdp::Codec::Pcmu,
        payload_type: 0,
        dtmf_payload_type: None,
        remote_rtp: ("192.0.2.1".parse().unwrap(), 5000),
        remote_rtcp: ("192.0.2.1".parse().unwrap(), 5001),
        remote_key: Some(mcu.clone()),
        remote_ice: None,
        direction: sdp::Direction::SendRecv,
        music_on_hold: false,
        ssrtp: true,
    };
    let mut b = Audio::new(&n, Some(&key(1)), [2; 10], "b@client").unwrap();
    let keys = srtp::SessionKeys::derive(&mcu.key, &mcu.salt, false);
    let header = [0x80, 0, 0, 7, 0, 0, 0, 160, 0xAB, 0xCD, 0, 1];
    let payload = vec![0xffu8; 160];
    let p = srtp::ssrtp_protect(&keys, &header, &payload, 0x0100_0000_0001, 0).unwrap();
    let mut wire = header.to_vec();
    wire.extend(p.encrypted_payload);
    wire.extend(&0x0100_0000_0001u64.to_be_bytes()[2..]);
    wire.push(1);
    wire.extend(p.tag);
    b.on_rtp(&wire, 0);
    assert_eq!(b.stats.received, 1);
    // SRTP format with the same key: not SSRTP, refused.
    let mut srtp_tx = srtp::Context::new(&mcu.key, &mcu.salt, Some(1));
    let mut plain = header.to_vec();
    plain[3] = 8;
    plain.extend(&payload);
    b.on_rtp(&srtp_tx.protect_rtp(&plain).unwrap(), 0);
    assert_eq!(b.stats.received, 1);
}

/// Function 45 (QE-02, OB-47; RFC 3550 §6.4.1): the quality figures of both directions – our
/// reception (loss of one packet in 20), the peer's report about our stream and the round trip
/// from LSR/DLSR (A sends its report at 1000 ms, B gets it at 1050 ms and reports 300 ms later,
/// A gets that at 1450 ms: RTT = 1450 − 1000 − 300 = 150 ms, ±1 ms rounding of DLSR).
#[test]
fn quality_figures() {
    let (mut a, mut b) = pair(sdp::Codec::G722, 9);
    assert!(a.quality().inbound.is_none(), "nothing received yet");
    for f in 0..20 {
        let p = a.on_capture(&tone(320, 16000.0, f * 320)).unwrap();
        if f != 10 {
            b.on_rtp(&p, f as u32 * 160);
        }
    }
    for p in a.rtcp_due(1000, 0x1234_5678_9abc_0000) {
        b.on_rtcp(&p, 1050);
    }
    for p in b.rtcp_due(1350, 0x0000_0001_0000_0000) {
        a.on_rtcp(&p, 1450);
    }
    let qa = a.quality();
    assert_eq!((qa.codec, qa.sample_rate), ("G722", 16000));
    assert_eq!(qa.outbound.packets, 20);
    let rtt = qa.outbound.rtt_avg_ms.unwrap();
    assert!((150..=151).contains(&rtt), "{rtt}");
    assert!(
        qa.outbound.loss_max_ppm.unwrap() > 0,
        "the peer reported a loss"
    );
    let qb = b.quality();
    let inb = qb.inbound.unwrap();
    assert_eq!(inb.packets, 19);
    assert_eq!(inb.ssrc, qa.outbound.ssrc);
    assert!(inb.loss_max_ppm.unwrap() > 0 && inb.jitter_avg_ms.is_some());
}
