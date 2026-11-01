use super::*;

/// The two layer descriptions of the MS-H264PF example §4.1 (p. 27).
fn example_layout() -> StreamLayout {
    StreamLayout::full(vec![
        LayerDescription {
            coded_width: 1280,
            coded_height: 720,
            display_width: 1280,
            display_height: 720,
            bitrate: 1_500_000,
            fps_idx: 2,
            layer_type: 0,
            prid: 56,
            constrained_baseline: false,
        },
        LayerDescription {
            coded_width: 1280,
            coded_height: 720,
            display_width: 1280,
            display_height: 720,
            bitrate: 1_000_000,
            fps_idx: 4,
            layer_type: 1,
            prid: 57,
            constrained_baseline: false,
        },
    ])
    .unwrap()
}

/// PL-03 / PL-07: the stream layout SEI of example §4.1 (p. 27), byte for byte, and back.
#[test]
fn stream_layout_example() {
    let expected: Vec<u8> = vec![
        0x06, 0x05, 0x3a, 0x13, 0x9f, 0xb1, 0xa9, 0x44, 0x6a, 0x4d, 0xec, 0x8c, 0xbf, 0x65, 0xb1,
        0xe1, 0x2d, 0x2c, 0xfd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x01, 0x10, //
        0x05, 0x00, 0x02, 0xd0, 0x05, 0x00, 0x02, 0xd0, 0x00, 0x16, 0xe3, 0x60, 0x10, 0xe0, 0x00,
        0x00, //
        0x05, 0x00, 0x02, 0xd0, 0x05, 0x00, 0x02, 0xd0, 0x00, 0x0f, 0x42, 0x40, 0x21, 0xe4, 0x00,
        0x00,
    ];
    let layout = example_layout();
    assert_eq!(layout.to_sei().unwrap(), expected);
    assert_eq!(StreamLayout::parse_sei(&expected).unwrap(), Some(layout));
    // Another UUID: not a stream layout.
    let mut other = expected.clone();
    if let Some(b) = other.get_mut(3) {
        *b = 0xbb;
    }
    assert_eq!(StreamLayout::parse_sei(&other).unwrap(), None);
    // A present bit without description, a short table, a truncated message.
    let mut lying = expected.clone();
    if let Some(b) = lying.get_mut(26) {
        *b = 0x07;
    }
    assert!(StreamLayout::parse_sei(&lying).is_err());
    assert!(StreamLayout::parse_sei(expected.get(..40).unwrap()).is_err());
    assert_eq!(fps_index(300), Some(4));
    assert_eq!(fps_index(240), None);
}

