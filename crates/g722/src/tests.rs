use super::*;
use std::path::PathBuf;

/// The ITU-T digital test sequences (G.722 Appendix II, ASCII format) are not redistributed
/// with CommSCraft (ITU copyright). Set `COMMSCRAFT_G722_VECTORS` to the `g722-ascii` folder of
/// the ITU-T G.722 (09/2012) electronic attachment to run the bit-exact tests.
fn vectors() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("COMMSCRAFT_G722_VECTORS")?);
    dir.is_dir().then_some(dir)
}

/// Reads an ASCII test sequence file (§II.4.4.1): two comment lines, then lines of 16 words in
/// 64 hex characters followed by a one-byte checksum (two's complement of the sum of the
/// preceding characters), a closing comment line.
fn read(dir: &std::path::Path, name: &str) -> Vec<u16> {
    let text = std::fs::read_to_string(dir.join(name)).unwrap();
    let mut words = Vec::new();
    for line in text.lines() {
        // The files end with an MS-DOS end-of-file mark (0x1A).
        let line = line.trim().trim_matches('\u{1a}');
        if line.is_empty() || line.starts_with("/*") {
            continue;
        }
        assert_eq!(line.len(), 66, "{name}: {line}");
        let sum: u32 = line[..64].bytes().map(u32::from).sum();
        let check = u8::from_str_radix(&line[64..], 16).unwrap();
        assert_eq!((sum as u8).wrapping_neg(), check, "{name}: checksum");
        for i in 0..16 {
            words.push(u16::from_str_radix(&line[i * 4..i * 4 + 4], 16).unwrap());
        }
    }
    words
}

/// Configuration 1 (INFA, INFB): encoder only, QMF by-passed.
fn encode_sequence(input: &[u16]) -> Vec<u16> {
    let mut low = Band::new(Sub::Low);
    let mut high = Band::new(Sub::High);
    input
        .iter()
        .map(|&w| {
            let rs = w & 1;
            if rs == 1 {
                low = Band::new(Sub::Low);
                high = Band::new(Sub::High);
                return 1;
            }
            let x = i32::from(w as i16) >> 1;
            let il = low.encode(x);
            let ih = high.encode(x);
            (((ih << 6) + il) as u16) << 8
        })
        .collect()
}

/// Configuration 2 (INFC, INFD): decoder only, QMF by-passed.
fn decode_sequence(input: &[u16], sub: Sub, mode: u8) -> Vec<u16> {
    let mut band = Band::new(sub);
    input
        .iter()
        .map(|&w| {
            if w & 1 == 1 {
                band = Band::new(sub);
                return 1;
            }
            let code = match sub {
                Sub::Low => i32::from((w >> 8) & 63),
                Sub::High => i32::from(w >> 14),
            };
            ((band.decode(code, mode) << 1) as u16) & 0xfffe
        })
        .collect()
}

fn same(name: &str, got: &[u16], want: &[u16]) {
    assert_eq!(got.len(), want.len(), "{name}: length");
    if let Some(i) = got.iter().zip(want).position(|(a, b)| a != b) {
        panic!(
            "{name}: first difference at word {i}: {:#06x} != {:#06x}",
            got[i], want[i]
        );
    }
}

/// [ITU-T G.722] Appendix II, Figure II.6: all encoder and decoder comparisons.
#[test]
fn itu_digital_test_sequences() {
    let Some(dir) = vectors() else {
        eprintln!("COMMSCRAFT_G722_VECTORS not set; ITU test sequences skipped");
        return;
    };
    for (src, cmp) in [("T1C1.XMT", "T2R1.COD"), ("T1C2.XMT", "T2R2.COD")] {
        same(src, &encode_sequence(&read(&dir, src)), &read(&dir, cmp));
    }
    for (src, n) in [("T2R1.COD", 1), ("T2R2.COD", 2), ("T1D3.COD", 3)] {
        let input = read(&dir, src);
        for mode in 1..=3u8 {
            let name = format!("T3L{n}.RC{mode}");
            same(
                &name,
                &decode_sequence(&input, Sub::Low, mode),
                &read(&dir, &name),
            );
        }
        let name = format!("T3H{n}.RC0");
        same(
            &name,
            &decode_sequence(&input, Sub::High, 1),
            &read(&dir, &name),
        );
    }
}

