use super::*;

/// Joins lines with CRLF and appends the empty line that ends the head.
fn wire(lines: &[&str]) -> Vec<u8> {
    let mut s = lines.join("\r\n");
    s.push_str("\r\n\r\n");
    s.into_bytes()
}

/// [MS-SIPAE] 20250520 §4.1 step 8 (p. 43): the signed 200 OK of the NTLM example. The PDF breaks
/// long lines; they are joined here without the break.
pub(crate) const SIPAE_41_200OK: &[&str] = &[
    "SIP/2.0 200 OK",
    "Authentication-Info: NTLM rspauth=\"01000000000000005CD422F0C750C7C6\", srand=\"0B9D33A2\", snum=\"1\", opaque=\"BCDC0C9D\", qop=\"auth\", targetname=\"server.contoso.com\", realm=\"SIP Communications Service\"",
    "From: <sip:alice@contoso.com>;tag=4a2b44d131;epid=8248ca9ebb",
    "To: <sip:alice@contoso.com>;tag=0858513FA91D3AAE1A5840DDB99599DF",
    "Call-ID: d5f2b95d5be64c2cbfb38aa5d3a87ae7",
    "CSeq: 171 REGISTER",
    "Via: SIP/2.0/TLS 192.0.2.1:4320;ms-received-cid=1C500",
    "Contact: <sip:192.0.2.1:4320;transport=tls;msreceivedcid=1C500>;expires=7200;+sip.instance=\"<urn:uuid:4233fd41093b5fd6b5d2651ed55969e6>\";gruu=\"sip:alice@contoso.com;opaque=user:epid:Qf0zQjsJ1l10mUe1Vlp5gAA;gruu\"",
    "Expires: 7200",
    "Content-Length: 0",
];

/// [MS-SIPAE] 20250520 §4.1 step 5 (p. 42): the second REGISTER of the NTLM example.
const SIPAE_41_REGISTER: &[&str] = &[
    "REGISTER sip:contoso.com SIP/2.0",
    "Via: SIP/2.0/TLS 192.0.2.1:4320",
    "From: <sip:alice@contoso.com>;tag=4a2b44d131;epid=8248ca9ebb",
    "To: <sip:alice@contoso.com>",
    "Call-ID: d5f2b95d5be64c2cbfb38aa5d3a87ae7",
    "CSeq: 171 REGISTER",
    "Authorization: NTLM opaque=\"BCDC0C9D\", qop=\"auth\", realm=\"SIP Communications Service\", targetname=\"server.contoso.com\", gssapi-data=\"12345678ABCDE\", version=3",
    "Contact: <sip:192.0.2.1:4320;transport=tls>;proxy=replace;+sip.instance=\"<urn:uuid:4233FD41-093B-5FD6-B5D2-651ED55969E6>\"",
    "UserAgent: UCCP/2.0.6362.0 OC/2.0.6362.0 (Microsoft Office Communicator)",
    "Supported: gruu-10",
    "Content-Length: 0",
];

fn limits() -> Limits {
    Limits::default()
}

#[test]
fn parses_the_sipae_200_ok() {
    let m = parse(&wire(SIPAE_41_200OK), &limits()).unwrap();
    assert_eq!(m.status_code(), Some(200));
    assert_eq!(
        m.call_id().unwrap(),
        Some("d5f2b95d5be64c2cbfb38aa5d3a87ae7")
    );
    let cseq = m.cseq().unwrap().unwrap();
    assert_eq!(
        (cseq.number_text, cseq.number, cseq.method),
        ("171", 171, "REGISTER")
    );
    let from = m.from().unwrap().unwrap();
    assert_eq!(from.uri, "sip:alice@contoso.com");
    assert_eq!(from.param("tag"), Some("4a2b44d131"));
    assert_eq!(from.param("EPID"), Some("8248ca9ebb"));
    let to = m.to().unwrap().unwrap();
    assert_eq!(to.param("tag"), Some("0858513FA91D3AAE1A5840DDB99599DF"));
    assert_eq!(m.expires().unwrap(), Some("7200"));
    let contact = m.name_addrs("Contact").unwrap();
    assert_eq!(contact.len(), 1);
    let c = contact.first().unwrap();
    assert_eq!(
        c.uri,
        "sip:192.0.2.1:4320;transport=tls;msreceivedcid=1C500"
    );
    assert_eq!(c.param("expires"), Some("7200"));
    assert_eq!(
        c.param("+sip.instance"),
        Some("\"<urn:uuid:4233fd41093b5fd6b5d2651ed55969e6>\"")
    );
    assert!(m.body.is_empty());
}

#[test]
fn parses_the_sipae_register_and_round_trips() {
    let bytes = wire(SIPAE_41_REGISTER);
    let m = parse(&bytes, &limits()).unwrap();
    assert_eq!(m.method(), Some("REGISTER"));
    assert_eq!(
        m.start,
        StartLine::Request {
            method: "REGISTER".into(),
            uri: "sip:contoso.com".into()
        }
    );
    assert_eq!(m.to().unwrap().unwrap().param("tag"), None);
    assert_eq!(m.to_bytes().unwrap(), bytes);
    let ok = wire(SIPAE_41_200OK);
    assert_eq!(parse(&ok, &limits()).unwrap().to_bytes().unwrap(), ok);
}

