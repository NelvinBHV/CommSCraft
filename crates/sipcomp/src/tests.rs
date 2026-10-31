use super::*;

fn bits(tokens: &[Token]) -> String {
    let mut w = BitWriter::default();
    for t in tokens {
        put_token(&mut w, *t).unwrap();
    }
    let all: String = w.bytes.iter().map(|b| format!("{b:08b}")).collect();
    all[..w.bits as usize].to_string()
}

fn offset_bits(offset: usize) -> String {
    let mut w = BitWriter::default();
    put_offset(&mut w, offset).unwrap();
    let all: String = w.bytes.iter().map(|b| format!("{b:08b}")).collect();
    all[..w.bits as usize].to_string()
}

fn length_bits(length: usize) -> String {
    let mut w = BitWriter::default();
    put_length(&mut w, length).unwrap();
    let all: String = w.bytes.iter().map(|b| format!("{b:08b}")).collect();
    all[..w.bits as usize].to_string()
}

/// RFC 2118 §4.1; MS-RDPBCGR §3.1.8.4.1.1 (OB-01).
#[test]
fn literal_examples() {
    assert_eq!(bits(&[Token::Literal(0x56)]), "01010110");
    assert_eq!(bits(&[Token::Literal(0xe7)]), "101100111");
}

/// RFC 2118 §4.2.1; MS-RDPBCGR §3.1.8.4.1.2.1 (K-CP-1).
#[test]
fn offset_examples() {
    assert_eq!(offset_bits(3), "1111000011");
    assert_eq!(offset_bits(128), "111001000000");
    assert_eq!(offset_bits(1024), "1100001011000000");
    // The literal reading of MS-SIPCOMP p. 14 ("lower 8 bits of the offset value") would give
    // 1110 10000000 for 128; RFC 2118 and MS-RDPBCGR give 1110 01000000.
    assert_ne!(offset_bits(128), "111010000000");
}

/// RFC 2118 §4.2.2; MS-RDPBCGR §3.1.8.4.1.2.2.
#[test]
fn length_examples() {
    assert_eq!(length_bits(3), "0");
    assert_eq!(length_bits(15), "110111");
    assert_eq!(length_bits(120), "111110111000");
    assert_eq!(length_bits(4097), "111111111110000000000001");
}

/// MS-SIPCOMP §3.2.5.1 (p. 15): <16,15> → 1111010000110111.
#[test]
fn sipcomp_tuple_example() {
    let t = Token::Copy {
        offset: 16,
        length: 15,
    };
    assert_eq!(bits(&[t]), "1111010000110111");
    assert_eq!(encode_tokens(&[t]).unwrap(), [0xf4, 0x37]);
}

/// RFC 2118 §4 / MS-SIPCOMP §3.2.5.1 (p. 14): "for whom the bell tolls,<16,15> <40,4><19,3>e."
#[test]
fn bell_example_tokens() {
    let text = b"for whom the bell tolls, the bell tolls for thee.";
    let tokens = tokenize(text, 0, text.len());
    let mut expected: Vec<Token> = b"for whom the bell tolls,"
        .iter()
        .map(|b| Token::Literal(*b))
        .collect();
    expected.extend([
        Token::Copy {
            offset: 16,
            length: 15,
        },
        Token::Literal(b' '),
        Token::Copy {
            offset: 40,
            length: 4,
        },
        Token::Copy {
            offset: 19,
            length: 3,
        },
        Token::Literal(b'e'),
        Token::Literal(b'.'),
    ]);
    assert_eq!(tokens, expected);

    let mut c = Compressor::default();
    let packet = c.compress(text).unwrap();
    assert_eq!(packet.flags, PACKET_COMPRESSED | PACKET_AT_FRONT);
    assert_eq!(decode_tokens(&packet.payload).unwrap(), expected);
    let mut d = Decompressor::default();
    assert_eq!(d.decompress(packet.flags, &packet.payload).unwrap(), text);
}

#[test]
fn token_round_trip_all_classes() {
    let mut tokens: Vec<Token> = (0..=255u8).map(Token::Literal).collect();
    for offset in [1u16, 63, 64, 319, 320, 8191] {
        for length in [3u16, 4, 7, 8, 31, 32, 255, 256, 4095, 4096, 8191] {
            tokens.push(Token::Copy { offset, length });
        }
    }
    let data = encode_tokens(&tokens).unwrap();
    assert_eq!(decode_tokens(&data).unwrap(), tokens);
}

