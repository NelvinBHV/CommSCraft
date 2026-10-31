use super::*;
use commscraft_sip_msg::{parse, Limits};

fn msg(lines: &[&str], body: &str) -> Message {
    let mut s = lines.join("\r\n");
    s.push_str(&format!("\r\nContent-Length: {}\r\n\r\n{body}", body.len()));
    parse(s.as_bytes(), &Limits::default()).unwrap()
}

fn invite() -> Message {
    let mut m = Message::request("INVITE", "sip:bob@contoso.com");
    m.push_header("Via", "SIP/2.0/TLS 192.0.2.1:4320;branch=z9hG4bKa1");
    m.push_header("From", "<sip:alice@contoso.com>;tag=111;epid=8248ca9ebb");
    m.push_header("To", "<sip:bob@contoso.com>");
    m.push_header("Call-ID", "im1");
    m.push_header("CSeq", "1 INVITE");
    invite_body(&mut m, "192.0.2.1".parse().unwrap());
    m
}

fn ok_200() -> Message {
    msg(
        &[
            "SIP/2.0 200 OK",
            "Via: SIP/2.0/TLS 192.0.2.1:4320;branch=z9hG4bKa1",
            "From: <sip:alice@contoso.com>;tag=111;epid=8248ca9ebb",
            "To: <sip:bob@contoso.com>;tag=222;epid=0b85fd1bd6",
            "Call-ID: im1",
            "CSeq: 1 INVITE",
            "Contact: <sip:bob@contoso.com;opaque=user:epid:tDDhdAyfsFq_rGXvlxZSLQAA;gruu>",
            "Record-Route: <sip:pool.contoso.com:5061;transport=tls;lr>",
            "Content-Type: application/sdp",
        ],
        "v=0\r\no=- 0 0 IN IP4 192.0.2.9\r\ns=session\r\nc=IN IP4 192.0.2.9\r\nt=0 0\r\nm=message 5060 sip null\r\na=accept-types:text/plain text/html\r\n",
    )
}

/// CI-02 (MS-CONFIM §2.2.5, p. 14): the SDP of a message session.
#[test]
fn sdp_offer_and_check() {
    let s = sdp("192.0.2.1".parse().unwrap());
    assert!(s.contains("m=message 5060 sip null\r\n"));
    assert!(s.contains("a=accept-types:text/plain\r\n"));
    assert!(!s.contains('*'), "never offer *");
    assert!(sdp("2001:db8::1".parse().unwrap()).contains("c=IN IP6 2001:db8::1"));
    assert_eq!(check_sdp(s.as_bytes()), Ok(()));
    // The MS-CONFIM example (p. 14): rtf, multipart, imdn – text/plain is still implied?
    // No: an explicit list without text/plain or * is refused (we only speak text/plain).
    assert_eq!(check_sdp(b"m=message 5060 sip null\r\na=accept-types:text/rtf multipart/alternative application/ms-imdn+xml\r\n"), Err(Error::BadSdp));
    assert_eq!(
        check_sdp(b"m=message 5060 sip null\r\n"),
        Ok(()),
        "no attribute: text/plain implied"
    );
    assert_eq!(
        check_sdp(b"m=message 5060 sip null\r\na=accept-types:*\r\n"),
        Ok(())
    );
    assert_eq!(check_sdp(b"m=audio 5000 RTP/AVP 0\r\n"), Err(Error::BadSdp));
    assert_eq!(
        check_sdp(b"m=message 1 sip null\r\nm=message 2 sip null\r\n"),
        Err(Error::BadSdp)
    );
    assert_eq!(check_sdp(&vec![b'a'; MAX_SDP + 1]), Err(Error::BadSdp));
}

