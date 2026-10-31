use super::*;

/// [MS-ABS] 20250520 §3.2 (pp. 59–60): the compressed D-0A10-0A11.lsabs file.
const COMPRESSED: &str = include_str!("testdata/ms-abs-3.2-compressed.hex");
/// [MS-ABS] 20250520 §3.1 (pp. 28–31): the same file after decompression.
const DECOMPRESSED: &str = include_str!("testdata/ms-abs-3.1-decompressed.hex");

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect()
}

#[test]
fn crc32_check_value() {
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
}

/// §3.3 (pp. 60–61): header of the example block.
#[test]
fn example_block_header() {
    let file = hex(COMPRESSED);
    assert_eq!(file.len(), 12 + 0x483);
    assert_eq!(&file[0..4], &0xa25c_656eu32.to_le_bytes());
    assert_eq!(&file[4..8], &0x483u32.to_le_bytes());
    assert_eq!(&file[8..12], &0xd72u32.to_le_bytes());
}

/// K-AB-1: with bit 31 as the first token the example decompresses into exactly
/// DecompressedSize bytes and consumes exactly CompressedSize bytes.
#[test]
fn example_block_decompresses_msb_first() {
    let file = hex(COMPRESSED);
    let (out, used) = decompress_block(&file[12..], 0xd72).unwrap();
    assert_eq!(used, 0x483);
    assert_eq!(out.len(), 0xd72);

    // K-AB-4: the result matches the §3.1 dump except for the 8 bytes that come from one run
    // copying "e.com" ("urtese.com" instead of "urtest.com"): the examples were edited
    // independently of each other.
    let expected = hex(DECOMPRESSED);
    assert_eq!(expected.len(), 0xd72);
    let diff: Vec<usize> = (0..out.len()).filter(|&i| out[i] != expected[i]).collect();
    assert_eq!(
        diff,
        [0xabb, 0xb6f, 0xb7d, 0xb9d, 0xc46, 0xc54, 0xcee, 0xcfc]
    );
    for i in diff {
        assert_eq!((out[i], expected[i]), (b'e', b't'));
    }
    assert_eq!(&out[0xab0..0xabd], b"User1@urtese.");
}

/// K-AB-4: the CRC printed in the example matches neither version of the data.
#[test]
fn example_crc_is_inconsistent() {
    let file = hex(COMPRESSED);
    assert_eq!(crc32(&hex(DECOMPRESSED)), 0x3c94_d862);
    assert_eq!(
        decompress_file(&file),
        Err(Error::CrcMismatch {
            stored: 0xa25c_656e,
            computed: 0x1fec_c257
        })
    );
}

/// The delta file starts with the delta FileType GUID and ends with LengthOfTrailer 0x0C
/// (AB-05; §3.1).
#[test]
fn example_structure() {
    let file = hex(COMPRESSED);
    let (out, _) = decompress_block(&file[12..], 0xd72).unwrap();
    assert_eq!(
        &out[..16],
        &hex("16 c1 4b b5 08 90 c7 47 b9 bd f3 bb 1a 0a b6 eb")[..]
    );
    assert_eq!(&out[16..20], &hex("10 0a 11 0a")[..]);
    assert_eq!(&out[out.len() - 4..], &12u32.to_le_bytes());
}

/// Run encoding per §5.3/§5.4, including the shared length nibble.
#[test]
fn run_encoding() {
    let mut out = Vec::new();
    let mut nibble = None;
    encode_run(1, 3, &mut out, &mut nibble);
    assert_eq!(out, [0x00, 0x00]);

    out.clear();
    encode_run(2, 10, &mut out, &mut nibble);
    // offset 2 → (2−1)·8 | 7; length 10 → nibble 0
    assert_eq!(out, [0x0f, 0x00, 0x00]);
    encode_run(2, 12, &mut out, &mut nibble);
    // second long run: high nibble of the shared byte = 12 − 10
    assert_eq!(out, [0x0f, 0x00, 0x20, 0x0f, 0x00]);

    out.clear();
    nibble = None;
    encode_run(1, 25 + 200, &mut out, &mut nibble);
    assert_eq!(out, [0x07, 0x00, 0x0f, 200]);

    out.clear();
    nibble = None;
    encode_run(1, 1000, &mut out, &mut nibble);
    let total = (1000u16 - 3).to_le_bytes();
    assert_eq!(out, [0x07, 0x00, 0x0f, 0xff, total[0], total[1]]);
}

