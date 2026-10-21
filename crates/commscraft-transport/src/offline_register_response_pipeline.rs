//! Offline bounded SIP response -> REGISTER transaction correlation -> fixture MIC gate.
//! RFC 3261 (June 2002) §§7.3, 8.1.3, 17.1.3, 20.16;
//! RFC 5922 (June 2010) §§7.1–7.3; MS-SIPAE §3.2.5.2.
//! No real TLS peer, server MIC, or authenticated sign-in is established.

use crate::offline_register_transaction::{OfflineRegisterTransaction, RegisterLifecycleError};
use crate::offline_session_identity::{FixtureSessionHandle, OfflineTransportSessionOwner};
use commscraft_core::security::ServerMicVerifier;
use commscraft_core::session::{GateEvent, RegisterVerificationGate};
use commscraft_core::sip::{parse_frame, SipError, SipMessage, StartLine};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegratedRegisterError {
    Frame(SipError),
    IncompleteFrame,
    TrailingBytes,
    InvalidResponse,
    Lifecycle(RegisterLifecycleError),
    SecurityGateRejected,
}

#[derive(Debug)]
pub struct OfflineRegisterResponsePipeline {
    transaction: OfflineRegisterTransaction,
}

fn unique_header<'a>(
    message: &'a SipMessage,
    key: &str,
) -> Result<&'a str, IntegratedRegisterError> {
    let mut found = None;
    for (name, value) in &message.headers {
        if name.eq_ignore_ascii_case(key) && found.replace(value.as_str()).is_some() {
            return Err(IntegratedRegisterError::InvalidResponse);
        }
    }
    found.ok_or(IntegratedRegisterError::InvalidResponse)
}

fn response_identity(message: &SipMessage) -> Result<(&str, u32), IntegratedRegisterError> {
    let via = unique_header(message, "Via")?;
    // Restricted fixture policy: one TLS Via, no comma list, no ambiguous branch.
    if via.len() > 512 || via.contains(',') || via.bytes().any(|b| b < 32 || b == 127) {
        return Err(IntegratedRegisterError::InvalidResponse);
    }
    let mut params = via.split(';');
    let sent_by = params
        .next()
        .ok_or(IntegratedRegisterError::InvalidResponse)?;
    if !sent_by.starts_with("SIP/2.0/TLS ") || sent_by.len() <= 12 {
        return Err(IntegratedRegisterError::InvalidResponse);
    }
    let mut branch = None;
    for param in params {
        let (key, value) = param
            .split_once('=')
            .ok_or(IntegratedRegisterError::InvalidResponse)?;
        if key.trim().eq_ignore_ascii_case("branch") && branch.replace(value.trim()).is_some() {
            return Err(IntegratedRegisterError::InvalidResponse);
        }
    }
    let branch = branch.ok_or(IntegratedRegisterError::InvalidResponse)?;
    let cseq = unique_header(message, "CSeq")?;
    if cseq.len() > 64 || !cseq.is_ascii() {
        return Err(IntegratedRegisterError::InvalidResponse);
    }
    let mut parts = cseq.split_ascii_whitespace();
    let (Some(number), Some("REGISTER"), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(IntegratedRegisterError::InvalidResponse);
    };
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return Err(IntegratedRegisterError::InvalidResponse);
    }
    let sequence = number
        .parse::<u32>()
        .map_err(|_| IntegratedRegisterError::InvalidResponse)?;
    if sequence == 0 || sequence > 2_147_483_647 {
        return Err(IntegratedRegisterError::InvalidResponse);
    }
    Ok((branch, sequence))
}

impl OfflineRegisterResponsePipeline {
    pub fn new_fixture(
        owner: &OfflineTransportSessionOwner,
        session: FixtureSessionHandle,
        branch: &str,
        cseq: u32,
        now_ms: u64,
        deadline_ms: u64,
    ) -> Result<Self, IntegratedRegisterError> {
        let transaction = OfflineRegisterTransaction::new_fixture(
            owner,
            session,
            branch,
            cseq,
            now_ms,
            deadline_ms,
        )
        .map_err(IntegratedRegisterError::Lifecycle)?;
        Ok(Self { transaction })
    }

    /// Every observation is terminal, including invalid framing and metadata.
    /// This deliberately models a single final response, not challenge retries.
    pub fn observe_final_frame_fixture(
        &mut self,
        owner: &OfflineTransportSessionOwner,
        wire: &[u8],
        mic_verified_fixture: bool,
        now_ms: u64,
    ) -> Result<(), IntegratedRegisterError> {
        // Poison first so malformed responses cannot be followed by a good 200.
        // The transaction's own one-shot gate remains authoritative.
        if self.transaction.is_closed() {
            return Err(IntegratedRegisterError::Lifecycle(
                RegisterLifecycleError::Closed,
            ));
        }
        let parsed = parse_frame(wire).map_err(IntegratedRegisterError::Frame);
        let (message, consumed) = match parsed {
            Ok(Some(value)) => value,
            Ok(None) => {
                self.transaction.disconnect_fixture();
                return Err(IntegratedRegisterError::IncompleteFrame);
            }
            Err(error) => {
                self.transaction.disconnect_fixture();
                return Err(error);
            }
        };
        if consumed != wire.len() {
            self.transaction.disconnect_fixture();
            return Err(IntegratedRegisterError::TrailingBytes);
        }
        let status = match &message.start_line {
            StartLine::Response { code, .. } if (200..=699).contains(code) => *code,
            _ => {
                self.transaction.disconnect_fixture();
                return Err(IntegratedRegisterError::InvalidResponse);
            }
        };
        let (branch, cseq) = match response_identity(&message) {
            Ok(value) => value,
            Err(error) => {
                self.transaction.disconnect_fixture();
                return Err(error);
            }
        };
        self.transaction
            .observe_final_fixture(owner, branch, cseq, status, mic_verified_fixture, now_ms)
            .map_err(IntegratedRegisterError::Lifecycle)
    }

