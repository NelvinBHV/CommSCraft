//! Wire-level synthetic regression: no network I/O or authentication.
use commscraft_core::sip::{illustrative_register, SipMessage, StartLine};
use commscraft_core::stream::SipStreamDecoder;
use commscraft_router::{ResponseRouter, RoutedEvent, RouterError};

fn request(seq: u32, branch: &str) -> SipMessage {
    let mut msg = illustrative_register("sip:alice@example.com").unwrap();
    for (name, value) in &mut msg.headers {
        if name == "CSeq" { *value = format!("{seq} REGISTER"); }
        if name == "Via" { *value = format!("SIP/2.0/TLS client.invalid;branch={branch}"); }
    }
    msg
}
fn response(req: &SipMessage, code: u16) -> SipMessage {
    SipMessage {
        start_line: StartLine::Response { code, reason: "Synthetic".into() },
        headers: ["Via", "Call-ID", "CSeq"].iter().map(|name|
            ((*name).into(), req.header(name).unwrap().into())).collect(),
        body: Vec::new(),
    }
}

#[test]
fn framed_tls_like_stream_routes_two_replies_without_interleaving() {
    let first = request(1, "z9hG4bK-wire-first");
    let second = request(2, "z9hG4bK-wire-second");
    let mut router = ResponseRouter::new();
    let first_id = router.track(&first, 0).unwrap();
    let second_id = router.track(&second, 0).unwrap();
    let wire = [response(&second, 100).encode().unwrap(),
        response(&first, 401).encode().unwrap(), response(&second, 200).encode().unwrap()].concat();
    let mut decoder = SipStreamDecoder::new();
    let mut events = Vec::new();
    for chunk in wire.chunks(13) {
        for message in decoder.push(chunk).unwrap() {
            events.push(router.response(&message, 200).unwrap());
        }
    }
    assert_eq!(decoder.pending_bytes(), 0);
    assert_eq!(events, vec![
        RoutedEvent::Provisional { id: second_id, code: 100 },
        RoutedEvent::Final { id: first_id, code: 401 },
        RoutedEvent::Final { id: second_id, code: 200 },
    ]);
    assert_eq!(router.len(), 0);
}

#[test]
fn duplicate_final_from_coalesced_stream_is_not_delivered_twice() {
    let req = request(3, "z9hG4bK-wire-third");
    let mut router = ResponseRouter::new();
    router.track(&req, 0).unwrap();
    let packet = response(&req, 200).encode().unwrap();
    let mut decoder = SipStreamDecoder::new();
    let replies = decoder.push(&[packet.clone(), packet].concat()).unwrap();
    assert_eq!(replies.len(), 2);
    assert!(matches!(router.response(&replies[0], 1), Ok(RoutedEvent::Final { .. })));
    assert_eq!(router.response(&replies[1], 2), Err(RouterError::DuplicateOrLateResponse));
}

#[test]
fn malformed_response_does_not_consume_tracked_request() {
    let req = request(4, "z9hG4bK-wire-fourth");
    let mut router = ResponseRouter::new();
    router.track(&req, 0).unwrap();
    let mut invalid = response(&req, 401);
    invalid.headers.push(("Via".into(), req.header("Via").unwrap().into()));
    let mut decoder = SipStreamDecoder::new();
    let wire = invalid.encode().unwrap();
    let decoded = decoder.push(&wire).unwrap();
    assert_eq!(decoded.len(), 1);
    assert!(router.response(&decoded[0], 1).is_err());
    assert_eq!(router.len(), 1);
    assert!(matches!(router.response(&response(&req, 401), 2), Ok(RoutedEvent::Final { code: 401, .. })));
}
