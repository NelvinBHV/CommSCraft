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
                reports: vec![rb],
                extensions: vec![]
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

/// [MS-RTP] §2.2.11 (RT-23): every extension type survives a round trip with the sizes of the
/// specification; unknown types and wrong sizes are skipped, at most 20 are read.
#[test]
fn profile_extensions() {
    let all = [
        Extension::Bandwidth {
            ssrc: 1,
            bps: BANDWIDTH_PACKET_PAIR,
            confidence: None,
        },
        Extension::Bandwidth {
            ssrc: 1,
            bps: 2_000_000,
            confidence: Some(15),
        },
        Extension::PacketLoss { sequence: 513 },
        Extension::VideoPreference {
            width: 640,
            height: 480,
        },
        Extension::Padding { words: 3 },
        Extension::PolicyBandwidth(100_000),
        Extension::TurnBandwidth(200_000),
        Extension::AudioHealer {
            ssrc: 2,
            concealed: 3,
            stretched: 4,
            compressed: 5,
            total: 600,
            quality: 1,
            fec_distance: 2,
        },
        Extension::ReceiverLimit(300_000),
        Extension::PacketTrain {
            ssrc: 3,
            last: true,
            index: 4,
            count: 5,
            bytes: 700,
        },
        Extension::PeerInfo {
            ssrc: 4,
            inbound: 10,
            outbound: 20,
            no_cache: true,
        },
        Extension::Congestion {
            ntp: 0x0102_0304_0506_0708,
            info: 0b1010,
        },
        Extension::ModalityLimit {
            modality: 2,
            bps: 400_000,
        },
    ];
    let sizes = [12, 16, 8, 20, 16, 12, 12, 28, 12, 12, 20, 16, 12];
    for (e, size) in all.iter().zip(sizes) {
        let mut v = Vec::new();
        e.write(&mut v);
        assert_eq!(v.len(), size, "{e:?}");
        assert_eq!(u16::from_be_bytes([v[2], v[3]]) as usize, size);
    }
    let r = Report {
        ssrc: 9,
        info: None,
        reports: &[],
        extensions: &all,
        cname: "c",
        quality: None,
    }
    .build();
    let parsed = parse_rtcp(&r).unwrap();
    assert_eq!(
        parsed[0],
        Rtcp::ReceiverReport {
            ssrc: 9,
            reports: vec![],
            extensions: all.to_vec()
        }
    );

    // Unknown type 99 skipped by its length, then a broken length ends the walk.
    let mut body = 9u32.to_be_bytes().to_vec();
    body.extend_from_slice(&[0, 99, 0, 8, 1, 2, 3, 4]);
    body.extend_from_slice(&[0, 8, 0, 12, 0, 0, 0, 0, 0, 0, 0, 5]);
    body.extend_from_slice(&[0, 7, 0, 3]);
    let mut p = vec![0x80, 201];
    p.extend_from_slice(&(((body.len() + 4) / 4 - 1) as u16).to_be_bytes());
    p.extend_from_slice(&body);
    assert_eq!(
        parse_rtcp(&p).unwrap(),
        vec![Rtcp::ReceiverReport {
            ssrc: 9,
            reports: vec![],
            extensions: vec![Extension::TurnBandwidth(5)]
        }]
    );
    // Out-of-range quality and FEC values read as 0 (§2.2.11.7).
    let mut v = Vec::new();
    Extension::AudioHealer {
        ssrc: 1,
        concealed: 0,
        stretched: 0,
        compressed: 0,
        total: 0,
        quality: 0,
        fec_distance: 0,
    }
    .write(&mut v);
    v[26] = 9;
    v[27] = 7;
    assert!(matches!(
        read_extensions(&v)[..],
        [Extension::AudioHealer {
            quality: 0,
            fec_distance: 0,
            ..
        }]
    ));
    // At most 20 extensions.
    let many: Vec<u8> = std::iter::repeat_n([0u8, 6, 0, 4], 30).flatten().collect();
    assert_eq!(read_extensions(&many).len(), MAX_EXTENSIONS);
}

