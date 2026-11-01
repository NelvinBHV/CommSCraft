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
use std::sync::{Arc, Mutex};
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
    /// In a meeting: the media source ID of the video to watch (CA-12, CA-13).
    Source(u32),
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

/// What we encode: size, frame rate, bit rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate: u32,
}

const START: Target = Target {
    width: WIDTH,
    height: HEIGHT,
    fps: FPS,
    bitrate: BITRATE,
};

/// The sizes we can send (4:3 and 16:9, from the camera's VGA by scaling), largest first.
const SIZES: [(u32, u32, u8); 8] = [
    (640, 480, feedback::ASPECT_4_3),
    (640, 360, feedback::ASPECT_16_9),
    (480, 360, feedback::ASPECT_4_3),
    (424, 240, feedback::ASPECT_16_9),
    (320, 240, feedback::ASPECT_4_3),
    (320, 180, feedback::ASPECT_16_9),
    (256, 192, feedback::ASPECT_4_3),
    (160, 120, feedback::ASPECT_4_3),
];

/// RT-25: what a video source request allows – the largest of our sizes within its maximum
/// width, height and pixel count and one of its aspect ratios, the highest of its frame rates
/// up to ours, a bit rate within its range. `None` if no entry asks for H.264 UC.
pub fn fit(entries: &[feedback::VsrEntry]) -> Option<Target> {
    let e = entries
        .iter()
        .find(|e| e.payload_type == sdp::video::H264UC_PT)?;
    let aspects = if e.aspect_ratios == 0 {
        feedback::ASPECT_4_3 | feedback::ASPECT_16_9
    } else {
        e.aspect_ratios
    };
    let max_pixels = if e.max_pixels == 0 {
        u32::MAX
    } else {
        e.max_pixels
    };
    let (width, height, _) = SIZES.iter().copied().find(|(w, h, a)| {
        aspects & a != 0
            && *w <= u32::from(e.max_width.max(1))
            && *h <= u32::from(e.max_height.max(1))
            && w * h <= max_pixels
    })?;
    let fps = [
        (feedback::FPS_30, 30),
        (feedback::FPS_15, 15),
        (feedback::FPS_7_5, 8),
    ]
    .iter()
    .find(|(bit, _)| e.frame_rates & bit != 0)
    .map_or(15, |(_, f)| *f)
    .min(FPS);
    // The histogram's highest requested level, at least the minimum, at most ours.
    let top = e
        .bitrate_histogram
        .iter()
        .rposition(|n| *n > 0)
        .map_or(0, |i| i as u32 + 1);
    let bitrate = e
        .min_bitrate
        .saturating_add(e.bitrate_per_level.saturating_mul(top))
        .clamp(32_000, BITRATE);
    Some(Target {
        width,
        height,
        fps,
        bitrate,
    })
}

fn layout_for(t: Target) -> Option<uc::StreamLayout> {
    uc::StreamLayout::full(vec![uc::LayerDescription {
        coded_width: t.width as u16,
        coded_height: t.height as u16,
        display_width: t.width as u16,
        display_height: t.height as u16,
        bitrate: t.bitrate,
        fps_idx: uc::fps_index(if t.fps == 8 { 75 } else { t.fps * 10 })?,
        layer_type: 0,
        prid: 0,
        constrained_baseline: true,
    }])
    .ok()
}

