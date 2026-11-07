//! GPU pictures on the screen, without copying (not in CI: needs a GPU and a compositor).
//! `cargo test --release -p commscraft-video-wayland -- --ignored --nocapture`

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::os::fd::AsFd;
use std::os::unix::fs::MetadataExt;
use std::rc::Rc;

use commscraft_video_ffmpeg as vf;
use commscraft_video_wayland::{Frame, Plane, Window};

fn cpu_ticks() -> u64 {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
    let after = stat.rsplit(')').next().unwrap();
    let f: Vec<&str> = after.split_whitespace().collect();
    f[11].parse::<u64>().unwrap() + f[12].parse::<u64>().unwrap()
}

#[test]
#[ignore]
fn gpu_pictures_on_the_screen() {
    vf::init().unwrap();
    let (w, h) = (1280u32, 720u32);
    let mut enc = vf::Encoder::new(vf::EncoderConfig {
        width: w,
        height: h,
        fps: 30,
        bitrate: 3_000_000,
        idr_interval: 300,
        hardware: false,
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
    for n in 0..150u32 {
        let mut p = vf::Picture::filled(w, h, 128, 128, 128);
        let shift = (n * 4) as usize;
        let (y, _) = p.data.split_at_mut((w * h) as usize);
        y.copy_from_slice(&texture[shift..shift + (w * h) as usize]);
        units.extend(enc.encode(&p, n == 0).unwrap());
    }
    let mut dec = vf::Decoder::with_hardware(true).unwrap();
    let mut window = Window::open("CommSCraft test").unwrap();
    let (cpu, wall) = (cpu_ticks(), std::time::Instant::now());
    let (mut shown, mut skipped) = (0, 0);
    for au in &units {
        for d in dec.decode_to_surfaces(au).unwrap() {
            let vf::Decoded::Surface(s) = d else {
                panic!("a picture in memory");
            };
            let s = Rc::new(s);
            let key = std::fs::File::from(s.planes[0].fd.try_clone().unwrap())
                .metadata()
                .unwrap()
                .ino();
            let planes: Vec<Plane<'_>> = s
                .planes
                .iter()
                .map(|p| Plane {
                    fd: p.fd.as_fd(),
                    offset: p.offset,
                    stride: p.stride,
                })
                .collect();
            let frame = Frame {
                width: s.width,
                height: s.height,
                fourcc: s.fourcc,
                modifier: s.modifier,
                planes: &planes,
                key,
            };
            if window.show(&frame, Box::new(Rc::clone(&s))).unwrap() {
                shown += 1;
            } else {
                skipped += 1;
            }
            std::thread::sleep(std::time::Duration::from_millis(33));
        }
    }
    eprintln!(
        "shown {shown}, skipped {skipped}, {:?}, {} CPU ticks",
        wall.elapsed(),
        cpu_ticks() - cpu
    );
    assert!(shown > 100);
}
