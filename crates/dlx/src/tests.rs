use super::*;

fn envelope(body: &str) -> String {
    format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"http://schemas.xmlsoap.org/soap/envelope/\"><soap:Body>{body}</soap:Body></soap:Envelope>")
}

#[test]
fn request_is_wrapped_and_escaped() {
    let r = search_request("  Müller <&> ").unwrap();
    let root = commscraft_xml::parse(&r, &Limits::default()).unwrap();
    let req = root.child("Body").unwrap().child("SearchAbEntry").unwrap();
    assert_eq!(req.namespace.as_deref(), Some("DistributionListExpander"));
    let r2 = req.child("AbEntryRequest").unwrap();
    let basic = r2.child("BasicSearch").unwrap();
    assert_eq!(basic.child("Value").unwrap().text(), "Müller <&>");
    assert_eq!(basic.child("Verb").unwrap().text(), "BeginsWith");
    let meta = r2.child("Metadata").unwrap();
    assert_eq!(meta.child("FromDialPad").unwrap().text(), "false");
    assert_eq!(meta.child("MaxResultNum").unwrap().text(), "20");
    assert!(meta
        .child("ReturnList")
        .unwrap()
        .text()
        .contains("msRTCSIP-PrimaryUserAddress"));
    assert_eq!(search_request("   "), Err(Error::EmptyQuery));
    assert_eq!(
        search_request(&"a".repeat(MAX_QUERY + 1)),
        Err(Error::QueryTooLong)
    );
}

/// [MS-DLX] 20250520 §4.5 (pp. 44–45): prefix search answer as printed (bare AbEntryResponse).
#[test]
fn example_answer_of_the_specification() {
    let xml = envelope(
        r#"<AbEntryResponse xmlns="DistributionListExpander"><Items>
<AbEntry><Attributes><Attribute><Name>displayname</Name><Value>TZ_tester</Value></Attribute></Attributes><EntryId>79d7099e-a85d-499d-a2c6-32b002937cf4</EntryId><Position>0</Position></AbEntry>
<AbEntry><Attributes><Attribute><Name>displayname</Name><Value>TZ_Grp_manager1</Value></Attribute></Attributes><EntryId>fb0b875d-c25f-4d3d-bfdb-718f4d398dcc</EntryId><Position>0</Position></AbEntry>
</Items><Metadata><ResponseCode>Succeeded</ResponseCode></Metadata></AbEntryResponse>"#,
    );
    let r = parse_response(&xml).unwrap();
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].display_name.as_deref(), Some("TZ_tester"));
    // §4.6 (p. 45): no entry found.
    let none = envelope(
        r#"<AbEntryResponse xmlns="DistributionListExpander"><Items /><Metadata><ResponseCode>NoEntryFound</ResponseCode></Metadata></AbEntryResponse>"#,
    );
    assert_eq!(parse_response(&none).unwrap(), vec![]);
}

#[test]
fn wrapped_answer_with_sip_address_and_duplicates() {
    let entry = r#"<AbEntry><Attributes>
<Attribute><Name>displayName</Name><Value>Bob Builder&#x202e;</Value></Attribute>
<Attribute><Name>msRTCSIP-PrimaryUserAddress</Name><Value>sip:bob@contoso.com</Value></Attribute>
<Attribute><Name>mail</Name><Value>bob@contoso.com</Value></Attribute>
<Attribute><Name>telephoneNumber</Name><Values><string>+49 40 123</string></Values></Attribute>
</Attributes></AbEntry>"#;
    let xml = envelope(&format!(
        r#"<SearchAbEntryResponse xmlns="DistributionListExpander"><SearchAbEntryResult><Items>{entry}{entry}
<AbEntry><Attributes><Attribute><Name>msRTCSIP-PrimaryUserAddress</Name><Value>sip:evil@x.com&gt;;a</Value></Attribute></Attributes></AbEntry>
</Items><Metadata><ResponseCode>Succeeded</ResponseCode></Metadata></SearchAbEntryResult></SearchAbEntryResponse>"#
    ));
    let r = parse_response(&xml).unwrap();
    assert_eq!(r.len(), 2, "duplicate removed");
    assert_eq!(r[0].display_name.as_deref(), Some("Bob Builder"));
    assert_eq!(r[0].sip_uri.as_deref(), Some("sip:bob@contoso.com"));
    assert_eq!(r[0].phone.as_deref(), Some("+49 40 123"));
    assert_eq!(r[1].sip_uri, None, "unsafe SIP URI dropped");
}

