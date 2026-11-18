//! Local preferences of the desktop client in `~/.config/commscraft/desktop.conf` (mode 0600):
//! favourites (SIP addresses, group titles, chat rooms) and the options of the options window
//! (language, display, contact list, notifications, sounds, camera device, meetings).
//!
//! No chat content, no presence, no password is ever written here. The file is read bounded;
//! unknown or malformed lines are skipped.

use commscraft_ui::app::Prefs;
use std::io::Write;
use std::path::PathBuf;

/// Largest file read.
const MAX_FILE: u64 = 64 * 1024;
/// Favourites kept at most.
pub const MAX_FAVORITES: usize = 500;

fn path() -> Option<PathBuf> {
    commscraft_client::setup::config_dir().map(|d| d.join("desktop.conf"))
}

/// A favourite as stored: a SIP address or `group:` and a title, printable, one line.
fn valid_favorite(v: &str) -> bool {
    let printable = !v.is_empty() && v.len() <= 300 && v.chars().all(|c| !c.is_control());
    printable
        && (commscraft_ui::normalise_address(v).as_deref() == Some(v)
            || v.starts_with("group:")
            || v.starts_with("room:ma-chan://"))
}

/// A sound device name as the audio adapter accepts it (empty: none chosen).
fn valid_device(name: &str) -> bool {
    !name.is_empty()
        && commscraft_audio_alsa::Config {
            device: name.to_string(),
            ..Default::default()
        }
        .validate()
        .is_ok()
}

/// Longest idle limit in minutes (design).
pub const MAX_IDLE_MINUTES: u32 = 360;

pub fn parse(text: &str) -> Prefs {
    let mut p = Prefs::default();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim();
        match k.trim() {
            "favorite" if valid_favorite(v) && p.favorites.len() < MAX_FAVORITES => {
                p.favorites.insert(v.to_string());
            }
            "high_contrast" => p.high_contrast = v == "true",
            "lang" => p.lang = commscraft_ui::text::Lang::from_locale(v),
            "always_on_top" => p.always_on_top = v == "true",
            "last_address" if commscraft_ui::normalise_address(v).as_deref() == Some(v) => {
                p.last_address = v.to_string()
            }
            "show_pictures" => p.show_pictures = v != "false",
            "compact_contacts" => p.compact_contacts = v == "true",
            "sort_by_availability" => p.sort_by_availability = v == "true",
            "away_group" => p.away_group = v == "true",
            "favorites_group" => p.favorites_group = v != "false",
            "message_alerts" => p.message_alerts = v != "false",
            "quiet_when_dnd" => p.quiet_when_dnd = v != "false",
            "sounds" => p.sounds = v != "false",
            "quiet_sounds_busy" => p.quiet_sounds_busy = v == "true",
            "meeting_participants" => p.meeting_participants = v == "true",
            "save_history" => p.save_history = v != "false",
            "tabbed_conversations" => p.tabbed_conversations = v != "false",
            "always_close_all_tabs" => p.always_close_all_tabs = v == "true",
            // A device name as the audio adapter accepts it.
            "audio_device" if valid_device(v) => p.audio_device = v.to_string(),
            // A device file only: printable, absolute, bounded.
            "camera"
                if v.starts_with("/dev/") && v.len() <= 200 && !v.chars().any(char::is_control) =>
            {
                p.camera = v.to_string()
            }
            "chat_text_size" => {
                if let Ok(s) = v.parse::<f32>() {
                    if (12.0..=20.0).contains(&s) {
                        p.chat_text_size = s;
                    }
                }
            }
            "context_open" => p.context_open = v != "false",
            "inactive_minutes" => {
                if let Ok(m) = v.parse::<u32>() {
                    p.inactive_minutes = m.min(MAX_IDLE_MINUTES);
                }
            }
            "away_minutes" => {
                if let Ok(m) = v.parse::<u32>() {
                    p.away_minutes = m.min(MAX_IDLE_MINUTES);
                }
            }
            "scale" => {
                if let Ok(s) = v.parse::<f32>() {
                    if (1.0..=2.0).contains(&s) {
                        p.scale = s;
                    }
                }
            }
            _ => {}
        }
    }
    p
}

