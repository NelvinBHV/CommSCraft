//! The sounds of the client (issues #82–#84): the owner-approved sound pack embedded in the
//! program (`assets/audio`: Pulse, Beat Call, Signal Message), a bounded reader for PCM WAV
//! files – the embedded sounds and a user's own –, band-limited resampling, level matching and
//! seamless looping.
//!
//! Sans-I/O: the caller reads a user's file (with a size limit) and plays the samples.
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

/// Zero crossings of the windowed sinc on each side, and the phases of its table.
const SINC_ZEROS: usize = 16;
const SINC_PHASES: usize = 256;

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

/// Band-limited resampling with a Hann-windowed sinc from a table of phases: nothing above the
/// lower Nyquist frequency folds back into the sound. `cyclic`: the sound repeats, so the filter
/// reads across the ends and the output has an exact whole number of samples per cycle – one
/// repetition follows the other without a gap or a drift against its own beat.
fn resampled(pcm: &Pcm, to: u32, cyclic: bool) -> Vec<i16> {
    let n_in = pcm.samples.len();
    if pcm.rate == to || n_in == 0 || to == 0 || pcm.rate == 0 {
        return pcm.samples.clone();
    }
    let n_out =
        ((n_in as u64 * u64::from(to) + u64::from(pcm.rate) / 2) / u64::from(pcm.rate)).max(1);
    let step = n_in as f64 / n_out as f64;
    // Against aliasing the cut-off follows the lower of the two rates.
    let cutoff = (1.0 / step).min(1.0);
    let half = ((SINC_ZEROS as f64 / cutoff).ceil() as usize).max(1);
    let width = half as f64;
    let taps = 2 * half;
    let mut table = vec![0f64; (SINC_PHASES + 1) * taps];
    for (phase, row) in table.chunks_exact_mut(taps).enumerate() {
        let frac = phase as f64 / SINC_PHASES as f64;
        for (t, h) in row.iter_mut().enumerate() {
            // The distance of tap t from the sampling point.
            let x = t as f64 - (half as f64 - 1.0) - frac;
            let window = 0.5 + 0.5 * (PI * (x / width).clamp(-1.0, 1.0)).cos();
            *h = cutoff * sinc(cutoff * x) * window;
        }
    }
    let n = n_in as i64;
    (0..n_out)
        .map(|i| {
            let pos = i as f64 * step;
            let whole = pos.floor();
            let phase = ((pos - whole) * SINC_PHASES as f64).round() as usize;
            let row = table
                .get(phase * taps..(phase + 1) * taps)
                .unwrap_or_default();
            let first = whole as i64 - (half as i64 - 1);
            let mut sum = 0.0;
            for (t, h) in row.iter().enumerate() {
                let k = first + t as i64;
                let index = if cyclic { k.rem_euclid(n) } else { k };
                if let Some(s) = usize::try_from(index).ok().and_then(|x| pcm.samples.get(x)) {
                    sum += f64::from(*s) * h;
                }
            }
            sum.round().clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}

/// A sound played once, at another rate.
pub fn resample(pcm: &Pcm, to: u32) -> Vec<i16> {
    resampled(pcm, to, false)
}

/// A repeating sound at another rate: one cycle of a whole number of samples.
pub fn resample_loop(pcm: &Pcm, to: u32) -> Vec<i16> {
    resampled(pcm, to, true)
}

/// Scales the samples to a volume in percent (at most 100).
pub fn set_volume(samples: &mut [i16], percent: u32) {
    let percent = i32::try_from(percent.min(100)).unwrap_or(100);
    if percent == 100 {
        return;
    }
    for s in samples.iter_mut() {
        *s = (i32::from(*s) * percent / 100) as i16;
    }
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

/// Plays samples in frames, from the start again at the end. A user's file gets a short fade in
/// and out so that the loop boundary does not click.
#[derive(Debug, Clone)]
pub struct Looper {
    samples: Vec<i16>,
    pos: usize,
}

impl Looper {
    /// `fade`: fade the ends in and out (for a user's file; the sound pack's loops join by
    /// themselves).
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

/// The owner-approved sound pack (issues #82–#84, `assets/audio/README.md`), embedded so that
/// the program needs no installed files; the CI checks the files against `SHA256SUMS.txt`.
/// The masters are played as mastered: not level-matched, the loops without a fade (they were
/// made to join without a jump).
pub mod soundpack {
    use super::{parse_wav, Error, Pcm};

    const PULSE: &[u8] = include_bytes!("../../../assets/audio/CommsCraft-Pulse-hold-loop.wav");
    const BEAT_CALL: &[u8] = include_bytes!("../../../assets/audio/CommsCraft-Beat-call-loop.wav");
    const SIGNAL_MESSAGE: &[u8] =
        include_bytes!("../../../assets/audio/CommsCraft-Signal-message-unchanged.wav");

    /// Music on hold: Pulse V5, a loop of 24 bars at 112 BPM.
    pub fn pulse() -> Result<Pcm, Error> {
        parse_wav(PULSE)
    }

    /// Ringtone: Beat Call V5, a loop of 2 bars at 112 BPM.
    pub fn beat_call() -> Result<Pcm, Error> {
        parse_wav(BEAT_CALL)
    }

    /// Notification sound: Signal Message V4, played once.
    pub fn signal_message() -> Result<Pcm, Error> {
        parse_wav(SIGNAL_MESSAGE)
    }
}

#[cfg(test)]
mod tests;