#[test]
fn stream_decoder_frames_messages_and_skips_keepalives() {
    let mut stream = b"\r\n\r\n".to_vec();
    stream.extend(wire(SIPAE_41_REGISTER));
    stream.extend(b"\r\n");
    let mut with_body = SIPAE_41_200OK.to_vec();
    with_body.pop();
    with_body.push("Content-Type: text/plain");
    with_body.push("l: 5");
    let mut second = wire(&with_body);
    second.extend(b"hello");
    stream.extend(&second);

    let mut d = StreamDecoder::new(limits());
    let mut got = Vec::new();
    for b in &stream {
        d.push(std::slice::from_ref(b)).unwrap();
        while let Some(m) = d.next_message().unwrap() {
            got.push(m);
        }
    }
    assert_eq!(got.len(), 2);
    assert_eq!(got.first().unwrap().method(), Some("REGISTER"));
    assert_eq!(got.get(1).unwrap().body, b"hello");
}

#[test]
fn folding_and_compact_forms() {
    let bytes = wire(&[
        "INVITE sip:bob@example.com SIP/2.0",
        "i: abc",
        "f: \"Alice A.\" <sip:alice@example.com>;tag=1",
        "t: Bob <sip:bob@example.com>",
        "Subject: first",
        "  second",
        "CSeq: 1 INVITE",
        "l: 0",
    ]);
    let m = parse(&bytes, &limits()).unwrap();
    assert_eq!(m.call_id().unwrap(), Some("abc"));
    let from = m.from().unwrap().unwrap();
    assert_eq!(from.display.as_deref(), Some("Alice A."));
    assert_eq!(from.param("tag"), Some("1"));
    assert_eq!(m.to().unwrap().unwrap().display.as_deref(), Some("Bob"));
    assert_eq!(m.single("Subject").unwrap(), Some("first second"));
}

#[test]
fn addr_spec_and_lists() {
    let a = parse_name_addr("sip:carol@example.com;tag=xyz").unwrap();
    assert_eq!(a.uri, "sip:carol@example.com");
    assert_eq!(a.param("tag"), Some("xyz"));
    let list = split_list("<sip:a@example.com>, \"x, y\" <tel:+4930123>").unwrap();
    assert_eq!(list.len(), 2);
    let tel = parse_name_addr(list.get(1).unwrap()).unwrap();
    assert_eq!(tel.scheme(), Some("tel"));
    for bad in [
        "",
        "<sip:a@b",
        "\"open <sip:a@b>",
        "<>",
        "<nouri>",
        "<sip:a@b>;",
        "<sip:a@b>x",
    ] {
        assert!(parse_name_addr(bad).is_err(), "{bad:?}");
    }
}

fn expect_err(lines: &[&str], err: Error) {
    assert_eq!(parse(&wire(lines), &limits()), Err(err), "{lines:?}");
}

#[test]
fn hostile_framing_is_rejected() {
    expect_err(
        &["OPTIONS sip:a@b SIP/2.0", "Content-Length: 0", "l: 0"],
        Error::BadContentLength,
    );
    expect_err(
        &["OPTIONS sip:a@b SIP/2.0", "Content-Length: -1"],
        Error::BadContentLength,
    );
    expect_err(
        &["OPTIONS sip:a@b SIP/2.0", "Content-Length: 99999999999"],
        Error::BadContentLength,
    );
    expect_err(
        &["OPTIONS sip:a@b SIP/2.0", "Content-Length: 2000000"],
        Error::TooLarge,
    );
    expect_err(
        &["OPTIONS sip:a@b SIP/2.0", "Via: x"],
        Error::MissingContentLength,
    );
    expect_err(
        &["OPTIONS sip:a@b SIP/2.0", "X: a\u{0}b", "l: 0"],
        Error::BadCharacter,
    );
    expect_err(
        &["OPTIONS sip:a@b SIP/2.0", "X: a\nb", "l: 0"],
        Error::BadCharacter,
    );
    expect_err(
        &["OPTIONS sip:a@b SIP/2.0", " folded-first", "l: 0"],
        Error::BadHeader,
    );
    expect_err(
        &["OPTIONS sip:a@b SIP/2.0", "No colon", "l: 0"],
        Error::BadHeader,
    );
    expect_err(&["OPTIONS sip:a@b SIP/3.0", "l: 0"], Error::BadVersion);
    expect_err(&["OPTIONS  sip:a@b SIP/2.0", "l: 0"], Error::BadStartLine);
    expect_err(&["SIP/2.0 999 Odd", "l: 0"], Error::BadStatusCode);
    expect_err(&["SIP/2.0 20 Short", "l: 0"], Error::BadStatusCode);

    let mut too_many = vec!["OPTIONS sip:a@b SIP/2.0"];
    too_many.extend(std::iter::repeat_n("X: y", 300));
    too_many.push("l: 0");
    expect_err(&too_many, Error::TooManyHeaders);

    let long = format!("X: {}", "a".repeat(20_000));
    expect_err(
        &["OPTIONS sip:a@b SIP/2.0", &long, "l: 0"],
        Error::LineTooLong,
    );

    let mut bytes = wire(&["OPTIONS sip:a@b SIP/2.0", "l: 0"]);
    bytes.push(b'x');
    assert_eq!(parse(&bytes, &limits()), Err(Error::TrailingData));
    let bytes = wire(&["OPTIONS sip:a@b SIP/2.0", "l: 4"]);
    assert_eq!(parse(&bytes, &limits()), Err(Error::Truncated));
    assert_eq!(
        parse(b"OPTIONS sip:a@b SIP/2.0\r\nX: \xff\r\n\r\n", &limits()),
        Err(Error::BadCharacter)
    );
}

