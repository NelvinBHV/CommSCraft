//! Calls in the interactive mode: one media thread per call (gathering, ICE, SRTP, codec) with
//! two device threads (ALSA capture and playback, decision D-11). The SIP side stays in the
//! engine; this module only moves media and reports to the main loop.
//!
//! Security by design: devices are opened only for an answered call and closed when it ends;
//! every queue between threads is bounded (frames are dropped, never buffered without limit);
//! the relay password is handed to the media thread once and zeroised there. A missing or
//! failing microphone or speaker does not end the call: the call goes on with silence or without
//! sound, and the user is told (a call is never hung up because of a local device).

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
    /// Something the user should know that does not end the call (a missing audio device).
    Notice(u32, String),
    /// The media ended: its quality figures for the report (function 45).
    Quality(u32, Box<cm::QualitySnapshot>),
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
    bwm: Option<cm::BwmConfig>,
    device: alsa::Config,
    events: Sender<FromMedia>,
) -> MediaHandle {
    let (tx, rx) = mpsc::channel::<ToMedia>();
    std::thread::spawn(move || run(id, local_ip, relay, bwm, device, rx, events));
    MediaHandle { tx }
}

fn run(
    id: u32,
    local_ip: IpAddr,
    relay: Option<cm::Relay>,
    bwm: Option<cm::BwmConfig>,
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
    if let Some(cfg) = bwm {
        media.enable_bandwidth_management(cfg);
    }
    media.send_initial(first);
    while !media.gathered(now_ms()) {
        media.poll(now_ms());
        std::thread::sleep(Duration::from_millis(5));
    }
    // [MS-ICE2BWM] §3.1.4.2: every path refused – no answer is sent, the call fails.
    if media.bandwidth_blocked() {
        let _ = events.send(FromMedia::Failed(
            id,
            "the bandwidth policy allows no media path".into(),
        ));
        media.stop(now_ms());
        return;
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
                // The audio stream carries no video.
                cm::Event::Video(_) => None,
            };
            if let Some(r) = report {
                let _ = events.send(r);
            }
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    stop.store(true, Ordering::Relaxed);
    if let Some(q) = media.quality() {
        let _ = events.send(FromMedia::Quality(id, Box::new(q)));
    }
    media.stop(now_ms());
}

/// The report's audio line from the media figures (QE-02; `offerer` is set by the engine).
pub fn audio_line(q: &cm::QualitySnapshot) -> commscraft_qoe::AudioLine {
    let s = |x: &commscraft_media::StreamQuality| commscraft_qoe::Stream {
        ssrc: x.ssrc,
        packets: x.packets,
        jitter_avg_ms: x.jitter_avg_ms,
        jitter_max_ms: x.jitter_max_ms,
        loss_avg_ppm: x.loss_avg_ppm,
        loss_max_ppm: x.loss_max_ppm,
        rtt_avg_ms: x.rtt_avg_ms,
        rtt_max_ms: x.rtt_max_ms,
    };
    commscraft_qoe::AudioLine {
        encrypted: q.encrypted,
        offerer: false,
        tcp: q.tcp,
        local: q.local,
        remote: q.remote,
        inbound: q.audio.inbound.as_ref().map(s),
        outbound: Some(s(&q.audio.outbound)),
        codec: q.audio.codec.to_string(),
        sample_rate: q.audio.sample_rate,
    }
}

fn spawn_capture(
    device: alsa::Config,
    tx: SyncSender<Vec<i16>>,
    stop: Arc<AtomicBool>,
    events: Sender<FromMedia>,
    id: u32,
) {
    std::thread::spawn(move || {
        let mut frame = Vec::new();
        match alsa::Capture::open(&device) {
            Ok(mut mic) => {
                while !stop.load(Ordering::Relaxed) {
                    if let Err(e) = mic.read_frame(&mut frame) {
                        let _ = events.send(FromMedia::Notice(
                            id,
                            format!("microphone stopped ({e}); sending silence"),
                        ));
                        break;
                    }
                    let _ = tx.try_send(frame.clone());
                }
            }
            Err(e) => {
                let _ = events.send(FromMedia::Notice(
                    id,
                    format!("no microphone ({e}); sending silence"),
                ));
            }
        }
        // Without a microphone the call goes on: silence in the packet rhythm keeps the media
        // flowing for the peer (the speaker side is unaffected).
        let silence = silence_frame(&device);
        let pace = Duration::from_millis(u64::from(device.frame_ms.max(1)));
        while !stop.load(Ordering::Relaxed) {
            let _ = tx.try_send(silence.clone());
            std::thread::sleep(pace);
        }
    });
}

