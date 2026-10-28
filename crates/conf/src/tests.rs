use super::*;

const FOCUS: &str =
    "sip:alice@contoso.com;gruu;opaque=app:conf:focus:id:FC275ECD12493E4E9C27C894FE91E104";
const CHAT: &str =
    "sip:alice@contoso.com;gruu;opaque=app:conf:chat:id:FC275ECD12493E4E9C27C894FE91E104";

fn parse(xml: &str) -> Element {
    commscraft_xml::parse(xml, &Limits::default()).unwrap()
}

#[test]
fn ids_and_focus_uris() {
    let r = [0xab; 16];
    assert_eq!(conference_id(&r), "AB".repeat(16));
    assert_eq!(guid(&r), "{ABABABAB-ABAB-ABAB-ABAB-ABABABABABAB}");
    assert!(is_focus_uri(FOCUS));
    assert!(!is_focus_uri(CHAT));
    assert!(!is_focus_uri("sip:a@b;opaque=app:conf:focus:id:1\"><x"));
}

/// [MS-CONFPRO] §4.1 (p. 41): addConference request and success response.
#[test]
fn add_conference_request_and_response() {
    let x = add_conference(
        7,
        "sip:alice@contoso.com",
        "sip:alice@contoso.com;gruu;opaque=app:conf:focusfactory",
        "TPDD8VYG",
    )
    .unwrap();
    let root = parse(&x);
    assert_eq!(
        (root.name.as_str(), root.namespace.as_deref()),
        ("request", Some(NS_CCCP))
    );
    assert_eq!(root.attr("C3PVersion"), Some("1"));
    assert_eq!(root.attr("requestId"), Some("7"));
    let ci = root
        .child("addConference")
        .unwrap()
        .child("conference-info")
        .unwrap();
    assert_eq!(ci.attr("entity"), Some(""));
    let d = ci.child("conference-description").unwrap();
    assert_eq!(d.child("conference-id").unwrap().text(), "TPDD8VYG");
    assert_eq!(
        d.child("conference-id").unwrap().namespace.as_deref(),
        Some(NS_MSCI)
    );
    assert_eq!(
        ci.child("conference-view")
            .unwrap()
            .child("entity-view")
            .unwrap()
            .attr("entity"),
        Some("chat")
    );
    assert!(add_conference(1, "sip:a@b.c", "sip:f@b.c", "x<y").is_err());
    assert!(add_conference(1, "sip:a@b.c\"", "sip:f@b.c", "ID").is_err());

    let ok = r#"<response C3PVersion="1" requestId="1" from="sip:alice@contoso.com;gruu;opaque=app:conf:focusfactory" to="sip:alice@contoso.com" code="success" xmlns="urn:ietf:params:xml:ns:cccp"><addConference><conference-info entity="sip:alice@contoso.com;gruu;opaque=app:conf:focus:id:SDFBSD12" state="partial" version="1" xmlns="urn:ietf:params:xml:ns:conference-info"/></addConference></response>"#;
    let r = parse_response(ok).unwrap();
    assert_eq!(
        (r.request_id, r.code, r.command.as_str()),
        (1, Code::Success, "addConference")
    );
    assert_eq!(
        r.conference.as_deref(),
        Some("sip:alice@contoso.com;gruu;opaque=app:conf:focus:id:SDFBSD12")
    );
    let fail = r#"<response C3PVersion="1" requestId="1" code="failure" xmlns="urn:ietf:params:xml:ns:cccp"><addConference reason="anonymousUsersNotAllowed"/></response>"#;
    let r = parse_response(fail).unwrap();
    assert_eq!(
        (r.code, r.reason.as_deref()),
        (Code::Failure, Some("anonymousUsersNotAllowed"))
    );
    assert!(matches!(
        parse_response("<!DOCTYPE x><response/>"),
        Err(Error::Xml(_))
    ));
    assert_eq!(
        parse_response("<response xmlns=\"urn:x\"/>"),
        Err(Error::WrongDocument("cccp response"))
    );
}