    /// Composes the transport-owned fixture transaction with the existing
    /// MS-SIPAE MIC, replay, and REGISTER verification gate. A successful
    /// result is only an offline MIC-checked observation, never a sign-in.
    ///
    /// The one-shot transport gate runs before the cryptographic verifier:
    /// invalid branch, CSeq, session or deadline never reaches the MIC gate.
    /// On any failure both gates become unusable for subsequent responses.
    pub fn observe_verified_final_frame_fixture<V: ServerMicVerifier>(
        &mut self,
        owner: &OfflineTransportSessionOwner,
        wire: &[u8],
        gate: &mut RegisterVerificationGate,
        verifier: &V,
        now_ms: u64,
    ) -> Result<GateEvent, IntegratedRegisterError> {
        // Do not interpret the synthetic flag as verification. This call
        // only reserves/consumes the correlated fixture transaction.
        if let Err(error) = self.observe_final_frame_fixture(owner, wire, true, now_ms) {
            gate.disconnect();
            return Err(error);
        }
        // The exact same complete frame has already passed strict framing.
        let response = match parse_frame(wire) {
            Ok(Some((message, consumed))) if consumed == wire.len() => message,
            _ => {
                gate.disconnect();
                return Err(IntegratedRegisterError::InvalidResponse);
            }
        };
        match gate.observe(&response, verifier, now_ms) {
            Ok(event @ GateEvent::MicChecked(_)) => Ok(event),
            _ => {
                gate.disconnect();
                Err(IntegratedRegisterError::SecurityGateRejected)
            }
        }
    }

    pub fn timeout_fixture(&mut self) {
        self.transaction.timeout_fixture();
    }

    pub fn disconnect_fixture(&mut self) {
        self.transaction.disconnect_fixture();
    }

