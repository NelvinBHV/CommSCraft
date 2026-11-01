//! Alert tones, ringing and music on hold (issues #82–#84): a bounded reader for PCM WAV files,
//! conversion to mono at the codec's rate, level matching, seamless looping, and tones generated
//! here – so the client needs no third-party sound file until one is cleared for distribution.
//!
//! Sans-I/O: the caller reads the file (with a size limit) and plays the samples.
//!
//! Security by design: a sound file is untrusted input – the reader walks a bounded number of
//! chunks with checked lengths, accepts only PCM with 1–2 channels, 8–48 kHz and 16 or 24 bits,
//! limits the size and the duration, and never allocates more than the data it was given.

use std::f64::consts::PI;

/// Largest sound file accepted (bytes).
pub const MAX_FILE: usize = 16 * 1024 * 1024;
/// Longest sound accepted (seconds).
pub const MAX_SECONDS: usize = 600;
/// Most RIFF chunks looked at.
const MAX_CHUNKS: usize = 64;
/// Peak level after matching (about −3 dBFS).
const PEAK: f64 = 23_000.0;
/// Fade at the loop boundary of a file (milliseconds), against clicks.
const LOOP_FADE_MS: usize = 10;

/// Mono 16-bit samples at a sample rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm {
    pub rate: u32,
    pub samples: Vec<i16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    TooLarge,
    TooLong,
    NotWav,
    /// Only PCM (format 1, or extensible with the PCM sub-format) is read.
    NotPcm,
    Unsupported(&'static str),
    Truncated,
    NoData,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::TooLarge => write!(f, "sound file larger than {MAX_FILE} bytes"),
            Error::TooLong => write!(f, "sound longer than {MAX_SECONDS} s"),
            Error::NotWav => write!(f, "not a RIFF/WAVE file"),
            Error::NotPcm => write!(f, "not PCM audio"),
            Error::Unsupported(w) => write!(f, "unsupported {w}"),
            Error::Truncated => write!(f, "truncated file"),
            Error::NoData => write!(f, "no audio data"),
        }
    }
}

