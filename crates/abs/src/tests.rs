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

// ----- book: decompressed full files (AB-05 … AB-09) ----------------------------------------

/// Builds a full file after §2.2.3–§2.2.14: `attrs` as (id, flags, name), `contacts` as values
/// (id, data) – data with a leading 0xFF byte is written as binary, otherwise as a string.
fn full_file(
    max_id: u16,
    rules: &[u8],
    attrs: &[(u16, u32, &str)],
    contacts: &[Vec<(u16, Vec<u8>)>],
) -> Vec<u8> {
    let mut f = book::FULL_FILE.to_vec();
    f.extend_from_slice(&0x24e2u16.to_le_bytes());
    f.extend_from_slice(&(attrs.len() as u16).to_le_bytes());
    f.extend_from_slice(&max_id.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&[0u8; 128]);
    f.extend_from_slice(&(rules.len() as u32).to_le_bytes());
    f.extend_from_slice(rules);
    for (id, flags, name) in attrs {
        let mut body = id.to_le_bytes().to_vec();
        body.extend_from_slice(&flags.to_le_bytes());
        body.extend_from_slice(name.as_bytes());
        body.push(0);
        f.extend_from_slice(&(body.len() as u16).to_le_bytes());
        f.extend_from_slice(&body);
    }
    for (n, values) in contacts.iter().enumerate() {
        let mut body = vec![n as u8 + 1; 16];
        body.extend_from_slice(&(values.len() as u16).to_le_bytes());
        for (id, data) in values {
            if max_id >= 256 {
                body.extend_from_slice(&id.to_le_bytes());
            } else {
                body.push(*id as u8);
            }
            if data.first() == Some(&0xff) {
                body.extend_from_slice(&((data.len() - 1) as u16).to_le_bytes());
                body.extend_from_slice(&data[1..]);
            } else {
                body.extend_from_slice(data);
                body.push(0);
            }
        }
        f.extend_from_slice(&(body.len() as u16).to_le_bytes());
        f.extend_from_slice(&body);
    }
    // Sentinel: Length 0x12, null GUID, no attributes.
    f.extend_from_slice(&0x12u16.to_le_bytes());
    f.extend_from_slice(&[0u8; 18]);
    // Full trailer: Hash, NumberOfContacts; then its length.
    f.extend_from_slice(&0xbeefu16.to_le_bytes());
    f.extend_from_slice(&(contacts.len() as u32).to_le_bytes());
    f.extend_from_slice(&6u32.to_le_bytes());
    f
}

const ATTRS: &[(u16, u32, &str)] = &[
    (1, 0x0100_0000, "givenName"),
    (3, 0x0300_0000, "displayName"),
    (8, 0x0800_0000, "msRTCSIP-PrimaryUserAddress"),
    (9, 0x0900_0000, "telephoneNumber"),
    (14, 0x0e00_0000, "mail"),
    (18, 0x1000_0001, "manager"),
];

fn people() -> Vec<Vec<(u16, Vec<u8>)>> {
    vec![
        vec![
            (3, b"Carol Test".to_vec()),
            (8, b"sip:carol@contoso.com".to_vec()),
            (9, b"+49 (40) 999-0003".to_vec()),
            (9, b"tel:+49409990003".to_vec()),
            (18, vec![0xff, 1, 2, 3]),
        ],
        vec![
            (3, b"Bob Example\x07".to_vec()),
            (8, b"sip:bob@contoso.com".to_vec()),
            (14, b"bob@contoso.com".to_vec()),
        ],
    ]
}

/// AB-05 … AB-09: header, rules (empty lines between pairs skipped; `E164` without
/// replacement), attribute table (Length before Id, K-AB-2), contacts with a repeated phone
/// attribute (stored and normalized) and a binary value, sentinel and trailer count.
#[test]
fn full_file_parses() {
    let rules = b"E164\r\n\r\n\r\n\\++(\\d+)\r\n+$1\r\n\0";
    let b = book::AddressBook::parse(&full_file(99, rules, ATTRS, &people())).unwrap();
    assert_eq!((b.created, b.normalized), (0x24e2, true));
    assert_eq!(
        b.rules,
        vec![
            book::Rule {
                pattern: "E164".into(),
                replacement: String::new()
            },
            book::Rule {
                pattern: "\\++(\\d+)".into(),
                replacement: "+$1".into()
            },
        ]
    );
    assert_eq!(b.attributes.len(), ATTRS.len());
    assert!(b.attributes[5].is_binary());
    assert_eq!(b.contacts.len(), 2);
    assert!(b.contacts[0]
        .values
        .contains(&(18, book::Value::Binary(vec![1, 2, 3]))));
    let carol = b.entry(&b.contacts[0]);
    assert_eq!(carol.display_name.as_deref(), Some("Carol Test"));
    assert_eq!(carol.phones, vec!["+49 (40) 999-0003", "tel:+49409990003"]);
    // Control characters are dropped from strings.
    assert_eq!(
        b.entry(&b.contacts[1]).display_name.as_deref(),
        Some("Bob Example")
    );
}

