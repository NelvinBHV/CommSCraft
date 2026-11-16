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

mod client_flow {
    use super::hex;
    use crate::client::{Client, Event, SIGNATURE};
    use crate::iface::{self, Kind, INTERFACES};
    use crate::wire::{Op, Record};
    use crate::{encode_generic_int, encode_string, Error};

    const TOKEN: &str = "3000000000000000E36032154C544908";

    fn rpc(op: &Op) -> Vec<u8> {
        Record::Rpc(op.encode().unwrap()).encode().unwrap()
    }

    fn call(proxy: i64, method: u8, params: Vec<u8>) -> Vec<u8> {
        rpc(&Op::Call {
            proxy,
            method,
            params,
        })
    }

    fn records(mut bytes: &[u8]) -> Vec<Record> {
        let mut out = Vec::new();
        while let Some((r, used)) = Record::decode(bytes).unwrap() {
            out.push(r);
            bytes = &bytes[used..];
        }
        assert!(bytes.is_empty());
        out
    }

    fn ops(bytes: &[u8]) -> Vec<Op> {
        records(bytes)
            .into_iter()
            .filter_map(|r| match r {
                Record::Rpc(b) => Some(Op::decode(&b).unwrap()),
                _ => None,
            })
            .collect()
    }

    /// The server side of versioning: version, an addProtocol per interface with the same
    /// sums, doneProtocols.
    fn server_versioning(skip: Option<Kind>, tamper: Option<Kind>) -> Vec<u8> {
        let mut out = Vec::new();
        let mut p = Vec::new();
        encode_generic_int(iface::CONNMGR_SERVER_HASH, &mut p);
        out.extend(call(0, 1, p));
        for i in INTERFACES.iter().filter(|i| Some(i.kind) != skip) {
            let mut p = Vec::new();
            encode_string(i.name, &mut p).unwrap();
            encode_generic_int(i.versions.len() as i64, &mut p);
            for v in i.versions {
                encode_generic_int(i64::from(v.version), &mut p);
            }
            encode_generic_int(i.versions.len() as i64, &mut p);
            for v in i.versions {
                let h = if Some(i.kind) == tamper {
                    v.sum().wrapping_add(1)
                } else {
                    v.sum()
                };
                encode_generic_int(h, &mut p);
            }
            out.extend(call(0, 2, p));
        }
        out.extend(call(0, 3, Vec::new()));
        out
    }

    fn connected() -> Client {
        let (mut c, _) = Client::new(TOKEN, 0).unwrap();
        c.on_bytes(&SIGNATURE, 0).unwrap();
        let events = c.on_bytes(&server_versioning(None, None), 0).unwrap();
        assert_eq!(events, [Event::Connected]);
        c.take_output();
        c
    }

