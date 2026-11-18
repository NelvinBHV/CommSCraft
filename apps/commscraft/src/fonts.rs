//! The system's fonts: CommSCraft ships no font files. fontconfig (`fc-match`) names the files of
//! the preferred sans and monospace faces; a few common paths are the fallback.

use eframe::egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Largest font file read (a full CJK face is about 20 MiB).
const MAX_FONT: u64 = 32 * 1024 * 1024;

const SANS: &[&str] = &[
    "Inter",
    "Noto Sans",
    "DejaVu Sans",
    "Liberation Sans",
    "sans-serif",
];
const MONO: &[&str] = &[
    "DejaVu Sans Mono",
    "Noto Sans Mono",
    "Liberation Mono",
    "monospace",
];
/// Emoji for emoticons (outline faces only: egui draws no colour bitmaps).
const EMOJI: &[&str] = &["Noto Emoji", "Symbola", "OpenMoji"];
const FALLBACK: &[&str] = &[
    "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
];

fn fc_match(pattern: &str, style: &str) -> Option<String> {
    let out = std::process::Command::new("fc-match")
        .args(["--format=%{file}", &format!("{pattern}:style={style}")])
        .output()
        .ok()?;
    let path = String::from_utf8(out.stdout).ok()?;
    let path = path.trim();
    (out.status.success() && !path.is_empty()).then(|| path.to_string())
}

/// The files of a family as fontconfig lists them (sorted: the plain face before the others).
fn fc_list(family: &str) -> Vec<String> {
    let Ok(out) = std::process::Command::new("fc-list")
        .args(["--format=%{file}\n", family])
        .output()
    else {
        return Vec::new();
    };
    let mut files: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .take(32)
        .collect();
    // "Regular" first, the variable font last.
    files.sort_by_key(|f| {
        let l = f.to_ascii_lowercase();
        (!l.contains("regular"), l.contains("variable"), l)
    });
    files
}

fn read_font(path: &str) -> Option<Vec<u8>> {
    let lower = path.to_ascii_lowercase();
    // egui reads TrueType and OpenType outlines, not bitmap or Type 1 fonts.
    if !(lower.ends_with(".ttf") || lower.ends_with(".otf") || lower.ends_with(".ttc")) {
        return None;
    }
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FONT {
        return None;
    }
    std::fs::read(path).ok()
}

fn first_font(patterns: &[&str], style: &str) -> Option<(String, Vec<u8>)> {
    patterns
        .iter()
        .filter_map(|p| fc_match(p, style))
        .chain(FALLBACK.iter().map(|s| s.to_string()))
        .find_map(|path| read_font(&path).map(|b| (path, b)))
}

/// The font definitions with the system's sans face for text and a monospace face for code.
pub fn system_fonts() -> Result<FontDefinitions, String> {
    let (sans_path, sans) = first_font(SANS, "Regular").ok_or(
        "no usable system font found (install a sans font such as Noto Sans or DejaVu Sans)",
    )?;
    let mut defs = FontDefinitions::empty();
    defs.font_data
        .insert("system-sans".into(), Arc::new(FontData::from_owned(sans)));
    let mut mono_family = Vec::new();
    // The bold face of the same family; the regular face when fontconfig has none.
    let bold = first_font(SANS, "Bold")
        .filter(|(p, _)| *p != sans_path)
        .map_or_else(
            || "system-sans".to_string(),
            |(_, b)| {
                defs.font_data
                    .insert("system-bold".into(), Arc::new(FontData::from_owned(b)));
                "system-bold".to_string()
            },
        );
    defs.families.insert(
        FontFamily::Name(commscraft_ui::theme::BOLD.into()),
        vec![bold, "system-sans".into()],
    );
    if let Some((_, mono)) = first_font(MONO, "Regular") {
        defs.font_data
            .insert("system-mono".into(), Arc::new(FontData::from_owned(mono)));
        mono_family.push("system-mono".to_string());
    }
    mono_family.push("system-sans".to_string());
    let mut proportional = vec!["system-sans".to_string()];
    // An emoji face as fallback, only if fontconfig really names one (no common-path guess).
    let emoji = EMOJI
        .iter()
        .flat_map(|p| fc_list(p))
        .filter(|p| {
            let l = p.to_ascii_lowercase();
            l.contains("emoji") || l.contains("symbola") || l.contains("openmoji")
        })
        .filter(|p| !p.to_ascii_lowercase().contains("color"))
        .find_map(|p| read_font(&p));
    if let Some(e) = emoji {
        defs.font_data
            .insert("system-emoji".into(), Arc::new(FontData::from_owned(e)));
        proportional.push("system-emoji".into());
    }
    defs.families.insert(FontFamily::Proportional, proportional);
    defs.families.insert(FontFamily::Monospace, mono_family);
    Ok(defs)
}