fn vga_layout() -> StreamLayout {
    StreamLayout::full(vec![LayerDescription {
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

fn annex_b(nals: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for n in nals {
        out.extend_from_slice(&[0, 0, 0, 1]);
        out.extend_from_slice(n);
    }
    out
}

fn feed(d: &mut Depacketizer, payloads: &[Payload], seq: u16, ts: u32) -> Vec<Output> {
    let mut out = Vec::new();
    for (i, p) in payloads.iter().enumerate() {
        out.extend(d.push(RtpIn {
            seq: seq.wrapping_add(i as u16),
            timestamp: ts,
            marker: p.marker,
            payload: &p.data,
        }));
    }
    out
}

/// Annex B splitting: 3- and 4-byte start codes, trailing zeros, bounds.
#[test]
fn annex_b_split() {
    let au = [
        0, 0, 1, 0x67, 1, 2, 0, 0, 0, 0, 1, 0x68, 3, 0, 0, 1, 0x65, 9,
    ];
    let nals = split_annex_b(&au).unwrap();
    assert_eq!(
        nals,
        vec![&[0x67, 1, 2][..], &[0x68, 3][..], &[0x65, 9][..]]
    );
    assert_eq!(split_annex_b(&[1, 2, 3]), Err(Error::Empty));
    assert_eq!(split_annex_b(&[]), Err(Error::Empty));
}

/// PL-02 / PL-08 / PL-09: an IDR access unit with SPS, PPS and a small slice goes into one
/// STAP-A led by a PACSI with the full layout; it comes back unchanged.
#[test]
fn small_idr_round_trip() {
    let sps: &[u8] = &[0x67, 0x42, 0xc0, 0x1e, 0xda];
    let pps: &[u8] = &[0x68, 0xce, 0x3c, 0x80];
    let idr: &[u8] = &[0x65, 0x88, 0x84, 0x00, 0x33];
    let au = annex_b(&[sps, pps, idr]);
    let mut p = Packetizer::new(1200, 0, vga_layout()).unwrap();
    let out = p.packetize(&au).unwrap();
    assert_eq!(out.len(), 1);
    let stap = &out.first().unwrap().data;
    assert_eq!(nal_type(stap), Some(NAL_STAP_A));
    assert!(out.first().unwrap().marker);
    // The PACSI comes first in the STAP-A, carrying the layout.
    let len = usize::from(u16::from_be_bytes([stap[1], stap[2]]));
    let pacsi = Pacsi::parse(stap.get(3..3 + len).unwrap()).unwrap();
    assert!(pacsi.idr && pacsi.end && !pacsi.start, "SPS leads: S = 0");
    assert_eq!(pacsi.nri, 3);
    assert_eq!(pacsi.donc, Some(0));
    assert_eq!(pacsi.layout, Some(vga_layout()));

    let mut d = Depacketizer::new();
    let got = feed(&mut d, &out, 100, 9000);
    assert_eq!(
        got,
        vec![Output::AccessUnit {
            timestamp: 9000,
            data: au.clone(),
            idr: true
        }]
    );
    // The next access unit (not IDR): PACSI without layout, DONC continues.
    let p_slice: &[u8] = &[0x41, 0x9a, 0x02];
    let out = p.packetize(&annex_b(&[p_slice])).unwrap();
    let stap = &out.first().unwrap().data;
    let len = usize::from(u16::from_be_bytes([stap[1], stap[2]]));
    let pacsi = Pacsi::parse(stap.get(3..3 + len).unwrap()).unwrap();
    assert_eq!(
        (pacsi.donc, pacsi.layout.is_none(), pacsi.start, pacsi.end),
        (Some(4), true, false, true)
    );
    let got = feed(&mut d, &out, 101, 12000);
    assert!(matches!(
        got.as_slice(),
        [Output::AccessUnit { idr: false, .. }]
    ));
}

/// [RFC6184] §5.8: a slice above the MTU is fragmented into FU-A units; the PACSI goes alone
/// first; all fragments come back as the original NAL unit; the marker is on the last packet.
#[test]
fn large_slice_fragmented() {
    let mut big = vec![0x65];
    big.extend((0..5000u32).map(|i| (i % 251) as u8 | 1));
    let au = annex_b(&[&big]);
    let mut p = Packetizer::new(1200, 0, vga_layout()).unwrap();
    let out = p.packetize(&au).unwrap();
    // PACSI and the prefix NAL unit share a STAP-A; the slice follows in FU-A fragments.
    assert_eq!(nal_type(&out.first().unwrap().data), Some(NAL_STAP_A));
    assert!(out
        .iter()
        .skip(1)
        .all(|p| nal_type(&p.data) == Some(NAL_FU_A)));
    assert!(out.iter().all(|p| p.data.len() <= 1200));
    assert_eq!(out.iter().filter(|p| p.marker).count(), 1);
    assert!(out.last().unwrap().marker);
    let mut d = Depacketizer::new();
    let got = feed(&mut d, &out, 65_530, 3000);
    assert_eq!(
        got,
        vec![Output::AccessUnit {
            timestamp: 3000,
            data: au,
            idr: true
        }],
        "sequence numbers wrap"
    );
}

/// PL-04 / §3.2.5.1: without a full layout first, packets are discarded; a missing packet loses
/// the access unit; a packet not led by a PACSI loses it; the next timestamp before the marker
/// reports the incomplete one as lost.
#[test]
fn discard_rules() {
    let mut big = vec![0x65];
    big.extend(std::iter::repeat_n(7u8, 3000));
    let mut p = Packetizer::new(1200, 0, vga_layout()).unwrap();
    let first = p.packetize(&annex_b(&[&big])).unwrap();
    let second = p.packetize(&annex_b(&[&[0x41, 1, 2]])).unwrap();

    // A receiver that missed the full layout drops the later access unit.
    let mut d = Depacketizer::new();
    assert_eq!(
        feed(&mut d, &second, 1, 6000),
        vec![Output::Lost { timestamp: 6000 }]
    );

    // A gap inside the first access unit.
    let mut d = Depacketizer::new();
    // Packet 3 of the access unit is missing (sequence numbers 1, 2, 4, …).
    let mut got = Vec::new();
    for (i, p) in first.iter().enumerate() {
        if i == 2 {
            continue;
        }
        got.extend(d.push(RtpIn {
            seq: 1 + i as u16,
            timestamp: 3000,
            marker: p.marker,
            payload: &p.data,
        }));
    }
    assert_eq!(got, vec![Output::Lost { timestamp: 3000 }]);

    // The first packet of a timestamp is no PACSI.
    let mut d = Depacketizer::new();
    let tail = first.get(1..).unwrap().to_vec();
    assert_eq!(
        feed(&mut d, &tail, 1, 3000),
        vec![Output::Lost { timestamp: 3000 }]
    );

    // A new timestamp before the marker.
    let mut d = Depacketizer::new();
    let unfinished = first.get(..2).unwrap();
    assert!(feed(&mut d, unfinished, 1, 3000).is_empty());
    let got = feed(&mut d, &second, 3, 6000);
    assert_eq!(got.first(), Some(&Output::Lost { timestamp: 3000 }));
}

/// Hostile payloads never panic and never yield an access unit: truncated STAP-A lengths,
/// forbidden packet types, FU-A without start, an end-start fragment, an empty payload.
#[test]
fn hostile_payloads() {
    let mut p = Packetizer::new(1200, 0, vga_layout()).unwrap();
    let good = p.packetize(&annex_b(&[&[0x65, 1, 2, 3]])).unwrap();
    let pacsi_stap = good.first().unwrap().data.clone();
    let cases: Vec<Vec<u8>> = vec![
        vec![],
        vec![NAL_STAP_A],
        vec![NAL_STAP_A, 0xff, 0xff, 0x65],
        vec![25, 0, 1],      // STAP-B
        vec![26, 0, 1],      // MTAP16
        vec![29, 0x85, 1],   // FU-B
        vec![0x7c, 0x05, 1], // FU-A middle without start
        vec![0x7c, 0xc5, 1], // start and end at once
        vec![0x1f],
    ];
    for bad in cases {
        let mut d = Depacketizer::new();
        // A valid PACSI-led start, then the bad packet with the marker.
        let mut got = d.push(RtpIn {
            seq: 10,
            timestamp: 1,
            marker: false,
            payload: &pacsi_stap,
        });
        got.extend(d.push(RtpIn {
            seq: 11,
            timestamp: 1,
            marker: true,
            payload: &bad,
        }));
        assert_eq!(got, vec![Output::Lost { timestamp: 1 }], "{bad:?}");
    }
    // PACSI with a truncated SEI length.
    assert!(Pacsi::parse(&[0x7e, 0x80, 0x80, 0x07, 0xa0, 0, 0, 0, 9, 6]).is_err());
}

/// Packetizer bounds: MTU, forbidden input types, a layout without the layer.
#[test]
fn packetizer_bounds() {
    assert_eq!(
        Packetizer::new(100, 0, vga_layout()).err(),
        Some(Error::BadMtu)
    );
    assert!(Packetizer::new(1200, 5, vga_layout()).is_err());
    let mut p = Packetizer::new(1200, 0, vga_layout()).unwrap();
    assert!(p.packetize(&annex_b(&[&[NAL_STAP_A, 0, 1]])).is_err());
    assert!(p.packetize(&[0, 0, 1]).is_err());
    assert!(StreamLayout::full(Vec::new()).is_err());
}

/// PL-13 ([MSFT-H264UCConfig] V1.1 §7.1, p. 11): a prefix NAL unit (type 14) with PRID, DID = QID
/// = TID = 0 and N = D = O = 1 goes before the first slice of every frame; the receiver drops it
/// before the decoder.
#[test]
fn prefix_nal_units() {
    assert_eq!(
        prefix_nal(&[0x65, 1], 0),
        vec![0x6e, 0xc0, 0x80, 0x0f, 0x20]
    );
    assert_eq!(
        prefix_nal(&[0x41, 1], 3),
        vec![0x4e, 0x83, 0x80, 0x0f, 0x20]
    );
    assert_eq!(
        prefix_nal(&[0x01, 1], 0),
        vec![0x0e, 0x80, 0x80, 0x0f],
        "nal_ref_idc 0: no body"
    );
    let mut p = Packetizer::new(1200, 0, vga_layout()).unwrap();
    let sps: &[u8] = &[0x67, 0x42, 0xc0, 0x1e];
    let slice: &[u8] = &[0x65, 0x88, 0x84];
    let out = p.packetize(&annex_b(&[sps, slice])).unwrap();
    let stap = &out[0].data;
    // STAP-A: PACSI, SPS, prefix, slice.
    let mut types = Vec::new();
    let mut rest = &stap[1..];
    while rest.len() >= 2 {
        let len = usize::from(u16::from_be_bytes([rest[0], rest[1]]));
        types.push(nal_type(&rest[2..2 + len]).unwrap());
        rest = &rest[2 + len..];
    }
    assert_eq!(types, vec![NAL_PACSI, 7, NAL_PREFIX, 5]);
    let mut d = Depacketizer::new();
    let got = feed(&mut d, &out, 1, 0);
    let [Output::AccessUnit { data, .. }] = got.as_slice() else {
        panic!("{got:?}")
    };
    assert_eq!(
        nal_types_of(data),
        vec![7, 5],
        "no prefix NAL unit for the decoder"
    );
    assert!(
        p.packetize(&annex_b(&[&[0x0e, 0x80, 0x80, 0x0f]])).is_err(),
        "no prefix in the input"
    );
}

fn nal_types_of(au: &[u8]) -> Vec<u8> {
    split_annex_b(au)
        .unwrap()
        .iter()
        .filter_map(|n| nal_type(n))
        .collect()
}
