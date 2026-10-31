use super::*;

const EXCERPT: &str = include_str!("testdata/ms-siprege-4.2.2-excerpt.xml");

fn corrected() -> String {
    // K-RG-4: the example closes an element with "</ requireFIPSCompliantMedia>", which is not
    // well-formed XML.
    EXCERPT.replace(
        "</ requireFIPSCompliantMedia>",
        "</requireFIPSCompliantMedia>",
    )
}

#[test]
fn the_example_as_printed_is_not_well_formed() {
    assert!(matches!(
        parse(EXCERPT),
        Err(Error::Xml(commscraft_xml::Error::Syntax(_)))
    ));
}

#[test]
fn parses_the_example_groups() {
    let p = parse(&corrected()).unwrap();
    let names: Vec<&str> = p.groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "ServerConfiguration",
            "ucPolicy",
            "userSetting",
            "persistentChatConfiguration",
            "publicProviders"
        ]
    );
    let sc = p.group("serverconfiguration").unwrap();
    assert_eq!(sc.get("dlxEnabled"), Some("true"));
    // Schema "EnableInCallQoS", prose "enableInCallQos", example "enableInCallQoS" (K-RG-4).
    assert_eq!(sc.get("EnableInCallQoS"), Some("false"));
    assert_eq!(sc.get("enableInCallQos"), Some("false"));
    let uc = p.group("ucPolicy").unwrap();
    assert_eq!(uc.instances.len(), 1);
    assert!(uc.instances[0].contains(&("EnableCallTransfer".into(), "true".into())));
    let chat = p.group("persistentChatConfiguration").unwrap();
    assert_eq!(chat.get("EnablePersistentChat"), Some("true"));
    assert_eq!(
        p.group("publicProviders").unwrap().get("PublicProviders"),
        Some("<PublicProviders>< /PublicProviders>")
    );
    assert_eq!(
        p.group("userSetting").unwrap().get("disableFileTransfer"),
        Some("false")
    );
}

#[test]
fn server_configuration_is_validated() {
    let p = parse(&corrected()).unwrap();
    let c =
        ServerConfiguration::from_group(p.group("ServerConfiguration").unwrap(), "fabrikam.com");
    assert!(c.rejected.is_empty(), "{:?}", c.rejected);
    let abs = c.abs_external.unwrap();
    assert_eq!(
        abs.url,
        "https://clientmainse.fabrikam.com:4443/abs/handler"
    );
    assert_eq!(abs.host, "clientmainse.fabrikam.com");
    assert!(abs.in_user_domain);
    assert_eq!(
        c.dlx_internal.unwrap().url,
        "https://clientmainse.fabrikam.com:443/groupexpansion/service.svc"
    );
    assert_eq!(
        c.mras_uri.as_deref(),
        Some("sip:clientmain_int.fabrikam.com@fabrikam.com;gruu;opaque=srvr:MRAS:lvmY26x3w1WbYUtPiviXXgAA")
    );
    assert_eq!(
        c.focus_factory_uri.as_deref(),
        Some("sip:alice@fabrikam.com;gruu;opaque=app:conf:focusfactory")
    );
    assert_eq!(c.av_encryption, Some(AvEncryption::Require));
    assert_eq!(c.port_range_enabled, Some(false));
    assert_eq!(
        c.media_ports,
        Some(PortRange {
            min: 5350,
            max: 5389
        })
    );
    assert_eq!(c.dlx_enabled, Some(true));
    assert_eq!(c.enable_bw_policy_check, Some(true));

    // The same document seen by a user of another domain: URLs are marked as foreign.
    let foreign =
        ServerConfiguration::from_group(p.group("ServerConfiguration").unwrap(), "contoso.com");
    assert!(!foreign.abs_external.unwrap().in_user_domain);
}

