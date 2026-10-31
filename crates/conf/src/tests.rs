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
        &[McuType::Chat, McuType::AudioVideo],
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
    // PV-12: one entity-view per MCU type, in order (the §4.1 example lists chat, audio-video …).
    let views: Vec<_> = ci
        .child("conference-view")
        .unwrap()
        .elements()
        .filter_map(|n| n.attr("entity"))
        .collect();
    assert_eq!(views, ["chat", "audio-video"]);
    let chat = &[McuType::Chat];
    assert!(add_conference(1, "sip:a@b.c", "sip:f@b.c", "x<y", chat).is_err());
    assert!(add_conference(1, "sip:a@b.c\"", "sip:f@b.c", "ID", chat).is_err());
    // No MCU or one twice: refused before anything is sent.
    assert!(add_conference(1, "sip:a@b.c", "sip:f@b.c", "ID", &[]).is_err());
    assert!(add_conference(
        1,
        "sip:a@b.c",
        "sip:f@b.c",
        "ID",
        &[McuType::Chat, McuType::Chat]
    )
    .is_err());

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

const AV: &str =
    "sip:alice@contoso.com;gruu;opaque=app:conf:audio-video:id:FC275ECD12493E4E9C27C894FE91E104";

/// [MS-CONFBAS] §3.4 example (p. 92): our focus endpoint `on-hold` = lobby; `connected` = admitted.
#[test]
fn lobby_state() {
    let doc = |status: &str, version: u32| {
        format!(
            r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" entity="{FOCUS}" state="partial" version="{version}"><users state="partial"><user entity="sip:bob@fabrikam.com" state="full"><display-text>Bob</display-text><roles><entry>attendee</entry></roles><endpoint entity="{{659dae6c-82aa-46c8-8b37-da684a1a0d06}}" msci:session-type="focus" msci:epid="732A323248" msci:endpoint-uri="sip:bob@fabrikam.com;opaque=user:epid:vAaOGYq2H12kV5u7RWPdEQAA;gruu"><status>{status}</status></endpoint></user></users></conference-info>"#
        )
    };
    let mut r = Roster::default();
    r.apply(&doc("on-hold", 2), FOCUS).unwrap();
    assert!(r.in_lobby("sip:bob@fabrikam.com"));
    assert!(!r.in_lobby("sip:alice@contoso.com"));
    r.apply(&doc("connected", 3), FOCUS).unwrap();
    assert!(!r.in_lobby("sip:bob@fabrikam.com"));
}

/// [MS-CONFAV] §4.3 (pp. 55–56): the A/V endpoint with its audio media and ingress filter; the
/// AVMCU URI from conf-uris (CA-01); own audio keys by GRUU.
#[test]
fn av_roster_and_mute_request() {
    let doc = format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" entity="{FOCUS}" state="full" version="16">
<conference-description><conf-uris><entry><uri>{AV}</uri><purpose>audio-video</purpose></entry></conf-uris></conference-description>
<users state="full"><user entity="sip:bob@fabrikam.com" state="full"><display-text>Bob Freer</display-text><roles><entry>attendee</entry></roles>
<endpoint entity="{{BB7A3FD2-6467-4A42-88D6-4D0488D74A9D}}" msci:session-type="focus" msci:epid="b7b13da6d6" msci:endpoint-uri="sip:bob@fabrikam.com;opaque=user:epid:yIwZ40vZRFqmQ2_rs6oOzQAA;gruu"><status>connected</status></endpoint>
<endpoint entity="{{679ACDDB-2171-4176-8414-6ABA41ED881A}}" msci:session-type="audio-video" msci:endpoint-uri="sip:bob@fabrikam.com;opaque=user:epid:yIwZ40vZRFqmQ2_rs6oOzQAA;gruu"><status>connected</status><joining-method>dialed-in</joining-method>
<media id="1"><type>audio</type><label>main-audio</label><status>sendrecv</status><msci:media-ingress-filter>
block
</msci:media-ingress-filter></media>
<media id="2"><type>video</type><label>main-video</label><status>sendrecv</status></media></endpoint></user></users></conference-info>"#
    );
    let mut r = Roster::default();
    r.apply(&doc, FOCUS).unwrap();
    assert_eq!(r.av_uri.as_deref(), Some(AV));
    assert_eq!(r.participants[0].audio_muted(), Some(true));
    let keys = r
        .own_audio(
            "sip:bob@fabrikam.com",
            "sip:bob@fabrikam.com;opaque=user:epid:yIwZ40vZRFqmQ2_rs6oOzQAA;gruu",
        )
        .unwrap();
    assert_eq!(keys.endpoint, "{679ACDDB-2171-4176-8414-6ABA41ED881A}");
    assert_eq!(keys.media_id, "1");
    let body =
        modify_endpoint_media(96451088, "sip:alice@contoso.com", FOCUS, AV, &keys, false).unwrap();
    assert!(
        body.contains(&format!("<modifyEndpointMedia mscp:mcuUri=\"{AV}\">")),
        "{body}"
    );
    assert!(
        body.contains("endpointEntity=\"{679ACDDB-2171-4176-8414-6ABA41ED881A}\" mediaId=\"1\"")
    );
    // K-CA-3: the filter element in the confinfoextensions namespace (schema, [MS-CONFBAS] p. 220).
    let root = parse(&body);
    let f = root
        .child("modifyEndpointMedia")
        .unwrap()
        .child("media")
        .unwrap()
        .child("media-ingress-filter")
        .unwrap();
    assert_eq!(
        (f.namespace.as_deref(), f.text().as_str()),
        (Some(NS_MSCI), "unblock")
    );
    let bad = AudioKeys {
        endpoint: "\"/><x".into(),
        ..keys
    };
    assert!(modify_endpoint_media(1, "sip:alice@contoso.com", FOCUS, AV, &bad, true).is_err());
}

