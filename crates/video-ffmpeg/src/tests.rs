use super::*;
use commscraft_h264uc as uc;

/// A picture with a moving gradient (frame `n`), so the encoder has something to code.
fn gradient(width: u32, height: u32, n: u32) -> Picture {
    let (w, h) = (width as usize, height as usize);
    let mut data = Vec::with_capacity(Picture::i420_len(width, height));
    for y in 0..h {
        for x in 0..w {
            data.push(((x + y + n as usize * 4) % 220 + 16) as u8);
        }
    }
    data.extend(std::iter::repeat_n(128u8, 2 * (w / 2) * (h / 2)));
    Picture {
        width,
        height,
        data,
    }
}

fn mean_abs_diff(a: &Picture, b: &Picture) -> f64 {
    let n = (a.width * a.height) as usize;
    let sum: u64 = a.data[..n]
        .iter()
        .zip(&b.data[..n])
        .map(|(x, y)| u64::from(x.abs_diff(*y)))
        .sum();
    sum as f64 / n as f64
}

fn config() -> EncoderConfig {
    EncoderConfig {
        width: 320,
        height: 240,
        fps: 30,
        bitrate: 400_000,
        idr_interval: 300,
        hardware: false,
    }
}

fn nal_types(au: &[u8]) -> Vec<u8> {
    uc::split_annex_b(au)
        .unwrap()
        .iter()
        .filter_map(|n| uc::nal_type(n))
        .collect()
}

/// I420 pictures survive the copy into an FFmpeg frame (with stride) and back.
#[test]
fn picture_frame_round_trip() {
    init().unwrap();
    let p = gradient(64, 48, 3);
    let f = to_frame(&p).unwrap();
    assert_eq!(from_frame(&f).unwrap(), p);
    assert!(to_frame(&Picture {
        width: 64,
        height: 48,
        data: vec![0; 10]
    })
    .is_err());
    assert!(to_frame(&Picture::filled(15, 48, 0, 0, 0)).is_err());
    assert!(to_frame(&Picture::filled(4096, 48, 0, 0, 0)).is_err());
}

/// D-12 / PL-03: libx264 in constrained baseline – the first access unit is an IDR with SPS and
/// PPS (Annex B), later ones are not, until an IDR is requested; decoding gives back pictures
/// close to the originals.
#[test]
fn encode_decode_round_trip() {
    init().unwrap();
    let mut enc = Encoder::new(config()).unwrap();
    let mut dec = Decoder::new().unwrap();
    let mut decoded = Vec::new();
    let mut aus = Vec::new();
    for n in 0..12 {
        let pic = gradient(320, 240, n);
        for au in enc.encode(&pic, n == 8).unwrap() {
            for d in dec.decode(&au).unwrap() {
                decoded.push((pic.clone(), d));
            }
            aus.push(au);
        }
    }
    assert!(
        aus.len() >= 10,
        "zero latency: about one access unit per picture"
    );
    let first = nal_types(aus.first().unwrap());
    assert!(
        first.contains(&7) && first.contains(&8) && first.contains(&5),
        "{first:?}"
    );
    // SPS profile_idc 66 (baseline) with constraint_set1 (constrained baseline).
    let sps = uc::split_annex_b(aus.first().unwrap())
        .unwrap()
        .into_iter()
        .find(|n| uc::nal_type(n) == Some(7))
        .unwrap()
        .to_vec();
    assert_eq!(sps[1], 66);
    assert!(sps[2] & 0x40 != 0, "constraint_set1_flag");
    let idrs = aus.iter().filter(|a| nal_types(a).contains(&5)).count();
    assert_eq!(idrs, 2, "the first picture and the requested one");
    assert!(
        aus.iter().any(|a| nal_types(a).contains(&1)),
        "P slices between"
    );
    assert!(!decoded.is_empty());
    for (orig, got) in &decoded {
        assert_eq!((got.width, got.height), (320, 240));
        assert!(mean_abs_diff(orig, got) < 6.0);
    }
}

/// The whole send/receive path without network: encoder → h264uc packetizer → depacketizer →
/// decoder.
#[test]
fn through_the_payload_format() {
    init().unwrap();
    let mut enc = Encoder::new(config()).unwrap();
    let layout = uc::StreamLayout::full(vec![uc::LayerDescription {
        coded_width: 320,
        coded_height: 240,
        display_width: 320,
        display_height: 240,
        bitrate: 400_000,
        fps_idx: 4,
        layer_type: 0,
        prid: 0,
        constrained_baseline: true,
    }])
    .unwrap();
    let mut pk = uc::Packetizer::new(1200, 0, layout).unwrap();
    let mut dp = uc::Depacketizer::new();
    let mut dec = Decoder::new().unwrap();
    let (mut seq, mut pictures) = (7u16, 0);
    for n in 0..6u32 {
        for au in enc.encode(&gradient(320, 240, n), false).unwrap() {
            for p in pk.packetize(&au).unwrap() {
                for out in dp.push(uc::RtpIn {
                    seq,
                    timestamp: n * 3000,
                    marker: p.marker,
                    payload: &p.data,
                }) {
                    match out {
                        uc::Output::AccessUnit { data, .. } => {
                            assert_eq!(
                                uc::split_annex_b(&data).unwrap(),
                                uc::split_annex_b(&au).unwrap(),
                                "the NAL units come back unchanged"
                            );
                            pictures += dec.decode(&data).unwrap().len();
                        }
                        uc::Output::Lost { .. } => panic!("nothing is lost here"),
                    }
                }
                seq = seq.wrapping_add(1);
            }
        }
    }
    assert!(pictures >= 5);
}