#[test]
fn hostile_values_are_dropped() {
    for bad in [
        "http://abs.fabrikam.com/abs",
        "https://user:pw@abs.fabrikam.com/abs",
        "https://abs.fabrikam.com:0/abs",
        "https://abs.fabrikam.com:65536/abs",
        "https://abs.fabrikam.com:080/abs",
        "https://localhost/abs",
        "https://-bad.fabrikam.com/abs",
        "https://abs.fabrikam.com/a b",
        "https://abs.fabrikam.com/\"x",
        "javascript:alert(1)",
        "https://192.168.1.1/abs",
    ] {
        assert!(service_url(bad, "fabrikam.com").is_none(), "{bad}");
    }
    assert!(
        !service_url("https://fabrikam.com.evil.example/abs", "fabrikam.com")
            .unwrap()
            .in_user_domain
    );
    assert!(
        !service_url("https://evilfabrikam.com/abs", "fabrikam.com")
            .unwrap()
            .in_user_domain
    );
    for bad in [
        "tel:+1",
        "sip:",
        "sip:a@b>;x",
        "sip:a@b\r\nX: y",
        "sip:a b@c",
        "sip:a@b,c",
    ] {
        assert!(sip_uri(bad).is_none(), "{bad:?}");
    }
    let g = Group {
        name: "ServerConfiguration".into(),
        properties: vec![
            ("absInternalServerUrl".into(), "http://abs/abs".into()),
            ("mrasUri".into(), "sip:mras@x>".into()),
            ("ucPC2PCAVEncryption".into(), "Maybe".into()),
            ("ucMinMediaPort".into(), "80".into()),
            ("ucMaxMediaPort".into(), "90".into()),
            ("ucMinAudioPort".into(), "6000".into()),
            ("ucMaxAudioPort".into(), "5000".into()),
            ("dlxEnabled".into(), "yes".into()),
        ],
        instances: vec![],
    };
    let c = ServerConfiguration::from_group(&g, "fabrikam.com");
    assert_eq!(
        c,
        ServerConfiguration {
            rejected: vec![
                "absInternalServerUrl".into(),
                "mrasUri".into(),
                "dlxEnabled".into(),
                "ucPC2PCAVEncryption".into(),
                "ucMediaPorts".into(),
                "ucAudioPorts".into(),
            ],
            ..ServerConfiguration::default()
        }
    );
}

#[test]
fn wrong_documents_are_refused() {
    assert_eq!(parse("<provisionGroupList/>"), Err(Error::WrongDocument));
    assert_eq!(
        parse(&format!("<x xmlns=\"{RESPONSE_NS}\"/>")),
        Err(Error::WrongDocument)
    );
    let dtd = format!("<!DOCTYPE x [<!ENTITY e SYSTEM \"file:///etc/passwd\">]><provisionGroupList xmlns=\"{RESPONSE_NS}\"/>");
    assert_eq!(
        parse(&dtd),
        Err(Error::Xml(commscraft_xml::Error::DtdNotAllowed))
    );
    let nameless = format!(
        "<provisionGroupList xmlns=\"{RESPONSE_NS}\"><provisionGroup/></provisionGroupList>"
    );
    assert_eq!(parse(&nameless), Err(Error::MissingName));
}

fn request() -> Request<'static> {
    Request {
        aor: "sip:alice@fabrikam.com",
        gruu: "sip:alice@fabrikam.com;opaque=user:epid:A3zr0x5_sVSbfPZ-4_JkrgAA;gruu",
        epid: "e48fdaa231",
        from_tag: "77399b3fb7",
        call_id: "dcb605d5e73e44ccbe84dbe4886d210f",
        cseq: 1,
        via: "SIP/2.0/TLS 172.24.32.149:53990;branch=z9hG4bK1",
        user_agent: "CommSCraft/0.0.0001.0",
        subnet: Some("172.24.32.0"),
    }
}

/// RG-40/RG-42, example §4.2.1 (pp. 97–98).
#[test]
fn subscribe_request() {
    let m = subscribe(&request());
    assert_eq!(m.method(), Some("SUBSCRIBE"));
    assert_eq!(m.single("Event").unwrap(), Some(EVENT));
    assert_eq!(m.single("Accept").unwrap(), Some(CONTENT_TYPE));
    assert_eq!(m.expires().unwrap(), Some("0"));
    assert_eq!(
        m.single("Contact").unwrap(),
        Some("<sip:alice@fabrikam.com;opaque=user:epid:A3zr0x5_sVSbfPZ-4_JkrgAA;gruu>")
    );
    let body = std::str::from_utf8(&m.body).unwrap();
    let root = commscraft_xml::parse(body, &commscraft_xml::Limits::default()).unwrap();
    assert_eq!(root.namespace.as_deref(), Some(REQUEST_NS));
    assert_eq!(root.attr("subnet"), Some("172.24.32.0"));
    let groups: Vec<&str> = root
        .children_named("provisioningGroup")
        .filter_map(|g| g.attr("name"))
        .collect();
    assert_eq!(groups, GROUPS);
    // Serializable (no injection through the inputs).
    assert!(m.to_bytes().is_ok());
}

