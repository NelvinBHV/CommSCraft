//! The engine against the test registrar of `commscraft-register` (feature `testing`), which
//! runs the server side of NTLM and signs its messages.

use super::*;
use commscraft_register::testing::{registrar, Counter, Server, NOW};

const AOR: &str = "sip:alice@contoso.com";

fn config() -> Config {
    Config {
        aor: AOR.into(),
        epid: "8248ca9ebb".into(),
        via_host: "192.0.2.1:4320".into(),
        local_ip: "192.0.2.1".parse().unwrap(),
        user_agent: "CommSCraft/test".into(),
        machine_name: "linux".into(),
    }
}

fn sends(out: &[Output]) -> Vec<Message> {
    out.iter()
        .filter_map(|o| match o {
            Output::Send(m) => Some(m.clone()),
            _ => None,
        })
        .collect()
}

fn events(out: &[Output]) -> Vec<Event> {
    out.iter()
        .filter_map(|o| match o {
            Output::Event(e) => Some(e.clone()),
            _ => None,
        })
        .collect()
}

/// An answer of the server to `req`, signed with the server keys.
fn answer(
    server: &mut Server,
    req: &Message,
    code: u16,
    ct: Option<&str>,
    body: &str,
    extra: &[(&str, &str)],
) -> Message {
    let mut r = Message::response(code, "X");
    for name in ["Via", "From", "Call-ID", "CSeq"] {
        for v in req.header_values(name) {
            r.push_header(name, v);
        }
    }
    let to = req.single("To").unwrap().unwrap();
    let to = if to.contains(";tag=") {
        to.to_string()
    } else {
        format!("{to};tag=srv1")
    };
    r.push_header("To", &to);
    r.push_header("Contact", "<sip:pool.contoso.com:5061;transport=tls>");
    for (k, v) in extra {
        r.push_header(k, v);
    }
    if let Some(ct) = ct {
        r.push_header("Content-Type", ct);
    }
    r.body = body.as_bytes().to_vec();
    server.sign(r)
}

/// A request of a peer in the dialog that `ours` (our request) and `theirs` (its 2xx) created.
fn peer_request(
    server: &mut Server,
    ours: &Message,
    method: &str,
    cseq: u32,
    ct: Option<&str>,
    body: &str,
    extra: &[(&str, &str)],
) -> Message {
    let mut m = Message::request(method, "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    m.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKsrv");
    let local = ours.from().unwrap().unwrap();
    let remote = ours.single("To").unwrap().unwrap();
    let remote = if remote.contains(";tag=") {
        remote.to_string()
    } else {
        format!("{remote};tag=srv1")
    };
    m.push_header("From", &remote);
    m.push_header(
        "To",
        &format!("<{}>;tag={}", local.uri, local.param("tag").unwrap()),
    );
    m.push_header("Call-ID", ours.call_id().unwrap().unwrap());
    m.push_header("CSeq", &format!("{cseq} {method}"));
    for (k, v) in extra {
        m.push_header(k, v);
    }
    if let Some(ct) = ct {
        m.push_header("Content-Type", ct);
    }
    m.body = body.as_bytes().to_vec();
    server.sign(m)
}

/// Signs in through the test registrar and returns the bootstrap output.
fn signed_in() -> (Engine<Counter>, Server, Vec<Output>) {
    let mut server = Server::new();
    let mut e = Engine::new(config(), registrar());
    let mut out = e.start(NOW);
    for _ in 0..3 {
        let req = sends(&out).pop().unwrap();
        out = e.on_message(&server.handle(&req), NOW);
    }
    assert!(e.is_registered());
    assert!(matches!(events(&out).first(), Some(Event::Registered(_))));
    (e, server, out)
}

const CONTACTS: &str = r#"<contactList deltaNum="7"><group id="1" name="~"/><contact uri="sip:john@contoso.com" name="John" groups="1"/><contact uri="bob@contoso.com" name="" groups="1"/></contactList>"#;

fn presence_doc(availability: u32) -> String {
    format!(
        "--b1\r\nContent-Type: application/rlmi+xml\r\n\r\n<list xmlns=\"urn:ietf:params:xml:ns:rlmi\" uri=\"sip:alice@contoso.com\" version=\"0\" fullState=\"false\">\
<resource uri=\"sip:bob@contoso.com\"><instance id=\"0\" state=\"resubscribe\" cid=\"bob@contoso.com\" poolFqdn=\"sip:pool2.contoso.com@contoso.com;gruu;opaque=srvr:HomeServer:abc\"/></resource></list>\r\n\
--b1\r\nContent-Type: application/msrtc-event-categories+xml\r\n\r\n<categories xmlns=\"http://schemas.microsoft.com/2006/09/sip/categories\" uri=\"sip:john@contoso.com\">\
<category name=\"state\"><state xsi:type=\"aggregateState\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns=\"http://schemas.microsoft.com/2006/09/sip/state\"><availability>{availability}</availability></state></category></categories>\r\n--b1--\r\n"
    )
}

