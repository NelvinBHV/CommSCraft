use super::*;

/// [MS-PRES] 20250520 §4.5 (pp. 175–177): first batch 200 OK, PDF line breaks joined; the
/// calendarData part is left out (not evaluated yet).
fn batch_200() -> Message {
    let body = "--d20e87acad9e400aac184344edb29a4a\r\n\
Content-Transfer-Encoding: binary\r\n\
Content-ID: resourceList\r\n\
Content-Type: application/rlmi+xml\r\n\
\r\n\
<list xmlns=\"urn:ietf:params:xml:ns:rlmi\" uri=\"sip:alice@contoso.com\" version=\"0\" fullState=\"false\"/>\r\n\
--d20e87acad9e400aac184344edb29a4a\r\n\
Content-Transfer-Encoding: binary\r\n\
Content-Type: application/msrtc-event-categories+xml\r\n\
\r\n\
<categories xmlns=\"http://schemas.microsoft.com/2006/09/sip/categories\" uri=\"sip:john@contoso.com\">\
<category name=\"contactCard\" instance=\"0\" publishTime=\"2008-01-22T23:41:04.577\">\
<contactCard xmlns=\"http://schemas.microsoft.com/2006/09/sip/contactcard\"><identity><name><displayName>john</displayName></name></identity></contactCard></category>\
<category name=\"note\"/>\
<category name=\"state\" instance=\"1\" publishTime=\"2008-01-24T02:04:35.327\">\
<state xsi:type=\"aggregateState\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns=\"http://schemas.microsoft.com/2006/09/sip/state\"><availability>6500</availability></state></category>\
<category name=\"services\" instance=\"0\" publishTime=\"2008-01-24T02:04:35.327\">\
<services xmlns=\"http://schemas.microsoft.com/2006/09/sip/service\"><service uri=\"sip:john@contoso.com\"><capabilities><text render=\"true\" capture=\"true\" deviceAvailability=\"3500\"/></capabilities></service></services></category>\
</categories>\r\n\
--d20e87acad9e400aac184344edb29a4a--\r\n";
    let mut m = Message::response(200, "OK");
    m.push_header(
        "Content-Type",
        "multipart/related; type=\"application/rlmi+xml\";start=resourceList; boundary=d20e87acad9e400aac184344edb29a4a",
    );
    m.push_header("Event", "presence");
    m.body = body.as_bytes().to_vec();
    m
}

#[test]
fn batch_answer_of_the_example() {
    let (states, contacts) = parse_batch(&batch_200()).unwrap();
    assert!(states.is_empty(), "fullState=false list without resources");
    assert_eq!(contacts.len(), 1);
    let john = &contacts[0];
    assert_eq!(john.uri, "sip:john@contoso.com");
    assert_eq!(john.availability, Some(6500));
    assert_eq!(john.class(), Availability::Busy);
    assert_eq!(john.display_name.as_deref(), Some("john"));
    assert_eq!(john.note, None);
}

/// §4.5 (p. 179) prints the second answer without the closing delimiter; it is still split.
#[test]
fn missing_close_delimiter_is_tolerated_but_garbage_is_not() {
    let mut m = batch_200();
    let body = String::from_utf8(m.body.clone()).unwrap();
    m.body = body
        .replace("--d20e87acad9e400aac184344edb29a4a--\r\n", "")
        .into_bytes();
    assert_eq!(parse_batch(&m).unwrap().1.len(), 1);
    assert_eq!(
        split_multipart("--b\r\nno blank line", "b"),
        Err(Error::BadMultipart)
    );
    assert_eq!(
        split_multipart("x", "bad boundary"),
        Err(Error::BadMultipart)
    );
    assert_eq!(
        split_multipart("x", &"b".repeat(71)),
        Err(Error::BadMultipart)
    );
    let many = "--b\r\nContent-Type: x\r\n\r\ny\r\n".repeat(MAX_PARTS + 1);
    assert_eq!(split_multipart(&many, "b"), Err(Error::BadMultipart));
}

