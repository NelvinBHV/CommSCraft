//! Test support for other crates: a test registrar that runs the server side of MS-SIPAE/NTLM
//! (challenge, AUTHENTICATE check, session key, signatures) and a deterministic random source.
//! Compiled only for tests or with the `testing` feature; never part of a release build.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::*;
use commscraft_ntlm::{nt_proof_str, ntowf_v2, session_base_key, Rc4};
use commscraft_sipae::{parse_auth_params, signature_buffer, BufferParams, Direction, Mic};

pub struct Counter(pub u8);

impl Random for Counter {
    fn fill(&mut self, buf: &mut [u8]) {
        for b in buf {
            self.0 = self.0.wrapping_add(37);
            *b = self.0;
        }
    }
}

pub const REALM: &str = "SIP Communications Service";
pub const TARGET: &str = "server.contoso.com";
pub const NOW: u64 = 1_800_000_000;

pub fn config() -> Config {
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

pub fn registrar() -> Registrar<Counter> {
    Registrar::new(config(), Password::new("Password".into()), Counter(1)).unwrap()
}

pub fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

pub fn from_utf16(b: &[u8]) -> String {
    String::from_utf16(
        &b.chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

/// CHALLENGE_MESSAGE in the layout of [MS-NLMP] §4.2.4.3 with the given flags, plus a timestamp.
pub fn ntlm_challenge(flags: u32) -> Vec<u8> {
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

pub const GOOD_FLAGS: u32 = 0xe28a_8233 | ntlm::flags::DATAGRAM;

#[derive(Default)]
pub struct Server {
    pub password: &'static str,
    pub flags: u32,
    pub key: Option<[u8; 16]>,
    pub snum: u64,
    pub challenge: Vec<u8>,
    pub presence_action: &'static str,
    /// MS-SIPAE version offered in the challenges (3 or 4).
    pub version: u32,
}

impl Server {
    pub fn new() -> Self {
        Self {
            password: "Password",
            flags: GOOD_FLAGS,
            presence_action: "added",
            version: 3,
            ..Self::default()
        }
    }

    pub fn reply(req: &Message, code: u16, reason: &str) -> Message {
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

    pub fn sign(&mut self, mut r: Message) -> Message {
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

    pub fn ok(&mut self, req: &Message) -> Message {
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
    pub fn accept_authenticate(&mut self, token: &[u8]) -> bool {
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

    pub fn challenge_401(&self, req: &Message) -> Message {
        let v = self.version;
        let mut r = Self::reply(req, 401, "Unauthorized");
        r.push_header(
            "WWW-Authenticate",
            &format!("NTLM realm=\"{REALM}\", targetname=\"{TARGET}\", version=\"{v}\""),
        );
        r.push_header(
            "WWW-Authenticate",
            &format!("Kerberos realm=\"{REALM}\", targetname=\"sip/{TARGET}\", version={v}"),
        );
        r
    }

    /// Checks crand/cnum/response of a signed request with the client keys (AE-29/AE-30).
    pub fn client_signature_ok(&self, req: &Message, get: &dyn Fn(&str) -> Option<String>) -> bool {
        let (Some(response), Some(crand), Some(cnum), Some(key)) =
            (get("response"), get("crand"), get("cnum"), self.key)
        else {
            return false;
        };
        let mut unsigned = req.clone();
        unsigned.headers.retain(|h| h.name != "Authorization");
        let buf = signature_buffer(
            &BufferParams {
                scheme: Scheme::Ntlm,
                rand: &crand,
                num: &cnum,
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
        NtlmMic::server(&key).verify_mic(buf.as_bytes(), &token)
    }

    pub fn handle(&mut self, req: &Message) -> Message {
        let Some(auth) = req.header_values("Authorization").next() else {
            return self.challenge_401(req);
        };
        let (_, params) = parse_auth_params(auth).unwrap();
        let get = |n: &str| params.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
        assert_eq!(
            get("version").as_deref(),
            Some(self.version.to_string().as_str()),
            "client answers with the server version"
        );
        match get("gssapi-data") {
            Some(g) if g.is_empty() => {
                self.challenge = ntlm_challenge(self.flags);
                let mut r = Self::reply(req, 401, "Unauthorized");
                r.push_header(
                    "WWW-Authenticate",
                    &format!(
                        "NTLM opaque=\"BCDC0C9D\", gssapi-data=\"{}\", targetname=\"{TARGET}\", realm=\"{REALM}\", version=\"{}\"",
                        base64::encode(&self.challenge),
                        self.version
                    ),
                );
                r
            }
            Some(g) => {
                assert_eq!(get("opaque").as_deref(), Some("BCDC0C9D"));
                if !self.accept_authenticate(&base64::decode(&g).unwrap()) {
                    return self.challenge_401(req);
                }
                // MS-SIPAE §3.2.5.1 step 6.8: with version 4 the AUTHENTICATE request is signed.
                if self.version >= 4 && !self.client_signature_ok(req, &get) {
                    self.key = None;
                    return self.challenge_401(req);
                }
                self.ok(req)
            }
            None => {
                assert!(self.client_signature_ok(req, &get), "client signature");
                self.ok(req)
            }
        }
    }
}

impl Server {
    /// Checks the client signature of any signed request or response (AE-29) with the client keys.
    pub fn verify_client(&self, msg: &Message) -> bool {
        let Some(auth) = msg.header_values("Authorization").next() else {
            return false;
        };
        let Ok((_, params)) = parse_auth_params(auth) else {
            return false;
        };
        let get = |n: &str| params.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
        self.client_signature_ok(msg, &get)
    }
}

/// A registrar of the test configuration that has signed in against `server`; returns the
/// REGISTER that was accepted (for its Call-ID and tags).
pub fn signed_in() -> (Registrar<Counter>, Server, Message) {
    let mut reg = registrar();
    let mut server = Server::new();
    let r1 = reg.start(NOW);
    let r2 = only_send_events(reg.on_message(&server.handle(&r1), NOW));
    let r3 = only_send_events(reg.on_message(&server.handle(&r2), NOW));
    let done = reg.on_message(&server.handle(&r3), NOW);
    assert!(
        matches!(done.as_slice(), [Event::Registered(_)]),
        "{done:?}"
    );
    (reg, server, r3)
}

fn only_send_events(events: Vec<Event>) -> Message {
    match events.as_slice() {
        [Event::Send(m)] => m.clone(),
        other => panic!("expected one Send, got {other:?}"),
    }
}