const MULTIPART: &str =
    "multipart/related; type=\"application/rlmi+xml\";start=resourceList; boundary=b1";

#[test]
fn bootstrap_contacts_presence_and_notify() {
    let (mut e, mut server, out) = signed_in();
    let sent = sends(&out);
    let publish = sent.iter().find(|m| m.method() == Some("SERVICE")).unwrap();
    assert!(server.verify_client(publish), "signed");
    assert!(
        String::from_utf8_lossy(&publish.body).contains("categoryName=\"device\""),
        "PR-33"
    );
    let contacts_sub = sent
        .iter()
        .find(|m| m.single("Event").unwrap() == Some("vnd-microsoft-roaming-contacts"))
        .unwrap();
    assert_eq!(contacts_sub.expires().unwrap(), None, "a real subscription");

    let out = e.on_message(&answer(&mut server, publish, 200, None, "", &[]), NOW);
    assert!(out.is_empty());
    let out = e.on_message(
        &answer(
            &mut server,
            contacts_sub,
            200,
            Some("application/vnd-microsoft-roaming-contacts+xml"),
            CONTACTS,
            &[("Expires", "600")],
        ),
        NOW,
    );
    let ev = events(&out);
    assert!(matches!(ev.first(), Some(Event::ContactList(l)) if l.contacts.len() == 2));
    let batch = sends(&out)
        .into_iter()
        .find(|m| m.single("Event").unwrap() == Some("presence"))
        .unwrap();
    assert_eq!(batch.expires().unwrap(), None);
    assert!(String::from_utf8_lossy(&batch.body).contains("sip:bob@contoso.com"));

    // 200 with the first document: John busy; Bob must be asked at his pool.
    let out = e.on_message(
        &answer(
            &mut server,
            &batch,
            200,
            Some(MULTIPART),
            &presence_doc(6500),
            &[("Expires", "900")],
        ),
        NOW,
    );
    assert!(events(&out).iter().any(|ev| matches!(ev, Event::Presence(p) if p.uri == "sip:john@contoso.com" && p.availability == Some(6500))));
    let pool_sub = sends(&out)
        .into_iter()
        .find(|m| Engine::<Counter>::request_uri(m).is_some_and(|u| u.contains("pool2")))
        .unwrap();
    assert_eq!(
        pool_sub.single("To").unwrap(),
        Some("<sip:alice@contoso.com>")
    );

    // A later NOTIFY in the batch dialog: answered (signed) and reported.
    let notify = peer_request(
        &mut server,
        &batch,
        "NOTIFY",
        2,
        Some(MULTIPART),
        &presence_doc(3500),
        &[
            ("Event", "presence"),
            ("subscription-state", "active;expires=800"),
        ],
    );
    let out = e.on_message(&notify, NOW + 5);
    let ok = sends(&out)
        .into_iter()
        .find(|m| m.status_code() == Some(200))
        .unwrap();
    assert!(server.verify_client(&ok));
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Presence(p) if p.availability == Some(3500))));

    // An unsigned NOTIFY is dropped without an answer.
    let mut forged = notify.clone();
    forged.headers.retain(|h| h.name != "Authentication-Info");
    assert!(e.on_message(&forged, NOW + 6).is_empty());

    // Refresh at 4/5 of the expiry (in the dialog, with To tag).
    let out = e.on_tick(NOW + 720);
    let refresh = sends(&out)
        .into_iter()
        .find(|m| m.single("Event").unwrap() == Some("presence"))
        .unwrap();
    assert!(refresh.single("To").unwrap().unwrap().contains("tag=srv1"));
    assert_eq!(refresh.cseq().unwrap().unwrap().number, 2);
}