/// CB-11 / example p. 64: addUser announces lobby-capable.
#[test]
fn add_user_is_lobby_capable() {
    let b = add_user_join(1, "sip:alice@contoso.com", FOCUS, "{G}", Role::Attendee).unwrap();
    assert!(b.contains("<lobby-capable xmlns=\"http://schemas.microsoft.com/rtc/2008/12/confinfoextensions\">true</lobby-capable>"));
    assert!(commscraft_xml::parse(&b, &limits()).is_ok());
}

/// [MS-CONFBAS] §2.2.6.3 example (p. 56).
#[test]
fn meeting_document() {
    let doc = r#"<conf-info xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns="http://schemas.microsoft.com/rtc/2009/05/simplejoinconfdoc">
  <conf-uri>sip:bob@fabrikam.com;gruu;opaque=app:conf:focus:id:MJMAY7RF</conf-uri>
  <server-time>1.0001</server-time>
  <original-incoming-url>https://www.fabrikam.com/meet/bob/MJMAY7RF</original-incoming-url>
  <conf-key>MJMAY7RF</conf-key>
  <fallback-url>https://www.fabrikam.com/meet/bob/MJMAY7RF?sl=1</fallback-url>
  <telemetry-id>00bfd6b5-6cd3-4327-b6e1-7c3aa0deac52</telemetry-id>
</conf-info>"#;
    assert_eq!(
        parse_meeting(doc).unwrap(),
        "sip:bob@fabrikam.com;gruu;opaque=app:conf:focus:id:MJMAY7RF"
    );
    assert!(
        parse_meeting(&doc.replace("app:conf:focus", "app:conf:chat")).is_err(),
        "not a focus"
    );
    assert!(parse_meeting(&doc.replace(
        "sip:bob@fabrikam.com;gruu;opaque=app:conf:focus:id:MJMAY7RF",
        ""
    ))
    .is_err());
    assert!(parse_meeting("<html/>").is_err());
}

/// Presenter commands with the bodies of the specification examples: setLobbyAccess (§4.4.8),
/// modifyUserRoles (§2.2.3.7), modifyConferenceLock (§2.2.3.9), deleteUser (§2.2.3.11) and the
/// audience mute of [MS-CONFAV] §4.5; unsafe URIs refused.
#[test]
fn presenter_commands() {
    let me = "sip:alice@contoso.com";
    let users = vec![
        "sip:bob@fabrikam.com".to_string(),
        "sip:carol@fabrikam.com".to_string(),
    ];
    let b = set_lobby_access(5, me, FOCUS, &users, true).unwrap();
    assert!(b.contains(&format!("<setLobbyAccess><conferenceKeys confEntity=\"{FOCUS}\"/><userEntity>sip:bob@fabrikam.com</userEntity><userEntity>sip:carol@fabrikam.com</userEntity><access>granted</access></setLobbyAccess>")), "{b}");
    assert!(set_lobby_access(5, me, FOCUS, &users, false)
        .unwrap()
        .contains("<access>denied</access>"));
    assert!(set_lobby_access(5, me, FOCUS, &[], true).is_err());
    assert!(set_lobby_access(5, me, FOCUS, &["sip:x@y.z><a/>".to_string()], true).is_err());
    let b = modify_user_roles(6, me, FOCUS, "sip:cathy@fabrikam.com", Role::Presenter).unwrap();
    assert!(b.contains(&format!("<modifyUserRoles><userKeys confEntity=\"{FOCUS}\" userEntity=\"sip:cathy@fabrikam.com\"/><user-roles><entry>presenter</entry></user-roles></modifyUserRoles>")));
    let b = modify_conference_lock(7, me, FOCUS, true).unwrap();
    assert!(b.contains(&format!("<modifyConferenceLock><conferenceKeys confEntity=\"{FOCUS}\"/><locked>true</locked></modifyConferenceLock>")));
    let b = delete_user(8, me, FOCUS, "sip:cathy@fabrikam.com").unwrap();
    assert!(b.contains(&format!("<deleteUser><userKeys confEntity=\"{FOCUS}\" userEntity=\"sip:cathy@fabrikam.com\"/></deleteUser>")));
    let b = mute_audience(9, me, FOCUS, AV, true).unwrap();
    assert!(b.contains(&format!("<modifyConference mscp:mcuUri=\"{AV}\">")));
    assert!(b.contains("<msci:role>default</msci:role><msci:value>false</msci:value>"));
    assert!(
        b.contains("<msci:role>default</msci:role><msci:ingressFilter>block</msci:ingressFilter>")
    );
    assert!(b.contains("<msav:type>audio</msav:type>"));
    for body in [
        &b,
        &modify_user_roles(6, me, FOCUS, "sip:c@d.e", Role::Attendee).unwrap(),
    ] {
        assert!(
            commscraft_xml::parse(body, &limits()).is_ok(),
            "well-formed"
        );
    }
    let b = mute_audience(9, me, FOCUS, AV, false).unwrap();
    assert!(b.contains("<msci:value>true</msci:value></msci:mayModifyOwnFilters><msci:initialFilters><msci:role>presenter</msci:role>"));
    assert!(b.contains(
        "<msci:role>default</msci:role><msci:ingressFilter>unblock</msci:ingressFilter>"
    ));
}

