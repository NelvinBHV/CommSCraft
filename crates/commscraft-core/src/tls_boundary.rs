//! Offline TLS boundary policy for SIP registration.
//! RFC 3261 (June 2002) §26.3.1 and RFC 5922 (June 2010) §§7.1-7.3.
//! This checks a *report* from a hypothetical certificate verifier, NOT X.509.
//! No socket, TLS handshake, real peer authentication, or sign-in is provided.

use crate::discovery::normalize_domain;
use crate::offline_register_branch::FixtureRegisterBranch;

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

/// Offline-only binding of a fixture TLS connection generation to an
/// independently pinned Kerberos SPN and REGISTER transaction identifier.
/// This is deliberately not constructible from a live transport, and never
/// grants permission to authenticate, send SIP, or use credentials.
/// RFC 5922 (June 2010) §§7.1–7.3; RFC 3261 (June 2002) §17.1.3;
/// MS-SIPAE §3.2.5.2 (server MIC validation remains independent).
#[derive(Debug)]
pub struct OfflineRegisterBinding {
    boundary: OfflineTlsBoundary,
    expected_spn: String,
    register_transaction: FixtureRegisterBranch,
}

impl OfflineRegisterBinding {
    pub fn from_fixture(
        report: FixturePeerReport,
        expected_spn: &str,
        register_transaction: &str,
        now_ms: u64,
    ) -> Result<Self, BoundaryError> {
        let boundary = OfflineTlsBoundary::from_fixture(report, now_ms)?;
        let spn_domain = expected_spn
            .strip_prefix("sip/")
            .ok_or(BoundaryError::NameMismatch)?;
        let normalized_spn =
            normalize_domain(spn_domain).map_err(|_| BoundaryError::InvalidDomain)?;
        if normalized_spn != boundary.sip_domain {
            return Err(BoundaryError::NameMismatch);
        }
        // A SIP Via branch is a local transaction correlation token, not
        // authentication. Require the RFC 3261 magic cookie and safe ASCII.
        let register_transaction = FixtureRegisterBranch::new_fixture(register_transaction)
            .map_err(|_| BoundaryError::InvalidDomain)?;
        Ok(Self {
            boundary,
            expected_spn: format!("sip/{normalized_spn}"),
            register_transaction,
        })
    }

    pub fn observe_fixture_server_mic(
        &mut self,
        generation: u64,
        spn: &str,
        transaction: &str,
        mic_checked: bool,
        sequence: u64,
        now_ms: u64,
    ) -> Result<(), BoundaryError> {
        if spn != self.expected_spn || transaction != self.register_transaction.as_str() {
            return Err(BoundaryError::PeerMismatch);
        }
        self.boundary
            .observe_offline_mic(generation, mic_checked, sequence, now_ms)
    }

    pub fn disconnect(&mut self) {
        self.boundary.disconnect();
    }

    /// Always false: synthetic evidence cannot authorize sign-in.
    pub fn authenticated_sign_in(&self) -> bool {
        false
    }
}

/// A non-authorizing model of the evidence that a future transport/SA
/// integration would need to keep bound to one TLS session. All inputs are
/// synthetic fixtures: a fingerprint is NOT proof of PKIX verification.
/// RFC 5922 (June 2010) §§7.1–7.3; RFC 3261 (June 2002) §17.1.3;
/// MS-SIPAE §3.2.5.2. No live constructor or authentication capability.
#[derive(Debug)]
pub struct FixtureSessionCorrelation {
    binding: OfflineRegisterBinding,
    leaf_sha256: [u8; 32],
    connection_generation: u64,
    closed: bool,
}

impl FixtureSessionCorrelation {
    pub fn from_fixture(
        report: FixturePeerReport,
        leaf_sha256: [u8; 32],
        spn: &str,
        transaction: &str,
        now_ms: u64,
    ) -> Result<Self, BoundaryError> {
        if leaf_sha256 == [0; 32] {
            return Err(BoundaryError::CertificateUnverified);
        }
        let connection_generation = report.generation;
        let binding = OfflineRegisterBinding::from_fixture(report, spn, transaction, now_ms)?;
        Ok(Self {
            binding,
            leaf_sha256,
            connection_generation,
            closed: false,
        })
    }

    /// Every observation must name the SAME fixture certificate, connection,
    /// SPN and transaction; a mismatch never advances the replay counter.
    pub fn observe_fixture(
        &mut self,
        leaf_sha256: &[u8; 32],
        generation: u64,
        spn: &str,
        transaction: &str,
        mic: (bool, u64, u64),
    ) -> Result<(), BoundaryError> {
        if self.closed || generation != self.connection_generation {
            return Err(BoundaryError::ConnectionChanged);
        }
        if leaf_sha256 != &self.leaf_sha256 {
            return Err(BoundaryError::PeerMismatch);
        }
        self.binding
            .observe_fixture_server_mic(generation, spn, transaction, mic.0, mic.1, mic.2)
    }

    pub fn disconnect(&mut self) {
        self.closed = true;
        self.binding.disconnect();
    }

