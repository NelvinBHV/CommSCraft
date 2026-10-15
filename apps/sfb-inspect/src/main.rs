//! Offline-only local SIP fixture analyzer. No network and no raw header dumps.
use sfb_core::sip::{illustrative_register, SipMessage, StartLine};
use sfb_core::challenge::parse_challenges;
use sfb_core::stream::SipStreamDecoder;
use sfb_core::registration::{Endpoint, RegisterFlow, RegisterOutcome};
use sfb_core::challenge::{ChallengeOffer, ChallengeSource, Scheme};
use sfb_core::security::{SecurityAssociation, TlsDskHmacSha256, TlsDskClientHmacSha256, SecurityError, AssociationState};
use sfb_core::sip::parse_frame;
use sfb_core::kerberos::{KerberosNegotiation, KerberosError, NoKerberosProvider, KerberosPhase};
use sfb_core::presence::PresenceSubscription;
use sfb_core::im::{ImDialog, ImState};
use sfb_core::sdp::inspect_sdp;
use sfb_router::{ResponseRouter, RoutedEvent, RouterError};
use std::fs::File;
use std::io::{self, Read};

const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;

fn summary(message: &SipMessage) -> String {
    match &message.start_line {
        StartLine::Request { method, .. } => {
            // Avoid printing arbitrary unsanitized attacker-controlled strings.
            if method.len() <= 32 && method.bytes().all(|b| b.is_ascii_uppercase()) {
                format!("Request {method}, body={} bytes", message.body.len())
            } else { format!("Request [unrecognized method], body={} bytes", message.body.len()) }
        }
        StartLine::Response { code, .. } => {
            if matches!(*code, 401 | 407) {
                match parse_challenges(message) {
                    Ok(offers) => format!("Response {code}, {} authentication scheme(s) advertised; values redacted", offers.len()),
                    Err(_) => format!("Response {code}, malformed or absent authentication challenge; values redacted"),
                }
            } else {
                format!("Response {code}, body={} bytes", message.body.len())
            }
        },
    }
}

fn inspect<R: Read>(mut reader: R) -> Result<(), Box<dyn std::error::Error>> {
    let mut decoder = SipStreamDecoder::new();
    let mut chunk = [0; 4096];
    let mut total = 0;
    let mut count = 0;
    loop {
        let size = reader.read(&mut chunk)?;
        if size == 0 { break; }
        total += size;
        if total > MAX_FILE_BYTES {
            return Err(format!("input exceeds offline inspection limit of {MAX_FILE_BYTES} bytes").into());
        }
        for message in decoder.push(&chunk[..size])? {
            count += 1;
            println!("{count}: {}", summary(&message));
        }
    }
    if decoder.pending_bytes() != 0 {
        return Err("trailing incomplete SIP stream".into());
    }
    println!("Decoded {count} complete SIP message(s), no identifying headers displayed.");
    Ok(())
}

/// Fully synthetic scenario. Never opens any network connection or displays secrets.
fn register_demo() -> Result<(), Box<dyn std::error::Error>> {
    let identity = Endpoint {
        aor: "sip:alice@example.com".into(),
        sent_by: "192.0.2.50:5061".into(),
        contact_uri: "sip:192.0.2.50:5061;transport=tls".into(),
        instance_uuid: "b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97".into(),
        epid: "492a7ce35f".into(),
        from_tag: "2d0be03279".into(),
        call_id: "74c55d45a6ee404680aa55c8fe126f11".into(),
    };
    let mut flow = RegisterFlow::new(identity, 1, "z9hG4bK-offline-aabbccddeeff", 3600, 0)?;
    println!("Synthetic REGISTER prepared: CSeq 1, TLS, Event: registration, +sip.instance");
    let reply = |f: &RegisterFlow, code: u16, extras: Vec<(String, String)>| -> SipMessage {
        let mut headers: Vec<(String, String)> = ["Via", "Call-ID", "CSeq"].iter()
            .map(|k| (k.to_string(), f.request().header(k).unwrap().into()))
            .collect();
        headers.extend(extras);
        SipMessage { start_line: StartLine::Response { code, reason: "Synthetic".into() }, headers, body: vec![] }
    };
    let short = reply(&flow, 423, vec![("Min-Expires".into(), "7200".into())]);
    let result = flow.observe(&short, 100)?;
    assert_eq!(result, RegisterOutcome::IntervalTooBrief { minimum_seconds: 7200 });
    println!("Synthetic 423: Min-Expires 7200; no automatic network retry");
    let mut retry = flow.plan_interval_retry("z9hG4bK-offline-newbranch", 200)?;
    println!("New offline REGISTER plan: CSeq {}, Expires {}", retry.cseq(), retry.expiry_request());
    let challenge = reply(&retry, 401, vec![("WWW-Authenticate".into(),
        "Kerberos realm=\"SIP Communications Service\", targetname=\"sip/example.com\", version=4".into())]);
    match retry.observe(&challenge, 300)? {
        RegisterOutcome::Challenge { schemes, .. } => println!("Synthetic 401: {:?} offered; authentication NOT implemented", schemes),
        _ => return Err("unexpected demo state".into()),
    }
    println!("Stopped safely: no user authenticated, no REGISTER transmitted, no credentials stored.");
    Ok(())
}