/// Lobby list, presenter role and another participant's audio keys from the roster.
#[test]
fn roster_queries() {
    let doc = format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" entity="{FOCUS}" state="full" version="4"><users>
<user entity="sip:alice@contoso.com" state="full"><roles><entry>presenter</entry></roles><endpoint entity="{{A}}" msci:session-type="focus"><status>connected</status></endpoint></user>
<user entity="sip:bob@fabrikam.com" state="full"><roles><entry>attendee</entry></roles><endpoint entity="{{B}}" msci:session-type="focus"><status>on-hold</status></endpoint></user>
<user entity="sip:carol@fabrikam.com" state="full"><roles><entry>attendee</entry></roles><endpoint entity="{{C}}" msci:session-type="focus"><status>connected</status></endpoint><endpoint entity="{{CAV}}" msci:session-type="audio-video"><status>connected</status><media id="1"><type>audio</type><status>sendrecv</status></media></endpoint></user>
</users></conference-info>"#
    );
    let mut r = Roster::default();
    r.apply(&doc, FOCUS).unwrap();
    assert_eq!(r.lobby(), vec!["sip:bob@fabrikam.com".to_string()]);
    assert!(r.is_presenter("sip:alice@contoso.com"));
    assert!(!r.is_presenter("sip:carol@fabrikam.com"));
    let k = r.audio_of("sip:carol@fabrikam.com").unwrap();
    assert_eq!((k.endpoint.as_str(), k.media_id.as_str()), ("{CAV}", "1"));
    assert!(r.audio_of("sip:bob@fabrikam.com").is_none());
}

/// CI-13: a dial-out INVITE's Ms-Focus-Uri is followed only for the same conference.
#[test]
fn same_conference_rules() {
    let focus = "sip:alice@contoso.com;gruu;opaque=app:conf:focus:id:FC275ECD";
    let chat = "sip:alice@contoso.com;gruu;opaque=app:conf:chat:id:fc275ecd";
    assert!(same_conference(focus, chat));
    assert!(same_conference(focus, focus));
    // Another conference ID, another organizer, not a focus, no conference at all.
    assert!(!same_conference(
        focus,
        "sip:alice@contoso.com;gruu;opaque=app:conf:chat:id:OTHER"
    ));
    assert!(!same_conference(
        focus,
        "sip:mallory@contoso.com;gruu;opaque=app:conf:chat:id:FC275ECD"
    ));
    assert!(!same_conference(chat, chat));
    assert!(!same_conference(focus, "sip:bob@contoso.com"));
    assert!(!same_conference(
        focus,
        "sip:alice@contoso.com;gruu;opaque=app:conf:chat:id:"
    ));
}

mod scheduling {
    use super::*;
    use crate::schedule::*;

    const FF: &str = "sip:alice@contoso.com;gruu;opaque=app:conf:focusfactory";

    fn spec() -> MeetingSpec {
        MeetingSpec {
            conference_id: "SDFBSD12".into(),
            subject: "Plan <Q4> & more".into(),
            expiry: 1_136_913_107,
            admission: Admission::OpenAuthenticated,
            extras: Extras::default(),
        }
    }

