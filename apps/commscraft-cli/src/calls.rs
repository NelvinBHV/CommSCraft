//! Calls in the interactive mode: one media thread per call (gathering, ICE, SRTP, codec) with
//! two device threads (ALSA capture and playback, decision D-11). The SIP side stays in the
//! engine; this module only moves media and reports to the main loop.
//!
//! Security by design: devices are opened only for an answered call and closed when it ends;
//! every queue between threads is bounded (frames are dropped, never buffered without limit);
//! the relay password is handed to the media thread once and zeroised there.

use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::time::Duration;

use commscraft_audio_alsa as alsa;
use commscraft_call_media::{self as cm, CallMedia};
use commscraft_sdp as sdp;

/// Commands to a media thread.
pub enum ToMedia {
    /// The answer: start the media (`early`: from a provisional response, SD-50). A second
    /// `Start` (an answer from another fork) starts it again with the new answer.
    Start {
        media: Box<sdp::Negotiated>,
        controlling: bool,
        early: bool,
    },
    /// The 200 of the early media's fork: speech may start, its differences take effect.
    Confirm(Box<sdp::Negotiated>),
    /// 199: the early media's fork ended.
    EndEarly,
    Update(Box<sdp::Negotiated>),
    Dtmf(char),
    Mute(bool),
    Hold(bool),
    Stop,
}

/// Reports of a media thread (with its local media id).
pub enum FromMedia {
    /// Gathering done: the SDP parameters for the INVITE or the 200 OK.
    Ready(u32, Box<sdp::Local>),
    /// ICE selected the path; the caller sends this as final offer (IC-07).
    Selected(u32, Box<sdp::Local>),
    Dtmf(u32, char),
    Failed(u32, String),
    PeerBye(u32),
}

pub struct MediaHandle {
    pub tx: Sender<ToMedia>,
}

const CNAME: &str = "commscraft@linux";

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The first relay of the MS-AVEDGEA answer that resolves (internet relays first).
pub fn relay_from(creds: &commscraft_avedgea::Credentials) -> Option<cm::Relay> {
    let mut relays: Vec<&commscraft_avedgea::Relay> = creds.relays.iter().collect();
    relays.sort_by_key(|r| r.location != Some(commscraft_avedgea::Location::Internet));
    let resolve = |r: &commscraft_avedgea::Relay, port: u16| match &r.host {
        commscraft_avedgea::RelayHost::Address(a) => Some(SocketAddr::new(*a, port)),
        commscraft_avedgea::RelayHost::Name(n) => (n.as_str(), port)
            .to_socket_addrs()
            .ok()?
            .find(SocketAddr::is_ipv4),
    };
    let (relay, server) = relays
        .iter()
        .find_map(|r| resolve(r, r.udp_port).map(|s| (*r, s)))?;
    Some(cm::Relay {
        server,
        tcp_server: resolve(relay, relay.tcp_port),
        username: creds.username.clone(),
        password: zeroize::Zeroizing::new(creds.password.as_bytes().to_vec()),
    })
}

/// Starts the media thread of one call: gathers candidates, reports `Ready`, then runs the
/// call until `Stop`.
pub fn spawn(
    id: u32,
    local_ip: IpAddr,
    relay: Option<cm::Relay>,
    device: alsa::Config,
    events: Sender<FromMedia>,
) -> MediaHandle {
    let (tx, rx) = mpsc::channel::<ToMedia>();
    std::thread::spawn(move || run(id, local_ip, relay, device, rx, events));
    MediaHandle { tx }
}