#[test]
fn round_trip_lengths() {
    for len in [3usize, 9, 10, 11, 24, 25, 26, 279, 280, 281, 5000, 0xfff0] {
        let mut data = b"xy".to_vec();
        data.extend(std::iter::repeat_n(b'a', len));
        data.extend_from_slice(b"zz");
        let packed = compress_block(&data);
        let (out, used) = decompress_block(&packed, data.len()).unwrap();
        assert_eq!(out, data, "len {len}");
        assert_eq!(used, packed.len());
    }
}

#[test]
fn round_trip_file() {
    let mut data = Vec::new();
    for i in 0..20_000u32 {
        data.extend_from_slice(format!("contact{};", i % 97).as_bytes());
        data.push((i % 251) as u8);
    }
    let file = compress_file(&data);
    assert!(file.len() < data.len());
    assert_eq!(decompress_file(&file).unwrap(), data);
}

#[test]
fn incompressible_block_is_stored() {
    let data: Vec<u8> = (0..=255u8).collect();
    let file = compress_file(&data);
    assert_eq!(&file[4..8], &256u32.to_le_bytes());
    assert_eq!(&file[8..12], &256u32.to_le_bytes());
    assert_eq!(decompress_file(&file).unwrap(), data);
}

#[test]
fn malformed_input() {
    // run before any output
    assert_eq!(
        decompress_block(&[0, 0, 0, 0x80, 0, 0], 3),
        Err(Error::OffsetOutOfRange)
    );
    // truncated token-type word and literal
    assert_eq!(decompress_block(&[0, 0], 1), Err(Error::Truncated));
    assert_eq!(decompress_block(&[0, 0, 0, 0], 1), Err(Error::Truncated));
    // run longer than the declared size
    assert_eq!(
        decompress_block(&[0, 0, 0, 0x40, b'a', 0, 0], 3),
        Err(Error::OutputOverflow)
    );
    assert_eq!(decompress_block(&[], 0x10001), Err(Error::BadBlockSize));
    // CompressedSize > DecompressedSize
    let mut bad = Vec::new();
    bad.extend_from_slice(&0u32.to_le_bytes());
    bad.extend_from_slice(&2u32.to_le_bytes());
    bad.extend_from_slice(&1u32.to_le_bytes());
    bad.extend_from_slice(&[0, 0]);
    assert_eq!(decompress_file(&bad), Err(Error::BadBlockSize));
    assert_eq!(decompress_file(&[1, 2, 3]), Err(Error::Truncated));
}

/// §1.3 (p. 10): F-0A8C = Saturday, 24 May 2008; §3.1 (p. 26): F-0A10 = Monday, 21 January 2008,
/// F-0A11 = Tuesday, 22 January 2008.
#[test]
fn file_name_dates() {
    let cases = [
        (0x0a8c, 2008, 5, 24, 5),
        (0x0a10, 2008, 1, 21, 0),
        (0x0a11, 2008, 1, 22, 1),
    ];
    for (n, year, month, day, wd) in cases {
        let date = Date { year, month, day };
        assert_eq!(date_from_day_number(n), date);
        assert_eq!(day_number_from_date(date), Some(n));
        assert_eq!(weekday(n), wd);
    }
    assert_eq!(
        date_from_day_number(0),
        Date {
            year: 2001,
            month: 1,
            day: 1
        }
    );
    assert_eq!(
        day_number_from_date(Date {
            year: 2000,
            month: 12,
            day: 31
        }),
        None
    );
}
