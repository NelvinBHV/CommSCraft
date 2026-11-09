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

// ----- book: delta files (AB-10) ------------------------------------------------------------

/// AB-10, published example §3.1 (D-0A10-0A11.lsabs): base and target date, 20 attributes, a
/// changed and a new contact, a deleted one (NumberOfAttributes 0 with
/// NumberOfDeletedAttributes 6), and the trailer counts 2 and 1 with both hashes.
#[test]
fn example_delta_file() {
    let d = book::Delta::parse(&hex(DECOMPRESSED)).unwrap();
    assert_eq!(
        (d.base, d.created, d.compact, d.normalized),
        (0x0a10, 0x0a11, false, true)
    );
    assert_eq!(d.attributes.len(), 20);
    assert_eq!((d.changed.len(), d.deleted.len()), (2, 1));
    assert_eq!((d.hash, d.base_hash), (Some(0xdfdb), Some(0xddd2)));
    let names: Vec<String> = d
        .changed
        .iter()
        .filter_map(|c| {
            c.values.iter().find_map(|(id, v)| match v {
                book::Value::Text(t)
                    if d.attributes
                        .iter()
                        .any(|a| a.id == *id && a.name == "displayName") =>
                {
                    Some(t.clone())
                }
                _ => None,
            })
        })
        .collect();
    assert_eq!(
        names,
        ["ABSUser1_displayname_changed", "ABSUser5_displayname"]
    );
    assert!(matches!(
        book::AddressBook::parse(&hex(DECOMPRESSED)),
        Err(book::Error::DeltaFile)
    ));
}

fn display_name(b: &book::AddressBook, guid: [u8; 16]) -> Option<String> {
    let c = b.contacts.iter().find(|c| c.guid == guid)?;
    b.entry(c).display_name
}

/// AB-10: the example delta applied to its base: the changed contact replaced, the new one
/// added, the deleted one removed, tables and date taken over; a delta of another base date or
/// base file hash is refused and leaves the book unchanged.
#[test]
fn delta_applies_to_its_base_only() {
    let d = book::Delta::parse(&hex(DECOMPRESSED)).unwrap();
    let (user1, user5, user2) = (d.changed[0].guid, d.changed[1].guid, d.deleted[0]);
    let name_id = d
        .attributes
        .iter()
        .find(|a| a.name == "displayName")
        .unwrap()
        .id;
    let contact = |guid, name: &str| book::Contact {
        guid,
        values: vec![(name_id, book::Value::Text(name.into()))],
    };
    let base = book::AddressBook {
        created: 0x0a10,
        normalized: false,
        rules: Vec::new(),
        attributes: d.attributes.clone(),
        contacts: vec![contact(user1, "ABSUser1_old"), contact(user2, "ABSUser2")],
        hash: Some(0xddd2),
    };
    let mut b = base.clone();
    b.apply(&d).unwrap();
    assert_eq!(
        (b.created, b.hash, b.contacts.len()),
        (0x0a11, Some(0xdfdb), 2)
    );
    assert_eq!(
        display_name(&b, user1).as_deref(),
        Some("ABSUser1_displayname_changed")
    );
    assert_eq!(
        display_name(&b, user5).as_deref(),
        Some("ABSUser5_displayname")
    );
    assert_eq!(display_name(&b, user2), None);
    assert_eq!(b.rules, d.rules);
    // Applied again (base date now 0x0a11), with another hash or another date: refused.
    assert_eq!(b.clone().apply(&d), Err(book::Error::WrongBase));
    let mut other = base.clone();
    other.hash = Some(0x1234);
    assert_eq!(other.apply(&d), Err(book::Error::WrongBase));
    assert_eq!(other.contacts, base.contacts);
}

/// AB-10, compact delta (§2.2.16): only the named attributes change, an empty value removes
/// one, the others stay.
#[test]
fn compact_delta_merges_attributes() {
    let b0 = book::AddressBook::parse(&full_file(99, b"", ATTRS, &people())).unwrap();
    let carol = b0.contacts[0].guid;
    let d = book::Delta {
        base: 0x24e2,
        created: 0x24e3,
        compact: true,
        normalized: true,
        rules: Vec::new(),
        attributes: b0.attributes.clone(),
        changed: vec![book::Contact {
            guid: carol,
            values: vec![
                (3, book::Value::Text("Carol New".into())),
                (9, book::Value::Text(String::new())),
            ],
        }],
        deleted: Vec::new(),
        hash: None,
        base_hash: Some(0xbeef),
    };
    let mut b = b0.clone();
    b.apply(&d).unwrap();
    let e = b.entry(&b.contacts[0]);
    assert_eq!(e.display_name.as_deref(), Some("Carol New"));
    assert!(e.phones.is_empty(), "phone attribute removed");
    assert_eq!(e.sip.as_deref(), Some("sip:carol@contoso.com"), "untouched");
    assert_eq!(b.contacts.len(), 2);
}

/// Security by design: a full file is no delta, dates must rise, trailer counts must match.
#[test]
fn hostile_delta_files_are_refused() {
    let full = full_file(99, b"", ATTRS, &people());
    assert_eq!(book::Delta::parse(&full), Err(book::Error::NotDeltaFile));
    let good = hex(DECOMPRESSED);
    let mut dates = good.clone();
    dates[16..20].copy_from_slice(&[0x11, 0x0a, 0x10, 0x0a]);
    assert_eq!(
        book::Delta::parse(&dates),
        Err(book::Error::Bad("delta dates"))
    );
    let mut count = good.clone();
    let at = count.len() - 8;
    count[at..at + 4].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        book::Delta::parse(&count),
        Err(book::Error::Bad("trailer contact count"))
    );
    for cut in [10, 200, good.len() - 40] {
        assert!(book::Delta::parse(&good[..cut]).is_err(), "cut {cut}");
    }
}

