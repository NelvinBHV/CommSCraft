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
    // 1:1 sessions offer plain text only (CI-14 applies to IM MCU sessions).
    assert!(String::from_utf8_lossy(&invite.body).contains("a=accept-types:text/plain\r\n"));
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
    // A video-only or other non-audio, non-IM session is refused with 488.
    let mut call = Message::request("INVITE", "sip:alice@contoso.com");
    call.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKa");
    call.push_header("From", "<sip:bob@contoso.com>;tag=b9");
    call.push_header("To", "<sip:alice@contoso.com>");
    call.push_header("Call-ID", "call1");
    call.push_header("CSeq", "1 INVITE");
    call.push_header("Contact", "<sip:bob@contoso.com>");
    call.push_header("Content-Type", "application/sdp");
    call.body = b"v=0\r\nm=video 5000 RTP/SAVP 122\r\n".to_vec();
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

/// PV-12 ([MS-CONFPRO] §2.2.3.1, p. 23): a focus factory without the A/V MCU refuses with
/// `mcuTypeNotAvailable`; the conference is created once more with the IM MCU only, and a second
/// refusal ends the attempt (no loop).
#[test]
fn group_without_av_mcu_falls_back_to_chat_once() {
    let (mut e, mut server, out) = signed_in();
    provisioned(&mut e, &mut server, &out);
    let out = e.command(
        Command::StartGroup {
            invite: vec!["sip:bob@contoso.com".into()],
        },
        NOW,
    );
    let first = find(&out, "SERVICE");
    assert!(String::from_utf8_lossy(&first.body).contains("entity=\"audio-video\""));
    let refused = r#"<response xmlns="urn:ietf:params:xml:ns:cccp" C3PVersion="1" requestId="1" code="failure"><addConference reason="mcuTypeNotAvailable"/></response>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &first,
            200,
            Some("application/cccp+xml"),
            refused,
            &[],
        ),
        NOW,
    );
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Notice(n) if n.contains("no conference audio"))));
    let second = find(&out, "SERVICE");
    let body = String::from_utf8_lossy(&second.body).to_string();
    assert!(body.contains("entity=\"chat\"") && !body.contains("audio-video"));
    let refused2 = r#"<response xmlns="urn:ietf:params:xml:ns:cccp" C3PVersion="1" requestId="2" code="failure"><addConference reason="mcuTypeNotAvailable"/></response>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &second,
            200,
            Some("application/cccp+xml"),
            refused2,
            &[],
        ),
        NOW,
    );
    assert!(!out
        .iter()
        .any(|o| matches!(o, Output::Send(m) if m.method() == Some("SERVICE"))));
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Notice(n) if n.contains("conference not created"))));
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
    // PV-12: the A/V MCU is allocated with the conference, so audio can be added later.
    assert!(body.contains("entity=\"audio-video\""));
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
    // CI-14: towards the IM MCU the IMDN type is announced (the 202 + IMDN path of CI-05).
    assert!(String::from_utf8_lossy(&chat_invite.body)
        .contains("a=accept-types:text/plain application/ms-imdn+xml"));
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

/// Function 40 (PV-06, PV-09, PV-13; [MS-CONFPRO] §2.2.2, examples §4.1–§4.5): schedule a
/// meeting, read its details (join URL), list, cancel; refusals and bad input are reported.
#[test]
fn schedule_list_show_cancel_meeting() {
    use commscraft_conf::schedule::Admission;
    let (mut e, mut server, out) = signed_in();
    provisioned(&mut e, &mut server, &out);
    let ff = "sip:alice@contoso.com;gruu;opaque=app:conf:focusfactory";
    // A lifetime of zero minutes is refused locally.
    let out = e.command(
        Command::ScheduleMeeting {
            subject: "Plan".into(),
            minutes: 0,
            admission: Admission::OpenAuthenticated,
        },
        NOW,
    );
    assert!(sends(&out).is_empty());
    let out = e.command(
        Command::ScheduleMeeting {
            subject: "Plan Q4".into(),
            minutes: 60,
            admission: Admission::ClosedAuthenticated,
        },
        NOW,
    );
    let add = find(&out, "SERVICE");
    assert_eq!(Engine::<Counter>::request_uri(&add), Some(ff));
    let body = String::from_utf8(add.body.clone()).unwrap();
    assert!(body.contains("<addConference>") && body.contains("<ci:subject>Plan Q4</ci:subject>"));
    assert!(body.contains("closedAuthenticated") && body.contains("entity=\"audio-video\""));
    let id = body
        .split("<msci:conference-id>")
        .nth(1)
        .and_then(|r| r.split('<').next())
        .unwrap()
        .to_string();
    assert_eq!(id.len(), 32);
    let created = format!(
        r#"<response xmlns="urn:ietf:params:xml:ns:cccp" C3PVersion="1" requestId="1" code="success"><addConference><conference-info xmlns="urn:ietf:params:xml:ns:conference-info" entity="sip:alice@contoso.com;gruu;opaque=app:conf:focus:id:{id}" state="partial" version="1"/></addConference></response>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &add,
            200,
            Some("application/cccp+xml"),
            &created,
            &[],
        ),
        NOW,
    );
    let Some(Event::MeetingScheduled(m)) = events(&out)
        .into_iter()
        .find(|ev| matches!(ev, Event::MeetingScheduled(_)))
    else {
        panic!("no MeetingScheduled")
    };
    assert_eq!(m.conference_id, id);
    assert!(m
        .focus
        .as_deref()
        .unwrap()
        .ends_with(&format!("focus:id:{id}")));
    assert_eq!(m.admission, Some(Admission::ClosedAuthenticated));
    // The details are asked for right away (join URL).
    let get = find(&out, "SERVICE");
    assert!(String::from_utf8_lossy(&get.body).contains("<getConference>"));
    let details = format!(
        r#"<response xmlns="urn:ietf:params:xml:ns:cccp" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" C3PVersion="1" requestId="2" code="success"><getConference><conference-info xmlns="urn:ietf:params:xml:ns:conference-info" entity="" version="1"><conference-description><subject>Plan Q4</subject><msci:conference-id>{id}</msci:conference-id><msci:join-url>https://meet.contoso.com/alice/{id}</msci:join-url></conference-description></conference-info></getConference></response>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &get,
            200,
            Some("application/cccp+xml"),
            &details,
            &[],
        ),
        NOW,
    );
    let url = format!("https://meet.contoso.com/alice/{id}");
    assert!(events(&out).iter().any(|ev| matches!(ev,
        Event::MeetingDetails(m) if m.join_url.as_deref() == Some(url.as_str()))));

    let out = e.command(Command::ListMeetings, NOW);
    let list = find(&out, "SERVICE");
    assert!(String::from_utf8_lossy(&list.body).contains("<getConferences/>"));
    let listed = format!(
        r#"<response xmlns="urn:ietf:params:xml:ns:cccp" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" requestId="3" code="success"><getConferences><conferences><conference-info xmlns="urn:ietf:params:xml:ns:conference-info" entity="sip:alice@contoso.com;gruu;opaque=app:conf:focus:id:{id}"><conference-description><subject>Plan Q4</subject><msci:conference-id>{id}</msci:conference-id></conference-description></conference-info></conferences></getConferences></response>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &list,
            200,
            Some("application/cccp+xml"),
            &listed,
            &[],
        ),
        NOW,
    );
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Meetings(l) if l.len() == 1)));

    let out = e.command(
        Command::CancelMeeting {
            conference_id: id.clone(),
        },
        NOW,
    );
    let del = find(&out, "SERVICE");
    assert!(String::from_utf8_lossy(&del.body).contains("<deleteConference>"));
    // Example §4.3 (p. 46): 404 with the C3P failure answer; the reason is shown.
    let refused = r#"<response xmlns="urn:ietf:params:xml:ns:cccp" requestId="4" code="failure"><deleteConference reason="conferenceDoesNotExist"/></response>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &del,
            404,
            Some("application/cccp+xml"),
            refused,
            &[],
        ),
        NOW,
    );
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Notice(n) if n.contains("conferenceDoesNotExist"))));
    let out = e.command(
        Command::CancelMeeting {
            conference_id: id.clone(),
        },
        NOW,
    );
    let del = find(&out, "SERVICE");
    let ok = r#"<response xmlns="urn:ietf:params:xml:ns:cccp" requestId="5" code="success"><deleteConference/></response>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &del,
            200,
            Some("application/cccp+xml"),
            ok,
            &[],
        ),
        NOW,
    );
    assert!(events(&out).contains(&Event::MeetingCancelled { conference_id: id }));
    // A malformed conference ID never leaves the client.
    let out = e.command(
        Command::ShowMeeting {
            conference_id: "x\"/><evil".into(),
        },
        NOW,
    );
    assert!(sends(&out).is_empty());
}

/// PV-09/PV-14 ([MS-CONFPRO] §3.1.4.3, p. 33; §2.2.2.2, p. 20): a change first reads the
/// meeting, then sends modifyConference with the current version; without a version nothing is
/// changed.
#[test]
fn change_meeting_uses_current_version() {
    use commscraft_conf::schedule::Admission;
    let (mut e, mut server, out) = signed_in();
    provisioned(&mut e, &mut server, &out);
    let change = |e: &mut Engine<Counter>| {
        e.command(
            Command::ChangeMeeting {
                conference_id: "SDFBSD12".into(),
                subject: "New subject".into(),
                minutes: 30,
                admission: Admission::Anonymous,
            },
            NOW,
        )
    };
    let out = change(&mut e);
    let get = find(&out, "SERVICE");
    assert!(String::from_utf8_lossy(&get.body).contains("<getConference>"));
    let details = r#"<response xmlns="urn:ietf:params:xml:ns:cccp" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" requestId="1" code="success"><getConference><conference-info xmlns="urn:ietf:params:xml:ns:conference-info" entity="" version="3"><conference-description><msci:conference-id>SDFBSD12</msci:conference-id></conference-description></conference-info></getConference></response>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &get,
            200,
            Some("application/cccp+xml"),
            details,
            &[],
        ),
        NOW,
    );
    let modify = find(&out, "SERVICE");
    let b = String::from_utf8_lossy(&modify.body).to_string();
    assert!(
        b.contains("<modifyConference>") && b.contains("version=\"3\""),
        "{b}"
    );
    assert!(b.contains("New subject") && b.contains("anonymous"));
    // Without a version in the answer the meeting is not changed.
    let out = change(&mut e);
    let get = find(&out, "SERVICE");
    let no_version = r#"<response xmlns="urn:ietf:params:xml:ns:cccp" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" requestId="3" code="success"><getConference><conference-info xmlns="urn:ietf:params:xml:ns:conference-info" entity=""><conference-description><msci:conference-id>SDFBSD12</msci:conference-id></conference-description></conference-info></getConference></response>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &get,
            200,
            Some("application/cccp+xml"),
            no_version,
            &[],
        ),
        NOW,
    );
    assert!(sends(&out).is_empty());
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Notice(n) if n.contains("not changed"))));
}