    /// §4.1.1–4.1.5 and §4.2 (pp. 92–96): the connection join, then on the server's signature
    /// SetChannel 0, version, the addProtocols and doneProtocols; after the server's versioning
    /// RPCOpen of channel 2 with lookup and SetChannel 2, byte for byte as in the examples.
    #[test]
    fn connection_and_versioning_as_in_the_examples() {
        let (mut c, first) = Client::new(TOKEN, 0).unwrap();
        let mut expected = hex("70 77 32 00 00 00 00 00 00 00 00 20");
        expected.extend_from_slice(TOKEN.as_bytes());
        assert_eq!(first, expected);
        assert!(c.on_bytes(&SIGNATURE[..2], 0).unwrap().is_empty());
        assert!(
            c.take_output().is_empty(),
            "nothing before the confirmation"
        );
        assert!(c.on_bytes(&SIGNATURE[2..], 0).unwrap().is_empty());
        let out = c.take_output();
        assert_eq!(&out[..5], &hex("04 00 00 00 00")[..]);
        assert_eq!(
            &out[5..21],
            &hex("16 00 00 00 0b 00 01 87 73 7d da 8b 97 1e 72 85")[..]
        );
        let sent = ops(&out);
        let add: Vec<_> = sent
            .iter()
            .filter(|o| matches!(o, Op::Call { method: 2, .. }))
            .collect();
        assert_eq!(add.len(), INTERFACES.len(), "one addProtocol per interface");
        assert_eq!(out[out.len() - 7..], hex("16 00 00 00 02 00 03")[..]);
        // The ConnMgr addProtocol as in §4.1.5 (p. 95), sent by the client the same way.
        let connmgr = hex("16,00,00,00,40,00,02,00,30,9d,88,91,71,7b,56,59,21,2c,47,28,ff,ff,83,ed,aa,92,87,67,61,0a,71,27,23,09,34,c9,ce,82,f0,ab,ba,84,68,7c,44,1a,15,39,03,56,ca,f5,c5,d2,80,b9,9d,01,01,01,87,01,65,85,61,94,5a,b9,51");
        assert!(out.windows(connmgr.len()).any(|w| w == connmgr));

        let events = c.on_bytes(&server_versioning(None, None), 0).unwrap();
        assert_eq!(events, [Event::Connected]);
        let out = c.take_output();
        let lookup = hex("37 00 00 00 02 00 00 00 28 00 05 00 12 9a 90 b4 4e 3f 51 4d 24 38 01 19 a4 e8 ce d1 a2 a8 8a 00 07 c7 f5 df e9 be bb 8b 8f 6e 14 7b cb cb 37 8e f7");
        assert_eq!(out[..49], lookup[..], "§4.2 lookup, 49 bytes");
        assert_eq!(out[49..], hex("04 00 00 00 02")[..]);
        assert!(c.is_open());
        assert_eq!(c.negotiated(Kind::Content), Some(20));
        assert_eq!(c.negotiated(Kind::ContentManager), Some(10));
    }