    /// PV-13: expiry-time as XML Schema dateTime in UTC; the §4.2 example's value.
    #[test]
    fn datetime() {
        assert_eq!(xs_datetime(0).unwrap(), "1970-01-01T00:00:00Z");
        assert_eq!(xs_datetime(1_136_913_107).unwrap(), "2006-01-10T17:11:47Z");
        assert_eq!(xs_datetime(951_782_400).unwrap(), "2000-02-29T00:00:00Z");
        assert_eq!(
            xs_datetime(253_402_300_799).unwrap(),
            "9999-12-31T23:59:59Z"
        );
        assert!(xs_datetime(253_402_300_800).is_none());
        assert!(xs_datetime(u64::MAX).is_none());
    }

    /// PV-06/PV-09 ([MS-CONFPRO] §2.2.2.1, p. 19; §4.1, p. 41): addConference with subject,
    /// conference ID, expiry time, admission policy, IM and A/V MCU.
    #[test]
    fn schedule_request() {
        let x = schedule_conference(9, "sip:alice@contoso.com", FF, &spec()).unwrap();
        let root = parse(&x);
        assert_eq!(root.attr("requestId"), Some("9"));
        let ci = root
            .child("addConference")
            .unwrap()
            .child("conference-info")
            .unwrap();
        assert_eq!(ci.attr("entity"), Some(""));
        let d = ci.child("conference-description").unwrap();
        assert_eq!(d.child("subject").unwrap().text(), "Plan <Q4> & more");
        assert_eq!(
            d.child("subject").unwrap().namespace.as_deref(),
            Some(NS_CI)
        );
        assert_eq!(d.child("conference-id").unwrap().text(), "SDFBSD12");
        assert_eq!(
            d.child("expiry-time").unwrap().text(),
            "2006-01-10T17:11:47Z"
        );
        assert_eq!(
            d.child("admission-policy").unwrap().namespace.as_deref(),
            Some(NS_MSCI)
        );
        assert_eq!(
            d.child("admission-policy").unwrap().text(),
            "openAuthenticated"
        );
        let views: Vec<_> = ci
            .child("conference-view")
            .unwrap()
            .elements()
            .filter_map(|e| e.attr("entity"))
            .collect();
        assert_eq!(views, ["chat", "audio-video"]);
    }

    /// Hostile or broken values are refused before anything is sent.
    #[test]
    fn schedule_refuses_bad_values() {
        let bad = |f: fn(&mut MeetingSpec)| {
            let mut s = spec();
            f(&mut s);
            schedule_conference(1, "sip:alice@contoso.com", FF, &s).is_err()
        };
        assert!(bad(|s| s.conference_id = "SHORT".into()));
        assert!(bad(|s| s.conference_id = "ABCDEFGH<".into()));
        assert!(bad(|s| s.conference_id = "A".repeat(33)));
        assert!(bad(|s| s.subject = "".into()));
        assert!(bad(|s| s.subject = "a\r\nInjected: x".into()));
        assert!(bad(|s| s.subject = "x".repeat(MAX_SUBJECT + 1)));
        assert!(bad(|s| s.expiry = u64::MAX));
        assert!(get_conference(1, "sip:alice@contoso.com", FF, "x\"/>").is_err());
        assert!(delete_conference(1, "sip:alice@contoso.com", FF, "1234567", false).is_err());
        assert!(modify_conference(1, "sip:alice@contoso.com", FF, 0, &spec()).is_err());
    }

    /// PV-14 (§2.2.2.2, p. 20): modifyConference carries the current version.
    #[test]
    fn modify_request() {
        let x = modify_conference(3, "sip:alice@contoso.com", FF, 2, &spec()).unwrap();
        let root = parse(&x);
        let ci = root
            .child("modifyConference")
            .unwrap()
            .child("conference-info")
            .unwrap();
        assert_eq!(
            (ci.attr("entity"), ci.attr("version")),
            (Some(""), Some("2"))
        );
    }

    /// §2.2.2.3–2.2.2.5 (pp. 20–21), examples §4.3–§4.5: conferenceKeys with the conference ID
    /// in the extension namespace; getConferences without children.
    #[test]
    fn keyed_requests() {
        for (x, cmd) in [
            (
                delete_conference(3, "sip:alice@contoso.com", FF, "BGSDFJ12", false).unwrap(),
                "deleteConference",
            ),
            (
                get_conference(5, "sip:alice@contoso.com", FF, "SDFBSD12").unwrap(),
                "getConference",
            ),
        ] {
            let root = parse(&x);
            let keys = root.child(cmd).unwrap().child("conferenceKeys").unwrap();
            let id = keys
                .attributes
                .iter()
                .find(|a| a.name.ends_with("conference-id"))
                .unwrap();
            assert_eq!(id.value.len(), 8);
            assert!(x.contains(&format!("xmlns:msci=\"{NS_MSCI}\"")));
        }
        let root = parse(&get_conferences(4, "sip:alice@contoso.com", FF, false).unwrap());
        assert_eq!(root.child("getConferences").unwrap().elements().count(), 0);
    }