/// CI-13: a Ms-Focus-Uri of another conference than the calling MCU's is not followed.
#[test]
fn dialed_out_foreign_focus_is_not_joined() {
    let (mut e, mut server, _) = signed_in();
    let mut invite = Message::request("INVITE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    invite.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKmcu2");
    invite.push_header("From", &format!("<{CHAT}>;tag=d52f12304e"));
    invite.push_header("To", "<sip:alice@contoso.com>;epid=8248ca9ebb");
    invite.push_header("Call-ID", "mcu2");
    invite.push_header("CSeq", "5 INVITE");
    invite.push_header("Contact", "<sip:pool.contoso.com:5062;transport=tls>");
    invite.push_header(
        "Ms-Focus-Uri",
        "<sip:mallory@contoso.com;gruu;opaque=app:conf:focus:id:0123456789ABCDEF>",
    );
    invite.push_header("Content-Type", "application/sdp");
    invite.body = b"v=0\r\no=- 0 0 IN IP4 0.0.0.0\r\ns=session\r\nc=IN IP4 0.0.0.0\r\nt=0 0\r\nm=message 5060 sip null\r\na=accept-types:*\r\n".to_vec();
    let out = e.on_message(&server.sign(invite), NOW);
    assert!(sends(&out).iter().any(|m| m.status_code() == Some(200)));
    assert!(!sends(&out).iter().any(|m| m.method() == Some("INVITE")));
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
    let out = e.on_message(&server.sign(invite.clone()), NOW);
    let ok = sends(&out)
        .into_iter()
        .find(|m| m.status_code() == Some(200))
        .unwrap();
    // CI-13: the focus of the same conference is joined as attendee for the roster.
    let focus_join = find(&out, "INVITE");
    assert_eq!(Engine::<Counter>::request_uri(&focus_join), Some(FOCUS));
    assert!(String::from_utf8_lossy(&focus_join.body).contains("<ci:entry>attendee</ci:entry>"));
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
    // The focus refuses: the chat goes on (no ConversationFailed).
    let out = e.on_message(&answer(&mut server, &focus_join, 403, None, "", &[]), NOW);
    assert!(!events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::ConversationFailed { .. })));
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Notice(n) if n.contains("the chat goes on"))));
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

fn media_local(port: u16, key_byte: u8) -> commscraft_sdp::Local {
    commscraft_sdp::Local {
        address: "192.0.2.1".parse().unwrap(),
        rtp_port: port,
        rtcp_port: port + 1,
        key: Some(commscraft_sdp::Key::new(1, [key_byte; 30])),
        ice: None,
        direction: commscraft_sdp::Direction::SendRecv,
        remote_candidates: Vec::new(),
        ssrtp: false,
    }
}

fn peer_sdp(key_byte: u8, proto: &str) -> String {
    let k = commscraft_sdp::Key::new(1, [key_byte; 30]);
    format!("v=0\r\no=- 0 0 IN IP4 198.51.100.7\r\ns=session\r\nc=IN IP4 198.51.100.7\r\nt=0 0\r\nm=audio 7000 {proto} 9 0 101\r\na=crypto:{}\r\na=rtcp:7001\r\na=rtpmap:9 G722/8000\r\na=rtpmap:0 PCMU/8000\r\na=rtpmap:101 telephone-event/8000\r\na=fmtp:101 0-16\r\n", k.attribute())
}

