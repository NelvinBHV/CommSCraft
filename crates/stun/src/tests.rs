use super::*;

fn hex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

const PASSWORD: &[u8] = b"VOkJxbRl1RmTxUk/WvJxBt";

/// RFC 5769 §2.1.
const REQUEST: &str = "0001 0058 2112a442 b7e7a701 bc34d686 fa87dfae
 8022 0010 5354554e 20746573 7420636c 69656e74
 0024 0004 6e0001ff
 8029 0008 932ff9b1 51263b36
 0006 0009 6576746a 3a683676 59202020
 0008 0014 9aeaa70c bfd8cb56 781ef2b5 b2d3f249 c1b571a2
 8028 0004 e57a3bcf";

/// RFC 5769 §2.2.
const RESPONSE4: &str = "0101 003c 2112a442 b7e7a701 bc34d686 fa87dfae
 8022 000b 74657374 20766563 746f7220
 0020 0008 0001a147 e112a643
 0008 0014 2b91f599 fd9e90c3 8c7489f9 2af9ba53 f06be7d7
 8028 0004 c07d4c96";

/// RFC 5769 §2.3.
const RESPONSE6: &str = "0101 0048 2112a442 b7e7a701 bc34d686 fa87dfae
 8022 000b 74657374 20766563 746f7220
 0020 0014 0002a147 0113a9fa a5d3f179 bc25f4b5 bed2b9d9
 0008 0014 a382954e 4be67bf1 1784c97c 8292c275 bfe3ed41
 8028 0004 c8fb0b4c";

/// RFC 5769 §2.4 (long-term credentials, password after SASLprep "TheMatrIX").
const LONG_TERM: &str = "0001 0060 2112a442 78ad3433 c6ad72c0 29da412e
 0006 0012 e3839ee3 8388e383 aae38383 e382afe3 82b90000
 0015 001c 662f2f34 39396b39 35346436 4f4c3334 6f4c3946 53547679 36347341
 0014 000b 6578616d 706c652e 6f726700
 0008 0014 f6702465 6dd64a3e 02b8e071 2e85c9a2 8ca89666";

#[test]
fn rfc5769_vectors_verify() {
    let req = hex(REQUEST);
    assert_eq!(req.len(), 20 + 0x58);
    verify_integrity(&req, Integrity::Rfc5389(PASSWORD)).unwrap();
    verify_fingerprint(&req).unwrap();
    let m = Message::decode(&req).unwrap();
    assert_eq!(m.format(), Format::Rfc5389);
    assert_eq!(m.get(USERNAME), Some(&b"evtj:h6vY"[..]));
    assert_eq!(m.get(PRIORITY), Some(&[0x6e, 0, 1, 0xff][..]));

    for (text, addr) in [
        (RESPONSE4, "192.0.2.1:32853"),
        (RESPONSE6, "[2001:db8:1234:5678:11:2233:4455:6677]:32853"),
    ] {
        let r = hex(text);
        verify_integrity(&r, Integrity::Rfc5389(PASSWORD)).unwrap();
        verify_fingerprint(&r).unwrap();
        let m = Message::decode(&r).unwrap();
        let mapped =
            decode_xor_address(m.get(XOR_MAPPED_ADDRESS).unwrap(), &m.transaction).unwrap();
        assert_eq!(mapped, addr.parse().unwrap());
        // Our encoder produces the same XOR value.
        assert_eq!(
            encode_xor_address(mapped, &m.transaction),
            m.get(XOR_MAPPED_ADDRESS).unwrap()
        );
    }

    let lt = hex(LONG_TERM);
    let user = "\u{30DE}\u{30C8}\u{30EA}\u{30C3}\u{30AF}\u{30B9}".as_bytes();
    let key = long_term_key(user, b"example.org", b"TheMatrIX");
    verify_integrity(&lt, Integrity::Rfc5389(&key)).unwrap();
    // Our encoder pads with zeros like this vector: rebuilding it gives identical octets.
    let m = Message::decode(&lt).unwrap();
    let mut again = Message::new(m.kind, m.transaction);
    for a in m.attributes.iter().filter(|a| a.kind != MESSAGE_INTEGRITY) {
        again.push(a.kind, a.value.clone());
    }
    let rebuilt = with_integrity(again.encode().unwrap(), Integrity::Rfc5389(&key)).unwrap();
    assert_eq!(rebuilt, lt);
}