    /// Example §4.4 (p. 47): the getConferences answer; the second entity contains a blank in the
    /// example ("gruu; opaque") and is therefore not reported as a focus.
    #[test]
    fn parse_conference_list() {
        let x = r#"<response C3PVersion="1" requestId="4" from="sip:alice@contoso.com;gruu;opaque=app:conf:focusfactory" to="sip:alice@contoso.com" code="success" xmlns="urn:ietf:params:xml:ns:cccp" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions"><getConferences><conferences><conference-info entity="sip:alice@contoso.com;gruu;opaque=app:conf:focus:id:SDFBSD12" state="partial" xmlns="urn:ietf:params:xml:ns:conference-info"><conference-description><subject>Conference subject 2</subject><msci:conference-id>SDFBSD12</msci:conference-id><msci:admission-policy>openAuthenticated</msci:admission-policy></conference-description></conference-info><conference-info entity="sip:alice@contoso.com;gruu; opaque=app:conf:focus:id:JASDFD2A" state="partial" xmlns="urn:ietf:params:xml:ns:conference-info"><conference-description><subject>Conference subject</subject><msci:conference-id>JASDFD2A</msci:conference-id><msci:admission-policy>anonymous</msci:admission-policy></conference-description></conference-info></conferences></getConferences></response>"#;
        let (r, list) = parse_meetings(x).unwrap();
        assert_eq!(
            (r.code, r.command.as_str()),
            (Code::Success, "getConferences")
        );
        assert_eq!(list.len(), 2);
        assert_eq!(
            list[0].focus.as_deref(),
            Some("sip:alice@contoso.com;gruu;opaque=app:conf:focus:id:SDFBSD12")
        );
        assert_eq!(list[0].subject.as_deref(), Some("Conference subject 2"));
        assert_eq!(list[1].admission, Some(Admission::Anonymous));
        assert_eq!(list[1].focus, None);
    }

    /// Example §4.5 (p. 49): getConference answer with version, expiry and admission policy; a
    /// join URL is kept only as https.
    #[test]
    fn parse_conference_details() {
        let x = |join: &str| {
            format!(
                r#"<response C3PVersion="1" requestId="5" code="success" xmlns="urn:ietf:params:xml:ns:cccp" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions"><getConference><conference-info entity="" version="2" xmlns="urn:ietf:params:xml:ns:conference-info"><conference-description><subject>Conference subject 2</subject><msci:conference-id>SDFBSD12</msci:conference-id><msci:expiry-time>2006-01-10T17:11:47Z</msci:expiry-time><msci:admission-policy>openAuthenticated</msci:admission-policy><msci:join-url>{join}</msci:join-url></conference-description></conference-info></getConference></response>"#
            )
        };
        let (_, m) = parse_meetings(&x("https://meet.contoso.com/alice/SDFBSD12")).unwrap();
        assert_eq!(m[0].version, Some(2));
        assert_eq!(m[0].expiry.as_deref(), Some("2006-01-10T17:11:47Z"));
        assert_eq!(m[0].admission, Some(Admission::OpenAuthenticated));
        assert_eq!(
            m[0].join_url.as_deref(),
            Some("https://meet.contoso.com/alice/SDFBSD12")
        );
        assert_eq!(m[0].focus, None);
        for bad in [
            "http://meet.contoso.com/x",
            "javascript:alert(1)",
            "https://a b",
        ] {
            assert_eq!(
                parse_meetings(&x(bad)).unwrap().1[0].join_url,
                None,
                "{bad}"
            );
        }
        // A failure answer (example §4.3) parses with its reason and no meetings.
        let fail = r#"<response C3PVersion="1" requestId="3" code="failure" xmlns="urn:ietf:params:xml:ns:cccp"><deleteConference reason="conferenceDoesNotExist"/></response>"#;
        let (r, m) = parse_meetings(fail).unwrap();
        assert_eq!(
            (r.reason.as_deref(), m.len()),
            (Some("conferenceDoesNotExist"), 0)
        );
    }

    /// Hostile answers: entries without a valid conference ID are skipped, the list is bounded.
    #[test]
    fn parse_is_bounded() {
        let one = r#"<conference-info entity="" xmlns="urn:ietf:params:xml:ns:conference-info"><conference-description><msci:conference-id>ABCDEFGH</msci:conference-id></conference-description></conference-info>"#;
        let bad = r#"<conference-info entity="" xmlns="urn:ietf:params:xml:ns:conference-info"><conference-description><msci:conference-id>x</msci:conference-id></conference-description></conference-info>"#;
        let x = format!(
            r#"<response requestId="1" code="success" xmlns="urn:ietf:params:xml:ns:cccp" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions"><getConferences><conferences>{bad}{}</conferences></getConferences></response>"#,
            one.repeat(MAX_MEETINGS + 50)
        );
        let (_, m) = parse_meetings(&x).unwrap();
        assert_eq!(m.len(), MAX_MEETINGS - 1);
    }
}

