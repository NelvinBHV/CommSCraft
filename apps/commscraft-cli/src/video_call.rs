//! The video of a call in the interactive mode (function 28): one media thread per video stream
//! (gathering, ICE, SRTP, H.264 UC payload, feedback) with a camera/encoder thread and a
//! decoder/window thread (decisions D-12, D-13).
//!
//! Security by design: the camera is opened only once the video stream is negotiated and closed
//! when it ends; every queue between threads is bounded (pictures and access units are dropped,
//! never queued without limit); the window shows only decoded pictures of the negotiated stream;
//! a missing camera, encoder or window does not end the call – the user is told.

use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::time::Duration;

use commscraft_call_media::{self as cm, CallMedia};
use commscraft_h264uc as uc;
use commscraft_media::VideoEvent;
use commscraft_rtp::feedback;
use commscraft_sdp as sdp;
use commscraft_video_ffmpeg as vf;

/// What we send: VGA at 30 frames per second, constrained baseline, one base layer.
pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 480;
pub const FPS: u32 = 30;
pub const BITRATE: u32 = 600_000;

pub enum ToVideo {
    /// The negotiated video stream: start ICE, camera, decoder.
    Start {
        video: Box<sdp::video::NegotiatedVideo>,
        controlling: bool,
    },
    Hold(bool),
    Stop,
}

pub enum FromVideo {
    /// Gathering done: our video stream for the INVITE or the 200 OK.
    Ready(u32, Box<sdp::video::VideoLocal>),
    /// ICE selected the path: the newer description for the final offer.
    Selected(u32, Box<sdp::video::VideoLocal>),
    /// Something the user should know (no camera, no window); the call goes on.
    Notice(u32, String),
    Failed(u32, String),
}