#[test]
fn faults_and_hostile_documents() {
    let fault = envelope("<soap:Fault><faultcode>soap:Client</faultcode><faultstring>Access denied</faultstring></soap:Fault>");
    assert_eq!(
        parse_response(&fault),
        Err(Error::Fault("Access denied".into()))
    );
    let err = envelope(
        r#"<AbEntryResponse xmlns="DistributionListExpander"><Metadata><ResponseCode>DatabaseError</ResponseCode></Metadata></AbEntryResponse>"#,
    );
    assert_eq!(
        parse_response(&err),
        Err(Error::Server("DatabaseError".into()))
    );
    assert!(matches!(
        parse_response("<!DOCTYPE x><x/>"),
        Err(Error::Xml(_))
    ));
    assert_eq!(
        parse_response("<a/>"),
        Err(Error::WrongDocument("soap:Envelope"))
    );
    let wrong_ns = envelope(r#"<AbEntryResponse xmlns="urn:other"/>"#);
    assert_eq!(
        parse_response(&wrong_ns),
        Err(Error::WrongDocument("namespace"))
    );
}

/// DL-04: the expansion request carries exactly one escaped `groupMailAddress` in the
/// `DistributionListExpander` namespace; empty and over-long addresses are refused.
#[test]
fn expand_request_form() {
    let r = expand_request(" team&co@contoso.com ").unwrap();
    assert!(r.contains(r#"<ExpandDistributionList xmlns="DistributionListExpander"><groupMailAddress>team&amp;co@contoso.com</groupMailAddress></ExpandDistributionList>"#), "{r}");
    assert_eq!(
        EXPAND_SOAP_ACTION,
        "\"DistributionListExpander/ExpandDistributionList\""
    );
    assert_eq!(expand_request("  "), Err(Error::EmptyQuery));
    assert_eq!(expand_request(&"a".repeat(300)), Err(Error::QueryTooLong));
    assert_eq!(expand_request("a\r\nb"), Err(Error::QueryTooLong));
}

fn expansion(result: &str) -> String {
    format!(
        r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><ExpandDistributionListResponse xmlns="DistributionListExpander"><ExpandDistributionListResult>{result}</ExpandDistributionListResult></ExpandDistributionListResponse></s:Body></s:Envelope>"#
    )
}

/// DL-04: status, users and nested lists (§3.1.4.1.3.3–§3.1.4.1.3.5); unusable SIP URIs are
/// dropped, display text cleaned, unknown states are Invalid.
#[test]
fn expand_answers() {
    let x = parse_expand_response(&expansion(
        r#"<ResponseStatus>Success</ResponseStatus><Users><ActiveDirectoryObjectInfo><displayName>Alice&#x202e;</displayName><mail>alice@contoso.com</mail><mailNickname>alice</mailNickname><sipUri>sip:alice@contoso.com</sipUri></ActiveDirectoryObjectInfo><ActiveDirectoryObjectInfo><displayName>Mallory</displayName><sipUri>sip:m@x.com&gt;;evil</sipUri></ActiveDirectoryObjectInfo></Users><NestedGroups><ActiveDirectoryObjectInfo><displayName>Sub</displayName><mail>sub@contoso.com</mail></ActiveDirectoryObjectInfo></NestedGroups>"#,
    ))
    .unwrap();
    assert_eq!(x.status, ExpandStatus::Success);
    assert_eq!(x.users.len(), 2);
    assert_eq!(x.users[0].display_name.as_deref(), Some("Alice"));
    assert_eq!(x.users[0].sip_uri.as_deref(), Some("sip:alice@contoso.com"));
    assert_eq!(x.users[1].sip_uri, None);
    assert_eq!(x.nested_groups[0].mail.as_deref(), Some("sub@contoso.com"));
    for (state, want) in [
        ("NotFound", ExpandStatus::NotFound),
        ("NotAuthorized", ExpandStatus::NotAuthorized),
        (
            "MemberCountLimitExceeded",
            ExpandStatus::MemberCountLimitExceeded,
        ),
        (
            "SimultaneousRequestLimitExceeded",
            ExpandStatus::SimultaneousRequestLimitExceeded,
        ),
        ("Something", ExpandStatus::Invalid),
    ] {
        let x = parse_expand_response(&expansion(&format!(
            "<ResponseStatus>{state}</ResponseStatus>"
        )))
        .unwrap();
        assert_eq!((x.status, x.users.len()), (want, 0));
    }
}

/// Security by design: wrong envelope or namespace, a fault, and too many members.
#[test]
fn expand_hostile_answers() {
    assert!(matches!(
        parse_expand_response("<a/>"),
        Err(Error::WrongDocument(_))
    ));
    let other_ns = expansion("<ResponseStatus>Success</ResponseStatus>")
        .replace("DistributionListExpander", "urn:other");
    assert!(matches!(
        parse_expand_response(&other_ns),
        Err(Error::WrongDocument(_))
    ));
    let fault = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><s:Fault><faultstring>no</faultstring></s:Fault></s:Body></s:Envelope>"#;
    assert_eq!(parse_expand_response(fault), Err(Error::Fault("no".into())));
    let many: String = (0..1500)
        .map(|i| format!("<ActiveDirectoryObjectInfo><mail>u{i}@contoso.com</mail></ActiveDirectoryObjectInfo>"))
        .collect();
    let x = parse_expand_response(&expansion(&format!(
        "<ResponseStatus>Success</ResponseStatus><Users>{many}</Users>"
    )))
    .unwrap();
    assert_eq!(x.users.len(), MAX_MEMBERS);
}

/// DL-06: PhotoRelPath, PhotoHash and PhotoSize of an entry (example §4.9, p. 47, whose path
/// value starts with a space); a path with a separator or traversal is dropped.
#[test]
fn photo_attributes() {
    let item = |path: &str| {
        format!(
            r#"<AbEntry><Attributes><Attribute><Name>msRTCSIP-PrimaryUserAddress</Name><Value>sip:a@contoso.com</Value></Attribute><Attribute><Name>PhotoRelPath</Name><Value>{path}</Value></Attribute><Attribute><Name>PhotoHash</Name><Value>86B20BBC-648a-44af-97ba-505e9cf7427d</Value></Attribute><Attribute><Name>PhotoSize</Name><Value>3691</Value></Attribute></Attributes></AbEntry>"#
        )
    };
    let answer = |path: &str| {
        envelope(&format!(
            r#"<SearchAbEntryResponse xmlns="DistributionListExpander"><SearchAbEntryResult><Items>{}</Items></SearchAbEntryResult></SearchAbEntryResponse>"#,
            item(path)
        ))
    };
    let r = parse_response(&answer(" e92d7790-3668.86b20bbc-648a.photo")).unwrap();
    let p = r[0].photo.clone().unwrap();
    assert_eq!(p.rel_path, "e92d7790-3668.86b20bbc-648a.photo");
    assert_eq!(
        p.hash.as_deref(),
        Some("86b20bbc-648a-44af-97ba-505e9cf7427d")
    );
    assert_eq!(p.size, Some(3691));
    for bad in ["../secret.photo", "a/b.photo", ".hidden", "a%2fb", "x\\y"] {
        assert_eq!(
            parse_response(&answer(bad)).unwrap()[0].photo,
            None,
            "{bad}"
        );
    }
}
