//! RTCP feedback of H.264 UC video ([MS-RTP] 20250520 §2.2.12, pp. 27–32; §3.2.2, p. 41; RT-25,
//! RT-26; [RFC4585] §6; reduced size per [RFC5506] §4.1): picture loss indication (standard and
//! extended with request ID and sync-frame bits per priority ID) and the video source request
//! (PSFB FMT 15, AFB type 1) with up to 20 entries of 0x44 bytes, plus the retransmission
//! schedule (fast 190 ms × 4, then for a VSR slow 3 s × 5).

use super::{be16, be32, Error};

/// Payload-specific feedback ([RFC4585] §6.1).
pub const PT_PSFB: u8 = 206;
pub const FMT_PLI: u8 = 1;
pub const FMT_AFB: u8 = 15;
/// AFB type of the video source request.
pub const AFB_VSR: u16 = 1;
/// Requested MSI: no source / the sender chooses (§2.2.12.2, p. 29).
pub const SOURCE_NONE: u32 = 0xffff_ffff;
pub const SOURCE_ANY: u32 = 0xffff_fffe;
/// Most entries in one VSR and the size of one entry.
pub const MAX_VSR_ENTRIES: usize = 20;
pub const VSR_ENTRY_LEN: usize = 0x44;
const VSR_HEADER_LEN: usize = 20;
/// Frame rate bits of a VSR entry (bit 4 = 30 fps).
pub const FPS_7_5: u32 = 1 << 0;
pub const FPS_15: u32 = 1 << 2;
pub const FPS_30: u32 = 1 << 4;
/// VSR entry flag: only (scalable) constrained baseline profile.
pub const FLAG_CONSTRAINED_BASELINE: u8 = 1 << 1;
/// Aspect ratio bits: 4:3 and 16:9.
pub const ASPECT_4_3: u8 = 1 << 0;
pub const ASPECT_16_9: u8 = 1 << 1;

/// A picture loss indication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pli {
    pub sender_ssrc: u32,
    pub media_ssrc: u32,
    /// Extended PLI: request ID and the sync-frame request bits (bit n = priority ID n).
    pub extended: Option<(u16, u64)>,
}

/// One set of video parameters a receiver asks for (§2.2.12.2, pp. 29–32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VsrEntry {
    pub payload_type: u8,
    /// Maximum UCConfig mode supported; only 1 is allowed.
    pub ucconfig_mode: u8,
    pub flags: u8,
    pub aspect_ratios: u8,
    pub max_width: u16,
    pub max_height: u16,
    pub min_bitrate: u32,
    pub bitrate_per_level: u32,
    pub bitrate_histogram: [u16; 10],
    pub frame_rates: u32,
    pub must_instances: u16,
    pub may_instances: u16,
    pub quality_histogram: [u16; 8],
    pub max_pixels: u32,
}

/// A video source request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vsr {
    pub sender_ssrc: u32,
    pub media_ssrc: u32,
    /// MSI of the requested source, or [`SOURCE_NONE`] / [`SOURCE_ANY`].
    pub msi: u32,
    pub request_id: u16,
    pub key_frame: bool,
    pub entries: Vec<VsrEntry>,
}

fn header(fmt: u8, words_after_first: usize, sender: u32, media: u32) -> Result<Vec<u8>, Error> {
    let len = u16::try_from(words_after_first).map_err(|_| Error::TooLarge)?;
    let mut p = Vec::with_capacity(12);
    p.push(0x80 | fmt);
    p.push(PT_PSFB);
    p.extend_from_slice(&len.to_be_bytes());
    p.extend_from_slice(&sender.to_be_bytes());
    p.extend_from_slice(&media.to_be_bytes());
    Ok(p)
}

impl Pli {
    /// The PLI packet (reduced size: on its own, [RFC5506]).
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        match self.extended {
            None => header(FMT_PLI, 2, self.sender_ssrc, self.media_ssrc),
            Some((id, sfr)) => {
                let mut p = header(FMT_PLI, 5, self.sender_ssrc, self.media_ssrc)?;
                p.extend_from_slice(&id.to_be_bytes());
                p.extend_from_slice(&[0, 0]);
                // SFR0 holds priority IDs 0 … 7 (its most significant bit is ID 7), and so on.
                p.extend_from_slice(&sfr.to_le_bytes());
                Ok(p)
            }
        }
    }
}

impl VsrEntry {
    fn write(&self, p: &mut Vec<u8>) {
        p.extend_from_slice(&[
            self.payload_type,
            self.ucconfig_mode,
            self.flags,
            self.aspect_ratios,
        ]);
        p.extend_from_slice(&self.max_width.to_be_bytes());
        p.extend_from_slice(&self.max_height.to_be_bytes());
        p.extend_from_slice(&self.min_bitrate.to_be_bytes());
        p.extend_from_slice(&[0; 4]);
        p.extend_from_slice(&self.bitrate_per_level.to_be_bytes());
        for v in self.bitrate_histogram {
            p.extend_from_slice(&v.to_be_bytes());
        }
        p.extend_from_slice(&self.frame_rates.to_be_bytes());
        p.extend_from_slice(&self.must_instances.to_be_bytes());
        p.extend_from_slice(&self.may_instances.to_be_bytes());
        for v in self.quality_histogram {
            p.extend_from_slice(&v.to_be_bytes());
        }
        p.extend_from_slice(&self.max_pixels.to_be_bytes());
    }

