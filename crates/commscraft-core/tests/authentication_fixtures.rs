//! Public synthetic fixtures – NEVER use these keys as real credentials.
use commscraft_core::challenge::{ChallengeOffer, ChallengeSource, Scheme};
use commscraft_core::security::{SecurityAssociation, SecurityError, TlsDskHmacSha256};
use commscraft_core::sip::parse_frame;

fn offer() -> ChallengeOffer {
    ChallengeOffer {
        scheme: Scheme::TlsDsk,
        source: ChallengeSource::Registrar,
        realm: Some("SIP Communications Service".into()),
        targetname: Some("server.example.com".into()),
        opaque: Some("opaque123".into()),
        version: Some(4),
        sts_uri: None,
        gssapi_data: None,
    }
}
#[test]
fn fixture_signed_response_and_corrupted_mic() {
    let ok = include_bytes!("../../../tests/fixtures/sip/signed-register-ok.sip");
    let bad = include_bytes!("../../../tests/fixtures/sip/signed-register-invalid-mic.sip");
    let (ok_msg, ok_n) = parse_frame(ok).unwrap().unwrap();
    let (bad_msg, bad_n) = parse_frame(bad).unwrap().unwrap();
    assert_eq!(ok_n, ok.len());
    assert_eq!(bad_n, bad.len());
    let mut sa = SecurityAssociation::begin(&offer(), 100_000).unwrap();
    let verify = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
    assert_eq!(
        sa.verify_server_message(&bad_msg, &verify, 1),
        Err(SecurityError::InvalidMic)
    );
    assert_eq!(sa.highest_verified(), None);
    sa.verify_server_message(&ok_msg, &verify, 2).unwrap();
    assert_eq!(sa.highest_verified(), Some(200));
    assert_eq!(
        sa.verify_server_message(&ok_msg, &verify, 3),
        Err(SecurityError::Replay)
    );
}
