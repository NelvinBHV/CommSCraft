//! Audio device adapter (L2) for Linux: microphone and speaker through the ALSA API.
//!
//! Decision D-11 (owner, 2026-10-28): ALSA/PipeWire. On PipeWire systems the ALSA device
//! `default` is routed to PipeWire by its ALSA plugin (pipewire-alsa), so one backend covers both;
//! sample-rate conversion is left to ALSA `plug`/PipeWire. Decision D-03: system libraries only
//! behind adapter crates – this is the only crate that links libasound (through the safe API of
//! the `alsa` crate; `unsafe_code` stays forbidden here as everywhere).
//!
//! Security by design and robustness: the device name is checked; only mono 16-bit PCM at the
//! codec rates 8000/16000 Hz with frames of 10 … 60 ms; the negotiated rate must equal the
//! requested one (no silent mismatch); buffers are bounded (4 frames); under-/overruns are
//! recovered (`snd_pcm_recover`) instead of failing the call. The adapter only moves samples; it
//! never interprets data from the network.

use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::{Direction, ValueOr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadConfig(&'static str),
    /// The device could not be opened or configured (ALSA error text).
    Device(String),
    /// The device runs at another rate than requested.
    Rate {
        wanted: u32,
        got: u32,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BadConfig(w) => write!(f, "audio configuration: {w}"),
            Error::Device(e) => write!(f, "audio device: {e}"),
            Error::Rate { wanted, got } => {
                write!(f, "audio device runs at {got} Hz, not {wanted} Hz")
            }
        }
    }
}

fn dev(e: alsa::Error) -> Error {
    Error::Device(e.to_string())
}

/// Device and frame layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// ALSA device name; `default` follows the system (PipeWire on current desktops).
    pub device: String,
    /// 8000 (G.711) or 16000 (G.722).
    pub rate: u32,
    /// Frame duration in milliseconds (RTP packet time, [MS-RTP] §2.2.1: 20 ms usual).
    pub frame_ms: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            device: "default".into(),
            rate: 16000,
            frame_ms: 20,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), Error> {
        let name_ok = !self.device.is_empty()
            && self.device.len() <= 128
            && self
                .device
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ":,._-=".contains(c));
        if !name_ok {
            return Err(Error::BadConfig("device name"));
        }
        if !matches!(self.rate, 8000 | 16000) {
            return Err(Error::BadConfig("rate must be 8000 or 16000"));
        }
        if !matches!(self.frame_ms, 10 | 20 | 30 | 40 | 60) {
            return Err(Error::BadConfig("frame must be 10, 20, 30, 40 or 60 ms"));
        }
        Ok(())
    }

    /// Samples per frame (mono).
    pub fn frame_samples(&self) -> usize {
        (self.rate / 1000 * self.frame_ms) as usize
    }
}

fn open(cfg: &Config, dir: Direction) -> Result<PCM, Error> {
    cfg.validate()?;
    let pcm = PCM::new(&cfg.device, dir, false).map_err(dev)?;
    {
        let hw = HwParams::any(&pcm).map_err(dev)?;
        hw.set_channels(1).map_err(dev)?;
        hw.set_rate(cfg.rate, ValueOr::Nearest).map_err(dev)?;
        hw.set_format(Format::s16()).map_err(dev)?;
        hw.set_access(Access::RWInterleaved).map_err(dev)?;
        let frame = cfg.frame_samples() as alsa::pcm::Frames;
        hw.set_period_size_near(frame, ValueOr::Nearest)
            .map_err(dev)?;
        hw.set_buffer_size_near(frame * 4).map_err(dev)?;
        pcm.hw_params(&hw).map_err(dev)?;
    }
    let got = pcm
        .hw_params_current()
        .and_then(|h| h.get_rate())
        .map_err(dev)?;
    if got != cfg.rate {
        return Err(Error::Rate {
            wanted: cfg.rate,
            got,
        });
    }
    Ok(pcm)
}

/// Microphone.
pub struct Capture {
    pcm: PCM,
    frame: usize,
}

impl Capture {
    pub fn open(cfg: &Config) -> Result<Self, Error> {
        let pcm = open(cfg, Direction::Capture)?;
        pcm.start().map_err(dev)?;
        Ok(Capture {
            pcm,
            frame: cfg.frame_samples(),
        })
    }

    pub fn frame_samples(&self) -> usize {
        self.frame
    }

    /// Blocks until one frame is read; `out` is resized to the frame. An overrun is recovered
    /// and the frame filled from the restarted stream.
    pub fn read_frame(&mut self, out: &mut Vec<i16>) -> Result<(), Error> {
        out.clear();
        out.resize(self.frame, 0);
        let io = self.pcm.io_i16().map_err(dev)?;
        let mut done = 0;
        let mut recoveries = 0;
        while done < self.frame {
            let Some(rest) = out.get_mut(done..) else {
                break;
            };
            match io.readi(rest) {
                Ok(n) => done += n,
                Err(e) if recoveries < 3 => {
                    recoveries += 1;
                    self.pcm.try_recover(e, true).map_err(dev)?;
                }
                Err(e) => return Err(dev(e)),
            }
        }
        Ok(())
    }
}

/// Speaker.
pub struct Playback {
    pcm: PCM,
    frame: usize,
}

impl Playback {
    pub fn open(cfg: &Config) -> Result<Self, Error> {
        Ok(Playback {
            pcm: open(cfg, Direction::Playback)?,
            frame: cfg.frame_samples(),
        })
    }

    pub fn frame_samples(&self) -> usize {
        self.frame
    }

    /// Blocks until `samples` are queued. An underrun is recovered.
    pub fn write(&mut self, samples: &[i16]) -> Result<(), Error> {
        let io = self.pcm.io_i16().map_err(dev)?;
        let mut done = 0;
        let mut recoveries = 0;
        while done < samples.len() {
            let Some(rest) = samples.get(done..) else {
                break;
            };
            match io.writei(rest) {
                Ok(n) => done += n,
                Err(e) if recoveries < 3 => {
                    recoveries += 1;
                    self.pcm.try_recover(e, true).map_err(dev)?;
                }
                Err(e) => return Err(dev(e)),
            }
        }
        Ok(())
    }

    /// Plays out what is queued. The device stays usable: `snd_pcm_drain` leaves the PCM in the
    /// SETUP state, where a further write fails with EBADFD that `snd_pcm_recover` cannot repair,
    /// so it is prepared again (ALSA library reference, PCM state machine: drain → SETUP,
    /// prepare → PREPARED).
    pub fn drain(&mut self) -> Result<(), Error> {
        self.pcm.drain().map_err(dev)?;
        self.pcm.prepare().map_err(dev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_is_checked() {
        let ok = Config::default();
        assert_eq!(ok.validate(), Ok(()));
        assert_eq!(ok.frame_samples(), 320);
        let g711 = Config {
            rate: 8000,
            ..Config::default()
        };
        assert_eq!(g711.frame_samples(), 160);
        for bad in [
            Config {
                device: "hw:0;rm -rf".into(),
                ..Config::default()
            },
            Config {
                device: String::new(),
                ..Config::default()
            },
            Config {
                rate: 44100,
                ..Config::default()
            },
            Config {
                frame_ms: 7,
                ..Config::default()
            },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
            assert!(matches!(
                Capture::open(&bad).err(),
                Some(Error::BadConfig(_))
            ));
        }
    }
}
