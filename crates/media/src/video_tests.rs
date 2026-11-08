use super::video::*;
use commscraft_h264uc as uc;
use commscraft_rtp as rtp;
use commscraft_sdp as sdp;

fn layout() -> uc::StreamLayout {
    uc::StreamLayout::full(vec![uc::LayerDescription {
        coded_width: 640,
        coded_height: 480,
        display_width: 640,
        display_height: 480,
        bitrate: 600_000,
        fps_idx: 4,
        layer_type: 0,
        prid: 0,
        constrained_baseline: true,
    }])
    .unwrap()
}

fn all_feedback() -> sdp::video::Feedback {
    sdp::video::Feedback {
        send_src: true,
        recv_src: true,
        send_pli: true,
        recv_pli: true,
    }
}

/// A pair of video cores facing each other: A has range 100–199 and key 1, B 200–299 and key 2.
fn pair(seq_zero: bool) -> (Video, Video) {
    let ka = sdp::Key::new(1, [1; 30]);
    let kb = sdp::Key::new(1, [2; 30]);
    let neg = |range, key: &sdp::Key| sdp::video::NegotiatedVideo {
        payload_type: 122,
        remote_rtp: ("192.0.2.1".parse().unwrap(), 1),
        remote_rtcp: ("192.0.2.1".parse().unwrap(), 2),
        remote_key: Some(key.clone()),
        remote_ice: None,
        direction: sdp::Direction::SendRecv,
        fec_payload_type: None,
        ssrc_range: range,
        feedback: all_feedback(),
    };
    let random = if seq_zero {
        [9, 9, 9, 9, 0, 0, 1, 2, 3, 4]
    } else {
        [9; 10]
    };
    let a = Video::new(
        &neg((200, 299), &kb),
        Some(&ka),
        (100, 199),
        layout(),
        random,
        "a",
    )
    .unwrap();
    let b = Video::new(
        &neg((100, 199), &ka),
        Some(&kb),
        (200, 299),
        layout(),
        [7; 10],
        "b",
    )
    .unwrap();
    (a, b)
}

fn au(idr: bool, size: usize) -> Vec<u8> {
    let mut v = Vec::new();
    if idr {
        v.extend_from_slice(&[
            0, 0, 0, 1, 0x67, 0x42, 0xc0, 0x1e, 0, 0, 0, 1, 0x68, 0xce, 0x3c,
        ]);
    }
    v.extend_from_slice(&[0, 0, 0, 1, if idr { 0x65 } else { 0x41 }]);
    v.extend((0..size).map(|i| (i % 250) as u8 + 1));
    v
}

fn nals(au: &[u8]) -> Vec<Vec<u8>> {
    uc::split_annex_b(au)
        .unwrap()
        .into_iter()
        .map(<[u8]>::to_vec)
        .collect()
}

/// PL-01 … PL-09 through SRTP: access units arrive unchanged; our SSRC is the start of our
/// range; sequence number 0 is skipped (note <2>).
#[test]
fn access_units_through_srtp() {
    let (mut a, mut b) = pair(true);
    assert_eq!(a.ssrc(), 100);
    for (n, (idr, size)) in [(true, 3000), (false, 400), (false, 2500)]
        .into_iter()
        .enumerate()
    {
        let unit = au(idr, size);
        let packets = a.on_access_unit(&unit, 3000 * n as u32);
        if n == 0 {
            assert_eq!(
                u16::from_be_bytes([packets[0][2], packets[0][3]]),
                1,
                "never sequence 0"
            );
        }
        let mut got = Vec::new();
        for p in &packets {
            got.extend(b.on_rtp(p, 0, 0));
        }
        match got.as_slice() {
            [VideoEvent::AccessUnit { data, idr: i }] => {
                assert_eq!(nals(data), nals(&unit));
                assert_eq!(*i, idr);
            }
            other => panic!("{other:?}"),
        }
    }
    assert_eq!((a.sent_units, b.received_units), (3, 3));
}