#[test]
fn incoming_conversation() {
    let (mut e, mut server, _) = signed_in();
    let mut invite = Message::request("INVITE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    invite.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKinv");
    invite.push_header("From", "<sip:bob@contoso.com>;tag=bob1;epid=123");
    invite.push_header("To", "<sip:alice@contoso.com>");
    invite.push_header("Call-ID", "inc1");
    invite.push_header("CSeq", "1 INVITE");
    invite.push_header("Contact", "<sip:bob@contoso.com;opaque=user:epid:y;gruu>");
    invite.push_header("Content-Type", "application/sdp");
    invite.body =
        b"v=0\r\nm=message 5060 sip null\r\na=accept-types:text/plain text/html\r\n".to_vec();
    let invite = server.sign(invite);
    let out = e.on_message(&invite, NOW);
    let ok = sends(&out).pop().unwrap();
    assert_eq!(ok.status_code(), Some(200));
    assert!(ok.single("To").unwrap().unwrap().contains(";tag="));
    assert!(String::from_utf8_lossy(&ok.body).contains("m=message"));
    assert!(server.verify_client(&ok));
    let Some(Event::ConversationStarted {
        conversation,
        peer,
        incoming: true,
    }) = events(&out).pop()
    else {
        panic!()
    };
    assert_eq!(peer, "sip:bob@contoso.com");
    let local_tag = ok.to().unwrap().unwrap().param("tag").unwrap().to_string();

    let in_dialog = |server: &mut Server, method: &str, ct: &str, body: &str, cseq: u32| {
        let mut m = Message::request(method, "sip:alice@contoso.com;opaque=user:epid:x;gruu");
        m.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKm");
        m.push_header("From", "<sip:bob@contoso.com>;tag=bob1");
        m.push_header("To", &format!("<sip:alice@contoso.com>;tag={local_tag}"));
        m.push_header("Call-ID", "inc1");
        m.push_header("CSeq", &format!("{cseq} {method}"));
        m.push_header("Content-Type", ct);
        m.body = body.as_bytes().to_vec();
        server.sign(m)
    };
    let out = e.on_message(
        &in_dialog(
            &mut server,
            "MESSAGE",
            "text/plain; charset=UTF-8",
            "Hallo Alice",
            2,
        ),
        NOW,
    );
    assert_eq!(
        events(&out),
        vec![Event::Text {
            conversation,
            sender: None,
            text: "Hallo Alice".into()
        }]
    );
    let out = e.on_message(
        &in_dialog(&mut server, "MESSAGE", "text/html", "<b>x</b>", 3),
        NOW,
    );
    assert_eq!(sends(&out).pop().unwrap().status_code(), Some(415));
    let out = e.on_message(&in_dialog(&mut server, "BYE", "text/plain", "", 4), NOW);
    assert_eq!(
        events(&out),
        vec![Event::ConversationEnded { conversation }]
    );
    // Afterwards the dialog is unknown: 481.
    let out = e.on_message(
        &in_dialog(&mut server, "MESSAGE", "text/plain", "late", 5),
        NOW,
    );
    assert_eq!(sends(&out).pop().unwrap().status_code(), Some(481));
}

#[test]
fn outgoing_conversation_with_queued_text() {
    let (mut e, mut server, _) = signed_in();
    let out = e.command(
        Command::StartConversation {
            to: "bob@contoso.com".into(),
        },
        NOW,
    );
    let invite = sends(&out).pop().unwrap();
    assert_eq!(
        Engine::<Counter>::request_uri(&invite),
        Some("sip:bob@contoso.com")
    );
    assert!(server.verify_client(&invite));
    // Written before the session is accepted: queued.
    assert!(e
        .command(
            Command::SendText {
                conversation: 1,
                text: "Hi Bob".into()
            },
            NOW
        )
        .is_empty());
    let ok = answer(
        &mut server,
        &invite,
        200,
        Some("application/sdp"),
        "v=0\r\nm=message 5060 sip null\r\n",
        &[],
    );
    let out = e.on_message(&ok, NOW);
    let sent = sends(&out);
    assert_eq!(sent[0].method(), Some("ACK"));
    assert_eq!(sent[1].method(), Some("MESSAGE"));
    assert_eq!(sent[1].body, b"Hi Bob");
    assert!(events(&out).contains(&Event::ConversationStarted {
        conversation: 1,
        peer: "sip:bob@contoso.com".into(),
        incoming: false
    }));
    let out = e.on_message(&answer(&mut server, &sent[1], 200, None, "", &[]), NOW);
    assert_eq!(events(&out), vec![Event::Delivered { conversation: 1 }]);
    let out = e.command(Command::EndConversation { conversation: 1 }, NOW);
    assert_eq!(sends(&out).pop().unwrap().method(), Some("BYE"));
}

