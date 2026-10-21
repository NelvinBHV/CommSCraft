//! Fixture-only transport -> REGISTER -> security handoff contract.
//! RFC 3261 (June 2002) §17.1.3; RFC 5922 (June 2010) §§7.1-7.3;
//! MS-SIPAE §3.2.5.2. This is not live protocol interoperability evidence.

use crate::offline_session_identity::{FixtureSessionHandle, OfflineTransportSessionOwner};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoffError {
    SessionInvalid,
    TransactionMismatch,
    SecurityUnverified,
    AlreadyConsumed,
}

/// A transaction-scoped offline handoff; the transport owns the session
/// handle, and the security adapter must independently verify real MICs in
/// any future production implementation.
#[derive(Debug)]
pub struct FixtureRegisterSecurityHandoff {
    session: FixtureSessionHandle,
    branch: String,
    consumed: bool,
}

impl FixtureRegisterSecurityHandoff {
    /// Synthetic branch is caller-supplied and never trusted as authentication.
    pub fn new_fixture(
        transport: &OfflineTransportSessionOwner,
        session: FixtureSessionHandle,
        branch: &str,
    ) -> Result<Self, HandoffError> {
        if transport.check_fixture(&session).is_err() {
            return Err(HandoffError::SessionInvalid);
        }
        if !branch.starts_with("z9hG4bK")
            || branch.len() <= 7
            || branch.len() > 128
            || !branch
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.' || b == b'_')
        {
            return Err(HandoffError::TransactionMismatch);
        }
        Ok(Self {
            session,
            branch: branch.to_owned(),
            consumed: false,
        })
    }

    /// A boolean fixture flag only; it is NOT proof of an actual Kerberos MIC.
    /// Consume at most once, and only if all checks succeed.
    pub fn accept_fixture(
        &mut self,
        transport: &OfflineTransportSessionOwner,
        branch: &str,
        mic_verified_fixture: bool,
    ) -> Result<(), HandoffError> {
        if self.consumed {
            return Err(HandoffError::AlreadyConsumed);
        }
        if transport.check_fixture(&self.session).is_err() {
            return Err(HandoffError::SessionInvalid);
        }
        if branch != self.branch {
            return Err(HandoffError::TransactionMismatch);
        }
        if !mic_verified_fixture {
            return Err(HandoffError::SecurityUnverified);
        }
        self.consumed = true;
        Ok(())
    }

    pub fn authenticated_sign_in(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_security_wrong_transaction_and_replay() {
        let mut owner = OfflineTransportSessionOwner::new_fixture(11).unwrap();
        let session = owner.connect_fixture().unwrap();
        let mut handoff =
            FixtureRegisterSecurityHandoff::new_fixture(&owner, session, "z9hG4bK-transaction")
                .unwrap();
        assert_eq!(
            handoff.accept_fixture(&owner, "z9hG4bK-transaction", false),
            Err(HandoffError::SecurityUnverified)
        );
        assert_eq!(
            handoff.accept_fixture(&owner, "z9hG4bK-other", true),
            Err(HandoffError::TransactionMismatch)
        );
        assert_eq!(
            handoff.accept_fixture(&owner, "z9hG4bK-transaction", true),
            Ok(())
        );
        assert_eq!(
            handoff.accept_fixture(&owner, "z9hG4bK-transaction", true),
            Err(HandoffError::AlreadyConsumed)
        );
        assert!(!handoff.authenticated_sign_in());
    }

    #[test]
    fn rejects_reconnect_disconnect_and_foreign_owner() {
        let mut first = OfflineTransportSessionOwner::new_fixture(12).unwrap();
        let mut second = OfflineTransportSessionOwner::new_fixture(13).unwrap();
        let old = first.connect_fixture().unwrap();
        let mut handoff =
            FixtureRegisterSecurityHandoff::new_fixture(&first, old, "z9hG4bK-first").unwrap();
        second.connect_fixture().unwrap();
        assert_eq!(
            handoff.accept_fixture(&second, "z9hG4bK-first", true),
            Err(HandoffError::SessionInvalid)
        );
        first.connect_fixture().unwrap();
        assert_eq!(
            handoff.accept_fixture(&first, "z9hG4bK-first", true),
            Err(HandoffError::SessionInvalid)
        );
        let current = first.connect_fixture().unwrap();
        let mut next =
            FixtureRegisterSecurityHandoff::new_fixture(&first, current, "z9hG4bK-next").unwrap();
        first.disconnect();
        assert_eq!(
            next.accept_fixture(&first, "z9hG4bK-next", true),
            Err(HandoffError::SessionInvalid)
        );
    }

    #[test]
    fn rejects_invalid_branch_at_creation() {
        let mut owner = OfflineTransportSessionOwner::new_fixture(14).unwrap();
        let session = owner.connect_fixture().unwrap();
        assert_eq!(
            FixtureRegisterSecurityHandoff::new_fixture(&owner, session, "z9hG4bK\r\nInjected")
                .err(),
            Some(HandoffError::TransactionMismatch)
        );
    }
}
