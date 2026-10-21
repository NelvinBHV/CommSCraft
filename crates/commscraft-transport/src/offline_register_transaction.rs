//! Non-authorizing, fixture-only REGISTER response lifecycle.
//! RFC 3261 (June 2002) §§8.1.1.7, 17.1.3; RFC 5922 (June 2010) §§7.1–7.3.
//! Synthetic MIC flags and correlation fields are never sign-in evidence.

use crate::offline_register_branch::FixtureRegisterBranch;
use crate::offline_session_identity::{FixtureSessionHandle, OfflineTransportSessionOwner};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterLifecycleError {
    InvalidSession,
    InvalidTransaction,
    InvalidDeadline,
    Expired,
    Unverified,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Pending,
    Closed,
}

/// One transaction bound to an opaque transport-owned fixture handle.
/// No clone or public field can reset its terminal state.
#[derive(Debug)]
pub struct OfflineRegisterTransaction {
    session: FixtureSessionHandle,
    branch: FixtureRegisterBranch,
    cseq: u32,
    deadline_ms: u64,
    phase: Phase,
}

impl OfflineRegisterTransaction {
    pub fn new_fixture(
        owner: &OfflineTransportSessionOwner,
        session: FixtureSessionHandle,
        branch: &str,
        cseq: u32,
        now_ms: u64,
        deadline_ms: u64,
    ) -> Result<Self, RegisterLifecycleError> {
        if owner.check_fixture(&session).is_err() {
            return Err(RegisterLifecycleError::InvalidSession);
        }
        let branch = FixtureRegisterBranch::new_fixture(branch)
            .map_err(|_| RegisterLifecycleError::InvalidTransaction)?;
        if cseq == 0 || cseq > 2_147_483_647 {
            return Err(RegisterLifecycleError::InvalidTransaction);
        }
        if deadline_ms <= now_ms {
            return Err(RegisterLifecycleError::InvalidDeadline);
        }
        Ok(Self {
            session,
            branch,
            cseq,
            deadline_ms,
            phase: Phase::Pending,
        })
    }

    /// Any failed final response observation poisons the fixture transaction.
    /// A successful synthetic 200 OK consumes it, but never signs in.
    pub fn observe_final_fixture(
        &mut self,
        owner: &OfflineTransportSessionOwner,
        branch: &str,
        cseq: u32,
        status: u16,
        mic_verified_fixture: bool,
        now_ms: u64,
    ) -> Result<(), RegisterLifecycleError> {
        if self.phase == Phase::Closed {
            return Err(RegisterLifecycleError::Closed);
        }
        self.phase = Phase::Closed;
        if owner.check_fixture(&self.session).is_err() {
            return Err(RegisterLifecycleError::InvalidSession);
        }
        if now_ms >= self.deadline_ms {
            return Err(RegisterLifecycleError::Expired);
        }
        if branch != self.branch.as_str() || cseq != self.cseq {
            return Err(RegisterLifecycleError::InvalidTransaction);
        }
        if status != 200 || !mic_verified_fixture {
            return Err(RegisterLifecycleError::Unverified);
        }
        Ok(())
    }

    pub fn timeout_fixture(&mut self) {
        self.phase = Phase::Closed;
    }

    pub fn disconnect_fixture(&mut self) {
        self.phase = Phase::Closed;
    }

    pub fn is_closed(&self) -> bool {
        self.phase == Phase::Closed
    }