    pub fn authenticated_sign_in(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (
        OfflineTransportSessionOwner,
        OfflineRegisterResponsePipeline,
    ) {
        let mut owner = OfflineTransportSessionOwner::new_fixture(51).unwrap();
        let handle = owner.connect_fixture().unwrap();
        let pipeline = OfflineRegisterResponsePipeline::new_fixture(
            &owner,
            handle,
            "z9hG4bK-pipeline",
            17,
            0,
            100,
        )
        .unwrap();
        (owner, pipeline)
    }

    fn wire(via: &str, cseq: &str, status: u16) -> Vec<u8> {
        format!("SIP/2.0 {status} Test\r\nVia: {via}\r\nCSeq: {cseq}\r\nContent-Length: 0\r\n\r\n")
            .into_bytes()
    }

    fn good() -> Vec<u8> {
        wire(
            "SIP/2.0/TLS client.invalid;branch=z9hG4bK-pipeline",
            "17 REGISTER",
            200,
        )
    }

    #[test]
    fn parsed_correlated_fixture_is_one_shot_not_sign_in() {
        let (owner, mut pipeline) = setup();
        assert_eq!(
            pipeline.observe_final_frame_fixture(&owner, &good(), true, 1),
            Ok(())
        );
        assert_eq!(
            pipeline.observe_final_frame_fixture(&owner, &good(), true, 2),
            Err(IntegratedRegisterError::Lifecycle(
                RegisterLifecycleError::Closed
            ))
        );
        assert!(!pipeline.authenticated_sign_in());
    }

    #[test]
    fn wrong_branch_cseq_status_and_mic_fail_closed() {
        for (wire, mic) in [
            (
                wire(
                    "SIP/2.0/TLS client.invalid;branch=z9hG4bK-other",
                    "17 REGISTER",
                    200,
                ),
                true,
            ),
            (
                wire(
                    "SIP/2.0/TLS client.invalid;branch=z9hG4bK-pipeline",
                    "18 REGISTER",
                    200,
                ),
                true,
            ),
            (
                wire(
                    "SIP/2.0/TLS client.invalid;branch=z9hG4bK-pipeline",
                    "17 REGISTER",
                    401,
                ),
                true,
            ),
            (good(), false),
        ] {
            let (owner, mut pipeline) = setup();
            assert!(pipeline
                .observe_final_frame_fixture(&owner, &wire, mic, 1)
                .is_err());
            assert_eq!(
                pipeline.observe_final_frame_fixture(&owner, &good(), true, 2),
                Err(IntegratedRegisterError::Lifecycle(
                    RegisterLifecycleError::Closed
                ))
            );
        }
    }

    #[test]
    fn malformed_duplicate_incomplete_and_concatenated_frames_are_terminal() {
        let mut duplicate = good();
        let pos = duplicate.windows(6).position(|v| v == b"CSeq: ").unwrap();
        duplicate.splice(pos..pos, b"CSeq: 17 REGISTER\r\n".iter().copied());
        let mut trailing = good();
        trailing.extend_from_slice(&good());
        for bytes in [
            duplicate,
            trailing,
            b"SIP/2.0 200 OK\r\nVia: x".to_vec(),
            b"SIP/2.0 200 OK\r\nContent-Length: 999999999\r\n\r\n".to_vec(),
            wire(
                "SIP/2.0/TLS client.invalid;branch=z9hG4bK-pipeline;branch=z9hG4bK-pipeline",
                "17 REGISTER",
                200,
            ),
            wire(
                "SIP/2.0/TLS client.invalid;branch=z9hG4bK-pipeline",
                "17 INVITE",
                200,
            ),
        ] {
            let (owner, mut pipeline) = setup();
            assert!(pipeline
                .observe_final_frame_fixture(&owner, &bytes, true, 1)
                .is_err());
            assert_eq!(
                pipeline.observe_final_frame_fixture(&owner, &good(), true, 2),
                Err(IntegratedRegisterError::Lifecycle(
                    RegisterLifecycleError::Closed
                ))
            );
        }
    }

    struct RejectMic;

    impl ServerMicVerifier for RejectMic {
        fn verify(&self, _message: &[u8], _token: &[u8]) -> bool {
            false
        }
    }

    fn security_gate() -> RegisterVerificationGate {
        use commscraft_core::challenge::{ChallengeOffer, ChallengeSource, Scheme};
        use commscraft_core::registration::{Endpoint, RegisterFlow};
        use commscraft_core::security::SecurityAssociation;

        let endpoint = Endpoint {
            aor: "sip:alice@example.com".into(),
            sent_by: "client.invalid".into(),
            contact_uri: "sip:alice@client.invalid;transport=tls".into(),
            instance_uuid: "b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97".into(),
            epid: "492a7ce35f".into(),
            from_tag: "2d0be03279".into(),
            call_id: "bridge-call@example.com".into(),
        };
        let flow = RegisterFlow::new(endpoint, 17, "z9hG4bK-pipeline", 3600, 0).unwrap();
        let association = SecurityAssociation::begin(
            &ChallengeOffer {
                scheme: Scheme::TlsDsk,
                source: ChallengeSource::Registrar,
                realm: Some("SIP Communications Service".into()),
                targetname: Some("server.example.com".into()),
                opaque: Some("opaque123".into()),
                version: Some(4),
                sts_uri: None,
                gssapi_data: None,
            },
            8_000_000,
        )
        .unwrap();
        RegisterVerificationGate::new(flow, association)
    }

    #[test]
    fn fixture_flag_cannot_bypass_real_mic_gate() {
        use commscraft_core::session::GateState;
        let (owner, mut pipeline) = setup();
        let mut gate = security_gate();
        assert_eq!(
            pipeline.observe_verified_final_frame_fixture(&owner, &good(), &mut gate, &RejectMic, 1),
            Err(IntegratedRegisterError::SecurityGateRejected)
        );
        assert_eq!(gate.state(), GateState::Disconnected);
        assert_eq!(gate.highest_verified_server_sequence(), None);
        assert_eq!(
            pipeline.observe_verified_final_frame_fixture(&owner, &good(), &mut gate, &RejectMic, 2),
            Err(IntegratedRegisterError::Lifecycle(RegisterLifecycleError::Closed))
        );
        assert!(!pipeline.authenticated_sign_in());
    }

    #[test]
    fn mismatched_transport_response_never_advances_mic_gate() {
        use commscraft_core::session::GateState;
        let (owner, mut pipeline) = setup();
        let mut gate = security_gate();
        let wrong = wire(
            "SIP/2.0/TLS client.invalid;branch=z9hG4bK-other",
            "17 REGISTER",
            200,
        );
        assert!(pipeline
            .observe_verified_final_frame_fixture(&owner, &wrong, &mut gate, &RejectMic, 1)
            .is_err());
        assert_eq!(gate.state(), GateState::Disconnected);
        assert_eq!(gate.highest_verified_server_sequence(), None);
    }

    #[test]
    fn timeout_and_reconnect_reject_late_success() {
        let (owner, mut pipeline) = setup();
        assert!(pipeline
            .observe_final_frame_fixture(&owner, &good(), true, 100)
            .is_err());
        let (mut owner, mut pipeline) = setup();
        owner.connect_fixture().unwrap();
        assert!(pipeline
            .observe_final_frame_fixture(&owner, &good(), true, 1)
            .is_err());
        let (owner, mut pipeline) = setup();
        pipeline.timeout_fixture();
        assert!(pipeline
            .observe_final_frame_fixture(&owner, &good(), true, 1)
            .is_err());
    }
}
