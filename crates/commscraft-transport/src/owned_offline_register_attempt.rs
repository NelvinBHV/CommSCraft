//! One owner for an entire offline REGISTER attempt, without socket or credentials.
//! RFC 3261 (June 2002) §§10.2 and 17.1.3; RFC 5922 (June 2010) §7.3;
//! MS-SIPAE §3.2.5.2. These fixtures never authenticate a real client.

use crate::offline_register_response_pipeline::{
    IntegratedRegisterError, OfflineRegisterResponsePipeline,
};
use crate::offline_session_identity::{OfflineTransportSessionOwner, SessionIdentityError};
use crate::offline_sign_in_binding::{BoundSignInError, TransportBoundOfflineSignIn};
use commscraft_core::offline_sign_in_coordinator::{OfflineSignInEvent, OfflineSignInPhase};
use commscraft_core::security::ServerMicVerifier;
use commscraft_core::session::{GateState, RegisterVerificationGate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedRegisterError {
    Owner(SessionIdentityError),
    Pipeline(IntegratedRegisterError),
    SignIn(BoundSignInError),
}

/// The lifecycle, transport handle, one-shot REGISTER transaction and MIC gate
/// cannot be independently supplied to `observe_final_fixture`. Deliberately
/// does not expose the internal owner, gate, or transaction to callers.
pub struct OwnedOfflineRegisterAttempt {
    owner: OfflineTransportSessionOwner,
    sign_in: TransportBoundOfflineSignIn,
    pipeline: OfflineRegisterResponsePipeline,
    gate: RegisterVerificationGate,
}

impl OwnedOfflineRegisterAttempt {
    pub fn new_fixture(
        owner_id: u64,
        branch: &str,
        cseq: u32,
        gate: RegisterVerificationGate,
        now_ms: u64,
        deadline_ms: u64,
    ) -> Result<Self, OwnedRegisterError> {
        let mut owner = OfflineTransportSessionOwner::new_fixture(owner_id)
            .map_err(OwnedRegisterError::Owner)?;
        let transaction_handle = owner.connect_fixture().map_err(OwnedRegisterError::Owner)?;
        let sign_in_handle = owner
            .issue_fixture_handle()
            .map_err(OwnedRegisterError::Owner)?;
        let pipeline = OfflineRegisterResponsePipeline::new_fixture(
            &owner,
            transaction_handle,
            branch,
            cseq,
            now_ms,
            deadline_ms,
        )
        .map_err(OwnedRegisterError::Pipeline)?;
        let sign_in =
            TransportBoundOfflineSignIn::new_fixture(&owner, sign_in_handle, now_ms, deadline_ms)
                .map_err(OwnedRegisterError::SignIn)?;
        Ok(Self {
            owner,
            sign_in,
            pipeline,
            gate,
        })
    }

    /// A fixture-only phase driver, not proof of DNS, TLS, SSPI or SIP writes.
    pub fn prepare_fixture(&mut self, now_ms: u64) -> Result<(), OwnedRegisterError> {
        for event in [
            OfflineSignInEvent::DiscoveryResolved,
            OfflineSignInEvent::TlsPeerVerified,
            OfflineSignInEvent::ChallengeMatched,
            OfflineSignInEvent::SecurityContextEstablished,
            OfflineSignInEvent::SignedRegisterPrepared,
        ] {
            if let Err(error) = self.sign_in.observe_fixture(&self.owner, event, now_ms) {
                self.disconnect_fixture();
                return Err(OwnedRegisterError::SignIn(error));
            }
        }
        Ok(())
    }

    /// One complete, correlated final frame: transport check → transaction →
    /// cryptographic verifier → MIC gate → offline sign-in state.
    pub fn observe_final_fixture<V: ServerMicVerifier>(
        &mut self,
        wire: &[u8],
        verifier: &V,
        now_ms: u64,
    ) -> Result<OfflineSignInPhase, OwnedRegisterError> {
        match self.sign_in.observe_verified_register_fixture(
            &self.owner,
            &mut self.pipeline,
            wire,
            &mut self.gate,
            verifier,
            now_ms,
        ) {
            Ok(phase) => Ok(phase),
            Err(error) => {
                self.disconnect_fixture();
                Err(OwnedRegisterError::SignIn(error))
            }
        }
    }

    pub fn phase(&self) -> OfflineSignInPhase {
        self.sign_in.phase()
    }

    pub fn mic_gate_state(&self) -> GateState {
        self.gate.state()
    }

    pub fn authenticated_sign_in(&self) -> bool {
        false
    }

    /// Simulates a new TLS connection; previous attempt components are revoked.
    pub fn reconnect_fixture(&mut self) -> Result<(), OwnedRegisterError> {
        self.disconnect_fixture();
        self.owner
            .connect_fixture()
            .map_err(OwnedRegisterError::Owner)?;
        Ok(())
    }

    pub fn disconnect_fixture(&mut self) {
        self.owner.disconnect();
        self.pipeline.disconnect_fixture();
        self.gate.disconnect();
        self.sign_in.disconnect_fixture();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commscraft_core::challenge::{ChallengeOffer, ChallengeSource, Scheme};
    use commscraft_core::registration::{Endpoint, RegisterFlow};
    use commscraft_core::security::{SecurityAssociation, ServerMicVerifier};

    struct RejectMic;
    impl ServerMicVerifier for RejectMic {
        fn verify(&self, _: &[u8], _: &[u8]) -> bool {
            false
        }
    }

    fn gate() -> RegisterVerificationGate {
        let endpoint = Endpoint {
            aor: "sip:alice@example.com".into(),
            sent_by: "client.invalid".into(),
            contact_uri: "sip:alice@client.invalid;transport=tls".into(),
            instance_uuid: "b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97".into(),
            epid: "492a7ce35f".into(),
            from_tag: "2d0be03279".into(),
            call_id: "owned-register@example.com".into(),
        };
        let flow = RegisterFlow::new(endpoint, 17, "z9hG4bK-owned-register", 3600, 0).unwrap();
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

    fn attempt() -> OwnedOfflineRegisterAttempt {
        OwnedOfflineRegisterAttempt::new_fixture(500, "z9hG4bK-owned-register", 17, gate(), 0, 100).unwrap()
    }

    #[test]
    fn complete_fixture_handoff_rejects_unverified_mic_and_replay() {
        let mut attempt = attempt();
        attempt.prepare_fixture(1).unwrap();
        assert_eq!(attempt.phase(), OfflineSignInPhase::ServerMicVerification);
        let wire = b"SIP/2.0 200 OK\r\nVia: SIP/2.0/TLS client.invalid;branch=z9hG4bK-owned-register\r\nCSeq: 17 REGISTER\r\nContent-Length: 0\r\n\r\n";
        assert!(attempt.observe_final_fixture(wire, &RejectMic, 2).is_err());
        assert_eq!(attempt.phase(), OfflineSignInPhase::Closed);
        assert_eq!(attempt.mic_gate_state(), GateState::Disconnected);
        assert!(attempt.observe_final_fixture(wire, &RejectMic, 3).is_err());
        assert!(!attempt.authenticated_sign_in());
    }

    #[test]
    fn reconnect_and_invalid_phase_are_terminal() {
        let mut disconnected = attempt();
        disconnected.reconnect_fixture().unwrap();
        assert_eq!(disconnected.phase(), OfflineSignInPhase::Closed);
        assert_eq!(disconnected.mic_gate_state(), GateState::Disconnected);
        assert!(disconnected.prepare_fixture(1).is_err());
        let mut premature = attempt();
        assert!(premature.observe_final_fixture(b"", &RejectMic, 1).is_err());
        assert_eq!(premature.phase(), OfflineSignInPhase::Closed);
        assert_eq!(premature.mic_gate_state(), GateState::Disconnected);
    }

    #[test]
    fn invalid_transaction_is_rejected_before_state_machine() {
        assert!(matches!(
            OwnedOfflineRegisterAttempt::new_fixture(501, "not-a-branch", 17, gate(), 0, 100),
            Err(OwnedRegisterError::Pipeline(_))
        ));
    }
}
