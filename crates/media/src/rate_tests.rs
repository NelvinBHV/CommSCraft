use super::rate::*;
use commscraft_rtp as rtp;

const CAP: u32 = 1_500_000;

/// Feeds one second of 50 packets of 1000 octets (400 kbit/s) from `seq`/`t0`; `late_ms` adds
/// a growing queueing delay per packet, `drop_every` loses every n-th packet.
fn second(e: &mut Estimator, seq: &mut u16, t0: u64, late_ms: u64, drop_every: u16) {
    for i in 0..50u64 {
        let s = *seq;
        *seq = seq.wrapping_add(1);
        if drop_every > 0 && s.is_multiple_of(drop_every) {
            continue;
        }
        let media_ms = t0 + i * 20;
        let ts = (media_ms as u32).wrapping_mul(90);
        e.on_packet(1000, s, ts, 90_000, media_ms + late_ms * i);
    }
}

/// RT-28: before any measurement the estimate is our capacity with a low confidence, in the
/// 16-octet form about the given SSRC; the capacity never lies below the floor.
#[test]
fn starts_at_capacity() {
    let e = Estimator::new(CAP);
    assert_eq!(
        e.extension(7),
        rtp::Extension::Bandwidth {
            ssrc: 7,
            bps: CAP,
            confidence: Some(2)
        }
    );
    assert_eq!(Estimator::new(10).capacity(), FLOOR_BPS);
}

/// A clean path: the estimate stays at the capacity, the state "uncongested" is reported once.
#[test]
fn clean_path() {
    let mut e = Estimator::new(CAP);
    let mut seq = 65_500; // wraps inside the test
    assert_eq!(e.update(0), None, "the first call starts an interval");
    for s in 0..5u64 {
        second(&mut e, &mut seq, s * 1000, 0, 0);
        let iv = e.update((s + 1) * 1000).unwrap();
        assert_eq!(iv.fraction_lost, 0, "no loss across the wrap");
        assert!(iv.queueing_ms.abs() < 30, "{iv:?}");
        assert!((390_000..=430_000).contains(&iv.received_bps), "{iv:?}");
    }
    assert_eq!(e.estimate(), CAP);
    assert!(e.confidence() >= 6);
    assert_eq!(
        e.take_congestion(1),
        Some(rtp::Extension::Congestion {
            ntp: 1,
            info: UNCONGESTED_DELAY | UNCONGESTED_LOSS
        })
    );
    assert_eq!(e.take_congestion(2), None, "only changes are reported");
}

/// Loss of 20 %: congested by loss, the estimate falls below what got through, confidence high;
/// clean seconds afterwards let it grow back by 5 % per interval up to the capacity.
#[test]
fn loss_then_recovery() {
    let mut e = Estimator::new(CAP);
    let mut seq = 0;
    e.update(0);
    // The stream's first interval is not judged (warm-up) …
    second(&mut e, &mut seq, 0, 0, 5);
    let iv = e.update(1000).unwrap();
    assert!(iv.fraction_lost >= 40, "{iv:?}");
    assert!(!e.congested(), "warm-up");
    // … the next one is.
    second(&mut e, &mut seq, 1000, 0, 5);
    let iv = e.update(2000).unwrap();
    assert!(iv.fraction_lost >= 40, "{iv:?}");
    assert!(e.congested());
    assert!(e.congestion_changed());
    let low = e.estimate();
    assert!(
        low <= iv.received_bps / 20 * 17 && low >= FLOOR_BPS,
        "{low}"
    );
    assert_eq!(e.confidence(), 12);
    assert_eq!(
        e.take_congestion(0),
        Some(rtp::Extension::Congestion {
            ntp: 0,
            info: UNCONGESTED_DELAY | CONGESTED_LOSS
        })
    );
    second(&mut e, &mut seq, 2000, 0, 0);
    let iv = e.update(3000).unwrap();
    // At least what the clean path carried, with 20 % headroom.
    assert_eq!(
        e.estimate(),
        (low + low / 20).max(iv.received_bps + iv.received_bps / 5)
    );
    assert!(!e.congested());
    for s in 3..200u64 {
        second(&mut e, &mut seq, s * 1000, 0, 0);
        e.update((s + 1) * 1000);
    }
    assert_eq!(e.estimate(), CAP, "never above the capacity");
}

/// A queue that builds up (each packet 5 ms later than its media time): congested by delay –
/// but only when it lasts two intervals; a single burst is no congestion.
#[test]
fn queueing_delay() {
    let mut e = Estimator::new(CAP);
    let mut seq = 0;
    e.update(0);
    second(&mut e, &mut seq, 0, 0, 0);
    e.update(1000);
    second(&mut e, &mut seq, 1000, 5, 0);
    let iv = e.update(2000).unwrap();
    assert!(iv.queueing_ms >= 100, "{iv:?}");
    assert!(!e.congested(), "one interval is a burst");
    let est = e.estimate();
    // The queue stays (packets arrive 250 ms late throughout).
    for i in 0..50u64 {
        let s = seq;
        seq = seq.wrapping_add(1);
        let media_ms = 2000 + i * 20;
        e.on_packet(
            1000,
            s,
            (media_ms as u32).wrapping_mul(90),
            90_000,
            media_ms + 250,
        );
    }
    e.update(3000);
    assert!(e.congested());
    assert!(e.estimate() < est);
    let bits = match e.take_congestion(0) {
        Some(rtp::Extension::Congestion { info, .. }) => info,
        other => panic!("{other:?}"),
    };
    assert_eq!(bits, CONGESTED_DELAY | UNCONGESTED_LOSS);
}