/// [MS-CONFIM] §4.1.1 steps 1 and 4 (pp. 28–30): addUser in the INVITE to the focus.
#[test]
fn join_request_and_granted_role() {
    let x = add_user_join(
        0,
        "sip:carol@contoso.com",
        FOCUS,
        "{34B0C35F-13A6-4DCF-A376-7C4EC2C9017D}",
        Role::Attendee,
    )
    .unwrap();
    let root = parse(&x);
    assert_eq!(root.attr("to"), Some(FOCUS));
    let add = root.child("addUser").unwrap();
    assert_eq!(
        add.child("conferenceKeys").unwrap().attr("confEntity"),
        Some(FOCUS)
    );
    let user = add.child("user").unwrap();
    assert_eq!(user.namespace.as_deref(), Some(NS_CI));
    assert_eq!(user.attr("entity"), Some("sip:carol@contoso.com"));
    assert_eq!(
        user.child("roles").unwrap().child("entry").unwrap().text(),
        "attendee"
    );
    let resp = format!(
        r#"<response xmlns="urn:ietf:params:xml:ns:cccp" xmlns:ci="urn:ietf:params:xml:ns:conference-info" requestId="0" C3PVersion="1" from="{FOCUS}" to="sip:carol@contoso.com" code="success"><addUser><conferenceKeys confEntity="{FOCUS}" /><ci:user entity="sip:carol@contoso.com"><ci:roles><ci:entry>attendee</ci:entry></ci:roles></ci:user></addUser></response>"#
    );
    let r = parse_response(&resp).unwrap();
    assert_eq!(r.user.as_deref(), Some("sip:carol@contoso.com"));
    assert_eq!(r.role.as_deref(), Some("attendee"));
}

/// [MS-CONFIM] §4.1.2 steps 1 and 14 (pp. 41–48): dial-out of a participant.
#[test]
fn dial_out_request_and_pending() {
    let x = add_user_dial_out(
        29137360,
        "sip:alice@contoso.com",
        FOCUS,
        CHAT,
        "sip:leslie@fabrikam.com",
        "{6CD05EA1-25CD-44A5-AAF2-9DA093F5A638}",
    )
    .unwrap();
    let root = parse(&x);
    let add = root.child("addUser").unwrap();
    assert_eq!(add.attr("mscp:mcuUri"), Some(CHAT));
    let ep = add.child("user").unwrap().child("endpoint").unwrap();
    assert_eq!(ep.child("joining-method").unwrap().text(), "dialed-out");
    assert_eq!(
        ep.child("media").unwrap().child("type").unwrap().text(),
        "chat"
    );
    assert!(add_user_dial_out(1, "sip:a@b.c", FOCUS, CHAT, "sip:x@y.z>", "{G}").is_err());
    let pending = r#"<response xmlns="urn:ietf:params:xml:ns:cccp" requestId="29137360" C3PVersion="1" code="pending"><addUser/></response>"#;
    assert_eq!(parse_response(pending).unwrap().code, Code::Pending);
}

fn roster_full() -> String {
    format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" entity="{FOCUS}" state="full" version="8">
<conference-description><conf-uris>
<entry><uri>sip:alice@contoso.com;gruu;opaque=app:conf:meeting:id:FC275ECD12493E4E9C27C894FE91E104</uri><display-text>meeting</display-text><purpose>meeting</purpose></entry>
<entry><uri>{CHAT}</uri><display-text>chat</display-text><purpose>chat</purpose></entry>
</conf-uris></conference-description>
<users state="full">
<user entity="sip:alice@contoso.com" state="full"><display-text>Alice</display-text><roles><entry>presenter</entry></roles>
<endpoint entity="{{1D5E1076-AB39-46C0-8D74-46B79B1DCA27}}" msci:session-type="focus"><status>connected</status></endpoint>
<endpoint entity="{{E9F6FF05-1C46-4D93-889D-9CA1398FFA49}}" msci:session-type="chat"><status>connected</status></endpoint>
</user>
<user entity="sip:bob@contoso.com" state="full"><display-text>Bob&#x202e;</display-text><roles><entry>attendee</entry></roles>
<endpoint entity="{{B1}}" msci:session-type="focus"><status>connected</status></endpoint></user>
</users></conference-info>"#
    )
}