pub struct VideoHandle {
    pub tx: Sender<ToVideo>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Our SSRC range: 100 values from a random start (SD-64), never 0, within the allowed maximum.
fn ssrc_range() -> (u32, u32) {
    let mut r = [0u8; 4];
    cm::os_random(&mut r);
    let start = 1 + u32::from_be_bytes(r) % (sdp::video::SSRC_RANGE_MAX - 200);
    (start, start + sdp::video::VIDEO_SSRC_RANGE - 1)
}

fn layout() -> Option<uc::StreamLayout> {
    uc::StreamLayout::full(vec![uc::LayerDescription {
        coded_width: WIDTH as u16,
        coded_height: HEIGHT as u16,
        display_width: WIDTH as u16,
        display_height: HEIGHT as u16,
        bitrate: BITRATE,
        fps_idx: uc::fps_index(FPS * 10)?,
        layer_type: 0,
        prid: 0,
        constrained_baseline: true,
    }])
    .ok()
}

/// What we ask the peer for (VSR entry, RT-25): H.264 UC up to VGA at 15 or 30 fps.
fn wanted() -> feedback::VsrEntry {
    feedback::VsrEntry {
        payload_type: sdp::video::H264UC_PT,
        ucconfig_mode: 1,
        flags: feedback::FLAG_CONSTRAINED_BASELINE,
        aspect_ratios: feedback::ASPECT_4_3 | feedback::ASPECT_16_9,
        max_width: WIDTH as u16,
        max_height: HEIGHT as u16,
        min_bitrate: 100_000,
        bitrate_per_level: 100_000,
        frame_rates: feedback::FPS_15 | feedback::FPS_30,
        must_instances: 1,
        max_pixels: WIDTH * HEIGHT,
        ..Default::default()
    }
}

pub fn spawn(
    id: u32,
    local_ip: IpAddr,
    relay: Option<cm::Relay>,
    camera: String,
    events: Sender<FromVideo>,
) -> VideoHandle {
    let (tx, rx) = mpsc::channel::<ToVideo>();
    std::thread::spawn(move || run(id, local_ip, relay, camera, rx, events));
    VideoHandle { tx }
}

fn run(
    id: u32,
    local_ip: IpAddr,
    relay: Option<cm::Relay>,
    camera: String,
    rx: Receiver<ToVideo>,
    events: Sender<FromVideo>,
) {
    let t0 = now_ms();
    let (mut media, first) = match CallMedia::prepare_video(local_ip, relay.as_ref(), t0) {
        Ok(x) => x,
        Err(e) => {
            let _ = events.send(FromVideo::Failed(id, format!("video sockets: {e}")));
            return;
        }
    };
    drop(relay);
    media.send_initial(first);
    while !media.gathered(now_ms()) {
        media.poll(now_ms());
        std::thread::sleep(Duration::from_millis(5));
    }
    let range = ssrc_range();
    let local = |m: &CallMedia, d: sdp::Local| sdp::video::VideoLocal {
        media: sdp::Local {
            key: Some(m.key().clone()),
            ..d
        },
        ssrc_range: range,
    };
    let _ = events.send(FromVideo::Ready(
        id,
        Box::new(local(&media, media.local_description())),
    ));
    let (negotiated, controlling) = loop {
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(ToVideo::Start { video, controlling }) => break (video, controlling),
            Ok(ToVideo::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                media.stop(now_ms());
                return;
            }
            Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {
                media.poll(now_ms());
            }
        }
    };
    let Some(layout) = layout() else {
        let _ = events.send(FromVideo::Failed(id, "stream layout".into()));
        return;
    };
    if let Err(e) = media.start_video(
        &negotiated,
        range,
        layout,
        controlling,
        "commscraft@linux",
        now_ms(),
    ) {
        let _ = events.send(FromVideo::Failed(id, e));
        return;
    }
    let stop = Arc::new(AtomicBool::new(false));
    let idr = Arc::new(AtomicBool::new(true));
    let (au_tx, au_rx) = mpsc::sync_channel::<(Vec<u8>, u32)>(4);
    let (dec_tx, dec_rx) = mpsc::sync_channel::<Vec<u8>>(8);
    spawn_camera(camera, au_tx, stop.clone(), idr.clone(), events.clone(), id);
    spawn_window(dec_rx, stop.clone(), events.clone(), id);
    let mut final_sent = false;
    let mut requested = false;
    loop {
        let now = now_ms();
        match rx.try_recv() {
            Ok(ToVideo::Stop) | Err(mpsc::TryRecvError::Disconnected) => break,
            Ok(ToVideo::Hold(h)) => media.set_hold(h),
            // A re-INVITE answer with the same stream: nothing to rebuild here.
            Ok(ToVideo::Start { .. }) | Err(mpsc::TryRecvError::Empty) => {}
        }
        while let Ok((au, ts)) = au_rx.try_recv() {
            media.send_access_unit(&au, ts, now);
        }
        for e in media.poll(now) {
            match e {
                cm::Event::Selected => {
                    if !requested {
                        // RT-25: ask the peer for its video (a 1:1 call: any source).
                        requested = true;
                        media.request_video(feedback::SOURCE_ANY, wanted(), now);
                    }
                    if controlling && !final_sent {
                        final_sent = true;
                        if let Some(d) = media.final_description() {
                            let _ =
                                events.send(FromVideo::Selected(id, Box::new(local(&media, d))));
                        }
                    }
                }
                cm::Event::Video(VideoEvent::AccessUnit { data, .. }) => {
                    // Bounded: a decoder that falls behind loses access units (the next
                    // key frame request repairs the picture).
                    if dec_tx.try_send(data).is_err() {
                        media.request_key_frame(now);
                    }
                }
                cm::Event::Video(VideoEvent::KeyFrameRequested) => {
                    idr.store(true, Ordering::Relaxed)
                }
                cm::Event::Video(VideoEvent::SourceRequested { msi, .. })
                    if msi == feedback::SOURCE_NONE =>
                {
                    let _ = events.send(FromVideo::Notice(
                        id,
                        "the other side wants no video now".into(),
                    ));
                }
                cm::Event::Video(VideoEvent::Bye) => {
                    let _ = events.send(FromVideo::Notice(
                        id,
                        "the other side ended its video".into(),
                    ));
                }
                cm::Event::Failed(why) => {
                    let _ = events.send(FromVideo::Failed(id, why));
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(3));
    }
    stop.store(true, Ordering::Relaxed);
    media.stop(now_ms());
}

/// Camera → encoder (GPU if available, D-13) → access units for the media thread.
fn spawn_camera(
    device: String,
    out: SyncSender<(Vec<u8>, u32)>,
    stop: Arc<AtomicBool>,
    idr: Arc<AtomicBool>,
    events: Sender<FromVideo>,
    id: u32,
) {
    std::thread::spawn(move || {
        let setup = vf::init().and_then(|()| {
            let cam = vf::Camera::open(&device, WIDTH, HEIGHT, FPS)?;
            let enc = vf::Encoder::new(vf::EncoderConfig {
                width: WIDTH,
                height: HEIGHT,
                fps: FPS,
                bitrate: BITRATE,
                idr_interval: 300,
                hardware: true,
            })?;
            Ok((cam, enc))
        });
        let (mut cam, mut enc) = match setup {
            Ok(x) => x,
            Err(e) => {
                let _ = events.send(FromVideo::Notice(id, format!("no camera picture: {e:?}")));
                return;
            }
        };
        let _ = events.send(FromVideo::Notice(
            id,
            format!(
                "camera on, encoding on the {}",
                match enc.backend() {
                    vf::Backend::Vaapi => "GPU (VA-API)",
                    vf::Backend::Software => "CPU (libx264)",
                }
            ),
        ));
        while !stop.load(Ordering::Relaxed) {
            let picture = match cam.next_picture() {
                Ok(p) => p,
                Err(e) => {
                    let _ = events.send(FromVideo::Notice(id, format!("camera stopped: {e:?}")));
                    return;
                }
            };
            let key = idr.swap(false, Ordering::Relaxed);
            let ts = (now_ms() as u32).wrapping_mul(90);
            match enc.encode(&picture, key) {
                Ok(units) => {
                    for au in units {
                        // Bounded: a slow network drops pictures here, not memory.
                        let _ = out.try_send((au, ts));
                    }
                }
                Err(e) => {
                    let _ = events.send(FromVideo::Notice(id, format!("encoder stopped: {e:?}")));
                    return;
                }
            }
        }
    });
}

/// Decoder → window (XVideo, D-12); the window opens with the first picture's size.
fn spawn_window(
    input: Receiver<Vec<u8>>,
    stop: Arc<AtomicBool>,
    events: Sender<FromVideo>,
    id: u32,
) {
    std::thread::spawn(move || {
        let mut decoder = match vf::init().and_then(|()| vf::Decoder::new()) {
            Ok(d) => d,
            Err(e) => {
                let _ = events.send(FromVideo::Notice(id, format!("no video decoder: {e:?}")));
                return;
            }
        };
        let mut window: Option<vf::Display> = None;
        let mut failed = false;
        while !stop.load(Ordering::Relaxed) {
            let Ok(au) = input.recv_timeout(Duration::from_millis(100)) else {
                continue;
            };
            let Ok(pictures) = decoder.decode(&au) else {
                continue;
            };
            for p in pictures {
                if window.as_ref().is_none() && !failed {
                    match vf::Display::open(&format!("CommSCraft – call {id}"), p.width, p.height)
                    {
                        Ok(w) => {
                            window = Some(w);
                            let _ = events.send(FromVideo::Notice(
                                id,
                                format!("receiving video {}x{}", p.width, p.height),
                            ));
                        }
                        Err(e) => {
                            failed = true;
                            let _ = events
                                .send(FromVideo::Notice(id, format!("no video window: {e:?}")));
                        }
                    }
                }
                if let Some(w) = window.as_mut() {
                    if w.show(&p).is_err() {
                        // Another size: a new window.
                        window = vf::Display::open(
                            &format!("CommSCraft – call {id}"),
                            p.width,
                            p.height,
                        )
                        .ok();
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SD-64: 100 SSRCs, never 0, within the allowed maximum.
    #[test]
    fn ssrc_ranges() {
        for _ in 0..1000 {
            let (a, b) = ssrc_range();
            assert!(a >= 1 && b - a == 99 && b <= sdp::video::SSRC_RANGE_MAX);
        }
    }

    /// PL-03 / RT-25: what we send and ask for – VGA, 30 fps, constrained baseline, UCConfig 1.
    #[test]
    fn layout_and_request() {
        let l = layout().unwrap();
        let d = l.layer(0).unwrap();
        assert_eq!((d.coded_width, d.coded_height, d.fps_idx), (640, 480, 4));
        assert!(d.constrained_baseline);
        let w = wanted();
        assert_eq!((w.payload_type, w.ucconfig_mode), (122, 1));
        assert!(w.flags & feedback::FLAG_CONSTRAINED_BASELINE != 0);
        assert!(w.frame_rates & feedback::FPS_30 != 0);
    }
}