/// SD-64 / RT-02: packets of an SSRC outside the peer's range, of another payload type or
/// failing SRTP authentication are dropped without effect.
#[test]
fn foreign_packets_dropped() {
    let (mut a, mut b) = pair(false);
    let packets = a.on_access_unit(&au(true, 100), 0);
    let mut forged = packets[0].clone();
    forged[10] ^= 1; // the SSRC changes – authentication fails
    assert!(b.on_rtp(&forged, 0, 0).is_empty());
    // Plaintext RTP with an SSRC outside the range, to a core without SRTP.
    let neg = sdp::video::NegotiatedVideo {
        payload_type: 122,
        remote_rtp: ("192.0.2.1".parse().unwrap(), 1),
        remote_rtcp: ("192.0.2.1".parse().unwrap(), 2),
        remote_key: None,
        remote_ice: None,
        direction: sdp::Direction::SendRecv,
        fec_payload_type: None,
        ssrc_range: (100, 199),
        feedback: all_feedback(),
    };
    let mut plain = Video::new(&neg, None, (200, 299), layout(), [3; 10], "p").unwrap();
    let mut s = rtp::Sender::new([0, 0, 1, 244, 0, 1, 0, 0, 0, 0]); // SSRC 500
    let pkt = s.packet(122, true, &[0x7c, 0x85, 1, 2], 0).unwrap();
    assert!(plain.on_rtp(&pkt, 0, 0).is_empty());
    // One side only encrypting is refused.
    assert!(Video::new(
        &neg,
        Some(&sdp::Key::new(1, [1; 30])),
        (200, 299),
        layout(),
        [3; 10],
        "p"
    )
    .is_err());
}

/// RT-25 / RT-26: a lost access unit makes the receiver send an extended PLI about the peer's
/// stream (repeated on the fast schedule); the sender reports a key frame request. A VSR goes
/// out with the key-frame flag and arrives as a source request.
#[test]
fn feedback_loop() {
    let (mut a, mut b) = pair(false);
    // Lose the middle packet of a fragmented access unit.
    let packets = a.on_access_unit(&au(true, 3000), 0);
    assert!(packets.len() >= 3);
    for (i, p) in packets.iter().enumerate() {
        if i != 1 {
            b.on_rtp(p, 0, 1000);
        }
    }
    assert_eq!(b.lost_units, 1);
    let rtcp = b.rtcp_due(1000, 0);
    let mut events = Vec::new();
    for p in &rtcp {
        events.extend(a.on_rtcp(p, 1000));
    }
    assert!(
        events.contains(&VideoEvent::KeyFrameRequested),
        "{events:?}"
    );
    // Repeated after 190 ms, not before.
    assert!(b.rtcp_due(1100, 0).is_empty());
    assert_eq!(b.rtcp_due(1190, 0).len(), 1);
    // VSR from B for any source with VGA at 30 fps.
    let entry = rtp::feedback::VsrEntry {
        payload_type: 122,
        ucconfig_mode: 1,
        flags: rtp::feedback::FLAG_CONSTRAINED_BASELINE,
        aspect_ratios: rtp::feedback::ASPECT_4_3,
        max_width: 640,
        max_height: 480,
        min_bitrate: 100_000,
        frame_rates: rtp::feedback::FPS_30,
        max_pixels: 640 * 480,
        ..Default::default()
    };
    b.request_source(rtp::feedback::SOURCE_ANY, entry, 5000);
    let mut events = Vec::new();
    for p in b.rtcp_due(5000, 0) {
        events.extend(a.on_rtcp(&p, 5000));
    }
    assert!(events.contains(&VideoEvent::KeyFrameRequested));
    assert!(events.iter().any(|e| matches!(e, VideoEvent::SourceRequested { msi, entries } if *msi == rtp::feedback::SOURCE_ANY && entries.len() == 1)));
}

fn plain_neg(range: (u32, u32)) -> sdp::video::NegotiatedVideo {
    sdp::video::NegotiatedVideo {
        payload_type: 122,
        remote_rtp: ("192.0.2.1".parse().unwrap(), 1),
        remote_rtcp: ("192.0.2.1".parse().unwrap(), 2),
        remote_key: None,
        remote_ice: None,
        direction: sdp::Direction::SendRecv,
        fec_payload_type: None,
        ssrc_range: range,
        feedback: all_feedback(),
    }
}