#[test]
fn failures_and_refusals() {
    let (mut e, mut server, _) = signed_in();
    // Busy peer: ACK with the INVITE's branch, conversation failed.
    let out = e.command(
        Command::StartConversation {
            to: "sip:bob@contoso.com".into(),
        },
        NOW,
    );
    let invite = sends(&out).pop().unwrap();
    let out = e.on_message(&answer(&mut server, &invite, 480, None, "", &[]), NOW);
    assert_eq!(sends(&out).pop().unwrap().method(), Some("ACK"));
    assert_eq!(
        events(&out),
        vec![Event::ConversationFailed {
            conversation: 1,
            status: 480
        }]
    );
    // No answer at all: 408 after the INVITE timeout.
    let out = e.command(
        Command::StartConversation {
            to: "sip:carol@contoso.com".into(),
        },
        NOW,
    );
    assert_eq!(sends(&out).len(), 1);
    // At 179 s only the unanswered bootstrap requests have timed out, not the INVITE.
    assert!(!events(&e.on_tick(NOW + 179))
        .iter()
        .any(|ev| matches!(ev, Event::ConversationFailed { .. })));
    assert!(
        events(&e.on_tick(NOW + 181)).contains(&Event::ConversationFailed {
            conversation: 2,
            status: 408
        })
    );
    // An audio call is not supported yet: 488.
    let mut call = Message::request("INVITE", "sip:alice@contoso.com");
    call.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKa");
    call.push_header("From", "<sip:bob@contoso.com>;tag=b9");
    call.push_header("To", "<sip:alice@contoso.com>");
    call.push_header("Call-ID", "call1");
    call.push_header("CSeq", "1 INVITE");
    call.push_header("Contact", "<sip:bob@contoso.com>");
    call.push_header("Content-Type", "application/sdp");
    call.body = b"v=0\r\nm=audio 5000 RTP/SAVP 0\r\n".to_vec();
    let out = e.on_message(&server.sign(call), NOW);
    assert_eq!(sends(&out).pop().unwrap().status_code(), Some(488));
    // Bad address.
    assert!(matches!(
        events(&e.command(
            Command::StartConversation {
                to: "javascript:x".into()
            },
            NOW
        ))
        .pop(),
        Some(Event::Notice(_))
    ));
}

#[test]
fn own_status_with_version_conflict() {
    let (mut e, mut server, _) = signed_in();
    let out = e.command(Command::SetStatus(pres::UserStatus::Busy), NOW);
    let first = sends(&out).pop().unwrap();
    assert!(String::from_utf8_lossy(&first.body).contains("version=\"0\""));
    let fault = r#"<Fault><Faultcode>Client.BadCall.WrongDelta</Faultcode><details><operation index="1" version="0" curVersion="3"/><operation index="2" version="0" curVersion="5"/></details></Fault>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &first,
            409,
            Some("application/msrtc-fault+xml"),
            fault,
            &[],
        ),
        NOW,
    );
    let retry = sends(&out).pop().unwrap();
    let body = String::from_utf8_lossy(&retry.body).to_string();
    assert!(
        body.contains("container=\"2\" version=\"3\"")
            && body.contains("container=\"3\" version=\"5\""),
        "{body}"
    );
    let out = e.on_message(&answer(&mut server, &retry, 200, None, "", &[]), NOW);
    assert_eq!(events(&out), vec![Event::OwnStatus(pres::UserStatus::Busy)]);
    // A second conflict with the same versions is not retried forever.
    let out = e.command(Command::SetStatus(pres::UserStatus::Away), NOW);
    let again = sends(&out).pop().unwrap();
    let fault = r#"<Fault><Faultcode>Client.BadCall.WrongDelta</Faultcode><details><operation index="1" version="4" curVersion="4"/></details></Fault>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &again,
            409,
            Some("application/msrtc-fault+xml"),
            fault,
            &[],
        ),
        NOW,
    );
    assert_eq!(events(&out), vec![Event::OwnStatusFailed(409)]);
}

#[test]
fn sign_out_ends_everything() {
    let (mut e, mut server, out) = signed_in();
    let contacts_sub = sends(&out)
        .into_iter()
        .find(|m| m.single("Event").unwrap() == Some("vnd-microsoft-roaming-contacts"))
        .unwrap();
    e.on_message(
        &answer(
            &mut server,
            &contacts_sub,
            200,
            Some("application/vnd-microsoft-roaming-contacts+xml"),
            "<contactList deltaNum=\"1\"/>",
            &[("Expires", "600")],
        ),
        NOW,
    );
    let out = e.command(Command::SignOut, NOW);
    let sent = sends(&out);
    assert!(
        sent.iter()
            .any(|m| m.method() == Some("SUBSCRIBE") && m.expires().unwrap() == Some("0")),
        "unsubscribe"
    );
    let reg = sent
        .iter()
        .find(|m| m.method() == Some("REGISTER"))
        .unwrap();
    assert_eq!(reg.expires().unwrap(), Some("0"));
    let out = e.on_message(&server.handle(reg), NOW);
    assert_eq!(events(&out), vec![Event::SignedOut]);
    assert!(!e.is_registered());
}

