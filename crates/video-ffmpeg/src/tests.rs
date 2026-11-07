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

/// PL-03: libx264 in constrained baseline – the first access unit is an IDR with SPS and
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

/// With `hardware` the GPU encodes if a VA-API device works (skipped otherwise); the
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

/// NV12 to I420: the chroma planes are separated, the strides of the frame do not leak in.
#[test]
fn nv12_to_i420() {
    let mut f = ff::frame::Video::new(ff::format::Pixel::NV12, 16, 16);
    let stride = f.stride(0);
    for (i, b) in f.data_mut(0).iter_mut().enumerate() {
        *b = if i % stride < 16 { 10 } else { 99 };
    }
    let stride = f.stride(1);
    for (i, b) in f.data_mut(1).iter_mut().enumerate() {
        *b = match (i % stride < 16, i % 2) {
            (false, _) => 99,
            (true, 0) => 20,
            (true, _) => 30,
        };
    }
    let p = from_nv12(&f).unwrap();
    assert_eq!(p.data.len(), Picture::i420_len(16, 16));
    assert!(p.data[..256].iter().all(|b| *b == 10));
    assert!(p.data[256..320].iter().all(|b| *b == 20));
    assert!(p.data[320..384].iter().all(|b| *b == 30));
    let other = ff::frame::Video::new(ff::format::Pixel::YUV420P, 16, 16);
    assert!(from_nv12(&other).is_err());
}

/// Decoding on the GPU where a device exists gives the same pictures as on the CPU, also at a
/// height that is cropped from the coded size; without a device the CPU decodes.
#[test]
fn hardware_decoder_when_available() {
    init().unwrap();
    let mut enc = Encoder::new(EncoderConfig {
        width: 640,
        height: 360,
        ..config()
    })
    .unwrap();
    let mut gpu = Decoder::with_hardware(true).unwrap();
    let mut cpu = Decoder::new().unwrap();
    let (mut compared, mut used_gpu) = (0, false);
    let mut from_gpu = std::collections::VecDeque::new();
    let mut from_cpu = std::collections::VecDeque::new();
    for n in 0..30 {
        for au in enc.encode(&gradient(640, 360, n), n == 0).unwrap() {
            from_gpu.extend(gpu.decode(&au).unwrap());
            used_gpu |= gpu.backend() == Backend::Vaapi;
            from_cpu.extend(cpu.decode(&au).unwrap());
        }
        while let (Some(g), Some(c)) = (from_gpu.front(), from_cpu.front()) {
            assert_eq!((g.width, g.height), (640, 360));
            assert!(mean_abs_diff(g, c) < 2.0, "{}", mean_abs_diff(g, c));
            compared += 1;
            from_gpu.pop_front();
            from_cpu.pop_front();
        }
    }
    eprintln!("decoded on the GPU: {used_gpu}, compared {compared}");
    assert!(compared >= 10);
}

/// A stream of failures moves a GPU decoder to the CPU for good, and it still decodes.
#[test]
fn hardware_decoder_gives_up_on_bad_input() {
    init().unwrap();
    let mut dec = Decoder::with_hardware(true).unwrap();
    let junk = [0, 0, 0, 1, 0x65, 0xff, 0xff, 0xff];
    dec.errors = MAX_GPU_ERRORS - 1;
    dec.resizes = MAX_GPU_RESIZES;
    let _ = dec.decode(&junk);
    assert!(!dec.gpu);
    assert_eq!(dec.backend(), Backend::Software);
    let mut enc = Encoder::new(config()).unwrap();
    let mut decoded = 0;
    for n in 0..10 {
        for au in enc.encode(&gradient(320, 240, n), n == 0).unwrap() {
            decoded += dec.decode(&au).unwrap().len();
        }
    }
    assert!(decoded > 0);
}

/// The CPU time of this process so far, in clock ticks (user and system).
fn cpu_ticks() -> u64 {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
    let after = stat.rsplit(')').next().unwrap();
    let f: Vec<&str> = after.split_whitespace().collect();
    f[11].parse::<u64>().unwrap() + f[12].parse::<u64>().unwrap()
}

/// CPU and wall time of decoding 720p with fine detail, all of it on this side of the test (not a
/// regular test; `cargo test --release decode_load -- --ignored --nocapture`, COMMSCRAFT_HW=1 or 0).
#[test]
#[ignore]
fn decode_load() {
    init().unwrap();
    let hardware = std::env::var("COMMSCRAFT_HW").is_ok_and(|v| v == "1");
    let big = std::env::var("COMMSCRAFT_1080").is_ok();
    let (w, h) = if big { (1920u32, 1080u32) } else { (1280, 720) };
    let mut enc = Encoder::new(EncoderConfig {
        width: w,
        height: h,
        bitrate: if big { 6_000_000 } else { 3_000_000 },
        ..config()
    })
    .unwrap();
    // Texture that moves: much harder to code than the gradient.
    let mut seed = 1u32;
    let texture: Vec<u8> = (0..(w * h * 2))
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 24) as u8
        })
        .collect();
    let mut units = Vec::new();
    for n in 0..120u32 {
        let mut p = Picture::filled(w, h, 128, 128, 128);
        let shift = (n * 4) as usize;
        let (y, _) = p.data.split_at_mut((w * h) as usize);
        y.copy_from_slice(&texture[shift..shift + (w * h) as usize]);
        units.extend(enc.encode(&p, n == 0).unwrap());
    }
    let mut dec = Decoder::with_hardware(hardware).unwrap();
    let (cpu, wall) = (cpu_ticks(), std::time::Instant::now());
    let mut pictures = 0;
    for _ in 0..3 {
        for au in &units {
            pictures += dec.decode(au).unwrap().len();
        }
    }
    eprintln!(
        "decoder backend {:?}: {pictures} pictures, {:?}, {} CPU ticks",
        dec.backend(),
        wall.elapsed(),
        cpu_ticks() - cpu
    );
}

