//! The desktop's own places: starting with the session (XDG Autostart: a desktop entry in
//! `$XDG_CONFIG_HOME/autostart`, Desktop Application Autostart Specification 0.5) and the
//! sound devices ALSA names (`/proc/asound/pcm`).
//!
//! Security by design: the autostart entry is written only when the user turns it on, with
//! the path of this program quoted per the Desktop Entry Specification (1.5, "The Exec key");
//! a path that cannot be quoted safely is refused. The device list is bounded and only
//! numbers from it become device names.

use commscraft_ui::model::{AppEvent, NoticeKind};
use std::path::PathBuf;

/// Most sound devices listed (design).
const MAX_DEVICES: usize = 64;

fn autostart_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("autostart").join("commscraft.desktop"))
}

/// Whether CommSCraft starts with the desktop session.
pub fn autostart_enabled() -> bool {
    autostart_file().is_some_and(|p| p.is_file())
}

/// Quotes a path for the Exec key: double quotes, with `"`, `` ` ``, `$` and `\` escaped; a
/// path with control characters or a `%` (field codes) is refused.
fn exec_quoted(path: &str) -> Option<String> {
    if path.chars().any(|c| c.is_control() || c == '%') {
        return None;
    }
    let mut q = String::from("\"");
    for c in path.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            q.push('\\');
        }
        q.push(c);
    }
    q.push('"');
    Some(q)
}

/// The desktop entry that starts this program.
fn entry(exe: &str) -> Option<String> {
    Some(format!(
        "[Desktop Entry]\nType=Application\nName=CommSCraft\nComment=Skype for Business client\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        exec_quoted(exe)?
    ))
}

/// Turns starting with the session on or off.
pub fn set_autostart(on: bool) -> Result<(), String> {
    let file = autostart_file().ok_or("no configuration directory")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    set_autostart_at(&file, on, &exe.to_string_lossy())
}

fn set_autostart_at(file: &std::path::Path, on: bool, exe: &str) -> Result<(), String> {
    if !on {
        return match std::fs::remove_file(file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        };
    }
    let text = entry(exe).ok_or("the program's path cannot be quoted")?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, text).map_err(|e| e.to_string())
}

/// The sound devices: (ALSA device name, label). `plughw` converts the sample rate.
pub fn sound_devices() -> Vec<(String, String)> {
    let text = std::fs::read_to_string("/proc/asound/pcm").unwrap_or_default();
    parse_pcm(&text)
}

/// `CC-DD: id : name : playback 1 : capture 1` lines → `plughw:C,D`.
fn parse_pcm(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let (nums, rest) = l.split_once(':')?;
            let (card, dev) = nums.trim().split_once('-')?;
            let card: u32 = card.parse().ok()?;
            let dev: u32 = dev.parse().ok()?;
            let fields: Vec<&str> = rest.split(" : ").map(str::trim).collect();
            let name = fields.get(1).or(fields.first())?;
            let mut kinds = Vec::new();
            if fields.iter().any(|f| f.starts_with("capture")) {
                kinds.push("capture");
            }
            if fields.iter().any(|f| f.starts_with("playback")) {
                kinds.push("playback");
            }
            let name: String = name.chars().filter(|c| !c.is_control()).take(64).collect();
            Some((
                format!("plughw:{card},{dev}"),
                format!("{name} ({})", kinds.join(", ")),
            ))
        })
        .take(MAX_DEVICES)
        .collect()
}

/// The test tone: 440 Hz for 0.8 s at 16 kHz, a quarter of full scale, with a short fade in and
/// out so it does not click.
fn test_tone() -> Vec<i16> {
    const RATE: f32 = 16_000.0;
    let n = (RATE * 0.8) as usize;
    let fade = 800.0;
    (0..n)
        .map(|i| {
            let x = i as f32;
            let env = (x / fade).min((n as f32 - x) / fade).clamp(0.0, 1.0);
            let s = (2.0 * std::f32::consts::PI * 440.0 * x / RATE).sin();
            (s * env * 8192.0) as i16
        })
        .collect()
}

fn test_config(device: &str, mic: &str) -> commscraft_audio_alsa::Config {
    let cfg = commscraft_audio_alsa::Config {
        device: if device.is_empty() {
            "default".into()
        } else {
            device.into()
        },
        capture_device: mic.into(),
        ..Default::default()
    };
    if cfg.validate().is_ok() {
        cfg
    } else {
        commscraft_audio_alsa::Config::default()
    }
}

/// Plays the test tone on the speaker in a thread of its own; a failure comes back as a notice.
pub fn test_speaker(device: String, tx: std::sync::mpsc::Sender<AppEvent>, repaint: Repaint) {
    let _ = std::thread::Builder::new()
        .name("speaker-test".into())
        .spawn(move || {
            let cfg = test_config(&device, "");
            let played = commscraft_audio_alsa::Playback::open(&cfg).and_then(|mut p| {
                let frame = p.frame_samples().max(1);
                for chunk in test_tone().chunks(frame) {
                    let mut f = chunk.to_vec();
                    f.resize(frame, 0);
                    p.write(&f)?;
                }
                Ok(())
            });
            if let Err(e) = played {
                let _ = tx.send(AppEvent::Notice(
                    NoticeKind::Warning,
                    format!("speaker test: {e:?}"),
                    unix_now(),
                ));
                repaint();
            }
        });
}