#[test]
fn availability_classes() {
    for (n, c) in [
        (0, Availability::Unknown),
        (3500, Availability::Online),
        (5000, Availability::Idle),
        (6500, Availability::Busy),
        (7500, Availability::BusyIdle),
        (9500, Availability::DoNotDisturb),
        (15500, Availability::Away),
        (18500, Availability::Offline),
    ] {
        assert_eq!(Availability::from_number(n), c, "{n}");
    }
    assert_eq!(UserStatus::parse("DND"), Some(UserStatus::DoNotDisturb));
    assert_eq!(UserStatus::parse("invisible"), None);
}

/// Publish body (PR-11, PR-31; layout of §4.2.1, p. 135, with the state category of §4.3.1.1).
#[test]
fn publish_body_is_valid_and_endpoint_scoped() {
    let b = publish_state_body(
        "sip:bob@contoso.com",
        UserStatus::Busy,
        [0, 4],
        Some("221EF77E-3A68-5570-86ED-6EA5BD4B7FF8"),
    )
    .unwrap();
    let root = commscraft_xml::parse(&b, &Limits::default()).unwrap();
    assert_eq!(root.namespace.as_deref(), Some(NS_PUBLISH));
    let pubs: Vec<&Element> = root
        .child("publications")
        .unwrap()
        .children_named("publication")
        .collect();
    assert_eq!(pubs.len(), 2);
    assert_eq!(pubs[0].attr("container"), Some("2"));
    assert_eq!(pubs[1].attr("container"), Some("3"));
    assert_eq!(pubs[1].attr("version"), Some("4"));
    assert_eq!(pubs[0].attr("expireType"), Some("endpoint"));
    assert_eq!(pubs[0].attr("instance"), Some("603979776"));
    let st = pubs[0].child("state").unwrap();
    assert_eq!(st.attr("xsi:type"), Some("userState"));
    assert_eq!(st.child("availability").unwrap().text(), "6500");
    assert!(
        publish_state_body("sip:bob@contoso.com\"><x/>", UserStatus::Busy, [0, 0], None).is_err()
    );
    assert!(publish_state_body(
        "sip:bob@contoso.com",
        UserStatus::Busy,
        [0, 0],
        Some("1\"/>")
    )
    .is_err());
}

/// §4.2.1 (p. 138): the 409 fault; K-PR-1 ("Protocol client." prefix in the example).
#[test]
fn version_conflict_fault() {
    let fault = r#"<Fault><Faultcode>Protocol client.BadCall.WrongDelta</Faultcode><details>
<operation index="1" version="1" curVersion="2"><note xmlns="http://schemas.microsoft.com/2006/09/sip/note"><body type="personal" uri="">Working until 5pm today</body></note></operation>
<operation index="2" version="1" curVersion="2"/></details></Fault>"#;
    assert_eq!(
        parse_fault(fault).unwrap(),
        vec![
            VersionConflict {
                index: 1,
                version: 1,
                cur_version: 2
            },
            VersionConflict {
                index: 2,
                version: 1,
                cur_version: 2
            }
        ]
    );
    assert!(parse_fault("<Fault><Faultcode>Server.Other</Faultcode></Fault>").is_err());
}

