//! Local recording of a meeting's audio ([MS-CONFBAS] §4.4.9: the client records, the focus is
//! only told): a 16-bit mono WAV file of what the user hears with the user's own microphone
//! mixed in, in `$XDG_VIDEOS_DIR/CommSCraft` (as the Windows client keeps its recordings in the
//! Videos folder; inference about the place, [MS-CONFBAS] does not say).
//!
//! Security by design: the file is created new (never overwritten, mode 0600), its name is
//! built from the time only, and it stops at [`MAX_BYTES`].

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Largest recording (design): about 6 hours at 16 kHz.
pub const MAX_BYTES: u64 = 700 * 1024 * 1024;

pub struct Recorder {
    file: File,
    rate: u32,
    data: u64,
    /// The newest microphone frame, mixed into the next played frame.
    mic: Vec<i16>,
    pub path: PathBuf,
}

/// The folder for recordings: `$XDG_VIDEOS_DIR` (from `user-dirs.dirs`) or `~/Videos`.
pub fn folder() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let videos = std::fs::read_to_string(config.join("user-dirs.dirs"))
        .ok()
        .and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("XDG_VIDEOS_DIR="))
                .map(|v| {
                    v.trim_matches('"')
                        .replace("$HOME", &home.to_string_lossy())
                })
        })
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join("Videos"));
    Some(videos.join("CommSCraft"))
}

fn header(rate: u32, data: u32) -> [u8; 44] {
    let mut h = [0u8; 44];
    let mut put = |at: usize, b: &[u8]| {
        if let Some(d) = h.get_mut(at..at + b.len()) {
            d.copy_from_slice(b);
        }
    };
    put(0, b"RIFF");
    put(4, &data.saturating_add(36).to_le_bytes());
    put(8, b"WAVEfmt ");
    put(16, &16u32.to_le_bytes());
    put(20, &1u16.to_le_bytes()); // PCM
    put(22, &1u16.to_le_bytes()); // mono
    put(24, &rate.to_le_bytes());
    put(28, &rate.saturating_mul(2).to_le_bytes());
    put(32, &2u16.to_le_bytes());
    put(34, &16u16.to_le_bytes());
    put(36, b"data");
    put(40, &data.to_le_bytes());
    h
}

impl Recorder {
    /// A new file in `dir` named after the time (`meeting-<unix seconds>.wav`).
    pub fn create(dir: &Path, rate: u32, now_secs: u64) -> std::io::Result<Recorder> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("meeting-{now_secs}.wav"));
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        let mut file = o.open(&path)?;
        file.write_all(&header(rate, 0))?;
        Ok(Recorder {
            file,
            rate,
            data: 0,
            mic: Vec::new(),
            path,
        })
    }

    /// The microphone's newest frame.
    pub fn microphone(&mut self, frame: &[i16]) {
        self.mic.clear();
        self.mic.extend_from_slice(frame);
    }

    /// A played frame, with the microphone mixed in; `false` once the size limit is reached.
    pub fn played(&mut self, frame: &[i16]) -> bool {
        if self.data + (frame.len() as u64) * 2 > MAX_BYTES {
            return false;
        }
        let mic = std::mem::take(&mut self.mic);
        let mut bytes = Vec::with_capacity(frame.len() * 2);
        for (i, s) in frame.iter().enumerate() {
            let m = mic.get(i).copied().unwrap_or(0);
            let v = (i32::from(*s) + i32::from(m)).clamp(i32::from(i16::MIN), i32::from(i16::MAX));
            bytes.extend_from_slice(&(v as i16).to_le_bytes());
        }
        if self.file.write_all(&bytes).is_err() {
            return false;
        }
        self.data += bytes.len() as u64;
        true
    }

    /// Writes the final sizes into the header.
    pub fn finish(mut self) -> std::io::Result<PathBuf> {
        let data = u32::try_from(self.data).unwrap_or(u32::MAX);
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&header(self.rate, data))?;
        self.file.flush()?;
        Ok(self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_file_with_mixed_microphone() {
        let dir = std::env::temp_dir().join(format!("cs-rec-{}", std::process::id()));
        let mut r = Recorder::create(&dir, 16000, 1).unwrap();
        r.microphone(&[1000, -1000]);
        assert!(r.played(&[100, 30000]));
        assert!(r.played(&[5, 6]));
        let path = r.finish().unwrap();
        let b = std::fs::read(&path).unwrap();
        assert_eq!(&b[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes([b[40], b[41], b[42], b[43]]), 8);
        assert_eq!(u32::from_le_bytes([b[24], b[25], b[26], b[27]]), 16000);
        let s = |i: usize| i16::from_le_bytes([b[44 + 2 * i], b[45 + 2 * i]]);
        assert_eq!((s(0), s(1), s(2), s(3)), (1100, 29000, 5, 6));
        // Never overwritten.
        assert!(Recorder::create(&dir, 16000, 1).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