#[test]
fn identity_headers_must_be_unique_and_well_formed() {
    let m = parse(
        &wire(&[
            "BYE sip:a@b SIP/2.0",
            "From: <sip:a@b>;tag=1",
            "From: <sip:evil@b>;tag=2",
            "CSeq: 2147483648 BYE",
            "To: <sip:a@b>, <sip:c@d>",
            "l: 0",
        ]),
        &limits(),
    )
    .unwrap();
    assert_eq!(m.from(), Err(Error::DuplicateHeader("From")));
    assert_eq!(m.cseq(), Err(Error::BadCSeq));
    assert_eq!(m.to(), Err(Error::BadNameAddr));
}

#[test]
fn serializer_refuses_injection() {
    let mut m = Message::request("MESSAGE", "sip:bob@example.com");
    m.push_header("Subject", "hi\r\nInjected: yes");
    assert_eq!(m.to_bytes(), Err(Error::InvalidValue));
    let mut m = Message::request("MESSAGE", "sip:bob@example.com");
    m.push_header("Bad Name", "x");
    assert_eq!(m.to_bytes(), Err(Error::InvalidValue));
    let m = Message::request("MESS AGE", "sip:bob@example.com");
    assert_eq!(m.to_bytes(), Err(Error::InvalidValue));
    let m = Message::response(200, "OK\r\n");
    assert_eq!(m.to_bytes(), Err(Error::InvalidValue));
    // A forged Content-Length is replaced by the real length.
    let mut m = Message::request("MESSAGE", "sip:bob@example.com");
    m.push_header("Content-Length", "0");
    m.body = b"abc".to_vec();
    let bytes = m.to_bytes().unwrap();
    assert_eq!(parse(&bytes, &limits()).unwrap().body, b"abc");
}

#[test]
fn decoder_is_poisoned_after_an_error() {
    let mut d = StreamDecoder::new(limits());
    d.push(&wire(&["OPTIONS sip:a@b SIP/2.0", "l: 0", "l: 0"]))
        .unwrap();
    assert_eq!(d.next_message(), Err(Error::BadContentLength));
    assert_eq!(d.next_message(), Err(Error::Poisoned));
    assert_eq!(d.push(b"x"), Err(Error::Poisoned));

    let small = Limits {
        max_message_len: 64,
        ..Limits::default()
    };
    let mut d = StreamDecoder::new(small);
    d.push(&[b'a'; 65]).unwrap();
    assert_eq!(d.next_message(), Err(Error::TooLarge));
    let mut d = StreamDecoder::new(small);
    assert_eq!(d.push(&[b'a'; 129]), Err(Error::TooLarge));
}

/// Deterministic mutation test: no input may make the parser or the decoder panic.
#[test]
fn mutated_inputs_never_panic() {
    let seeds = [wire(SIPAE_41_200OK), wire(SIPAE_41_REGISTER)];
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut rnd = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let interesting = [
        b'\r', b'\n', b'<', b'>', b'"', b';', b',', b':', b' ', 0, 0xff, b'\\', b'=',
    ];
    for i in 0..20_000 {
        let seed = seeds.get(i % seeds.len()).unwrap();
        let mut data = seed.clone();
        for _ in 0..(rnd() % 8 + 1) {
            let pos = (rnd() as usize) % data.len().max(1);
            match rnd() % 4 {
                0 => {
                    if let Some(b) = data.get_mut(pos) {
                        *b = interesting[(rnd() as usize) % interesting.len()];
                    }
                }
                1 => {
                    if pos < data.len() {
                        data.remove(pos);
                    }
                }
                2 => data.insert(pos.min(data.len()), (rnd() & 0xff) as u8),
                _ => data.truncate(pos),
            }
        }
        if let Ok(m) = parse(&data, &limits()) {
            let _ = (
                m.from(),
                m.to(),
                m.cseq(),
                m.name_addrs("Contact"),
                m.to_bytes(),
            );
        }
        let mut d = StreamDecoder::new(limits());
        for chunk in data.chunks(7) {
            if d.push(chunk).is_err() {
                break;
            }
            while let Ok(Some(_)) = d.next_message() {}
        }
    }
}