#[test]
fn codeword_tables() {
    // Table 16 against Table 18: every encoder codeword maps back to its interval.
    for mil in 1..=30 {
        for negative in [false, true] {
            let il = il_code(negative, mil);
            assert_eq!(table18(il), (if negative { -1 } else { 0 }, mil as usize));
        }
    }
    // Table 20 against Table 21.
    for mih in 1..=2 {
        for negative in [false, true] {
            let ih = ih_code(negative, mih);
            assert_eq!(table21(ih), (if negative { -1 } else { 0 }, mih as usize));
        }
    }
    // Suppressed codewords (transmission errors) map to specified intervals.
    assert_eq!(table18(0), (-1, 1));
    assert_eq!(table19(1), (-1, 1));
    assert_eq!(table17(0), (0, 0));
}

/// Encoder → decoder through both QMFs: a 1020 Hz tone (§2.3) at −10 dBm0 comes back with a
/// signal-to-total distortion ratio well above the Annex A masks.
#[test]
fn round_trip_through_the_qmf() {
    let n = 16000;
    // +9 dBm0 overload point (§2.2) at full scale of the 15-bit format: −10 dBm0 is 19 dB below.
    let amp = 16384.0 * 2.0 * 10f64.powf(-19.0 / 20.0);
    let pcm: Vec<i16> = (0..n)
        .map(|i| (amp * (2.0 * std::f64::consts::PI * 1020.0 * i as f64 / 16000.0).sin()) as i16)
        .collect();
    let mut octets = Vec::new();
    Encoder::new().encode(&pcm, &mut octets);
    assert_eq!(octets.len(), n / 2);
    let mut out = Vec::new();
    Decoder::new().decode(&octets, &mut out);
    assert_eq!(out.len(), n);
    // Find the codec delay, then compare after the start-up.
    let best = (0..64)
        .map(|d| {
            let (mut sig, mut err) = (0f64, 0f64);
            for i in 4000..n - 64 {
                let s = f64::from(pcm[i]);
                sig += s * s;
                let e = f64::from(out[i + d]) - s;
                err += e * e;
            }
            (10.0 * (sig / err).log10(), d)
        })
        .fold((f64::MIN, 0), |a, b| if b.0 > a.0 { b } else { a });
    assert!(
        best.0 > 35.0,
        "S/(N+D) {:.1} dB at delay {}",
        best.0,
        best.1
    );
}

#[test]
fn hostile_input_does_not_panic() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    let all: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
    d.decode(&all, &mut out);
    let mut e = Encoder::new();
    let mut o = Vec::new();
    e.encode(&[i16::MIN, i16::MAX].repeat(2000), &mut o);
    e.encode(&[1], &mut o);
    assert_eq!(o.len(), 2000);
}

fn fnv1a(data: impl IntoIterator<Item = u8>) -> u64 {
    data.into_iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}

/// Regression guard for CI, where the ITU sequences are not available: the encoder and decoder
/// output for a fixed synthetic signal, recorded from the implementation that passed all ITU
/// digital test sequences (2026-10-28).
#[test]
fn pinned_output_for_ci() {
    let mut seed = 1u32;
    let pcm: Vec<i16> = (0..32000)
        .map(|i| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            let noise = ((seed >> 16) as i16) >> 4;
            let tone = (9000.0 * (i as f64 * 0.37).sin()) as i16;
            tone.saturating_add(noise)
        })
        .collect();
    let mut octets = Vec::new();
    Encoder::new().encode(&pcm, &mut octets);
    let mut out = Vec::new();
    Decoder::new().decode(&octets, &mut out);
    let enc = fnv1a(octets.iter().copied());
    let dec = fnv1a(out.iter().flat_map(|s| s.to_le_bytes()));
    println!("pinned: encoder {enc:#018x} decoder {dec:#018x}");
    assert_eq!((enc, dec), (PINNED_ENCODER, PINNED_DECODER));
}

const PINNED_ENCODER: u64 = 0xf027c36f90b457ce;
const PINNED_DECODER: u64 = 0x79f159d12ee79ae9;
