//! The dictionary for spell checking in the composer: Hunspell `.aff`/`.dic` files read with
//! `zspell` (pure Rust), from the XDG data directories (`$XDG_DATA_HOME`, then
//! `$XDG_DATA_DIRS`, default `/usr/local/share:/usr/share`), in `hunspell/` or `myspell/`. The
//! Flatpak brings de_DE and en_US in `/app/share/hunspell`, which is in its `XDG_DATA_DIRS`.
//!
//! Security by design: only files of the expected names are read, each at most [`MAX_FILE`];
//! the dictionary is built in a thread of its own, so a large one does not hold the interface.

use std::path::PathBuf;
use std::sync::Arc;

use commscraft_ui::spell::Speller;
use commscraft_ui::text::Lang;

/// Largest dictionary file read (design; de_DE.dic is about 4 MB).
const MAX_FILE: u64 = 32 * 1024 * 1024;
/// Suggestions offered at most.
const MAX_SUGGESTIONS: usize = 8;

struct Hunspell(zspell::Dictionary);

impl Speller for Hunspell {
    fn check(&self, word: &str) -> bool {
        // A word in capitals (an abbreviation) is not checked.
        word.chars().all(|c| !c.is_lowercase()) || self.0.check_word(word)
    }

    fn suggest(&self, word: &str) -> Vec<String> {
        // Swapped neighbouring letters first (the most common typing slip; zspell's
        // suggestions only look at one edit), then zspell's.
        let chars: Vec<char> = word.chars().collect();
        let mut out: Vec<String> = (1..chars.len())
            .filter_map(|i| {
                let mut c = chars.clone();
                c.swap(i - 1, i);
                let w: String = c.into_iter().collect();
                self.0.check_word(&w).then_some(w)
            })
            .collect();
        for w in self.0.entry(word).suggest().unwrap_or_default() {
            if !out.iter().any(|o| o == w) {
                out.push(w.to_string());
            }
        }
        out.truncate(MAX_SUGGESTIONS);
        out
    }
}

/// The dictionary names for a language, in order of preference.
fn names(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::De => &["de_DE", "de_DE_frami", "de_AT", "de_CH"],
        Lang::En => &["en_US", "en_GB"],
    }
}

fn data_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(h) = std::env::var_os("XDG_DATA_HOME") {
        out.push(PathBuf::from(h));
    } else if let Some(home) = std::env::var_os("HOME") {
        out.push(PathBuf::from(home).join(".local/share"));
    }
    let dirs = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    out.extend(
        dirs.split(':')
            .filter(|d| d.starts_with('/'))
            .map(PathBuf::from),
    );
    out
}

fn read(path: &std::path::Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    // Many Hunspell dictionaries are in ISO 8859-1/-15 (`SET` in the .aff): read as Latin-1
    // where the bytes are no UTF-8 (the euro sign of -15 does not occur in word lists).
    Some(match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => e.into_bytes().iter().map(|&b| char::from(b)).collect(),
    })
}

/// Finds and builds the dictionary for a language; `None` without one.
fn load(lang: Lang) -> Option<Arc<dyn Speller>> {
    for dir in data_dirs() {
        for sub in ["hunspell", "myspell"] {
            for name in names(lang) {
                let base = dir.join(sub).join(name);
                let (Some(aff), Some(dic)) = (
                    read(&base.with_extension("aff")),
                    read(&base.with_extension("dic")),
                ) else {
                    continue;
                };
                if let Ok(d) = zspell::builder().config_str(&aff).dict_str(&dic).build() {
                    return Some(Arc::new(Hunspell(d)));
                }
            }
        }
    }
    None
}

/// Builds the dictionary in the background; the result arrives through the receiver.
pub fn start(lang: Lang) -> std::sync::mpsc::Receiver<Option<Arc<dyn Speller>>> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("spell".into())
        .spawn(move || {
            let _ = tx.send(load(lang));
        });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With a German Hunspell dictionary installed: known and unknown words, suggestions.
    #[test]
    fn german_dictionary_when_installed() {
        let started = std::time::Instant::now();
        let Some(d) = load(Lang::De) else {
            eprintln!("no German dictionary installed – skipped");
            return;
        };
        eprintln!("built in {:?}", started.elapsed());
        assert!(d.check("Haus") && d.check("Häuser") && d.check("schreiben"));
        assert!(!d.check("Hasu"));
        assert!(d.check("ABC"), "abbreviations are not checked");
        let t = std::time::Instant::now();
        let s = d.suggest("Hasu");
        eprintln!("suggestions {s:?} in {:?}", t.elapsed());
        assert!(s.iter().any(|w| w == "Haus"));
    }
}