/// Cryptographically checked *synthetic* MS-SIPAE response. The test key is
/// public and must never be used in a production TLS-DSK association.
fn sa_demo() -> Result<(), Box<dyn std::error::Error>> {
    let data = include_bytes!("../../../tests/fixtures/sip/signed-register-ok.sip");
    let (message, consumed) = parse_frame(data)?.ok_or("incomplete synthetic SIP fixture")?;
    if consumed != data.len() { return Err("fixture contains trailing bytes".into()); }
    let challenge = ChallengeOffer {
        scheme: Scheme::TlsDsk, source: ChallengeSource::Registrar,
        realm: Some("SIP Communications Service".into()),
        targetname: Some("server.example.com".into()),
        opaque: Some("opaque123".into()), version: Some(4), sts_uri: None, gssapi_data: None,
    };
    let mut association = SecurityAssociation::begin(&challenge, 100_000)?;
    let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
    association.verify_server_message(&message, &verifier, 1)?;
    if association.state() != AssociationState::Established {
        return Err("synthetic association did not establish".into());
    }
    if association.verify_server_message(&message, &verifier, 2) != Err(SecurityError::Replay) {
        return Err("replayed message was not rejected".into());
    }
    println!("Synthetic HMAC-SHA256 server MIC: verified; duplicate server sequence: rejected.");
    println!("Offline demonstration only. Public test key, no real GSS/TLS-DSK handshake or sign-in.");
    Ok(())
}


/// Demonstrates outgoing MS-SIPAE v4 signing with two DIFFERENT public
/// test keys. Neither this key nor the synthetic signed 200 has an authentic
/// server identity; generated Authorization headers must never be sent.
fn signing_demo() -> Result<(), Box<dyn std::error::Error>> {
    let data = include_bytes!("../../../tests/fixtures/sip/signed-register-ok.sip");
    let (server_message, used) = parse_frame(data)?.ok_or("invalid test fixture")?;
    if used != data.len() { return Err("extra fixture bytes".into()); }
    let offer = ChallengeOffer {
        scheme: Scheme::TlsDsk, source: ChallengeSource::Registrar,
        realm: Some("SIP Communications Service".into()),
        targetname: Some("server.example.com".into()),
        opaque: Some("opaque123".into()), version: Some(4), sts_uri: None, gssapi_data: None,
    };
    let mut sa = SecurityAssociation::begin(&offer, 100_000)?;
    let server_verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
    sa.verify_server_message(&server_message, &server_verifier, 1)?;
    let client_signer = TlsDskClientHmacSha256::from_client_authentication_key([0x6b; 32]);
    let original = illustrative_register("sip:alice@example.com")?;
    let signed = sa.sign_client_request(&original, &client_signer, 2)?;
    let wire = signed.encode()?;
    let (decoded, consumed) = parse_frame(&wire)?.ok_or("incomplete signed draft")?;
    assert_eq!(consumed, wire.len());
    assert!(decoded.header("Authorization").is_some());
    assert!(original.header("Authorization").is_none());
    println!("Synthetic SA: server MIC validated using public test key.");
    println!("Outgoing REGISTER draft: client MIC generated, cnum=1, OS-random crand.");
    println!("Round-trip: {} bytes decoded; authorization content redacted.", consumed);
    println!("NO real server authentication, GSS/TLS-DSK negotiation or network transmission.");
    Ok(())
}

