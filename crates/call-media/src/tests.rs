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
