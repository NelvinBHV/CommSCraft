//! ITU-T G.722 7 kHz audio coding at 64 kbit/s (SB-ADPCM, RTP payload type 9).
//!
//! Evidence: `docs/protocol/evidence/17-payload-formats-video-audio-appsharing.md` (G2-01 …
//! G2-06). Source: [ITU-T G.722] (09/2012): §1.4.4 octet format (p. 5), §5 QMF (pp. 21–27,
//! Table 11), §6 sub-band ADPCM (pp. 27–39, Tables 13–21, blocks 1L … 6L, 1H … 5H), Appendix II
//! digital test sequences. RTP: RFC 3551 §4.5.2 (clock rate 8000 although sampled at 16 kHz).
//!
//! The ADPCM part follows §6.2 block by block with its 16-bit saturating arithmetic and is
//! checked bit-exactly against the ITU digital test sequences (`tests.rs`, all 14 comparison
//! files, when the sequences are available locally). The QMF has implementation freedom (§5.2
//! Note 1: ≥ 24-bit accumulators, truncation or rounding) and no test sequences; we use exact
//! 32-bit accumulation and arithmetic shifts. Only the scale-factor Method 2 (32-entry table ILB)
//! is used. Only Mode 1 (64 kbit/s) is offered on the network; Modes 2 and 3 exist in the decoder
//! for the test sequences.
//!
//! No allocation per sample, bounded table lookups, no panics.

/// 16-bit saturation of an intermediate result.
fn sat(x: i32) -> i32 {
    x.clamp(-32768, 32767)
}

/// "+" and "−" of §6.2: saturating 16-bit addition and subtraction.
fn add(a: i32, b: i32) -> i32 {
    sat(a + b)
}
fn sub(a: i32, b: i32) -> i32 {
    sat(a - b)
}

/// "*" of §6.2: (A times B) >> 15.
fn mul(a: i32, b: i32) -> i32 {
    sat((a * b) >> 15)
}

/// "x >> n" of §6.2 with negative n meaning a left shift.
fn shr(x: i32, n: i32) -> i32 {
    if n >= 0 {
        x >> n.min(31)
    } else {
        sat(x << (-n).min(15))
    }
}

/// Sign as in §6.2: `x >> 15` of a 16-bit value (−1 or 0).
fn sign(x: i32) -> i32 {
    if x < 0 {
        -1
    } else {
        0
    }
}

fn at(table: &[i32], i: usize) -> i32 {
    table.get(i).copied().unwrap_or(0)
}

// ----- Table 14 ---------------------------------------------------------------------------

/// Q6(1 … 29), index 0 unused.
const Q6: [i32; 30] = [
    0, 35, 72, 110, 150, 190, 233, 276, 323, 370, 422, 473, 530, 587, 650, 714, 786, 858, 940,
    1023, 1121, 1219, 1339, 1458, 1612, 1765, 1980, 2195, 2557, 2919,
];
/// QQ6(1 … 30), index 0 unused.
const QQ6: [i32; 31] = [
    0, 17, 54, 91, 130, 170, 211, 254, 300, 347, 396, 447, 501, 558, 618, 682, 750, 822, 899, 982,
    1072, 1170, 1279, 1399, 1535, 1689, 1873, 2088, 2376, 2738, 3101,
];
/// QQ5(1 … 15), index 0 unused.
const QQ5: [i32; 16] = [
    0, 35, 110, 190, 276, 370, 473, 587, 714, 858, 1023, 1219, 1458, 1765, 2195, 2919,
];
/// QQ4 and WL addressed by IL4 = 0 … 7. Table 14 prints them on addresses 2 … 9; the address
/// is IL4 (Table 17 yields 0 … 7) – K-G2-1, confirmed by the digital test sequences.
const QQ4: [i32; 8] = [0, 150, 323, 530, 786, 1121, 1612, 2557];
const WL: [i32; 8] = [-60, -30, 58, 172, 334, 538, 1198, 3042];
/// Higher sub-band: Q2(1), QQ2(1, 2), WH(1, 2).
const Q2: i32 = 564;
const QQ2: [i32; 3] = [0, 202, 926];
const WH: [i32; 3] = [0, -214, 798];

// ----- Table 15 (ILB, Method 2) ----------------------------------------------------------------