fn run(
    id: u32,
    local_ip: IpAddr,
    relay: Option<cm::Relay>,
    mut device: alsa::Config,
    rx: Receiver<ToMedia>,
    events: Sender<FromMedia>,
) {
    let t0 = now_ms();
    let (mut media, first) = match CallMedia::prepare(local_ip, relay.as_ref(), t0) {
        Ok(x) => x,
        Err(e) => {
            let _ = events.send(FromMedia::Failed(id, format!("media sockets: {e}")));
            return;
        }
    };
    drop(relay);
    media.send_initial(first);
    while !media.gathered(now_ms()) {
        media.poll(now_ms());
        std::thread::sleep(Duration::from_millis(5));
    }
    let _ = events.send(FromMedia::Ready(id, Box::new(media.local_description())));

    // Wait for the answer (or the end of the call).
    let (negotiated, controlling, early) = loop {
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(ToMedia::Start {
                media: n,
                controlling,
                early,
            }) => break (n, controlling, early),
            Ok(ToMedia::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                media.stop(now_ms());
                return;
            }
            Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {
                media.poll(now_ms());
            }
        }
    };
    let started = if early {
        media.start_early(&negotiated, CNAME, now_ms())
    } else {
        media.start(&negotiated, controlling, CNAME, now_ms())
    };
    if let Err(e) = started {
        let _ = events.send(FromMedia::Failed(id, e));
        return;
    }
    device.rate = media.sample_rate();
    let rate = device.rate;
    // The final offer (IC-07) is a re-INVITE: only once the call is answered.
    let mut answered = !early;
    let mut selected_early = false;
    let stop = Arc::new(AtomicBool::new(false));
    let (cap_tx, cap_rx) = mpsc::sync_channel::<Vec<i16>>(8);
    let (play_tx, play_rx) = mpsc::sync_channel::<Vec<i16>>(8);
    spawn_capture(device.clone(), cap_tx, stop.clone(), events.clone(), id);
    spawn_playback(device, play_rx, stop.clone(), events.clone(), id);

    let mut next_frame = now_ms();
    let mut final_sent = false;
    let mut dtmf: std::collections::VecDeque<char> = std::collections::VecDeque::new();
    loop {
        let now = now_ms();
        let mut restarted = None;
        match rx.try_recv() {
            Ok(ToMedia::Stop) | Err(mpsc::TryRecvError::Disconnected) => break,
            Ok(ToMedia::Update(n)) => media.update(&n),
            Ok(ToMedia::Start {
                media: n, early, ..
            }) => {
                restarted = Some(if early {
                    media.start_early(&n, CNAME, now)
                } else {
                    media.start(&n, true, CNAME, now)
                });
                answered = !early;
                final_sent = false;
                selected_early = false;
            }
            Ok(ToMedia::Confirm(n)) => {
                restarted = Some(media.confirm(&n, CNAME, now));
                answered = true;
                if selected_early && controlling && !final_sent {
                    final_sent = true;
                    if let Some(d) = media.final_description() {
                        let _ = events.send(FromMedia::Selected(id, Box::new(d)));
                    }
                }
            }
            Ok(ToMedia::EndEarly) => {
                media.end_early();
                selected_early = false;
            }
            Ok(ToMedia::Dtmf(d)) => {
                if dtmf.len() < 32 {
                    dtmf.push_back(d);
                }
            }
            Ok(ToMedia::Mute(m)) => media.set_muted(m),
            Ok(ToMedia::Hold(h)) => media.set_hold(h),
            Err(mpsc::TryRecvError::Empty) => {}
        }
        match restarted {
            Some(Err(e)) => {
                let _ = events.send(FromMedia::Failed(id, e));
                break;
            }
            // The devices run at the first answer's rate; another codec cannot be played.
            Some(Ok(())) if media.sample_rate() != rate => {
                let _ = events.send(FromMedia::Failed(
                    id,
                    "codec changed after early media".into(),
                ));
                break;
            }
            _ => {}
        }
        // One digit at a time; the next one when the previous has been sent (RFC 4733).
        if let Some(&d) = dtmf.front() {
            if media.send_dtmf(d) {
                dtmf.pop_front();
            }
        }
        while let Ok(frame) = cap_rx.try_recv() {
            media.send_frame(&frame, now);
        }
        if now >= next_frame {
            next_frame = now + 20;
            // Bounded: if the speaker falls behind, the frame is dropped.
            let _ = play_tx.try_send(media.playout());
        }
        for e in media.poll(now) {
            let report = match e {
                cm::Event::Selected if !answered => {
                    selected_early = true;
                    None
                }
                cm::Event::Selected if controlling && !final_sent => {
                    final_sent = true;
                    media
                        .final_description()
                        .map(|d| FromMedia::Selected(id, Box::new(d)))
                }
                cm::Event::Selected => None,
                cm::Event::Dtmf(d) => Some(FromMedia::Dtmf(id, d)),
                cm::Event::Failed(why) => Some(FromMedia::Failed(id, why)),
                cm::Event::PeerBye => Some(FromMedia::PeerBye(id)),
            };
            if let Some(r) = report {
                let _ = events.send(r);
            }
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    stop.store(true, Ordering::Relaxed);
    media.stop(now_ms());
}

fn spawn_capture(
    device: alsa::Config,
    tx: SyncSender<Vec<i16>>,
    stop: Arc<AtomicBool>,
    events: Sender<FromMedia>,
    id: u32,
) {
    std::thread::spawn(move || {
        let mut mic = match alsa::Capture::open(&device) {
            Ok(m) => m,
            Err(e) => {
                let _ = events.send(FromMedia::Failed(id, format!("microphone: {e}")));
                return;
            }
        };
        let mut frame = Vec::new();
        while !stop.load(Ordering::Relaxed) {
            if mic.read_frame(&mut frame).is_err() {
                break;
            }
            let _ = tx.try_send(frame.clone());
        }
    });
}

fn spawn_playback(
    device: alsa::Config,
    rx: Receiver<Vec<i16>>,
    stop: Arc<AtomicBool>,
    events: Sender<FromMedia>,
    id: u32,
) {
    std::thread::spawn(move || {
        let mut speaker = match alsa::Playback::open(&device) {
            Ok(s) => s,
            Err(e) => {
                let _ = events.send(FromMedia::Failed(id, format!("speaker: {e}")));
                return;
            }
        };
        while !stop.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(frame) => {
                    if speaker.write(&frame).is_err() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

/// The encryption policy from provisioning (SD-14): Require ⇒ required, Support ⇒ optional
/// without plaintext fallback (design), DoNotSupport ⇒ plaintext; unknown ⇒ required.
pub fn policy(av: Option<commscraft_provisioning::AvEncryption>) -> sdp::Encryption {
    match av {
        Some(commscraft_provisioning::AvEncryption::DoNotSupport) => sdp::Encryption::Off,
        Some(commscraft_provisioning::AvEncryption::Support) => sdp::Encryption::Optional {
            allow_plaintext: false,
        },
        _ => sdp::Encryption::Required,
    }
}
