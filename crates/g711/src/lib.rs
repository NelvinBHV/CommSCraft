//! G.711 PCM of voice frequencies: A-law and µ-law (RTP payload types 8 PCMA and 0 PCMU).
//!
//! Evidence: `docs/protocol/evidence/17-payload-formats-video-audio-appsharing.md` (G7-01 …
//! G7-05). Source: [ITU-T G.711] (11/1988) §3 (p. 1), Tables 1a/1b (A-law, pp. 3–4), Tables
//! 2a/2b (µ-law, pp. 5–6), Tables 3/4 (µ-A / A-µ conversion, pp. 7–9), Tables 5/6 (digital
//! milliwatt, p. 10). RTP use: [MS-RTP] §2.2.1 (pp. 14–15), RFC 3551 §4.5.14.
//!
//! The decision values `x_n` are generated from the segment structure of the tables (column 2:
//! number of intervals × interval size) and checked against every segment end point in the tests;
//! the decoder output is `y_n` of Note 3. The scaling between 16-bit linear PCM and the uniform
//! code (A-law: 13 bit, ±4096; µ-law: 14 bit, ±8159) is left to the equipment by §3.6; we use
//! the binary shifts (×8 / ×4) – inference.
//!
//! Pure functions without allocation per sample; all lookups are bounded (no panics).

/// A-law decision value `x_n` for n = 0 … 128 (Table 1a column 5; x_128 = 4096 is virtual).
fn alaw_x(n: u32) -> i32 {
    let n = n.min(128) as i32;
    if n <= 32 {
        // Segment 1: 32 intervals of 2.
        2 * n
    } else {
        // Segments 2 … 7: 16 intervals of 2^seg, starting at 2^(seg+4).
        let seg = (n - 33) / 16 + 2;
        let first = 32 + 16 * (seg - 2);
        (1 << (seg + 4)) + (n - first) * (1 << seg)
    }
}

/// µ-law decision value `x_n` for n = 0 … 128 (Table 2a column 5; x_128 = 8159 is virtual).
fn ulaw_x(n: u32) -> i32 {
    let n = n.min(128) as i32;
    if n == 0 {
        0
    } else if n <= 16 {
        // Segment 1: 1 × 1, then 15 intervals of 2.
        2 * n - 1
    } else {
        // Segments 2 … 8: 16 intervals of 2^seg, starting at 2^(seg+4) − 33.
        let seg = (n - 17) / 16 + 2;
        let first = 16 * (seg - 1);
        (1 << (seg + 4)) - 33 + (n - first) * (1 << seg)
    }
}

/// A-law quantized value `y_n` (n = 1 … 128), Note 3 of Table 1a: (x_{n−1} + x_n) / 2.
fn alaw_y(n: u32) -> i32 {
    (alaw_x(n.saturating_sub(1)) + alaw_x(n)) / 2
}

/// µ-law quantized value `y_n` (n = 0 … 127), Note 3 of Table 2a: y_0 = 0, (x_n + x_{n+1}) / 2.
fn ulaw_y(n: u32) -> i32 {
    if n == 0 {
        0
    } else {
        (ulaw_x(n) + ulaw_x(n + 1)) / 2
    }
}

/// Even-bit inversion of A-law character signals (Table 1a Note 2): bits 2, 4, 6, 8.
const ALAW_EVEN_BITS: u8 = 0x55;

/// Decodes one A-law octet to the uniform code (13-bit range, ±4032).
pub fn alaw_to_uniform(code: u8) -> i16 {
    let c = code ^ ALAW_EVEN_BITS;
    // Positive: character 128 + (n − 1) ↔ value number n (Table 1a); negative: character n − 1.
    let n = u32::from(c & 0x7f) + 1;
    let y = alaw_y(n) as i16;
    if c & 0x80 != 0 {
        y
    } else {
        -y
    }
}

/// Encodes a uniform-code value (13-bit range) to A-law: the value number n of the interval
/// x_{n−1} ≤ |v| < x_n (Table 1a/1b column 4; the decision value itself belongs to the upper
/// interval – inference).
pub fn uniform_to_alaw(v: i16) -> u8 {
    let mag = i32::from(v).unsigned_abs().min(4095) as i32;
    // Smallest n with mag < x_n (n = 1 … 128).
    let n = (1..=128u32).find(|&n| mag < alaw_x(n)).unwrap_or(128);
    let c = (n - 1) as u8;
    let c = if v >= 0 { 0x80 | c } else { c };
    c ^ ALAW_EVEN_BITS
}

/// Decodes one µ-law octet to the uniform code (14-bit range, ±8031).
pub fn ulaw_to_uniform(code: u8) -> i16 {
    // Positive: character 255 − n; negative: character 127 − n (Tables 2a/2b, Note 2).
    let n = u32::from(0x7f - (code & 0x7f));
    let y = ulaw_y(n) as i16;
    if code & 0x80 != 0 {
        y
    } else {
        -y
    }
}

/// Encodes a uniform-code value (14-bit range) to µ-law: value number n of the interval
/// x_n ≤ |v| < x_{n+1} (Table 2a/2b column 4).
pub fn uniform_to_ulaw(v: i16) -> u8 {
    let mag = i32::from(v).unsigned_abs().min(8158) as i32;
    let n = (0..128u32).rev().find(|&n| mag >= ulaw_x(n)).unwrap_or(0);
    let c = (0x7f - n) as u8;
    if v >= 0 {
        0x80 | c
    } else {
        c
    }
}

/// 16-bit linear PCM → A-law octet (uniform code = sample / 8).
pub fn encode_alaw(sample: i16) -> u8 {
    uniform_to_alaw(sample >> 3)
}

/// A-law octet → 16-bit linear PCM.
pub fn decode_alaw(code: u8) -> i16 {
    alaw_to_uniform(code).saturating_mul(8)
}

/// 16-bit linear PCM → µ-law octet (uniform code = sample / 4).
pub fn encode_ulaw(sample: i16) -> u8 {
    uniform_to_ulaw(sample >> 2)
}

/// µ-law octet → 16-bit linear PCM.
pub fn decode_ulaw(code: u8) -> i16 {
    ulaw_to_uniform(code).saturating_mul(4)
}

/// The two laws as RTP payloads (RFC 3551: PCMU = 0, PCMA = 8; 8000 Hz, one octet per sample).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Law {
    Ulaw,
    Alaw,
}

impl Law {
    pub fn payload_type(self) -> u8 {
        match self {
            Law::Ulaw => 0,
            Law::Alaw => 8,
        }
    }

    pub fn encode(self, pcm: &[i16], out: &mut Vec<u8>) {
        out.extend(pcm.iter().map(|&s| match self {
            Law::Ulaw => encode_ulaw(s),
            Law::Alaw => encode_alaw(s),
        }));
    }

    pub fn decode(self, payload: &[u8], out: &mut Vec<i16>) {
        out.extend(payload.iter().map(|&c| match self {
            Law::Ulaw => decode_ulaw(c),
            Law::Alaw => decode_alaw(c),
        }));
    }
}

#[cfg(test)]
mod tests;