/// A pair without SRTP, so the tests can read the RTCP packets themselves.
fn plain_pair() -> (Video, Video) {
    let neg = plain_neg;
    let a = Video::new(&neg((200, 299)), None, (100, 199), layout(), [9; 10], "a").unwrap();
    let b = Video::new(&neg((100, 199)), None, (200, 299), layout(), [7; 10], "b").unwrap();
    (a, b)
}

fn extensions_of(packets: &[Vec<u8>]) -> Vec<rtp::Extension> {
    packets
        .iter()
        .flat_map(|p| rtp::parse_rtcp(p).unwrap())
        .flat_map(|i| match i {
            rtp::Rtcp::SenderReport { extensions, .. }
            | rtp::Rtcp::ReceiverReport { extensions, .. } => extensions,
            _ => Vec::new(),
        })
        .collect()
}

/// RT-28 … RT-30: every report carries our estimate of the peer's stream – also before it
/// sends (about the start of its range) –, the receiver-side limit five times and peer info
/// until the peer reported an estimate for us; a probe goes right before the report; the
/// peer's estimate becomes our send rate.
#[test]
fn rate_feedback_in_reports() {
    let (mut a, mut b) = plain_pair();
    let first = a.rtcp_due(0, 0);
    assert_eq!(first.len(), 2, "packet pair: probe + report");
    assert_eq!(
        rtp::parse_rtcp(&first[0]).unwrap().len(),
        1,
        "probe: a simple SR"
    );
    let ext = extensions_of(&first);
    assert!(ext.contains(&rtp::Extension::Bandwidth {
        ssrc: 200,
        bps: 1_500_000,
        confidence: Some(2)
    }));
    assert!(ext.contains(&rtp::Extension::ReceiverLimit(1_500_000)));
    assert!(ext.iter().any(|e| matches!(
        e,
        rtp::Extension::PeerInfo {
            ssrc: 100,
            no_cache: true,
            ..
        }
    )));
    // B learns what it may send to A.
    let mut events = Vec::new();
    for p in &first {
        events.extend(b.on_rtcp(p, 0));
    }
    assert_eq!(events, vec![VideoEvent::SendRate(1_500_000)]);
    assert_eq!(b.send_target(0), Some(1_500_000));
    // The same rate again is no new event.
    assert!(b.on_rtcp(&first[1], 10).is_empty());
    // B got an estimate for itself: no peer info from B; A still sends it.
    let ext_b = extensions_of(&b.rtcp_due(0, 0));
    assert!(!ext_b
        .iter()
        .any(|e| matches!(e, rtp::Extension::PeerInfo { .. })));
    // The limit five times in all.
    let mut limits = 1;
    for k in 1..8u64 {
        let ext = extensions_of(&a.rtcp_due(k * 5000, 0));
        limits += ext
            .iter()
            .filter(|e| matches!(e, rtp::Extension::ReceiverLimit(_)))
            .count();
        assert!(ext
            .iter()
            .any(|e| matches!(e, rtp::Extension::Bandwidth { ssrc: 200, .. })));
    }
    assert_eq!(limits, 5);
    // Our capacity can be set (start value and limit).
    a.set_receive_capacity(600_000);
    assert_eq!(a.receive_estimate(), 600_000);
}

