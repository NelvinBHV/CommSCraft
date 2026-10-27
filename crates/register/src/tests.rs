//! Offline sign-in against a test registrar that follows MS-SIPAE §4.1 and MS-SIPREGE §4.1.1.
//! The test registrar runs the server side of NTLM with the same primitives (NTProofStr check,
//! session key decryption) and signs its responses with the server keys.

use super::*;
use commscraft_ntlm::{nt_proof_str, ntowf_v2, session_base_key, Rc4};
use commscraft_sipae::{parse_auth_params, signature_buffer, BufferParams, Direction, Mic};

struct Counter(u8);

impl Random for Counter {
    fn fill(&mut self, buf: &mut [u8]) {
        for b in buf {
            self.0 = self.0.wrapping_add(37);
            *b = self.0;
        }
    }
}

const REALM: &str = "SIP Communications Service";
const TARGET: &str = "server.contoso.com";
const NOW: u64 = 1_800_000_000;

fn config() -> Config {
    Config {
        aor: "sip:alice@contoso.com".into(),
        registrar_uri: "sip:contoso.com".into(),
        epid: "8248ca9ebb".into(),
        local_host: "192.0.2.1".into(),
        local_port: 4320,
        user_agent: "CommSCraft/0.0.0001.0".into(),
        user: "User".into(),
        domain: "Domain".into(),
        workstation: "LINUX".into(),
        subnet: Some("192.0.2.0".into()),
        policy: Policy::default(),
        ntlm_legacy: false,
    }
}

