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
    let sr = a.rtcp_due(0, 1 << 32).unwrap();
    assert_eq!(classify(&sr), Kind::Rtcp);
    assert!(a.rtcp_due(4999, 0).is_none(), "5 s interval");
    assert!(!b.on_rtcp(&sr));
    assert!(b.on_rtcp(&a.bye().unwrap()), "BYE of the peer");
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
