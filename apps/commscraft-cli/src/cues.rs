//! Ringing, notification sound and music on hold in the interactive mode (issues #82–#84).
//!
//! Each sound is chosen by an environment variable – a PCM WAV file, or `off`:
//! `COMMSCRAFT_RINGTONE` and `COMMSCRAFT_NOTIFY_SOUND` default to our own generated tones
//! (`commscraft-tones`), `COMMSCRAFT_HOLD_MUSIC` is off unless a file is named. No third-party
//! sound is bundled until its rights are cleared.
//!
//! Security by design: a sound file is read with a size limit and parsed by the bounded WAV
//! reader; the ringtone and the notification play on their own playback stream, never into the
//! call (only music on hold goes to the peer, and only while we hold); notifications are rate
//! limited; every playback thread ends with its stop flag or its sound.

use std::collections::BTreeMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use commscraft_audio_alsa as alsa;
use commscraft_tones as tones;

/// The rate the alert sounds are played at.
const RATE: u32 = 16000;
/// At most one notification sound in this time (milliseconds).
const NOTIFY_GAP_MS: u64 = 2000;

/// A sound chosen by an environment variable.
fn load(var: &str, default: Option<fn(u32) -> Vec<i16>>) -> (Option<tones::Pcm>, Option<String>) {
    match std::env::var(var) {
        Ok(v) if v.trim().eq_ignore_ascii_case("off") => (None, None),
        Ok(path) => match read_wav(&path) {
            Ok(p) => (Some(p), None),
            Err(e) => (
                default.map(generated),
                Some(format!(
                    "{var}: {e}; {}",
                    if default.is_some() {
                        "using the built-in sound"
                    } else {
                        "off"
                    }
                )),
            ),
        },
        Err(_) => (default.map(generated), None),
    }
}

fn generated(f: fn(u32) -> Vec<i16>) -> tones::Pcm {
    tones::Pcm {
        rate: RATE,
        samples: f(RATE),
    }
}

/// Reads a sound file, at most `tones::MAX_FILE` bytes.
fn read_wav(path: &str) -> Result<tones::Pcm, String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    f.take(tones::MAX_FILE as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    tones::parse_wav(&data).map_err(|e| e.to_string())
}

/// Samples at `rate`, level-matched.
fn prepared(p: &tones::Pcm, rate: u32) -> Vec<i16> {
    let mut s = tones::resample(p, rate);
    tones::normalize(&mut s);
    s
}

pub struct Cues {
    ring: Option<Arc<Vec<i16>>>,
    ring_is_file: bool,
    notify: Option<Arc<Vec<i16>>>,
    hold: Option<Arc<tones::Pcm>>,
    last_notify: Option<u64>,
    ringing: BTreeMap<u32, Arc<AtomicBool>>,
}

impl Cues {
    /// The sounds from the environment, and notices for the user (unusable files).
    pub fn from_env() -> (Cues, Vec<String>) {
        let mut notices = Vec::new();
        let ring_var = std::env::var("COMMSCRAFT_RINGTONE").ok();
        let (ring, n) = load("COMMSCRAFT_RINGTONE", Some(tones::ringtone));
        notices.extend(n);
        let (notify, n) = load("COMMSCRAFT_NOTIFY_SOUND", Some(tones::notification));
        notices.extend(n);
        let (hold, n) = load("COMMSCRAFT_HOLD_MUSIC", None);
        notices.extend(n);
        let ring_is_file = ring_var.is_some_and(|v| !v.trim().eq_ignore_ascii_case("off"))
            && notices
                .iter()
                .all(|n| !n.starts_with("COMMSCRAFT_RINGTONE"));
        let cues = Cues {
            ring: ring.map(|p| Arc::new(prepared(&p, RATE))),
            ring_is_file,
            notify: notify.map(|p| Arc::new(prepared(&p, RATE))),
            hold: hold.map(Arc::new),
            last_notify: None,
            ringing: BTreeMap::new(),
        };
        (cues, notices)
    }

    /// Music on hold for the call's media (resampled there to the codec's rate).
    pub fn hold_music(&self) -> Option<Arc<tones::Pcm>> {
        self.hold.clone()
    }

    /// An incoming call alerts: the ringtone loops until `stop_ring`.
    pub fn start_ring(&mut self, call: u32) {
        let Some(ring) = self.ring.clone() else {
            return;
        };
        if self.ringing.contains_key(&call) {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        self.ringing.insert(call, stop.clone());
        let fade = self.ring_is_file;
        std::thread::spawn(move || {
            let Ok(mut out) = alsa::Playback::open(&alsa::Config::default()) else {
                return;
            };
            let frame = out.frame_samples();
            let mut looper = tones::Looper::new(ring.to_vec(), RATE, fade);
            while !stop.load(Ordering::Relaxed) {
                if out.write(&looper.next_frame(frame)).is_err() {
                    return;
                }
            }
        });
    }

    /// The call was answered, declined, cancelled or timed out.
    pub fn stop_ring(&mut self, call: u32) {
        if let Some(stop) = self.ringing.remove(&call) {
            stop.store(true, Ordering::Relaxed);
        }
    }

    pub fn stop_all(&mut self) {
        for (_, stop) in std::mem::take(&mut self.ringing) {
            stop.store(true, Ordering::Relaxed);
        }
    }

    /// An incoming message: the notification sound once, at most every 2 s.
    pub fn notify(&mut self, now_ms: u64) {
        let Some(sound) = self.notify.clone() else {
            return;
        };
        if !notify_due(self.last_notify, now_ms) {
            return;
        }
        self.last_notify = Some(now_ms);
        std::thread::spawn(move || {
            let Ok(mut out) = alsa::Playback::open(&alsa::Config::default()) else {
                return;
            };
            let frame = out.frame_samples().max(1);
            for chunk in sound.chunks(frame) {
                let mut f = chunk.to_vec();
                f.resize(frame, 0);
                if out.write(&f).is_err() {
                    return;
                }
            }
        });
    }
}

fn notify_due(last: Option<u64>, now_ms: u64) -> bool {
    last.is_none_or(|t| now_ms.saturating_sub(t) >= NOTIFY_GAP_MS)
}

/// Music on hold at the codec's rate, looped in frames (with a fade at the loop boundary).
pub fn hold_looper(music: &tones::Pcm, rate: u32) -> tones::Looper {
    tones::Looper::new(prepared(music, rate), rate, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notifications_rate_limited() {
        assert!(notify_due(None, 0));
        assert!(!notify_due(Some(1000), 2999));
        assert!(notify_due(Some(1000), 3000));
        assert!(!notify_due(Some(5000), 1000), "a clock that went back");
    }

    #[test]
    fn hold_music_at_the_codec_rate() {
        let music = tones::Pcm {
            rate: 44100,
            samples: vec![1000; 44100],
        };
        let mut l = hold_looper(&music, 8000);
        let f = l.next_frame(160);
        assert_eq!(f.len(), 160);
        assert!(f.iter().skip(80).all(|s| s.abs() > 10_000), "level-matched");
    }
}