const ILB: [i32; 32] = [
    2048, 2093, 2139, 2186, 2233, 2282, 2332, 2383, 2435, 2489, 2543, 2599, 2656, 2714, 2774, 2834,
    2896, 2960, 3025, 3091, 3158, 3228, 3298, 3371, 3444, 3520, 3597, 3676, 3756, 3838, 3922, 4008,
];

// ----- Tables 16 – 21 --------------------------------------------------------------------------

/// Table 16: (SIL, MIL) → IL.
fn il_code(negative: bool, mil: i32) -> i32 {
    match (negative, mil) {
        (true, 1) => 0b111111,
        (true, 2) => 0b111110,
        (true, m) => 34 - m,  // 3 → 011111 … 30 → 000100
        (false, m) => 62 - m, // 1 → 111101 … 30 → 100000
    }
}

/// Table 17: 4-bit RIL → (SIL, IL4).
fn table17(ril: i32) -> (i32, usize) {
    match ril & 15 {
        0 => (0, 0),
        r @ 1..=7 => (-1, (8 - r) as usize),
        r => (0, (15 - r) as usize),
    }
}

/// Table 18: 6-bit RIL → (SIL, IL6).
fn table18(ril: i32) -> (i32, usize) {
    match ril & 63 {
        0..=3 | 63 => (-1, 1),
        62 => (-1, 2),
        r @ 4..=31 => (-1, (34 - r) as usize),
        r => (0, (62 - r) as usize),
    }
}

/// Table 19: 5-bit RIL → (SIL, IL5).
fn table19(ril: i32) -> (i32, usize) {
    match ril & 31 {
        0 | 1 | 31 => (-1, 1),
        r @ 2..=15 => (-1, (17 - r) as usize),
        r => (0, (31 - r) as usize),
    }
}

/// Table 20: (SIH, MIH) → IH.
fn ih_code(negative: bool, mih: i32) -> i32 {
    match (negative, mih) {
        (true, 2) => 0b00,
        (true, _) => 0b01,
        (false, 1) => 0b11,
        (false, _) => 0b10,
    }
}

/// Table 21: IH → (SIH, IH2).
fn table21(ih: i32) -> (i32, usize) {
    match ih & 3 {
        0b00 => (-1, 2),
        0b01 => (-1, 1),
        0b11 => (0, 1),
        _ => (0, 2),
    }
}

// ----- sub-band ADPCM (§6.2) -------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sub {
    Low,
    High,
}

/// The state of one sub-band ADPCM (the variables marked * in Table 13).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Band {
    sub: Sub,
    det: i32,
    nb: i32,
    a: [i32; 2], // A1, A2
    b: [i32; 6], // B1 … B6
    d: [i32; 6], // D1 … D6 (quantized difference signal, delays 1 … 6)
    p: [i32; 2], // P1, P2 (partially reconstructed signal)
    r: [i32; 2], // R1, R2 (reconstructed signal)
}

impl Band {
    fn new(sub: Sub) -> Self {
        Band {
            sub,
            // DELAYL / DELAYH reset values.
            det: if sub == Sub::Low { 32 } else { 8 },
            nb: 0,
            a: [0; 2],
            b: [0; 6],
            d: [0; 6],
            p: [0; 2],
            r: [0; 2],
        }
    }

    /// FILTEZ, FILTEP, PREDIC: (S, SZ).
    fn predict(&self) -> (i32, i32) {
        // FILTEZ: ((((WD6 + WD5) + WD4) + WD3) + WD2) + WD1.
        let sz = self
            .b
            .iter()
            .zip(self.d.iter())
            .rev()
            .fold(0, |acc, (&b, &d)| add(acc, mul(b, add(d, d))));
        let [a1, a2] = self.a;
        let [r1, r2] = self.r;
        let sp = add(mul(a1, add(r1, r1)), mul(a2, add(r2, r2)));
        (add(sp, sz), sz)
    }

    /// INVQAL / INVQAH: quantized difference signal for the predictor.
    fn invq_predictor(&self, code: i32) -> i32 {
        let (s, wd1) = match self.sub {
            Sub::Low => {
                let (s, il4) = table17(code >> 2);
                (s, at(&QQ4, il4) << 3)
            }
            Sub::High => {
                let (s, ih2) = table21(code);
                (s, at(&QQ2, ih2) << 3)
            }
        };
        mul(self.det, if s == 0 { wd1 } else { -wd1 })
    }