/// Hostile and odd input: huge sizes, clocks that jump back or to the end, timestamps that
/// wrap – no panic, the estimate stays within floor and capacity.
#[test]
fn hostile_input() {
    let mut e = Estimator::new(CAP);
    e.update(0);
    e.on_packet(usize::MAX, 1, u32::MAX, 90_000, u64::MAX);
    e.on_packet(usize::MAX, 0, 0, 0, 0);
    e.on_packet(1, 40_000, 7, 1, 5);
    e.update(u64::MAX);
    e.update(u64::MAX);
    assert!((FLOOR_BPS..=CAP).contains(&e.estimate()));
    let mut seq = 0;
    for s in 0..3u64 {
        second(&mut e, &mut seq, s * 1000, 0, 0);
    }
}

/// RT-30: the target is the smallest of what the peer reports for our SSRC (estimate, limits),
/// special values and other SSRCs are no estimate, a congestion notification takes 30 % off
/// and the target recovers by 5 % per second.
#[test]
fn send_rate() {
    let mut r = SendRate::default();
    assert_eq!(r.target(0), None, "nothing reported yet");
    let est = |ssrc, bps| rtp::Extension::Bandwidth {
        ssrc,
        bps,
        confidence: None,
    };
    assert!(
        !r.on_extension(&est(9, 900_000), 5, 0),
        "about another SSRC"
    );
    assert!(!r.on_extension(&est(5, rtp::BANDWIDTH_PACKET_PAIR), 5, 0));
    assert!(!r.on_extension(&est(5, rtp::BANDWIDTH_SEND_TRAIN), 5, 0));
    assert!(!r.on_extension(&est(5, 0), 5, 0));
    assert_eq!(r.target(0), None);
    assert!(r.on_extension(&est(5, 2_000_000), 5, 0));
    assert_eq!(r.target(0), Some(2_000_000));
    r.on_extension(&rtp::Extension::ReceiverLimit(1_000_000), 5, 0);
    r.on_extension(
        &rtp::Extension::ModalityLimit {
            modality: 1,
            bps: 100_000,
        },
        5,
        0,
    );
    assert_eq!(r.target(0), Some(1_000_000), "modality 1 is not video");
    r.on_extension(&rtp::Extension::PolicyBandwidth(800_000), 5, 0);
    assert_eq!(r.target(0), Some(800_000));
    r.on_extension(
        &rtp::Extension::Congestion {
            ntp: 0,
            info: CONGESTED_LOSS,
        },
        5,
        10_000,
    );
    assert_eq!(r.target(10_000), Some(560_000));
    assert_eq!(r.target(12_000), Some(640_000), "two seconds: +10 %");
    assert_eq!(r.target(100_000), Some(800_000), "fully recovered");
    for _ in 0..20 {
        r.on_extension(
            &rtp::Extension::Congestion {
                ntp: 0,
                info: CONGESTED_DELAY,
            },
            5,
            200_000,
        );
    }
    assert_eq!(
        r.target(200_000),
        Some(200_000),
        "back-off stops at a quarter"
    );
    r.on_extension(&rtp::Extension::TurnBandwidth(10), 5, 0);
    assert_eq!(r.target(200_000), Some(FLOOR_BPS));
}

/// BW-17: the resolution table, largest first, every step's minimum below its maximum.
#[test]
fn resolution_steps() {
    assert_eq!(
        step_of(640, 480).map(|s| (s.min_bps, s.max_bps)),
        Some((300_000, 800_000))
    );
    assert_eq!(
        step_of(320, 180).map(|s| (s.min_bps, s.max_bps)),
        Some((15_000, 250_000))
    );
    assert_eq!(step_of(641, 480), None);
    assert!(STEPS.windows(2).all(|w| w[0].min_bps >= w[1].min_bps));
    assert!(STEPS.iter().all(|s| s.min_bps < s.max_bps));
}

/// A mixer switching the source jumps in the sequence numbers: a new start, no loss.
#[test]
fn sequence_jump_is_no_loss() {
    let mut e = Estimator::new(CAP);
    let mut seq = 100;
    e.update(0);
    second(&mut e, &mut seq, 0, 0, 0);
    let mut seq = 40_000;
    second(&mut e, &mut seq, 1000, 0, 0);
    let iv = e.update(2000).unwrap();
    assert_eq!(iv.fraction_lost, 0, "{iv:?}");
    assert!(!e.congested());
    // A real gap (10 of 50 lost) is still loss.
    let mut seq = 40_050;
    second(&mut e, &mut seq, 2000, 0, 5);
    assert!(e.update(3000).unwrap().fraction_lost >= 40);
}