/// §2.2.11.2: a gap in the sequence numbers is reported at once (not with the next regular
/// report) and at most every 500 ms; the sender answers it with a key frame request.
#[test]
fn packet_loss_notification() {
    let (mut a, mut b) = plain_pair();
    let _ = b.rtcp_due(0, 0);
    let mut sent = Vec::new();
    for k in 0..3u32 {
        sent.extend(a.on_access_unit(&au(k == 0, 100), k * 3000));
    }
    assert_eq!(sent.len(), 3);
    let lost = rtp::parse(&sent[1]).unwrap().header.sequence;
    b.on_rtp(&sent[0], 0, 100);
    b.on_rtp(&sent[2], 0, 120);
    let early = b.rtcp_due(130, 0);
    let ext = extensions_of(&early);
    assert!(
        ext.contains(&rtp::Extension::PacketLoss { sequence: lost }),
        "{ext:?}"
    );
    let mut events = Vec::new();
    for p in &early {
        events.extend(a.on_rtcp(p, 130));
    }
    assert!(events.contains(&VideoEvent::KeyFrameRequested));
    // Another gap 200 ms later waits until 500 ms have passed.
    let more: Vec<Vec<u8>> = (3..6u32)
        .flat_map(|k| a.on_access_unit(&au(false, 100), k * 3000))
        .collect();
    b.on_rtp(&more[0], 0, 300);
    b.on_rtp(&more[2], 0, 320);
    let too_soon = extensions_of(&b.rtcp_due(330, 0));
    assert!(!too_soon
        .iter()
        .any(|e| matches!(e, rtp::Extension::PacketLoss { .. })));
    let later = extensions_of(&b.rtcp_due(640, 0));
    let second = rtp::parse(&more[1]).unwrap().header.sequence;
    assert!(later.contains(&rtp::Extension::PacketLoss { sequence: second }));
}

/// Function 31 (PL-50): a lost picture from a peer that takes no PLI asks the application for a
/// signalled picture update – at most once a second; with PLI negotiated it does not.
#[test]
fn picture_update_without_pli() {
    let neg = |range, pli| sdp::video::NegotiatedVideo {
        payload_type: 122,
        remote_rtp: ("192.0.2.1".parse().unwrap(), 1),
        remote_rtcp: ("192.0.2.1".parse().unwrap(), 2),
        remote_key: None,
        remote_ice: None,
        direction: sdp::Direction::SendRecv,
        fec_payload_type: None,
        ssrc_range: range,
        feedback: sdp::video::Feedback {
            recv_pli: pli,
            ..all_feedback()
        },
    };
    for pli in [false, true] {
        let mut a = Video::new(
            &neg((200, 299), true),
            None,
            (100, 199),
            layout(),
            [9; 10],
            "a",
        )
        .unwrap();
        let mut b = Video::new(
            &neg((100, 199), pli),
            None,
            (200, 299),
            layout(),
            [7; 10],
            "b",
        )
        .unwrap();
        let mut asked = 0;
        for (k, at) in [(0u32, 0u64), (1, 300), (2, 1300)] {
            let packets = a.on_access_unit(&au(k == 0, 3000), k * 3000);
            assert!(packets.len() >= 3);
            for (i, p) in packets.iter().enumerate() {
                if i != 1 {
                    asked += b
                        .on_rtp(p, 0, at)
                        .iter()
                        .filter(|e| **e == VideoEvent::PictureUpdateNeeded)
                        .count();
                }
            }
        }
        // Three lost pictures at 0, 300 and 1300 ms: two requests without PLI, none with it.
        assert_eq!(asked, if pli { 0 } else { 2 }, "pli negotiated: {pli}");
    }
}