    /// LOGSCL / LOGSCH and SCALEL / SCALEH (Method 2): the next scale factor.
    fn adapt_scale(&mut self, code: i32) {
        let (w, limit, base) = match self.sub {
            Sub::Low => (at(&WL, table17(code >> 2).1), 18432, 8),
            Sub::High => (at(&WH, table21(code).1), 22528, 10),
        };
        let nbp = add(mul(self.nb, 32512), w).clamp(0, limit);
        let wd1 = (nbp >> 6) & 31;
        let wd2 = nbp >> 11;
        let wd3 = shr(at(&ILB, wd1 as usize), base - wd2);
        self.nb = nbp;
        self.det = sat(wd3 << 2);
    }

    /// PARREC, RECONS, UPZERO, UPPOL2, UPPOL1 and the delays (BLOCK 4L / 4H).
    fn update(&mut self, dt: i32, s: i32, sz: i32) {
        let pt = add(dt, sz);
        let rt = add(s, dt);
        // UPZERO
        let wd1 = if dt == 0 { 0 } else { 128 };
        let sg0 = sign(dt);
        let mut bp = self.b;
        for (b, &d) in bp.iter_mut().zip(self.d.iter()) {
            let wd2 = if sg0 == sign(d) { wd1 } else { -wd1 };
            *b = add(wd2, mul(*b, 32640));
        }
        // UPPOL2
        let [a1, a2] = self.a;
        let [p1, p2] = self.p;
        let sg0 = sign(pt);
        let sg1 = sign(p1);
        let sg2 = sign(p2);
        let mut wd1 = add(a1, a1);
        wd1 = add(wd1, wd1);
        let wd2 = if sg0 == sg1 { sub(0, wd1) } else { wd1 } >> 7;
        let wd3 = if sg0 == sg2 { 128 } else { -128 };
        let ap2 = add(add(wd2, wd3), mul(a2, 32512)).clamp(-12288, 12288);
        // UPPOL1
        let wd1 = if sg0 == sg1 { 192 } else { -192 };
        let ap1 = add(wd1, mul(a1, 32640));
        let wd3 = sub(15360, ap2);
        let ap1 = ap1.clamp(-wd3, wd3);
        // delays
        self.a = [ap1, ap2];
        self.b = bp;
        let [d1, d2, d3, d4, d5, _] = self.d;
        self.d = [dt, d1, d2, d3, d4, d5];
        self.p = [pt, p1];
        self.r = [rt, self.r[0]];
    }

    /// Encoder step (BLOCK 1 … 4): input X (15-bit), output the codeword (IL 6 bit, IH 2 bit).
    fn encode(&mut self, x: i32) -> i32 {
        let (s, sz) = self.predict();
        let e = sub(x, s);
        let negative = e < 0;
        let wd = if negative { 32767 - (e & 32767) } else { e };
        let code = match self.sub {
            Sub::Low => {
                // QUANTL: LDL ≤ WD < LDU, otherwise MIL = 30.
                let mil = (1..=29)
                    .find(|&k| wd < mul(at(&Q6, k as usize) << 3, self.det))
                    .unwrap_or(30);
                il_code(negative, mil)
            }
            Sub::High => {
                let mih = if wd < mul(Q2 << 3, self.det) { 1 } else { 2 };
                ih_code(negative, mih)
            }
        };
        let dt = self.invq_predictor(code);
        self.adapt_scale(code);
        self.update(dt, s, sz);
        code
    }

    /// Decoder step: codeword in, reconstructed signal out (BLOCK 2 … 6). `mode` 1 … 3 selects
    /// the inverse quantizer of the lower sub-band output (INVQBL).
    fn decode(&mut self, code: i32, mode: u8) -> i32 {
        let (s, sz) = self.predict();
        let dt = self.invq_predictor(code);
        let out_d = match self.sub {
            Sub::High => dt,
            Sub::Low => {
                let (sil, wd1) = match mode {
                    1 => {
                        let (s, i) = table18(code);
                        (s, at(&QQ6, i) << 3)
                    }
                    2 => {
                        let (s, i) = table19(code >> 1);
                        (s, at(&QQ5, i) << 3)
                    }
                    _ => {
                        let (s, i) = table17(code >> 2);
                        (s, at(&QQ4, i) << 3)
                    }
                };
                mul(self.det, if sil == 0 { wd1 } else { -wd1 })
            }
        };
        let y = add(s, out_d);
        self.adapt_scale(code);
        self.update(dt, s, sz);
        // LIMIT (BLOCK 6L / 5H).
        y.clamp(-16384, 16383)
    }
}

