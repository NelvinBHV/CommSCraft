//! The desktop's own places: starting with the session (XDG Autostart: a desktop entry in
//! `$XDG_CONFIG_HOME/autostart`, Desktop Application Autostart Specification 0.5) and the
//! sound devices ALSA names (`/proc/asound/pcm`).
//!
//! Security by design: the autostart entry is written only when the user turns it on, with
//! the path of this program quoted per the Desktop Entry Specification (1.5, "The Exec key");
//! a path that cannot be quoted safely is refused. The device list is bounded and only
//! numbers from it become device names.

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
    fn only_plain_mail_addresses_reach_the_mail_program() {
        assert!(valid_mail("bob@beispiel.invalid"));
        assert!(!valid_mail("bob"));
        assert!(!valid_mail("-x@beispiel.invalid"));
        assert!(!valid_mail("a@b.c?subject=x"));
        assert!(!valid_mail("a b@beispiel.invalid"));
    }
}
