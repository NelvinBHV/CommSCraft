use super::*;

#[test]
fn header_round_trip_and_validation() {
    let h = Header {
        marker: true,
        payload_type: 9,
        sequence: 0x1234,
        timestamp: 0xdecafbad,
        ssrc: 0xcafebabe,
        csrc: vec![7, 8],
    };
    let p = build(&h, &[1, 2, 3]).unwrap();
    assert_eq!(&p[..2], &[0x82, 0x89]);
    let parsed = parse(&p).unwrap();
    assert_eq!((parsed.header, parsed.payload), (h, &[1u8, 2, 3][..]));
    // Padding (P bit, last octet = count) and a header extension are removed / skipped.
    let mut q = vec![
        0xb0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0xbe, 0xde, 0, 1, 9, 9, 9, 9, 5, 6, 0, 2,
    ];
    let parsed = parse(&q).unwrap();
    assert_eq!(parsed.payload, &[5, 6]);
    q[23] = 30;
    assert_eq!(parse(&q), Err(Error::Malformed));
    assert_eq!(parse(&[0x40; 12]), Err(Error::Version));
    assert_eq!(
        parse(&[0x8f; 20]),
        Err(Error::Malformed),
        "CSRC list longer than the packet"
    );
    assert_eq!(parse(&vec![0x80; 1501]), Err(Error::TooLarge));
}

#[test]
fn sender_numbers() {
    let mut s = Sender::new([0, 0, 0, 0, 0xff, 0xfe, 0, 0, 0, 100]);
    assert_eq!(s.ssrc, 1, "SSRC 0 is never used");
    let a = parse_owned(&s.packet(9, true, &[0; 160], 160).unwrap());
    let b = parse_owned(&s.packet(9, false, &[0; 160], 160).unwrap());
    assert_eq!(
        a.sequence, 0x7ffe,
        "start sequence below 32768 (MS-SRTP §5.1)"
    );
    assert_eq!((b.sequence, b.timestamp - a.timestamp), (0x7fff, 160));
    assert_eq!((s.packets, s.octets), (2, 320));
}

fn parse_owned(p: &[u8]) -> Header {
    parse(p).unwrap().header
}

/// RFC 3550 A.8 (integer form) with hand-computed values, and A.3 loss counting.
#[test]
fn receiver_statistics() {
    let mut r = Receiver::default();
    let h = |seq: u16, ts: u32| Header {
        marker: false,
        payload_type: 0,
        sequence: seq,
        timestamp: ts,
        ssrc: 1,
        csrc: vec![],
    };
    // Transit times 1000, 1016, 1000: |D| = 16 then 16.
    assert!(r.on_packet(&h(65534, 0), 1000, true));
    assert!(r.on_packet(&h(65535, 160), 1176, true));
    // j16 = 0 + 16 − ((0+8)>>4) = 16; then 16 + 16 − ((16+8)>>4) = 31.
    assert_eq!(r.jitter16, 16);
    assert!(r.on_packet(&h(1, 480), 1480, true)); // seq 0 lost, wrap
    assert_eq!(r.jitter16, 31);
    assert_eq!(r.jitter(), 1);
    // A telephone event does not change the jitter (RT-10).
    assert!(r.on_packet(&h(2, 480), 9999, false));
    assert_eq!(r.jitter16, 31);
    assert_eq!(r.extended_max(), 65536 + 2);
    let rb = r.report(1, 0, 0);
    assert_eq!((rb.cumulative_lost, rb.fraction_lost), (1, 51));
    // An implausible jump is not counted.
    assert!(!r.on_packet(&h(30000, 0), 0, true));
}

#[test]
fn rtcp_compound_and_bye() {
    let rb = ReportBlock {
        ssrc: 9,
        fraction_lost: 12,
        cumulative_lost: -3,
        highest_sequence: 70000,
        jitter: 5,
        lsr: 6,
        dlsr: 7,
    };
    let info = SenderInfo {
        ntp: 0x0102030405060708,
        rtp_timestamp: 160,
        packets: 3,
        octets: 480,
    };
    let p = compound(0xcafebabe, Some(info), &[rb], "alice@client\u{7}");
    assert_eq!(p.len() % 4, 0);
    // SR length field: (4 + 24 + 24) / 4 words + header − 1.
    assert_eq!(&p[..4], &[0x81, 200, 0, 12]);
    let parsed = parse_rtcp(&p).unwrap();
    assert_eq!(
        parsed,
        vec![
            Rtcp::SenderReport {
                ssrc: 0xcafebabe,
                info,
                reports: vec![rb]
            },
            Rtcp::Cname {
                ssrc: 0xcafebabe,
                cname: "alice@client".into()
            }
        ]
    );
    let b = bye(5, "x");
    assert!(parse_rtcp(&b)
        .unwrap()
        .contains(&Rtcp::Bye { sources: vec![5] }));
    // Truncated or hostile packets.
    assert_eq!(parse_rtcp(&p[..p.len() - 4]), Err(Error::Malformed));
    assert_eq!(
        parse_rtcp(&[0x9f, 201, 0, 1, 0, 0, 0, 1]),
        Err(Error::Malformed)
    );
    assert_eq!(
        parse_rtcp(&[0x80, 210, 0, 0]).unwrap(),
        vec![],
        "unknown type skipped"
    );
}

/// RFC 4733 §2.3 layout, §2.5.1.2 marker/end, §2.5.1.4 three final packets; MS-DTMF limits.
#[test]
fn telephone_events() {
    assert_eq!(dtmf_event('5'), Some(5));
    assert_eq!(dtmf_event('#'), Some(11));
    assert_eq!(dtmf_event('d'), Some(15));
    assert_eq!(dtmf_event('x'), None);
    assert_eq!(dtmf_char(10), Some('*'));
    let e = TelephoneEvent {
        event: 11,
        end: true,
        volume: 10,
        duration: 800,
    };
    assert_eq!(e.encode(), [11, 0x8a, 0x03, 0x20]);
    assert_eq!(TelephoneEvent::decode(&e.encode()), Some(e));
    assert_eq!(TelephoneEvent::decode(&[17, 0, 0, 1]), None);
    assert_eq!(TelephoneEvent::decode(&[1, 0, 0, 0]), None, "zero duration");
    assert_eq!(
        TelephoneEvent::decode(&[1, 0, 0, 1, 2, 0, 0, 1]),
        None,
        "one event per packet"
    );
    // 100 ms digit with 20 ms updates at 8000 Hz.
    let pk = dtmf_packets(1, 800, 160, 10);
    assert_eq!(pk.len(), 4 + 3);
    assert!(pk[0].0 && pk.iter().skip(1).all(|(m, _)| !m));
    let decoded: Vec<_> = pk
        .iter()
        .map(|(_, p)| TelephoneEvent::decode(p).unwrap())
        .collect();
    assert_eq!(
        decoded.iter().map(|e| e.duration).collect::<Vec<_>>(),
        vec![160, 320, 480, 640, 800, 800, 800]
    );
    assert_eq!(decoded.iter().filter(|e| e.end).count(), 3);
}