/// Piggybacked answer in the 200 OK (RG-41; headers of example §4.2.2, p. 98).
#[test]
fn piggyback_response() {
    let req = subscribe(&request());
    let mut ok = Message::response(200, "OK");
    for h in ["Via", "From", "Call-ID", "CSeq"] {
        ok.push_header(h, req.single(h).unwrap().unwrap());
    }
    ok.push_header("To", "<sip:alice@fabrikam.com>;tag=24010080");
    ok.push_header("Expires", "0");
    ok.push_header("Content-Type", CONTENT_TYPE);
    ok.push_header("Event", EVENT);
    ok.push_header("subscription-state", "terminated;expires=0");
    ok.push_header("ms-piggyback-cseq", "1");
    ok.body = corrected().into_bytes();
    let p = from_message(&ok).unwrap().unwrap();
    assert_eq!(p.groups.len(), 5);

    let mut empty = ok.clone();
    empty.body.clear();
    assert_eq!(from_message(&empty), Ok(None));
    let mut other = ok.clone();
    other.headers.retain(|h| h.name != "Event");
    assert_eq!(from_message(&other), Err(Error::NotProvisioning));
}

/// RG-51 ([MS-SIPREGE] §2.2.2.5.7, pp. 56–57): the locationPolicy group; EmergencyNumbers wins
/// over EmergencyDialString; masks; bad entries dropped; nothing while disabled.
#[test]
fn location_policy() {
    let g = |props: &[(&str, &str)]| Group {
        name: "locationPolicy".into(),
        properties: props
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect(),
        ..Group::default()
    };
    let p = LocationPolicy::from_group(&g(&[
        ("EnhancedEmergencyServicesEnabled", "true"),
        ("LocationPolicyTagID", "user-tagid"),
        ("LocationRequired", "disclaimer"),
        ("EmergencyDialString", "999"),
        ("EmergencyNumbers", "911-112,100;117;bad<x>-1"),
        (
            "NotificationUri",
            "sip:security@contoso.com, sip:desk@contoso.com,http://x",
        ),
        ("ConferenceUri", "sip:+14255550199@contoso.com;user=phone"),
        ("ConferenceMode", "TwoWay"),
        ("LocationRefreshInterval", "4"),
    ]));
    assert!(p.enabled);
    assert_eq!(p.tag_id.as_deref(), Some("user-tagid"));
    assert_eq!(p.location_required, LocationRequired::Disclaimer);
    assert_eq!(p.numbers.len(), 2, "the bad combination is dropped");
    assert_eq!(
        (p.emergency("911"), p.emergency("112"), p.emergency("100")),
        (Some("911"), Some("911"), Some("911"))
    );
    assert_eq!(p.emergency("117"), Some("117"));
    assert_eq!(
        p.emergency("999"),
        None,
        "EmergencyNumbers takes precedence"
    );
    assert_eq!(p.emergency("0401234"), None);
    assert_eq!(
        p.notification_uris,
        ["sip:security@contoso.com", "sip:desk@contoso.com"]
    );
    assert_eq!(p.conference_mode.as_deref(), Some("twoway"));
    assert_eq!(p.refresh_hours, Some(4));
    let old = LocationPolicy::from_group(&g(&[
        ("EnhancedEmergencyServicesEnabled", "true"),
        ("EmergencyDialString", "911"),
        ("EmergencyDialMask", "112;999"),
    ]));
    assert_eq!(old.emergency("999"), Some("911"));
    let off = LocationPolicy::from_group(&g(&[("EmergencyDialString", "911")]));
    assert_eq!(off.emergency("911"), None, "disabled: no emergency path");
}