#[test]
fn not_encodable() {
    for t in [
        Token::Copy {
            offset: 0,
            length: 3,
        },
        Token::Copy {
            offset: 8192,
            length: 3,
        },
        Token::Copy {
            offset: 1,
            length: 2,
        },
        Token::Copy {
            offset: 1,
            length: 8192,
        },
    ] {
        assert_eq!(encode_tokens(&[t]), Err(Error::NotEncodable));
    }
}

fn sip_message(i: usize) -> Vec<u8> {
    format!(
        "SIP/2.0 200 OK\r\nVia: SIP/2.0/TLS 192.0.2.{i}:5061;branch=z9hG4bK{i:08x}\r\n\
         From: <sip:alice@example.com>;tag={i}\r\nTo: <sip:bob@example.com>\r\n\
         Call-ID: {i:016x}\r\nCSeq: {i} INVITE\r\nContent-Length: 0\r\n\r\n"
    )
    .into_bytes()
}

/// Several packets share the history; when the next one does not fit at the end, it goes to the
/// front with PACKET_AT_FRONT (CP-06).
#[test]
fn stream_round_trip_with_wrap() {
    let mut c = Compressor::default();
    let mut d = Decompressor::default();
    let mut total = 0;
    let mut at_front = 0;
    for i in 0..100 {
        let msg = sip_message(i);
        let p = c.compress(&msg).unwrap();
        assert_eq!(p.flags & PACKET_FLUSHED, 0);
        if p.flags & PACKET_AT_FRONT != 0 {
            at_front += 1;
        } else {
            assert!(p.payload.len() < msg.len() / 2, "packet {i} not compressed");
        }
        assert_eq!(d.decompress(p.flags, &p.payload).unwrap(), msg);
        total += msg.len();
    }
    assert!(total > 2 * HISTORY_SIZE);
    assert!(at_front >= 3);
}

/// Expansion: the data goes out uncompressed with PACKET_FLUSHED alone (CP-06; p. 15).
#[test]
fn expansion_flushes() {
    let mut c = Compressor::default();
    let mut d = Decompressor::default();
    let first = sip_message(1);
    let p = c.compress(&first).unwrap();
    assert_eq!(d.decompress(p.flags, &p.payload).unwrap(), first);

    let noisy: Vec<u8> = (0..100u32).map(|i| (0x80 | (i * 37 % 128)) as u8).collect();
    let p = c.compress(&noisy).unwrap();
    assert_eq!(p.flags, PACKET_FLUSHED);
    assert_eq!(p.payload, noisy);
    assert_eq!(d.decompress(p.flags, &p.payload).unwrap(), noisy);

    let p = c.compress(&first).unwrap();
    assert_eq!(p.flags, PACKET_COMPRESSED | PACKET_AT_FRONT);
    assert_eq!(d.decompress(p.flags, &p.payload).unwrap(), first);
}

/// CP-08: FLUSHED together with COMPRESSED is inconsistent; bit 0x1 is never set.
#[test]
fn inconsistent_flags() {
    let mut d = Decompressor::default();
    for flags in [PACKET_FLUSHED | PACKET_COMPRESSED, 0x1, 0x10] {
        assert_eq!(
            d.decompress(flags, &[]),
            Err(Error::InconsistentFlags(flags))
        );
    }
}

#[test]
fn malformed_streams() {
    let mut d = Decompressor::default();
    // <5,3> as the first token: nothing to copy from
    let data = encode_tokens(&[Token::Copy {
        offset: 5,
        length: 3,
    }])
    .unwrap();
    assert_eq!(
        d.decompress(PACKET_COMPRESSED | PACKET_AT_FRONT, &data),
        Err(Error::OffsetOutOfRange)
    );
    // offset 0
    assert_eq!(
        d.decompress(PACKET_COMPRESSED | PACKET_AT_FRONT, &[0xf0, 0x00]),
        Err(Error::OffsetOutOfRange)
    );
    // a length prefix of twelve one-bits
    assert_eq!(
        decode_tokens(&[0xf0, 0x7f, 0xfc, 0x00]),
        Err(Error::BadLength)
    );
    // a copy-tuple cut off after the offset
    assert_eq!(decode_tokens(&[0xf0, 0x7f]), Err(Error::Truncated));
}

#[test]
fn oversized_packet() {
    let mut c = Compressor::default();
    assert_eq!(
        c.compress(&vec![b'a'; HISTORY_SIZE + 1]),
        Err(Error::HistoryOverflow)
    );
}
