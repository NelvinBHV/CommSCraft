//! Synthetic, credential-free interoperability fixtures.
use commscraft_core::sip::{parse_frame, SipMessage};
use commscraft_core::challenge::{parse_challenges, Scheme, ChallengeSource};
use commscraft_core::stream::SipStreamDecoder;
use commscraft_core::transaction::{NonInviteClientTransaction, TransactionEvent, TransactionState};

const REGISTER: &[u8] = include_bytes!("../../../tests/fixtures/sip/register.sip");
const TRYING: &[u8] = include_bytes!("../../../tests/fixtures/sip/trying.sip");
const UNAUTHORIZED: &[u8] = include_bytes!("../../../tests/fixtures/sip/unauthorized.sip");
const CHALLENGE: &[u8] = include_bytes!("../../../tests/fixtures/sip/challenge.sip");
const TOO_BRIEF: &[u8] = include_bytes!("../../../tests/fixtures/sip/register-423.sip");
const MS_GRUU_OK: &[u8] = include_bytes!("../../../tests/fixtures/sip/register-ms-gruu-200.sip");

fn parse(raw: &[u8]) -> SipMessage {
    let (message, used) = parse_frame(raw).unwrap().unwrap();
    assert_eq!(raw.len(), used);
    message
}

#[test]
fn parses_fragments_at_every_possible_split() {
    let complete = [REGISTER, TRYING, UNAUTHORIZED].concat();
    for cut in 0..=complete.len() {
        let mut decoder = SipStreamDecoder::new();
        let first = decoder.push(&complete[..cut]).unwrap();
        let second = decoder.push(&complete[cut..]).unwrap();
        assert_eq!(first.len() + second.len(), 3, "split at byte {cut}");
        assert_eq!(decoder.pending_bytes(), 0);
    }
}

#[test]
fn modeled_register_challenge_is_terminal_but_not_login() {
    let request = parse(REGISTER);
    let mut txn = NonInviteClientTransaction::new(&request, 1000).unwrap();
    assert_eq!(txn.receive_response(&parse(TRYING), 1200).unwrap(), TransactionEvent::Provisional(100));
    assert_eq!(txn.receive_response(&parse(UNAUTHORIZED), 1300).unwrap(), TransactionEvent::Final(401));
    assert_eq!(txn.state(), TransactionState::Terminated);
}

#[test]
fn microsoft_auth_challenges_are_parsed_without_authenticating() {
    let response = parse(CHALLENGE);
    let offers = parse_challenges(&response).unwrap();
    assert_eq!(offers.len(), 2);
    assert_eq!(offers[0].scheme, Scheme::Ntlm);
    assert_eq!(offers[1].scheme, Scheme::Kerberos);
    assert_eq!(offers[0].source, ChallengeSource::Registrar);
    assert_eq!(offers[1].targetname.as_deref(), Some("sip/server.example.com"));
    assert!(!format!("{:?}", offers).contains("server.example.com"));
}

#[test]
fn rfc_423_and_ms_registration_response_fixtures_are_bounded() {
    use commscraft_core::sip::StartLine;
    let short = parse(TOO_BRIEF);
    assert!(matches!(short.start_line, StartLine::Response { code: 423, .. }));
    assert_eq!(short.header("Min-Expires"), Some("7200"));
    let ok = parse(MS_GRUU_OK);
    assert!(matches!(ok.start_line, StartLine::Response { code: 200, .. }));
    assert!(ok.header("Contact").unwrap().contains("+sip.instance="));
    assert!(ok.header("Contact").unwrap().contains(";gruu=\"sip:"));
    assert_eq!(ok.header("Content-Length"), Some("0"));
}