fn registrar() -> Registrar<Counter> {
    Registrar::new(config(), Password::new("Password".into()), Counter(1)).unwrap()
}

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn from_utf16(b: &[u8]) -> String {
    String::from_utf16(
        &b.chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

/// CHALLENGE_MESSAGE in the layout of [MS-NLMP] §4.2.4.3 with the given flags, plus a timestamp.
fn ntlm_challenge(flags: u32) -> Vec<u8> {
    let name = utf16("Server");
    let mut info = vec![0x02, 0x00, 0x0c, 0x00];
    info.extend(utf16("Domain"));
    info.extend([0x01, 0x00, 0x0c, 0x00]);
    info.extend(utf16("Server"));
    info.extend([0x07, 0x00, 0x08, 0x00]);
    info.extend(130_000_000_000_000_000u64.to_le_bytes());
    info.extend([0, 0, 0, 0]);
    let mut m = b"NTLMSSP\0".to_vec();
    m.extend(2u32.to_le_bytes());
    m.extend((name.len() as u16).to_le_bytes());
    m.extend((name.len() as u16).to_le_bytes());
    m.extend(56u32.to_le_bytes());
    m.extend(flags.to_le_bytes());
    m.extend([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);
    m.extend([0; 8]);
    m.extend((info.len() as u16).to_le_bytes());
    m.extend((info.len() as u16).to_le_bytes());
    m.extend((56 + name.len() as u32).to_le_bytes());
    m.extend([0; 8]);
    m.extend(name);
    m.extend(info);
    m
}

const GOOD_FLAGS: u32 = 0xe28a_8233 | ntlm::flags::DATAGRAM;

#[derive(Default)]
struct Server {
    password: &'static str,
    flags: u32,
    key: Option<[u8; 16]>,
    snum: u64,
    challenge: Vec<u8>,
    presence_action: &'static str,
}

impl Server {
    fn new() -> Self {
        Self {
            password: "Password",
            flags: GOOD_FLAGS,
            presence_action: "added",
            ..Self::default()
        }
    }

    fn reply(req: &Message, code: u16, reason: &str) -> Message {
        let mut r = Message::response(code, reason);
        for name in ["Via", "From", "Call-ID", "CSeq"] {
            for v in req.header_values(name) {
                r.push_header(name, v);
            }
        }
        r.push_header(
            "To",
            &format!(
                "{};tag=0858513FA91D3AAE1A5840DDB99599DF",
                req.header_values("To").next().unwrap()
            ),
        );
        r
    }

    fn sign(&mut self, mut r: Message) -> Message {
        let key = self.key.unwrap();
        self.snum += 1;
        let snum = self.snum.to_string();
        let buf = signature_buffer(
            &BufferParams {
                scheme: Scheme::Ntlm,
                rand: "0B9D33A2",
                num: &snum,
                realm: REALM,
                targetname: TARGET,
                version: Some(3),
            },
            &r,
            Direction::Incoming,
        )
        .unwrap();
        let mic = NtlmMic::server(&key).get_mic(buf.as_bytes()).unwrap();
        let hex: String = mic.iter().map(|b| format!("{b:02X}")).collect();
        r.push_header(
            "Authentication-Info",
            &format!("NTLM rspauth=\"{hex}\", srand=\"0B9D33A2\", snum=\"{snum}\", opaque=\"BCDC0C9D\", qop=\"auth\", targetname=\"{TARGET}\", realm=\"{REALM}\""),
        );
        r
    }

    fn ok(&mut self, req: &Message) -> Message {
        let mut r = Self::reply(req, 200, "OK");
        let expires = if req.expires().unwrap() == Some("0") {
            "0"
        } else {
            "7200"
        };
        r.push_header(
            "Contact",
            &format!(
                "<sip:192.0.2.1:4320;transport=tls;msreceivedcid=1C500>;expires={expires};+sip.instance=\"<urn:uuid:{}>\";gruu=\"sip:alice@contoso.com;opaque=user:epid:Qf0zQjsJ1l10mUe1Vlp5gAA;gruu\"",
                commscraft_sip_identity::instance_uuid_from_epid("8248ca9ebb").unwrap().replace('-', "")
            ),
        );
        r.push_header("Expires", expires);
        r.push_header(
            "ms-keep-alive",
            "UAS; tcp=no; hop-hop=yes; end-end=no; timeout=300",
        );
        r.push_header(
            "presence-state",
            &format!("register-action=\"{}\"", self.presence_action),
        );
        self.presence_action = "refreshed";
        self.sign(r)
    }

    /// Verifies the AUTHENTICATE_MESSAGE as a server would and recovers the session key.
    fn accept_authenticate(&mut self, token: &[u8]) -> bool {
        let field = |at: usize| {
            let len = usize::from(u16::from_le_bytes([token[at], token[at + 1]]));
            let off = u32::from_le_bytes(token[at + 4..at + 8].try_into().unwrap()) as usize;
            token[off..off + len].to_vec()
        };
        let nt = field(20);
        let (user, domain) = (from_utf16(&field(36)), from_utf16(&field(28)));
        let k = ntowf_v2(self.password, &user, &domain);
        let proof = nt_proof_str(
            &k,
            [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef],
            &nt[16..],
        );
        if proof[..] != nt[..16] {
            return false;
        }
        let mut key: [u8; 16] = field(52).try_into().unwrap();
        Rc4::new(&session_base_key(&k, &proof)).apply(&mut key);
        self.key = Some(key);
        true
    }

    fn handle(&mut self, req: &Message) -> Message {
        let Some(auth) = req.header_values("Authorization").next() else {
            let mut r = Self::reply(req, 401, "Unauthorized");
            r.push_header(
                "WWW-Authenticate",
                &format!("NTLM realm=\"{REALM}\", targetname=\"{TARGET}\", version=3"),
            );
            r.push_header(
                "WWW-Authenticate",
                &format!("Kerberos realm=\"{REALM}\", targetname=\"sip/{TARGET}\", version=3"),
            );
            return r;
        };
        let (_, params) = parse_auth_params(auth).unwrap();
        let get = |n: &str| params.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
        if let Some(response) = get("response") {
            // A signed request: check the client signature with the client keys.
            let mut unsigned = req.clone();
            unsigned.headers.retain(|h| h.name != "Authorization");
            let buf = signature_buffer(
                &BufferParams {
                    scheme: Scheme::Ntlm,
                    rand: &get("crand").unwrap(),
                    num: &get("cnum").unwrap(),
                    realm: REALM,
                    targetname: TARGET,
                    version: Some(3),
                },
                &unsigned,
                Direction::Outgoing,
            )
            .unwrap();
            let token: Vec<u8> = (0..response.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&response[i..i + 2], 16).unwrap())
                .collect();
            assert!(
                NtlmMic::server(&self.key.unwrap()).verify_mic(buf.as_bytes(), &token),
                "client signature"
            );
            return self.ok(req);
        }
        let gssapi = get("gssapi-data").unwrap();
        if gssapi.is_empty() {
            self.challenge = ntlm_challenge(self.flags);
            let mut r = Self::reply(req, 401, "Unauthorized");
            r.push_header(
                "WWW-Authenticate",
                &format!(
                    "NTLM opaque=\"BCDC0C9D\", gssapi-data=\"{}\", targetname=\"{TARGET}\", realm=\"{REALM}\", version=3",
                    base64::encode(&self.challenge)
                ),
            );
            return r;
        }
        assert_eq!(get("opaque").as_deref(), Some("BCDC0C9D"));
        if !self.accept_authenticate(&base64::decode(&gssapi).unwrap()) {
            let mut r = Self::reply(req, 401, "Unauthorized");
            r.push_header(
                "WWW-Authenticate",
                &format!("NTLM realm=\"{REALM}\", targetname=\"{TARGET}\", version=3"),
            );
            return r;
        }
        self.ok(req)
    }
}

fn only_send(events: Vec<Event>) -> Message {
    match events.as_slice() {
        [Event::Send(m)] => m.clone(),
        other => panic!("expected one Send, got {other:?}"),
    }
}

/// Runs the handshake up to the event after the final response.
fn sign_in(reg: &mut Registrar<Counter>, server: &mut Server) -> Vec<Event> {
    let r1 = reg.start(NOW);
    let r2 = only_send(reg.on_message(&server.handle(&r1), NOW));
    let r3 = only_send(reg.on_message(&server.handle(&r2), NOW));
    reg.on_message(&server.handle(&r3), NOW)
}

#[test]
fn full_sign_in_refresh_and_sign_out() {
    let mut reg = registrar();
    let mut server = Server::new();

    let r1 = reg.start(NOW);
    assert_eq!(r1.header_values("Authorization").count(), 0, "AE-20");
    let from = r1.from().unwrap().unwrap();
    assert_eq!(from.param("epid"), Some("8248ca9ebb"), "RE-03");
    let contact = r1.single("Contact").unwrap().unwrap();
    assert!(
        contact.contains(&format!("+sip.instance=\"<urn:uuid:{}>\"", reg.instance())),
        "RE-06"
    );
    assert_eq!(r1.single("ms-subnet").unwrap(), Some("192.0.2.0"));
    assert_eq!(
        r1.single("ms-keep-alive").unwrap(),
        Some("UAC;hop-hop=yes"),
        "CON-30"
    );
    assert!(r1
        .single("Via")
        .unwrap()
        .unwrap()
        .contains(";branch=z9hG4bK"));

    let r2 = only_send(reg.on_message(&server.handle(&r1), NOW));
    let a2 = r2.single("Authorization").unwrap().unwrap().to_string();
    assert!(
        a2.contains("gssapi-data=\"\"") && a2.contains("version=3"),
        "AE-24: {a2}"
    );
    assert_eq!(r2.cseq().unwrap().unwrap().number, 2);

    let r3 = only_send(reg.on_message(&server.handle(&r2), NOW));
    let a3 = r3.single("Authorization").unwrap().unwrap().to_string();
    assert!(
        a3.starts_with("NTLM opaque=\"BCDC0C9D\", qop=\"auth\""),
        "{a3}"
    );
    let (_, p) = parse_auth_params(&a3).unwrap();
    let token = base64::decode(&p.iter().find(|(k, _)| k == "gssapi-data").unwrap().1).unwrap();
    let flags = u32::from_le_bytes(token[60..64].try_into().unwrap());
    assert_ne!(flags & ntlm::flags::DATAGRAM, 0);
    assert_ne!(flags & ntlm::flags::EXTENDED_SESSIONSECURITY, 0);

    let events = reg.on_message(&server.handle(&r3), NOW);
    let expected = Registration {
        gruu: Some("sip:alice@contoso.com;opaque=user:epid:Qf0zQjsJ1l10mUe1Vlp5gAA;gruu".into()),
        expires: 7200,
        refresh_at: NOW + 5760,
        keepalive_timeout: Some(300),
    };
    assert_eq!(events, vec![Event::Registered(expected)]);
    assert!(reg.is_registered());

    assert!(reg.on_tick(NOW + 5759).is_empty());
    let refresh = only_send(reg.on_tick(NOW + 5760));
    assert!(refresh
        .single("Authorization")
        .unwrap()
        .unwrap()
        .contains("cnum=\"1\""));
    let events = reg.on_message(&server.handle(&refresh), NOW + 5760);
    assert!(
        matches!(events.as_slice(), [Event::Registered(r)] if r.refresh_at == NOW + 5760 + 5760)
    );

    let bye = reg.unregister(NOW + 6000).unwrap();
    assert_eq!(bye.expires().unwrap(), Some("0"), "RG-29");
    assert!(bye
        .single("Authorization")
        .unwrap()
        .unwrap()
        .contains("cnum=\"2\""));
    assert_eq!(
        reg.on_message(&server.handle(&bye), NOW + 6000),
        vec![Event::Unregistered]
    );
    assert!(!reg.is_registered());
}

#[test]
fn wrong_password_is_rejected() {
    let mut reg = registrar();
    let mut server = Server {
        password: "Other",
        ..Server::new()
    };
    assert_eq!(
        sign_in(&mut reg, &mut server),
        vec![Event::Failed(Failure::AuthenticationRejected)]
    );
}

#[test]
fn unauthenticated_success_is_refused() {
    let mut reg = registrar();
    let r1 = reg.start(NOW);
    let ok = Server::reply(&r1, 200, "OK");
    assert_eq!(
        reg.on_message(&ok, NOW),
        vec![Event::Failed(Failure::BadResponse(
            "registration without authentication"
        ))]
    );
}

#[test]
fn forged_final_response_is_dropped_and_times_out() {
    let mut reg = registrar();
    let mut server = Server::new();
    let r1 = reg.start(NOW);
    let r2 = only_send(reg.on_message(&server.handle(&r1), NOW));
    let r3 = only_send(reg.on_message(&server.handle(&r2), NOW));
    let _ = server.handle(&r3); // the real server would answer; an attacker answers instead:
    let mut forged = server.ok(&r3);
    for h in forged.headers.iter_mut().filter(|h| h.name == "Expires") {
        h.value = "60".into();
    }
    assert!(
        reg.on_message(&forged, NOW).is_empty(),
        "bad signature dropped"
    );
    let mut unsigned = server.ok(&r3);
    unsigned.headers.retain(|h| h.name != "Authentication-Info");
    assert!(
        reg.on_message(&unsigned, NOW).is_empty(),
        "unsigned 200 dropped"
    );
    assert!(!reg.is_registered());
    assert!(reg.on_tick(NOW + TRANSACTION_TIMEOUT - 1).is_empty());
    assert_eq!(
        reg.on_tick(NOW + TRANSACTION_TIMEOUT),
        vec![Event::Failed(Failure::Timeout)]
    );
}

#[test]
fn stray_responses_are_ignored() {
    let mut reg = registrar();
    let mut server = Server::new();
    let r1 = reg.start(NOW);
    let good = server.handle(&r1);
    let mut tweak = |name: &str, value: &str| {
        let mut m = good.clone();
        for h in m.headers.iter_mut().filter(|h| h.name == name) {
            h.value = value.to_string();
        }
        assert!(reg.on_message(&m, NOW).is_empty(), "{name}");
    };
    tweak("Call-ID", "other");
    tweak("CSeq", "7 REGISTER");
    tweak("Via", "SIP/2.0/TLS 192.0.2.1:4320;branch=z9hG4bKforged");
    tweak("From", "<sip:alice@contoso.com>;tag=forged");
    assert!(
        reg.on_message(&r1, NOW).is_empty(),
        "a request is not a response"
    );
    assert_eq!(reg.on_message(&good, NOW).len(), 1);
}

#[test]
fn weak_or_missing_challenges_fail_closed() {
    // Kerberos only: no supported challenge (Kerberos needs its adapter).
    let mut reg = registrar();
    let r1 = reg.start(NOW);
    let mut r = Server::reply(&r1, 401, "Unauthorized");
    r.push_header(
        "WWW-Authenticate",
        &format!("Kerberos realm=\"{REALM}\", targetname=\"sip/{TARGET}\", version=3"),
    );
    assert_eq!(
        reg.on_message(&r, NOW),
        vec![Event::Failed(Failure::NoSupportedChallenge)]
    );

    // No version parameter: version 2 is refused (downgrade protection).
    let mut reg = registrar();
    let r1 = reg.start(NOW);
    let mut r = Server::reply(&r1, 401, "Unauthorized");
    r.push_header(
        "WWW-Authenticate",
        &format!("NTLM realm=\"{REALM}\", targetname=\"{TARGET}\""),
    );
    assert_eq!(
        reg.on_message(&r, NOW),
        vec![Event::Failed(Failure::Auth(
            commscraft_sipae::Error::VersionTooLow(None)
        ))]
    );

    // NTLM without extended session security.
    let mut reg = registrar();
    let mut server = Server {
        flags: GOOD_FLAGS & !ntlm::flags::EXTENDED_SESSIONSECURITY,
        ..Server::new()
    };
    let r1 = reg.start(NOW);
    let r2 = only_send(reg.on_message(&server.handle(&r1), NOW));
    assert_eq!(
        reg.on_message(&server.handle(&r2), NOW),
        vec![Event::Failed(Failure::Ntlm(ntlm::Error::MissingFlag(
            ntlm::flags::EXTENDED_SESSIONSECURITY
        )))]
    );
}

#[test]
fn refresh_reporting_a_new_registration_means_lost() {
    let mut reg = registrar();
    let mut server = Server::new();
    assert!(matches!(
        sign_in(&mut reg, &mut server).as_slice(),
        [Event::Registered(_)]
    ));
    server.presence_action = "added";
    let refresh = only_send(reg.on_tick(NOW + 5760));
    assert_eq!(
        reg.on_message(&server.handle(&refresh), NOW + 5760),
        vec![Event::Failed(Failure::RegistrationLost)]
    );
}

#[test]
fn challenge_on_refresh_starts_over() {
    let mut reg = registrar();
    let mut server = Server::new();
    assert!(matches!(
        sign_in(&mut reg, &mut server).as_slice(),
        [Event::Registered(_)]
    ));
    let refresh = only_send(reg.on_tick(NOW + 5760));
    let mut r = Server::reply(&refresh, 401, "Unauthorized");
    r.push_header(
        "WWW-Authenticate",
        &format!("NTLM realm=\"{REALM}\", targetname=\"{TARGET}\", version=3"),
    );
    let again = only_send(reg.on_message(&r, NOW + 5760));
    assert_eq!(
        again.header_values("Authorization").count(),
        0,
        "AE-23: new SA"
    );
    assert!(!reg.is_registered());
}

/// [MS-SIPREGE] 20250819 §4.1.1 (p. 94): the REGISTER of the example carries a real NTLM
/// AUTHENTICATE_MESSAGE. It is NTLMv1 without extended session security or 128-bit keys
/// (24-byte LM and NT responses, flags 0x42908255, Windows version 5.2.3790). The CommSCraft
/// policy refuses such a server offer (OB-43; interop test IT-14).
#[test]
fn siprege_example_authenticate_is_ntlm_v1() {
    let b64 = "TlRMTVNTUAADAAAAGAAYAHoAAAAYABgAkgAAAAoACgBIAAAAEAAQAFIAAAAYABgAYgAAABAAEACqAAAAVYK\
               QQgUCzg4AAAAPTgBUAEQARQBWAHMAVQBuAGsAYQByAGEAbgBTAEEATgBLAEEAUgBBAE4ALQBEAEUAVgDnJVs84fE5\
               9ziCDecY7XprbBXcdABFbpbnJVs84fE59ziCDecY7XprbBXcdABFbpZwMLke/elE9fi69enePNxU";
    let m = base64::decode(b64).unwrap();
    assert_eq!(m.len(), 186);
    assert_eq!(&m[..12], b"NTLMSSP\0\x03\0\0\0");
    let flags = u32::from_le_bytes(m[60..64].try_into().unwrap());
    assert_eq!(flags, 0x4290_8255);
    assert_eq!(&m[12..14], &[0x18, 0x00], "24-byte LM response");
    assert_eq!(&m[20..22], &[0x18, 0x00], "24-byte NT response: NTLMv1");
    assert_eq!(flags & ntlm::flags::EXTENDED_SESSIONSECURITY, 0);
    assert!(ntlm::choose_flags(flags).is_err());
}

#[test]
fn configuration_is_validated() {
    let bad = |f: fn(&mut Config)| {
        let mut c = config();
        f(&mut c);
        Registrar::new(c, Password::new("x".into()), Counter(0)).unwrap_err()
    };
    assert_eq!(
        bad(|c| c.aor = "sip:alice@contoso.com>\r\nX: y".into()),
        ConfigError::Aor
    );
    assert_eq!(bad(|c| c.aor = "tel:+4930123".into()), ConfigError::Aor);
    assert_eq!(bad(|c| c.epid = "not-hex".into()), ConfigError::Epid);
    assert_eq!(bad(|c| c.local_host = "a;b".into()), ConfigError::LocalHost);
    assert_eq!(
        bad(|c| c.user_agent = "x\r\n".into()),
        ConfigError::UserAgent
    );
    assert_eq!(
        format!("{:?}", Password::new("secret".into())),
        "Password(<redacted>)"
    );
    assert!(!format!("{:?}", registrar()).contains("Password"));
}

/// The MS-SIPAE signature does not cover the Contact header (§3.2.4.1 step 2): a changed Contact
/// `expires` still verifies. The client therefore takes the expiry from the signed Expires header;
/// the GRUU in the Contact is only protected by TLS (documented in docs/security.md).
#[test]
fn expiry_comes_from_the_signed_header() {
    let mut reg = registrar();
    let mut server = Server::new();
    let r1 = reg.start(NOW);
    let r2 = only_send(reg.on_message(&server.handle(&r1), NOW));
    let r3 = only_send(reg.on_message(&server.handle(&r2), NOW));
    let mut ok = server.handle(&r3);
    for h in ok.headers.iter_mut().filter(|h| h.name == "Contact") {
        h.value = h.value.replace("expires=7200", "expires=60");
    }
    let events = reg.on_message(&ok, NOW);
    assert!(
        matches!(events.as_slice(), [Event::Registered(r)] if r.expires == 7200),
        "{events:?}"
    );
}

/// RG-50: the client version filter answers 403 with Warning 311 ([MS-SIPREGE] §3.4.2.5).
#[test]
fn client_version_block_is_reported() {
    let mut reg = registrar();
    let r1 = reg.start(NOW);
    let mut r = Server::reply(&r1, 403, "Forbidden");
    r.push_header(
        "Warning",
        "311 lcs.microsoft.com \"http://example.invalid/update\"",
    );
    assert_eq!(
        reg.on_message(&r, NOW),
        vec![Event::Failed(Failure::VersionBlocked(311))]
    );
}

/// A NOTIFY from the registrar in the registration dialog, signed with the server keys
/// (layout of MS-SIPREGE §4.1.3–4.1.5, pp. 96–97, with a well-formed To header: K-RG-4).
fn registrar_notify(
    server: &mut Server,
    reg_request: &Message,
    body: &str,
    extra: &[(&str, &str)],
) -> Message {
    let mut n = Message::request("NOTIFY", "sip:192.0.2.1:4320;transport=tls");
    n.push_header(
        "Via",
        "SIP/2.0/TLS 192.0.2.10:5061;branch=z9hG4bKD948963B.BFDD97BF",
    );
    n.push_header("Max-Forwards", "70");
    n.push_header(
        "From",
        "<sip:alice@contoso.com>;tag=25498F391570ACE435E286F873E6AACC",
    );
    let tag = reg_request
        .from()
        .unwrap()
        .unwrap()
        .param("tag")
        .unwrap()
        .to_string();
    n.push_header(
        "To",
        &format!("<sip:alice@contoso.com>;tag={tag};epid=8248ca9ebb"),
    );
    n.push_header("Call-ID", reg_request.call_id().unwrap().unwrap());
    n.push_header("CSeq", "1 NOTIFY");
    n.push_header("Content-Type", "text/registration-event");
    n.push_header("Event", "registration-notify");
    n.push_header("subscription-state", "terminated;expires=0");
    for (k, v) in extra {
        n.push_header(k, v);
    }
    n.body = body.as_bytes().to_vec();
    server.sign(n)
}

fn registered() -> (Registrar<Counter>, Server, Message) {
    let mut reg = registrar();
    let mut server = Server::new();
    let r1 = reg.start(NOW);
    let r2 = only_send(reg.on_message(&server.handle(&r1), NOW));
    let r3 = only_send(reg.on_message(&server.handle(&r2), NOW));
    assert!(matches!(
        reg.on_message(&server.handle(&r3), NOW).as_slice(),
        [Event::Registered(_)]
    ));
    (reg, server, r3)
}

#[test]
fn deregister_notify_ends_the_registration() {
    let (mut reg, mut server, r3) = registered();
    let n = registrar_notify(
        &mut server,
        &r3,
        "deregistered;event=rejected",
        &[("ms-diagnostics-public", "4141;reason=\"User disabled\"")],
    );
    let events = reg.on_message(&n, NOW + 10);
    assert_eq!(events.len(), 2);
    match &events[0] {
        Event::Send(ok) => {
            assert_eq!(ok.status_code(), Some(200));
            assert_eq!(ok.cseq().unwrap().unwrap().method, "NOTIFY");
            assert!(
                ok.single("Authorization").unwrap().is_some(),
                "answer is signed"
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(events[1], Event::Deregistered(Deregistration::Rejected));
    assert!(!reg.is_registered());
}

#[test]
fn survivable_mode_keeps_the_registration() {
    let (mut reg, mut server, r3) = registered();
    let n = registrar_notify(
        &mut server,
        &r3,
        "  registered;event=userservices-unavailable\r\n",
        &[],
    );
    let events = reg.on_message(&n, NOW + 10);
    assert_eq!(events.last(), Some(&Event::SurvivableMode));
    assert!(reg.is_registered());
}

#[test]
fn preferred_registrar_is_reported_validated() {
    let (mut reg, mut server, r3) = registered();
    let n = registrar_notify(
        &mut server,
        &r3,
        "deregistered;event=preferred-registrar-change",
        &[(
            "P-Preferred-Registrar",
            "<sip:server1.contoso.com:5061;transport=tls>",
        )],
    );
    assert_eq!(
        reg.on_message(&n, NOW + 10).last(),
        Some(&Event::Deregistered(
            Deregistration::PreferredRegistrarChange(Some(
                "sip:server1.contoso.com:5061;transport=tls".into()
            ))
        ))
    );
}

#[test]
fn forged_or_foreign_notifies_are_dropped() {
    let (mut reg, mut server, r3) = registered();
    // Unsigned.
    let mut n = registrar_notify(&mut server, &r3, "deregistered;event=rejected", &[]);
    n.headers.retain(|h| h.name != "Authentication-Info");
    assert!(reg.on_message(&n, NOW).is_empty());
    // Wrong dialog: other Call-ID.
    let mut n = registrar_notify(&mut server, &r3, "deregistered;event=rejected", &[]);
    for h in n.headers.iter_mut().filter(|h| h.name == "Call-ID") {
        h.value = "other".into();
    }
    assert!(reg.on_message(&n, NOW).is_empty());
    // Wrong event package.
    let mut n = registrar_notify(&mut server, &r3, "deregistered;event=rejected", &[]);
    for h in n.headers.iter_mut().filter(|h| h.name == "Event") {
        h.value = "presence".into();
    }
    assert!(reg.on_message(&n, NOW).is_empty());
    // The example form "To: sip:alice@fabrikam.com>" (K-RG-4) is malformed and dropped.
    let mut n = registrar_notify(&mut server, &r3, "deregistered;event=rejected", &[]);
    for h in n.headers.iter_mut().filter(|h| h.name == "To") {
        h.value = h.value.replacen('<', "", 1);
    }
    assert!(reg.on_message(&n, NOW).is_empty());
    // Unknown body.
    let n = registrar_notify(&mut server, &r3, "deregistered;event=whatever", &[]);
    assert!(reg.on_message(&n, NOW).is_empty());
    assert!(reg.is_registered());
}

/// O-RG-1: a signed 301 after the handshake sends the client to its home pool.
#[test]
fn signed_redirect_to_home_pool() {
    let mut reg = registrar();
    let mut server = Server::new();
    let r1 = reg.start(NOW);
    let r2 = only_send(reg.on_message(&server.handle(&r1), NOW));
    let r3 = only_send(reg.on_message(&server.handle(&r2), NOW));
    assert!(
        matches!(server.handle(&r3).status_code(), Some(200)),
        "server learns the key"
    );
    let redirect = |contact: &str, server: &mut Server| {
        let mut r = Server::reply(&r3, 301, "Moved Permanently");
        r.push_header("Contact", contact);
        server.sign(r)
    };
    // Unsigned: dropped.
    let mut unsigned = redirect("<sip:pool2.contoso.com:5061;transport=tls>", &mut server);
    unsigned.headers.retain(|h| h.name != "Authentication-Info");
    assert!(reg.on_message(&unsigned, NOW).is_empty());
    // Plain TCP target: refused.
    let mut reg2 = registrar();
    let mut s2 = Server::new();
    let a = reg2.start(NOW);
    let b = only_send(reg2.on_message(&s2.handle(&a), NOW));
    let c = only_send(reg2.on_message(&s2.handle(&b), NOW));
    let _ = s2.handle(&c);
    let mut tcp = Server::reply(&c, 301, "Moved Permanently");
    tcp.push_header("Contact", "<sip:pool2.contoso.com:5060;transport=tcp>");
    let tcp = s2.sign(tcp);
    assert_eq!(
        reg2.on_message(&tcp, NOW),
        vec![Event::Failed(Failure::BadResponse("unusable redirect"))]
    );
    // Signed, TLS, in the domain.
    let ok = redirect("<sip:pool2.contoso.com:5061;transport=tls>", &mut server);
    assert_eq!(
        reg.on_message(&ok, NOW),
        vec![Event::Redirected(RedirectTarget {
            host: "pool2.contoso.com".into(),
            port: 5061,
            in_user_domain: true
        })]
    );
}

#[test]
fn ntlm_diagnostics_and_legacy_mode() {
    for legacy in [false, true] {
        let mut cfg = config();
        cfg.ntlm_legacy = legacy;
        let mut reg = Registrar::new(cfg, Password::new("Password".into()), Counter(1)).unwrap();
        let mut server = Server::new();
        let r1 = reg.start(NOW);
        let r2 = only_send(reg.on_message(&server.handle(&r1), NOW));
        let r3 = only_send(reg.on_message(&server.handle(&r2), NOW));
        let info = reg.ntlm_info().unwrap().clone();
        assert_eq!(info.sipae_version, Some(3));
        assert!(info.has_timestamp);
        assert_eq!(info.modern, !legacy);
        assert!(info
            .names
            .contains(&("NetBIOS domain", "Domain".to_string())));
        assert!(NtlmInfo::flag_names(info.chosen_flags.unwrap()).contains(&"DATAGRAM"));
        // Both variants are accepted by the test registrar.
        assert!(
            matches!(
                reg.on_message(&server.handle(&r3), NOW).as_slice(),
                [Event::Registered(_)]
            ),
            "legacy={legacy}"
        );
        let a3 = r3.single("Authorization").unwrap().unwrap().to_string();
        let (_, p) = parse_auth_params(&a3).unwrap();
        let token = base64::decode(&p.iter().find(|(k, _)| k == "gssapi-data").unwrap().1).unwrap();
        let lm_len = u16::from_le_bytes([token[12], token[13]]);
        assert_eq!(lm_len, if legacy { 24 } else { 0 });
    }
}
