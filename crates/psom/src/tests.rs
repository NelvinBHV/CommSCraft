use super::*;

fn hex(s: &str) -> Vec<u8> {
    s.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect()
}

fn gi(x: i64) -> Vec<u8> {
    let mut v = Vec::new();
    encode_generic_int(x, &mut v);
    v
}

const CONNMGR: &str = "Microsoft.Rtc.Server.DataMCU.Meeting.Pod.ConnMgr";
const CONNMGR_CLIENT_HASH: i64 = 8_322_047_979_521_208_965;
const CONNMGR_SERVER_HASH: i64 = -8_221_414_758_688_209_204;

/// §6.1 table (p. 102).
#[test]
fn generic_int_table() {
    for (x, bytes) in [
        (0i64, &[0x00][..]),
        (255, &[0x80, 0xff]),
        (-255, &[0x88, 0xff]),
        (256, &[0x81, 0x01, 0x00]),
    ] {
        assert_eq!(gi(x), bytes, "{x}");
        assert_eq!(decode_generic_int(bytes).unwrap(), (x, bytes.len()));
    }
}

/// Range limits of §6.1 (one byte for −112…127; data lengths 1, 2, 3, 4, 6, 8).
#[test]
fn generic_int_ranges() {
    assert_eq!(gi(127), [0x7f]);
    assert_eq!(gi(-112), [0x90]);
    assert_eq!(gi(128), [0x80, 0x80]);
    assert_eq!(gi(-113), [0x88, 0x71]);
    assert_eq!(gi(0x1_0000_0000), [0x85, 0, 0x01, 0, 0, 0, 0]);
    assert_eq!(
        gi(i64::MAX),
        [0x87, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );
    for x in [
        -1,
        -112,
        -113,
        127,
        128,
        65_535,
        65_536,
        -16_777_216,
        i64::from(i32::MIN),
        0xffff_ffff_ffff,
        0x1_0000_0000_0000,
        i64::MAX,
        i64::MIN,
    ] {
        let v = gi(x);
        assert_eq!(decode_generic_int(&v).unwrap(), (x, v.len()), "{x}");
    }
}

/// K-PO-2: the pseudo code yields the regular forms; the irregular table forms decode as
/// documented.
#[test]
fn generic_int_irregular_forms() {
    assert_eq!(gi(i64::from(i32::MIN)), [0x8b, 0x80, 0, 0, 0]);
    assert_eq!(gi(i64::MIN), [0x8f, 0x80, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(
        decode_generic_int(&[0x88, 0x00]).unwrap(),
        (i64::from(i32::MIN), 2)
    );
    assert_eq!(
        decode_generic_int(&[0x8d, 0, 0, 0, 0, 0, 0]).unwrap(),
        (i64::MIN, 7)
    );
    assert_eq!(
        decode_generic_int(&[0x89, 0, 0]),
        Err(Error::IrregularGenericInt)
    );
}

#[test]
fn generic_int_malformed() {
    for lead in [0x84, 0x86, NULL_REFERENCE, 0x8e] {
        assert_eq!(
            decode_generic_int(&[lead, 0, 0, 0, 0, 0, 0, 0, 0]),
            Err(Error::InvalidGenericInt(lead))
        );
    }
    assert_eq!(decode_generic_int(&[]), Err(Error::Truncated));
    assert_eq!(decode_generic_int(&[0x81, 0x01]), Err(Error::Truncated));
    assert_eq!(
        decode_generic_int(&[0x87, 0x80, 0, 0, 0, 0, 0, 0, 0]),
        Err(Error::Overflow)
    );
}

/// §6.2 example (p. 104).
#[test]
fn string_example() {
    let mut v = Vec::new();
    encode_string("pptdemo2.pptx", &mut v).unwrap();
    assert_eq!(v, hex("00,0d,53,44,31,32,02,15,e6,a8,85,cc,bd,aa,97"));
    assert_eq!(decode_string(&v).unwrap(), ("pptdemo2.pptx".into(), 15));
}

#[test]
fn string_limits() {
    let mut v = Vec::new();
    let s = "äöü€".to_string();
    encode_string(&s, &mut v).unwrap();
    assert_eq!(&v[..2], &[0, 9]);
    assert_eq!(decode_string(&v).unwrap(), (s, 11));
    assert_eq!(
        encode_string("\u{1f600}", &mut v),
        Err(Error::UnsupportedCharacter('\u{1f600}'))
    );
    assert_eq!(
        encode_string(&"a".repeat(0x10000), &mut v),
        Err(Error::StringTooLong)
    );
    assert_eq!(decode_string(&[0, 5, 1, 2]), Err(Error::Truncated));
}

/// §4.1.4.1 (pp. 93–94): client `version(stubHash)`.
#[test]
fn connmgr_client_version() {
    let mut params = Vec::new();
    encode_generic_int(CONNMGR_CLIENT_HASH, &mut params);
    assert_eq!(params, hex("87,73,7d,da,8b,97,1e,72,85"));
    let call = RpcCall {
        proxy_id: 0,
        method_index: 1,
        parameters: params,
    };
    let frame = call.encode().unwrap();
    assert_eq!(
        frame,
        hex("16 00 00 00 0b 00 01 87 73 7d da 8b 97 1e 72 85")
    );
    assert_eq!(RpcCall::decode(&frame).unwrap(), (call, 16));
}

/// §4.1.5 (p. 95): server `version` carries the ConnMgr server interface hash.
#[test]
fn connmgr_server_version() {
    let frame = hex("16,00,00,00,0b,00,01,8f,72,18,55,2a,02,c3,b9,34");
    let (call, used) = RpcCall::decode(&frame).unwrap();
    assert_eq!(used, 16);
    assert_eq!((call.proxy_id, call.method_index), (0, 1));
    assert_eq!(
        decode_generic_int(&call.parameters).unwrap(),
        (CONNMGR_SERVER_HASH, 9)
    );
}

/// §4.1.4.2 (pp. 94–95) and §4.1.5 (p. 95): `addProtocol(name, versions, hashes)` with the hash
/// sum of client and server interface.
#[test]
fn connmgr_add_protocol() {
    let sum = CONNMGR_SERVER_HASH.wrapping_add(CONNMGR_CLIENT_HASH);
    assert_eq!(sum, 100_633_220_832_999_761);

    let mut params = Vec::new();
    encode_string(CONNMGR, &mut params).unwrap();
    encode_generic_int(1, &mut params); // array length
    encode_generic_int(1, &mut params); // version 1
    encode_generic_int(1, &mut params); // array length
    encode_generic_int(sum, &mut params);
    let frame = RpcCall {
        proxy_id: 0,
        method_index: 2,
        parameters: params,
    }
    .encode()
    .unwrap();
    let expected = hex(
        "16,00,00,00,40,00,02,00,30,9d,88,91,71,7b,56,59,21,2c,47,28,ff,ff,83,ed,aa,92,87,67,61,\
         0a,71,27,23,09,34,c9,ce,82,f0,ab,ba,84,68,7c,44,1a,15,39,03,56,ca,f5,c5,d2,80,b9,9d,\
         01,01,01,87,01,65,85,61,94,5a,b9,51",
    );
    assert_eq!(expected.len(), 69);
    assert_eq!(frame, expected);
}

/// §4.1.4.3 and §4.1.5 (p. 95): `doneProtocols()`.
#[test]
fn connmgr_done_protocols() {
    let frame = RpcCall {
        proxy_id: 0,
        method_index: 3,
        parameters: Vec::new(),
    }
    .encode()
    .unwrap();
    assert_eq!(frame, hex("16,00,00,00,02,00,03"));
}

/// §4.1.5 (p. 96): `addProtocol` for the Meeting interface decodes cleanly. K-PO-3: it announces
/// version 1 with a hash that is not the sum of the printed Version=2 hashes (§3.1.4.1.6.1, p. 36).
#[test]
fn meeting_add_protocol() {
    let frame = hex(
        "16,00,00,00,3c,00,02,00,2c,59,4c,55,35,37,1a,15,ed,e8,83,ec,bb,83,df,51,76,56,43,23,25,\
         46,3d,eb,ef,cd,f0,8d,8a,de,4c,77,46,40,2c,38,00,56,c4,ff,ce,c8,a4,b0,88,01,01,01,8f,1b,\
         db,fa,2d,bc,55,32,5a",
    );
    assert_eq!(frame.len(), 65);
    let (call, used) = RpcCall::decode(&frame).unwrap();
    assert_eq!(used, 65);
    assert_eq!(call.method_index, 2);
    let p = &call.parameters;
    let (name, n1) = decode_string(p).unwrap();
    assert_eq!(name, "Microsoft.Rtc.Server.DataMCU.Meeting.Meeting");
    assert_eq!(&p[n1..n1 + 3], &[1, 1, 1]);
    let (hash, n2) = decode_generic_int(&p[n1 + 3..]).unwrap();
    assert_eq!(n1 + 3 + n2, p.len());
    assert_eq!(hash, -2_007_473_133_263_860_314);
    let version2_sum = 7_811_924_786_664_530_844i64.wrapping_add(2_106_930_589_629_680_263);
    assert_ne!(hash, version2_sum);
}

#[test]
fn frame_malformed() {
    assert_eq!(RpcCall::decode(&[0x17, 0, 0, 0, 0]), Err(Error::BadFrame));
    assert_eq!(RpcCall::decode(&[0x16, 0, 0, 0]), Err(Error::Truncated));
    assert_eq!(
        RpcCall::decode(&[0x16, 0, 0, 0, 5, 0]),
        Err(Error::Truncated)
    );
    assert_eq!(
        RpcCall::decode(&[0x16, 0, 0, 0, 1, 0]),
        Err(Error::BadFrame)
    );
}