/// RT-22: SDES PRIV "MS-EVT" (prefix and value not NUL-terminated), CNAME NUL-terminated (RT-20).
#[test]
fn media_quality_sdes() {
    let q = Quality {
        known: 0x1,
        bad: 0x1,
    };
    let r = Report {
        ssrc: 7,
        info: None,
        reports: &[],
        extensions: &[],
        cname: "a@b",
        quality: Some(q),
    }
    .build();
    let text = String::from_utf8_lossy(&r);
    assert!(
        text.contains("\x06MS-EVTv=1 m=00000001 q=00000001"),
        "{text:?}"
    );
    assert!(
        r.windows(5).any(|w| w == [1, 4, b'a', b'@', b'b']),
        "CNAME length counts the NUL"
    );
    let parsed = parse_rtcp(&r).unwrap();
    assert!(parsed.contains(&Rtcp::Cname {
        ssrc: 7,
        cname: "a@b".into()
    }));
    assert!(parsed.contains(&Rtcp::MediaQuality {
        ssrc: 7,
        quality: q
    }));
    // Longer masks: only the last 8 digits count; unknown fields ignored; q ignored if m = 0.
    assert_eq!(
        Quality::parse(b"v=1 m=ff00000004 q=0000000004 x=1"),
        Some(Quality { known: 4, bad: 4 })
    );
    assert_eq!(
        Quality::parse(b"v=1 m=00000000 q=ffffffff"),
        Some(Quality { known: 0, bad: 0 })
    );
    assert_eq!(Quality::parse(b"v=2 m=00000001 q=00000001"), None);
    assert_eq!(Quality::parse(b"garbage"), None);
}

/// §2.2.3–2.2.7: probe (simple SR without blocks), padded pair packet, train packets of equal size.
#[test]
fn probe_pair_and_train() {
    let info = SenderInfo {
        ntp: 1,
        rtp_timestamp: 2,
        packets: 3,
        octets: 4,
    };
    let probe = probe(5, info);
    assert_eq!(probe.len(), 28);
    assert_eq!(&probe[..4], &[0x80, 200, 0, 6]);
    let pair = compound(5, Some(info), &[], "c");
    let padded = pad_report(&pair, 120);
    assert_eq!(padded.len(), 120);
    let parsed = parse_rtcp(&padded).unwrap();
    assert!(
        matches!(&parsed[0], Rtcp::SenderReport { extensions, .. } if extensions == &[Extension::Padding { words: (120 - pair.len() - 4) as u16 / 4 }])
    );
    assert!(parsed.contains(&Rtcp::Cname {
        ssrc: 5,
        cname: "c".into()
    }));
    let mut total = 0u16;
    for i in 0..5u8 {
        total += 120;
        let p = train_packet(5, i, 5, total, 120);
        assert_eq!(p.len(), 120);
        let Rtcp::ReceiverReport { extensions, .. } = &parse_rtcp(&p).unwrap()[0] else {
            panic!()
        };
        assert_eq!(
            extensions[0],
            Extension::PacketTrain {
                ssrc: 5,
                last: i == 4,
                index: i,
                count: 5,
                bytes: total
            }
        );
    }
}

/// RT-25 ([MS-RTP] §2.2.12.1, p. 27): standard PLI (12 bytes) and extended PLI with request ID
/// and sync-frame bits (SFR0 = priority IDs 0 … 7, its most significant bit ID 7).
#[test]
fn picture_loss_indication() {
    use feedback::*;
    let std = Pli {
        sender_ssrc: 0x11,
        media_ssrc: 0x22,
        extended: None,
    };
    let b = std.to_bytes().unwrap();
    assert_eq!(b, vec![0x81, 206, 0, 2, 0, 0, 0, 0x11, 0, 0, 0, 0x22]);
    assert_eq!(parse(&b).unwrap(), Some(Feedback::Pli(std)));
    let ext = Pli {
        extended: Some((7, 1 << 7 | 1 << 56)),
        ..std
    };
    let b = ext.to_bytes().unwrap();
    assert_eq!(b.len(), 24);
    assert_eq!(&b[2..4], &[0, 5]);
    assert_eq!(&b[12..16], &[0, 7, 0, 0]);
    assert_eq!(
        b[16], 0x80,
        "SFR0: priority ID 7 is the most significant bit"
    );
    assert_eq!(
        b[23], 0x01,
        "SFR7: priority ID 56 is the least significant bit"
    );
    assert_eq!(parse(&b).unwrap(), Some(Feedback::Pli(ext)));
}