    /// §4.3 (p. 97): SetChannel 2, cSetUrlBase, Connect ContentUserManager and cMeetingReady
    /// from the example bytes; then content, its extension and a poll with a vote.
    #[test]
    fn meeting_content_and_poll() {
        let mut c = connected();
        let mut s = hex("04 00 00 00 02");
        s.extend(hex("16 00 00 00 26 00 04 00 22 d6 bb 94 81 38 3c 0b 50 3e 36 05 09 e6 fe 82 de a1 b2 df 62 7d 4d 52 20 24 02 16 ea ff 84 8d fd ef da"));
        s.extend(hex("16 00 00 00 1f 84 00 00 12 ad b0 9e 75 77 4d 40 10 25 02 0a c4 fb c5 dd aa bb 9d 87 49 d5 9c 18 ed 9d 9e 0c"));
        // The server connects ContentManager (its 2nd connect: our id −2) with the server hash
        // of the negotiated version 10 (K-PO-4).
        s.extend(rpc(&Op::Connect {
            parent: 0,
            part: "contentManager".into(),
            hash: -7_533_840_286_919_557_416,
        }));
        s.extend(hex("16 00 00 00 02 00 01"));
        let events = c.on_bytes(&s, 0).unwrap();
        assert_eq!(
            events,
            [
                Event::UrlBase("http://example.com/conference/1015".into()),
                Event::Ready
            ]
        );
        // Users: ContentUserManager is our −1, the server addresses it as +1.
        let mut p = Vec::new();
        encode_generic_int(1, &mut p);
        encode_generic_int(5, &mut p);
        encode_generic_int(1, &mut p);
        encode_string("sip:bob@contoso.com", &mut p).unwrap();
        encode_generic_int(1, &mut p);
        encode_string("Bob", &mut p).unwrap();
        let ev = c.on_bytes(&call(1, 1, p), 0).unwrap();
        assert_eq!(
            ev,
            [Event::UsersAdded(vec![(
                5,
                "sip:bob@contoso.com".into(),
                "Bob".into()
            )])]
        );
        // cContentAdded(7, "Content.Poll") on ContentManager (+2): we connect "content.7".
        let mut p = Vec::new();
        encode_generic_int(7, &mut p);
        encode_string("Content.Poll", &mut p).unwrap();
        // Application methods count alphabetically (K-PO-1): cContentAdded is 2 in version 10.
        let ev = c.on_bytes(&call(2, 2, p), 0).unwrap();
        assert_eq!(
            ev,
            [Event::ContentAdded {
                content: 7,
                kind: "Content.Poll".into()
            }]
        );
        assert_eq!(
            ops(&c.take_output()),
            [Op::Connect {
                parent: -2,
                part: "content.7".into(),
                hash: 974_079_596_268_293_062
            }]
        );
        // Content (our +1, the server's −1): cSetTitle, then cConnectCompleted → extension.
        let mut p = Vec::new();
        encode_string("Lunch?", &mut p).unwrap();
        let mut s = call(-1, 12, p);
        s.extend(call(-1, 1, Vec::new()));
        let ev = c.on_bytes(&s, 0).unwrap();
        assert_eq!(
            ev,
            [Event::Title {
                content: 7,
                title: "Lunch?".into()
            }]
        );
        assert_eq!(
            ops(&c.take_output()),
            [Op::Connect {
                parent: 1,
                part: "extendedContent".into(),
                hash: -1_572_291_151_947_590_318
            }]
        );
        // PollContent (our +2): cSetQuestion and cSetResults.
        let mut p = Vec::new();
        encode_string("Pizza or pasta?", &mut p).unwrap();
        encode_generic_int(2, &mut p);
        encode_string("Pizza", &mut p).unwrap();
        encode_string("Pasta", &mut p).unwrap();
        p.push(0);
        encode_string("", &mut p).unwrap();
        let mut s = call(-2, 4, p);
        let mut p = Vec::new();
        encode_generic_int(2, &mut p);
        encode_generic_int(3, &mut p);
        encode_generic_int(1, &mut p);
        s.extend(call(-2, 5, p));
        let ev = c.on_bytes(&s, 0).unwrap();
        assert_eq!(
            ev,
            [
                Event::PollQuestion {
                    content: 7,
                    question: "Pizza or pasta?".into(),
                    choices: vec!["Pizza".into(), "Pasta".into()]
                },
                Event::PollResults {
                    content: 7,
                    results: vec![3, 1]
                }
            ]
        );
        c.vote(7, 1).unwrap();
        assert_eq!(
            ops(&c.take_output()),
            [Op::Call {
                proxy: 2,
                method: 5,
                params: vec![1]
            }]
        );
        assert!(c.vote(7, 7).is_err(), "at most 7 choices");
        assert!(c.vote(8, 0).is_err(), "no such poll");
        // The keep-alive goes on channel zero and back.
        c.tick(30_000).unwrap();
        assert_eq!(
            c.take_output(),
            hex("04 00 00 00 00 16 00 00 00 02 00 06 04 00 00 00 02")
        );
        // A removed content forgets its objects; calls to them end the connection.
        let mut p = Vec::new();
        encode_generic_int(7, &mut p);
        assert_eq!(
            c.on_bytes(&call(2, 5, p), 0).unwrap(),
            [Event::ContentRemoved(7)]
        );
        assert_eq!(
            c.on_bytes(&call(-2, 5, vec![0]), 0),
            Err(Error::UnknownObject)
        );
    }