    /// Deliberately constant: no fixture can authorize real sign-in.
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
    fn fixture_session_rejects_certificate_spn_transaction_replay_and_disconnect() {
        let fingerprint = [0x42; 32];
        let branch = "z9hG4bK-fixture-session";
        let mut session = FixtureSessionCorrelation::from_fixture(
            report(),
            fingerprint,
            "sip/example.com",
            branch,
            0,
        )
        .unwrap();
        assert!(!session.authenticated_sign_in());
        assert_eq!(
            session.observe_fixture(&[0x43; 32], 1, "sip/example.com", branch, (true, 1, 1)),
            Err(BoundaryError::PeerMismatch)
        );
        assert_eq!(
            session.observe_fixture(&fingerprint, 2, "sip/example.com", branch, (true, 1, 1)),
            Err(BoundaryError::ConnectionChanged)
        );
        assert_eq!(
            session.observe_fixture(&fingerprint, 1, "sip/evil.example", branch, (true, 1, 1)),
            Err(BoundaryError::PeerMismatch)
        );
        assert_eq!(
            session.observe_fixture(
                &fingerprint,
                1,
                "sip/example.com",
                "z9hG4bK-other",
                (true, 1, 1)
            ),
            Err(BoundaryError::PeerMismatch)
        );
        assert_eq!(
            session.observe_fixture(&fingerprint, 1, "sip/example.com", branch, (false, 1, 1)),
            Err(BoundaryError::MicUnverified)
        );
        assert_eq!(
            session.observe_fixture(&fingerprint, 1, "sip/example.com", branch, (true, 1, 1)),
            Ok(())
        );
        assert_eq!(
            session.observe_fixture(&fingerprint, 1, "sip/example.com", branch, (true, 1, 2)),
            Err(BoundaryError::Replay)
        );
        session.disconnect();
        assert_eq!(
            session.observe_fixture(&fingerprint, 1, "sip/example.com", branch, (true, 2, 3)),
            Err(BoundaryError::ConnectionChanged)
        );
        assert!(!session.authenticated_sign_in());
    }

    #[test]
    fn fixture_session_rejects_missing_certificate_and_expiry() {
        assert_eq!(
            FixtureSessionCorrelation::from_fixture(
                report(),
                [0; 32],
                "sip/example.com",
                "z9hG4bK-fixture",
                0
            )
            .err(),
            Some(BoundaryError::CertificateUnverified)
        );
        assert!(FixtureSessionCorrelation::from_fixture(
            report(),
            [1; 32],
            "sip/example.com",
            "z9hG4bK-fixture",
            1000
        )
        .is_err());
    }

    #[test]
    fn fixture_binding_rejects_spn_transaction_generation_and_unverified_mic() {
        let branch = "z9hG4bK-offline-registration-1";
        let mut binding =
            OfflineRegisterBinding::from_fixture(report(), "sip/example.com", branch, 0).unwrap();
        assert!(!binding.authenticated_sign_in());
        assert_eq!(
            binding.observe_fixture_server_mic(1, "sip/attacker.example", branch, true, 1, 1),
            Err(BoundaryError::PeerMismatch)
        );
        assert_eq!(
            binding.observe_fixture_server_mic(1, "sip/example.com", "z9hG4bK-other", true, 1, 1),
            Err(BoundaryError::PeerMismatch)
        );
        assert_eq!(
            binding.observe_fixture_server_mic(2, "sip/example.com", branch, true, 1, 1),
            Err(BoundaryError::ConnectionChanged)
        );
        assert_eq!(
            binding.observe_fixture_server_mic(1, "sip/example.com", branch, false, 1, 1),
            Err(BoundaryError::MicUnverified)
        );
        assert_eq!(
            binding.observe_fixture_server_mic(1, "sip/example.com", branch, true, 1, 1),
            Ok(())
        );
        assert_eq!(
            binding.observe_fixture_server_mic(1, "sip/example.com", branch, true, 1, 2),
            Err(BoundaryError::Replay)
        );
        binding.disconnect();
        assert_eq!(
            binding.observe_fixture_server_mic(1, "sip/example.com", branch, true, 2, 3),
            Err(BoundaryError::ConnectionChanged)
        );
        assert!(!binding.authenticated_sign_in());
    }

    #[test]
    fn fixture_binding_rejects_untrusted_identity_and_bad_branch() {
        for spn in [
            "sip/attacker.example",
            "HTTP/example.com",
            "sip/example.com.evil",
        ] {
            assert!(
                OfflineRegisterBinding::from_fixture(report(), spn, "z9hG4bK-safe", 0).is_err()
            );
        }
        for branch in ["", "no-cookie", "z9hG4bK", "z9hG4bK\r\nInjected"] {
            assert!(
                OfflineRegisterBinding::from_fixture(report(), "sip/example.com", branch, 0)
                    .is_err()
            );
        }
        let mut invalid = report();
        invalid.chain_validated = false;
        assert!(OfflineRegisterBinding::from_fixture(
            invalid,
            "sip/example.com",
            "z9hG4bK-safe",
            0
        )
        .is_err());
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
