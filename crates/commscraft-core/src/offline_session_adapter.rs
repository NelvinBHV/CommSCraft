//! Non-authorizing, fixture-only transport / SIP transaction / security-association
//! evidence adapter. No live transport adapter, SSPI handles or credentials.
//! RFC 5922 (June 2010) §§7.1–7.3; RFC 3261 (June 2002) §17.1.3;
//! [MS-SIPAE] §3.2.5.2. These are design constraints, not interop evidence.

use crate::tls_boundary::{BoundaryError, FixturePeerReport, FixtureSessionCorrelation};

/// Synthetic evidence captured together for ONE fixture connection.
/// Private fields prevent callers from mutating the connection identity after
/// construction; this is still NOT proof of a live authenticated TLS session.
#[derive(Debug)]
pub struct FixtureTransportEvidence {
    report: FixturePeerReport,
    leaf_sha256: [u8; 32],
}

impl FixtureTransportEvidence {
    pub fn new_fixture(report: FixturePeerReport, leaf_sha256: [u8; 32]) -> Self {
        Self {
            report,
            leaf_sha256,
        }
    }
}

/// A locally selected REGISTER branch and expected SPN. Neither can be
/// derived from a received SIP response or treated as authentication.
#[derive(Debug)]
pub struct FixtureRegisterIntent {
    spn: String,
    branch: String,
}

impl FixtureRegisterIntent {
    pub fn new_fixture(spn: &str, branch: &str) -> Self {
        Self {
            spn: spn.into(),
            branch: branch.into(),
        }
    }
}

/// Synthetic security-association observation; the MIC flag is a TEST VALUE,
/// never evidence of actual Kerberos MIC verification.
#[derive(Debug)]
pub struct FixtureSecurityObservation {
    pub mic_checked: bool,
    pub sequence: u64,
    pub now_ms: u64,
}

#[derive(Debug)]
pub struct OfflineSessionAdapter {
    correlation: FixtureSessionCorrelation,
    generation: u64,
    leaf_sha256: [u8; 32],
    spn: String,
    branch: String,
}

impl OfflineSessionAdapter {
    pub fn bind_fixture(
        transport: FixtureTransportEvidence,
        intent: FixtureRegisterIntent,
        now_ms: u64,
    ) -> Result<Self, BoundaryError> {
        let generation = transport.report.generation;
        let correlation = FixtureSessionCorrelation::from_fixture(
            transport.report,
            transport.leaf_sha256,
            &intent.spn,
            &intent.branch,
            now_ms,
        )?;
        Ok(Self {
            correlation,
            generation,
            leaf_sha256: transport.leaf_sha256,
            spn: intent.spn,
            branch: intent.branch,
        })
    }

    /// Observe only the synthetic security-association event for the pinned
    /// fixture session. The caller cannot supply a replacement peer identity
    /// to this method. A live adapter must be separately security reviewed.
    pub fn observe_fixture_security(
        &mut self,
        observation: FixtureSecurityObservation,
    ) -> Result<(), BoundaryError> {
        self.correlation.observe_fixture(
            &self.leaf_sha256,
            self.generation,
            &self.spn,
            &self.branch,
            (
                observation.mic_checked,
                observation.sequence,
                observation.now_ms,
            ),
        )
    }

    pub fn disconnect(&mut self) {
        self.correlation.disconnect();
    }

    /// No fixture observation can authorize a real sign-in.
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
            authenticated_sip_domain: "example.com".into(),
            selected_tls_host: "edge.example.com".into(),
            reported_tls_host: "edge.example.com".into(),
            chain_validated: true,
            eku_approved: true,
            generation: 1,
            expires_at_ms: 100,
        }
    }

    fn adapter() -> OfflineSessionAdapter {
        OfflineSessionAdapter::bind_fixture(
            FixtureTransportEvidence::new_fixture(report(), [0x11; 32]),
            FixtureRegisterIntent::new_fixture("sip/example.com", "z9hG4bK-local"),
            0,
        )
        .unwrap()
    }

    #[test]
    fn fixture_adapter_rejects_unverified_mic_replay_expiry_and_disconnect() {
        let mut a = adapter();
        assert!(!a.authenticated_sign_in());
        assert_eq!(
            a.observe_fixture_security(FixtureSecurityObservation {
                mic_checked: false,
                sequence: 1,
                now_ms: 1,
            }),
            Err(BoundaryError::MicUnverified)
        );
        assert_eq!(
            a.observe_fixture_security(FixtureSecurityObservation {
                mic_checked: true,
                sequence: 1,
                now_ms: 1,
            }),
            Ok(())
        );
        assert_eq!(
            a.observe_fixture_security(FixtureSecurityObservation {
                mic_checked: true,
                sequence: 1,
                now_ms: 2,
            }),
            Err(BoundaryError::Replay)
        );
        assert_eq!(
            a.observe_fixture_security(FixtureSecurityObservation {
                mic_checked: true,
                sequence: 2,
                now_ms: 100,
            }),
            Err(BoundaryError::Expired)
        );
        a.disconnect();
        assert_eq!(
            a.observe_fixture_security(FixtureSecurityObservation {
                mic_checked: true,
                sequence: 2,
                now_ms: 3,
            }),
            Err(BoundaryError::ConnectionChanged)
        );
        assert!(!a.authenticated_sign_in());
    }

    #[test]
    fn fixture_adapter_rejects_mismatched_identity_and_transaction() {
        let mut bad = report();
        bad.authenticated_sip_domain = "attacker.example".into();
        assert!(OfflineSessionAdapter::bind_fixture(
            FixtureTransportEvidence::new_fixture(bad, [0x11; 32]),
            FixtureRegisterIntent::new_fixture("sip/example.com", "z9hG4bK-local"),
            0,
        )
        .is_err());
        for spn in ["sip/attacker.example", "HTTP/example.com"] {
            assert!(OfflineSessionAdapter::bind_fixture(
                FixtureTransportEvidence::new_fixture(report(), [0x11; 32]),
                FixtureRegisterIntent::new_fixture(spn, "z9hG4bK-local"),
                0,
            )
            .is_err());
        }
        assert!(OfflineSessionAdapter::bind_fixture(
            FixtureTransportEvidence::new_fixture(report(), [0x11; 32]),
            FixtureRegisterIntent::new_fixture("sip/example.com", "z9hG4bK\r\nattack"),
            0,
        )
        .is_err());
        assert!(OfflineSessionAdapter::bind_fixture(
            FixtureTransportEvidence::new_fixture(report(), [0; 32]),
            FixtureRegisterIntent::new_fixture("sip/example.com", "z9hG4bK-local"),
            0,
        )
        .is_err());
    }
}