pub fn format(p: &Prefs) -> String {
    let mut out = String::from("# CommSCraft desktop preferences (no chat content)\n");
    out.push_str(&format!("high_contrast={}\n", p.high_contrast));
    out.push_str(&format!("context_open={}\n", p.context_open));
    out.push_str(&format!("scale={}\n", p.scale));
    out.push_str(&format!(
        "lang={}\n",
        if p.lang == commscraft_ui::text::Lang::En {
            "en"
        } else {
            "de"
        }
    ));
    for (k, v) in [
        ("always_on_top", p.always_on_top),
        ("show_pictures", p.show_pictures),
        ("compact_contacts", p.compact_contacts),
        ("sort_by_availability", p.sort_by_availability),
        ("away_group", p.away_group),
        ("favorites_group", p.favorites_group),
        ("message_alerts", p.message_alerts),
        ("quiet_when_dnd", p.quiet_when_dnd),
        ("sounds", p.sounds),
        ("quiet_sounds_busy", p.quiet_sounds_busy),
        ("meeting_participants", p.meeting_participants),
        ("save_history", p.save_history),
        ("tabbed_conversations", p.tabbed_conversations),
        ("always_close_all_tabs", p.always_close_all_tabs),
    ] {
        out.push_str(&format!("{k}={v}\n"));
    }
    out.push_str(&format!("chat_text_size={}\n", p.chat_text_size));
    out.push_str(&format!("inactive_minutes={}\n", p.inactive_minutes));
    if valid_device(&p.audio_device) {
        out.push_str(&format!("audio_device={}\n", p.audio_device));
    }
    out.push_str(&format!("away_minutes={}\n", p.away_minutes));
    if commscraft_ui::normalise_address(&p.last_address).as_deref() == Some(p.last_address.as_str())
    {
        out.push_str(&format!("last_address={}\n", p.last_address));
    }
    if p.camera.starts_with("/dev/") {
        out.push_str(&format!("camera={}\n", p.camera));
    }
    for f in p
        .favorites
        .iter()
        .filter(|f| valid_favorite(f))
        .take(MAX_FAVORITES)
    {
        out.push_str(&format!("favorite={f}\n"));
    }
    out
}

/// The language chosen in the options, if the file names one.
pub fn saved_lang() -> Option<commscraft_ui::text::Lang> {
    let path = path()?;
    let ok = std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= MAX_FILE);
    let text = if ok {
        std::fs::read_to_string(&path).ok()?
    } else {
        return None;
    };
    text.lines()
        .find_map(|l| l.strip_prefix("lang="))
        .map(|v| commscraft_ui::text::Lang::from_locale(v.trim()))
}

pub fn load() -> Prefs {
    let Some(path) = path() else {
        return Prefs::default();
    };
    let ok = std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= MAX_FILE);
    if !ok {
        return Prefs::default();
    }
    let mut p = std::fs::read_to_string(&path)
        .map(|t| parse(&t))
        .unwrap_or_default();
    // Whether the autostart entry is there is the truth, not a copy in this file.
    p.autostart = crate::desktop::autostart_enabled();
    p
}

pub fn save(p: &Prefs) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let path = path().ok_or("no HOME")?;
    if let Some(dir) = path.parent() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("conf.new");
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(|e| e.to_string())?;
    f.write_all(format(p).as_bytes())
        .map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_limits_are_bounded() {
        let p = parse("inactive_minutes=99999\naway_minutes=-3\naudio_device=hw;reboot\n");
        assert!(p.audio_device.is_empty());
        assert_eq!(p.inactive_minutes, MAX_IDLE_MINUTES);
        assert_eq!(p.away_minutes, 5);
    }

    #[test]
    fn round_trip_and_hostile_lines() {
        let mut p = Prefs {
            high_contrast: true,
            scale: 1.25,
            lang: commscraft_ui::text::Lang::En,
            compact_contacts: true,
            sounds: false,
            camera: "/dev/video2".into(),
            chat_text_size: 16.0,
            last_address: "sip:mara@beispiel.invalid".into(),
            inactive_minutes: 0,
            away_minutes: 30,
            save_history: false,
            audio_device: "plughw:1,0".into(),
            ..Default::default()
        };
        p.favorites
            .insert("room:ma-chan://beispiel.invalid/ab-12".into());
        p.favorites.insert("sip:anna@beispiel.invalid".into());
        p.favorites.insert("group:Projekt Nord".into());
        let back = parse(&format(&p));
        assert_eq!(back, p);
        let odd = parse(
            "favorite=not an address\nfavorite=sip:a@b.invalid>;x\nscale=9\nhigh_contrast=yes\nnonsense\n",
        );
        assert!(odd.favorites.is_empty());
        assert_eq!(odd.scale, 1.0);
        assert!(!odd.high_contrast);
    }
}