/// Where the time of GPU decoding goes: decoding alone, then download, then the conversion (not a
/// regular test).
#[test]
#[ignore]
fn decode_phases() {
    init().unwrap();
    let big = std::env::var("COMMSCRAFT_1080").is_ok();
    let (w, h) = if big { (1920u32, 1080u32) } else { (1280, 720) };
    let mut enc = Encoder::new(EncoderConfig {
        width: w,
        height: h,
        bitrate: if big { 6_000_000 } else { 3_000_000 },
        ..config()
    })
    .unwrap();
    let mut seed = 1u32;
    let texture: Vec<u8> = (0..(w * h * 2))
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 24) as u8
        })
        .collect();
    let mut units = Vec::new();
    for n in 0..120u32 {
        let mut p = Picture::filled(w, h, 128, 128, 128);
        let shift = (n * 4) as usize;
        let (y, _) = p.data.split_at_mut((w * h) as usize);
        y.copy_from_slice(&texture[shift..shift + (w * h) as usize]);
        units.extend(enc.encode(&p, n == 0).unwrap());
    }
    for phase in ["decode only", "decode+download", "decode+download+convert"] {
        let mut dec = open_decoder(true).unwrap();
        let (cpu, wall) = (cpu_ticks(), std::time::Instant::now());
        let (mut n, mut hw) = (0, 0);
        let mut frame = ff::frame::Video::empty();
        for _ in 0..3 {
            for au in &units {
                dec.send_packet(&ff::Packet::copy(au)).unwrap();
                while dec.receive_frame(&mut frame).is_ok() {
                    n += 1;
                    if frame.format() != ff::format::Pixel::VAAPI {
                        continue;
                    }
                    hw += 1;
                    if phase != "decode only" {
                        let sw = hw::download(&frame).unwrap();
                        if phase == "decode+download+convert" {
                            from_nv12(&sw).unwrap();
                        }
                    }
                }
            }
        }
        eprintln!(
            "{phase}: {n} pictures ({hw} on the GPU), {:?}, {} CPU ticks",
            wall.elapsed(),
            cpu_ticks() - cpu
        );
    }
}

/// A GPU encoder changing size and bit rate next to a GPU decoder of another stream, as in a
/// meeting. Not a regular test: in one process (`COMMSCRAFT_STRESS_ROLE` unset) this hangs the
/// video engine of an integrated AMD GPU with VCN 4 within seconds. With `encoder` and `decoder` in
/// two processes it does not: that is why the decoder has a process of its own.
#[test]
#[ignore]
fn gpu_encoder_changing_next_to_a_decoder() {
    use std::sync::mpsc;
    init().unwrap();
    let var = |n: &str| std::env::var(n).unwrap_or_default();
    let seconds: u64 = var("COMMSCRAFT_SECONDS").parse().unwrap_or(30);
    let role = var("COMMSCRAFT_STRESS_ROLE");
    let (dec_gpu, enc_gpu) = (role != "encoder", role != "decoder");
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(8);
    let decoder = std::thread::spawn(move || {
        let mut dec = Decoder::with_hardware(dec_gpu).unwrap();
        let mut held = std::collections::VecDeque::new();
        let (mut pictures, mut gpu) = (0u64, 0u64);
        while let Ok(au) = rx.recv() {
            for d in dec.decode_to_surfaces(&au).unwrap_or_default() {
                pictures += 1;
                gpu += u64::from(dec.backend() == Backend::Vaapi);
                // As the window: a few pictures stay in use.
                held.push_back(d);
                if held.len() > 3 {
                    held.pop_front();
                }
            }
        }
        (pictures, gpu)
    });
    let start = std::time::Instant::now();
    let sizes = [(320, 180), (640, 480), (640, 360), (320, 240)];
    let mut stream = Encoder::new(EncoderConfig {
        width: 320,
        height: 180,
        ..config()
    })
    .unwrap();
    let (mut changes, mut frame, mut on_gpu) = (0u32, 0u32, 0u32);
    while start.elapsed().as_secs() < seconds {
        let (w, h) = sizes[changes as usize % sizes.len()];
        changes += 1;
        let mut enc = Encoder::new(EncoderConfig {
            width: w,
            height: h,
            bitrate: 300_000 + changes % 50 * 7_000,
            hardware: enc_gpu,
            ..config()
        })
        .unwrap();
        on_gpu += u32::from(enc.backend() == Backend::Vaapi);
        let picture = gradient(w, h, frame);
        for n in 0..12 {
            frame += 1;
            enc.encode(&picture, n == 0).unwrap();
            for au in stream
                .encode(&gradient(320, 180, frame), frame == 1)
                .unwrap()
            {
                tx.send(au).unwrap();
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
    drop(tx);
    let (pictures, gpu) = decoder.join().unwrap();
    eprintln!("changes {changes} ({on_gpu} on the GPU), pictures {pictures} ({gpu} on the GPU)");
}

/// The window (only with COMMSCRAFT_DISPLAY=1; not in CI): a moving gradient for one second;
/// a picture of another size is refused.
#[test]
fn display_shows_pictures() {
    if std::env::var("COMMSCRAFT_DISPLAY").ok().as_deref() != Some("1") {
        return;
    }
    init().unwrap();
    let mut d = Display::open("CommSCraft test\u{1b}[2J", 320, 240).unwrap();
    for n in 0..30 {
        d.show(&gradient(320, 240, n)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(33));
    }
    assert!(d.show(&gradient(160, 120, 0)).is_err());
}