/// PL-06: a pair with FEC. An IDR picture gets a FEC packet behind its data, with the marker, and a
/// lost packet of it comes back without a key frame request; an ordinary picture gets FEC only
/// after the peer reported loss.
#[test]
fn fec_recovers_a_lost_packet() {
    let ka = sdp::Key::new(1, [1; 30]);
    let kb = sdp::Key::new(1, [2; 30]);
    let neg = |range, key: &sdp::Key, fec| sdp::video::NegotiatedVideo {
        payload_type: 122,
        remote_rtp: ("192.0.2.1".parse().unwrap(), 1),
        remote_rtcp: ("192.0.2.1".parse().unwrap(), 2),
        remote_key: Some(key.clone()),
        remote_ice: None,
        direction: sdp::Direction::SendRecv,
        fec_payload_type: fec,
        ssrc_range: range,
        feedback: all_feedback(),
    };
    let mut a = Video::new(
        &neg((200, 299), &kb, Some(123)),
        Some(&ka),
        (100, 199),
        layout(),
        [9; 10],
        "a",
    )
    .unwrap();
    let mut b = Video::new(
        &neg((100, 199), &ka, Some(123)),
        Some(&kb),
        (200, 299),
        layout(),
        [8; 10],
        "b",
    )
    .unwrap();
    let plain_type = |p: &[u8]| p.get(1).map(|b| b & 0x7f);
    let idr = |n: usize| {
        let mut au = vec![0, 0, 0, 1, 0x65];
        au.extend((0..n).map(|i| (i % 200 + 1) as u8));
        au
    };
    // A large IDR picture (the stream layout goes with the first one).
    let mut packets = a.on_access_unit(&idr(4000), 9000);
    assert!(packets.len() >= 3);
    let fec = packets.last().unwrap().clone();
    assert_eq!(plain_type(&fec), Some(123));
    assert_eq!(
        fec[1] & 0x80,
        0x80,
        "the last FEC packet of the unit has the marker"
    );
    // One lost in the middle: nothing is lost in the end, no key frame request is due.
    packets.remove(1);
    let mut units = Vec::new();
    for (i, p) in packets.iter().enumerate() {
        for e in b.on_rtp(p, i as u32, 1000) {
            if let VideoEvent::AccessUnit { data, idr } = e {
                assert!(idr);
                units.push(data);
            }
        }
    }
    assert_eq!(units.len(), 1, "recovered");
    assert_eq!(b.lost_units, 0);
    assert_eq!(b.recovered_packets(), 1);
    // An ordinary picture has no FEC ...
    let inter = |n: usize| {
        let mut au = vec![0, 0, 0, 1, 0x41];
        au.extend((0..n).map(|i| (i % 200 + 1) as u8));
        au
    };
    let before = a.on_access_unit(&inter(4000), 12000);
    assert!(before.iter().all(|p| plain_type(p) == Some(122)));
    // ... until the peer reports loss (it saw the gap, also of the recovered packet).
    for report in b.rtcp_due(100_000, 1 << 40) {
        let _ = a.on_rtcp(&report, 100_000);
    }
    let after = a.on_access_unit(&inter(4000), 15000);
    assert_eq!(plain_type(after.last().unwrap()), Some(123));
    // A single small packet gets none.
    let small = a.on_access_unit(&inter(20), 18000);
    assert!(small.iter().all(|p| plain_type(p) == Some(122)));

    // Without FEC negotiated its packets are not taken.
    let mut c = Video::new(
        &neg((100, 199), &ka, None),
        Some(&kb),
        (200, 299),
        layout(),
        [7; 10],
        "c",
    )
    .unwrap();
    for (i, p) in after.iter().enumerate() {
        let _ = c.on_rtp(p, i as u32, 3000);
    }
    assert_eq!(c.recovered_packets(), 0);
}

fn layer(prid: u8, width: u16, height: u16) -> uc::LayerDescription {
    uc::LayerDescription {
        coded_width: width,
        coded_height: height,
        display_width: width,
        display_height: height,
        bitrate: 200_000,
        fps_idx: 2,
        layer_type: 0,
        prid,
        constrained_baseline: true,
    }
}