    /// Hostile or unexpected input ends the connection.
    #[test]
    fn hostile_input_ends_the_connection() {
        // A wrong confirmation.
        let (mut c, _) = Client::new(TOKEN, 0).unwrap();
        assert!(c.on_bytes(b"nope", 0).is_err());
        // The same version with another hash, and a missing required interface.
        let (mut c, _) = Client::new(TOKEN, 0).unwrap();
        c.on_bytes(&SIGNATURE, 0).unwrap();
        assert_eq!(
            c.on_bytes(&server_versioning(None, Some(Kind::Poll)), 0),
            Err(Error::HashMismatch)
        );
        let (mut c, _) = Client::new(TOKEN, 0).unwrap();
        c.on_bytes(&SIGNATURE, 0).unwrap();
        assert_eq!(
            c.on_bytes(&server_versioning(Some(Kind::Meeting), None), 0),
            Err(Error::HashMismatch)
        );
        // A method index beyond the version, a call to an unknown object, an unknown child,
        // a wrong connect hash, an RPCOpen from the server, an oversized record.
        let mut c = connected();
        c.on_bytes(&hex("04 00 00 00 02"), 0).unwrap();
        assert_eq!(
            c.on_bytes(&call(0, 5, Vec::new()), 0),
            Err(Error::UnknownMethod)
        );
        let mut c = connected();
        c.on_bytes(&hex("04 00 00 00 02"), 0).unwrap();
        assert_eq!(
            c.on_bytes(&call(9, 1, Vec::new()), 0),
            Err(Error::UnknownObject)
        );
        let mut c = connected();
        c.on_bytes(&hex("04 00 00 00 02"), 0).unwrap();
        let unknown = rpc(&Op::Connect {
            parent: 0,
            part: "Surprise".into(),
            hash: 1,
        });
        assert_eq!(c.on_bytes(&unknown, 0), Err(Error::UnknownObject));
        let mut c = connected();
        c.on_bytes(&hex("04 00 00 00 02"), 0).unwrap();
        let wrong = rpc(&Op::Connect {
            parent: 0,
            part: "ContentManager".into(),
            hash: 1,
        });
        assert_eq!(c.on_bytes(&wrong, 0), Err(Error::HashMismatch));
        let mut c = connected();
        let open = Record::RpcOpen {
            channel: 2,
            body: vec![0, 5],
        };
        assert!(c.on_bytes(&open.encode().unwrap(), 0).is_err());
        let mut c = connected();
        assert!(c.on_bytes(&hex("16 7f ff ff ff"), 0).is_err());
        // A Break with its reason closes; nothing after it is read.
        let mut c = connected();
        let brk = Record::Break("bye".into()).encode().unwrap();
        assert_eq!(
            brk,
            hex("06 00 00 00 03 62 79 65"),
            "§2.2.1.1.3 example, p. 20"
        );
        assert_eq!(c.on_bytes(&brk, 0).unwrap(), [Event::Closed("bye".into())]);
        assert!(c.on_bytes(&call(0, 1, Vec::new()), 0).unwrap().is_empty());
    }

    /// Parameters: booleans are 0 or 1, arrays are bounded by the bytes left.
    #[test]
    fn reader_bounds() {
        use crate::wire::Reader;
        assert!(Reader::new(&[2]).boolean().is_err());
        let mut big = Vec::new();
        encode_generic_int(1_000_000, &mut big);
        assert!(Reader::new(&big).ints().is_err());
        assert!(
            Reader::new(&[3, 1]).ints().is_err(),
            "count beyond the bytes"
        );
        assert_eq!(Reader::new(&[2, 5, 6]).bytes().unwrap(), [5, 6]);
    }
}

/// K-PO-1: ConnMgr counts its methods in declaration order (the examples), every application
/// interface in ordinal alphabetical order (`tested-in-lab`).
#[test]
fn method_indices() {
    use crate::iface::{client_method, interface, server_index, Kind};
    let cm = interface(Kind::ContentManager).unwrap();
    let v10 = cm.versions.iter().find(|v| v.version == 10).unwrap();
    let v2 = cm.versions.iter().find(|v| v.version == 2).unwrap();
    assert_eq!(
        client_method(Kind::ContentManager, v10, 1),
        Some("cAdvertiseContentCreationManifest")
    );
    assert_eq!(
        client_method(Kind::ContentManager, v10, 2),
        Some("cContentAdded")
    );
    assert_eq!(
        client_method(Kind::ContentManager, v2, 1),
        Some("cContentAdded")
    );
    assert_eq!(client_method(Kind::ContentManager, v10, 10), None);
    assert_eq!(client_method(Kind::ContentManager, v10, 0), None);
    let content = interface(Kind::Content).unwrap();
    let v20 = content.versions.iter().find(|v| v.version == 20).unwrap();
    assert_eq!(
        client_method(Kind::Content, v20, 8),
        Some("cSetPresentInfo")
    );
    assert_eq!(client_method(Kind::Content, v20, 12), Some("cSetTitle"));
    let meeting = &interface(Kind::Meeting).unwrap().versions[0];
    assert_eq!(
        client_method(Kind::Meeting, meeting, 4),
        Some("cSetUrlBase"),
        "§4.3"
    );
    let connmgr = &interface(Kind::ConnMgr).unwrap().versions[0];
    assert_eq!(server_index(Kind::ConnMgr, connmgr, "version"), Some(1));
    assert_eq!(
        server_index(Kind::ConnMgr, connmgr, "lookup"),
        Some(5),
        "§4.2"
    );
    let poll = &interface(Kind::Poll).unwrap().versions[0];
    assert_eq!(server_index(Kind::Poll, poll, "sVote"), Some(5));
}