/// RG-40/RG-41: provisioning is fetched at bootstrap; the piggybacked document is checked and
/// web service URLs outside the user's domain are marked.
#[test]
fn provisioning_at_bootstrap() {
    let (mut e, mut server, out) = signed_in();
    let sub = sends(&out)
        .into_iter()
        .find(|m| m.single("Event").unwrap() == Some("vnd-microsoft-provisioning-v2"))
        .unwrap();
    assert_eq!(sub.expires().unwrap(), Some("0"), "piggyback");
    assert!(server.verify_client(&sub));
    let doc = r#"<provisionGroupList xmlns="http://schemas.microsoft.com/2006/09/sip/provisiongrouplist-notification"><provisionGroup name="ServerConfiguration"><absInternalServerUrl>https://pool.contoso.com/abs/int/handler</absInternalServerUrl><dlxInternalUrl>https://pool.contoso.com/groupexpansion/service.svc</dlxInternalUrl><abwqExternalURL>https://evil.example/x</abwqExternalURL></provisionGroup></provisionGroupList>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &sub,
            200,
            Some("application/vnd-microsoft-roaming-provisioning-v2+xml"),
            doc,
            &[("Event", "vnd-microsoft-provisioning-v2")],
        ),
        NOW,
    );
    let ev = events(&out);
    let Some(Event::Provisioned(c)) = ev.first() else {
        panic!("{ev:?}")
    };
    let dlx = c.dlx_internal.as_ref().unwrap();
    assert_eq!(
        dlx.url,
        "https://pool.contoso.com/groupexpansion/service.svc"
    );
    assert!(dlx.in_user_domain);
    assert!(!c.abwq_external.as_ref().unwrap().in_user_domain);
}

const FOCUS: &str =
    "sip:alice@contoso.com;gruu;opaque=app:conf:focus:id:FC275ECD12493E4E9C27C894FE91E104";
const CHAT: &str =
    "sip:alice@contoso.com;gruu;opaque=app:conf:chat:id:FC275ECD12493E4E9C27C894FE91E104";

fn provisioned(e: &mut Engine<Counter>, server: &mut Server, out: &[Output]) {
    let sub = sends(out)
        .into_iter()
        .find(|m| m.single("Event").unwrap() == Some("vnd-microsoft-provisioning-v2"))
        .unwrap();
    let doc = r#"<provisionGroupList xmlns="http://schemas.microsoft.com/2006/09/sip/provisiongrouplist-notification"><provisionGroup name="ServerConfiguration"><focusFactoryUri>sip:alice@contoso.com;gruu;opaque=app:conf:focusfactory</focusFactoryUri></provisionGroup></provisionGroupList>"#;
    e.on_message(
        &answer(
            server,
            &sub,
            200,
            Some("application/vnd-microsoft-roaming-provisioning-v2+xml"),
            doc,
            &[("Event", "vnd-microsoft-provisioning-v2")],
        ),
        NOW,
    );
}

fn find(out: &[Output], method: &str) -> Message {
    sends(out)
        .into_iter()
        .find(|m| m.method() == Some(method))
        .unwrap_or_else(|| panic!("no {method}"))
}

fn roster(version: u32, state: &str, users: &str) -> String {
    format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" entity="{FOCUS}" state="{state}" version="{version}"><conference-description><conf-uris><entry><uri>{CHAT}</uri><purpose>chat</purpose></entry></conf-uris></conference-description><users>{users}</users></conference-info>"#
    )
}