/// [MS-CONFIM] §4.1.1 step 9 (pp. 31–33): full roster with conf-uris; then partial updates.
#[test]
fn roster_full_partial_deleted() {
    let mut r = Roster::default();
    assert!(r.apply(&roster_full(), FOCUS).unwrap());
    assert_eq!(r.chat_uri.as_deref(), Some(CHAT));
    assert_eq!(r.participants.len(), 2);
    assert!(r.participants[0].in_chat());
    assert_eq!(r.participants[1].name.as_deref(), Some("Bob"));
    assert!(!r.participants[1].in_chat());
    // Bob joins the chat MCU (partial, version 9).
    let partial = format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" entity="{FOCUS}" state="partial" version="9"><users state="partial"><user entity="sip:bob@contoso.com" state="partial"><endpoint entity="{{B2}}" msci:session-type="chat"><status>connected</status></endpoint></user></users></conference-info>"#
    );
    assert!(r.apply(&partial, FOCUS).unwrap());
    assert!(r.participants[1].in_chat());
    assert_eq!(
        r.participants[1].name.as_deref(),
        Some("Bob"),
        "kept from the full state"
    );
    // The same version again is ignored.
    assert!(!r.apply(&partial, FOCUS).unwrap());
    let gone = format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" entity="{FOCUS}" state="partial" version="10"><users state="partial"><user entity="sip:bob@contoso.com" state="deleted"/></users></conference-info>"#
    );
    assert!(r.apply(&gone, FOCUS).unwrap());
    assert_eq!(r.participants.len(), 1);
    // Documents of another conference are refused.
    let other = roster_full().replace(
        "opaque=app:conf:focus:id:FC27",
        "opaque=app:conf:focus:id:0000",
    );
    assert_eq!(
        r.apply(&other, FOCUS),
        Err(Error::WrongDocument("other conference"))
    );
}

#[test]
fn roster_is_bounded() {
    let mut users = String::new();
    for i in 0..MAX_PARTICIPANTS + 10 {
        users.push_str(&format!(r#"<user entity="sip:u{i}@contoso.com"/>"#));
    }
    let x = format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" entity="{FOCUS}" state="full" version="1"><users>{users}<user entity="sip:x@y.z&gt;"/></users></conference-info>"#
    );
    let mut r = Roster::default();
    r.apply(&x, FOCUS).unwrap();
    assert_eq!(r.participants.len(), MAX_PARTICIPANTS);
}

/// [MS-CONFIM] §4.2.2 (pp. 57–58): IMDN with one failed recipient.
#[test]
fn imdn_failure() {
    let x = r#"<?xml version="1.0" encoding="utf-8"?><imdn xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns="http://schemas.microsoft.com/rtc/2005/08/imdn">
  <message-id>2</message-id>
    <recipient uri="&lt;sip:bob@contoso.com;opaque=user:epid:d7NL6_mHNFSL3I1rbITi2gAA;gruu&gt;">
      <status>408</status>
      <entry><key>ms-diagnostics</key><value>6001;source="C20-OCG.contoso.com";reason="Request Timeout";component="ImMcu"</value></entry>
    </recipient>
  </imdn>"#;
    let i = parse_imdn(x).unwrap();
    assert_eq!(i.message_id, 2);
    assert_eq!(i.failures, vec![("sip:bob@contoso.com".to_string(), 408)]);
    assert_eq!(message_id(Some(" 2")), 2);
    assert_eq!(message_id(Some("x")), 0);
    assert_eq!(message_id(None), 0);
}

/// [MS-CONFIM] §4.2.1 step 3 (p. 54): Ms-Sender of a forwarded message.
#[test]
fn sender_of_forwarded_messages() {
    let (name, uri) = ms_sender(
        "\"Alice\"<sip:alice@contoso.com;opaque=user:epid:XVjJHDlexlmisnhQrAkWqwAA;gruu>",
    )
    .unwrap();
    assert_eq!(
        (name.as_deref(), uri.as_str()),
        (Some("Alice"), "sip:alice@contoso.com")
    );
    assert_eq!(ms_sender("<sip:bob@contoso.com>").unwrap().0, None);
    assert_eq!(ms_sender("<sip:bad\"@x.y>"), None);
}

/// Invitation (secondary evidence, K-CI-3).
#[test]
fn invitations() {
    let x = format!("<Conferencing version=\"2.0\"><focus-uri>{FOCUS}</focus-uri><subject>Plan\u{202e}</subject><im available=\"true\"><first-im/></im></Conferencing>");
    let i = parse_invitation(&x).unwrap();
    assert_eq!(i.focus, FOCUS);
    assert_eq!(i.subject.as_deref(), Some("Plan"));
    assert!(i.im);
    let not_focus = format!("<Conferencing><focus-uri>{CHAT}</focus-uri></Conferencing>");
    assert_eq!(parse_invitation(&not_focus), Err(Error::UnsafeUri));
    assert!(parse_invitation("<Conferencing/>").is_err());
    assert!(matches!(
        parse_invitation(&"<a>".repeat(5000)),
        Err(Error::Xml(_))
    ));
}