// ----- QMF (§5.2, Table 11) --------------------------------------------------------------------

/// H0 … H23 (Table 11, scaled by 2^13).
const H: [i32; 24] = [
    3, -11, -11, 53, 12, -156, 32, 362, -210, -805, 951, 3876, 3876, 951, -805, -210, 362, 32,
    -156, 12, 53, -11, -11, 3,
];

fn even(v: &[i32]) -> impl Iterator<Item = i32> + '_ {
    v.iter().step_by(2).copied()
}

fn odd(v: &[i32]) -> impl Iterator<Item = i32> + '_ {
    v.iter().skip(1).step_by(2).copied()
}

/// The G.722 encoder (64 kbit/s, Mode 1): 16 kHz linear PCM in, one octet per two samples out.
#[derive(Debug, Clone)]
pub struct Encoder {
    /// XIN, XIN1 … XIN23 (15-bit values).
    x: [i32; 24],
    low: Band,
    high: Band,
}

impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Encoder {
    pub fn new() -> Self {
        Encoder {
            x: [0; 24],
            low: Band::new(Sub::Low),
            high: Band::new(Sub::High),
        }
    }

    /// Encodes pairs of 16-bit samples; a trailing odd sample is ignored (frames of 20 ms hold
    /// 320 samples).
    pub fn encode(&mut self, pcm: &[i16], out: &mut Vec<u8>) {
        let (pairs, _) = pcm.as_chunks::<2>();
        for pair in pairs {
            for &s in pair {
                self.x.rotate_right(1);
                if let Some(x0) = self.x.first_mut() {
                    // 16-bit PCM → 15-bit format of Table 9.
                    *x0 = i32::from(s) >> 1;
                }
            }
            // ACCUMA / ACCUMB (exact), LOWT / HIGHT.
            let xa: i32 = even(&self.x).zip(even(&H)).map(|(x, h)| x * h).sum();
            let xb: i32 = odd(&self.x).zip(odd(&H)).map(|(x, h)| x * h).sum();
            let xl = ((xa + xb) >> 13).clamp(-16384, 16383);
            let xh = ((xa - xb) >> 13).clamp(-16384, 16383);
            let il = self.low.encode(xl);
            let ih = self.high.encode(xh);
            // §1.4.4: IH1 IH2 IL1 … IL6.
            out.push(((ih << 6) | il) as u8);
        }
    }
}

/// The G.722 decoder (64 kbit/s, Mode 1): one octet in, two 16 kHz samples out.
#[derive(Debug, Clone)]
pub struct Decoder {
    /// XD, XD1 … XD11 and XS, XS1 … XS11.
    xd: [i32; 12],
    xs: [i32; 12],
    low: Band,
    high: Band,
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder {
    pub fn new() -> Self {
        Decoder {
            xd: [0; 12],
            xs: [0; 12],
            low: Band::new(Sub::Low),
            high: Band::new(Sub::High),
        }
    }

    pub fn decode(&mut self, octets: &[u8], out: &mut Vec<i16>) {
        for &o in octets {
            let rl = self.low.decode(i32::from(o & 63), 1);
            let rh = self.high.decode(i32::from(o >> 6), 1);
            // RECA, RECB, DELAYZ.
            self.xd.rotate_right(1);
            self.xs.rotate_right(1);
            if let (Some(d), Some(s)) = (self.xd.first_mut(), self.xs.first_mut()) {
                *d = rl - rh;
                *s = rl + rh;
            }
            // ACCUMC / ACCUMD, SELECT.
            let w1: i32 = self.xd.iter().zip(even(&H)).map(|(x, h)| x * h).sum();
            let w2: i32 = self.xs.iter().zip(odd(&H)).map(|(x, h)| x * h).sum();
            for w in [w1, w2] {
                let x = (w >> 12).clamp(-16384, 16383);
                out.push((x << 1) as i16);
            }
        }
    }
}

/// RTP payload type and clock rate (RFC 3551 §4.5.2, [MS-RTP] §2.2.1).
pub const PAYLOAD_TYPE: u8 = 9;
pub const RTP_CLOCK_RATE: u32 = 8000;

#[cfg(test)]
mod tests;