/// CB-42 ([MS-CONFBAS] §2.2.3.9, second example, p. 42; K-CB-3): lock state plus the three options
/// after the separator, in the confinfoextensions namespace; unknown autopromote bits refused.
#[test]
fn meeting_options_request() {
    let focus = "sip:alice@fabrikam.com;gruu;opaque=app:conf:focus:id:5D3747C";
    let o = MeetingOptions {
        admission: schedule::Admission::OpenAuthenticated,
        autopromote: AUTOPROMOTE_COMPANY,
        pstn_lobby_bypass: false,
    };
    let body = modify_conference_options(12, "sip:bob@fabrikam.com", focus, true, &o).unwrap();
    assert!(body.contains("<locked>true</locked><separator xmlns=\"urn:ietf:params:xml:ns:conference-info-separator\"/><msci:admission-policy>openAuthenticated</msci:admission-policy><msci:autopromote>32768</msci:autopromote><msci:pstn-lobby-bypass>false</msci:pstn-lobby-bypass></modifyConferenceLock>"));
    let root = commscraft_xml::parse(&body, &commscraft_xml::Limits::default()).unwrap();
    let cmd = root.child("modifyConferenceLock").unwrap();
    assert_eq!(
        cmd.child("autopromote").unwrap().namespace.as_deref(),
        Some(NS_MSCI)
    );
    let bad = MeetingOptions {
        autopromote: 1,
        ..o
    };
    assert!(modify_conference_options(1, "sip:bob@fabrikam.com", focus, false, &bad).is_err());
    assert!(modify_conference_options(1, "sip:bob@fabrikam.com", "sip:x\"/>", false, &o).is_err());
}

/// CB-43 ([MS-CONFBAS] §2.2.2.3, pp. 26–28; §2.2.4.4.1, p. 49): settings from the
/// conference-description and the lock state from the focus entity-view; partial documents keep
/// what they do not mention; nonsense values become unknown.
#[test]
fn meeting_settings_from_roster() {
    let focus = "sip:alice@fabrikam.com;gruu;opaque=app:conf:focus:id:5D3747C";
    let full = format!(
        "<conference-info xmlns=\"urn:ietf:params:xml:ns:conference-info\" xmlns:msci=\"{NS_MSCI}\" entity=\"{focus}\" state=\"full\" version=\"1\"><conference-description><msci:admission-policy>openAuthenticated</msci:admission-policy><msci:lobby-capable>true</msci:lobby-capable><msci:anonymous-type-allowed>false</msci:anonymous-type-allowed><msci:autopromote>32768</msci:autopromote><msci:autopromote-allowed>2147516416</msci:autopromote-allowed><msci:pstn-lobby-bypass>false</msci:pstn-lobby-bypass><msci:pstn-lobby-bypass-allowed>true</msci:pstn-lobby-bypass-allowed></conference-description><msci:conference-view><msci:entity-view entity=\"{focus}\"><msci:entity-state><msci:locked>false</msci:locked></msci:entity-state></msci:entity-view></msci:conference-view></conference-info>"
    );
    let mut r = Roster::default();
    assert!(r.apply(&full, focus).unwrap());
    let s = &r.settings;
    assert_eq!(s.locked, Some(false));
    assert_eq!(s.admission, Some(schedule::Admission::OpenAuthenticated));
    assert_eq!(s.lobby_capable, Some(true));
    assert_eq!(s.anonymous_allowed, Some(false));
    assert_eq!(
        s.autopromote_allowed,
        Some(AUTOPROMOTE_COMPANY | AUTOPROMOTE_EVERYONE)
    );
    assert_eq!(
        s.options(),
        Some(MeetingOptions {
            admission: schedule::Admission::OpenAuthenticated,
            autopromote: AUTOPROMOTE_COMPANY,
            pstn_lobby_bypass: false,
        })
    );
    let partial = format!(
        "<conference-info xmlns=\"urn:ietf:params:xml:ns:conference-info\" xmlns:msci=\"{NS_MSCI}\" entity=\"{focus}\" state=\"partial\" version=\"2\"><conference-description><msci:pstn-lobby-bypass>true</msci:pstn-lobby-bypass><msci:autopromote>-1</msci:autopromote></conference-description><msci:conference-view><msci:entity-view entity=\"sip:other@fabrikam.com\"><msci:entity-state><msci:locked>true</msci:locked></msci:entity-state></msci:entity-view></msci:conference-view></conference-info>"
    );
    assert!(r.apply(&partial, focus).unwrap());
    assert_eq!(r.settings.pstn_lobby_bypass, Some(true));
    assert_eq!(r.settings.autopromote, None, "not an unsigned integer");
    assert_eq!(
        r.settings.locked,
        Some(false),
        "another entity's view does not count"
    );
    assert_eq!(
        r.settings.admission,
        Some(schedule::Admission::OpenAuthenticated)
    );
}

