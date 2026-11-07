//! Ringing, notification sound and music on hold in the interactive mode (issues #82–#84).
//!
//! The defaults are the owner-approved sounds (Beat Call, Signal Message, Pulse; embedded by
//! `commscraft-tones`). Each can be replaced or switched off by an environment variable – a PCM
//! WAV file, or `off` – and has a volume in percent:
//! `COMMSCRAFT_RINGTONE` / `COMMSCRAFT_RINGTONE_VOLUME`,
//! `COMMSCRAFT_NOTIFY_SOUND` / `COMMSCRAFT_NOTIFY_VOLUME`,
//! `COMMSCRAFT_HOLD_MUSIC` / `COMMSCRAFT_HOLD_VOLUME`.
//!
//! Security by design: a sound file is read with a size limit and parsed by the bounded WAV
//! reader; the ringtone and the notification play on their own playback stream, never into the
//! call; only music on hold goes to the peer, only while we hold and never into a meeting;
//! notifications are rate limited; every playback thread ends with its stop flag or its sound.

use std::collections::BTreeMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use commscraft_audio_alsa as alsa;
use commscraft_tones as tones;

/// The rates the alert sounds are played at: the masters' rate, else a rate every device has.
const ALERT_RATES: [u32; 2] = [44100, 16000];
/// At most one notification sound in this time (milliseconds).
const NOTIFY_GAP_MS: u64 = 2000;

/// A sound ready to be played: the approved master or a user's file, at a volume.
#[derive(Debug, Clone)]
pub struct Sound {
    pcm: Arc<tones::Pcm>,
    own_file: bool,
    volume: u32,
}

impl Sound {
    /// One cycle of a looped sound at `rate`, level-matched if it is a user's file.
    fn looped_at(&self, rate: u32) -> tones::Looper {
        let mut samples = tones::resample_loop(&self.pcm, rate);
        self.level(&mut samples);
        tones::Looper::new(samples, rate, self.own_file)
    }

    fn once_at(&self, rate: u32) -> Vec<i16> {
        let mut samples = tones::resample(&self.pcm, rate);
        self.level(&mut samples);
        samples
    }

    fn level(&self, samples: &mut [i16]) {
        if self.own_file {
            tones::normalize(samples);
        }
        tones::set_volume(samples, self.volume);
    }
}

/// The sound for a setting: unset = the built-in sound, `off` = none, else a WAV file (an
/// unusable file gives the built-in sound and a notice).
fn choose(
    name: &str,
    setting: Option<&str>,
    builtin: fn() -> Result<tones::Pcm, tones::Error>,
    volume: u32,
    notices: &mut Vec<String>,
) -> Option<Sound> {
    let own = match setting.map(str::trim) {
        Some(v) if v.eq_ignore_ascii_case("off") => return None,
        Some(path) => match read_wav(path) {
            Ok(p) => Some(p),
            Err(e) => {
                notices.push(format!("{name}: {e}; using the built-in sound"));
                None
            }
        },
        None => None,
    };
    let own_file = own.is_some();
    let pcm = match own {
        Some(p) => p,
        None => match builtin() {
            Ok(p) => p,
            Err(e) => {
                notices.push(format!("{name}: built-in sound: {e}"));
                return None;
            }
        },
    };
    Some(Sound {
        pcm: Arc::new(pcm),
        own_file,
        volume,
    })
}

