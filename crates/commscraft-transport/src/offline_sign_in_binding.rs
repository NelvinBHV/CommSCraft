//! Fixture-only bridge from transport-owned connection to sign-in lifecycle.
//! RFC 3261 (June 2002) §17.1.2; RFC 5922 (June 2010) §7.3.
//! Neither synthetic handles nor state transitions authorize real sign-in.

use crate::offline_register_response_pipeline::{
    IntegratedRegisterError, OfflineRegisterResponsePipeline,
};
use crate::offline_session_identity::{FixtureSessionHandle, OfflineTransportSessionOwner};
use commscraft_core::offline_sign_in_coordinator::{
    OfflineSignInCoordinator, OfflineSignInError, OfflineSignInEvent, OfflineSignInPhase,
};
use commscraft_core::security::ServerMicVerifier;
use commscraft_core::session::RegisterVerificationGate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundSignInError {
    InvalidSession,
    Lifecycle(OfflineSignInError),
    Register(IntegratedRegisterError),
}

/// Non-cloneable owner of the original fixture handle. No caller-supplied
/// generation is accepted for an observation, and reconnect revokes the handle.
#[derive(Debug)]
pub struct TransportBoundOfflineSignIn {
    session: FixtureSessionHandle,
    coordinator: OfflineSignInCoordinator,
}

impl TransportBoundOfflineSignIn {
    pub fn new_fixture(
        owner: &OfflineTransportSessionOwner,
        session: FixtureSessionHandle,
        now_ms: u64,
        deadline_ms: u64,
    ) -> Result<Self, BoundSignInError> {
        owner
            .check_fixture(&session)
            .map_err(|_| BoundSignInError::InvalidSession)?;
        // This local coordinator epoch is not a TLS identity. The transport
        // handle is separately checked on every observation.
        let coordinator = OfflineSignInCoordinator::new_fixture(1, now_ms, deadline_ms)
            .map_err(BoundSignInError::Lifecycle)?;
        Ok(Self {
            session,
            coordinator,
        })
    }

    pub fn phase(&self) -> OfflineSignInPhase {
        self.coordinator.phase()
    }

    pub fn authenticated_sign_in(&self) -> bool {
        false
    }

    pub fn disconnect_fixture(&mut self) {
        self.coordinator.close();
    }

    fn check_owner(
        &mut self,
        owner: &OfflineTransportSessionOwner,
    ) -> Result<(), BoundSignInError> {
        if self.coordinator.phase() == OfflineSignInPhase::Closed {
            return Err(BoundSignInError::Lifecycle(OfflineSignInError::Closed));
        }
        if owner.check_fixture(&self.session).is_err() {
            self.coordinator.close();
            return Err(BoundSignInError::InvalidSession);
        }
        Ok(())
    }

    pub fn observe_fixture(
        &mut self,
        owner: &OfflineTransportSessionOwner,
        event: OfflineSignInEvent,
        now_ms: u64,
    ) -> Result<OfflineSignInPhase, BoundSignInError> {
        self.check_owner(owner)?;
        self.coordinator
            .observe_fixture(1, event, now_ms)
            .map_err(BoundSignInError::Lifecycle)
    }

    /// A correlated final REGISTER response and the verified MIC must both
    /// succeed on the same transport owner before advancing this attempt.
    /// This fixture API cannot create a real authenticated session.
    pub fn observe_verified_register_fixture<V: ServerMicVerifier>(
        &mut self,
        owner: &OfflineTransportSessionOwner,
        pipeline: &mut OfflineRegisterResponsePipeline,
        wire: &[u8],
        gate: &mut RegisterVerificationGate,
        verifier: &V,
        now_ms: u64,
    ) -> Result<OfflineSignInPhase, BoundSignInError> {
        self.check_owner(owner)?;
        if self.coordinator.phase() != OfflineSignInPhase::ServerMicVerification {
            self.coordinator.close();
            pipeline.disconnect_fixture();
            gate.disconnect();
            return Err(BoundSignInError::Lifecycle(
                OfflineSignInError::InvalidTransition,
            ));
        }
        if let Err(error) =
            pipeline.observe_verified_final_frame_fixture(owner, wire, gate, verifier, now_ms)
        {
            self.coordinator.close();
            return Err(BoundSignInError::Register(error));
        }
        self.coordinator
            .observe_mic_gate_fixture(1, gate, now_ms)
            .map_err(BoundSignInError::Lifecycle)
    }