/// §4.5 (p. 175): the batch body, with checked and escaped URIs.
#[test]
fn batch_request_body() {
    let b = batch_body(
        "sip:alice@contoso.com",
        &["sip:john@contoso.com".into(), "bob@contoso.com".into()],
        1177624,
    )
    .unwrap();
    let root = commscraft_xml::parse(&b, &Limits::default()).unwrap();
    assert_eq!(root.namespace.as_deref(), Some(NS_BATCH));
    let action = root.child("action").unwrap();
    assert_eq!(action.attr("name"), Some("subscribe"));
    let res: Vec<&str> = action
        .child("adhocList")
        .unwrap()
        .children_named("resource")
        .filter_map(|r| r.attr("uri"))
        .collect();
    assert_eq!(res, ["sip:john@contoso.com", "sip:bob@contoso.com"]);
    let cats: Vec<&str> = action
        .child("categoryList")
        .unwrap()
        .children_named("category")
        .filter_map(|c| c.attr("name"))
        .collect();
    assert_eq!(cats, CATEGORIES);
    assert_eq!(
        batch_body("sip:a@b.c", &["sip:x@y.z\"/><evil/>".into()], 1),
        Err(Error::UnsafeUri)
    );
    let too_many: Vec<String> = (0..=MAX_BATCH)
        .map(|i| format!("sip:u{i}@contoso.com"))
        .collect();
    assert_eq!(
        batch_body("sip:a@b.c", &too_many, 1),
        Err(Error::TooManyResources)
    );
    let mut m = Message::request("SUBSCRIBE", "sip:alice@contoso.com");
    batch_headers(&mut m, true);
    assert_eq!(m.expires().unwrap(), Some("0"));
    assert_eq!(
        m.single("Require").unwrap(),
        Some("adhoclist, categoryList")
    );
}

/// [MS-SIP] example (p. 49, trailing spaces in URIs) and [MS-PRES] §4.14.1 (p. 197).
#[test]
fn contact_lists_of_the_examples() {
    let sip = r#"<contactList deltaNum="685" >
  <group id="1" name="~" externalURI="" />
  <group id="2" name="Team" externalURI="" />
  <contact uri="contact1@tradewind.com" name="" groups="1 3 " subscribed="true" externalURI="" />
  <contact uri="contact2@tradewind.com " name="" groups="1 2 " subscribed="true" externalURI="" />
</contactList>"#;
    let l = parse_contact_list(sip).unwrap();
    assert_eq!(l.delta_num, Some(685));
    assert_eq!(
        l.groups[1],
        Group {
            id: 2,
            name: "Team".into()
        }
    );
    assert_eq!(l.contacts[1].uri, "sip:contact2@tradewind.com");
    assert_eq!(l.contacts[0].groups, [1, 3]);

    // §4.14.1 contains "+14252057570@ contoso.com" (space inside the URI): skipped, not guessed.
    let ucs = r#"<contactList deltaNum="16" ucsMode="allowed" >
  <group id="1" name="Group 1 Name" externalURI="" />
  <contact uri="user0@contoso.com" name="" groups="1" subscribed="true" externalURI="" />
  <contact uri="+14252057570@ contoso.com" name="" groups="1" subscribed="true" externalURI="" >
    <contactExtension><contactSettings></contactSettings><phone><uri>tel:+14252057570</uri></phone></contactExtension>
  </contact>
</contactList>"#;
    let l = parse_contact_list(ucs).unwrap();
    assert_eq!(l.ucs_mode.as_deref(), Some("allowed"));
    assert_eq!(l.contacts.len(), 1);
    assert_eq!(l.skipped, ["+14252057570@ contoso.com"]);
}

#[test]
fn hostile_presence_data_is_neutralised() {
    let doc = "<categories xmlns=\"http://schemas.microsoft.com/2006/09/sip/categories\" uri=\"sip:eve@contoso.com\">\
<category name=\"contactCard\"><contactCard xmlns=\"x\"><identity><name><displayName>Eve&#x9b;2J&#x202e;evil</displayName></name></identity></contactCard></category>\
<category name=\"note\"><note xmlns=\"n\"><body>hi&#x85;</body></note></category></categories>";
    let p = parse_categories(doc).unwrap();
    // C1 CSI (U+009B, allowed in XML 1.0) and a bidi override are removed; C0 escapes such as
    // ESC are already refused by the XML reader.
    assert_eq!(p.display_name.as_deref(), Some("Eve2Jevil"));
    assert!(parse_categories("<categories xmlns=\"http://schemas.microsoft.com/2006/09/sip/categories\" uri=\"sip:a@b.c\">&#x1b;</categories>").is_err());
    assert_eq!(p.note.as_deref(), Some("hi"));
    assert!(parse_categories("<categories xmlns=\"urn:other\" uri=\"sip:a@b.c\"/>").is_err());
    assert!(parse_categories("<categories xmlns=\"http://schemas.microsoft.com/2006/09/sip/categories\" uri=\"javascript:x\"/>").is_err());
    assert_eq!(printable(&"x".repeat(1000)).len(), 256);
}