/// OCS-UC12 / MS-SIPRE §3.15: outgoing call with multipart offer, ringing, answer, final offer
/// (re-INVITE keeping the key), hang-up.
#[test]
fn outgoing_call_flow() {
    let (mut e, mut server, _) = signed_in();
    let out = e.command(
        Command::Call {
            to: "sip:bob@contoso.com".into(),
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let invite = find(&out, "INVITE");
    assert!(server.verify_client(&invite));
    assert!(invite
        .single("Content-Type")
        .unwrap()
        .unwrap()
        .starts_with("multipart/alternative"));
    let offer = call::audio_sdp(&invite).expect("ICE-19 SDP as the last part");
    assert!(String::from_utf8(offer)
        .unwrap()
        .contains("m=audio 40000 RTP/SAVP 9 0 8 101"));

    let out = e.on_message(&answer(&mut server, &invite, 180, None, "", &[]), NOW);
    assert_eq!(events(&out), vec![Event::CallRinging { call: 1 }]);
    let out = e.on_message(
        &answer(
            &mut server,
            &invite,
            200,
            Some("application/sdp"),
            &peer_sdp(9, "RTP/SAVP"),
            &[],
        ),
        NOW,
    );
    assert_eq!(
        find(&out, "ACK").call_id().unwrap(),
        invite.call_id().unwrap()
    );
    let Some(Event::CallAnswered { call: 1, media, .. }) = events(&out).pop() else {
        panic!()
    };
    assert_eq!(media.payload_type, 9);
    assert_eq!(media.remote_rtp, ("198.51.100.7".parse().unwrap(), 7000));

    // Final offer: a re-INVITE in the dialog; a different key in the update is replaced (SD-13).
    let mut fin = media_local(40000, 99);
    fin.remote_candidates = vec![(1, "198.51.100.7:7000".parse().unwrap())];
    let out = e.command(
        Command::UpdateCall {
            call: 1,
            local: Box::new(fin),
        },
        NOW,
    );
    let reinvite = find(&out, "INVITE");
    assert_eq!(reinvite.call_id().unwrap(), invite.call_id().unwrap());
    let body = String::from_utf8(reinvite.body.clone()).unwrap();
    assert!(body.contains("a=remote-candidates:1 198.51.100.7 7000"));
    assert!(
        body.contains(&commscraft_sdp::Key::new(1, [7; 30]).attribute()),
        "original key kept"
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &reinvite,
            200,
            Some("application/sdp"),
            &peer_sdp(9, "RTP/SAVP"),
            &[],
        ),
        NOW,
    );
    assert!(matches!(
        events(&out).pop(),
        Some(Event::CallUpdated { call: 1, .. })
    ));

    // Hold (MS-SDPEXT §3.1.5.23.1): inactive, sip.rendering="no", same key; then resume.
    let out = e.command(
        Command::Hold {
            call: 1,
            hold: true,
        },
        NOW,
    );
    let hold = find(&out, "INVITE");
    let body = String::from_utf8(hold.body.clone()).unwrap();
    assert!(body.contains("a=inactive"), "{body}");
    assert!(body.contains(&commscraft_sdp::Key::new(1, [7; 30]).attribute()));
    assert!(hold
        .single("Contact")
        .unwrap()
        .unwrap()
        .ends_with(";+sip.rendering=\"no\""));
    e.on_message(
        &answer(
            &mut server,
            &hold,
            200,
            Some("application/sdp"),
            &peer_sdp(9, "RTP/SAVP"),
            &[],
        ),
        NOW,
    );
    let out = e.command(
        Command::Hold {
            call: 1,
            hold: false,
        },
        NOW,
    );
    let resume = find(&out, "INVITE");
    let body = String::from_utf8(resume.body.clone()).unwrap();
    assert!(
        body.contains("a=sendrecv") && !body.contains("a=inactive"),
        "{body}"
    );
    assert!(!resume
        .single("Contact")
        .unwrap()
        .unwrap()
        .contains("sip.rendering"));
    e.on_message(
        &answer(
            &mut server,
            &resume,
            200,
            Some("application/sdp"),
            &peer_sdp(9, "RTP/SAVP"),
            &[],
        ),
        NOW,
    );

    let out = e.command(Command::Hangup { call: 1 }, NOW);
    assert!(sends(&out).iter().any(|m| m.method() == Some("BYE")));
    assert_eq!(events(&out), vec![Event::CallEnded { call: 1, status: 0 }]);
}

/// Security by design: a plaintext answer to a required-encryption offer ends the call with BYE.
#[test]
fn plaintext_answer_is_refused() {
    let (mut e, mut server, _) = signed_in();
    let out = e.command(
        Command::Call {
            to: "sip:bob@contoso.com".into(),
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let invite = find(&out, "INVITE");
    let plain = "v=0\r\nc=IN IP4 198.51.100.7\r\nm=audio 7000 RTP/AVP 0\r\n";
    let out = e.on_message(
        &answer(
            &mut server,
            &invite,
            200,
            Some("application/sdp"),
            plain,
            &[],
        ),
        NOW,
    );
    assert!(sends(&out).iter().any(|m| m.method() == Some("BYE")));
    assert!(events(&out).contains(&Event::CallEnded {
        call: 1,
        status: 488
    }));
}

/// Outgoing call cancelled while ringing (RFC 3261 §9.1): CANCEL with the INVITE's Via, then
/// the 487 ends the call.
#[test]
fn cancel_while_ringing() {
    let (mut e, mut server, _) = signed_in();
    let out = e.command(
        Command::Call {
            to: "sip:bob@contoso.com".into(),
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let invite = find(&out, "INVITE");
    let out = e.command(Command::Hangup { call: 1 }, NOW);
    let cancel = find(&out, "CANCEL");
    assert_eq!(cancel.single("Via").unwrap(), invite.single("Via").unwrap());
    assert_eq!(
        cancel.cseq().unwrap().unwrap().number,
        invite.cseq().unwrap().unwrap().number
    );
    let out = e.on_message(&answer(&mut server, &invite, 487, None, "", &[]), NOW);
    assert!(find(&out, "ACK").method() == Some("ACK"));
    assert_eq!(
        events(&out),
        vec![Event::CallEnded {
            call: 1,
            status: 487
        }]
    );
}

fn incoming_invite(server: &mut Server, body: &str, multipart: bool) -> Message {
    let mut invite = Message::request("INVITE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    invite.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKcall");
    invite.push_header("From", "<sip:bob@contoso.com>;tag=bobcall");
    invite.push_header("To", "<sip:alice@contoso.com>");
    invite.push_header("Call-ID", "call1");
    invite.push_header("CSeq", "1 INVITE");
    invite.push_header("Contact", "<sip:bob@contoso.com;opaque=user:epid:y;gruu>");
    if multipart {
        // MS-SIPRE §4.12.1 structure: ICEv6 part with ms-proxy-2007fallback, ICEv19 part last.
        invite.push_header("Content-Type", "multipart/alternative;boundary=\"b\"");
        invite.body = format!(
            "--b\r\nContent-Type: application/sdp\r\nContent-Disposition: session; handling=optional; ms-proxy-2007fallback\r\n\r\nv=0\r\nc=IN IP4 198.51.100.66\r\nm=audio 1 RTP/AVP 0\r\n\r\n--b\r\nContent-Type: application/sdp\r\nContent-Disposition: session; handling=optional\r\n\r\n{body}\r\n--b--\r\n"
        )
        .into_bytes();
    } else {
        invite.push_header("Content-Type", "application/sdp");
        invite.body = body.as_bytes().to_vec();
    }
    server.sign(invite)
}

/// Incoming call: 180, the user accepts, 200 with the answer to the ICE-19 part; BYE ends it.
#[test]
fn incoming_call_flow() {
    let (mut e, mut server, _) = signed_in();
    let out = e.on_message(
        &incoming_invite(&mut server, &peer_sdp(9, "RTP/AVP"), true),
        NOW,
    );
    let ringing = sends(&out).pop().unwrap();
    assert_eq!(ringing.status_code(), Some(180));
    // The callee's bandwidth check needs Call-ID and offer while it rings ([MS-ICE2BWM] §3.1.4.2).
    let (call_id, offer) = e.incoming_offer(1).unwrap();
    assert_eq!(call_id, "call1");
    assert!(String::from_utf8_lossy(offer).contains("m=audio"));
    assert!(ringing.single("To").unwrap().unwrap().contains(";tag="));
    assert_eq!(
        events(&out),
        vec![Event::IncomingCall {
            call: 1,
            from: "sip:bob@contoso.com".into()
        }]
    );
    let out = e.command(
        Command::AcceptCall {
            call: 1,
            local: Box::new(media_local(42000, 3)),
            policy: commscraft_sdp::Encryption::Optional {
                allow_plaintext: false,
            },
        },
        NOW,
    );
    let ok = sends(&out).pop().unwrap();
    assert_eq!(ok.status_code(), Some(200));
    let body = String::from_utf8(ok.body.clone()).unwrap();
    assert!(
        body.contains("RTP/SAVP"),
        "optional offer answered encrypted (SD-11)"
    );
    assert!(!body.contains("198.51.100.66"));
    let Some(Event::CallAnswered { call: 1, media, .. }) = events(&out).pop() else {
        panic!()
    };
    assert_eq!(
        media.remote_rtp,
        ("198.51.100.7".parse().unwrap(), 7000),
        "the ICE-19 part was used"
    );
    let tag = ok.to().unwrap().unwrap().param("tag").unwrap().to_string();
    let mut bye = Message::request("BYE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    bye.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKbye");
    bye.push_header("From", "<sip:bob@contoso.com>;tag=bobcall");
    bye.push_header("To", &format!("<sip:alice@contoso.com>;tag={tag}"));
    bye.push_header("Call-ID", "call1");
    bye.push_header("CSeq", "2 BYE");
    let out = e.on_message(&server.sign(bye), NOW);
    assert_eq!(sends(&out)[0].status_code(), Some(200));
    assert_eq!(events(&out), vec![Event::CallEnded { call: 1, status: 0 }]);
}

#[test]
fn incoming_call_cancel_and_timeout() {
    let (mut e, mut server, _) = signed_in();
    e.on_message(
        &incoming_invite(&mut server, &peer_sdp(9, "RTP/SAVP"), false),
        NOW,
    );
    let mut cancel = Message::request("CANCEL", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    cancel.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKcall");
    cancel.push_header("From", "<sip:bob@contoso.com>;tag=bobcall");
    cancel.push_header("To", "<sip:alice@contoso.com>");
    cancel.push_header("Call-ID", "call1");
    cancel.push_header("CSeq", "1 CANCEL");
    let out = e.on_message(&server.sign(cancel), NOW);
    let codes: Vec<_> = sends(&out).iter().map(|m| m.status_code()).collect();
    assert_eq!(codes, vec![Some(200), Some(487)]);
    assert_eq!(
        events(&out),
        vec![Event::CallEnded {
            call: 1,
            status: 487
        }]
    );
    // An unanswered call ends after 60 s with 480.
    e.on_message(
        &incoming_invite(&mut server, &peer_sdp(9, "RTP/SAVP"), false),
        NOW,
    );
    let out = e.on_tick(NOW + call::RING_TIMEOUT);
    assert!(sends(&out).iter().any(|m| m.status_code() == Some(480)));
    assert!(events(&out).contains(&Event::CallEnded {
        call: 2,
        status: 480
    }));
}

fn peer_sdp_ice() -> String {
    peer_sdp(9, "RTP/SAVP")
        + "a=ice-ufrag:PEER\r\na=ice-pwd:peerpassword0123456789ab\r\na=candidate:1 1 UDP 2130706431 198.51.100.7 7000 typ host\r\n"
}

/// MS-SDPEXT §3.1.5.12/§3.1.5.13 and RFC 3262 (SD-50, SD-51): a reliable 183 with an SDP
/// answer starts early media and is acknowledged with PRACK (RAck, no SDP); a retransmission is
/// not acknowledged again; the 200 of the same fork continues the early media; CSeq keeps rising.
#[test]
fn early_media_with_prack() {
    let (mut e, mut server, _) = signed_in();
    let out = e.command(
        Command::Call {
            to: "sip:bob@contoso.com".into(),
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let invite = find(&out, "INVITE");
    let supported: Vec<&str> = invite.header_values("Supported").collect();
    assert!(supported.contains(&"100rel") && supported.contains(&"ms-early-media"));
    assert!(
        invite
            .header_values("Require")
            .all(|v| !v.contains("100rel")),
        "SD-51"
    );

    let reliable = [("Require", "100rel"), ("RSeq", "1")];
    let r183 = answer(
        &mut server,
        &invite,
        183,
        Some("application/sdp"),
        &peer_sdp_ice(),
        &reliable,
    );
    let out = e.on_message(&r183, NOW);
    let prack = find(&out, "PRACK");
    assert_eq!(prack.single("RAck").unwrap(), Some("1 1 INVITE"));
    assert!(prack.body.is_empty(), "no SDP in PRACK");
    assert_eq!(prack.cseq().unwrap().unwrap().number, 2);
    assert!(
        prack.single("To").unwrap().unwrap().contains("tag=srv1"),
        "early dialog"
    );
    let ev = events(&out);
    assert!(ev.contains(&Event::CallRinging { call: 1 }));
    assert!(ev
        .iter()
        .any(|e| matches!(e, Event::CallEarlyMedia { call: 1, .. })));

    // Retransmission (same RSeq): neither PRACK nor a second early media event.
    let out = e.on_message(&r183, NOW);
    assert!(sends(&out).iter().all(|m| m.method() != Some("PRACK")));
    assert!(events(&out).is_empty());
    // The next reliable 18x (RSeq 2) is acknowledged; its SDP is ignored (one answer only).
    let r180 = answer(
        &mut server,
        &invite,
        180,
        Some("application/sdp"),
        &peer_sdp_ice(),
        &[("Require", "100rel"), ("RSeq", "2")],
    );
    let out = e.on_message(&r180, NOW);
    assert_eq!(
        find(&out, "PRACK").single("RAck").unwrap(),
        Some("2 1 INVITE")
    );
    assert!(events(&out).is_empty());

    let out = e.on_message(
        &answer(
            &mut server,
            &invite,
            200,
            Some("application/sdp"),
            &peer_sdp_ice(),
            &[],
        ),
        NOW,
    );
    assert!(events(&out).iter().any(|e| matches!(
        e,
        Event::CallAnswered {
            call: 1,
            continues_early: true,
            ..
        }
    )));
    let out = e.command(Command::Hangup { call: 1 }, NOW);
    assert!(
        find(&out, "BYE").cseq().unwrap().unwrap().number > 3,
        "CSeq above the PRACKs"
    );
}

/// SD-50: no early media without ICE or for a forked call; 199 ends the early media of its fork.
#[test]
fn early_media_limits() {
    let (mut e, mut server, _) = signed_in();
    let call = |e: &mut Engine<_>| {
        let out = e.command(
            Command::Call {
                to: "sip:bob@contoso.com".into(),
                local: Box::new(media_local(40000, 7)),
                policy: commscraft_sdp::Encryption::Required,
            },
            NOW,
        );
        find(&out, "INVITE")
    };
    let invite = call(&mut e);
    // Unreliable 183 without ICE: no early media.
    let out = e.on_message(
        &answer(
            &mut server,
            &invite,
            183,
            Some("application/sdp"),
            &peer_sdp(9, "RTP/SAVP"),
            &[],
        ),
        NOW,
    );
    assert!(!events(&out)
        .iter()
        .any(|e| matches!(e, Event::CallEarlyMedia { .. })));
    // Forked: no early media.
    let invite2 = call(&mut e);
    let out = e.on_message(
        &answer(
            &mut server,
            &invite2,
            183,
            Some("application/sdp"),
            &peer_sdp_ice(),
            &[("Ms-Forking", "Active")],
        ),
        NOW,
    );
    assert!(!events(&out)
        .iter()
        .any(|e| matches!(e, Event::CallEarlyMedia { .. })));
    // Unreliable 183 with ICE (gateway, PS-06): early media; 199 of that fork ends it.
    let invite3 = call(&mut e);
    let out = e.on_message(
        &answer(
            &mut server,
            &invite3,
            183,
            Some("application/sdp"),
            &peer_sdp_ice(),
            &[],
        ),
        NOW,
    );
    assert!(
        sends(&out).iter().all(|m| m.method() != Some("PRACK")),
        "unreliable: no PRACK"
    );
    let id = match events(&out).iter().find_map(|e| match e {
        Event::CallEarlyMedia { call, .. } => Some(*call),
        _ => None,
    }) {
        Some(id) => id,
        None => panic!("early media expected"),
    };
    let out = e.on_message(&answer(&mut server, &invite3, 199, None, "", &[]), NOW);
    assert_eq!(events(&out), vec![Event::CallEarlyMediaEnded { call: id }]);
}

/// SD-51 / RFC 3262 §4: an INVITE that requires 100rel is refused with 420 and Unsupported.
#[test]
fn invite_requiring_100rel_refused() {
    let (mut e, mut server, _) = signed_in();
    let mut invite = Message::request("INVITE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    invite.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKrel");
    invite.push_header("From", "<sip:bob@contoso.com>;tag=bobrel");
    invite.push_header("To", "<sip:alice@contoso.com>");
    invite.push_header("Call-ID", "callrel");
    invite.push_header("CSeq", "1 INVITE");
    invite.push_header("Contact", "<sip:bob@contoso.com;opaque=user:epid:y;gruu>");
    invite.push_header("Require", "100rel");
    invite.push_header("Content-Type", "application/sdp");
    invite.body = peer_sdp(9, "RTP/AVP").into_bytes();
    let out = e.on_message(&server.sign(invite), NOW);
    let r = sends(&out).pop().unwrap();
    assert_eq!(r.status_code(), Some(420));
    assert_eq!(r.single("Unsupported").unwrap(), Some("100rel"));
    assert!(events(&out).is_empty(), "no ringing");
}

const AVMCU: &str =
    "sip:alice@contoso.com;gruu;opaque=app:conf:audio-video:id:FC275ECD12493E4E9C27C894FE91E104";

/// MS-CONFBAS §3.4 (lobby, CB-23) and MS-CONFAV (CA-01, CA-02, CA-20): join a meeting by its
/// focus URI as lobby-capable attendee, wait in the lobby, get admitted, join the audio with an
/// INVITE to the A/V MCU, mute ourselves with modifyEndpointMedia, and leave (the audio ends).
#[test]
fn meeting_lobby_audio_and_mute() {
    let (mut e, mut server, _) = signed_in();
    let out = e.command(
        Command::JoinConference {
            focus: FOCUS.into(),
        },
        NOW,
    );
    let focus_invite = find(&out, "INVITE");
    let b = String::from_utf8_lossy(&focus_invite.body).to_string();
    assert!(b.contains("<ci:entry>attendee</ci:entry>") && b.contains("lobby-capable"));
    let granted = format!(
        r#"<response xmlns="urn:ietf:params:xml:ns:cccp" requestId="1" C3PVersion="1" code="success"><addUser><conferenceKeys confEntity="{FOCUS}"/></addUser></response>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &focus_invite,
            200,
            Some("application/cccp+xml"),
            &granted,
            &[],
        ),
        NOW,
    );
    let roster_sub = find(&out, "SUBSCRIBE");
    let lobby_doc = format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" entity="{FOCUS}" state="full" version="1"><users><user entity="sip:alice@contoso.com" state="full"><endpoint entity="{{F}}" msci:session-type="focus"><status>on-hold</status></endpoint></user></users></conference-info>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &roster_sub,
            200,
            Some("application/conference-info+xml"),
            &lobby_doc,
            &[("Expires", "3600")],
        ),
        NOW,
    );
    let conversation = events(&out)
        .iter()
        .find_map(|ev| match ev {
            Event::ConferenceLobby {
                conversation,
                waiting: true,
            } => Some(*conversation),
            _ => None,
        })
        .expect("lobby reported");
    let out = e.command(
        Command::JoinConferenceAudio {
            conversation,
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    assert_eq!(
        events(&out),
        vec![Event::Notice("still in the lobby".into())]
    );

    // Admitted: full document with the MCU URIs and our A/V endpoint.
    let admitted = format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" entity="{FOCUS}" state="full" version="2"><conference-description><conf-uris><entry><uri>{AVMCU}</uri><purpose>audio-video</purpose></entry></conf-uris></conference-description><users><user entity="sip:alice@contoso.com" state="full"><endpoint entity="{{F}}" msci:session-type="focus"><status>connected</status></endpoint></user></users></conference-info>"#
    );
    let notify = peer_request(
        &mut server,
        &roster_sub,
        "BENOTIFY",
        2,
        Some("application/conference-info+xml"),
        &admitted,
        &[("Event", "conference")],
    );
    let out = e.on_message(&notify, NOW);
    assert!(events(&out).contains(&Event::ConferenceLobby {
        conversation,
        waiting: false
    }));

    let out = e.command(
        Command::JoinConferenceAudio {
            conversation,
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let av_invite = find(&out, "INVITE");
    assert_eq!(
        Engine::<Counter>::request_uri(&av_invite),
        Some(AVMCU),
        "CA-02: INVITE to the A/V MCU"
    );
    assert!(
        String::from_utf8_lossy(&av_invite.body).contains("a=cryptoscale:2 client"),
        "SD-12: SRTP and client SSRTP offered to the MCU"
    );
    let call = events(&out)
        .iter()
        .find_map(|ev| match ev {
            Event::ConferenceAudio { call, .. } => Some(*call),
            _ => None,
        })
        .expect("conference audio call");

    // Our A/V endpoint appears; mute.
    let with_av = format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" entity="{FOCUS}" state="partial" version="3"><users><user entity="sip:alice@contoso.com" state="partial"><endpoint entity="{{AV1}}" msci:session-type="audio-video"><status>connected</status><media id="1"><type>audio</type><label>main-audio</label><status>sendrecv</status><msci:media-ingress-filter>unblock</msci:media-ingress-filter></media></endpoint></user></users></conference-info>"#
    );
    e.on_message(
        &peer_request(
            &mut server,
            &roster_sub,
            "BENOTIFY",
            3,
            Some("application/conference-info+xml"),
            &with_av,
            &[("Event", "conference")],
        ),
        NOW,
    );
    let out = e.command(
        Command::ConferenceMute {
            conversation,
            mute: true,
        },
        NOW,
    );
    let info = find(&out, "INFO");
    assert_eq!(
        info.call_id().unwrap(),
        focus_invite.call_id().unwrap(),
        "in the focus dialog"
    );
    let b = String::from_utf8(info.body.clone()).unwrap();
    assert!(
        b.contains("<modifyEndpointMedia")
            && b.contains("endpointEntity=\"{AV1}\"")
            && b.contains(">block<"),
        "{b}"
    );

    // Leaving ends the conference audio too (CANCEL, as it is not answered yet).
    let out = e.command(Command::EndConversation { conversation }, NOW);
    assert!(sends(&out)
        .iter()
        .any(|m| m.method() == Some("CANCEL")
            && m.call_id().unwrap() == av_invite.call_id().unwrap()));
    assert!(call > 0);
}

/// Joins FOCUS and delivers `users` as the full roster (with the A/V MCU); returns the engine,
/// the server, the focus INVITE and the conversation.
fn joined_meeting(users: &str) -> (Engine<Counter>, Server, Message, ConversationId) {
    let (mut e, mut server, _) = signed_in();
    let out = e.command(
        Command::JoinConference {
            focus: FOCUS.into(),
        },
        NOW,
    );
    let focus_invite = find(&out, "INVITE");
    let granted = format!(
        r#"<response xmlns="urn:ietf:params:xml:ns:cccp" requestId="1" C3PVersion="1" code="success"><addUser><conferenceKeys confEntity="{FOCUS}"/></addUser></response>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &focus_invite,
            200,
            Some("application/cccp+xml"),
            &granted,
            &[],
        ),
        NOW,
    );
    let roster_sub = find(&out, "SUBSCRIBE");
    let doc = format!(
        r#"<conference-info xmlns="urn:ietf:params:xml:ns:conference-info" xmlns:msci="http://schemas.microsoft.com/rtc/2005/08/confinfoextensions" entity="{FOCUS}" state="full" version="1"><conference-description><conf-uris><entry><uri>{AVMCU}</uri><purpose>audio-video</purpose></entry></conf-uris></conference-description><users>{users}</users></conference-info>"#
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &roster_sub,
            200,
            Some("application/conference-info+xml"),
            &doc,
            &[("Expires", "3600")],
        ),
        NOW,
    );
    let conversation = events(&out)
        .iter()
        .find_map(|ev| match ev {
            Event::Participants { conversation, .. } => Some(*conversation),
            _ => None,
        })
        .unwrap();
    (e, server, focus_invite, conversation)
}

/// SD-13 / SD-12 ([MS-SDPEXT] §3.1.5.2.2, p. 22): the A/V MCU answers our offer with server
/// SSRTP; the final offer after ICE keeps the same crypto and cryptoscale lines and the same key
/// (a re-INVITE must not renegotiate any parameter of these lines).
#[test]
fn conference_audio_final_offer_keeps_ssrtp() {
    let (mut e, mut server, _, conversation) = joined_meeting(PRESENTER_ROSTER);
    let out = e.command(
        Command::JoinConferenceAudio {
            conversation,
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let av_invite = find(&out, "INVITE");
    let call = events(&out)
        .iter()
        .find_map(|ev| match ev {
            Event::ConferenceAudio { call, .. } => Some(*call),
            _ => None,
        })
        .expect("conference audio call");
    let server_key = commscraft_sdp::Key::new(2, [9; 30]);
    let mcu = format!(
        "v=0\r\no=- 0 0 IN IP4 198.51.100.9\r\ns=session\r\nc=IN IP4 198.51.100.9\r\nt=0 0\r\nm=audio 7000 RTP/SAVP 9 101\r\na=cryptoscale:{} server {}\r\na=rtcp:7001\r\na=rtpmap:9 G722/8000\r\na=rtpmap:101 telephone-event/8000\r\n",
        server_key.attribute().split_once(' ').unwrap().0,
        server_key.attribute().split_once(' ').unwrap().1
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &av_invite,
            200,
            Some("application/sdp"),
            &mcu,
            &[],
        ),
        NOW,
    );
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::CallAnswered { .. })));
    let mut fin = media_local(40000, 99);
    fin.remote_candidates = vec![(1, "198.51.100.9:7000".parse().unwrap())];
    let out = e.command(
        Command::UpdateCall {
            call,
            local: Box::new(fin),
        },
        NOW,
    );
    let reinvite = find(&out, "INVITE");
    assert_eq!(reinvite.call_id().unwrap(), av_invite.call_id().unwrap());
    let body = String::from_utf8(reinvite.body.clone()).unwrap();
    let ours = commscraft_sdp::Key::new(1, [7; 30]).attribute();
    let (_, rest) = ours.split_once(' ').unwrap();
    assert!(body.contains(&format!("a=crypto:{ours}")), "{body}");
    assert!(
        body.contains(&format!("a=cryptoscale:2 client {rest}")),
        "same SSRTP offer as at first: {body}"
    );
}

const PRESENTER_ROSTER: &str = r#"<user entity="sip:alice@contoso.com" state="full"><roles><entry>presenter</entry></roles><endpoint entity="{A}" msci:session-type="focus"><status>connected</status></endpoint></user><user entity="sip:bob@fabrikam.com" state="full"><roles><entry>attendee</entry></roles><endpoint entity="{B}" msci:session-type="focus"><status>on-hold</status></endpoint></user><user entity="sip:carol@fabrikam.com" state="full"><roles><entry>attendee</entry></roles><endpoint entity="{C}" msci:session-type="focus"><status>connected</status></endpoint><endpoint entity="{CAV}" msci:session-type="audio-video"><status>connected</status><media id="1"><type>audio</type><status>sendrecv</status></media></endpoint></user>"#;

/// CB-35 / CA-20 / CA-22: a presenter admits everyone waiting, mutes a participant and the
/// audience over INFO in the focus dialog; a refusal of the focus is reported.
#[test]
fn presenter_commands_in_the_focus_dialog() {
    let (mut e, mut server, focus_invite, conversation) = joined_meeting(PRESENTER_ROSTER);
    let control = |e: &mut Engine<Counter>, action| {
        let out = e.command(
            Command::ConferenceControl {
                conversation,
                action,
            },
            NOW,
        );
        let info = find(&out, "INFO");
        assert_eq!(info.call_id().unwrap(), focus_invite.call_id().unwrap());
        String::from_utf8(info.body.clone()).unwrap()
    };
    let b = control(
        &mut e,
        ConferenceAction::Lobby {
            users: Vec::new(),
            admit: true,
        },
    );
    assert!(
        b.contains("<userEntity>sip:bob@fabrikam.com</userEntity><access>granted</access>"),
        "{b}"
    );
    let b = control(
        &mut e,
        ConferenceAction::MuteUser {
            user: "sip:carol@fabrikam.com".into(),
            mute: true,
        },
    );
    assert!(
        b.contains("userEntity=\"sip:carol@fabrikam.com\" endpointEntity=\"{CAV}\"")
            && b.contains(">block<")
    );
    let b = control(&mut e, ConferenceAction::MuteAudience(true));
    assert!(b.contains("<modifyConference") && b.contains(AVMCU));
    let b = control(&mut e, ConferenceAction::Lock(true));
    assert!(b.contains("<locked>true</locked>"));
    let out = e.command(
        Command::ConferenceControl {
            conversation,
            action: ConferenceAction::MuteUser {
                user: "sip:bob@fabrikam.com".into(),
                mute: true,
            },
        },
        NOW,
    );
    assert_eq!(
        events(&out),
        vec![Event::Notice("that participant has no audio".into())]
    );
    // The focus refuses: shown.
    let fail = r#"<response xmlns="urn:ietf:params:xml:ns:cccp" requestId="9" C3PVersion="1" code="failure" reason="unauthorized"><modifyConferenceLock/></response>"#;
    let info = peer_request(
        &mut server,
        &focus_invite,
        "INFO",
        9,
        Some("application/cccp+xml"),
        fail,
        &[],
    );
    let out = e.on_message(&info, NOW);
    assert!(events(&out).iter().any(|ev| matches!(ev, Event::Notice(n) if n.contains("modifyConferenceLock refused (unauthorized)"))));
}

/// CB-33: an attendee gets a hint instead of a request.
#[test]
fn attendees_cannot_control() {
    let users = PRESENTER_ROSTER.replacen("<entry>presenter</entry>", "<entry>attendee</entry>", 1);
    let (mut e, _, _, conversation) = joined_meeting(&users);
    let out = e.command(
        Command::ConferenceControl {
            conversation,
            action: ConferenceAction::Lock(true),
        },
        NOW,
    );
    assert!(sends(&out).is_empty());
    assert_eq!(
        events(&out),
        vec![Event::Notice("only presenters can do that".into())]
    );
}

fn provision(e: &mut Engine<Counter>, server: &mut Server, out: &[Output], doc: &str) {
    let sub = sends(out)
        .into_iter()
        .find(|m| m.single("Event").unwrap() == Some("vnd-microsoft-provisioning-v2"))
        .unwrap();
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

/// PS-01 / RE-40: dial strings become SIP URIs – global numbers directly, local ones with the
/// location profile as phone-context (the user's profile wins), `dialstring` without one.
#[test]
fn phone_numbers() {
    let (mut e, mut server, out) = signed_in();
    assert_eq!(
        e.phone_uri("+49 (40) 123-456").as_deref(),
        Some("sip:+4940123456@contoso.com;user=phone")
    );
    assert_eq!(
        e.phone_uri("0401234").as_deref(),
        Some("sip:0401234;phone-context=dialstring@contoso.com;user=phone")
    );
    assert_eq!(
        e.phone_uri("*55#").as_deref(),
        Some("sip:*55%23;phone-context=dialstring@contoso.com;user=phone")
    );
    for bad in [
        "",
        "abc",
        "+",
        "+1234567890123456",
        "12;x",
        "sip:bob@contoso.com",
        "+49*1",
    ] {
        assert_eq!(e.phone_uri(bad), None, "{bad}");
    }
    let doc = r#"<provisionGroupList xmlns="http://schemas.microsoft.com/2006/09/sip/provisiongrouplist-notification"><provisionGroup name="ServerConfiguration"><ucLocationProfile>DefaultProfile</ucLocationProfile></provisionGroup><provisionGroup name="userSetting"><ucUserLocationProfile>Redmond.contoso.com</ucUserLocationProfile></provisionGroup></provisionGroupList>"#;
    provision(&mut e, &mut server, &out, doc);
    let uri = e.phone_uri("1234").unwrap();
    assert_eq!(
        uri,
        "sip:1234;phone-context=Redmond.contoso.com@contoso.com;user=phone"
    );
    let out = e.command(
        Command::Call {
            to: uri.clone(),
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let invite = find(&out, "INVITE");
    assert_eq!(Engine::<Counter>::request_uri(&invite), Some(uri.as_str()));
    let body = String::from_utf8_lossy(&invite.body).to_string();
    assert!(
        body.contains("Content-ID: <audio-ice19@commscraft>"),
        "PS-09"
    );
}

/// An unsafe profile name from provisioning is rejected, not used.
#[test]
fn unsafe_location_profile_rejected() {
    let doc = r#"<provisionGroupList xmlns="http://schemas.microsoft.com/2006/09/sip/provisiongrouplist-notification"><provisionGroup name="ServerConfiguration"><ucLocationProfile>a@b;c</ucLocationProfile></provisionGroup></provisionGroupList>"#;
    let p = commscraft_provisioning::parse(doc).unwrap();
    let c = commscraft_provisioning::ServerConfiguration::from_group(
        p.group("ServerConfiguration").unwrap(),
        "contoso.com",
    );
    assert_eq!(c.location_profile, None);
    assert!(c.rejected.contains(&"ucLocationProfile".to_string()));
}

/// PS-05 (605 for a loop call while ringing), PS-02/PS-04/PS-07 (asserted number, gateway,
/// anonymous), PS-09 (ms-accepted-content-id names the answered part).
#[test]
fn pstn_incoming_calls() {
    let (mut e, mut server, _) = signed_in();
    let gw_invite = |server: &mut Server, call_id: &str, from: &str, extra: &[(&str, &str)]| {
        let mut m = Message::request("INVITE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
        m.push_header(
            "Via",
            &format!("SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bK{call_id}"),
        );
        m.push_header("From", &format!("<{from}>;tag=gw{call_id}"));
        m.push_header("To", "<sip:alice@contoso.com>");
        m.push_header("Call-ID", call_id);
        m.push_header("CSeq", "1 INVITE");
        m.push_header(
            "Contact",
            "<sip:gw.contoso.com:5068;transport=tls>;isGateway",
        );
        for (k, v) in extra {
            m.push_header(k, v);
        }
        m.push_header("Content-Type", "multipart/alternative;boundary=\"b\"");
        m.body = format!("--b\r\nContent-Type: application/sdp\r\nContent-ID: <gw-part-1@gw.contoso.com>\r\n\r\n{}\r\n--b--\r\n", peer_sdp(9, "RTP/AVP")).into_bytes();
        server.sign(m)
    };
    let out = e.on_message(
        &gw_invite(
            &mut server,
            "pstn1",
            "sip:+4940555@contoso.com;user=phone",
            &[("P-Asserted-Identity", "<tel:+4940555>")],
        ),
        NOW,
    );
    assert_eq!(sends(&out).pop().unwrap().status_code(), Some(180));
    let who = e.caller(1).unwrap();
    assert_eq!(
        (who.number.as_deref(), who.gateway, who.anonymous),
        (Some("+4940555"), true, false)
    );
    assert_eq!(
        e.callback_uri(1).as_deref(),
        Some("sip:+4940555@contoso.com;user=phone")
    );

    // A loop call while call 1 rings: 605.
    let out = e.on_message(
        &gw_invite(
            &mut server,
            "pstn2",
            "sip:+4940666@contoso.com;user=phone",
            &[("Ms-Call-Source", "ms-rtc")],
        ),
        NOW,
    );
    assert_eq!(sends(&out).pop().unwrap().status_code(), Some(605));

    // Anonymous caller: shown so, no call back.
    e.on_message(
        &gw_invite(
            &mut server,
            "pstn3",
            "sip:anonymous@contoso.com;user=phone",
            &[],
        ),
        NOW,
    );
    let anon = e.caller(2).unwrap();
    assert!(anon.anonymous);
    assert_eq!(e.callback_uri(2), None);

    // Answering call 1 names the accepted part.
    let out = e.command(
        Command::AcceptCall {
            call: 1,
            local: Box::new(media_local(42000, 3)),
            policy: commscraft_sdp::Encryption::Optional {
                allow_plaintext: true,
            },
        },
        NOW,
    );
    let ok = sends(&out).pop().unwrap();
    assert_eq!(ok.status_code(), Some(200));
    assert_eq!(
        ok.single("ms-accepted-content-id").unwrap(),
        Some("<gw-part-1@gw.contoso.com>")
    );
}

/// An outgoing call to `to`, answered: returns our INVITE (the dialog's request).
fn active_call(e: &mut Engine<Counter>, server: &mut Server, to: &str) -> Message {
    let out = e.command(
        Command::Call {
            to: to.into(),
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let invite = find(&out, "INVITE");
    assert!(
        invite
            .header_values("Supported")
            .any(|v| v == "ms-safe-transfer"),
        "RE-64"
    );
    e.on_message(
        &answer(
            server,
            &invite,
            200,
            Some("application/sdp"),
            &peer_sdp(9, "RTP/SAVP"),
            &[],
        ),
        NOW,
    );
    invite
}

fn our_tag(m: &Message) -> String {
    m.from().unwrap().unwrap().param("tag").unwrap().to_string()
}

const CPS: &str = "sip:callparkservice.contoso.com@contoso.com;gruu;opaque=srvr:Microsoft.Rtc.Applications.Cps:4H28w6olDlm9KG3nl5tyBQAA";

/// Provisioning with the call park server (MS-SIPREGE §2.2.2.5.1, CK-01).
fn provisioned_with_cps(e: &mut Engine<Counter>, server: &mut Server, out: &[Output]) {
    let sub = sends(out)
        .into_iter()
        .find(|m| m.single("Event").unwrap() == Some("vnd-microsoft-provisioning-v2"))
        .unwrap();
    let doc = format!(
        r#"<provisionGroupList xmlns="http://schemas.microsoft.com/2006/09/sip/provisiongrouplist-notification"><provisionGroup name="ServerConfiguration"><callParkServerUri>{CPS}</callParkServerUri></provisionGroup></provisionGroupList>"#
    );
    e.on_message(
        &answer(
            server,
            &sub,
            200,
            Some("application/vnd-microsoft-roaming-provisioning-v2+xml"),
            &doc,
            &[("Event", "vnd-microsoft-provisioning-v2")],
        ),
        NOW,
    );
}

/// Function 37 (CK-01 … CK-04; [MS-SIPAPP] §3.1.5.1–3.1.5.5, pp. 15–17, example §4.1.1): park an
/// active call – INVITE to the CPS with the call's From/To tags and the peer's contact, the orbit
/// from the 200 OK, session timer refresh of the parker's dialog, unpark notification by INFO,
/// BYE of the CPS; refusal with 500 (no free orbit).
#[test]
fn park_a_call() {
    let (mut e, mut server, out) = signed_in();
    // Without a call park server nothing is sent.
    let out2 = e.command(Command::ParkCall { call: 1 }, NOW);
    assert!(sends(&out2).is_empty());
    provisioned_with_cps(&mut e, &mut server, &out);
    let invite = active_call(&mut e, &mut server, "sip:bob@contoso.com");
    let out = e.command(Command::ParkCall { call: 1 }, NOW);
    let park = find(&out, "INVITE");
    assert_eq!(Engine::<Counter>::request_uri(&park), Some(CPS));
    assert_eq!(
        park.single("Content-Type").unwrap(),
        Some("application/ms-call-park+xml")
    );
    let b = String::from_utf8(park.body.clone()).unwrap();
    assert!(b.contains(&format!(
        "<call-id>{}</call-id>",
        invite.call_id().unwrap().unwrap()
    )));
    assert!(
        b.contains(&format!("<from-tag>{}</from-tag>", our_tag(&invite))),
        "we called: our tag is the From tag"
    );
    assert!(b.contains("<to-tag>srv1</to-tag>"));
    assert!(b.contains("<target>sip:pool.contoso.com:5061;transport=tls</target>"));
    let response = r#"<?xml version="1.0"?><park-response version="1.0" request-id="1" xmlns="http://schemas.microsoft.com/rtc/2008/12/callpark"><orbit>1504</orbit><ms-parked-call>3e851601;from-tag=76ea4cdd28</ms-parked-call></park-response>"#;
    let out = e.on_message(
        &answer(
            &mut server,
            &park,
            200,
            Some("application/ms-call-park+xml"),
            response,
            &[("Session-Expires", "320;refresher=uac")],
        ),
        NOW,
    );
    assert_eq!(
        find(&out, "ACK").call_id().unwrap(),
        park.call_id().unwrap()
    );
    assert!(events(&out).contains(&Event::CallParked {
        call: 1,
        orbit: "1504".into()
    }));
    // RFC 4028: the CPS asked us to refresh (320 s): UPDATE in the parker's dialog.
    let update = find(&e.on_tick(NOW + 170), "UPDATE");
    assert_eq!(update.call_id().unwrap(), park.call_id().unwrap());
    // CK-04: unpark notification after a retrieval (example p. 30).
    let info = peer_request(
        &mut server,
        &park,
        "INFO",
        2,
        Some("application/ms-call-park+xml"),
        r#"<unpark-notification version="1.0" request-id="1" xmlns="http://schemas.microsoft.com/rtc/2008/12/callpark"><reason>retrieval</reason><target>sip:carol@contoso.com</target></unpark-notification>"#,
        &[],
    );
    let out = e.on_message(&info, NOW);
    assert_eq!(sends(&out)[0].status_code(), Some(200));
    assert!(events(&out).contains(&Event::ParkNotification {
        orbit: "1504".into(),
        reason: commscraft_park::UnparkReason::Retrieval,
        target: Some("sip:carol@contoso.com".into()),
    }));
    // The CPS ends the dialog; afterwards the dialog is unknown.
    let bye = peer_request(&mut server, &park, "BYE", 3, None, "", &[]);
    assert_eq!(sends(&e.on_message(&bye, NOW))[0].status_code(), Some(200));
    let late = peer_request(&mut server, &park, "INFO", 4, None, "", &[]);
    assert_eq!(sends(&e.on_message(&late, NOW))[0].status_code(), Some(481));

    // CK-03: no free orbit.
    active_call(&mut e, &mut server, "sip:carol@contoso.com");
    let out = e.command(Command::ParkCall { call: 2 }, NOW);
    let park = find(&out, "INVITE");
    let out = e.on_message(&answer(&mut server, &park, 500, None, "", &[]), NOW);
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Notice(n) if n.contains("no free orbit"))));
}

const QOS: &str = "sip:fe.contoso.com@contoso.com;gruu;opaque=srvr:QoS:jcH1fn2RSB6uyC59R-IH8QAA";

fn audio_line() -> commscraft_qoe::AudioLine {
    commscraft_qoe::AudioLine {
        encrypted: true,
        offerer: true,
        tcp: false,
        local: "192.0.2.1:40000".parse().unwrap(),
        remote: "198.51.100.7:7000".parse().unwrap(),
        inbound: Some(commscraft_qoe::Stream {
            ssrc: 7,
            packets: 100,
            jitter_avg_ms: Some(3),
            ..Default::default()
        }),
        outbound: None,
        codec: "G722".into(),
        sample_rate: 16000,
    }
}

/// Function 45 (QE-01 … QE-04; [MS-QoE] §1.3, §2.2.1, §3.2): after a call the report goes in a
/// SERVICE to qosUri with the call's dialog data and LocalPAI = our address; a 202 is quiet, an
/// error is reported; the rating goes the same way, a bad rating is not sent.
#[test]
fn quality_report_and_rating() {
    let (mut e, mut server, out) = signed_in();
    let sub = sends(&out)
        .into_iter()
        .find(|m| m.single("Event").unwrap() == Some("vnd-microsoft-provisioning-v2"))
        .unwrap();
    // Without qosUri nothing is sent.
    active_call(&mut e, &mut server, "sip:carol@contoso.com");
    let none = e.command(
        Command::ReportQuality {
            call: 1,
            audio: Box::new(audio_line()),
        },
        NOW,
    );
    assert!(sends(&none).is_empty());
    let doc = format!(
        r#"<provisionGroupList xmlns="http://schemas.microsoft.com/2006/09/sip/provisiongrouplist-notification"><provisionGroup name="ServerConfiguration"><qosUri>{QOS}</qosUri></provisionGroup></provisionGroupList>"#
    );
    e.on_message(
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
    let invite = active_call(&mut e, &mut server, "sip:bob@contoso.com");
    let out = e.command(
        Command::ReportQuality {
            call: 2,
            audio: Box::new(audio_line()),
        },
        NOW + 60,
    );
    let service = find(&out, "SERVICE");
    assert_eq!(Engine::<Counter>::request_uri(&service), Some(QOS));
    assert_eq!(
        service.single("Content-Type").unwrap(),
        Some("application/vq-rtcpxr+xml")
    );
    let b = String::from_utf8(service.body.clone()).unwrap();
    assert!(b.contains(&format!(
        "CallId=\"{}\"",
        invite.call_id().unwrap().unwrap()
    )));
    assert!(
        b.contains(&format!("FromTag=\"{}\"", our_tag(&invite))) && b.contains("ToTag=\"srv1\"")
    );
    assert!(
        b.contains("<Caller>true</Caller>")
            && b.contains("<LocalPAI>sip:alice@contoso.com</LocalPAI>")
    );
    assert!(b.contains("<ToURI>sip:bob@contoso.com</ToURI>"));
    let quiet = e.on_message(&answer(&mut server, &service, 202, None, "", &[]), NOW + 60);
    assert!(events(&quiet).is_empty());
    let out = e.command(
        Command::RateCall {
            call: 2,
            stars: 4,
            feedback: Some("fine".into()),
        },
        NOW + 70,
    );
    let rating = find(&out, "SERVICE");
    assert_eq!(
        rating.single("Content-Type").unwrap(),
        Some("application/ms-cqf+xml")
    );
    assert!(String::from_utf8_lossy(&rating.body).contains("<Rating>4</Rating>"));
    let refused = e.on_message(&answer(&mut server, &rating, 400, None, "", &[]), NOW + 70);
    assert!(events(&refused)
        .iter()
        .any(|ev| matches!(ev, Event::Notice(n) if n.contains("not accepted (400)"))));
    let bad = e.command(
        Command::RateCall {
            call: 2,
            stars: 9,
            feedback: None,
        },
        NOW,
    );
    assert!(sends(&bad).is_empty());
}

/// Function 39 (RE-67, RG-51; [MS-SIPRE] §2.2.23, §3.17.5.1, example §4.14; [MS-SIPREGE]
/// §2.2.2.5.7): the location policy makes 112 (and its mask 911) an emergency number; the
/// emergency call publishes a time-bound e911active preamble and sends the INVITE with Priority,
/// Supported: geolocation, Geolocation and the PIDF-LO as last part of multipart/mixed; without a
/// location the call still goes out; other numbers never take this path.
#[test]
fn emergency_call_with_location() {
    let (mut e, mut server, out) = signed_in();
    let sub = sends(&out)
        .into_iter()
        .find(|m| m.single("Event").unwrap() == Some("vnd-microsoft-provisioning-v2"))
        .unwrap();
    let doc = r#"<provisionGroupList xmlns="http://schemas.microsoft.com/2006/09/sip/provisiongrouplist-notification"><provisionGroup name="locationPolicy"><propertyEntryList><property name="EnhancedEmergencyServicesEnabled">true</property><property name="LocationPolicyTagID">user-tagid</property><property name="EmergencyNumbers">112-911</property><property name="NotificationUri">sip:security@contoso.com</property><property name="ConferenceMode">oneway</property></propertyEntryList></provisionGroup><provisionGroup name="ServerConfiguration"><locationProfile>Redmond</locationProfile></provisionGroup></provisionGroupList>"#;
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
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::LocationPolicy(p) if p.enabled)));
    assert_eq!(e.emergency_dial("911").as_deref(), Some("112"));
    assert_eq!(e.emergency_dial("0401234"), None);
    // No location yet: the call still goes out, without PIDF-LO.
    let out = e.command(
        Command::EmergencyCall {
            dialled: "112".into(),
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let invite = find(&out, "INVITE");
    assert_eq!(invite.single("Priority").unwrap(), Some("emergency"));
    assert!(invite.single("Geolocation").unwrap().is_none());
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Notice(n) if n.contains("WITHOUT location"))));
    e.command(Command::Hangup { call: 1 }, NOW);

    let loc = commscraft_lis::Location {
        country: "US".into(),
        a3: "Redmond".into(),
        rd: "163rd".into(),
        hno: "3910".into(),
        nam: "Contoso <Corp>".into(),
        manual: true,
        ..Default::default()
    };
    let out = e.command(Command::SetLocation(Box::new(loc)), NOW);
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Location(_))));
    let out = e.command(
        Command::EmergencyCall {
            dialled: "911".into(),
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let publish = sends(&out)
        .into_iter()
        .find(|m| m.method() == Some("SERVICE"))
        .unwrap();
    let p = String::from_utf8_lossy(&publish.body).to_string();
    assert!(
        p.contains("e911active") && p.contains("expireType=\"time\""),
        "{p}"
    );
    let invite = find(&out, "INVITE");
    let uri = Engine::<Counter>::request_uri(&invite).unwrap();
    assert!(uri.starts_with("sip:112;phone-context="), "{uri}");
    assert_eq!(invite.single("Priority").unwrap(), Some("emergency"));
    assert!(invite
        .header_values("Supported")
        .any(|v| v == "geolocation"));
    assert_eq!(
        invite.single("Geolocation").unwrap(),
        Some("<cid:alice@contoso.com>;inserted-by=\"sip:alice@contoso.com\"")
    );
    let ct = invite.single("Content-Type").unwrap().unwrap();
    assert!(ct.starts_with("multipart/mixed"), "{ct}");
    let body = String::from_utf8_lossy(&invite.body).to_string();
    let pidf_at = body.find("Content-Type: application/pidf+xml").unwrap();
    assert!(
        body.find("application/sdp").unwrap() < pidf_at,
        "the PIDF-LO is the last part"
    );
    assert!(body.contains("Content-ID: <alice@contoso.com>"));
    assert!(
        body.contains("<ca:NAM>Contoso &lt;Corp&gt;</ca:NAM>")
            && body.contains("<gp:method>Manual</gp:method>")
    );
    assert!(
        body.contains(">user-tagid</LocationPolicyTagID>")
            && body.contains("<ms:ConferenceMode>oneway</ms:ConferenceMode>")
    );
    // After the call: nothing was published by the user in this session – a notice, no
    // publication that could overwrite rules set elsewhere (O-RE-3).
    e.command(Command::Hangup { call: 2 }, NOW);
    e.on_message(&answer(&mut server, &invite, 487, None, "", &[]), NOW);
    let out = e.on_tick(NOW + 1);
    assert!(!sends(&out).iter().any(|m| m.method() == Some("SERVICE")));
    assert!(events(&out)
        .iter()
        .any(|ev| matches!(ev, Event::Notice(n) if n.contains("check your forwarding"))));
    // A number that is no emergency number never takes this path.
    let out = e.command(
        Command::EmergencyCall {
            dialled: "0401234".into(),
            local: Box::new(media_local(40000, 7)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    assert!(sends(&out).is_empty());
}

/// RFC 3515 / RFC 5589 §6, OB-25: unattended transfer – REFER with Refer-To and Referred-By;
/// after 202 our call ends.
#[test]
fn unattended_transfer() {
    let (mut e, mut server, _) = signed_in();
    let invite = active_call(&mut e, &mut server, "sip:bob@contoso.com");
    let out = e.command(
        Command::Transfer {
            call: 1,
            to: TransferTarget::Address("sip:carol@contoso.com".into()),
        },
        NOW,
    );
    let refer = find(&out, "REFER");
    assert_eq!(
        refer.call_id().unwrap(),
        invite.call_id().unwrap(),
        "in the call dialog"
    );
    assert_eq!(
        refer.single("Refer-To").unwrap(),
        Some("<sip:carol@contoso.com>")
    );
    assert_eq!(
        refer.single("Referred-By").unwrap(),
        Some("<sip:alice@contoso.com>")
    );
    let out = e.on_message(&answer(&mut server, &refer, 202, None, "", &[]), NOW);
    assert!(events(&out).contains(&Event::TransferResult {
        call: 1,
        status: 202
    }));
    assert!(sends(&out).iter().any(|m| m.method() == Some("BYE")));
}

/// RFC 5589 §7 / RFC 3891 §3: attended transfer – Refer-To the consultation peer with Replaces
/// (to-tag = its tag, from-tag = ours); completed by NOTIFY 200, then our call ends.
#[test]
fn attended_transfer() {
    let (mut e, mut server, _) = signed_in();
    let first = active_call(&mut e, &mut server, "sip:bob@contoso.com");
    let consult = active_call(&mut e, &mut server, "sip:carol@contoso.com");
    let out = e.command(
        Command::Transfer {
            call: 1,
            to: TransferTarget::Call(2),
        },
        NOW,
    );
    let refer = find(&out, "REFER");
    let rt = refer.single("Refer-To").unwrap().unwrap().to_string();
    let cid = consult.call_id().unwrap().unwrap().replace('@', "%40");
    assert!(
        rt.contains(&format!(
            "?Replaces={cid}%3Bto-tag%3Dsrv1%3Bfrom-tag%3D{}",
            our_tag(&consult)
        )),
        "{rt}"
    );
    let out = e.on_message(&answer(&mut server, &refer, 202, None, "", &[]), NOW);
    assert!(
        sends(&out).iter().all(|m| m.method() != Some("BYE")),
        "waits for the NOTIFY"
    );
    let trying = peer_request(
        &mut server,
        &first,
        "NOTIFY",
        5,
        Some("message/sipfrag;version=2.0"),
        "SIP/2.0 100 Trying\r\n",
        &[("Event", "refer"), ("Subscription-State", "active")],
    );
    let out = e.on_message(&trying, NOW);
    assert_eq!(sends(&out).pop().unwrap().status_code(), Some(200));
    assert!(events(&out).is_empty());
    let done = peer_request(
        &mut server,
        &first,
        "NOTIFY",
        6,
        Some("message/sipfrag;version=2.0"),
        "SIP/2.0 200 OK\r\n",
        &[
            ("Event", "refer"),
            ("Subscription-State", "terminated;reason=noresource"),
        ],
    );
    let out = e.on_message(&done, NOW);
    assert!(events(&out).contains(&Event::TransferResult {
        call: 1,
        status: 200
    }));
    assert!(sends(&out)
        .iter()
        .any(|m| m.method() == Some("BYE") && m.call_id().unwrap() == first.call_id().unwrap()));
}

/// RFC 3515 §2.4 / RFC 3892 / RE-64: we are transferred – 202, NOTIFY 100, the new call carries
/// Referred-By, Replaces and Ms-Sensitivity; its answer is reported with NOTIFY 200 (terminated).
#[test]
fn transferee_follows_refer() {
    let (mut e, mut server, _) = signed_in();
    let first = active_call(&mut e, &mut server, "sip:bob@contoso.com");
    let refer = peer_request(
        &mut server,
        &first,
        "REFER",
        4,
        None,
        "",
        &[
            ("Refer-To", "<sip:carol@contoso.com?Replaces=x1%40h%3Bto-tag%3Da%3Bfrom-tag%3Db&Ms-Sensitivity=private&Route=evil>"),
            ("Referred-By", "<sip:bob@contoso.com>"),
        ],
    );
    let out = e.on_message(&refer, NOW);
    let s = sends(&out);
    assert_eq!(s[0].status_code(), Some(202));
    let notify = s.iter().find(|m| m.method() == Some("NOTIFY")).unwrap();
    assert_eq!(notify.single("Event").unwrap(), Some("refer"));
    assert_eq!(
        String::from_utf8_lossy(&notify.body),
        "SIP/2.0 100 Trying\r\n"
    );
    assert!(events(&out).contains(&Event::TransferRequested {
        call: 1,
        to: "sip:carol@contoso.com".into()
    }));

    let out = e.command(
        Command::FollowTransfer {
            call: 1,
            local: Box::new(media_local(41000, 8)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    let invite = find(&out, "INVITE");
    assert_eq!(
        Engine::<Counter>::request_uri(&invite),
        Some("sip:carol@contoso.com")
    );
    assert_eq!(
        invite.single("Referred-By").unwrap(),
        Some("<sip:bob@contoso.com>")
    );
    assert_eq!(
        invite.single("Replaces").unwrap(),
        Some("x1@h;to-tag=a;from-tag=b")
    );
    assert_eq!(invite.single("Ms-Sensitivity").unwrap(), Some("private"));
    assert!(
        invite.headers.iter().all(|h| !h.value.contains("evil")),
        "only Replaces and Ms-Sensitivity are taken from the Refer-To URI"
    );
    let out = e.on_message(
        &answer(
            &mut server,
            &invite,
            200,
            Some("application/sdp"),
            &peer_sdp(9, "RTP/SAVP"),
            &[],
        ),
        NOW,
    );
    let notify = sends(&out)
        .into_iter()
        .find(|m| m.method() == Some("NOTIFY"))
        .expect("result NOTIFY");
    assert_eq!(String::from_utf8_lossy(&notify.body), "SIP/2.0 200 OK\r\n");
    assert_eq!(
        notify.single("Subscription-State").unwrap(),
        Some("terminated;reason=noresource")
    );

    // A REFER to something that is not a SIP URI is refused.
    let second = active_call(&mut e, &mut server, "sip:dave@contoso.com");
    let bad = peer_request(
        &mut server,
        &second,
        "REFER",
        4,
        None,
        "",
        &[("Refer-To", "<tel:+491234>")],
    );
    assert_eq!(
        sends(&e.on_message(&bad, NOW)).pop().unwrap().status_code(),
        Some(400)
    );
}

/// RFC 3891 §3: an INVITE with Replaces naming our established call replaces it (answered, the
/// old call ends); one naming no dialog of ours gets 481.
#[test]
fn transfer_target_replaces() {
    let (mut e, mut server, _) = signed_in();
    let first = active_call(&mut e, &mut server, "sip:bob@contoso.com");
    let replaces = format!(
        "{};to-tag={};from-tag=srv1",
        first.call_id().unwrap().unwrap(),
        our_tag(&first)
    );
    let mut invite = Message::request("INVITE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    invite.push_header("Via", "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKrep");
    invite.push_header("From", "<sip:carol@contoso.com>;tag=carol1");
    invite.push_header("To", "<sip:alice@contoso.com>");
    invite.push_header("Call-ID", "replacing1");
    invite.push_header("CSeq", "1 INVITE");
    invite.push_header("Contact", "<sip:carol@contoso.com;opaque=user:epid:c;gruu>");
    invite.push_header("Content-Type", "application/sdp");
    invite.body = peer_sdp(9, "RTP/SAVP").into_bytes();
    let mut good = invite.clone();
    good.push_header("Replaces", &replaces);
    e.on_message(&server.sign(good), NOW);
    assert_eq!(e.replaces(2), Some(1));
    let out = e.command(
        Command::AcceptCall {
            call: 2,
            local: Box::new(media_local(42000, 3)),
            policy: commscraft_sdp::Encryption::Required,
        },
        NOW,
    );
    assert!(sends(&out).iter().any(|m| m.status_code() == Some(200)));
    assert!(sends(&out)
        .iter()
        .any(|m| m.method() == Some("BYE") && m.call_id().unwrap() == first.call_id().unwrap()));

    // Replaces naming no dialog of ours: 481.
    let mut unknown = Message::request("INVITE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
    for h in invite.headers.iter().filter(|h| h.name != "Call-ID") {
        unknown.push_header(&h.name, &h.value);
    }
    unknown.push_header("Call-ID", "replacing2");
    unknown.push_header("Replaces", "unknown@x;to-tag=1;from-tag=2");
    unknown.body = invite.body.clone();
    let out = e.on_message(&server.sign(unknown), NOW);
    assert_eq!(sends(&out).pop().unwrap().status_code(), Some(481));
}

/// RE-50 / [MS-PRES] §2.2.2.7.7, PR-05: forwarding is published as routing preamble to the five
/// containers; a version conflict is retried with the server's versions; success is reported.
#[test]
fn forwarding_publication() {
    let (mut e, mut server, _) = signed_in();
    let rules = pres::Forwarding {
        forward_to: e.phone_uri("+4940123456"),
        immediate: false,
        simultaneous_ring: None,
        ring_seconds: Some(15),
        e911_active: false,
        ..pres::Forwarding::default()
    };
    let out = e.command(Command::SetForwarding(rules.clone()), NOW);
    let first = sends(&out).pop().unwrap();
    assert_eq!(first.method(), Some("SERVICE"));
    let body = String::from_utf8_lossy(&first.body).to_string();
    assert!(
        body.contains("categoryName=\"routing\" instance=\"0\" container=\"400\" version=\"0\""),
        "{body}"
    );
    assert!(
        body.contains("<target uri=\"sip:+4940123456@contoso.com;user=phone\"/>")
            && body.contains("value=\"enablecf\"")
            && body.contains("seconds=\"15\"")
    );
    let fault = r#"<Fault><Faultcode>Client.BadCall.WrongDelta</Faultcode><details><operation index="5" version="0" curVersion="7"/></details></Fault>"#;
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
    assert!(String::from_utf8_lossy(&retry.body).contains("container=\"400\" version=\"7\""));
    let out = e.on_message(&answer(&mut server, &retry, 200, None, "", &[]), NOW);
    assert_eq!(events(&out), vec![Event::Forwarding(rules)]);
    // An unsafe target never reaches the server.
    let bad = pres::Forwarding {
        simultaneous_ring: Some("sip:x@y\"/>".into()),
        ..pres::Forwarding::default()
    };
    let out = e.command(Command::SetForwarding(bad), NOW);
    assert!(sends(&out).is_empty());
}

/// RE-72 (inference from RFC 3892 §3): a call that reaches us through team or delegate ringing
/// carries Referred-By with the address it was meant for; it is reported checked, never our own.
#[test]
fn incoming_call_on_behalf_of() {
    let (mut e, mut server, _) = signed_in();
    let invite = |referred: &str, id: &str| {
        let mut m = Message::request("INVITE", "sip:alice@contoso.com;opaque=user:epid:x;gruu");
        m.push_header(
            "Via",
            &format!("SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bK{id}"),
        );
        m.push_header("From", "<sip:carol@contoso.com>;tag=c1");
        m.push_header("To", "<sip:alice@contoso.com>");
        m.push_header("Call-ID", id);
        m.push_header("CSeq", "1 INVITE");
        m.push_header("Contact", "<sip:carol@contoso.com;opaque=user:epid:c;gruu>");
        m.push_header("Referred-By", referred);
        m.push_header("Content-Type", "application/sdp");
        m.body = peer_sdp(9, "RTP/SAVP").into_bytes();
        m
    };
    e.on_message(
        &server.sign(invite("<sip:bob@contoso.com>;ms-identity=\"x\"", "t1")),
        NOW,
    );
    assert_eq!(
        e.caller(1).unwrap().on_behalf_of.as_deref(),
        Some("sip:bob@contoso.com")
    );
    e.on_message(
        &server.sign(invite("<sip:bob@contoso.com\"><x>", "t2")),
        NOW,
    );
    assert_eq!(e.caller(2).unwrap().on_behalf_of, None);
    e.on_message(&server.sign(invite("<sip:alice@contoso.com>", "t3")), NOW);
    assert_eq!(
        e.caller(3).unwrap().on_behalf_of,
        None,
        "never our own address"
    );
}