/// [MS-CONFIM] §4.1.1/§4.1.2/§4.2: create, join focus and IM MCU, invite by dial-out, send with
/// 202 + IMDN, receive with Ms-Sender, refresh the focus dialog, leave.
#[test]
fn group_conversation_flow() {
    let (mut e, mut server, out) = signed_in();
    provisioned(&mut e, &mut server, &out);
    let out = e.command(
        Command::StartGroup {
            invite: vec!["sip:bob@contoso.com".into(), "bad\"uri".into()],
        },
        NOW,
    );
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Notice(n) if n.contains("not a usable"))));
    let service = find(&out, "SERVICE");
    assert_eq!(
        Engine::<Counter>::request_uri(&service),
        Some("sip:alice@contoso.com;gruu;opaque=app:conf:focusfactory")
    );
    assert!(server.verify_client(&service));
    let body = String::from_utf8(service.body.clone()).unwrap();
    assert!(body.contains("<addConference>") && body.contains("entity=\"chat\""));
    let created = format!(
        r#"<response xmlns="urn:ietf:params:xml:ns:cccp" C3PVersion="1" requestId="1" code="success"><addConference><conference-info xmlns="urn:ietf:params:xml:ns:conference-info" entity="{FOCUS}" state="partial" version="1"/></addConference></response>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &service,
            200,
            Some("application/cccp+xml"),
            &created,
            &[],
        ),
        NOW,
    );
    let focus_invite = find(&out, "INVITE");
    assert_eq!(Engine::<Counter>::request_uri(&focus_invite), Some(FOCUS));
    assert!(String::from_utf8_lossy(&focus_invite.body).contains("<ci:entry>presenter</ci:entry>"));

    let granted = format!(
        r#"<response xmlns="urn:ietf:params:xml:ns:cccp" xmlns:ci="urn:ietf:params:xml:ns:conference-info" requestId="2" C3PVersion="1" code="success"><addUser><conferenceKeys confEntity="{FOCUS}"/><ci:user entity="sip:alice@contoso.com"><ci:roles><ci:entry>presenter</ci:entry></ci:roles></ci:user></addUser></response>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &focus_invite,
            200,
            Some("application/cccp+xml"),
            &granted,
            &[
                ("Session-Expires", "7200;refresher=uac"),
                ("Require", "timer"),
            ],
        ),
        NOW,
    );
    assert_eq!(
        find(&out, "ACK").call_id().unwrap(),
        focus_invite.call_id().unwrap()
    );
    let roster_sub = find(&out, "SUBSCRIBE");
    assert_eq!(roster_sub.single("Event").unwrap(), Some("conference"));

    let users = r#"<user entity="sip:alice@contoso.com" state="full"><display-text>Alice</display-text><endpoint entity="{A}" msci:session-type="focus"/></user>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &roster_sub,
            200,
            Some("application/conference-info+xml"),
            &roster(1, "full", users),
            &[("Expires", "3348")],
        ),
        NOW,
    );
    assert!(events(&out).iter().any(
        |ev| matches!(ev, Event::Participants { participants, .. } if participants.len() == 1)
    ));
    let chat_invite = find(&out, "INVITE");
    assert_eq!(Engine::<Counter>::request_uri(&chat_invite), Some(CHAT));
    assert!(chat_invite
        .header_values("Supported")
        .any(|v| v == "ms-sender"));

    let out = e.on_message(
        &answer(
            &mut server,
            &chat_invite,
            200,
            Some("application/sdp"),
            "v=0\r\nm=message 5060 sip null\r\na=accept-types:*\r\n",
            &[],
        ),
        NOW,
    );
    let Some(Event::ConversationStarted {
        conversation, peer, ..
    }) = events(&out).into_iter().next()
    else {
        panic!()
    };
    assert_eq!(peer, FOCUS);
    let dial_out = find(&out, "INFO");
    assert_eq!(
        dial_out.call_id().unwrap(),
        focus_invite.call_id().unwrap(),
        "in the focus dialog"
    );
    let b = String::from_utf8(dial_out.body.clone()).unwrap();
    assert!(b.contains("sip:bob@contoso.com") && b.contains("dialed-out") && b.contains(CHAT));
    assert!(
        events(&e.on_message(&answer(&mut server, &dial_out, 202, None, "", &[]), NOW)).is_empty()
    );
    // The final C3P answer as INFO of the focus: failure is reported.
    let fail = r#"<response xmlns="urn:ietf:params:xml:ns:cccp" xmlns:ci="urn:ietf:params:xml:ns:conference-info" requestId="3" C3PVersion="1" code="failure" reason="userDoesntExist"><addUser><ci:user entity="sip:bob@contoso.com"/></addUser></response>"#;
    let info = peer_request(
        &mut server,
        &focus_invite,
        "INFO",
        7,
        Some("application/cccp+xml"),
        fail,
        &[],
    );
    let out = e.on_message(&info, NOW);
    assert_eq!(sends(&out)[0].status_code(), Some(200));
    assert!(matches!(&events(&out)[0], Event::Notice(n) if n.contains("userDoesntExist")));

    // Send: 202 with Message-Id, then the IMDN with one failed recipient.
    let out = e.command(
        Command::SendText {
            conversation,
            text: "Hallo".into(),
        },
        NOW,
    );
    let msg = find(&out, "MESSAGE");
    assert!(e
        .on_message(
            &answer(&mut server, &msg, 202, None, "", &[("Message-Id", "2")]),
            NOW
        )
        .is_empty());
    let imdn = r#"<imdn xmlns="http://schemas.microsoft.com/rtc/2005/08/imdn"><message-id>2</message-id><recipient uri="&lt;sip:bob@contoso.com;gruu&gt;"><status>408</status></recipient></imdn>"#;
    let benotify = peer_request(
        &mut server,
        &chat_invite,
        "BENOTIFY",
        1,
        Some("application/ms-imdn+xml"),
        imdn,
        &[],
    );
    let out = e.on_message(&benotify, NOW);
    assert!(sends(&out).is_empty(), "BENOTIFY is never answered");
    assert_eq!(
        events(&out),
        vec![Event::NotDeliveredTo {
            conversation,
            recipient: "sip:bob@contoso.com".into(),
            status: 408
        }]
    );
    // A second message without IMDN: 408 after 10 s (CI-05).
    let out = e.command(
        Command::SendText {
            conversation,
            text: "Noch da?".into(),
        },
        NOW,
    );
    let msg = find(&out, "MESSAGE");
    e.on_message(
        &answer(&mut server, &msg, 202, None, "", &[("Message-Id", "3")]),
        NOW,
    );
    let out = e.on_tick(NOW + 11);
    assert!(events(&out).contains(&Event::NotDelivered {
        conversation,
        status: 408
    }));

    // Receive: the sender comes from Ms-Sender.
    let fwd = peer_request(
        &mut server,
        &chat_invite,
        "MESSAGE",
        2,
        Some("text/plain; charset=UTF-8;msgr=WAAtAE0A"),
        "Hi all",
        &[(
            "Ms-Sender",
            "\"Bob\"<sip:bob@contoso.com;opaque=user:epid:x;gruu>",
        )],
    );
    let out = e.on_message(&fwd, NOW);
    assert_eq!(
        events(&out),
        vec![Event::Text {
            conversation,
            sender: Some("sip:bob@contoso.com".into()),
            text: "Hi all".into()
        }]
    );

    // Session refresh of the focus dialog after half of 7200 s.
    let out = e.on_tick(NOW + 3601);
    let update = find(&out, "UPDATE");
    assert_eq!(update.call_id().unwrap(), focus_invite.call_id().unwrap());

    // Leave: BYE to the IM MCU, unsubscribe, BYE to the focus.
    let out = e.command(Command::EndConversation { conversation }, NOW + 3602);
    let s = sends(&out);
    let byes: Vec<_> = s.iter().filter(|m| m.method() == Some("BYE")).collect();
    assert_eq!(byes.len(), 2);
    assert!(s
        .iter()
        .any(|m| m.method() == Some("SUBSCRIBE") && m.expires().unwrap() == Some("0")));
    assert!(events(&out).contains(&Event::ConversationEnded { conversation }));
}