/// Exercises added RFC/MS observation primitives with synthetic identities.
fn parity_protocol_demo() -> Result<(), Box<dyn std::error::Error>> {
    let mut presence = PresenceSubscription::new("presenceCall", "sip:alice@example.org",
        "sip:bob@example.org", "localTag")?;
    let subscribe = presence.offline_subscribe("client.example.org:5061", "z9hG4bKpresence123",
        "sip:alice@client.example.org", 1, 300)?;
    let _ = subscribe.encode()?;
    let notify = SipMessage { start_line: StartLine::Request { method: "NOTIFY".into(),
        uri: "sip:alice@example.org".into() }, headers: vec![
        ("Call-ID".into(), "presenceCall".into()),
        ("To".into(), "<sip:alice@example.org>;tag=localTag".into()),
        ("From".into(), "<sip:bob@example.org>;tag=remoteTag".into()),
        ("CSeq".into(), "1 NOTIFY".into()),
        ("Event".into(), "presence".into()),
        ("Subscription-State".into(), "active;expires=300".into()),
        ("Content-Type".into(), "application/pidf+xml".into()) ],
        body: b"<presence/>".to_vec() };
    println!("Synthetic PIDF subscription: {:?}", presence.observe_notify(&notify)?);
    assert!(presence.observe_notify(&notify).is_err(), "replayed NOTIFY must fail");

    let mut im = ImDialog::new("sip:alice@example.org", "sip:bob@example.org",
        "imCall", "aliceTag", 1)?;
    let accepted = SipMessage { start_line: StartLine::Response {
        code: 200, reason: "OK".into() }, headers: vec![
        ("From".into(), "<sip:alice@example.org>;tag=aliceTag".into()),
        ("To".into(), "<sip:bob@example.org>;tag=bobTag".into()),
        ("Call-ID".into(), "imCall".into()),
        ("CSeq".into(), "1 INVITE".into()),
        ("Contact".into(), "<sip:bob@edge.example.org>".into()) ], body: Vec::new() };
    if im.observe_invite_response(&accepted)? != ImState::NeedsAck {
        return Err("synthetic INVITE response must require ACK".into());
    }
    im.note_ack_sent()?;
    let template = im.offline_message("Hallo (offline)", "client.example.org:5061",
        "z9hG4bKimMessage1")?;
    let _ = template.encode()?;
    println!("Synthetic IM: INVITE 200 -> ACK noted -> MESSAGE CSeq 2 planned (not transmitted)");

    let sdp = inspect_sdp(b"v=0\r\no=- 1 1 IN IP4 192.0.2.1\r\ns=-\r\nt=0 0\r\nm=audio 49170 RTP/AVP 0\r\n")?;
    println!("Synthetic SDP: {} media description(s) inspected, no media negotiated", sdp.media.len());
    println!("No real session, auth, contacts, IM delivery or audio/video is provided.");
    Ok(())
}

/// Demonstrate MS-SIPAE v3 Kerberos challenge planning *without* any
/// credential backend or network packets. Must fail closed by default.
fn kerberos_plan_demo() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = Endpoint {
        aor: "sip:alice@example.com".into(),
        sent_by: "192.0.2.50:5061".into(),
        contact_uri: "sip:192.0.2.50:5061;transport=tls".into(),
        instance_uuid: "b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97".into(),
        epid: "492a7ce35f".into(), from_tag: "2d0be03279".into(),
        call_id: "74c55d45a6ee404680aa55c8fe126f11".into(),
    };
    let mut first = RegisterFlow::new(endpoint, 1, "z9hG4bK-first-kerberos-flow", 3600, 0)?;
    let mut headers: Vec<(String, String)> = ["Via", "Call-ID", "CSeq"].iter()
        .map(|key| ((*key).into(), first.request().header(key).unwrap().into())).collect();
    headers.push(("WWW-Authenticate".into(),
        "Kerberos realm=\"SIP Communications Service\", targetname=\"sip/server.example.com\", version=4".into()));
    let reply = SipMessage { start_line: StartLine::Response {
        code: 401, reason: "Synthetic".into() }, headers, body: vec![] };
    first.observe(&reply, 100)?;
    let retry = first.plan_authentication_retry("z9hG4bK-second-kerberos-flow", 200)?;
    let mut negotiated = KerberosNegotiation::new("sip/server.example.com")?;
    let outcome = negotiated.respond(&first, &reply, &retry, &mut NoKerberosProvider);
    if outcome != Err(KerberosError::ProviderUnavailable) || negotiated.phase() != KerberosPhase::Blocked {
        return Err("missing native Kerberos provider must block retry".into());
    }
    println!("Synthetic 401 -> new REGISTER CSeq {} planned; trusted SPN pinned.", retry.cseq());
    println!("Offline demo uses the denying GSS provider: no token or SIP packet sent.");
    Ok(())
}