/// One frame of silence for the device's rate and packet time (`rate × frame_ms / 1000`
/// samples, bounded by one second of audio).
fn silence_frame(device: &alsa::Config) -> Vec<i16> {
    let samples = u64::from(device.rate)
        .saturating_mul(u64::from(device.frame_ms))
        .checked_div(1000)
        .unwrap_or(0)
        .min(u64::from(device.rate));
    vec![0; usize::try_from(samples).unwrap_or(0)]
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
            Ok(s) => Some(s),
            Err(e) => {
                let _ = events.send(FromMedia::Notice(
                    id,
                    format!("no speaker ({e}); the call goes on without sound"),
                ));
                None
            }
        };
        while !stop.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(frame) => {
                    // Without a speaker the frames are dropped; the queue never fills up.
                    if let Some(sp) = &mut speaker {
                        if let Err(e) = sp.write(&frame) {
                            let _ = events.send(FromMedia::Notice(
                                id,
                                format!("speaker stopped ({e}); the call goes on without sound"),
                            ));
                            speaker = None;
                        }
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

/// The bandwidth management settings for a call when the server enables it (`enableBWPolicyCheck`,
/// [MS-SIPREGE]). `offer`: Call-ID and offer of an incoming call (the callee checks). Inference:
/// a peer in our own SIP domain is not federated, any other counts as enterprise federation;
/// locations are not known to the client ("unknown").
pub fn bandwidth_config(
    enabled: bool,
    own_aor: &str,
    peer: Option<&str>,
    offer: Option<(&str, &[u8])>,
) -> Option<cm::BwmConfig> {
    use commscraft_turn_bwm as b;
    if !enabled {
        return None;
    }
    let domain = |u: &str| u.rsplit_once('@').map(|(_, d)| d.to_ascii_lowercase());
    let federation = match (peer.and_then(domain), domain(own_aor)) {
        (Some(p), Some(o)) if p != o => b::Federation::Enterprise,
        _ => b::Federation::None,
    };
    Some(cm::BwmConfig {
        call_id: offer.map(|(c, _)| c.to_string()).unwrap_or_default(),
        profile: b::LocationProfile {
            peer: b::Location::Unknown,
            own: b::Location::Unknown,
            federation,
        },
        peer: offer.and_then(|(_, o)| sdp::offer_ice(o)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bandwidth_config_rules() {
        assert!(
            bandwidth_config(false, "sip:a@contoso.com", None, None).is_none(),
            "off unless provisioned"
        );
        let c =
            bandwidth_config(true, "sip:a@contoso.com", Some("sip:b@contoso.com"), None).unwrap();
        assert_eq!(c.profile.federation, commscraft_turn_bwm::Federation::None);
        assert!(c.peer.is_none() && c.call_id.is_empty(), "caller: no check");
        let c = bandwidth_config(
            true,
            "sip:a@contoso.com",
            Some("sip:b@fabrikam.com"),
            Some(("id1", b"x")),
        )
        .unwrap();
        assert_eq!(
            c.profile.federation,
            commscraft_turn_bwm::Federation::Enterprise
        );
        assert_eq!(c.call_id, "id1");
    }

    #[test]
    fn silence_frame_size() {
        let cfg = |rate, frame_ms| alsa::Config {
            device: "default".into(),
            rate,
            frame_ms,
        };
        // G.722 and G.711 at 20 ms ([MS-RTP] §2.2.1).
        assert_eq!(silence_frame(&cfg(16000, 20)).len(), 320);
        assert_eq!(silence_frame(&cfg(8000, 20)).len(), 160);
        assert!(silence_frame(&cfg(8000, 20)).iter().all(|&x| x == 0));
        // Hostile or broken settings stay bounded: never more than one second of audio.
        assert_eq!(silence_frame(&cfg(8000, u32::MAX)).len(), 8000);
        assert_eq!(silence_frame(&cfg(0, 20)).len(), 0);
    }
}