/// MS-H264PF §3.1.5.3 / MS-RTP §3.1: a second simulcast stream goes out on its own SSRC (the
/// start of our range + its priority ID; inference) with its own sequence numbers and PRID 1 in
/// PACSI and prefix NAL unit, and both carry the full stream layout of both streams. A stream
/// not in the layout sends nothing; the one sender report comes from the lowest SSRC and counts
/// both streams (§2.2.8); a PLI about the second stream asks for a key frame.
#[test]
fn simulcast_streams() {
    let (mut a, mut b) = plain_pair();
    assert!(a.on_layer_access_unit(1, &au(true, 100), 0).is_empty());
    let both = uc::StreamLayout::full(vec![layout().layers[0], layer(1, 320, 180)]).unwrap();
    a.set_layout(both.clone()).unwrap();
    assert_eq!(a.prids(), vec![0, 1]);
    let big = a.on_layer_access_unit(0, &au(true, 3000), 9000);
    let small = a.on_layer_access_unit(1, &au(true, 300), 9000);
    let ssrcs = |ps: &[Vec<u8>]| -> Vec<u32> {
        ps.iter()
            .map(|p| rtp::parse(p).unwrap().header.ssrc)
            .collect()
    };
    assert!(ssrcs(&big).iter().all(|s| *s == 100));
    assert!(ssrcs(&small).iter().all(|s| *s == 101));
    assert_eq!(a.sent_units, 1, "an access unit counts once per picture");
    // Each stream decodes on its own and names its PRID and the whole layout.
    for (packets, prid) in [(&big, 0u8), (&small, 1)] {
        let mut d = uc::Depacketizer::new();
        let mut units = Vec::new();
        for p in packets.iter() {
            let r = rtp::parse(p).unwrap();
            let pacsi = match uc::nal_type(r.payload) {
                Some(uc::NAL_STAP_A) => {
                    let len = usize::from(u16::from_be_bytes([r.payload[1], r.payload[2]]));
                    Some(uc::Pacsi::parse(&r.payload[3..3 + len]).unwrap())
                }
                _ => None,
            };
            if let Some(p) = pacsi {
                assert_eq!(p.prid, prid);
                assert_eq!(p.layout.as_ref(), Some(&both));
            }
            units.extend(d.push(uc::RtpIn {
                seq: r.header.sequence,
                timestamp: r.header.timestamp,
                marker: r.header.marker,
                payload: r.payload,
            }));
        }
        assert!(
            matches!(units.as_slice(), [uc::Output::AccessUnit { idr: true, .. }]),
            "{units:?}"
        );
    }
    // Our peer takes the first stream it sees (the A/V MCU forwards one, §3.1.5.3.1).
    let got: Vec<VideoEvent> = big.iter().flat_map(|p| b.on_rtp(p, 0, 0)).collect();
    assert_eq!(got.len(), 1);
    // One sender report from SSRC 100 with the packets of both streams.
    let reports: Vec<rtp::Rtcp> = a
        .rtcp_due(0, 0)
        .iter()
        .flat_map(|p| rtp::parse_rtcp(p).unwrap())
        .collect();
    let counted = (big.len() + small.len()) as u32;
    assert!(reports.iter().any(|r| matches!(r,
        rtp::Rtcp::SenderReport { ssrc: 100, info, .. } if info.packets == counted)));
    assert!(!reports
        .iter()
        .any(|r| matches!(r, rtp::Rtcp::SenderReport { ssrc: 101, .. })));
    let pli = rtp::feedback::Pli {
        sender_ssrc: 200,
        media_ssrc: 101,
        extended: Some((1, 2)),
    }
    .to_bytes()
    .unwrap();
    assert!(a.on_rtcp(&pli, 0).contains(&VideoEvent::KeyFrameRequested));
    // Back to one stream: the second one ends.
    a.set_layout(layout()).unwrap();
    assert_eq!(a.prids(), vec![0]);
    assert!(a.on_layer_access_unit(1, &au(true, 100), 0).is_empty());
    // A priority ID beyond our SSRC range, or a layout without PRID 0, is refused.
    let far = uc::StreamLayout::full(vec![layout().layers[0], layer(63, 160, 120)]).unwrap();
    let mut narrow = Video::new(
        &plain_neg((200, 299)),
        None,
        (100, 110),
        layout(),
        [1; 10],
        "n",
    )
    .unwrap();
    assert!(narrow.set_layout(far).is_err());
    let without_base = uc::StreamLayout::full(vec![layer(1, 320, 180)]).unwrap();
    assert!(a.set_layout(without_base).is_err());
    assert_eq!(a.prids(), vec![0]);
}

