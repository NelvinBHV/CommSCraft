//! Offline sign-in lifecycle contract. No network I/O or authentication grants.
//! RFC 3261 (June 2002) §§10.2, 17.1.2; RFC 5922 (June 2010) §7.3;
//! MS-SIPAE §3.2.5.2. Synthetic events are never evidence of real sign-in.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflineSignInPhase {
    Discovery,
    TlsPeerVerification,
    RegisterChallenge,
    SecurityNegotiation,
    SignedRegister,
    ServerMicVerification,
    RegistrationObserved,
    Provisioning,
    OfflineComplete,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflineSignInEvent {
    DiscoveryResolved,
    TlsPeerVerified,
    ChallengeMatched,
    SecurityContextEstablished,
    SignedRegisterPrepared,
    ServerMicChecked,
    RegistrationBindingObserved,
    ProvisioningObserved,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflineSignInError {
    InvalidDeadline,
    InvalidGeneration,
    InvalidTransition,
    Expired,
    Closed,
}

/// One offline attempt tied to one transport generation and deadline.
/// The only success state is `OfflineComplete`, never authenticated sign-in.
#[derive(Debug)]
pub struct OfflineSignInCoordinator {
    generation: u64,
    deadline_ms: u64,
    phase: OfflineSignInPhase,
}

impl OfflineSignInCoordinator {
    pub fn new_fixture(
        generation: u64,
        now_ms: u64,
        deadline_ms: u64,
    ) -> Result<Self, OfflineSignInError> {
        if generation == 0 {
            return Err(OfflineSignInError::InvalidGeneration);
        }
        if deadline_ms <= now_ms {
            return Err(OfflineSignInError::InvalidDeadline);
        }
        Ok(Self {
            generation,
            deadline_ms,
            phase: OfflineSignInPhase::Discovery,
        })
    }

    pub fn phase(&self) -> OfflineSignInPhase {
        self.phase
    }

    pub fn authenticated_sign_in(&self) -> bool {
        false
    }

    pub fn close(&mut self) {
        self.phase = OfflineSignInPhase::Closed;
    }

    /// Any invalid transition, expired deadline or generation change revokes
    /// the entire attempt. A new connection requires a new coordinator.
    pub fn observe_fixture(
        &mut self,
        generation: u64,
        event: OfflineSignInEvent,
        now_ms: u64,
    ) -> Result<OfflineSignInPhase, OfflineSignInError> {
        if self.phase == OfflineSignInPhase::Closed {
            return Err(OfflineSignInError::Closed);
        }
        if event == OfflineSignInEvent::Close {
            self.close();
            return Ok(self.phase);
        }
        if generation != self.generation {
            self.close();
            return Err(OfflineSignInError::InvalidGeneration);
        }
        if now_ms >= self.deadline_ms {
            self.close();
            return Err(OfflineSignInError::Expired);
        }
        let next = match (self.phase, event) {
            (OfflineSignInPhase::Discovery, OfflineSignInEvent::DiscoveryResolved) => {
                OfflineSignInPhase::TlsPeerVerification
            }
            (OfflineSignInPhase::TlsPeerVerification, OfflineSignInEvent::TlsPeerVerified) => {
                OfflineSignInPhase::RegisterChallenge
            }
            (OfflineSignInPhase::RegisterChallenge, OfflineSignInEvent::ChallengeMatched) => {
                OfflineSignInPhase::SecurityNegotiation
            }
            (
                OfflineSignInPhase::SecurityNegotiation,
                OfflineSignInEvent::SecurityContextEstablished,
            ) => OfflineSignInPhase::SignedRegister,
            (OfflineSignInPhase::SignedRegister, OfflineSignInEvent::SignedRegisterPrepared) => {
                OfflineSignInPhase::ServerMicVerification
            }
            (OfflineSignInPhase::ServerMicVerification, OfflineSignInEvent::ServerMicChecked) => {
                OfflineSignInPhase::RegistrationObserved
            }
            (
                OfflineSignInPhase::RegistrationObserved,
                OfflineSignInEvent::RegistrationBindingObserved,
            ) => OfflineSignInPhase::Provisioning,
            (OfflineSignInPhase::Provisioning, OfflineSignInEvent::ProvisioningObserved) => {
                OfflineSignInPhase::OfflineComplete
            }
            _ => {
                self.close();
                return Err(OfflineSignInError::InvalidTransition);
            }
        };
        self.phase = next;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEPS: [OfflineSignInEvent; 8] = [
        OfflineSignInEvent::DiscoveryResolved,
        OfflineSignInEvent::TlsPeerVerified,
        OfflineSignInEvent::ChallengeMatched,
        OfflineSignInEvent::SecurityContextEstablished,
        OfflineSignInEvent::SignedRegisterPrepared,
        OfflineSignInEvent::ServerMicChecked,
        OfflineSignInEvent::RegistrationBindingObserved,
        OfflineSignInEvent::ProvisioningObserved,
    ];

    #[test]
    fn offline_sequence_never_authenticates() {
        let mut coordinator = OfflineSignInCoordinator::new_fixture(1, 0, 100).unwrap();
        for (index, event) in STEPS.iter().enumerate() {
            assert_eq!(
                coordinator.observe_fixture(1, *event, 1),
                Ok(match index {
                    0 => OfflineSignInPhase::TlsPeerVerification,
                    1 => OfflineSignInPhase::RegisterChallenge,
                    2 => OfflineSignInPhase::SecurityNegotiation,
                    3 => OfflineSignInPhase::SignedRegister,
                    4 => OfflineSignInPhase::ServerMicVerification,
                    5 => OfflineSignInPhase::RegistrationObserved,
                    6 => OfflineSignInPhase::Provisioning,
                    _ => OfflineSignInPhase::OfflineComplete,
                })
            );
            assert!(!coordinator.authenticated_sign_in());
        }
        assert_eq!(
            coordinator.observe_fixture(1, STEPS[0], 2),
            Err(OfflineSignInError::InvalidTransition)
        );
        assert_eq!(coordinator.phase(), OfflineSignInPhase::Closed);
    }

    #[test]
    fn skipped_step_closes_attempt() {
        let mut coordinator = OfflineSignInCoordinator::new_fixture(1, 0, 100).unwrap();
        assert_eq!(
            coordinator.observe_fixture(1, OfflineSignInEvent::ServerMicChecked, 1),
            Err(OfflineSignInError::InvalidTransition)
        );
        assert_eq!(
            coordinator.observe_fixture(1, STEPS[0], 2),
            Err(OfflineSignInError::Closed)
        );
    }

    #[test]
    fn reconnect_timeout_and_close_are_terminal() {
        let mut changed = OfflineSignInCoordinator::new_fixture(1, 0, 100).unwrap();
        assert_eq!(
            changed.observe_fixture(2, STEPS[0], 1),
            Err(OfflineSignInError::InvalidGeneration)
        );
        assert_eq!(changed.phase(), OfflineSignInPhase::Closed);
        let mut expired = OfflineSignInCoordinator::new_fixture(1, 0, 100).unwrap();
        assert_eq!(
            expired.observe_fixture(1, STEPS[0], 100),
            Err(OfflineSignInError::Expired)
        );
        let mut closed = OfflineSignInCoordinator::new_fixture(1, 0, 100).unwrap();
        assert_eq!(
            closed.observe_fixture(1, OfflineSignInEvent::Close, 1),
            Ok(OfflineSignInPhase::Closed)
        );
        assert_eq!(
            closed.observe_fixture(1, STEPS[0], 2),
            Err(OfflineSignInError::Closed)
        );
        assert_eq!(
            OfflineSignInCoordinator::new_fixture(0, 0, 100).err(),
            Some(OfflineSignInError::InvalidGeneration)
        );
        assert_eq!(
            OfflineSignInCoordinator::new_fixture(1, 100, 100).err(),
            Some(OfflineSignInError::InvalidDeadline)
        );
    }
}