#[test]
fn tampering_is_detected() {
    let mut r = hex(RESPONSE4);
    assert_eq!(
        verify_integrity(&r, Integrity::Rfc5389(b"wrong")),
        Err(Error::Integrity)
    );
    r[30] ^= 1;
    assert_eq!(
        verify_integrity(&r, Integrity::Rfc5389(PASSWORD)),
        Err(Error::Integrity)
    );
    assert_eq!(verify_fingerprint(&r), Err(Error::Fingerprint));
    // MI must be last (only FINGERPRINT may follow).
    let mut m = Message::new(0x0101, Message::rfc5389_transaction([1; 12]));
    m.push(SOFTWARE, b"x".to_vec());
    let p = with_integrity(m.encode().unwrap(), Integrity::Rfc5389(PASSWORD)).unwrap();
    let mut extended = Message::decode(&p).unwrap();
    extended.push(SOFTWARE, b"y".to_vec());
    let q = extended.encode().unwrap();
    assert_eq!(
        verify_integrity(&q, Integrity::Rfc5389(PASSWORD)),
        Err(Error::Integrity)
    );
}

/// STUN-02 / MS-TURN format: no padding, MS magic cookie first, legacy MI (64-octet text).
#[test]
fn legacy_format_round_trip() {
    let tid = [0x5a; 16];
    let mut m = Message::new(0x0003, tid);
    m.push(
        MS_MAGIC_COOKIE,
        MS_MAGIC_COOKIE_VALUE.to_be_bytes().to_vec(),
    );
    m.push(MS_VERSION, 6u32.to_be_bytes().to_vec());
    m.push(USERNAME, b"abcde".to_vec()); // odd length, no padding
    m.push(REALM, b"realm".to_vec());
    assert_eq!(m.format(), Format::Legacy);
    let raw = m.encode().unwrap();
    assert_eq!(raw.len(), 20 + 8 + 8 + 9 + 9);
    let key = long_term_key(b"abcde", b"realm", b"secret");
    let p1 = with_integrity(raw.clone(), Integrity::LegacySha1(&key)).unwrap();
    verify_integrity(&p1, Integrity::LegacySha1(&key)).unwrap();
    assert_eq!(
        Message::decode(&p1).unwrap().get(USERNAME),
        Some(&b"abcde"[..])
    );
    // The 64-octet padding matters: a plain HMAC over the text does not verify.
    assert!(verify_integrity(&p1, Integrity::Rfc5389(&key)).is_err());
    let k256 = ms_turn_sha256_key(b"nonce", b"secret", b"abcde", b"realm");
    let p2 = with_integrity(raw, Integrity::LegacySha256(&k256)).unwrap();
    assert_eq!(p2.len(), p1.len() + 12, "32-octet HMAC");
    verify_integrity(&p2, Integrity::LegacySha256(&k256)).unwrap();
    assert!(verify_integrity(&p2, Integrity::LegacySha1(&key)).is_err());
    // Legacy XOR-MAPPED-ADDRESS (MS-TURN 0x8020) uses the whole 128-bit transaction ID.
    let a: SocketAddr = "[2001:db8::7]:4000".parse().unwrap();
    assert_eq!(
        decode_xor_address(&encode_xor_address(a, &tid), &tid).unwrap(),
        a
    );
    // Fingerprint after legacy integrity (ICE checks in STUN-02 format, O-IC-2).
    let p3 = with_fingerprint(p1).unwrap();
    verify_fingerprint(&p3).unwrap();
    verify_integrity(&p3, Integrity::LegacySha1(&key)).unwrap();
}

#[test]
fn hostile_messages() {
    assert_eq!(Message::decode(&[0u8; 19]), Err(Error::Malformed));
    assert_eq!(
        Message::decode(&[0x80; 20]),
        Err(Error::Malformed),
        "not STUN"
    );
    assert_eq!(Message::decode(&vec![0u8; 1501]), Err(Error::TooLarge));
    let mut p = vec![0, 1, 0, 8];
    p.extend_from_slice(&[7; 16]);
    p.extend_from_slice(&[0, 6, 0, 9, 1, 2, 3, 4]); // attribute longer than the message
    assert_eq!(Message::decode(&p), Err(Error::Malformed));
    let mut many = Message::new(1, [3; 16]);
    for _ in 0..40 {
        many.push(SOFTWARE, Vec::new());
    }
    assert_eq!(
        Message::decode(&many.encode().unwrap()),
        Err(Error::TooManyAttributes)
    );
    assert_eq!(
        decode_address(&[0, 3, 0, 1, 1, 2, 3, 4]),
        Err(Error::Malformed)
    );
    assert_eq!(
        decode_error(&encode_error(401, "Unauthorized\u{7}")).unwrap(),
        (401, "Unauthorized".into())
    );
}
