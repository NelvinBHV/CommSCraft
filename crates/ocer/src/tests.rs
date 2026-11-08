use super::*;

/// ER-01: the example of §2.2.1.1 (p. 12) – ErrorId, reason, source and further parameters,
/// quoted strings with `;` inside kept together.
#[test]
fn example_header() {
    let v = r#"1007;reason="Temporarily cannot route";source="sip.contoso.com";ErrorType="Connect Attempt Failure";WinsockFailureDescription="The peer actively refused the connection attempt";WinsockFailureCode="274D(WSAECONNREFUSED)";Peer="sip.fabrikam.com""#;
    let d = Diagnostic::parse(v, false).unwrap();
    assert_eq!(d.id, 1007);
    assert_eq!(d.reason.as_deref(), Some("Temporarily cannot route"));
    assert_eq!(d.source.as_deref(), Some("sip.contoso.com"));
    assert_eq!(d.params.len(), 4);
    assert_eq!(d.params[3], ("Peer".into(), "sip.fabrikam.com".into()));
    assert!(d.explanation().is_some());
    let q = Diagnostic::parse(r#"6009;reason="a \"long\"; message";source="x""#, false).unwrap();
    assert_eq!(q.reason.as_deref(), Some(r#"a "long"; message"#));
}

/// ER-02: the public header (example §2.2.1.2) carries no source; one given anyway is ignored.
#[test]
fn public_header() {
    let d = Diagnostic::parse(
        r#"3027;reason=" User not allowed in closed conference""#,
        true,
    )
    .unwrap();
    assert_eq!((d.id, d.public), (3027, true));
    assert_eq!(
        explain(3027),
        Some("you are not allowed into this closed meeting")
    );
    let s = Diagnostic::parse(r#"3027;reason="x";source="internal.example""#, true).unwrap();
    assert_eq!(s.source, None);
}

/// From a message: ms-diagnostics first, then ms-diagnostics-public.
#[test]
fn from_messages() {
    let raw = "SIP/2.0 404 Not Found\r\nVia: SIP/2.0/TLS a;branch=z9hG4bK1\r\nms-diagnostics-public: 3026;reason=\"x\"\r\nms-diagnostics: 1003;reason=\"User does not exist\";source=\"pool.contoso.com\"\r\nContent-Length: 0\r\n\r\n";
    let m =
        commscraft_sip_msg::parse(raw.as_bytes(), &commscraft_sip_msg::Limits::default()).unwrap();
    let d = Diagnostic::from_message(&m).unwrap();
    assert_eq!((d.id, d.public), (1003, false));
    let raw = "SIP/2.0 404 Not Found\r\nms-diagnostics-public: 3026;reason=\"x\"\r\nContent-Length: 0\r\n\r\n";
    let m =
        commscraft_sip_msg::parse(raw.as_bytes(), &commscraft_sip_msg::Limits::default()).unwrap();
    assert_eq!(Diagnostic::from_message(&m).unwrap().id, 3026);
}

/// Hostile values: no ErrorId, an over-long value, control characters and many parameters.
#[test]
fn hostile_headers() {
    assert_eq!(
        Diagnostic::parse("reason=\"x\"", false),
        Err(Error::Malformed)
    );
    assert_eq!(
        Diagnostic::parse("-1;reason=x", false),
        Err(Error::Malformed)
    );
    assert_eq!(
        Diagnostic::parse(&"9".repeat(5000), false),
        Err(Error::Malformed)
    );
    let d = Diagnostic::parse("12;reason=\"a\u{1b}[31mb\"", false).unwrap();
    assert_eq!(d.reason.as_deref(), Some("a[31mb"));
    let many: String = (0..100).map(|i| format!(";p{i}=v")).collect();
    assert_eq!(
        Diagnostic::parse(&format!("1{many}"), false)
            .unwrap()
            .params
            .len(),
        MAX_PARAMS
    );
    assert_eq!(explain(99_999), None);
}

/// ER-04: the report document of example §4.1 (pp. 19–21) shape, escaped values and the
/// length limits of §2.2.2.
#[test]
fn report_document() {
    let r = Report {
        to_uri: Some("sip:user@contoso.com".into()),
        call_id: "5ec5a21ab8bb4960b98de162f45cd204".into(),
        from_tag: Some("823bb11e1a".into()),
        to_tag: Some("4F230FC472C24AD1255468D8C334D8FE".into()),
        request_type: "INVITE".into(),
        content_type: Some("application/sdp;call-type=audiovideo".into()),
        response_code: 408,
        diag_header: Some("10000;reason=\"Gateway returned a SIP failure code\"".into()),
        progress: vec!["12006;reason=\"Trying next hop\";appName=\"OutboundRouting\"".into()],
        ..Report::default()
    };
    let b = report_error_body(&r).unwrap();
    assert!(b.starts_with(r#"<reportError xmlns="http://schemas.microsoft.com/2006/09/sip/error-reporting"><error toUri="sip:user@contoso.com" callId="5ec5a21ab8bb4960b98de162f45cd204""#));
    assert!(b.contains(r#"responseCode="408" requestType="INVITE">"#));
    assert!(b.contains(
        "<diagHeader>10000;reason=&quot;Gateway returned a SIP failure code&quot;</diagHeader>"
    ));
    assert!(b.contains("<progressReports><progressReport><diagHeader>12006;"));
    assert!(report_error_body(&Report {
        request_type: "X".repeat(34),
        ..r.clone()
    })
    .is_err());
    assert!(report_error_body(&Report {
        call_id: "c".repeat(700),
        ..r.clone()
    })
    .is_err());
    assert!(report_error_body(&Report {
        call_id: String::new(),
        ..r.clone()
    })
    .is_err());
    assert!(report_error_body(&Report {
        content_type: Some("t".repeat(258)),
        ..r
    })
    .is_err());
}

/// ER-05: no further reports after a 503 without diagnostics or with 2019; busy (Retry-After),
/// 413 and other answers do not stop them.
#[test]
fn report_answers() {
    let d2019 = Diagnostic::parse("2019;reason=\"x\"", false).unwrap();
    let other = Diagnostic::parse("2000;reason=\"x\"", false).unwrap();
    assert!(!keep_reporting(503, false, None));
    assert!(!keep_reporting(503, false, Some(&d2019)));
    assert!(keep_reporting(503, false, Some(&other)));
    assert!(keep_reporting(503, true, None));
    assert!(keep_reporting(413, false, None));
    assert!(keep_reporting(200, false, None));
}