/// A volume in percent; anything but 0 … 100 gives full volume and a notice.
fn volume_of(name: &str, setting: Option<&str>, notices: &mut Vec<String>) -> u32 {
    match setting.map(|v| v.trim().parse::<u32>()) {
        None => 100,
        Some(Ok(v)) if v <= 100 => v,
        Some(_) => {
            notices.push(format!("{name}: expected 0 to 100; using 100"));
            100
        }
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

/// A playback stream for an alert: at the masters' rate if the device has it.
fn open_alert() -> Option<(alsa::Playback, u32)> {
    ALERT_RATES.iter().find_map(|&rate| {
        alsa::Playback::open(&alsa::Config {
            rate,
            ..alsa::Config::default()
        })
        .ok()
        .map(|p| (p, rate))
    })
}

pub struct Cues {
    ring: Option<Sound>,
    notify: Option<Sound>,
    hold: Option<Sound>,
    last_notify: Option<u64>,
    ringing: BTreeMap<u32, Arc<AtomicBool>>,
}

impl Cues {
    /// The sounds from the environment, and notices for the user (unusable settings).
    pub fn from_env() -> (Cues, Vec<String>) {
        let var = |name: &str| std::env::var(name).ok();
        let mut notices = Vec::new();
        let mut sound = |name: &str, volume_name: &str, builtin| {
            let volume = volume_of(volume_name, var(volume_name).as_deref(), &mut notices);
            choose(name, var(name).as_deref(), builtin, volume, &mut notices)
        };
        let cues = Cues {
            ring: sound(
                "COMMSCRAFT_RINGTONE",
                "COMMSCRAFT_RINGTONE_VOLUME",
                tones::soundpack::beat_call,
            ),
            notify: sound(
                "COMMSCRAFT_NOTIFY_SOUND",
                "COMMSCRAFT_NOTIFY_VOLUME",
                tones::soundpack::signal_message,
            ),
            hold: sound(
                "COMMSCRAFT_HOLD_MUSIC",
                "COMMSCRAFT_HOLD_VOLUME",
                tones::soundpack::pulse,
            ),
            last_notify: None,
            ringing: BTreeMap::new(),
        };
        (cues, notices)
    }

    /// Music on hold for a call we put on hold – none in a meeting: it would play into the
    /// meeting for everybody (and a=feature:MoH is about a held peer, SD-07).
    pub fn hold_music(&self, in_meeting: bool) -> Option<Sound> {
        self.hold.clone().filter(|_| !in_meeting)
    }

    /// An incoming call alerts: the ringtone loops until `stop_ring`, the device's clock keeps
    /// the beat (frame after frame without a gap).
    pub fn start_ring(&mut self, call: u32) {
        let Some(ring) = self.ring.clone() else {
            return;
        };
        if self.ringing.contains_key(&call) {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        self.ringing.insert(call, stop.clone());
        std::thread::spawn(move || {
            let Some((mut out, rate)) = open_alert() else {
                return;
            };
            let frame = out.frame_samples();
            let mut looper = ring.looped_at(rate);
            while !stop.load(Ordering::Relaxed) {
                if out.write(&looper.next_frame(frame)).is_err() {
                    return;
                }
            }
            // Closing the stream discards what is queued: no ringing after the answer.
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
            let Some((mut out, rate)) = open_alert() else {
                return;
            };
            let samples = sound.once_at(rate);
            let frame = out.frame_samples().max(1);
            for chunk in samples.chunks(frame) {
                let mut f = chunk.to_vec();
                f.resize(frame, 0);
                if out.write(&f).is_err() {
                    return;
                }
            }
            let _ = out.drain();
        });
    }
}

fn notify_due(last: Option<u64>, now_ms: u64) -> bool {
    last.is_none_or(|t| now_ms.saturating_sub(t) >= NOTIFY_GAP_MS)
}

/// What goes to the peer while a call is held or not (#82): the microphone, or – while we hold
/// with music – only the music, never a mix. The microphone's frames only pace it.
#[derive(Debug, Default)]
pub struct HoldMedia {
    music: Option<tones::Looper>,
    user_muted: bool,
}

impl HoldMedia {
    /// Hold or resume; the music starts over at every hold.
    pub fn set_hold(&mut self, hold: bool, music: Option<&Sound>, rate: u32) {
        self.music = music.filter(|_| hold).map(|m| m.looped_at(rate));
    }

    pub fn set_user_muted(&mut self, muted: bool) {
        self.user_muted = muted;
    }

    pub fn has_music(&self) -> bool {
        self.music.is_some()
    }

    /// The media's mute: the user's, but it does not silence the music on hold.
    pub fn muted(&self) -> bool {
        self.user_muted && self.music.is_none()
    }

    pub fn frame(&mut self, microphone: &[i16]) -> Vec<i16> {
        match self.music.as_mut() {
            Some(m) => m.next_frame(microphone.len()),
            None => microphone.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_sound(own_file: bool, volume: u32) -> Sound {
        Sound {
            pcm: Arc::new(tones::Pcm {
                rate: 8000,
                samples: vec![1000; 8000],
            }),
            own_file,
            volume,
        }
    }

    fn cues_with_hold() -> Cues {
        Cues {
            ring: None,
            notify: None,
            hold: Some(test_sound(false, 100)),
            last_notify: None,
            ringing: BTreeMap::new(),
        }
    }

    /// #84: burst throttling.
    #[test]
    fn notifications_rate_limited() {
        assert!(notify_due(None, 0));
        assert!(!notify_due(Some(1000), 2999));
        assert!(notify_due(Some(1000), 3000));
        assert!(!notify_due(Some(5000), 1000), "a clock that went back");
    }

    /// #82–#84: the defaults are the approved sounds; `off` mutes; a user's file replaces them;
    /// an unusable file falls back with a notice.
    #[test]
    fn choosing_a_sound() {
        let mut notices = Vec::new();
        let builtin = tones::soundpack::signal_message;
        let default = choose("X", None, builtin, 100, &mut notices).unwrap();
        assert_eq!(default.pcm.samples.len(), 52_920);
        assert!(!default.own_file);
        assert!(choose("X", Some(" OFF "), builtin, 100, &mut notices).is_none());
        assert!(notices.is_empty());
        let missing = choose("X", Some("/no/such/file.wav"), builtin, 100, &mut notices).unwrap();
        assert!(!missing.own_file);
        assert_eq!(notices.len(), 1);
        assert!(notices[0].starts_with("X: "), "{notices:?}");
        // A file that is not WAV.
        let mut notices = Vec::new();
        let dir = std::env::temp_dir().join(format!("commscraft-cues-{}", std::process::id()));
        std::fs::write(&dir, b"not a sound").unwrap();
        let bad = choose("X", dir.to_str(), builtin, 100, &mut notices).unwrap();
        std::fs::remove_file(&dir).unwrap();
        assert!(!bad.own_file);
        assert_eq!(notices.len(), 1);
    }

    #[test]
    fn volumes() {
        let mut notices = Vec::new();
        assert_eq!(volume_of("V", None, &mut notices), 100);
        assert_eq!(volume_of("V", Some(" 40 "), &mut notices), 40);
        assert_eq!(volume_of("V", Some("0"), &mut notices), 0);
        assert!(notices.is_empty());
        assert_eq!(volume_of("V", Some("140"), &mut notices), 100);
        assert_eq!(volume_of("V", Some("loud"), &mut notices), 100);
        assert_eq!(notices.len(), 2);
    }

    /// The approved master keeps its level; a user's file is matched; the volume applies to
    /// both.
    #[test]
    fn levels() {
        let peak = |s: &Sound| s.once_at(8000).iter().map(|x| x.abs()).max().unwrap();
        assert_eq!(peak(&test_sound(false, 100)), 1000);
        assert_eq!(peak(&test_sound(false, 50)), 500);
        assert!(peak(&test_sound(true, 100)) > 20_000);
    }

    /// #82: while held with music only the music goes out, never the microphone; on resume the
    /// microphone again; a mute of the user does not silence the music but holds afterwards.
    #[test]
    fn hold_routing() {
        let mic = [7i16; 160];
        let music = test_sound(false, 100);
        let mut h = HoldMedia::default();
        assert_eq!(h.frame(&mic), mic.to_vec(), "no hold: the microphone");
        h.set_user_muted(true);
        assert!(h.muted());
        h.set_hold(true, Some(&music), 8000);
        assert!(h.has_music());
        assert!(!h.muted(), "the music is not muted");
        let out = h.frame(&mic);
        assert_eq!(out.len(), 160);
        assert!(
            out.iter().all(|s| *s == 1000),
            "only the music: {:?}",
            &out[..4]
        );
        h.set_hold(false, Some(&music), 8000);
        assert!(!h.has_music());
        assert!(h.muted(), "the user's mute holds again");
        assert_eq!(h.frame(&mic), mic.to_vec());
        // Holding without music: nothing changes here, the media stops sending.
        h.set_hold(true, None, 8000);
        assert!(!h.has_music());
    }

    /// No music into a meeting.
    #[test]
    fn no_music_in_a_meeting() {
        let c = cues_with_hold();
        assert!(c.hold_music(false).is_some());
        assert!(c.hold_music(true).is_none());
    }
}