/// CI-01: the IM MCU calls us (dial-out) with Ms-Focus-Uri and a session timer we refresh;
/// an invitation (secondary evidence) is only reported, joining is a command.
#[test]
fn dialed_out_to_us_and_invitation() {
    let (mut e, mut server, _) = signed_in();
    let mut invite = Message::request("INVITE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    invite.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKmcu");
    invite.push_header("From", &format!("<{CHAT}>;tag=d52f12304d"));
    invite.push_header("To", "<sip:alice@contoso.com>;epid=8248ca9ebb");
    invite.push_header("Call-ID", "mcu1");
    invite.push_header("CSeq", "5 INVITE");
    invite.push_header("Contact", "<sip:pool.contoso.com:5062;transport=tls>");
    invite.push_header("Referred-By", "<sip:bob@contoso.com>");
    invite.push_header("Ms-Focus-Uri", &format!("<{FOCUS}>"));
    invite.push_header("Supported", "timer");
    invite.push_header("Session-Expires", "600;refresher=uas");
    invite.push_header("Content-Type", "application/sdp");
    invite.body = b"v=0\r\no=- 0 0 IN IP4 0.0.0.0\r\ns=session\r\nc=IN IP4 0.0.0.0\r\nt=0 0\r\nm=message 5060 sip null\r\na=accept-types:*\r\n".to_vec();
    let out = e.on_message(&server.sign(invite), NOW);
    let ok = sends(&out).pop().unwrap();
    assert_eq!(ok.status_code(), Some(200));
    assert_eq!(
        ok.single("Session-Expires").unwrap(),
        Some("600;refresher=uas")
    );
    assert!(ok.header_values("Supported").any(|v| v == "ms-sender"));
    let Some(Event::ConversationStarted {
        conversation,
        peer,
        incoming: true,
    }) = events(&out).pop()
    else {
        panic!()
    };
    assert_eq!(peer, FOCUS);
    let update = find(&e.on_tick(NOW + 300), "UPDATE");
    assert_eq!(update.call_id().unwrap(), Some("mcu1"));
    // No focus dialog: no invitations from here.
    let out = e.command(
        Command::InviteToGroup {
            conversation,
            to: "sip:carol@contoso.com".into(),
        },
        NOW,
    );
    assert!(sends(&out).is_empty());

    let mut inv = Message::request("INVITE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    inv.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKci");
    inv.push_header("From", "<sip:bob@contoso.com>;tag=b1");
    inv.push_header("To", "<sip:alice@contoso.com>");
    inv.push_header("Call-ID", "ci1");
    inv.push_header("CSeq", "1 INVITE");
    inv.push_header("Content-Type", "application/ms-conf-invite+xml");
    inv.body = format!("<Conferencing version=\"2.0\"><focus-uri>{FOCUS}</focus-uri><subject>Plan</subject><im available=\"true\"><first-im/></im></Conferencing>").into_bytes();
    let out = e.on_message(&server.sign(inv), NOW);
    assert_eq!(sends(&out)[0].status_code(), Some(200));
    let Some(Event::ConferenceInvitation { from, invitation }) = events(&out).pop() else {
        panic!()
    };
    assert_eq!(
        (from.as_str(), invitation.focus.as_str()),
        ("sip:bob@contoso.com", FOCUS)
    );
    let out = e.command(
        Command::JoinConference {
            focus: invitation.focus,
        },
        NOW,
    );
    let join = find(&out, "INVITE");
    assert_eq!(Engine::<Counter>::request_uri(&join), Some(FOCUS));
    assert!(String::from_utf8_lossy(&join.body).contains("<ci:entry>attendee</ci:entry>"));
    // A hostile invitation is refused.
    let mut bad = Message::request("INVITE", "sip:alice@contoso.com");
    bad.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKbad");
    bad.push_header("From", "<sip:bob@contoso.com>;tag=b2");
    bad.push_header("To", "<sip:alice@contoso.com>");
    bad.push_header("Call-ID", "ci2");
    bad.push_header("CSeq", "1 INVITE");
    bad.push_header("Content-Type", "application/ms-conf-invite+xml");
    bad.body = b"<!DOCTYPE x><Conferencing/>".to_vec();
    let out = e.on_message(&server.sign(bad), NOW);
    assert_eq!(sends(&out)[0].status_code(), Some(488));
}

