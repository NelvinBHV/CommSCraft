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
use commscraft_media::{rate, VideoEvent};
use commscraft_rtp::feedback;
use commscraft_sdp as sdp;
use commscraft_video_ffmpeg as vf;

/// What we send: VGA at 30 frames per second, constrained baseline, one base layer.
pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 480;
pub const FPS: u32 = 30;
pub const BITRATE: u32 = 600_000;
/// The most we send: the maximum of VGA in the resolution table (BW-17).
pub const BITRATE_MAX: u32 = 800_000;

pub enum ToVideo {
    /// The negotiated video stream: start ICE, camera, decoder.
    Start {
        video: Box<sdp::video::NegotiatedVideo>,
        controlling: bool,
    },
    Hold(bool),
    /// In a meeting: the media source ID of the video to watch (CA-12, CA-13).
    Source(u32),
    /// Picture control through signalling (function 31, PL-50).
    Control(commscraft_session::media_control::VideoControl),
    Stop,
}

pub enum FromVideo {
    /// Gathering done: our video stream for the INVITE or the 200 OK.
    Ready(u32, Box<sdp::video::VideoLocal>),
    /// ICE selected the path: the newer description for the final offer.
    Selected(u32, Box<sdp::video::VideoLocal>),
    /// Something the user should know (no camera, no window); the call goes on.
    Notice(u32, String),
    /// The peer takes no PLI: ask it for a full picture through SIP INFO (PL-50).
    PictureUpdate(u32),
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

/// The sizes we can send (4:3 and 16:9, from the camera's VGA by scaling), largest first; all
/// are steps of the resolution table (BW-17, `rate::STEPS`).
const SIZES: [(u32, u32, u8); 9] = [
    (640, 480, feedback::ASPECT_4_3),
    (640, 360, feedback::ASPECT_16_9),
    (480, 270, feedback::ASPECT_16_9),
    (424, 320, feedback::ASPECT_4_3),
    (424, 240, feedback::ASPECT_16_9),
    (320, 240, feedback::ASPECT_4_3),
    (320, 180, feedback::ASPECT_16_9),
    (212, 160, feedback::ASPECT_4_3),
    (160, 120, feedback::ASPECT_4_3),
];

/// RT-25 and RT-30: what a video source request and the path allow – the highest of the
/// request's frame rates up to ours, a bit rate within its range and at most `path_bps`, and
/// the largest of our sizes within the request's width, height, pixel count and aspect ratios
/// whose minimum bit rate (BW-17) that bit rate reaches (else the smallest allowed one); the
/// bit rate then stays below the size's maximum. `None` if no entry asks for H.264 UC.
pub fn fit(entries: &[feedback::VsrEntry], path_bps: Option<u32>) -> Option<Target> {
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
    let allowed: Vec<(u32, u32)> = SIZES
        .iter()
        .copied()
        .filter(|(w, h, a)| {
            aspects & a != 0
                && *w <= u32::from(e.max_width.max(1))
                && *h <= u32::from(e.max_height.max(1))
                && w * h <= max_pixels
        })
        .map(|(w, h, _)| (w, h))
        .collect();
    let fps = [
        (feedback::FPS_30, 30),
        (feedback::FPS_15, 15),
        (feedback::FPS_7_5, 8),
    ]
    .iter()
    .find(|(bit, _)| e.frame_rates & bit != 0)
    .map_or(15, |(_, f)| *f)
    .min(FPS);
    // The lower edge of the histogram's highest requested level (element i counts
    // [min + (i − 1) × level, min + i × level], §2.2.12.2, p. 32): inside the range on any
    // reading of its upper end, which a mixer may treat as excluded (inference, O-RT-2).
    let top = e
        .bitrate_histogram
        .iter()
        .rposition(|n| *n > 0)
        .map_or(0, |i| i as u32);
    let bitrate = e
        .min_bitrate
        .saturating_add(e.bitrate_per_level.saturating_mul(top))
        .min(path_bps.unwrap_or(u32::MAX))
        .clamp(32_000, BITRATE_MAX);
    let step = |(w, h): (u32, u32)| rate::step_of(w, h);
    let (width, height) = allowed
        .iter()
        .copied()
        .find(|s| step(*s).is_some_and(|st| st.min_bps <= bitrate))
        .or_else(|| allowed.last().copied())?;
    let bitrate = step((width, height)).map_or(bitrate, |st| bitrate.min(st.max_bps));
    // BW-18: below 640x360 at most 15 fps – the highest of the request's lower rates.
    let fps = if fps > 15 && width * height < 640 * 360 {
        [(feedback::FPS_15, 15), (feedback::FPS_7_5, 8)]
            .iter()
            .find(|(bit, _)| e.frame_rates & bit != 0)
            .map_or(15, |(_, f)| *f)
    } else {
        fps
    };
    Some(Target {
        width,
        height,
        fps,
        bitrate,
    })
}

/// A new target from the request and the path: the encoder follows (camera thread), the stream
/// layout changes, the next picture is an IDR; the user hears of other sizes and rates.
fn retarget(
    media: &mut CallMedia,
    target: &Mutex<Target>,
    idr: &AtomicBool,
    events: &Sender<FromVideo>,
    id: u32,
    request: &[feedback::VsrEntry],
    path_bps: Option<u32>,
) {
    let Some(t) = fit(request, path_bps) else {
        return;
    };
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
            format!(
                "sending {}x{} at {} fps, {} kbit/s",
                t.width,
                t.height,
                t.fps,
                t.bitrate / 1000
            ),
        ));
    }
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
        // One receiver (us) wanting 500 … 600 kbit/s: element i = 5 (index 4) counts
        // [min + (i − 1) × level, min + i × level] ([MS-RTP] §2.2.12.2, p. 32; RT-25).
        bitrate_histogram: [0, 0, 0, 0, 1, 0, 0, 0, 0, 0],
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
    // An MCU asked us to stop sending video (picture_freeze, PL-50): RTCP goes on.
    let mut frozen = false;
    // The peer's last request (none yet: our own limits) and what the path carries (RT-30).
    let mut request: Vec<feedback::VsrEntry> = vec![wanted()];
    let mut path_bps: Option<u32> = None;
    loop {
        let now = now_ms();
        match rx.try_recv() {
            Ok(ToVideo::Stop) | Err(mpsc::TryRecvError::Disconnected) => break,
            Ok(ToVideo::Hold(h)) => media.set_hold(h),
            Ok(ToVideo::Control(control)) => {
                use commscraft_session::media_control::VideoControl;
                match control {
                    // XMLMC §3.1.5.1: stop sending RTP video, keep RTCP.
                    VideoControl::Freeze => {
                        if !frozen {
                            let _ = events.send(FromVideo::Notice(
                                id,
                                "the meeting paused your video".into(),
                            ));
                        }
                        frozen = true;
                    }
                    // XMLSMC §4: a full picture now; it also ends a freeze (XMLMC §1.3).
                    VideoControl::FastUpdate => {
                        if frozen {
                            let _ = events.send(FromVideo::Notice(
                                id,
                                "the meeting takes your video again".into(),
                            ));
                        }
                        frozen = false;
                        idr.store(true, Ordering::Relaxed);
                    }
                }
            }
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
            if !frozen {
                media.send_access_unit(&au, ts, now);
            }
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
                cm::Event::Video(VideoEvent::PictureUpdateNeeded) => {
                    let _ = events.send(FromVideo::PictureUpdate(id));
                }
                cm::Event::Video(VideoEvent::KeyFrameRequested) => {
                    idr.store(true, Ordering::Relaxed)
                }
                cm::Event::Video(VideoEvent::SendRate(bps)) => {
                    // RT-30: the path changed – the same request at another bit rate.
                    path_bps = Some(bps);
                    retarget(&mut media, &target, &idr, &events, id, &request, path_bps);
                }
                cm::Event::Video(VideoEvent::SourceRequested { msi, entries }) => {
                    // RT-25: send what the request allows (size, frame rate, bit rate).
                    if msi != feedback::SOURCE_NONE && !entries.is_empty() {
                        request = entries;
                        retarget(&mut media, &target, &idr, &events, id, &request, path_bps);
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
            // What is asked for already (a request may come before the camera is open).
            let first = target.lock().map(|t| *t).unwrap_or(START);
            let enc = encoder_for(first, hardware_allowed())?;
            Ok((cam, enc, first))
        });
        let (mut cam, enc, first) = match setup {
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
        let mut hardware = enc.backend() == vf::Backend::Vaapi;
        let mut enc = Some(enc);
        let mut current = first;
        let mut rebuilt_at = 0u64;
        let mut scaler = scaler_for(first);
        let mut next_due = 0u64;
        let mut camera_errors = 0u32;
        while !stop.load(Ordering::Relaxed) {
            let wanted = target.lock().map(|t| *t).unwrap_or(START);
            if needs_rebuild(current, wanted, now_ms().saturating_sub(rebuilt_at)) {
                // The old encoder (and its GPU session) goes before the new one is opened.
                drop(enc.take());
                let built = encoder_for(wanted, hardware).or_else(|e| {
                    if hardware {
                        hardware = false;
                        let _ = events.send(FromVideo::Notice(
                            id,
                            format!("GPU encoder failed ({e:?}), encoding on the CPU"),
                        ));
                        encoder_for(wanted, false)
                    } else {
                        Err(e)
                    }
                });
                match built {
                    Ok(e) => {
                        enc = Some(e);
                        scaler = scaler_for(wanted);
                        current = wanted;
                        rebuilt_at = now_ms();
                        idr.store(true, Ordering::Relaxed);
                    }
                    Err(e) => {
                        let _ =
                            events.send(FromVideo::Notice(id, format!("encoder stopped: {e:?}")));
                        return;
                    }
                }
            }
            let Some(enc) = enc.as_mut() else {
                return;
            };
            let picture = match cam.next_picture() {
                Ok(p) => {
                    camera_errors = 0;
                    p
                }
                // A single bad picture (e.g. a USB camera's truncated buffer) is skipped; only a
                // run of failures means the camera is gone.
                Err(e) => {
                    camera_errors += 1;
                    if camera_gone(camera_errors) {
                        let _ =
                            events.send(FromVideo::Notice(id, format!("camera stopped: {e:?}")));
                        return;
                    }
                    continue;
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
fn encoder_for(t: Target, hardware: bool) -> Result<vf::Encoder, vf::Error> {
    vf::Encoder::new(vf::EncoderConfig {
        width: t.width,
        height: t.height,
        fps: t.fps.max(5),
        bitrate: t.bitrate,
        idr_interval: 300,
        hardware,
    })
}

/// Failed camera reads in a row after which the camera counts as gone (design value: about a
/// second of pictures).
const CAMERA_ERRORS: u32 = 30;

fn camera_gone(errors_in_a_row: u32) -> bool {
    errors_in_a_row >= CAMERA_ERRORS
}

/// Camera VGA to the target size (none when it is VGA).
fn scaler_for(t: Target) -> Option<vf::Scaler> {
    if (t.width, t.height) == (WIDTH, HEIGHT) {
        None
    } else {
        vf::Scaler::new((WIDTH, HEIGHT), (t.width, t.height)).ok()
    }
}

/// GPU encoding unless `COMMSCRAFT_VIDEO_HW=0` asks for the CPU (D-13).
fn hardware_allowed() -> bool {
    std::env::var("COMMSCRAFT_VIDEO_HW").map_or(true, |v| v.trim() != "0")
}

/// Shortest time between two encoder rebuilds for a higher bit rate alone, and the changes that
/// are worth one (design values: a rebuild costs an IDR picture).
const REBUILD_MS: u64 = 2000;
const RAISE_PERCENT: u32 = 20;
const LOWER_PERCENT: u32 = 5;

/// Whether the encoder must be built again: another size or frame rate at once; a lower bit
/// rate at once from 5 % on – we never send more than was asked for (a mixer may drop a stream
/// above the request, inference); a higher one only from 20 % on and
/// when the last rebuild is old enough.
fn needs_rebuild(current: Target, wanted: Target, since_ms: u64) -> bool {
    if (current.width, current.height, current.fps) != (wanted.width, wanted.height, wanted.fps) {
        return true;
    }
    let diff = u64::from(current.bitrate.abs_diff(wanted.bitrate)) * 100;
    let base = u64::from(current.bitrate);
    if wanted.bitrate < current.bitrate {
        return diff >= base * u64::from(LOWER_PERCENT);
    }
    diff >= base * u64::from(RAISE_PERCENT) && since_ms >= REBUILD_MS
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

    fn size(t: Option<Target>) -> Option<(u32, u32, u32, u32)> {
        t.map(|t| (t.width, t.height, t.fps, t.bitrate))
    }

    /// RT-25 + RT-30 + BW-17/18: our own request without a path limit gives VGA at 500 kbit/s;
    /// below 640x360 at most 15 fps;
    /// a narrower path picks the largest size whose minimum bit rate it reaches.
    #[test]
    fn fit_follows_the_path() {
        let ours = [wanted()];
        // Our histogram asks for 500 … 600 kbit/s: the lower edge.
        assert_eq!(size(fit(&ours, None)), Some((640, 480, 30, 500_000)));
        assert_eq!(
            size(fit(&ours, Some(2_000_000))),
            Some((640, 480, 30, 500_000))
        );
        // An A/V MCU passing on that wish: 500 kbit/s, 10 kbit/s per level, first level.
        let passed_on = feedback::VsrEntry {
            min_bitrate: 500_000,
            bitrate_per_level: 10_000,
            bitrate_histogram: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ..wanted()
        };
        assert_eq!(size(fit(&[passed_on], None)), Some((640, 480, 30, 500_000)));
        assert_eq!(
            size(fit(&ours, Some(300_000))),
            Some((640, 480, 30, 300_000))
        );
        assert_eq!(
            size(fit(&ours, Some(250_000))),
            Some((480, 270, 15, 250_000))
        );
        assert_eq!(
            size(fit(&ours, Some(120_000))),
            Some((424, 240, 15, 120_000))
        );
        assert_eq!(size(fit(&ours, Some(40_000))), Some((320, 180, 15, 40_000)));
        assert_eq!(
            size(fit(&ours, Some(1))),
            Some((320, 180, 15, 32_000)),
            "floor"
        );
        // An A/V MCU's request: at most 61440 pixels, 15 fps, 40 kbit/s and nothing above.
        let mcu = feedback::VsrEntry {
            payload_type: 122,
            ucconfig_mode: 1,
            aspect_ratios: feedback::ASPECT_16_9,
            max_width: 640,
            max_height: 480,
            min_bitrate: 40_000,
            frame_rates: feedback::FPS_15,
            max_pixels: 61_440,
            ..Default::default()
        };
        assert_eq!(size(fit(&[mcu], None)), Some((320, 180, 15, 40_000)));
        // A wide request with a high histogram level stays below the size's maximum.
        let wide = feedback::VsrEntry {
            min_bitrate: 2_000_000,
            ..wanted()
        };
        assert_eq!(size(fit(&[wide], None)), Some((640, 480, 30, 800_000)));
        assert_eq!(size(fit(&[], None)), None);
    }

    /// A bad picture now and then does not end the video; a run of failures does.
    #[test]
    fn camera_errors_tolerated() {
        assert!(!camera_gone(1));
        assert!(!camera_gone(CAMERA_ERRORS - 1));
        assert!(camera_gone(CAMERA_ERRORS));
    }

    /// A new size or frame rate rebuilds at once, a lower bit rate from 5 % on; a higher one
    /// only from 20 % on and not within 2 s of the last rebuild.
    #[test]
    fn rebuild_hysteresis() {
        let t = |width, height, fps, bitrate| Target {
            width,
            height,
            fps,
            bitrate,
        };
        let cur = t(640, 480, 30, 600_000);
        assert!(needs_rebuild(cur, t(424, 240, 30, 600_000), 0));
        assert!(needs_rebuild(cur, t(640, 480, 15, 600_000), 0));
        assert!(!needs_rebuild(cur, cur, 10_000));
        // Lower: at once from 5 %.
        assert!(
            !needs_rebuild(cur, t(640, 480, 30, 580_000), 0),
            "under 5 %"
        );
        assert!(needs_rebuild(cur, t(640, 480, 30, 510_000), 0));
        // Higher: from 20 % and not within 2 s.
        assert!(
            !needs_rebuild(cur, t(640, 480, 30, 700_000), 10_000),
            "under 20 %"
        );
        assert!(
            !needs_rebuild(cur, t(640, 480, 30, 720_000), 1_000),
            "too soon"
        );
        assert!(needs_rebuild(cur, t(640, 480, 30, 720_000), 2_000));
    }
}