fn vga_entry() -> feedback::VsrEntry {
    feedback::VsrEntry {
        payload_type: 122,
        ucconfig_mode: 1,
        flags: feedback::FLAG_CONSTRAINED_BASELINE,
        aspect_ratios: feedback::ASPECT_4_3 | feedback::ASPECT_16_9,
        max_width: 640,
        max_height: 480,
        min_bitrate: 100_000,
        bitrate_per_level: 50_000,
        bitrate_histogram: [0, 0, 0, 0, 0, 1, 0, 0, 0, 0],
        frame_rates: feedback::FPS_15 | feedback::FPS_30,
        must_instances: 1,
        may_instances: 0,
        quality_histogram: [0; 8],
        max_pixels: 640 * 480,
    }
}

/// RT-25 (§2.2.12.2, pp. 28–32): VSR header of 20 bytes, entries of 0x44 bytes, the length field
/// counting the FCI with AFB type and length; round trip; SOURCE_NONE without entries; inside
/// a compound packet after a receiver report.
#[test]
fn video_source_request() {
    use feedback::*;
    let vsr = Vsr {
        sender_ssrc: 0x0100_0001,
        media_ssrc: 0,
        msi: SOURCE_ANY,
        request_id: 3,
        key_frame: true,
        entries: vec![vga_entry()],
    };
    let b = vsr.to_bytes().unwrap();
    assert_eq!(b.len(), 12 + 20 + 68);
    assert_eq!(b[0], 0x80 | 15);
    assert_eq!(
        usize::from(u16::from_be_bytes([b[2], b[3]])),
        b.len() / 4 - 1
    );
    assert_eq!(&b[12..16], &[0, 1, 0, 88], "AFB type 1, length 20 + 68");
    assert_eq!(
        &b[24..28],
        &[0, 0x80, 1, 0x44],
        "version, key frame flag, 1 entry of 0x44"
    );
    assert_eq!(parse(&b).unwrap(), Some(Feedback::Vsr(vsr.clone())));
    let none = Vsr {
        msi: SOURCE_NONE,
        key_frame: false,
        entries: Vec::new(),
        ..vsr.clone()
    };
    assert_eq!(
        parse(&none.to_bytes().unwrap()).unwrap(),
        Some(Feedback::Vsr(none))
    );
    // In a compound packet after an RR.
    let mut compound = vec![0x80, 201, 0, 1, 0, 0, 0, 9];
    compound.extend_from_slice(&b);
    let parsed = parse_rtcp(&compound).unwrap();
    assert!(parsed
        .iter()
        .any(|r| matches!(r, Rtcp::Feedback(Feedback::Vsr(v)) if v.request_id == 3)));
    // Too many entries refused when building.
    let many = Vsr {
        entries: vec![vga_entry(); 21],
        ..vsr
    };
    assert!(many.to_bytes().is_err());
}

/// Hostile feedback packets: entry counts above 20, entries shorter than 0x44, a length beyond
/// the packet – all errors, never a panic.
#[test]
fn hostile_feedback() {
    use feedback::*;
    let good = Vsr {
        sender_ssrc: 1,
        media_ssrc: 2,
        msi: 5,
        request_id: 1,
        key_frame: false,
        entries: vec![vga_entry()],
    }
    .to_bytes()
    .unwrap();
    let mut too_many = good.clone();
    too_many[26] = 21;
    assert!(parse(&too_many).is_err());
    let mut short_entry = good.clone();
    short_entry[27] = 0x10;
    assert!(parse(&short_entry).is_err());
    let mut long = good.clone();
    long[14] = 0xff;
    assert!(parse(&long).is_err());
    assert!(parse(&good[..10]).is_err());
    let _ = parse_rtcp(&too_many);
}

/// RT-26 (§3.2.2, p. 41): sent at once, then 4 times every 190 ms; a VSR then 5 more times
/// every 3 s; a PLI stops after the fast phase.
#[test]
fn feedback_retransmission() {
    use feedback::Retransmit;
    let mut pli = Retransmit::new(1000, false);
    let mut times = Vec::new();
    for t in (1000..20_000).step_by(10) {
        if pli.due(t) {
            times.push(t);
        }
    }
    assert_eq!(times, vec![1000, 1190, 1380, 1570, 1760]);
    assert!(pli.finished());
    let mut vsr = Retransmit::new(0, true);
    let mut times = Vec::new();
    for t in (0..40_000).step_by(10) {
        if vsr.due(t) {
            times.push(t);
        }
    }
    assert_eq!(times.len(), 10);
    assert_eq!(times[4], 760);
    assert_eq!(&times[5..], &[3760, 6760, 9760, 12_760, 15_760]);
}