/// AB-11, recomputed against the example §3.1: with the file's own rules every stored phone
/// value normalizes to the normalized value stored next to it; the other values do not
/// normalize; a `null` rule and an over-long number give nothing; E164 strips separators.
#[test]
fn example_rules_normalize_like_the_server() {
    let d = book::Delta::parse(&hex(DECOMPRESSED)).unwrap();
    let n = normalize::Normalizer::new(&d.rules);
    assert_eq!((n.skipped, d.rules.len()), (0, 18));
    let mut pairs = 0;
    for c in &d.changed {
        let texts: Vec<&str> = c
            .values
            .iter()
            .filter_map(|(_, v)| match v {
                book::Value::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        for w in texts.windows(2) {
            if w[1].starts_with("tel:") {
                assert_eq!(n.normalize(w[0]).as_deref(), Some(w[1]), "{}", w[0]);
                pairs += 1;
            }
        }
    }
    assert_eq!(pairs, 6);
    assert_eq!(n.normalize("ABSUser1@urtest.com"), None);
    assert_eq!(n.normalize("(882) 8080 1234"), None, "null rule");
    assert_eq!(n.normalize("533-4312").as_deref(), Some("tel:+14255334312"));
    assert_eq!(
        n.normalize("x 12345").as_deref(),
        Some("tel:12345;phone-context=proseware.com")
    );
    assert_eq!(n.normalize(&"1".repeat(65)), None);
}

/// AB-11: E164 alone, .NET replacement syntax, and rules the engine cannot take are left out.
#[test]
fn rule_syntax() {
    let rule = |p: &str, r: &str| book::Rule {
        pattern: p.into(),
        replacement: r.into(),
    };
    let n = normalize::Normalizer::new(&[
        rule("(?<=x)1", "a"),
        rule(r"(\1)\1", "b"),
        rule(r"0(\d)(\d+)", "+49$1${2}0$$$&"),
        rule("E164", "ignored"),
    ]);
    assert_eq!(n.skipped, 2);
    assert_eq!(n.normalize("04012").as_deref(), Some("tel:+4940120$04012"));
    assert_eq!(
        n.normalize("+49 (40) 123.45-6").as_deref(),
        None,
        "plus sign is not ignored"
    );
    assert_eq!(
        n.normalize("49 (40) 123.45-6").as_deref(),
        Some("tel:+4940123456")
    );
    assert_eq!(n.normalize("1234567890123456"), None, "more than 15 digits");
}

/// AB-11: when the server left the numbers as they are, the reverse lookup normalizes them
/// with the file's rules.
#[test]
fn lookup_normalizes_unnormalized_books() {
    let rules = b"0(\\d+)\r\n+4940$1\r\n\0";
    let mut file = full_file(
        99,
        rules,
        ATTRS,
        &[vec![(3, b"Local".to_vec()), (9, b"0123".to_vec())]],
    );
    // UseNormalizationRules = 0.
    file[22..24].copy_from_slice(&0u16.to_le_bytes());
    let b = book::AddressBook::parse(&file).unwrap();
    assert!(!b.normalized);
    assert_eq!(
        b.lookup_number("tel:+4940123")
            .and_then(|e| e.display_name)
            .as_deref(),
        Some("Local")
    );
}

fn guid(n: u32) -> [u8; 16] {
    let mut g = [0u8; 16];
    g[..4].copy_from_slice(&n.to_le_bytes());
    g[15] = 1;
    g
}

fn plain_book(n: u32) -> book::AddressBook {
    book::AddressBook {
        created: 0x0a10,
        normalized: false,
        rules: Vec::new(),
        attributes: Vec::new(),
        contacts: (0..n)
            .map(|i| book::Contact {
                guid: guid(i),
                values: Vec::new(),
            })
            .collect(),
        hash: None,
    }
}

fn plain_delta(
    changed: impl Iterator<Item = u32>,
    deleted: impl Iterator<Item = u32>,
) -> book::Delta {
    book::Delta {
        base: 0x0a10,
        created: 0x0a11,
        compact: false,
        normalized: false,
        rules: Vec::new(),
        attributes: Vec::new(),
        changed: changed
            .map(|i| book::Contact {
                guid: guid(i),
                values: Vec::new(),
            })
            .collect(),
        deleted: deleted.map(guid).collect(),
        hash: None,
        base_hash: None,
    }
}

/// Issue #126: a delta that would exceed the contact limit is refused before anything changes –
/// the book keeps its contacts, date and tables; deletions in the same delta make room.
#[test]
fn refused_delta_leaves_the_book_unchanged() {
    let max = book::MAX_CONTACTS as u32;
    let base = plain_book(max - 1);
    let mut b = base.clone();
    let d = plain_delta([max, max + 1].into_iter(), [0].into_iter().take(0));
    assert_eq!(b.apply(&d), Err(book::Error::Bad("contact count")));
    assert_eq!(b, base);
    // Two new, one deleted: fits exactly.
    let d = plain_delta([max, max + 1].into_iter(), [0].into_iter());
    b.apply(&d).unwrap();
    assert_eq!(b.contacts.len(), book::MAX_CONTACTS);
    assert_eq!(b.created, 0x0a11);
}

/// Issue #125: many deletions and changes are applied in linear time (with list searches this
/// would take on the order of 10^10 comparisons).
#[test]
fn large_delta_applies_in_linear_time() {
    let mut b = plain_book(200_000);
    let d = plain_delta(100_000..200_000, 0..100_000);
    let start = std::time::Instant::now();
    b.apply(&d).unwrap();
    assert_eq!(b.contacts.len(), 100_000);
    assert!(start.elapsed() < std::time::Duration::from_secs(20));
}
