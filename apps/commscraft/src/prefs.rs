//! Local preferences of the desktop client in `~/.config/commscraft/desktop.conf` (mode 0600):
//! favourites (SIP addresses and group titles), high contrast, scale, the side panel.
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
        && (commscraft_ui::normalise_address(v).as_deref() == Some(v) || v.starts_with("group:"))
}

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
            "context_open" => p.context_open = v != "false",
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

pub fn load() -> Prefs {
    let Some(path) = path() else {
        return Prefs::default();
    };
    let ok = std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= MAX_FILE);
    if !ok {
        return Prefs::default();
    }
    std::fs::read_to_string(&path)
        .map(|t| parse(&t))
        .unwrap_or_default()
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
    fn round_trip_and_hostile_lines() {
        let mut p = Prefs {
            high_contrast: true,
            scale: 1.25,
            ..Default::default()
        };
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