/// §3.4.5.1.3.1 (p. 102): resubscribe with and without poolFqdn; quota exceeded.
#[test]
fn rlmi_rejections() {
    let rlmi = r#"<list xmlns="urn:ietf:params:xml:ns:rlmi" uri="sip:bob@contoso.com" version="0" fullState="false">
<resource uri="sip:alice@contoso.com"><instance id="0" state="resubscribe" cid="alice@contoso.com"
 poolFqdn="sip:server.contoso.com@contoso.com;gruu;opaque=srvr:HomeServer:dL8cwxBrTuG8eC4-Q_GNGAAA"/></resource>
<resource uri="sip:john@fabrikam.com"><instance id="0" state="resubscribe" cid="john@fabrikam.com"/></resource>
<resource uri="sip:eve@contoso.com"><instance id="0" state="resubscribe" cid="eve@contoso.com" poolFqdn="sip:x@y.z&gt;;evil"/></resource>
<resource uri="sip:alice2@contoso.com"><instance id="0" state="terminated" reason="quotaExceeded" statusCode="413" cid="alice2@contoso.com"/></resource>
</list>"#;
    let r = parse_rlmi(rlmi).unwrap();
    assert_eq!(r[0].pool_fqdn.as_deref(), Some("sip:server.contoso.com@contoso.com;gruu;opaque=srvr:HomeServer:dL8cwxBrTuG8eC4-Q_GNGAAA"));
    assert_eq!(r[1].pool_fqdn, None);
    assert_eq!(r[2].pool_fqdn, None, "unsafe pool URI dropped");
    assert_eq!(r[3].reason.as_deref(), Some("quotaExceeded"));
    let mut m = Message::request("SUBSCRIBE", "sip:john@fabrikam.com");
    single_headers(&mut m, true);
    assert_eq!(m.single("Require").unwrap(), None);
}

/// PR-31/PR-33: device + machineState into container 2, endpoint-scoped.
#[test]
fn endpoint_publication() {
    let id = "221EF77E-3A68-5570-86ED-6EA5BD4B7FF8";
    let b =
        publish_endpoint_body("sip:bob@contoso.com", id, "linux\u{1b}box<", 3500, [0, 2]).unwrap();
    let root = commscraft_xml::parse(&b, &Limits::default()).unwrap();
    let pubs: Vec<&Element> = root
        .child("publications")
        .unwrap()
        .children_named("publication")
        .collect();
    assert_eq!(pubs.len(), 2);
    assert_eq!(pubs[0].attr("categoryName"), Some("device"));
    assert_eq!(pubs[0].attr("container"), Some("2"));
    assert_eq!(pubs[1].attr("version"), Some("2"));
    assert!(pubs
        .iter()
        .all(|p| p.attr("expireType") == Some("endpoint") && p.attr("endpointId") == Some(id)));
    let dev = pubs[0].child("device").unwrap();
    assert_eq!(dev.child("machineName").unwrap().text(), "linuxbox<");
    assert!(dev.child("capabilities").unwrap().child("text").is_some());
    assert_eq!(
        pubs[1].child("state").unwrap().attr("xsi:type"),
        Some("machineState")
    );
    assert_eq!(
        endpoint_instances(id),
        endpoint_instances(&id.to_lowercase()),
        "stable"
    );
    assert!(publish_endpoint_body("sip:bob@contoso.com", "x\"/>", "m", 3500, [0, 0]).is_err());
    assert_eq!(
        contact_document_kind("<contactDelta deltaNum=\"2\" prevDeltaNum=\"1\"/>"),
        ContactDocument::Delta
    );
    assert_eq!(
        contact_document_kind("<contactList deltaNum=\"2\"/>"),
        ContactDocument::Full
    );
}