/// Search: every word in name, SIP or mail address, case-insensitive, bounded; reverse lookup by
/// digits matches stored and normalized forms and ignores parameters.
#[test]
fn search_and_reverse_lookup() {
    let b = book::AddressBook::parse(&full_file(99, b"", ATTRS, &people())).unwrap();
    let names = |q: &str| -> Vec<String> {
        b.search(q, 10)
            .into_iter()
            .filter_map(|e| e.display_name)
            .collect()
    };
    assert_eq!(names("carol"), ["Carol Test"]);
    assert_eq!(names("CONTOSO"), ["Carol Test", "Bob Example"]);
    assert_eq!(names("bob contoso"), ["Bob Example"]);
    assert!(names("carol bob").is_empty());
    assert!(names("   ").is_empty());
    assert_eq!(b.search("contoso", 1).len(), 1);
    for n in ["+49409990003", "tel:+49409990003;ext=12", "+49 40 999 0003"] {
        assert_eq!(
            b.lookup_number(n).and_then(|e| e.display_name).as_deref(),
            Some("Carol Test"),
            "{n}"
        );
    }
    assert!(b.lookup_number("+49409990004").is_none());
    assert!(b.lookup_number("sip:carol@contoso.com").is_none());
    assert_eq!(
        book::digits("tel:+1 (425) 555-0100;phone-context=x").as_deref(),
        Some("14255550100")
    );
    assert_eq!(book::digits("123456789012345678901"), None);
}

/// Attribute Ids are two bytes in contacts when MaximumAttributeId ≥ 256 (§2.2.11).
#[test]
fn wide_attribute_ids() {
    let attrs = [(300, 0x0300_0000, "displayName")];
    let contacts = vec![vec![(300u16, b"Wide Id".to_vec())]];
    let b = book::AddressBook::parse(&full_file(300, b"", &attrs, &contacts)).unwrap();
    assert_eq!(
        b.entry(&b.contacts[0]).display_name.as_deref(),
        Some("Wide Id")
    );
}

/// Security by design: truncated data, a wrong file type, a delta file, an attribute Id above
/// the maximum, a trailer that counts differently and oversized counts are refused; a highly
/// compressed file cannot grow beyond its bound.
#[test]
fn hostile_files_are_refused() {
    let good = full_file(99, b"", ATTRS, &people());
    for cut in [10, 150, good.len() - 30] {
        assert!(book::AddressBook::parse(&good[..cut]).is_err(), "cut {cut}");
    }
    let mut wrong = good.clone();
    wrong[0] ^= 1;
    assert_eq!(
        book::AddressBook::parse(&wrong),
        Err(book::Error::NotFullFile)
    );
    let mut delta = good.clone();
    delta[..16].copy_from_slice(&book::DELTA_FILE);
    assert_eq!(
        book::AddressBook::parse(&delta),
        Err(book::Error::DeltaFile)
    );
    let high = full_file(5, b"", ATTRS, &people());
    assert!(matches!(
        book::AddressBook::parse(&high),
        Err(book::Error::Bad(_))
    ));
    let mut count = good.clone();
    let at = count.len() - 8;
    count[at..at + 4].copy_from_slice(&7u32.to_le_bytes());
    assert_eq!(
        book::AddressBook::parse(&count),
        Err(book::Error::Bad("trailer contact count"))
    );
    let mut attrs = good.clone();
    attrs[18..20].copy_from_slice(&5000u16.to_le_bytes());
    assert!(book::AddressBook::parse(&attrs).is_err());
    // 1 MiB of zeros compresses to a few hundred bytes; the bound stops it.
    let packed = compress_file(&vec![0u8; 1 << 20]);
    assert!(packed.len() < 64 * 1024);
    assert_eq!(
        decompress_file_bounded(&packed, 1 << 19),
        Err(Error::OutputOverflow)
    );
    assert_eq!(
        decompress_file_bounded(&packed, 1 << 20).unwrap().len(),
        1 << 20
    );
}