    pub fn authenticated_sign_in(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (OfflineTransportSessionOwner, OfflineRegisterTransaction) {
        let mut owner = OfflineTransportSessionOwner::new_fixture(31).unwrap();
        let handle = owner.connect_fixture().unwrap();
        let tx = OfflineRegisterTransaction::new_fixture(
            &owner,
            handle,
            "z9hG4bK-register-31",
            42,
            10,
            100,
        )
        .unwrap();
        (owner, tx)
    }

    #[test]
    fn valid_fixture_is_one_shot_and_never_authenticates() {
        let (owner, mut tx) = setup();
        assert_eq!(
            tx.observe_final_fixture(&owner, "z9hG4bK-register-31", 42, 200, true, 11),
            Ok(())
        );
        assert!(tx.is_closed());
        assert_eq!(
            tx.observe_final_fixture(&owner, "z9hG4bK-register-31", 42, 200, true, 12),
            Err(RegisterLifecycleError::Closed)
        );
        assert!(!tx.authenticated_sign_in());
    }

    #[test]
    fn mismatches_and_unverified_responses_are_terminal() {
        for (branch, cseq, status, mic, expected) in [
            (
                "z9hG4bK-other",
                42,
                200,
                true,
                RegisterLifecycleError::InvalidTransaction,
            ),
            (
                "z9hG4bK-register-31",
                43,
                200,
                true,
                RegisterLifecycleError::InvalidTransaction,
            ),
            (
                "z9hG4bK-register-31",
                42,
                401,
                true,
                RegisterLifecycleError::Unverified,
            ),
            (
                "z9hG4bK-register-31",
                42,
                200,
                false,
                RegisterLifecycleError::Unverified,
            ),
        ] {
            let (owner, mut tx) = setup();
            assert_eq!(
                tx.observe_final_fixture(&owner, branch, cseq, status, mic, 11),
                Err(expected)
            );
            assert_eq!(
                tx.observe_final_fixture(&owner, "z9hG4bK-register-31", 42, 200, true, 12),
                Err(RegisterLifecycleError::Closed)
            );
        }
    }

    #[test]
    fn expiry_disconnect_and_reconnect_revoke_transaction() {
        let (owner, mut tx) = setup();
        assert_eq!(
            tx.observe_final_fixture(&owner, "z9hG4bK-register-31", 42, 200, true, 100),
            Err(RegisterLifecycleError::Expired)
        );
        assert!(tx.is_closed());

        let (mut owner, mut tx) = setup();
        owner.connect_fixture().unwrap();
        assert_eq!(
            tx.observe_final_fixture(&owner, "z9hG4bK-register-31", 42, 200, true, 11),
            Err(RegisterLifecycleError::InvalidSession)
        );
        let (mut owner, mut tx) = setup();
        owner.disconnect();
        assert_eq!(
            tx.observe_final_fixture(&owner, "z9hG4bK-register-31", 42, 200, true, 11),
            Err(RegisterLifecycleError::InvalidSession)
        );
    }

    #[test]
    fn timeout_and_explicit_disconnect_are_terminal() {
        let (owner, mut tx) = setup();
        tx.timeout_fixture();
        assert_eq!(
            tx.observe_final_fixture(&owner, "z9hG4bK-register-31", 42, 200, true, 11),
            Err(RegisterLifecycleError::Closed)
        );
        let (owner, mut tx) = setup();
        tx.disconnect_fixture();
        assert_eq!(
            tx.observe_final_fixture(&owner, "z9hG4bK-register-31", 42, 200, true, 11),
            Err(RegisterLifecycleError::Closed)
        );
    }

    #[test]
    fn rejects_invalid_cseq_branch_and_deadline() {
        for cseq in [0, 2_147_483_648] {
            let mut owner = OfflineTransportSessionOwner::new_fixture(32).unwrap();
            let handle = owner.connect_fixture().unwrap();
            assert_eq!(
                OfflineRegisterTransaction::new_fixture(
                    &owner,
                    handle,
                    "z9hG4bK-test",
                    cseq,
                    10,
                    100
                )
                .err(),
                Some(RegisterLifecycleError::InvalidTransaction)
            );
        }
        let mut owner = OfflineTransportSessionOwner::new_fixture(33).unwrap();
        let handle = owner.connect_fixture().unwrap();
        assert_eq!(
            OfflineRegisterTransaction::new_fixture(&owner, handle, "z9hG4bK-test", 1, 10, 10)
                .err(),
            Some(RegisterLifecycleError::InvalidDeadline)
        );
        let handle = owner.connect_fixture().unwrap();
        assert_eq!(
            OfflineRegisterTransaction::new_fixture(&owner, handle, "bad-branch", 1, 10, 100).err(),
            Some(RegisterLifecycleError::InvalidTransaction)
        );
    }
}