/// AV-01/AV-09: after provisioning the engine fetches media relay credentials from the MRAS URI
/// and keeps them; the event carries no secrets.
#[test]
fn media_relay_credentials_at_bootstrap() {
    let (mut e, mut server, out) = signed_in();
    let sub = sends(&out)
        .into_iter()
        .find(|m| m.single("Event").unwrap() == Some("vnd-microsoft-provisioning-v2"))
        .unwrap();
    let mras = "sip:relay.contoso.com@contoso.com;gruu;opaque=srvr:MRAS:OKPDbAVxIEKtPh2g624vPAAA";
    let doc = format!(
        r#"<provisionGroupList xmlns="http://schemas.microsoft.com/2006/09/sip/provisiongrouplist-notification"><provisionGroup name="ServerConfiguration"><mrasUri>{mras}</mrasUri></provisionGroup></provisionGroupList>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &sub,
            200,
            Some("application/vnd-microsoft-roaming-provisioning-v2+xml"),
            &doc,
            &[("Event", "vnd-microsoft-provisioning-v2")],
        ),
        NOW,
    );
    let service = find(&out, "SERVICE");
    assert_eq!(Engine::<Counter>::request_uri(&service), Some(mras));
    assert_eq!(
        service.single("Content-Type").unwrap(),
        Some("application/msrtc-media-relay-auth+xml")
    );
    let body = String::from_utf8(service.body.clone()).unwrap();
    let id = body
        .split("requestID=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string();
    let resp = format!(
        r#"<response requestID="{id}" version="2.0" serverVersion="3.0" reasonPhrase="OK" xmlns="http://schemas.microsoft.com/2006/09/sip/mrasp"><credentialsResponse credentialsRequestID="{id}"><credentials><username>dXNlcg==</username><password>cGFzcw==</password><duration>480</duration></credentials><mediaRelayList><mediaRelay><location>internet</location><hostName>edge.contoso.com</hostName><udpPort>3478</udpPort><tcpPort>443</tcpPort></mediaRelay></mediaRelayList></credentialsResponse></response>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &service,
            200,
            Some("application/msrtc-media-relay-auth+xml"),
            &resp,
            &[],
        ),
        NOW,
    );
    assert_eq!(
        events(&out),
        vec![Event::MediaRelayReady {
            relays: 1,
            minutes: 480
        }]
    );
    assert_eq!(&*e.media_relay().unwrap().password, "cGFzcw==");
    // Refreshed at 90 % of the lifetime (the registration itself would need refreshing first in
    // this test, so the timer is checked directly).
    assert_eq!(e.media_relay_refresh_at, Some(NOW + 480 * 54));
}