#[test]
fn session_flow_requests() {
    let inv = invite();
    let ok = ok_200();
    check_sdp(&ok.body).unwrap();
    let mut d = Dialog::from_response(&inv, &ok).unwrap();
    let ack = ack_for_2xx(&d, 1);
    assert_eq!(ack.cseq().unwrap().unwrap().number, 1);
    assert_eq!(ack.method(), Some("ACK"));
    assert_eq!(ack.header_values("Route").count(), 1);
    let m = message(&mut d, "Hallo Bob").unwrap();
    assert_eq!(m.cseq().unwrap().unwrap().number, 2);
    assert_eq!(
        m.single("Content-Type").unwrap(),
        Some("text/plain; charset=UTF-8")
    );
    assert_eq!(m.body, b"Hallo Bob");
    let t = typing(&mut d, true);
    assert_eq!(t.cseq().unwrap().unwrap().number, 3);
    assert!(String::from_utf8(t.body)
        .unwrap()
        .contains("status=\"type\""));
    let bye = d.request("BYE");
    assert_eq!(bye.cseq().unwrap().unwrap().method, "BYE");
    assert_eq!(message(&mut d, "  "), Err(Error::EmptyText));
    assert_eq!(
        message(&mut d, &"x".repeat(MAX_TEXT + 1)),
        Err(Error::TextTooLong)
    );
}

#[test]
fn ack_for_a_failure_reuses_the_branch() {
    let inv = invite();
    let busy = msg(
        &[
            "SIP/2.0 480 Temporarily Unavailable",
            "Via: SIP/2.0/TLS 192.0.2.1:4320;branch=z9hG4bKa1",
            "From: <sip:alice@contoso.com>;tag=111;epid=8248ca9ebb",
            "To: <sip:bob@contoso.com>;tag=333",
            "Call-ID: im1",
            "CSeq: 1 INVITE",
        ],
        "",
    );
    let ack = ack_for_failure(&inv, &busy);
    assert_eq!(
        ack.single("Via").unwrap(),
        Some("SIP/2.0/TLS 192.0.2.1:4320;branch=z9hG4bKa1")
    );
    assert_eq!(ack.to().unwrap().unwrap().param("tag"), Some("333"));
    assert_eq!(ack.cseq().unwrap().unwrap().number, 1);
}

fn peer(method: &str, ct: &str, body: &str) -> Message {
    msg(
        &[
            &format!("{method} sip:alice@contoso.com;gruu SIP/2.0"),
            "Via: SIP/2.0/TLS 10.0.0.1;branch=z9hG4bKp",
            "From: <sip:bob@contoso.com>;tag=222",
            "To: <sip:alice@contoso.com>;tag=111",
            "Call-ID: im1",
            "CSeq: 5 X",
            &format!("Content-Type: {ct}"),
        ],
        body,
    )
}

#[test]
fn incoming_requests() {
    let (i, r) = handle_request(&peer(
        "MESSAGE",
        "text/plain; charset=UTF-8",
        "Hi Alice\u{1b}[31m",
    ));
    assert_eq!(i, Incoming::Text("Hi Alice[31m".into()));
    assert_eq!(r.status_code(), Some(200));
    let (i, r) = handle_request(&peer("MESSAGE", "text/html", "<script>x</script>"));
    assert_eq!(i, Incoming::Refused(415));
    assert_eq!(r.single("Accept").unwrap(), Some("text/plain"));
    let (i, _) = handle_request(&peer("MESSAGE", "text/plain; charset=ISO-8859-1", "x"));
    assert_eq!(i, Incoming::Refused(415));
    let (i, r) = handle_request(&peer(
        "INFO",
        "application/xml",
        "<?xml version=\"1.0\"?><KeyboardActivity><status status=\"type\" /></KeyboardActivity>",
    ));
    assert_eq!((i, r.status_code()), (Incoming::Typing(true), Some(200)));
    let (i, _) = handle_request(&peer(
        "INFO",
        "application/xml",
        "<KeyboardActivity><status status=\"idle\"/></KeyboardActivity>",
    ));
    assert_eq!(i, Incoming::Typing(false));
    let (i, _) = handle_request(&peer(
        "INFO",
        "application/xml",
        "<!DOCTYPE x><KeyboardActivity/>",
    ));
    assert_eq!(i, Incoming::Refused(415));
    let (i, r) = handle_request(&peer("BYE", "text/plain", ""));
    assert_eq!((i, r.status_code()), (Incoming::Bye, Some(200)));
    let (i, r) = handle_request(&peer("REFER", "text/plain", ""));
    assert_eq!((i, r.status_code()), (Incoming::Refused(501), Some(501)));
}
