use super::*;
use commscraft_sip_msg::{parse, Limits};

fn wire(lines: &[&str]) -> Message {
    let mut s = lines.join("\r\n");
    s.push_str("\r\n\r\n");
    parse(s.as_bytes(), &Limits::default()).unwrap()
}

/// [MS-SIPAE] 20250520 §4.1 step 8 (p. 43), PDF line breaks joined.
fn ntlm_200_ok() -> Message {
    wire(&[
        "SIP/2.0 200 OK",
        "Authentication-Info: NTLM rspauth=\"01000000000000005CD422F0C750C7C6\", srand=\"0B9D33A2\", snum=\"1\", opaque=\"BCDC0C9D\", qop=\"auth\", targetname=\"server.contoso.com\", realm=\"SIP Communications Service\"",
        "From: <sip:alice@contoso.com>;tag=4a2b44d131;epid=8248ca9ebb",
        "To: <sip:alice@contoso.com>;tag=0858513FA91D3AAE1A5840DDB99599DF",
        "Call-ID: d5f2b95d5be64c2cbfb38aa5d3a87ae7",
        "CSeq: 171 REGISTER",
        "Via: SIP/2.0/TLS 192.0.2.1:4320;ms-received-cid=1C500",
        "Contact: <sip:192.0.2.1:4320;transport=tls;msreceivedcid=1C500>;expires=7200;+sip.instance=\"<urn:uuid:4233fd41093b5fd6b5d2651ed55969e6>\";gruu=\"sip:alice@contoso.com;opaque=user:epid:Qf0zQjsJ1l10mUe1Vlp5gAA;gruu\"",
        "Expires: 7200",
        "Content-Length: 0",
    ])
}

/// [MS-SIPAE] 20250520 §4.2 step 6 (pp. 44–45).
fn kerberos_200_ok() -> Message {
    wire(&[
        "SIP/2.0 200 OK",
        "Authentication-Info: Kerberos rspauth=\"602306092\", srand=\"211639C4\", snum=\"1\", opaque=\"A9A0BB9C\", qop=\"auth\", targetname=\"sip/server.contoso.com\", realm=\"SIP Communications Service\"",
        "From: <sip:alice@contoso.com>;tag=604168c9c0;epid=2ebb6f264f",
        "To: <sip:alice@contoso.com>;tag=9588410E2DA11CEE9D0AE7733E07830F",
        "Call-ID: c7142b90f8c94668807a382f552a6770",
        "CSeq: 2 REGISTER",
        "Via: SIP/2.0/TLS 192.0.2.1:4849;ms-received-cid=900",
        "Contact: <sip:192.0.2.1:4849;transport=tls;msreceivedcid=900>;expires=7200;+sip.instance=\"<urn:uuid:124841E4264D52E896C5D22AA8CDC316>\";gruu=\"sip:alice@contoso.com;opaque=user:epid:5EFIEk0m6FKWxdIqqM3DFgAA;gruu\"",
        "Expires: 7200",
        "Content-Length: 0",
    ])
}

const NTLM_CHALLENGE: &str = "NTLM opaque=\"BCDC0C9D\", gssapi-data=\"12345678ABCDEF\", targetname=\"server.contoso.com\", realm=\"SIP Communications Service\", version=3";

/// §4.1 step 7 (p. 43): the buffer printed for the 200 OK (recomputed from the message).
#[test]
fn ntlm_example_buffer() {
    let m = ntlm_200_ok();
    let info = AuthInfo::parse(m.header_values("Authentication-Info").next().unwrap()).unwrap();
    assert_eq!((info.srand.as_str(), info.snum), ("0B9D33A2", 1));
    assert_eq!(info.rspauth.len(), 16);
    let buf = signature_buffer(
        &BufferParams {
            scheme: Scheme::Ntlm,
            rand: &info.srand,
            num: &info.snum_text,
            realm: &info.realm,
            targetname: &info.targetname,
            version: Some(3),
        },
        &m,
        Direction::Incoming,
    )
    .unwrap();
    assert_eq!(
        buf,
        "<NTLM><0B9D33A2><1><SIP Communications Service><server.contoso.com>\
         <d5f2b95d5be64c2cbfb38aa5d3a87ae7><171><REGISTER><sip:alice@contoso.com><4a2b44d131>\
         <sip:alice@contoso.com><0858513FA91D3AAE1A5840DDB99599DF><><><7200><200>"
    );
}

