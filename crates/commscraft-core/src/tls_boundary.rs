//! Offline TLS boundary policy for SIP registration.
//! RFC 3261 (June 2002) §26.3.1 and RFC 5922 (June 2010) §§7.1-7.3.
//! This checks a *report* from a hypothetical certificate verifier, NOT X.509.
//! No socket, TLS handshake, real peer authentication, or sign-in is provided.

use crate::discovery::normalize_domain;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryError {
    InvalidDomain,
    InvalidGeneration,
    CertificateUnverified,
    NameMismatch,
    PeerMismatch,
    ConnectionChanged,
    Expired,
    Replay,
    MicUnverified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixturePeerReport {
    pub expected_sip_domain: String,
    pub authenticated_sip_domain: String,
    pub selected_tls_host: String,
    pub reported_tls_host: String,
    pub chain_validated: bool,
    pub eku_approved: bool,
    pub generation: u64,
    pub expires_at_ms: u64,
}

#[derive(Debug)]
pub struct OfflineTlsBoundary {
    sip_domain: String,
    tls_host: String,
    generation: u64,
    expires_at_ms: u64,
    highest_mic_sequence: Option<u64>,
    disconnected: bool,
}

impl OfflineTlsBoundary {
    /// Strict ASCII DNS subset. A hostname report is no substitute for RFC 5922
    /// SIP-domain authorization; both identities are checked independently.
    pub fn from_fixture(report: FixturePeerReport, now_ms: u64) -> Result<Self, BoundaryError> {
        let normalized = |s: &str| normalize_domain(s).map_err(|_| BoundaryError::InvalidDomain);
        let expected = normalized(&report.expected_sip_domain)?;
        let identity = normalized(&report.authenticated_sip_domain)?;
        let selected = normalized(&report.selected_tls_host)?;
        let reported = normalized(&report.reported_tls_host)?;
        if report.generation == 0 {
            return Err(BoundaryError::InvalidGeneration);
        }
        if !report.chain_validated || !report.eku_approved {
            return Err(BoundaryError::CertificateUnverified);
        }
        if expected != identity {
            return Err(BoundaryError::NameMismatch);
        }
        if selected != reported {
            return Err(BoundaryError::PeerMismatch);
        }
        if now_ms >= report.expires_at_ms {
            return Err(BoundaryError::Expired);
        }
        Ok(Self {
            sip_domain: expected,
            tls_host: selected,
            generation: report.generation,
            expires_at_ms: report.expires_at_ms,
            highest_mic_sequence: None,
            disconnected: false,
        })
    }

    /// A MIC already checked by the independent security association is required.
    /// No result of this function can authorize an actual sign-in.
    pub fn observe_offline_mic(
        &mut self,
        generation: u64,
        mic_checked: bool,
        sequence: u64,
        now_ms: u64,
    ) -> Result<(), BoundaryError> {
        if self.disconnected || generation != self.generation {
            return Err(BoundaryError::ConnectionChanged);
        }
        if now_ms >= self.expires_at_ms {
            return Err(BoundaryError::Expired);
        }
        if !mic_checked {
            return Err(BoundaryError::MicUnverified);
        }
        if self
            .highest_mic_sequence
            .is_some_and(|last| sequence <= last)
        {
            return Err(BoundaryError::Replay);
        }
        self.highest_mic_sequence = Some(sequence);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.disconnected = true;
    }
    pub fn highest_mic_sequence(&self) -> Option<u64> {
        self.highest_mic_sequence
    }
    pub fn diagnostic_domains(&self) -> (&str, &str) {
        (&self.sip_domain, &self.tls_host)
    }
    /// Deliberately constant: fixture reports are not real verified channels.
    pub fn authenticated_sign_in(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report() -> FixturePeerReport {
        FixturePeerReport {
            expected_sip_domain: "example.com".into(),
            authenticated_sip_domain: "EXAMPLE.COM".into(),
            selected_tls_host: "registrar.example.com".into(),
            reported_tls_host: "registrar.example.com".into(),
            chain_validated: true,
            eku_approved: true,
            generation: 1,
            expires_at_ms: 1000,
        }
    }
    #[test]
    fn offline_tls_valid_fixture_mic_replay_and_disconnect() {
        let mut gate = OfflineTlsBoundary::from_fixture(report(), 0).unwrap();
        assert_eq!(
            gate.diagnostic_domains(),
            ("example.com", "registrar.example.com")
        );
        assert!(!gate.authenticated_sign_in());
        assert_eq!(gate.observe_offline_mic(1, true, 42, 1), Ok(()));
        assert_eq!(
            gate.observe_offline_mic(1, true, 42, 2),
            Err(BoundaryError::Replay)
        );
        assert_eq!(
            gate.observe_offline_mic(1, true, 41, 3),
            Err(BoundaryError::Replay)
        );
        assert_eq!(gate.highest_mic_sequence(), Some(42));
        assert_eq!(
            gate.observe_offline_mic(1, false, 43, 4),
            Err(BoundaryError::MicUnverified)
        );
        assert_eq!(gate.highest_mic_sequence(), Some(42));
        assert_eq!(
            gate.observe_offline_mic(2, true, 43, 5),
            Err(BoundaryError::ConnectionChanged)
        );
        assert_eq!(
            gate.observe_offline_mic(1, true, 43, 1000),
            Err(BoundaryError::Expired)
        );
        gate.disconnect();
        assert_eq!(
            gate.observe_offline_mic(1, true, 43, 6),
            Err(BoundaryError::ConnectionChanged)
        );
        let mut new_connection = report();
        new_connection.generation = 2;
        let mut fresh = OfflineTlsBoundary::from_fixture(new_connection, 6).unwrap();
        assert_eq!(fresh.highest_mic_sequence(), None);
        assert_eq!(fresh.observe_offline_mic(2, true, 43, 7), Ok(()));
        assert!(!fresh.authenticated_sign_in());
    }
    #[test]
    fn offline_tls_rejects_unverified_chain_eku_hostname_and_peer() {
        let mut r = report();
        r.chain_validated = false;
        assert!(matches!(
            OfflineTlsBoundary::from_fixture(r, 0),
            Err(BoundaryError::CertificateUnverified)
        ));
        let mut r = report();
        r.eku_approved = false;
        assert!(matches!(
            OfflineTlsBoundary::from_fixture(r, 0),
            Err(BoundaryError::CertificateUnverified)
        ));
        let mut r = report();
        r.authenticated_sip_domain = "attacker.example".into();
        assert!(matches!(
            OfflineTlsBoundary::from_fixture(r, 0),
            Err(BoundaryError::NameMismatch)
        ));
        let mut r = report();
        r.reported_tls_host = "attacker.example".into();
        assert!(matches!(
            OfflineTlsBoundary::from_fixture(r, 0),
            Err(BoundaryError::PeerMismatch)
        ));
        let mut r = report();
        r.generation = 0;
        assert!(matches!(
            OfflineTlsBoundary::from_fixture(r, 0),
            Err(BoundaryError::InvalidGeneration)
        ));
        let mut r = report();
        r.selected_tls_host = "bad..example".into();
        assert!(matches!(
            OfflineTlsBoundary::from_fixture(r, 0),
            Err(BoundaryError::InvalidDomain)
        ));
        assert!(matches!(
            OfflineTlsBoundary::from_fixture(report(), 1000),
            Err(BoundaryError::Expired)
        ));
    }
}