/// The level of a frame, 0–100: its peak in dB below full scale, −60 dBFS (and quieter) to 0.
fn level(frame: &[i16]) -> u8 {
    let peak = frame.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
    if peak == 0 {
        return 0;
    }
    let db = 20.0 * (f32::from(peak) / 32_768.0).log10();
    ((db + 60.0) / 60.0 * 100.0).clamp(0.0, 100.0) as u8
}

/// Listens to the microphone for five seconds and reports its level ten times a second; nothing
/// is kept or sent.
pub fn test_microphone(device: String, tx: std::sync::mpsc::Sender<AppEvent>, repaint: Repaint) {
    let _ = std::thread::Builder::new()
        .name("microphone-test".into())
        .spawn(move || {
            let cfg = test_config("default", &device);
            let _ = tx.send(AppEvent::MicLevel(Some(0)));
            let result = commscraft_audio_alsa::Capture::open(&cfg).and_then(|mut c| {
                let mut frame = Vec::new();
                let frames_per_report = (100 / cfg.frame_ms.max(1)).max(1) as usize;
                let reports = 50;
                for _ in 0..reports {
                    let mut peak = 0u8;
                    for _ in 0..frames_per_report {
                        c.read_frame(&mut frame)?;
                        peak = peak.max(level(&frame));
                    }
                    let _ = tx.send(AppEvent::MicLevel(Some(peak)));
                    repaint();
                }
                Ok(())
            });
            if let Err(e) = result {
                let _ = tx.send(AppEvent::Notice(
                    NoticeKind::Warning,
                    format!("microphone test: {e:?}"),
                    unix_now(),
                ));
            }
            let _ = tx.send(AppEvent::MicLevel(None));
            repaint();
        });
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Asks the window to draw again (from a test thread).
pub type Repaint = std::sync::Arc<dyn Fn() + Send + Sync>;

/// Opens the system's mail program for this address (`xdg-open mailto:`); only plain addresses
/// pass, so nothing else reaches the command line.
pub fn send_email(address: &str) {
    if !valid_mail(address) {
        return;
    }
    let _ = std::process::Command::new("xdg-open")
        .arg(format!("mailto:{address}"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Opens the recordings folder (`$XDG_VIDEOS_DIR/CommSCraft`, created if missing) in the
/// desktop's file manager.
pub fn open_recordings() {
    let Some(dir) = commscraft_client::recording::folder() else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::process::Command::new("xdg-open")
        .arg(&dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn valid_mail(a: &str) -> bool {
    let Some((user, host)) = a.split_once('@') else {
        return false;
    };
    a.len() <= 254
        && !user.is_empty()
        && host.contains('.')
        && !a.starts_with('-')
        && a.chars()
            .all(|c| c.is_ascii_alphanumeric() || "@._+-".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_lines_become_device_names() {
        let d = parse_pcm(
            "00-03: HDMI 0 : HDMI 0 : playback 1\n01-00: USB Audio : USB Audio : playback 1 : capture 1\nnonsense\n",
        );
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].0, "plughw:0,3");
        assert_eq!(
            d[1],
            ("plughw:1,0".into(), "USB Audio (capture, playback)".into())
        );
        // The names fit the device name check of the audio adapter.
        for (n, _) in &d {
            assert!(commscraft_audio_alsa::Config {
                device: n.clone(),
                ..Default::default()
            }
            .validate()
            .is_ok());
        }
    }

    #[test]
    fn the_autostart_entry_is_written_and_removed() {
        let dir = std::env::temp_dir().join(format!("commscraft-autostart-{}", std::process::id()));
        let file = dir.join("autostart").join("commscraft.desktop");
        set_autostart_at(&file, true, "/usr/bin/commscraft").unwrap();
        assert!(std::fs::read_to_string(&file)
            .unwrap()
            .starts_with("[Desktop Entry]"));
        set_autostart_at(&file, false, "/usr/bin/commscraft").unwrap();
        assert!(!file.exists());
        // Off when already off is fine; a path that cannot be quoted is refused.
        set_autostart_at(&file, false, "").unwrap();
        assert!(set_autostart_at(&file, true, "/tmp/%u").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_exec_key_is_quoted_or_refused() {
        assert_eq!(
            exec_quoted("/opt/Comm S$craft/bin").as_deref(),
            Some("\"/opt/Comm S\\$craft/bin\"")
        );
        assert!(exec_quoted("/tmp/%f").is_none());
        assert!(exec_quoted("/tmp/a\nb").is_none());
        assert!(entry("/usr/bin/commscraft")
            .unwrap()
            .contains("Exec=\"/usr/bin/commscraft\""));
    }

    #[test]
    fn the_test_tone_and_the_level_stay_in_bounds() {
        let tone = test_tone();
        assert_eq!(tone.len(), 12_800);
        assert!(tone.iter().all(|s| s.unsigned_abs() <= 8192));
        // Faded in and out: no click at the ends.
        assert!(tone[0].unsigned_abs() < 100 && tone[tone.len() - 1].unsigned_abs() < 100);
        assert_eq!(level(&[]), 0);
        assert_eq!(level(&[i16::MIN]), 100);
        // Half of full scale is −6 dBFS; −60 dBFS and quieter is 0.
        assert_eq!(level(&[16_384, -100]), 89);
        assert_eq!(level(&[30]), 0);
    }

    #[test]
    fn only_plain_mail_addresses_reach_the_mail_program() {
        assert!(valid_mail("bob@beispiel.invalid"));
        assert!(!valid_mail("bob"));
        assert!(!valid_mail("-x@beispiel.invalid"));
        assert!(!valid_mail("a@b.c?subject=x"));
        assert!(!valid_mail("a b@beispiel.invalid"));
    }
}
