//! Fixture-only bridge from transport-owned connection to sign-in lifecycle.
//! RFC 3261 (June 2002) §17.1.2; RFC 5922 (June 2010) §7.3.
//! Neither synthetic handles nor state transitions authorize real sign-in.

use crate::offline_session_identity::{FixtureSessionHandle, OfflineTransportSessionOwner};
use commscraft_core::offline_sign_in_coordinator::{
    OfflineSignInCoordinator, OfflineSignInError, OfflineSignInEvent, OfflineSignInPhase,
};
use commscraft_core::session::RegisterVerificationGate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundSignInError {
    InvalidSession,
    Lifecycle(OfflineSignInError),
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