/// §4.2 step 5 (p. 44): the Kerberos buffer, target with the `sip/` prefix.
#[test]
fn kerberos_example_buffer() {
    let m = kerberos_200_ok();
    // The example's rspauth "602306092" has 9 hex digits and cannot be Base16-decoded (K-AE-4):
    // the parser refuses it instead of guessing.
    let raw = m.header_values("Authentication-Info").next().unwrap();
    assert_eq!(AuthInfo::parse(raw), Err(Error::BadAuthHeader));
    let buf = signature_buffer(
        &BufferParams {
            scheme: Scheme::Kerberos,
            rand: "211639C4",
            num: "1",
            realm: "SIP Communications Service",
            targetname: "sip/server.contoso.com",
            version: Some(3),
        },
        &m,
        Direction::Incoming,
    )
    .unwrap();
    assert_eq!(
        buf,
        "<Kerberos><211639C4><1><SIP Communications Service><sip/server.contoso.com>\
         <c7142b90f8c94668807a382f552a6770><2><REGISTER><sip:alice@contoso.com><604168c9c0>\
         <sip:alice@contoso.com><9588410E2DA11CEE9D0AE7733E07830F><><><7200><200>"
    );
}

/// Version 2 omits fields 11, 13 and 14 (§3.2.4.1 step 2); PAI and PPI fill 13/14 from v3.
#[test]
fn version_dependent_fields_and_identities() {
    let mut m = Message::request("INVITE", "sip:bob@contoso.com");
    m.push_header("From", "<sip:alice@contoso.com>;tag=1");
    m.push_header("To", "<sip:bob@contoso.com>");
    m.push_header("Call-ID", "c1");
    m.push_header("CSeq", "5 INVITE");
    m.push_header(
        "P-Preferred-Identity",
        "<sip:alice@contoso.com>, <tel:+4930123>",
    );
    let p = |version| BufferParams {
        scheme: Scheme::Ntlm,
        rand: "00000001",
        num: "7",
        realm: "R",
        targetname: "T",
        version,
    };
    assert_eq!(
        signature_buffer(&p(None), &m, Direction::Outgoing).unwrap(),
        "<NTLM><00000001><7><R><T><c1><5><INVITE><sip:alice@contoso.com><1><><>"
    );
    assert_eq!(
        signature_buffer(&p(Some(3)), &m, Direction::Outgoing).unwrap(),
        "<NTLM><00000001><7><R><T><c1><5><INVITE><sip:alice@contoso.com><1><sip:bob@contoso.com><>\
         <sip:alice@contoso.com><tel:+4930123><>"
    );
    // A server message is only covered by P-Asserted-Identity.
    assert_eq!(
        signature_buffer(&p(Some(3)), &m, Direction::Incoming).unwrap(),
        "<NTLM><00000001><7><R><T><c1><5><INVITE><sip:alice@contoso.com><1><sip:bob@contoso.com><><><><>"
    );
}

#[test]
fn challenges_from_the_examples() {
    let c = Challenge::parse(NTLM_CHALLENGE, false).unwrap().unwrap();
    assert_eq!(c.scheme, Scheme::Ntlm);
    assert_eq!(c.realm, "SIP Communications Service");
    assert_eq!(c.targetname, "server.contoso.com");
    assert_eq!(c.opaque.as_deref(), Some("BCDC0C9D"));
    assert_eq!(c.gssapi_data.as_deref(), Some("12345678ABCDEF"));
    assert_eq!(c.version, Some(3));
    // §4.2 step 2 (p. 43)
    let k = Challenge::parse(
        "Kerberos realm=\"SIP Communications Service\", targetname=\"sip/server.contoso.com\", version=3",
        true,
    )
    .unwrap()
    .unwrap();
    assert_eq!((k.scheme, k.proxy), (Scheme::Kerberos, true));
    assert_eq!(
        Challenge::parse("Digest realm=\"x\", nonce=\"y\"", false),
        Ok(None)
    );
}

