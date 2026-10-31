use super::*;

fn dialog() -> CallDialog {
    CallDialog {
        call_id: "0fa2623eb27c4fa1b61e5ea6544e8dd1".into(),
        from_tag: "fdb9094ccb".into(),
        to_tag: "5d42553c6f".into(),
    }
}

/// CK-01 ([MS-SIPAPP] §2.2.1.1, pp. 10–11; example §4.1.1, pp. 21–22): the park request with
/// the example's dialog and target, in the callpark namespace (K-CK-1).
#[test]
fn park_request_document() {
    let target = "sip:bob@contoso.com;opaque=user:epid:NbwN8db8n1uZbHVdCgJh0AAA;gruu";
    let x = park_request(1, &dialog(), target).unwrap();
    let r = commscraft_xml::parse(&x, &limits()).unwrap();
    assert_eq!(
        (r.name.as_str(), r.namespace.as_deref()),
        ("park-request", Some(NS))
    );
    assert_eq!(
        (r.attr("version"), r.attr("request-id")),
        (Some("1.0"), Some("1"))
    );
    let audio = r.child("audio").unwrap();
    let d = audio.child("dialog-info").unwrap();
    assert_eq!(
        d.child("call-id").unwrap().text(),
        "0fa2623eb27c4fa1b61e5ea6544e8dd1"
    );
    assert_eq!(d.child("from-tag").unwrap().text(), "fdb9094ccb");
    assert_eq!(d.child("to-tag").unwrap().text(), "5d42553c6f");
    assert_eq!(audio.child("target").unwrap().text(), target);
    assert_eq!(r.elements().count(), 1, "exactly one audio element");
}

/// Hostile dialog values or targets never reach the document.
#[test]
fn park_request_refuses_bad_values() {
    let mut d = dialog();
    d.to_tag = "x</to-tag><evil/>".into();
    assert!(park_request(1, &d, "sip:bob@contoso.com").is_err());
    let mut d = dialog();
    d.call_id = String::new();
    assert!(park_request(1, &d, "sip:bob@contoso.com").is_err());
    assert!(park_request(1, &dialog(), "http://evil.example").is_err());
    assert!(park_request(1, &dialog(), "sip:gw.contoso.com:5067;transport=tls").is_ok());
    assert!(park_request(1, &dialog(), "sip:bob@contoso.com\"><x").is_err());
}

/// CK-02 (§2.2.1.2, pp. 11–12; example p. 24): the park response with orbit and parked call.
#[test]
fn park_response_example() {
    let x = r#"<?xml version="1.0"?><park-response xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" version="1.0" request-id="1" xmlns="http://schemas.microsoft.com/rtc/2008/12/callpark">
  <orbit>1504</orbit>
  <ms-parked-call>3e851601-72d9-40d8-88d7-647e6ab8a7d5;from-tag=76ea4cdd28;to-tag=2950af20b5</ms-parked-call>
</park-response>"#;
    let p = parse_park_response(x).unwrap();
    assert_eq!((p.orbit.as_str(), p.request_id.as_str()), ("1504", "1"));
    assert!(p
        .parked_call
        .starts_with("3e851601-72d9-40d8-88d7-647e6ab8a7d5"));
}

/// CK-02 orbit shape; anything else is refused (it is dialled later).
#[test]
fn orbit_shape() {
    for ok in ["1", "1504", "123456789", "*1", "#12345678", "*100"] {
        assert!(valid_orbit(ok), "{ok}");
    }
    for bad in [
        "",
        "1234567890",
        "*",
        "#123456789",
        "12a",
        "+491",
        "*#1",
        "1 2",
        "sip:x@y",
    ] {
        assert!(!valid_orbit(bad), "{bad}");
    }
    let wrong = |orbit: &str| {
        format!(
            r#"<park-response xmlns="{NS}" version="1.0" request-id="1"><orbit>{orbit}</orbit><ms-parked-call>x</ms-parked-call></park-response>"#
        )
    };
    assert!(parse_park_response(&wrong("sip:evil@x")).is_err());
    assert!(parse_park_response(&wrong("99")).is_ok());
}

/// Wrong root, wrong namespace, other major version, entities: refused.
#[test]
fn hostile_answers() {
    let body = "<orbit>1</orbit><ms-parked-call>x</ms-parked-call>";
    assert!(parse_park_response(&format!(
        r#"<park-response version="1.0" request-id="1">{body}</park-response>"#
    ))
    .is_err());
    assert!(parse_park_response(&format!(
        r#"<park-request xmlns="{NS}" version="1.0" request-id="1">{body}</park-request>"#
    ))
    .is_err());
    assert!(parse_park_response(&format!(
        r#"<park-response xmlns="{NS}" version="2.0" request-id="1">{body}</park-response>"#
    ))
    .is_err());
    assert!(parse_park_response(&format!(r#"<!DOCTYPE x [<!ENTITY a "b">]><park-response xmlns="{NS}" version="1.0" request-id="1">{body}</park-response>"#)).is_err());
    assert!(parse_park_response(&"x".repeat(10_000)).is_err());
}

/// CK-04 (§2.2.1.3, pp. 12–13; example p. 30): the unpark notification after a retrieval;
/// all reasons; an unknown reason is refused; an unusable target is dropped.
#[test]
fn unpark_notifications() {
    let x = r#"<?xml version="1.0"?><unpark-notification xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" version="1.0" request-id="1" xmlns="http://schemas.microsoft.com/rtc/2008/12/callpark">
  <reason>retrieval</reason>
  <target>sip:carol@contoso.com</target></unpark-notification>"#;
    let u = parse_unpark(x).unwrap();
    assert_eq!(u.reason, UnparkReason::Retrieval);
    assert_eq!(u.target.as_deref(), Some("sip:carol@contoso.com"));
    let n = |reason: &str, target: &str| {
        parse_unpark(&format!(
            r#"<unpark-notification xmlns="{NS}" version="1.0" request-id="1"><reason>{reason}</reason>{target}</unpark-notification>"#
        ))
    };
    for (r, e) in [
        ("hang-up", UnparkReason::HangUp),
        ("ringback", UnparkReason::Ringback),
        ("fallback", UnparkReason::Fallback),
        ("drop", UnparkReason::Drop),
    ] {
        assert_eq!(n(r, "").unwrap().reason, e);
    }
    assert!(n("explode", "").is_err());
    assert_eq!(
        n("fallback", "<target>javascript:x</target>")
            .unwrap()
            .target,
        None
    );
}

/// Trailing NUL padding after the document is cut off; a NUL inside the document is refused.
#[test]
fn trailing_nul_padding() {
    let doc = format!(
        r#"<?xml version="1.0"?>
<park-response version="1.0" request-id="1" xmlns="{NS}">
  <orbit>7057</orbit>
  <ms-parked-call>92d1d30c</ms-parked-call>
</park-response>"#
    );
    let padded = format!("{doc}{}", "\0".repeat(200));
    assert_eq!(parse_park_response(&padded).unwrap().orbit, "7057");
    let inside = doc.replace("<orbit>", "<orbit>\0");
    assert!(parse_park_response(&inside).is_err());
}
