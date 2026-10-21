//! Offline model of a transport-owned TLS connection identity.
//! This module never opens a socket or authorizes SIP or Kerberos authentication.
//! RFC 5922 (June 2010) §§7.1-7.3: SIP identity must be verified on the
//! actual TLS peer; a caller-provided digest or generation is not evidence.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE_OWNER_INSTANCE: AtomicU64 = AtomicU64::new(1);

/// Opaque, non-cloneable handle issued by a single offline transport owner.
/// It is not a live TLS session identifier or a cryptographic capability.
#[derive(Debug, PartialEq, Eq)]
pub struct FixtureSessionHandle {
    owner: u64,
    instance: u64,
    generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionIdentityError {
    InvalidOwner,
    StaleSession,
    Disconnected,
    Exhausted,
}

/// A local fixture owner controls handle issuance and revocation. The
/// generation is never accepted from an external observation.
#[derive(Debug)]
pub struct OfflineTransportSessionOwner {
    owner: u64,
    instance: u64,
    generation: u64,
    connected: bool,
}

impl OfflineTransportSessionOwner {
    /// The owner identifier is synthetic and cannot prove a live connection.
    pub fn new_fixture(owner: u64) -> Result<Self, SessionIdentityError> {
        if owner == 0 {
            return Err(SessionIdentityError::InvalidOwner);
        }
        let instance = NEXT_FIXTURE_OWNER_INSTANCE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| SessionIdentityError::Exhausted)?;
        Ok(Self {
            owner,
            instance,
            generation: 0,
            connected: false,
        })
    }

    /// Simulate a new connection. Previous handles become invalid.
    pub fn connect_fixture(&mut self) -> Result<FixtureSessionHandle, SessionIdentityError> {
        self.connected = false;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(SessionIdentityError::Exhausted)?;
        self.connected = true;
        Ok(FixtureSessionHandle {
            owner: self.owner,
            instance: self.instance,
            generation: self.generation,
        })
    }

    /// Check only the local lifecycle; this does not verify TLS or X.509.
    pub fn check_fixture(&self, handle: &FixtureSessionHandle) -> Result<(), SessionIdentityError> {
        if !self.connected {
            return Err(SessionIdentityError::Disconnected);
        }
        if handle.owner != self.owner
            || handle.instance != self.instance
            || handle.generation != self.generation
        {
            return Err(SessionIdentityError::StaleSession);
        }
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    pub fn authenticated_sign_in(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_cross_owner_and_reconnect_stale_handles() {
        let mut first = OfflineTransportSessionOwner::new_fixture(1).unwrap();
        let mut second = OfflineTransportSessionOwner::new_fixture(2).unwrap();
        let old = first.connect_fixture().unwrap();
        let foreign = second.connect_fixture().unwrap();
        assert_eq!(first.check_fixture(&old), Ok(()));
        assert_eq!(
            first.check_fixture(&foreign),
            Err(SessionIdentityError::StaleSession)
        );
        let current = first.connect_fixture().unwrap();
        assert_eq!(
            first.check_fixture(&old),
            Err(SessionIdentityError::StaleSession)
        );
        assert_eq!(first.check_fixture(&current), Ok(()));
        first.disconnect();
        assert_eq!(
            first.check_fixture(&current),
            Err(SessionIdentityError::Disconnected)
        );
        assert!(!first.authenticated_sign_in());
    }

    #[test]
    fn rejects_zero_owner_and_disconnected_session() {
        assert_eq!(
            OfflineTransportSessionOwner::new_fixture(0).err(),
            Some(SessionIdentityError::InvalidOwner)
        );
        let owner = OfflineTransportSessionOwner::new_fixture(1).unwrap();
        let handle = FixtureSessionHandle {
            owner: 1,
            instance: owner.instance,
            generation: 0,
        };
        assert_eq!(
            owner.check_fixture(&handle),
            Err(SessionIdentityError::Disconnected)
        );
    }
    #[test]
    fn rejects_distinct_instances_with_identical_fixture_owner_ids() {
        let mut first = OfflineTransportSessionOwner::new_fixture(99).unwrap();
        let mut second = OfflineTransportSessionOwner::new_fixture(99).unwrap();
        let first_handle = first.connect_fixture().unwrap();
        let second_handle = second.connect_fixture().unwrap();
        assert_eq!(
            first.check_fixture(&second_handle),
            Err(SessionIdentityError::StaleSession)
        );
        assert_eq!(
            second.check_fixture(&first_handle),
            Err(SessionIdentityError::StaleSession)
        );
    }

    #[test]
    fn generation_exhaustion_revokes_existing_handle_and_never_wraps() {
        let mut owner = OfflineTransportSessionOwner::new_fixture(100).unwrap();
        owner.generation = u64::MAX - 1;
        let last = owner.connect_fixture().unwrap();
        assert_eq!(owner.check_fixture(&last), Ok(()));
        assert_eq!(
            owner.connect_fixture(),
            Err(SessionIdentityError::Exhausted)
        );
        assert_eq!(
            owner.check_fixture(&last),
            Err(SessionIdentityError::Disconnected)
        );
        assert_eq!(
            owner.connect_fixture(),
            Err(SessionIdentityError::Exhausted)
        );
    }
}