fn layout() -> Option<uc::StreamLayout> {
    layout_for(START)
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
        // One receiver (us) wanting 500 … 600 kbit/s: bucket 5 of min + i × level (RT-25).
        bitrate_histogram: [0, 0, 0, 0, 0, 1, 0, 0, 0, 0],
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
    // RT-25: what the peer's video source request allows (VGA at 30 fps until one arrives).
    let target = Arc::new(Mutex::new(START));
    let (au_tx, au_rx) = mpsc::sync_channel::<(Vec<u8>, u32)>(4);
    let (dec_tx, dec_rx) = mpsc::sync_channel::<Vec<u8>>(8);
    spawn_camera(
        camera,
        au_tx,
        stop.clone(),
        idr.clone(),
        target.clone(),
        events.clone(),
        id,
    );
    spawn_window(dec_rx, stop.clone(), events.clone(), id);
    let mut final_sent = false;
    let mut requested = false;
    // The source to ask for: any (1:1) until a meeting names one (CA-13).
    let mut source = feedback::SOURCE_ANY;
    let mut watched_by_peer: Option<bool> = None;
    loop {
        let now = now_ms();
        match rx.try_recv() {
            Ok(ToVideo::Stop) | Err(mpsc::TryRecvError::Disconnected) => break,
            Ok(ToVideo::Hold(h)) => media.set_hold(h),
            Ok(ToVideo::Source(msi)) => {
                if msi != source {
                    source = msi;
                    if requested {
                        media.request_video(source, wanted(), now);
                    }
                }
            }
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
                        media.request_video(source, wanted(), now);
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
                cm::Event::Video(VideoEvent::SourceRequested { msi, entries }) => {
                    // RT-25: send what the request allows (size, frame rate, bit rate).
                    if msi != feedback::SOURCE_NONE {
                        if let Some(t) = fit(&entries) {
                            let changed = target.lock().map(|mut cur| {
                                let changed = *cur != t;
                                *cur = t;
                                changed
                            });
                            if changed.unwrap_or(false) {
                                if let Some(l) = layout_for(t) {
                                    media.set_video_layout(l);
                                }
                                idr.store(true, Ordering::Relaxed);
                                let _ = events.send(FromVideo::Notice(
                                    id,
                                    format!("sending {}x{} at {} fps", t.width, t.height, t.fps),
                                ));
                            }
                        }
                    }
                    // A VSR is repeated (RT-26): tell the user only when it changes.
                    let watched = msi != feedback::SOURCE_NONE;
                    if watched_by_peer != Some(watched) {
                        watched_by_peer = Some(watched);
                        let _ = events.send(FromVideo::Notice(
                            id,
                            if watched {
                                "your video is watched".into()
                            } else {
                                "nobody watches your video now".into()
                            },
                        ));
                    }
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
    target: Arc<Mutex<Target>>,
    events: Sender<FromVideo>,
    id: u32,
) {
    std::thread::spawn(move || {
        let setup = vf::init().and_then(|()| {
            let cam = vf::Camera::open(&device, WIDTH, HEIGHT, FPS)?;
            let enc = encoder_for(START)?;
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
        let mut current = START;
        let mut scaler: Option<vf::Scaler> = None;
        let mut next_due = 0u64;
        while !stop.load(Ordering::Relaxed) {
            let wanted = target.lock().map(|t| *t).unwrap_or(START);
            if wanted != current {
                match encoder_for(wanted) {
                    Ok(e) => {
                        enc = e;
                        scaler = if (wanted.width, wanted.height) == (WIDTH, HEIGHT) {
                            None
                        } else {
                            vf::Scaler::new((WIDTH, HEIGHT), (wanted.width, wanted.height)).ok()
                        };
                        current = wanted;
                        idr.store(true, Ordering::Relaxed);
                    }
                    Err(e) => {
                        let _ = events.send(FromVideo::Notice(id, format!("encoder: {e:?}")));
                    }
                }
            }
            let picture = match cam.next_picture() {
                Ok(p) => p,
                Err(e) => {
                    let _ = events.send(FromVideo::Notice(id, format!("camera stopped: {e:?}")));
                    return;
                }
            };
            // The requested frame rate: drop camera pictures in between.
            let now = now_ms();
            if now < next_due {
                continue;
            }
            next_due = now + 1000 / u64::from(current.fps.max(1)) - 5;
            let picture = match scaler.as_mut() {
                Some(s) => match s.scale(&picture) {
                    Ok(p) => p,
                    Err(_) => continue,
                },
                None => picture,
            };
            let key = idr.swap(false, Ordering::Relaxed);
            let ts = (now as u32).wrapping_mul(90);
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

/// An encoder for a target (GPU first, D-13).
fn encoder_for(t: Target) -> Result<vf::Encoder, vf::Error> {
    vf::Encoder::new(vf::EncoderConfig {
        width: t.width,
        height: t.height,
        fps: t.fps.max(5),
        bitrate: t.bitrate,
        idr_interval: 300,
        hardware: true,
    })
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