/// Hostile decoder input: garbage and truncated access units give an error or nothing, never a
/// panic; oversized input is refused before FFmpeg.
#[test]
fn decoder_hostile_input() {
    init().unwrap();
    let mut dec = Decoder::new().unwrap();
    for junk in [
        vec![0u8; 0],
        vec![0, 0, 0, 1, 0x65, 0xff, 0xff, 0xff],
        vec![0, 0, 1, 0x67, 0x42],
        (0..5000u32).map(|i| (i * 31 % 256) as u8).collect(),
    ] {
        let _ = dec.decode(&junk);
    }
    assert_eq!(
        dec.decode(&vec![0; MAX_ACCESS_UNIT + 1]),
        Err(Error::TooLarge)
    );
}

/// Encoder parameter bounds and a picture of the wrong size.
#[test]
fn encoder_bounds() {
    init().unwrap();
    let bad = |f: fn(&mut EncoderConfig)| {
        let mut c = config();
        f(&mut c);
        Encoder::new(c).is_err()
    };
    assert!(bad(|c| c.fps = 0));
    assert!(bad(|c| c.bitrate = 10));
    assert!(bad(|c| c.width = 4000));
    assert!(bad(|c| c.idr_interval = 0));
    let mut enc = Encoder::new(config()).unwrap();
    assert!(enc.encode(&gradient(160, 120, 0), false).is_err());
    assert!(Camera::open("/etc/passwd", 640, 480, 30).is_err());
}

/// The camera of the machine (only with COMMSCRAFT_CAMERA=/dev/videoN; not in CI).
#[test]
fn camera_delivers_pictures() {
    let Ok(device) = std::env::var("COMMSCRAFT_CAMERA") else {
        return;
    };
    init().unwrap();
    let mut cam = Camera::open(&device, 640, 480, 30).unwrap();
    let mut enc = Encoder::new(EncoderConfig {
        width: 640,
        height: 480,
        ..config()
    })
    .unwrap();
    for _ in 0..10 {
        let p = cam.next_picture().unwrap();
        assert_eq!(p.data.len(), Picture::i420_len(640, 480));
        enc.encode(&p, false).unwrap();
    }
}

/// D-13: with `hardware` the GPU encodes if a VA-API device works (skipped otherwise); the
/// result is the same kind of stream – constrained baseline, IDR first and on request, SPS/PPS
/// with every IDR – and decodes like the software one.
#[test]
fn vaapi_encoder_when_available() {
    init().unwrap();
    let mut enc = Encoder::new(EncoderConfig {
        hardware: true,
        ..config()
    })
    .unwrap();
    eprintln!("encoder backend: {:?}", enc.backend());
    if enc.backend() == Backend::Software {
        return; // no VA-API here (CI): the fallback is what the user gets
    }
    let mut dec = Decoder::new().unwrap();
    let mut aus = Vec::new();
    let mut decoded = 0;
    for n in 0..12 {
        for au in enc.encode(&gradient(320, 240, n), n == 8).unwrap() {
            decoded += dec.decode(&au).unwrap().len();
            aus.push(au);
        }
    }
    assert!(aus.len() >= 10);
    let first = nal_types(aus.first().unwrap());
    assert!(
        first.contains(&7) && first.contains(&8) && first.contains(&5),
        "{first:?}"
    );
    let sps = uc::split_annex_b(aus.first().unwrap())
        .unwrap()
        .into_iter()
        .find(|n| uc::nal_type(n) == Some(7))
        .unwrap()
        .to_vec();
    assert_eq!(sps[1], 66, "baseline profile_idc");
    assert!(sps[2] & 0x40 != 0, "constraint_set1_flag");
    let idrs: Vec<usize> = aus
        .iter()
        .enumerate()
        .filter(|(_, a)| nal_types(a).contains(&5))
        .map(|(i, _)| i)
        .collect();
    assert!(idrs.len() >= 2, "IDR first and on request: {idrs:?}");
    for i in &idrs {
        assert!(nal_types(&aus[*i]).contains(&7), "SPS with every IDR");
    }
    assert!(decoded >= 10);
}

/// CPU load of the two paths (manual: `cargo test -p commscraft-video-ffmpeg encode_load --
/// --ignored` with COMMSCRAFT_HW=1 or 0, timed from outside).
#[test]
#[ignore]
fn encode_load() {
    init().unwrap();
    let hardware = std::env::var("COMMSCRAFT_HW").is_ok_and(|v| v == "1");
    let mut enc = Encoder::new(EncoderConfig {
        width: 1280,
        height: 720,
        bitrate: 1_500_000,
        hardware,
        ..config()
    })
    .unwrap();
    eprintln!("encoder backend: {:?}", enc.backend());
    let pictures: Vec<Picture> = (0..30).map(|n| gradient(1280, 720, n)).collect();
    for n in 0..300 {
        enc.encode(&pictures[n % 30], false).unwrap();
    }
}