    pub fn observe_mic_gate_fixture(
        &mut self,
        owner: &OfflineTransportSessionOwner,
        gate: &RegisterVerificationGate,
        now_ms: u64,
    ) -> Result<OfflineSignInPhase, BoundSignInError> {
        self.check_owner(owner)?;
        self.coordinator
            .observe_mic_gate_fixture(1, gate, now_ms)
            .map_err(BoundSignInError::Lifecycle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RejectMic;

    impl ServerMicVerifier for RejectMic {
        fn verify(&self, _message: &[u8], _token: &[u8]) -> bool {
            false
        }
    }

    #[test]
    fn premature_register_response_closes_attempt_and_pipeline() {
        let mut owner = OfflineTransportSessionOwner::new_fixture(93).unwrap();
        let pipeline_session = owner.connect_fixture().unwrap();
        let mut pipeline = OfflineRegisterResponsePipeline::new_fixture(
            &owner,
            pipeline_session,
            "z9hG4bK-pipeline",
            17,
            0,
            100,
        )
        .unwrap();
        let attempt_session = owner.issue_fixture_handle().unwrap();
        let mut attempt =
            TransportBoundOfflineSignIn::new_fixture(&owner, attempt_session, 0, 100).unwrap();
        let mut gate = register_gate();
        let wire = b"SIP/2.0 200 OK\\r\\nVia: SIP/2.0/TLS client.invalid;branch=z9hG4bK-pipeline\\r\\nCSeq: 17 REGISTER\\r\\nContent-Length: 0\\r\\n\\r\\n";
        assert_eq!(
            attempt.observe_verified_register_fixture(
                &owner,
                &mut pipeline,
                wire,
                &mut gate,
                &RejectMic,
                1,
            ),
            Err(BoundSignInError::Lifecycle(
                OfflineSignInError::InvalidTransition
            ))
        );
        assert_eq!(attempt.phase(), OfflineSignInPhase::Closed);
        assert_eq!(
            gate.state(),
            commscraft_core::session::GateState::Disconnected
        );
        assert!(!attempt.authenticated_sign_in());
    }

    fn register_gate() -> RegisterVerificationGate {
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
            call_id: "bound-register@example.com".into(),
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
    fn connection_replacement_revokes_attempt() {
        let mut owner = OfflineTransportSessionOwner::new_fixture(90).unwrap();
        let handle = owner.connect_fixture().unwrap();
        let mut attempt = TransportBoundOfflineSignIn::new_fixture(&owner, handle, 0, 100).unwrap();
        assert_eq!(
            attempt.observe_fixture(&owner, OfflineSignInEvent::DiscoveryResolved, 1),
            Ok(OfflineSignInPhase::TlsPeerVerification)
        );
        owner.connect_fixture().unwrap();
        assert_eq!(
            attempt.observe_fixture(&owner, OfflineSignInEvent::TlsPeerVerified, 2),
            Err(BoundSignInError::InvalidSession)
        );
        assert_eq!(attempt.phase(), OfflineSignInPhase::Closed);
        assert!(!attempt.authenticated_sign_in());
    }

    #[test]
    fn foreign_owner_disconnect_and_timeout_are_terminal() {
        let mut owner = OfflineTransportSessionOwner::new_fixture(91).unwrap();
        let session = owner.connect_fixture().unwrap();
        let mut foreign = OfflineTransportSessionOwner::new_fixture(92).unwrap();
        foreign.connect_fixture().unwrap();
        let mut attempt =
            TransportBoundOfflineSignIn::new_fixture(&owner, session, 0, 100).unwrap();
        assert_eq!(
            attempt.observe_fixture(&foreign, OfflineSignInEvent::DiscoveryResolved, 1),
            Err(BoundSignInError::InvalidSession)
        );
        assert_eq!(attempt.phase(), OfflineSignInPhase::Closed);
        let session = owner.connect_fixture().unwrap();
        let mut expired =
            TransportBoundOfflineSignIn::new_fixture(&owner, session, 0, 100).unwrap();
        assert_eq!(
            expired.observe_fixture(&owner, OfflineSignInEvent::DiscoveryResolved, 100),
            Err(BoundSignInError::Lifecycle(OfflineSignInError::Expired))
        );
        assert_eq!(expired.phase(), OfflineSignInPhase::Closed);
        let session = owner.connect_fixture().unwrap();
        let mut disconnected =
            TransportBoundOfflineSignIn::new_fixture(&owner, session, 0, 100).unwrap();
        owner.disconnect();
        assert_eq!(
            disconnected.observe_fixture(&owner, OfflineSignInEvent::DiscoveryResolved, 1),
            Err(BoundSignInError::InvalidSession)
        );
        assert_eq!(disconnected.phase(), OfflineSignInPhase::Closed);
    }
}