/// PV-15 ([MS-CONFPRO] §2.2.1.2–2.2.1.4, pp. 14–16; example §4.2, pp. 42–44; schema
/// conference-description-type, MS-CONFBAS §6.3.2): autopromote and pstn-lobby-bypass after two
/// separators, conference-state/locked, users with one role each; the organizer is left out,
/// duplicates and bad URIs are refused; nothing extra without options.
#[test]
fn meeting_extras() {
    use crate::schedule::*;
    let ff = "sip:alice@contoso.com;gruu;opaque=app:conf:focusfactory";
    let mut spec = MeetingSpec {
        conference_id: "SDFBSD12".into(),
        subject: "Plan".into(),
        expiry: 1_136_913_107,
        admission: Admission::ClosedAuthenticated,
        extras: Extras {
            autopromote: Some(0),
            pstn_lobby_bypass: Some(true),
            locked: Some(true),
            users: vec![
                ("sip:bob@contoso.com".into(), true),
                ("sip:Alice@contoso.com".into(), true),
                ("sip:carol@contoso.com".into(), false),
            ],
        },
    };
    let x = schedule_conference(1, "sip:alice@contoso.com", ff, &spec).unwrap();
    assert!(x.contains("<msci:admission-policy>closedAuthenticated</msci:admission-policy><cis:separator xmlns:cis=\"urn:ietf:params:xml:ns:conference-info-separator\"/><cis:separator xmlns:cis=\"urn:ietf:params:xml:ns:conference-info-separator\"/><msci:autopromote>0</msci:autopromote><msci:pstn-lobby-bypass>true</msci:pstn-lobby-bypass></ci:conference-description><ci:conference-state><ci:locked>true</ci:locked></ci:conference-state><ci:users><ci:user entity=\"sip:bob@contoso.com\"><ci:roles><ci:entry>presenter</ci:entry></ci:roles></ci:user><ci:user entity=\"sip:carol@contoso.com\"><ci:roles><ci:entry>attendee</ci:entry></ci:roles></ci:user></ci:users><msci:conference-view>"), "{x}");
    assert!(commscraft_xml::parse(&x, &commscraft_xml::Limits::default()).is_ok());
    spec.extras
        .users
        .push(("sip:BOB@contoso.com".into(), false));
    assert!(
        schedule_conference(1, "sip:alice@contoso.com", ff, &spec).is_err(),
        "duplicate"
    );
    spec.extras.users = vec![("sip:x@y\"/><evil".into(), true)];
    assert!(schedule_conference(1, "sip:alice@contoso.com", ff, &spec).is_err());
    spec.extras.users = vec![("sip:x@contoso.com".into(), false); MAX_INVITEES + 1];
    assert!(schedule_conference(1, "sip:alice@contoso.com", ff, &spec).is_err());
    spec.extras = Extras {
        autopromote: Some(7),
        ..Extras::default()
    };
    assert!(schedule_conference(1, "sip:alice@contoso.com", ff, &spec).is_err());
    spec.extras = Extras::default();
    let plain = schedule_conference(1, "sip:alice@contoso.com", ff, &spec).unwrap();
    assert!(
        !plain.contains("separator")
            && !plain.contains("conference-state")
            && !plain.contains("users")
    );
}

/// PV-16 (§2.2.2.1, p. 19, example §4.1; §2.2.2.3; §2.2.2.5, example §4.6): the static meeting
/// is created with only its ID and static="true", listed and deleted with static="true"; an
/// answer to the static listing marks its meetings static.
#[test]
fn static_meeting() {
    use crate::schedule::*;
    let ff = "sip:alice@contoso.com;gruu;opaque=app:conf:focusfactory";
    let x = schedule_static(1, "sip:alice@contoso.com", ff, "WJ7UVUYO").unwrap();
    assert!(x.contains("entity=\"\" xmlns:msci=\"http://schemas.microsoft.com/rtc/2005/08/confinfoextensions\" static=\"true\"><ci:conference-description><msci:conference-id>WJ7UVUYO</msci:conference-id></ci:conference-description></ci:conference-info></addConference>"), "{x}");
    assert!(schedule_static(1, "sip:alice@contoso.com", ff, "short").is_err());
    let d = delete_conference(2, "sip:alice@contoso.com", ff, "WJ7UVUYO", true).unwrap();
    assert!(
        d.contains("<deleteConference static=\"true\"><conferenceKeys")
            && d.contains("</deleteConference>")
    );
    assert!(get_conferences(3, "sip:alice@contoso.com", ff, true)
        .unwrap()
        .contains("<getConferences static=\"true\"/>"));
    let answer = r#"<response xmlns="urn:ietf:params:xml:ns:cccp" requestId="3" code="success"><getConferences static="true"><conferences><conference-info xmlns="urn:ietf:params:xml:ns:conference-info" entity="sip:alice@contoso.com;gruu;opaque=app:conf:focus:id:I5TQBR7J" state="partial" version="471"><conference-description><conference-id xmlns="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions">I5TQBR7J</conference-id></conference-description></conference-info></conferences></getConferences></response>"#;
    let (_, meetings) = parse_meetings(answer).unwrap();
    assert_eq!(meetings.len(), 1);
    assert!(meetings[0].static_meeting);
}

