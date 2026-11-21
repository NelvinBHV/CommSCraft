//! The interface fonts: Inter and the script and symbol faces of craft-fonts
//! (https://github.com/storytold/craft-fonts, embedded at build time when `CRAFT_FONTS_DIR` is
//! set, see `build.rs`), Noto Emoji for emoji, the Lucide icon font; without craft-fonts the
//! system's sans face (fontconfig `fc-match`, a few common paths as fallback). Monospace from
//! the system.

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
/// A font from craft-fonts (empty unless built with `CRAFT_FONTS_DIR`, see `build.rs`).
pub struct CraftFont {
    pub family: &'static str,
    pub style: &'static str,
    /// ISO 15924 scripts the font is for, e.g. `"Jpan"` (kept as the integration recipe has it).
    #[allow(dead_code)]
    pub scripts: &'static [&'static str],
    pub bytes: &'static [u8],
}
include!(concat!(env!("OUT_DIR"), "/craft_fonts.rs"));

/// The emoji face (Noto Emoji, OFL-1.1, google/fonts; static Regular instance, see
/// ATTRIBUTION.md): craft-fonts has symbol faces but no emoji face for faces like 🙂 or 👍.
const EMOJI_FONT: &[u8] = include_bytes!("../../../assets/fonts/NotoEmoji-Regular.ttf");

fn craft(family: &str, style: &str) -> Option<&'static CraftFont> {
    CRAFT_FONTS
        .iter()
        .find(|f| f.family == family && f.style == style)
}

/// The font definitions: Inter from craft-fonts for the interface (else the system's sans face),
/// a monospace face, the craft-fonts script and symbol faces as fallbacks, then Noto Emoji.
pub fn system_fonts() -> Result<FontDefinitions, String> {
    let mut defs = FontDefinitions::empty();
    let (regular, bold) = match (craft("Inter", "Regular"), craft("Inter", "Bold")) {
        (Some(r), b) => {
            defs.font_data.insert(
                "ui-regular".into(),
                Arc::new(FontData::from_static(r.bytes)),
            );
            let bold = match b {
                Some(b) => {
                    defs.font_data
                        .insert("ui-bold".into(), Arc::new(FontData::from_static(b.bytes)));
                    "ui-bold"
                }
                None => "ui-regular",
            };
            ("ui-regular".to_string(), bold.to_string())
        }
        (None, _) => {
            let (sans_path, sans) = first_font(SANS, "Regular").ok_or(
                "no usable system font found (install a sans font such as Noto Sans or DejaVu Sans)",
            )?;
            defs.font_data
                .insert("ui-regular".into(), Arc::new(FontData::from_owned(sans)));
            // The bold face of the same family; the regular face when fontconfig has none.
            let bold = first_font(SANS, "Bold")
                .filter(|(p, _)| *p != sans_path)
                .map_or_else(
                    || "ui-regular".to_string(),
                    |(_, b)| {
                        defs.font_data
                            .insert("ui-bold".into(), Arc::new(FontData::from_owned(b)));
                        "ui-bold".to_string()
                    },
                );
            ("ui-regular".to_string(), bold)
        }
    };
    // Fallbacks: every other craft-fonts face (scripts, symbols), then the emoji face.
    let mut fallbacks = Vec::new();
    for (i, f) in CRAFT_FONTS
        .iter()
        .filter(|f| f.family != "Inter")
        .filter(|f| f.style == "Regular" || f.style.is_empty())
        .enumerate()
    {
        let name = format!("craft-{i}-{}", f.family);
        defs.font_data
            .insert(name.clone(), Arc::new(FontData::from_static(f.bytes)));
        fallbacks.push(name);
    }
    defs.font_data
        .insert("emoji".into(), Arc::new(FontData::from_static(EMOJI_FONT)));
    fallbacks.push("emoji".into());
    let mut proportional = vec![regular.clone()];
    proportional.extend(fallbacks.iter().cloned());
    let mut bold_family = vec![bold, regular.clone()];
    bold_family.extend(fallbacks.iter().cloned());
    defs.families.insert(
        FontFamily::Name(commscraft_ui::theme::BOLD.into()),
        bold_family,
    );
    let mut mono_family = Vec::new();
    if let Some((_, mono)) = first_font(MONO, "Regular") {
        defs.font_data
            .insert("system-mono".into(), Arc::new(FontData::from_owned(mono)));
        mono_family.push("system-mono".to_string());
    }
    mono_family.push(regular);
    mono_family.extend(fallbacks);
    defs.families.insert(FontFamily::Proportional, proportional);
    defs.families.insert(FontFamily::Monospace, mono_family);
    // The interface icons (Lucide).
    commscraft_ui::install_icons(&mut defs);
    Ok(defs)
}