#[test]
fn hostile_challenges_are_refused() {
    for bad in [
        "NTLM realm=\"a\", realm=\"b\", targetname=\"t\", version=3",
        "NTLM realm=\"a\\\"b\", targetname=\"t\", version=3",
        "NTLM realm=\"a<b>\", targetname=\"t\", version=3",
        "NTLM realm=\"a\r\nX: y\", targetname=\"t\", version=3",
        "NTLM realm=\"a\", targetname=\"t\", version=99999",
        "NTLM realm=\"a\", targetname=\"t\" version=3",
        "NTLM realm=\"a\", targetname=\"\", version=3",
        "NTLM targetname=\"t\", version=3",
        "Kerberos realm=\"a\", targetname=\"server\", version=3",
        "NTLM realm=\"unterminated, targetname=\"t\"",
        "NTLM",
    ] {
        assert!(Challenge::parse(bad, false).is_err(), "{bad:?}");
    }
    for bad in [
        "NTLM rspauth=\"zz\", srand=\"0B9D33A2\", snum=\"1\", targetname=\"t\", realm=\"r\"",
        "NTLM rspauth=\"00\", srand=\"0B9D33\", snum=\"1\", targetname=\"t\", realm=\"r\"",
        "NTLM rspauth=\"00\", srand=\"0B9D33A2\", snum=\"-1\", targetname=\"t\", realm=\"r\"",
        "NTLM rspauth=\"00\", srand=\"0B9D33A2\", snum=\"99999999999\", targetname=\"t\", realm=\"r\"",
        "Digest rspauth=\"00\", srand=\"0B9D33A2\", snum=\"1\", targetname=\"t\", realm=\"r\"",
    ] {
        assert!(AuthInfo::parse(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn downgrade_below_version_3_is_refused() {
    let c = Challenge::parse(
        "NTLM realm=\"SIP Communications Service\", targetname=\"server.contoso.com\"",
        false,
    )
    .unwrap()
    .unwrap();
    let err =
        SecurityAssociation::new(c, NtlmMic::client(&[7; 16]), &Policy::default()).unwrap_err();
    assert_eq!(err, Error::VersionTooLow(None));
}

/// A server-signed 200 OK for the example message, produced with the server's NTLM keys.
fn signed_by_server(mut m: Message, key: &[u8; 16], snum: u64) -> Message {
    m.headers.retain(|h| h.name != "Authentication-Info");
    let srand = "0B9D33A2";
    let snum_text = snum.to_string();
    let buf = signature_buffer(
        &BufferParams {
            scheme: Scheme::Ntlm,
            rand: srand,
            num: &snum_text,
            realm: "SIP Communications Service",
            targetname: "server.contoso.com",
            version: Some(3),
        },
        &m,
        Direction::Incoming,
    )
    .unwrap();
    let token = NtlmMic::server(key).get_mic(buf.as_bytes()).unwrap();
    let hex: String = token.iter().map(|b| format!("{b:02X}")).collect();
    m.push_header(
        "Authentication-Info",
        &format!(
            "NTLM rspauth=\"{hex}\", srand=\"{srand}\", snum=\"{snum}\", opaque=\"BCDC0C9D\", qop=\"auth\", targetname=\"server.contoso.com\", realm=\"SIP Communications Service\""
        ),
    );
    m
}

fn ntlm_sa(key: &[u8; 16]) -> SecurityAssociation<NtlmMic> {
    let c = Challenge::parse(NTLM_CHALLENGE, false).unwrap().unwrap();
    SecurityAssociation::new(c, NtlmMic::client(key), &Policy::default()).unwrap()
}

#[test]
fn verify_establishes_and_rejects_tampering_and_replays() {
    let key = [0x55; 16];
    let mut sa = ntlm_sa(&key);
    // The example's own rspauth was not made with our key.
    assert_eq!(sa.verify(&ntlm_200_ok()), Err(Error::BadSignature));
    assert_eq!(sa.state(), SaState::Establishing);

    let ok = signed_by_server(ntlm_200_ok(), &key, 1);
    assert_eq!(
        sa.verify(&ok),
        Ok(Verified {
            snum: 1,
            newly_established: true
        })
    );
    assert_eq!(sa.state(), SaState::Established);
    assert_eq!(sa.verify(&ok), Err(Error::Replay));

    let mut tampered = signed_by_server(ntlm_200_ok(), &key, 2);
    for h in tampered.headers.iter_mut().filter(|h| h.name == "Expires") {
        h.value = "0".into();
    }
    assert_eq!(sa.verify(&tampered), Err(Error::BadSignature));

    // A duplicated To header (header confusion) is refused before any signature check.
    let mut dup = signed_by_server(ntlm_200_ok(), &key, 3);
    dup.push_header("To", "<sip:mallory@contoso.com>");
    assert!(matches!(sa.verify(&dup), Err(Error::Message(_))));

    // Wrong realm: no matching SA.
    let mut other = signed_by_server(ntlm_200_ok(), &key, 4);
    for h in other
        .headers
        .iter_mut()
        .filter(|h| h.name == "Authentication-Info")
    {
        h.value = h.value.replace("SIP Communications Service", "Other");
    }
    assert_eq!(sa.verify(&other), Err(Error::NoMatchingSa));
}

#[test]
fn forbidden_while_establishing_asks_for_new_credentials() {
    let key = [0x11; 16];
    let mut sa = ntlm_sa(&key);
    let mut forbidden = ntlm_200_ok();
    forbidden.start = commscraft_sip_msg::StartLine::Response {
        code: 403,
        reason: "Forbidden".into(),
    };
    let signed = signed_by_server(forbidden, &key, 1);
    assert_eq!(sa.verify(&signed), Err(Error::CredentialsRejected));
}

#[test]
fn signing_produces_a_verifiable_authorization_header() {
    let key = [0x23; 16];
    let mut sa = ntlm_sa(&key);
    let mut req = Message::request("SUBSCRIBE", "sip:alice@contoso.com");
    assert_eq!(sa.sign(&mut req, 1), Err(Error::WrongState));
    sa.verify(&signed_by_server(ntlm_200_ok(), &key, 1))
        .unwrap();

    req.push_header(
        "From",
        "<sip:alice@contoso.com>;tag=4a2b44d131;epid=8248ca9ebb",
    );
    req.push_header("To", "<sip:alice@contoso.com>");
    req.push_header("Call-ID", "123");
    req.push_header("CSeq", "1 SUBSCRIBE");
    sa.sign(&mut req, 0x1d7d_4ecf).unwrap();
    let auth = req
        .header_values("Authorization")
        .next()
        .unwrap()
        .to_string();
    let (scheme, params) = parse_auth_params(&auth).unwrap();
    assert_eq!(scheme, "NTLM");
    let get = |n: &str| params.iter().find(|(k, _)| k == n).map(|(_, v)| v.as_str());
    assert_eq!(get("crand"), Some("1d7d4ecf"));
    assert_eq!(get("cnum"), Some("1"));
    assert_eq!(get("opaque"), Some("BCDC0C9D"));
    assert_eq!(get("qop"), Some("auth"));
    assert_eq!(get("version"), Some("3"));

    // The server recomputes the buffer and checks the response with the client keys.
    let mut unsigned = req.clone();
    unsigned.headers.retain(|h| h.name != "Authorization");
    let buf = signature_buffer(
        &BufferParams {
            scheme: Scheme::Ntlm,
            rand: "1d7d4ecf",
            num: "1",
            realm: "SIP Communications Service",
            targetname: "server.contoso.com",
            version: Some(3),
        },
        &unsigned,
        Direction::Outgoing,
    )
    .unwrap();
    let response = get("response").unwrap();
    let token: Vec<u8> = (0..response.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(response.get(i..i + 2).unwrap(), 16).unwrap())
        .collect();
    assert!(NtlmMic::server(&key).verify_mic(buf.as_bytes(), &token));
    assert_eq!(
        &token[12..16],
        &100u32.to_le_bytes(),
        "fixed sequence number 100 (AE-30)"
    );

    sa.sign(&mut req, 2).unwrap();
    assert_eq!(req.header_values("Authorization").count(), 2);
    assert!(req
        .header_values("Authorization")
        .nth(1)
        .unwrap()
        .contains("cnum=\"2\""));
}

#[test]
fn replay_window_of_256() {
    let mut w = ReplayWindow::default();
    w.record(1000).unwrap();
    assert_eq!(w.record(1000), Err(Error::Replay));
    w.record(744).unwrap(); // exactly 256 below
    assert_eq!(w.check(743), Err(Error::Replay)); // 257 below
    w.record(999).unwrap();
    w.record(1001).unwrap();
    assert_eq!(w.record(999), Err(Error::Replay));
    assert_eq!(w.record(744), Err(Error::Replay));
    w.record(2000).unwrap();
    assert_eq!(w.check(1743), Err(Error::Replay));
    w.record(1744).unwrap();
    assert_eq!(w.record(1001), Err(Error::Replay));
}

#[test]
fn keys_are_not_printed() {
    let mic = NtlmMic::client(&[0x5a; 16]);
    assert_eq!(format!("{mic:?}"), "NtlmMic { keys: <redacted> }");
    let printed = format!("{:?}", ntlm_sa(&[0x5a; 16]));
    assert!(printed.contains("Establishing"));
    assert!(
        !printed.contains("90") && !printed.contains("5a"),
        "{printed}"
    );
}