/// Explicit local credential-handle self-check. Does not call
/// InitializeSecurityContext, obtain a service ticket, or send network data.
#[cfg(windows)]
fn sspi_check() -> Result<(), Box<dyn std::error::Error>> {
    match sfb_core::windows_sspi::WindowsKerberosProvider::new() {
        Ok(_) => println!("Windows Kerberos SSPI credential handle acquired. Not authenticated."),
        Err(_) => println!("Windows Kerberos SSPI unavailable for this user. No fallback."),
    }
    Ok(())
}
#[cfg(not(windows))]
fn sspi_check() -> Result<(), Box<dyn std::error::Error>> {
    println!("Native Windows Kerberos SSPI is unavailable on this platform.");
    Ok(())
}


/// Offline only: demonstrates RFC 3261 transaction multiplexing, no SIP send.
fn router_demo() -> Result<(), Box<dyn std::error::Error>> {
    let mut router = ResponseRouter::new();
    let first = illustrative_register("sip:alice@example.com")?;
    let mut second = first.clone();
    second.headers.iter_mut().find(|(key,_)| key == "Via").unwrap().1 =
        "SIP/2.0/TLS client.invalid;branch=z9hG4bK-offline-router-second".into();
    second.headers.iter_mut().find(|(key,_)| key == "CSeq").unwrap().1 = "2 REGISTER".into();
    let first_id = router.track(&first, 0)?;
    let second_id = router.track(&second, 0)?;
    let reply = |message: &SipMessage, code: u16| SipMessage {
        start_line: StartLine::Response { code, reason: "Synthetic".into() },
        headers: ["Via", "Call-ID", "CSeq"].iter().map(|key|
            ((*key).into(), message.header(key).unwrap().into())).collect(),
        body: Vec::new(),
    };
    assert_eq!(router.response(&reply(&second, 100), 1)?,
        RoutedEvent::Provisional { id: second_id, code: 100 });
    assert_eq!(router.response(&reply(&first, 401), 2)?,
        RoutedEvent::Final { id: first_id, code: 401 });
    assert_eq!(router.response(&reply(&first, 401), 3), Err(RouterError::DuplicateOrLateResponse));
    assert_eq!(router.response(&reply(&second, 200), 4)?,
        RoutedEvent::Final { id: second_id, code: 200 });
    assert!(router.is_empty());
    println!("Synthetic reliable-SIP routing: two in-flight REGISTER transactions, out-of-order 100/401/200, duplicate final quarantined.");
    println!("No network, authenticated session or Microsoft server interoperability implied.");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    let _binary = args.next();
    let first = args.next();
    let second = args.next();
    match (first.as_deref(), second) {
        (Some("--register-demo"), None) => register_demo(),
        (Some("--sa-demo"), None) => sa_demo(),
        (Some("--signing-demo"), None) => signing_demo(),
        (Some("--kerberos-plan-demo"), None) => kerberos_plan_demo(),
        (Some("--sspi-check"), None) => sspi_check(),
        (Some("--protocol-demo"), None) => parity_protocol_demo(),
        (Some("--router-demo"), None) => router_demo(),
        (Some("--demo"), None) => {
            let demo = illustrative_register("sip:alice@example.com")?;
            inspect(io::Cursor::new(demo.encode()?))
        }
        (Some(path), None) if !path.starts_with('-') => inspect(File::open(path)?),
        _ => {
            eprintln!("Usage: sfb-inspect --demo | --register-demo | --sa-demo | --signing-demo | --kerberos-plan-demo | --sspi-check | --protocol-demo | --router-demo | <local-sip-fixture-file>");
            std::process::exit(2);
        }
    }
}