    fn read(e: &[u8]) -> Result<VsrEntry, Error> {
        let b = |i: usize| e.get(i).copied().ok_or(Error::Malformed);
        let mut bitrate_histogram = [0u16; 10];
        for (k, v) in bitrate_histogram.iter_mut().enumerate() {
            *v = be16(e, 20 + 2 * k)?;
        }
        let mut quality_histogram = [0u16; 8];
        for (k, v) in quality_histogram.iter_mut().enumerate() {
            *v = be16(e, 48 + 2 * k)?;
        }
        Ok(VsrEntry {
            payload_type: b(0)?,
            ucconfig_mode: b(1)?,
            flags: b(2)?,
            aspect_ratios: b(3)?,
            max_width: be16(e, 4)?,
            max_height: be16(e, 6)?,
            min_bitrate: be32(e, 8)?,
            bitrate_per_level: be32(e, 16)?,
            bitrate_histogram,
            frame_rates: be32(e, 40)?,
            must_instances: be16(e, 44)?,
            may_instances: be16(e, 46)?,
            quality_histogram,
            max_pixels: be32(e, 64)?,
        })
    }
}

impl Vsr {
    /// The VSR packet (PSFB FMT 15, AFB type 1).
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        if self.entries.len() > MAX_VSR_ENTRIES {
            return Err(Error::TooLarge);
        }
        let fci = VSR_HEADER_LEN + VSR_ENTRY_LEN * self.entries.len();
        let mut p = header(FMT_AFB, 2 + fci / 4, self.sender_ssrc, self.media_ssrc)?;
        p.extend_from_slice(&AFB_VSR.to_be_bytes());
        let len = u16::try_from(fci).map_err(|_| Error::TooLarge)?;
        p.extend_from_slice(&len.to_be_bytes());
        p.extend_from_slice(&self.msi.to_be_bytes());
        p.extend_from_slice(&self.request_id.to_be_bytes());
        p.extend_from_slice(&[0, 0]);
        let n = u8::try_from(self.entries.len()).map_err(|_| Error::TooLarge)?;
        p.extend_from_slice(&[
            0,
            if self.key_frame { 0x80 } else { 0 },
            n,
            VSR_ENTRY_LEN as u8,
        ]);
        p.extend_from_slice(&[0; 4]);
        for e in &self.entries {
            e.write(&mut p);
        }
        Ok(p)
    }
}

/// A feedback message read from an RTCP packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Feedback {
    Pli(Pli),
    Vsr(Vsr),
}

/// Reads one PSFB packet (`pkt` is the whole packet including its header); other feedback types
/// give `None`.
pub fn parse(pkt: &[u8]) -> Result<Option<Feedback>, Error> {
    let fmt = pkt.first().ok_or(Error::Malformed)? & 0x1f;
    let sender_ssrc = be32(pkt, 4)?;
    let media_ssrc = be32(pkt, 8)?;
    let fci = pkt.get(12..).ok_or(Error::Malformed)?;
    match fmt {
        FMT_PLI => {
            let extended = if fci.len() >= 12 {
                let sfr: [u8; 8] = fci
                    .get(4..12)
                    .and_then(|b| b.try_into().ok())
                    .ok_or(Error::Malformed)?;
                Some((be16(fci, 0)?, u64::from_le_bytes(sfr)))
            } else {
                None
            };
            Ok(Some(Feedback::Pli(Pli {
                sender_ssrc,
                media_ssrc,
                extended,
            })))
        }
        FMT_AFB if be16(fci, 0)? == AFB_VSR => {
            let length = usize::from(be16(fci, 2)?);
            let fci = fci.get(..length).ok_or(Error::Malformed)?;
            let count = usize::from(*fci.get(14).ok_or(Error::Malformed)?);
            let entry_len = usize::from(*fci.get(15).ok_or(Error::Malformed)?);
            if count > MAX_VSR_ENTRIES || (count > 0 && entry_len < VSR_ENTRY_LEN) {
                return Err(Error::Malformed);
            }
            let mut entries = Vec::with_capacity(count);
            for k in 0..count {
                let at = VSR_HEADER_LEN + k * entry_len;
                let e = fci.get(at..at + entry_len).ok_or(Error::Malformed)?;
                entries.push(VsrEntry::read(e)?);
            }
            Ok(Some(Feedback::Vsr(Vsr {
                sender_ssrc,
                media_ssrc,
                msi: be32(fci, 4)?,
                request_id: be16(fci, 8)?,
                key_frame: fci.get(13).is_some_and(|b| b & 0x80 != 0),
                entries,
            })))
        }
        _ => Ok(None),
    }
}

/// Fast retransmission interval and count, slow interval and count (§3.2.2, p. 41; RT-26).
pub const FAST_MS: u64 = 190;
pub const FAST_COUNT: u32 = 4;
pub const SLOW_MS: u64 = 3000;
pub const SLOW_COUNT: u32 = 5;

/// When a feedback message is sent again: once at once, then 4 times every 190 ms, then – for a
/// VSR only – 5 times every 3 s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retransmit {
    next_ms: u64,
    sent: u32,
    vsr: bool,
}

impl Retransmit {
    /// A message first sent at `now_ms`.
    pub fn new(now_ms: u64, vsr: bool) -> Retransmit {
        Retransmit {
            next_ms: now_ms,
            sent: 0,
            vsr,
        }
    }

    /// True if the message is due now (and counts the sending).
    pub fn due(&mut self, now_ms: u64) -> bool {
        let total = 1 + FAST_COUNT + if self.vsr { SLOW_COUNT } else { 0 };
        if self.sent >= total || now_ms < self.next_ms {
            return false;
        }
        self.sent += 1;
        let step = if self.sent <= FAST_COUNT {
            FAST_MS
        } else {
            SLOW_MS
        };
        self.next_ms = now_ms.saturating_add(step);
        true
    }

    pub fn finished(&self) -> bool {
        self.sent >= 1 + FAST_COUNT + if self.vsr { SLOW_COUNT } else { 0 }
    }
}
