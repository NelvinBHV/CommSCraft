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
