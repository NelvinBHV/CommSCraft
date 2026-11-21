//! Embeds the fonts of craft-fonts (https://github.com/storytold/craft-fonts) when
//! `CRAFT_FONTS_DIR` names a checkout, following its `docs/integration.md`: the main manifest
//! (script and symbol fallbacks) and, from the Latin manifest, Inter (the interface face).
//! Unset: nothing is embedded and the system fonts are used. `CRAFT_FONTS_REQUIRED=1` (release
//! builds) turns a missing or unusable checkout into a build error.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Manifests read, each with a filter of the families taken from it (empty: all).
const MANIFESTS: &[(&str, &[&str])] = &[
    ("fonts/manifest.txt", &[]),
    ("fonts/latin-manifest.txt", &["Inter"]),
];

fn main() {
    println!("cargo::rerun-if-env-changed=CRAFT_FONTS_DIR");
    println!("cargo::rerun-if-env-changed=CRAFT_FONTS_REQUIRED");
    let mut src = String::from("pub static CRAFT_FONTS: &[CraftFont] = &[\n");
    let required = std::env::var_os("CRAFT_FONTS_REQUIRED").is_some();
    if let Some(dir) = std::env::var_os("CRAFT_FONTS_DIR").map(PathBuf::from) {
        let entries: Result<String, String> = MANIFESTS
            .iter()
            .map(|(rel, families)| craft_fonts(&dir, rel, families))
            .collect();
        match entries {
            Ok(e) => src.push_str(&e),
            Err(e) if required => println!("cargo::error=CRAFT_FONTS_DIR={}: {e}", dir.display()),
            Err(e) => println!(
                "cargo::warning=building without craft-fonts: CRAFT_FONTS_DIR={}: {e}",
                dir.display()
            ),
        }
    } else if required {
        println!("cargo::error=CRAFT_FONTS_REQUIRED is set but CRAFT_FONTS_DIR is missing");
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("craft_fonts.rs");
    if let Err(e) = std::fs::write(&out, src) {
        println!("cargo::error=writing {}: {e}", out.display());
    }
}

/// One `CraftFont { .. }` initialiser per manifest line of the wanted families.
fn craft_fonts(dir: &Path, rel: &str, families: &[&str]) -> Result<String, String> {
    let manifest = dir.join(rel);
    println!("cargo::rerun-if-changed={}", manifest.display());
    let text =
        std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let mut out = String::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let f: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [family, style, file, scripts, ..] = f.as_slice() else {
            return Err(format!("malformed manifest line: {line}"));
        };
        if !families.is_empty() && !families.contains(family) {
            continue;
        }
        let path = dir
            .join(file)
            .canonicalize()
            .map_err(|e| format!("{file}: {e}"))?;
        println!("cargo::rerun-if-changed={}", path.display());
        let scripts: Vec<String> = scripts
            .split(',')
            .map(|s| format!("{:?}", s.trim()))
            .collect();
        let _ = writeln!(
            out,
            "    CraftFont {{ family: {family:?}, style: {style:?}, scripts: &[{}], bytes: include_bytes!({:?}) }},",
            scripts.join(", "),
            path.display().to_string(),
        );
    }
    Ok(out)
}
