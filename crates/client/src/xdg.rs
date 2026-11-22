//! Where CommSCraft keeps its files: the XDG Base Directory Specification 0.8 (`$XDG_CONFIG_HOME`,
//! `$XDG_CACHE_HOME`, `$XDG_DATA_HOME` with their default `~/.config`, `~/.cache`,
//! `~/.local/share`), and in each a folder `CommSCraft`, so the files are easy to find (owner's
//! rule). Earlier versions used `commscraft`; such a folder is renamed once when `CommSCraft`
//! does not exist yet.

use std::path::{Path, PathBuf};

/// The folder name inside every XDG directory.
pub const FOLDER: &str = "CommSCraft";
/// The folder name of earlier versions.
const OLD_FOLDER: &str = "commscraft";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Config,
    Cache,
    Data,
}

impl Kind {
    fn var(self) -> &'static str {
        match self {
            Kind::Config => "XDG_CONFIG_HOME",
            Kind::Cache => "XDG_CACHE_HOME",
            Kind::Data => "XDG_DATA_HOME",
        }
    }

    fn default_under_home(self) -> &'static str {
        match self {
            Kind::Config => ".config",
            Kind::Cache => ".cache",
            Kind::Data => ".local/share",
        }
    }
}

/// The base directory of a kind; a relative `$XDG_*` is ignored (the specification: "invalid").
fn base(kind: Kind) -> Option<PathBuf> {
    std::env::var_os(kind.var())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(kind.default_under_home()))
        })
}

/// `<base>/CommSCraft`, after moving a folder of earlier versions there.
pub fn dir(kind: Kind) -> Option<PathBuf> {
    let base = base(kind)?;
    migrate(&base);
    Some(base.join(FOLDER))
}

/// Renames `<base>/commscraft` to `<base>/CommSCraft` once (a real folder only, no link).
fn migrate(base: &Path) {
    let new = base.join(FOLDER);
    let old = base.join(OLD_FOLDER);
    let is_dir = |p: &Path| std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir());
    if !new.exists() && is_dir(&old) {
        let _ = std::fs::rename(&old, &new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_folder_is_moved_once() {
        let base = std::env::temp_dir().join(format!("cs-xdg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join(OLD_FOLDER)).unwrap();
        std::fs::write(base.join(OLD_FOLDER).join("desktop.conf"), "x").unwrap();
        migrate(&base);
        assert!(base.join(FOLDER).join("desktop.conf").exists());
        assert!(!base.join(OLD_FOLDER).exists());
        // An existing new folder is never replaced.
        std::fs::create_dir_all(base.join(OLD_FOLDER)).unwrap();
        migrate(&base);
        assert!(base.join(OLD_FOLDER).exists());
        assert!(base.join(FOLDER).join("desktop.conf").exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