/// CA-24: one language tag, no parameters, RFC 5646 shape.
#[test]
fn language_tags() {
    for ok in ["de", "de-DE", "en-US", "zh-Hant-TW", "sr-Latn-RS"] {
        assert!(valid_language_tag(ok), "{ok}");
    }
    for bad in [
        "",
        "d",
        "de_DE",
        "de-DE;q=1",
        "de, en",
        "de-",
        "x".repeat(40).as_str(),
        "de-\r\nX",
    ] {
        assert!(!valid_language_tag(bad), "{bad:?}");
    }
}

/// CA-21 ([MS-CONFAV] §2.2.3.4, example §4.4, p. 59) and CA-23 ([MS-CONFBAS] §2.2.3.15;
/// [MS-CONFAV] §2.2.3.1): the announcements command and the audio dial-out of a phone number;
/// only phone URIs are dialled out.
#[test]
fn announcements_and_phone_dial_out() {
    let focus = "sip:bob@fabrikam.com;gruu;opaque=app:conf:focus:id:YAZ75GMP";
    let av = "sip:bob@fabrikam.com;gruu;opaque=app:conf:audio-video:id:YAZ75GMP";
    let a = modify_conference_announcements(7, "sip:bob@fabrikam.com", focus, av, true).unwrap();
    assert!(a.contains(&format!("<modifyConferenceAnnouncements mscp:mcuUri=\"{av}\"><conferenceKeys confEntity=\"{focus}\"/><enabled>true</enabled></modifyConferenceAnnouncements>")));
    let d = add_user_dial_out_audio(
        8,
        "sip:bob@fabrikam.com",
        focus,
        av,
        "sip:+4940123456@fabrikam.com;user=phone",
        "{G}",
    )
    .unwrap();
    assert!(d.contains("entity=\"sip:+4940123456@fabrikam.com;user=phone\"") && d.contains("<ci:joining-method>dialed-out</ci:joining-method><ci:media id=\"1\"><ci:type>audio</ci:type>"));
    assert!(commscraft_xml::parse(&d, &commscraft_xml::Limits::default()).is_ok());
    assert!(add_user_dial_out_audio(
        8,
        "sip:bob@fabrikam.com",
        focus,
        av,
        "tel:+4940123456",
        "{G}"
    )
    .is_ok());
    for bad in [
        "sip:carol@fabrikam.com",
        "tel:",
        "tel:12a",
        "sip:+49\"/>@x;user=phone",
    ] {
        assert!(
            add_user_dial_out_audio(8, "sip:bob@fabrikam.com", focus, av, bad, "{G}").is_err(),
            "{bad}"
        );
    }
}

/// CA-21 ([MS-CONFAV] §2.2.1.1.4, p. 16): announcements state from the A/V MCU's entity-view;
/// another MCU's view does not count.
#[test]
fn announcements_from_roster() {
    let focus = "sip:bob@fabrikam.com;gruu;opaque=app:conf:focus:id:YAZ75GMP";
    let av = "sip:bob@fabrikam.com;gruu;opaque=app:conf:audio-video:id:YAZ75GMP";
    let doc = format!(
        "<conference-info xmlns=\"urn:ietf:params:xml:ns:conference-info\" xmlns:msci=\"{NS_MSCI}\" xmlns:cmn=\"http://schemas.microsoft.com/rtc/2009/03/commonmcuextensions\" entity=\"{focus}\" state=\"full\" version=\"1\"><conference-description><conf-uris><entry><uri>{av}</uri><purpose>audio-video</purpose></entry></conf-uris></conference-description><msci:conference-view><msci:entity-view entity=\"sip:x@fabrikam.com;gruu;opaque=app:conf:chat:id:1\"><msci:entity-state><cmn:entry-exit-announcements><cmn:enabled>false</cmn:enabled></cmn:entry-exit-announcements></msci:entity-state></msci:entity-view><msci:entity-view entity=\"{av}\"><msci:entity-state><cmn:entry-exit-announcements><cmn:modifiable>true</cmn:modifiable><cmn:enabled>true</cmn:enabled></cmn:entry-exit-announcements></msci:entity-state></msci:entity-view></msci:conference-view></conference-info>"
    );
    let mut r = Roster::default();
    r.apply(&doc, focus).unwrap();
    assert_eq!(r.settings.announcements, Some(true));
    assert_eq!(r.settings.announcements_modifiable, Some(true));
}
