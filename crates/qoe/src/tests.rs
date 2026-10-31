use super::*;
use commscraft_xml::{parse, Limits};

fn sample() -> Report {
    Report {
        endpoint: "alice-laptop".into(),
        dialog: DialogInfo {
            call_id: "ab323818af644d1eab6bacd6d66d03a7".into(),
            from_tag: "e957a6c0d5".into(),
            to_tag: "313433a5ba".into(),
            start: 1_199_735_226,
            end: 1_199_735_711,
            from_uri: "sip:alice@contoso.com".into(),
            to_uri: "sip:bob@contoso.com".into(),
            caller: true,
            local_contact: "sip:alice@contoso.com;opaque=user:epid:x;gruu".into(),
            remote_contact: "sip:bob@contoso.com;opaque=user:epid:y;gruu".into(),
            local_user_agent: "CommSCraft/0.0.1".into(),
            remote_user_agent: "UCCP <&> 2.0".into(),
            local_pai: "sip:alice@contoso.com".into(),
        },
        audio: AudioLine {
            encrypted: true,
            offerer: true,
            tcp: false,
            local: "192.0.2.1:50004".parse().unwrap(),
            remote: "198.51.100.7:63284".parse().unwrap(),
            inbound: Some(Stream {
                ssrc: 1_536_632_130,
                packets: 23_148,
                jitter_avg_ms: Some(2),
                jitter_max_ms: Some(3),
                loss_avg_ppm: Some(0),
                loss_max_ppm: Some(9259),
                rtt_avg_ms: None,
                rtt_max_ms: None,
            }),
            outbound: Some(Stream {
                ssrc: 1_869_098_562,
                packets: 7497,
                jitter_avg_ms: Some(2),
                jitter_max_ms: Some(2),
                loss_avg_ppm: None,
                loss_max_ppm: None,
                rtt_avg_ms: Some(1),
                rtt_max_ms: Some(2),
            }),
            codec: "G722".into(),
            sample_rate: 16000,
        },
    }
}

fn names(e: &commscraft_xml::Element) -> Vec<&str> {
    e.elements().map(|c| c.name.as_str()).collect()
}

/// QE-02/QE-04 ([MS-QoE] §2.2.1, example §4.1 pp. 82–84): the report's structure, element
/// order of the schema, LocalPAI, escaping, network figures, required Payload/Audio.
#[test]
fn report_document() {
    let x = report(&sample()).unwrap();
    let r = parse(&x, &Limits::default()).unwrap();
    assert_eq!(
        (r.name.as_str(), r.namespace.as_deref()),
        ("VQReportEvent", Some(NS_REPORT))
    );
    let s = r.child("VQSessionReport").unwrap();
    assert_eq!(
        s.attr("SessionId"),
        Some("ab323818af644d1eab6bacd6d66d03a7;from-tag=e957a6c0d5;to-tag=313433a5ba")
    );
    assert_eq!(names(s), ["Endpoint", "DialogInfo", "MediaLine"]);
    let d = s.child("DialogInfo").unwrap();
    assert_eq!(d.attr("Start"), Some("2008-01-07T19:47:06Z"));
    assert_eq!(d.attr("End"), Some("2008-01-07T19:55:11Z"));
    assert_eq!(
        names(d),
        [
            "FromURI",
            "ToURI",
            "Caller",
            "LocalContactURI",
            "RemoteContactURI",
            "LocalUserAgent",
            "RemoteUserAgent",
            "LocalPAI"
        ]
    );
    assert_eq!(d.child("RemoteUserAgent").unwrap().text(), "UCCP <&> 2.0");
    assert_eq!(d.child("LocalPAI").unwrap().text(), "sip:alice@contoso.com");
    let m = s.child("MediaLine").unwrap();
    assert_eq!(m.attr("Label"), Some("main-audio"));
    let desc = m.child("Description").unwrap();
    assert_eq!(
        names(desc),
        [
            "Security",
            "Offerer",
            "Transport",
            "LocalAddr",
            "RemoteAddr"
        ]
    );
    let inb = m.child("InboundStream").unwrap();
    assert_eq!(inb.attr("Id"), Some("1536632130"));
    let net = inb.child("Network").unwrap();
    assert_eq!(names(net), ["Jitter", "PacketLoss", "Utilization"]);
    assert_eq!(
        net.child("PacketLoss")
            .unwrap()
            .child("LossRateMax")
            .unwrap()
            .text(),
        "0.009259"
    );
    let out = m.child("OutboundStream").unwrap();
    let net = out.child("Network").unwrap();
    assert_eq!(
        names(net),
        ["Jitter", "Delay", "Utilization"],
        "no loss figures: no element"
    );
    assert_eq!(
        out.child("Payload")
            .unwrap()
            .child("Audio")
            .unwrap()
            .child("SampleRate")
            .unwrap()
            .text(),
        "16000"
    );
}

/// QE-05: long texts are cut, identifiers that must not be cut are refused.
#[test]
fn lengths() {
    let mut r = sample();
    r.dialog.remote_user_agent = "x".repeat(1000);
    let x = report(&r).unwrap();
    assert!(x.contains(&format!(
        "<RemoteUserAgent>{}</RemoteUserAgent>",
        "x".repeat(256)
    )));
    for f in [
        |r: &mut Report| r.dialog.call_id = "c".repeat(257),
        |r: &mut Report| r.endpoint = String::new(),
        |r: &mut Report| r.dialog.from_tag = "a\r\nb".into(),
        |r: &mut Report| r.dialog.start = u64::MAX,
    ] {
        let mut r = sample();
        f(&mut r);
        assert!(report(&r).is_err());
    }
}

/// QE-03 ([MS-QoE] §2.2.2, example §4.2 p. 84): call rating 1–5 with optional text.
#[test]
fn call_rating() {
    let x = rating(
        "ab323818af644d1eab6bacd6d66d03a7",
        "e957a6c0d5",
        "313433a5ba",
        "sip:alice@contoso.com",
        4,
        Some("Nothing more to tell, call quality was great!"),
    )
    .unwrap();
    let r = parse(&x, &Limits::default()).unwrap();
    assert_eq!(
        (r.name.as_str(), r.namespace.as_deref()),
        ("CallQualityFeedbackReport", Some(NS_RATING))
    );
    assert_eq!(names(&r), ["ReportingUserURI", "Rating", "Feedback"]);
    assert_eq!(r.child("Rating").unwrap().text(), "4");
    for stars in [0, 6] {
        assert!(rating("c", "f", "t", "sip:a@b", stars, None).is_err());
    }
    let long = rating("c", "f", "t", "sip:a@b", 1, Some(&"y".repeat(5000))).unwrap();
    assert!(long.contains(&"y".repeat(4000)) && !long.contains(&"y".repeat(4001)));
}
