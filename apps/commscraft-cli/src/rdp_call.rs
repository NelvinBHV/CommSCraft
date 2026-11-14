//! Viewing a window shared over RDP (function 43): the data stream of application sharing
//! (`CallMedia::prepare_data`, MS-RTASPF over ICE-TCP), the RDP viewer (`commscraft-rdp-view`)
//! and the window (the video window's helper process in its raw mode).
//!
//! The RDP bytes are decoded here, in memory-safe Rust with panics of the decoder caught by the
//! adapter; the pictures go to the sandboxed window process, scaled down to what it shows.

use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use commscraft_call_media::{self as cm, CallMedia};
use commscraft_rdp_view as rv;
use commscraft_sdp as sdp;

use crate::video_call::FromVideo;

/// Longest wait for the sharer's first bytes once the path is up (design).
const START_TIMEOUT_MS: u64 = 20_000;
/// Most pictures per second sent to the window (design: window contents change slowly).
const MAX_FPS: u64 = 15;

pub enum ToRdp {
    /// The answered RDP stream: start ICE and the viewer.
    Start {
        rdp: Box<sdp::screen::NegotiatedRdp>,
        controlling: bool,
    },
    Stop,
}

pub enum FromRdp {
    /// Our data stream for the 200 OK.
    Ready(u32, Box<sdp::Local>),
    Notice(u32, String),
    /// The viewing ended (the reason).
    Ended(u32, String),
}

pub struct RdpHandle {
    pub tx: Sender<ToRdp>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

pub fn spawn(
    id: u32,
    local_ip: IpAddr,
    sip_uri: String,
    events: Sender<FromRdp>,
    window_events: Sender<FromVideo>,
) -> RdpHandle {
    let (tx, rx) = mpsc::channel::<ToRdp>();
    std::thread::spawn(move || {
        let why = run(id, local_ip, sip_uri, &rx, &events, window_events);
        let _ = events.send(FromRdp::Ended(id, why));
    });
    RdpHandle { tx }
}

fn run(
    id: u32,
    local_ip: IpAddr,
    sip_uri: String,
    rx: &Receiver<ToRdp>,
    events: &Sender<FromRdp>,
    window_events: Sender<FromVideo>,
) -> String {
    let mut media = match CallMedia::prepare_data(local_ip, now_ms()) {
        Ok(m) => m,
        Err(e) => return format!("data stream: {e}"),
    };
    let local = sdp::Local {
        key: Some(media.key().clone()),
        ..media.local_description()
    };
    let _ = events.send(FromRdp::Ready(id, Box::new(local)));
    let (rdp, controlling) = loop {
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(ToRdp::Start { rdp, controlling }) => break (rdp, controlling),
            Ok(ToRdp::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                media.stop(now_ms());
                return "stopped".into();
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                media.poll(now_ms());
            }
        }
    };
    if let Err(e) = media.start_data(
        rdp.remote_key.as_ref(),
        &rdp.remote_ice,
        rdp.remote_rtp,
        controlling,
        now_ms(),
    ) {
        return format!("data stream: {e}");
    }
    media.set_data_abs_send_time(rdp.abs_send_time);
    // The Windows clients' CNAME is their SIP URI (`tested-in-lab`).
    media.set_data_cname(&sip_uri);
    let (mut viewer, first) = match rv::Viewer::new(rv::Config {
        width: 1920,
        height: 1080,
        client_name: "commscraft".into(),
        sip_uri,
    }) {
        Ok(v) => v,
        Err(e) => return format!("RDP: {e:?}"),
    };
    let mut first = Some(first);
    let started = now_ms();
    let stop = Arc::new(AtomicBool::new(false));
    let (pictures, window_rx) = mpsc::sync_channel::<Vec<u8>>(2);
    let mut window = Some((window_rx, window_events));
    let mut dirty = false;
    let mut last_picture = 0u64;
    let mut selected = false;
    let mut final_answer = false;
    let why = 'session: loop {
        match rx.try_recv() {
            Ok(ToRdp::Stop) | Err(mpsc::TryRecvError::Disconnected) => break "stopped".into(),
            // The sharer's final offer (re-INVITE after ICE) was answered.
            Ok(ToRdp::Start { .. }) => final_answer = true,
            Err(mpsc::TryRecvError::Empty) => {}
        }
        let now = now_ms();
        for e in media.poll(now) {
            match e {
                cm::Event::Data(bytes) => match viewer.on_bytes(&bytes) {
                    Ok(out) => {
                        for o in out {
                            match o {
                                rv::Event::Send(b) => media.send_data(&b, now),
                                rv::Event::Active { width, height } => {
                                    let _ = events.send(FromRdp::Notice(
                                        id,
                                        format!("viewing the shared window ({width}x{height})"),
                                    ));
                                    if let Some((rx, ev)) = window.take() {
                                        crate::video_window::spawn_raw(rx, stop.clone(), ev, id);
                                    }
                                }
                                rv::Event::Updated { .. } => dirty = true,
                                rv::Event::Ended(why) => break 'session format!("ended ({why})"),
                            }
                        }
                    }
                    Err(e) => break 'session format!("RDP: {e:?}"),
                },
                cm::Event::Failed(why) => break 'session format!("data path: {why}"),
                cm::Event::PeerBye => break 'session "the sharer stopped".into(),
                cm::Event::Selected => selected = true,
                cm::Event::Dtmf(_) | cm::Event::Video(_) => {}
            }
        }
        if media.data_broken() {
            break "data lost (MS-RTASPF: the stream ends at a loss)".into();
        }
        // The first RDP bytes once ICE selected the path, the peer's first RTCP came in (as the
        // Windows viewer does it, `tested-in-lab`) and the final offer was answered (design).
        if first.is_some() && selected && media.data_peer_heard() && final_answer {
            if let Some(b) = first.take() {
                media.send_data(&b, now);
            }
        }
        if !viewer.active() && now.saturating_sub(started) > START_TIMEOUT_MS {
            break "the shared window did not start".into();
        }
        if dirty && now.saturating_sub(last_picture) >= 1000 / MAX_FPS {
            if let Some((w, h, px)) = viewer
                .image()
                .and_then(|(w, h, rgba)| crate::video_window::scaled_bgrx(w, h, rgba))
            {
                // A full queue: this picture is dropped, a newer one follows.
                let _ = pictures.try_send(crate::video_window::raw_unit(w, h, &px));
            }
            dirty = false;
            last_picture = now;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    stop.store(true, Ordering::Relaxed);
    media.stop(now_ms());
    why
}