fn le16(b: &[u8], at: usize) -> Result<u16, Error> {
    b.get(at..at + 2)
        .and_then(|x| x.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or(Error::Truncated)
}

fn le32(b: &[u8], at: usize) -> Result<u32, Error> {
    b.get(at..at + 4)
        .and_then(|x| x.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or(Error::Truncated)
}

/// The PCM sub-format GUID of WAVE_FORMAT_EXTENSIBLE (KSDATAFORMAT_SUBTYPE_PCM).
const PCM_GUID_TAIL: [u8; 14] = [
    0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

/// Reads a PCM WAV file (RIFF/WAVE with `fmt ` and `data` chunks) into mono samples at the
/// file's rate; stereo is averaged, 24 bits are cut to 16.
pub fn parse_wav(file: &[u8]) -> Result<Pcm, Error> {
    if file.len() > MAX_FILE {
        return Err(Error::TooLarge);
    }
    if file.get(0..4) != Some(b"RIFF") || file.get(8..12) != Some(b"WAVE") {
        return Err(Error::NotWav);
    }
    let mut at = 12usize;
    let mut format: Option<(u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    for _ in 0..MAX_CHUNKS {
        if at.saturating_add(8) > file.len() {
            break;
        }
        let id = file.get(at..at + 4).ok_or(Error::Truncated)?;
        let len = le32(file, at + 4)? as usize;
        let body_start = at + 8;
        let body_end = body_start.checked_add(len).ok_or(Error::Truncated)?;
        // A `data` chunk may claim more than the file holds (streamed writers): take what is there.
        let body = match file.get(body_start..body_end) {
            Some(b) => b,
            None if id == b"data" => file.get(body_start..).ok_or(Error::Truncated)?,
            None => return Err(Error::Truncated),
        };
        match id {
            b"fmt " => {
                let tag = le16(body, 0)?;
                let channels = le16(body, 2)?;
                let rate = le32(body, 4)?;
                let bits = le16(body, 14)?;
                let pcm = match tag {
                    1 => true,
                    0xfffe => body.get(26..40) == Some(&PCM_GUID_TAIL[..]) && le16(body, 24)? == 1,
                    _ => false,
                };
                if !pcm {
                    return Err(Error::NotPcm);
                }
                if !(1..=2).contains(&channels) {
                    return Err(Error::Unsupported("channel count"));
                }
                if !(8000..=48_000).contains(&rate) {
                    return Err(Error::Unsupported("sample rate"));
                }
                if bits != 16 && bits != 24 {
                    return Err(Error::Unsupported("sample size"));
                }
                format = Some((channels, rate, bits));
            }
            b"data" => data = Some(body),
            _ => {}
        }
        if format.is_some() && data.is_some() {
            break;
        }
        // Chunks are padded to an even length.
        at = body_end.saturating_add(len & 1);
    }
    let (channels, rate, bits) = format.ok_or(Error::NotWav)?;
    let data = data.ok_or(Error::NoData)?;
    let width = usize::from(bits / 8) * usize::from(channels);
    let frames = data.len() / width;
    if frames == 0 {
        return Err(Error::NoData);
    }
    if frames / rate as usize > MAX_SECONDS {
        return Err(Error::TooLong);
    }
    let sample = |frame: &[u8], ch: usize| -> i32 {
        let off = ch * usize::from(bits / 8);
        // 24 bits: the upper two bytes are the 16-bit value.
        let at = if bits == 16 { off } else { off + 1 };
        frame
            .get(at..at + 2)
            .and_then(|b| b.try_into().ok())
            .map_or(0, |b| i32::from(i16::from_le_bytes(b)))
    };
    let samples = data
        .chunks_exact(width)
        .map(|f| {
            let sum: i32 = (0..usize::from(channels)).map(|c| sample(f, c)).sum();
            (sum / i32::from(channels)) as i16
        })
        .collect();
    Ok(Pcm { rate, samples })
}

/// The sound at another rate (linear interpolation; enough for alerts and hold music over a
/// telephone codec).
pub fn resample(pcm: &Pcm, to: u32) -> Vec<i16> {
    if pcm.rate == to || pcm.samples.is_empty() || to == 0 {
        return pcm.samples.clone();
    }
    let out_len = (pcm.samples.len() as u64 * u64::from(to) / u64::from(pcm.rate)) as usize;
    let step = f64::from(pcm.rate) / f64::from(to);
    let last = pcm.samples.len() - 1;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * step;
            let k = (pos as usize).min(last);
            let frac = pos - k as f64;
            let a = f64::from(pcm.samples.get(k).copied().unwrap_or(0));
            let b = f64::from(pcm.samples.get((k + 1).min(last)).copied().unwrap_or(0));
            (a + (b - a) * frac) as i16
        })
        .collect()
}

/// Scales the samples so that the peak is about −3 dBFS (quiet files get louder, loud ones
/// softer; silence stays silence).
pub fn normalize(samples: &mut [i16]) {
    let peak = samples
        .iter()
        .map(|s| i32::from(*s).unsigned_abs())
        .max()
        .unwrap_or(0);
    if peak == 0 {
        return;
    }
    let gain = PEAK / f64::from(peak);
    for s in samples.iter_mut() {
        *s = (f64::from(*s) * gain).clamp(-32768.0, 32767.0) as i16;
    }
}

/// Plays samples in frames, from the start again at the end. A file gets a short fade in and
/// out so that the loop boundary does not click.
#[derive(Debug, Clone)]
pub struct Looper {
    samples: Vec<i16>,
    pos: usize,
}

impl Looper {
    /// `fade`: fade the ends in and out (for files; generated tones are silent at the ends).
    pub fn new(mut samples: Vec<i16>, rate: u32, fade: bool) -> Looper {
        if fade {
            let n = (rate as usize * LOOP_FADE_MS / 1000).min(samples.len() / 2);
            let len = samples.len();
            for i in 0..n {
                let g = i as f64 / n as f64;
                if let Some(s) = samples.get_mut(i) {
                    *s = (f64::from(*s) * g) as i16;
                }
                if let Some(s) = samples.get_mut(len - 1 - i) {
                    *s = (f64::from(*s) * g) as i16;
                }
            }
        }
        Looper { samples, pos: 0 }
    }

    /// The next `n` samples (silence if there is nothing to play).
    pub fn next_frame(&mut self, n: usize) -> Vec<i16> {
        if self.samples.is_empty() {
            return vec![0; n];
        }
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            if self.pos >= self.samples.len() {
                self.pos = 0;
            }
            let take = (n - out.len()).min(self.samples.len() - self.pos);
            out.extend_from_slice(self.samples.get(self.pos..self.pos + take).unwrap_or(&[]));
            self.pos += take;
        }
        out
    }
}

fn tone(rate: u32, ms: u32, hz: &[f64], level: f64) -> Vec<i16> {
    let n = (rate as usize * ms as usize) / 1000;
    let attack = (rate as usize * 5 / 1000).max(1);
    (0..n)
        .map(|i| {
            let t = i as f64 / f64::from(rate);
            let v: f64 = hz.iter().map(|f| (2.0 * PI * f * t).sin()).sum::<f64>() / hz.len() as f64;
            // A short attack and an exponential decay: a soft, bell-like note.
            let env = (i as f64 / attack as f64).min(1.0) * (-3.0 * i as f64 / n as f64).exp();
            (v * env * level) as i16
        })
        .collect()
}

fn silence(rate: u32, ms: u32) -> Vec<i16> {
    vec![0; (rate as usize * ms as usize) / 1000]
}

/// Our own ringtone (generated here, no third-party material): two short rising chimes, then a
/// pause – one cycle of 3 s, meant to be looped while a call is alerting.
pub fn ringtone(rate: u32) -> Vec<i16> {
    let mut s = Vec::new();
    for _ in 0..2 {
        s.extend(tone(rate, 180, &[659.25, 1318.5], PEAK));
        s.extend(tone(rate, 180, &[783.99, 1567.98], PEAK));
        s.extend(tone(rate, 260, &[987.77, 1975.5], PEAK));
        s.extend(silence(rate, 120));
    }
    s.extend(silence(rate, 3000u32.saturating_sub(2 * 740)));
    s
}

/// Our own notification sound (generated here): two short notes, played once.
pub fn notification(rate: u32) -> Vec<i16> {
    let mut s = tone(rate, 110, &[880.0], PEAK * 0.8);
    s.extend(tone(rate, 220, &[1318.5], PEAK * 0.8));
    s
}

#[cfg(test)]
mod tests;
