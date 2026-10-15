//! Offline cross-module REGISTER/MIC tests. Keys are TEST MATERIAL, not a
//! replacement for a real TLS-DSK handshake or a trusted TLS connection.
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sfb_core::challenge::{ChallengeOffer, ChallengeSource, Scheme};
use sfb_core::registration::{
    Endpoint, MicCheckedRegisterError, MicCheckedRegisterResult, RegisterFlow,
    RegisterState, RegistrationError,
};
use sfb_core::security::{
    parse_authentication_info, server_mic_buffer, AssociationState,
    SecurityAssociation, SecurityError, TlsDskHmacSha256,
};
use sfb_core::sip::{SipMessage, StartLine};

const SERVER_KEY: [u8; 32] = [0x5a; 32]; // Synthetic only
const BRANCH: &str = "z9hG4bK-integrated-000001";
const NEXT_BRANCH: &str = "z9hG4bK-integrated-000002";

fn endpoint() -> Endpoint {
    Endpoint {
        aor: "sip:alice@example.com".into(),
        sent_by: "192.0.2.50:5061".into(),
        contact_uri: "sip:192.0.2.50:5061;transport=tls".into(),
        instance_uuid: "b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97".into(),
        epid: "492a7ce35f".into(),
        from_tag: "2d0be03279".into(),
        call_id: "aabbccdd0011@example.com".into(),
    }
}
fn flow() -> RegisterFlow { RegisterFlow::new(endpoint(), 8, BRANCH, 3600, 0).unwrap() }
fn sa() -> SecurityAssociation {
    SecurityAssociation::begin(&ChallengeOffer {
        scheme: Scheme::TlsDsk,
        source: ChallengeSource::Registrar,
        realm: Some("SIP Communications Service".into()),
        targetname: Some("server.example.com".into()),
        opaque: Some("opaque123".into()),
        version: Some(4),
        sts_uri: None,
        gssapi_data: None,
    }, 8_000_000).unwrap()
}
fn verifier() -> TlsDskHmacSha256 {
    TlsDskHmacSha256::from_server_authentication_key(SERVER_KEY)
}
fn contact(expires: u32) -> String {
    format!("<sip:192.0.2.50:5061;transport=tls>;expires={expires};+sip.instance=\"<urn:uuid:b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97>\";gruu=\"sip:alice@example.com;opaque=user:epid:synthetic;gruu\"")
}
fn signed_ok(flow: &RegisterFlow, n: u64, expiration: u32) -> SipMessage {
    let mut headers: Vec<(String, String)> = ["Via", "Call-ID", "CSeq"].iter()
        .map(|name| (name.to_string(), flow.request().header(name).unwrap().to_string()))
        .collect();
    headers.extend([
        ("From".into(), format!("<sip:alice@example.com>;tag={};epid=492a7ce35f", "2d0be03279")),
        ("To".into(), "<sip:alice@example.com>;tag=server".into()),
        ("Contact".into(), contact(expiration)),
        ("Expires".into(), expiration.to_string()),
        ("Authentication-Info".into(), format!(
            "TLS-DSK realm=\"SIP Communications Service\", targetname=\"server.example.com\", opaque=\"opaque123\", qop=auth, snum={n}, srand=abcdef09, version=4, rspauth=\"00\""
        )),
    ]);
    let mut response = SipMessage {
        start_line: StartLine::Response { code: 200, reason: "OK".into() },
        headers,
        body: Vec::new(),
    };
    // RFC/MS buffer signed by the *synthetic* server key.
    let info = parse_authentication_info(&response).unwrap();
    let data = server_mic_buffer(&response, &info).unwrap();
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&SERVER_KEY).unwrap();
    mac.update(&data);
    let signature = mac.finalize().into_bytes();
    let hex: String = signature.iter().map(|b| format!("{b:02x}")).collect();
    let auth = response.headers.iter_mut().find(|(name, _)| name == "Authentication-Info").unwrap();
    auth.1 = auth.1.replace("rspauth=\"00\"", &format!("rspauth=\"{hex}\""));
    response
}

#[test]
fn commits_only_after_transaction_binding_and_mic() {
    let mut f = flow();
    let mut association = sa();
    let response = signed_ok(&f, 1, 3600);
    let result = f.observe_mic_checked_200(&response, &mut association, &verifier(), 100).unwrap();
    assert_eq!(result, MicCheckedRegisterResult::RegistrarReportedBinding {
        expires_seconds: 3600,
        untrusted_gruu: Some("sip:alice@example.com;opaque=user:epid:synthetic;gruu".into()),
        suggested_refresh_at_ms: 2_880_100,
        expires_at_ms: 3_600_100,
    });
    assert_eq!(f.state(), RegisterState::MicCheckedAcceptance);
    assert_eq!(association.state(), AssociationState::Established);
    assert_eq!(association.highest_verified(), Some(1));
    let refresh = f.plan_refresh(NEXT_BRANCH, 150).unwrap();
    assert_eq!(refresh.request().header("CSeq"), Some("9 REGISTER"));
}

