//! Offline model of a transport-owned TLS connection identity.
//! This module never opens a socket or authorizes SIP or Kerberos authentication.
//! RFC 5922 (June 2010) §§7.1-7.3: SIP identity must be verified on the
//! actual TLS peer; a caller-provided digest or generation is not evidence.

/// Opaque, non-cloneable handle issued by a single offline transport owner.
/// It is not a live TLS session identifier or a cryptographic capability.
#[derive(Debug, PartialEq, Eq)]
pub struct FixtureSessionHandle {
    owner: u64,
    generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionIdentityError {
    InvalidOwner,
    StaleSession,
    Disconnected,
}

/// A local fixture owner controls handle issuance and revocation. The
/// generation is never accepted from an external observation.
#[derive(Debug)]
pub struct OfflineTransportSessionOwner {
    owner: u64,
    generation: u64,
    connected: bool,
}

impl OfflineTransportSessionOwner {
    /// The owner identifier is synthetic and cannot prove a live connection.
    pub fn new_fixture(owner: u64) -> Result<Self, SessionIdentityError> {
        if owner == 0 {
            return Err(SessionIdentityError::InvalidOwner);
        }
        Ok(Self {
            owner,
            generation: 0,
            connected: false,
        })
    }

    /// Simulate a new connection. Previous handles become invalid.
    pub fn connect_fixture(&mut self) -> FixtureSessionHandle {
        self.generation = self.generation.saturating_add(1);
        self.connected = true;
        FixtureSessionHandle {
            owner: self.owner,
            generation: self.generation,
        }
    }

    /// Check only the local lifecycle; this does not verify TLS or X.509.
    pub fn check_fixture(&self, handle: &FixtureSessionHandle) -> Result<(), SessionIdentityError> {
        if !self.connected {
            return Err(SessionIdentityError::Disconnected);
        }
        if handle.owner != self.owner || handle.generation != self.generation {
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
        let old = first.connect_fixture();
        let foreign = second.connect_fixture();
        assert_eq!(first.check_fixture(&old), Ok(()));
        assert_eq!(
            first.check_fixture(&foreign),
            Err(SessionIdentityError::StaleSession)
        );
        let current = first.connect_fixture();
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
            generation: 0,
        };
        assert_eq!(
            owner.check_fixture(&handle),
            Err(SessionIdentityError::Disconnected)
        );
    }
}