/// A second stream gets other sequence numbers than the first (its own start).
#[test]
fn simulcast_sequence_numbers_differ() {
    let (mut a, _) = plain_pair();
    let both = uc::StreamLayout::full(vec![layout().layers[0], layer(1, 320, 180)]).unwrap();
    a.set_layout(both).unwrap();
    let seq = |p: &Vec<u8>| rtp::parse(p).unwrap().header.sequence;
    let s0 = seq(&a.on_layer_access_unit(0, &au(false, 100), 0)[0]);
    let s1 = seq(&a.on_layer_access_unit(1, &au(false, 100), 0)[0]);
    assert_ne!(s0, s1);
    assert!(s1 < 0x8000, "MS-SRTP §5.1: a low start");
}

/// MS-RTP §3.1: within the peer's range the stream may move to another SSRC (an A/V MCU
/// forwarding another simulcast stream). The new SSRC takes over after 200 ms
/// of silence on the current one, its access units arrive, and a PLI about it goes out; a
/// second SSRC while the current one still sends is dropped.
#[test]
fn stream_moves_to_another_ssrc() {
    // A takes the peer's range 200–299.
    let (mut b, _) = plain_pair();
    let mut first = Video::new(
        &plain_neg((100, 199)),
        None,
        (200, 299),
        layout(),
        [7; 10],
        "f",
    )
    .unwrap();
    let mut second = Video::new(
        &plain_neg((100, 199)),
        None,
        (201, 299),
        layout(),
        [5; 10],
        "s",
    )
    .unwrap();
    let units = |v: &mut Video, b: &mut Video, now: u64| -> usize {
        v.on_access_unit(&au(true, 500), (now * 90) as u32)
            .iter()
            .flat_map(|p| b.on_rtp(p, 0, now))
            .filter(|e| matches!(e, VideoEvent::AccessUnit { .. }))
            .count()
    };
    assert_eq!(units(&mut first, &mut b, 1000), 1);
    assert_eq!(
        units(&mut second, &mut b, 1100),
        0,
        "the current one still sends"
    );
    assert_eq!(units(&mut first, &mut b, 1150), 1);
    assert_eq!(
        units(&mut second, &mut b, 1400),
        1,
        "silent for 250 ms: followed"
    );
    assert_eq!(
        units(&mut first, &mut b, 1450),
        0,
        "the old one is gone now"
    );
    let pli = b
        .rtcp_due(1400, 0)
        .iter()
        .flat_map(|p| rtp::parse_rtcp(p).unwrap())
        .any(|r| matches!(r, rtp::Rtcp::Feedback(rtp::feedback::Feedback::Pli(p)) if p.media_ssrc == 201));
    assert!(pli, "a key frame from the new stream");
    assert_eq!(b.received_units, 3);
}

/// MS-RTP §2.2.12.1: the PLI asks for a sync frame on the stream that arrives – priority ID 1
/// when a mixer forwards our peer's second simulcast stream.
#[test]
fn pli_names_the_stream_that_arrives() {
    let (mut b, _) = plain_pair();
    let both = uc::StreamLayout::full(vec![layout().layers[0], layer(1, 320, 180)]).unwrap();
    let mut peer = Video::new(
        &plain_neg((100, 199)),
        None,
        (200, 299),
        both.clone(),
        [7; 10],
        "p",
    )
    .unwrap();
    peer.set_layout(both).unwrap();
    let first = peer.on_layer_access_unit(1, &au(true, 3000), 0);
    for p in &first {
        b.on_rtp(p, 0, 0);
    }
    // Lose a middle packet of the next one.
    let next = peer.on_layer_access_unit(1, &au(false, 3000), 3000);
    for (i, p) in next.iter().enumerate() {
        if i != 1 {
            b.on_rtp(p, 0, 10);
        }
    }
    assert_eq!(b.lost_units, 1);
    let bits: Vec<Option<(u16, u64)>> = b
        .rtcp_due(10, 0)
        .iter()
        .flat_map(|p| rtp::parse_rtcp(p).unwrap())
        .filter_map(|r| match r {
            rtp::Rtcp::Feedback(rtp::feedback::Feedback::Pli(p)) => Some(p.extended),
            _ => None,
        })
        .collect();
    assert_eq!(bits.len(), 1);
    assert_eq!(bits[0].map(|(_, b)| b), Some(0b10));
}
