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

/// A pair without SRTP, so the tests can read the RTCP packets themselves.
fn plain_pair() -> (Video, Video) {
    let neg = |range| sdp::video::NegotiatedVideo {
        payload_type: 122,
        remote_rtp: ("192.0.2.1".parse().unwrap(), 1),
        remote_rtcp: ("192.0.2.1".parse().unwrap(), 2),
        remote_key: None,
        remote_ice: None,
        direction: sdp::Direction::SendRecv,
        ssrc_range: range,
        feedback: all_feedback(),
    };
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