#[test]
fn a_bad_mic_never_consumes_transaction_or_replay_counter() {
    let mut f = flow();
    let mut association = sa();
    let mut corrupted = signed_ok(&f, 4, 3600);
    let auth = corrupted.headers.iter_mut().find(|(k, _)| k == "Authentication-Info").unwrap();
    auth.1 = auth.1.replace("rspauth=\"", "rspauth=\"aa");
    assert_eq!(f.observe_mic_checked_200(&corrupted, &mut association, &verifier(), 10),
        Err(MicCheckedRegisterError::Security(SecurityError::InvalidMic)));
    assert_eq!(f.state(), RegisterState::AwaitingResponse);
    assert_eq!(association.highest_verified(), None);
    let good = signed_ok(&f, 4, 3600);
    assert!(f.observe_mic_checked_200(&good, &mut association, &verifier(), 11).is_ok());
}

#[test]
fn a_mismatched_transaction_cannot_consume_a_valid_signed_sequence() {
    let mut f = flow();
    let mut association = sa();
    let mut altered = signed_ok(&f, 8, 3600);
    altered.headers.iter_mut().find(|(n,_)| n == "Via").unwrap().1.push_str("-foreign");
    assert_eq!(f.observe_mic_checked_200(&altered, &mut association, &verifier(), 1),
        Err(MicCheckedRegisterError::Transaction));
    assert_eq!(association.highest_verified(), None);
    assert_eq!(f.state(), RegisterState::AwaitingResponse);
    let good = signed_ok(&f, 8, 3600);
    assert!(f.observe_mic_checked_200(&good, &mut association, &verifier(), 2).is_ok());
}

#[test]
fn rejects_contact_expiry_different_from_signed_expires() {
    let mut f = flow();
    let mut association = sa();
    let mut response = signed_ok(&f, 2, 3600);
    response.headers.iter_mut().find(|(k, _)| k == "Contact").unwrap().1 = contact(7200);
    assert_eq!(f.observe_mic_checked_200(&response, &mut association, &verifier(), 1),
        Err(MicCheckedRegisterError::Binding(RegistrationError::InvalidResponse)));
    assert_eq!(association.highest_verified(), None);
    assert_eq!(f.state(), RegisterState::AwaitingResponse);
}

#[test]
fn rejects_bad_endpoint_binding_without_consuming_sa_replay() {
    let mut f = flow();
    let mut association = sa();
    let mut response = signed_ok(&f, 2, 3600);
    response.headers.iter_mut().find(|(k, _)| k == "Contact").unwrap().1 =
        contact(3600).replace("b43b3d1d", "d43b3d1d");
    assert_eq!(f.observe_mic_checked_200(&response, &mut association, &verifier(), 1),
        Err(MicCheckedRegisterError::Binding(RegistrationError::NoMatchingContact)));
    assert_eq!(association.highest_verified(), None);
}

#[test]
fn missing_authentication_info_is_not_a_signed_acceptance() {
    let mut f = flow();
    let mut association = sa();
    let mut response = signed_ok(&f, 5, 3600);
    response.headers.retain(|(k,_)| k != "Authentication-Info");
    assert!(matches!(f.observe_mic_checked_200(&response, &mut association, &verifier(), 1),
        Err(MicCheckedRegisterError::Security(_))));
    assert_eq!(f.state(), RegisterState::AwaitingResponse);
    assert_eq!(association.highest_verified(), None);
}

#[test]
fn contact_gruu_is_not_signed_by_sipae_and_must_remain_untrusted() {
    let mut f = flow();
    let mut association = sa();
    let mut response = signed_ok(&f, 6, 3600);
    response.headers.iter_mut().find(|(k,_)| k == "Contact").unwrap().1 =
        contact(3600).replace("synthetic", "altered");
    // The MIC succeeds because Contact is not in the MS-SIPAE buffer.
    // The API MUST label the parsed GRUU as untrusted, not as a secure route.
    let result = f.observe_mic_checked_200(&response, &mut association, &verifier(), 1).unwrap();
    match result {
        MicCheckedRegisterResult::RegistrarReportedBinding { untrusted_gruu, .. } => {
            assert!(untrusted_gruu.unwrap().contains("altered"));
        }
        _ => panic!("expected a binding claim"),
    }
}

#[test]
fn signed_removal_is_not_assumed_from_unsigned_removal() {
    let mut original = flow();
    let mut association = sa();
    let initial = signed_ok(&original, 1, 3600);
    original.observe_mic_checked_200(&initial, &mut association, &verifier(), 1).unwrap();
    let mut removal = original.plan_unregistration(NEXT_BRANCH, 2).unwrap();
    let mut response = signed_ok(&removal, 2, 0);
    response.headers.retain(|(k,_)| k != "Contact");
    // After preparing a response to unregister, the Expires = 0 is MIC-covered;
    // note the signer above signed that header already.
    assert_eq!(removal.observe_mic_checked_200(&response, &mut association, &verifier(), 3),
        Ok(MicCheckedRegisterResult::RegistrarReportedRemoval));
    assert_eq!(removal.state(), RegisterState::MicCheckedUnregistration);
}

#[test]
fn bare_observe_200_still_means_unverified_acceptance() {
    let mut f = flow();
    let response = signed_ok(&f, 1, 3600);
    f.observe(&response, 1).unwrap();
    assert_eq!(f.state(), RegisterState::UnverifiedAcceptance);
}
