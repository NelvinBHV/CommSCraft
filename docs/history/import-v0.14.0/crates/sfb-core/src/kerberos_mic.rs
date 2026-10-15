//! Kerberos per-message MIC bridge for Microsoft SIPAE security associations.
//!
//! References: MS-SIPAE 3.2.4.1 / 3.2.5.2 and Kerberos example 4.2;
//! RFC 2743 §2.3 (GSS_GetMIC / GSS_VerifyMIC); Windows SSPI
//! MakeSignature / VerifySignature. No credentials or transport here.
//!
//! The bridge is bound to a *completed* Kerberos GSS context and a single
//! MS-SIPAE SA key (scheme, realm, target and UAS/proxy source). It NEVER
//! creates or completes a GSS context, verifies the SIP transaction, or
//! authorizes actual sign-in. `SecurityAssociation` still enforces the signed
//! server fields and replay window. Using a fake backend is for tests ONLY.

use crate::security::{ClientMicSigner, SaKey, SecurityError, ServerMicVerifier};
use std::cell::RefCell;
use std::fmt;

pub const MAX_MIC_INPUT: usize = 64 * 1024;
pub const MAX_MIC_TOKEN: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerberosMicError {
    IncompleteContext,
    OriginMismatch,
    InvalidInput,
    NativeFailure,
}

/// GSS/SSPI per-message service (NOT a handshake). Both methods require the
/// same trusted completed context; the Windows implementation uses SSPI.
/// Production callers must not substitute a self-generated test backend.
pub trait KerberosMicBackend {
    fn complete(&self) -> bool;
    fn target_spn(&self) -> Option<&str>;
    fn get_mic(&mut self, signed_bytes: &[u8]) -> Result<Vec<u8>, KerberosMicError>;
    fn verify_mic(&mut self, signed_bytes: &[u8], token: &[u8]) -> Result<(), KerberosMicError>;
}

impl<T: KerberosMicBackend + ?Sized> KerberosMicBackend for &mut T {
    fn complete(&self) -> bool { (**self).complete() }
    fn target_spn(&self) -> Option<&str> { (**self).target_spn() }
    fn get_mic(&mut self, bytes: &[u8]) -> Result<Vec<u8>, KerberosMicError> {
        (**self).get_mic(bytes)
    }
    fn verify_mic(&mut self, bytes: &[u8], token: &[u8]) -> Result<(), KerberosMicError> {
        (**self).verify_mic(bytes, token)
    }
}

/// Non-cloneable, non-Sync shared view of ONE native context, used as both
/// `ServerMicVerifier` and `ClientMicSigner` with serialized mutable access.
/// The original `SecurityAssociation` is still responsible for peer identity,
/// expiration, and replay checks. Construction never changes the SA state.
pub struct KerberosMicBridge<B: KerberosMicBackend> {
    backend: RefCell<B>,
    bound_key: SaKey,
}
impl<B: KerberosMicBackend> fmt::Debug for KerberosMicBridge<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KerberosMicBridge")
            .field("context_and_identity", &"[REDACTED]").finish()
    }
}
impl<B: KerberosMicBackend> KerberosMicBridge<B> {
    pub fn bind(backend: B, sa: &SaKey) -> Result<Self, KerberosMicError> {
        if !backend.complete() { return Err(KerberosMicError::IncompleteContext); }
        if sa.scheme != "Kerberos" || sa.realm.is_empty() ||
            !sa.realm.is_ascii() ||
            sa.realm.bytes().any(|b| b == b'<' || b == b'>' || b.is_ascii_control()) ||
            backend.target_spn() != Some(sa.targetname.as_str()) {
            return Err(KerberosMicError::OriginMismatch);
        }
        Ok(Self { backend: RefCell::new(backend), bound_key: sa.clone() })
    }
    fn ready(&self) -> bool {
        self.backend.try_borrow().map(|b| b.complete() &&
            b.target_spn() == Some(self.bound_key.targetname.as_str())).unwrap_or(false)
    }
    /// Check that MS-SIPAE input fields #1/#4/#5 match this bound context.
    /// `SecurityAssociation` additionally checks authorization source and the
    /// full SA identity. Without this, a caller could reuse the bridge on an
    /// unrelated SA whose MIC buffer contains different peer identifiers.
    fn accepts_mic_input(&self, message: &[u8]) -> bool {
        let mut rest = message;
        for index in 0..5 {
            if !rest.starts_with(b"<") { return false; }
            let Some(end) = rest[1..].iter().position(|&b| b == b'>') else { return false; };
            let value = &rest[1..1 + end];
            if value.contains(&b'<') { return false; }
            let expected = match index {
                0 => Some(self.bound_key.scheme.as_bytes()),
                3 => Some(self.bound_key.realm.as_bytes()),
                4 => Some(self.bound_key.targetname.as_bytes()),
                _ => None,
            };
            if expected.is_some_and(|e| e != value) { return false; }
            rest = &rest[2 + end..];
        }
        !rest.is_empty()
    }
}
impl<B: KerberosMicBackend> ServerMicVerifier for KerberosMicBridge<B> {
    fn verify(&self, message: &[u8], token: &[u8]) -> bool {
        if message.is_empty() || message.len() > MAX_MIC_INPUT ||
            token.is_empty() || token.len() > MAX_MIC_TOKEN ||
            !self.ready() || !self.accepts_mic_input(message) { return false; }
        let Ok(mut backend) = self.backend.try_borrow_mut() else { return false; };
        backend.verify_mic(message, token).is_ok()
    }
}
impl<B: KerberosMicBackend> ClientMicSigner for KerberosMicBridge<B> {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SecurityError> {
        if message.is_empty() || message.len() > MAX_MIC_INPUT ||
            !self.ready() || !self.accepts_mic_input(message) {
            return Err(SecurityError::SigningFailed);
        }
        let mut backend = self.backend.try_borrow_mut()
            .map_err(|_| SecurityError::SigningFailed)?;
        let result = backend.get_mic(message).map_err(|_| SecurityError::SigningFailed)?;
        if result.is_empty() || result.len() > MAX_MIC_TOKEN {
            return Err(SecurityError::SigningFailed);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::AuthSource;

    // Synthetic backend deliberately NOT cryptographically secure and only
    // compiled under #[cfg(test)]. No production test-key fallback exists.
    struct FakeBackend { ready: bool, target: String, calls: usize }
    impl KerberosMicBackend for FakeBackend {
        fn complete(&self) -> bool { self.ready }
        fn target_spn(&self) -> Option<&str> { Some(&self.target) }
        fn get_mic(&mut self, data: &[u8]) -> Result<Vec<u8>, KerberosMicError> {
            self.calls += 1;
            Ok(vec![data.iter().fold(0u8, |sum, b| sum.wrapping_add(*b))])
        }
        fn verify_mic(&mut self, data: &[u8], token: &[u8]) -> Result<(), KerberosMicError> {
            self.calls += 1;
            let expected = [data.iter().fold(0u8, |sum, b| sum.wrapping_add(*b))];
            if token == expected.as_slice() { Ok(()) } else { Err(KerberosMicError::NativeFailure) }
        }
    }
    fn key() -> SaKey { SaKey { scheme: "Kerberos".into(), realm: "SIP Communications Service".into(),
        targetname: "sip/server.example.com".into(), source: AuthSource::Registrar } }
    fn fake() -> FakeBackend { FakeBackend {ready: true, target: "sip/server.example.com".into(), calls: 0} }

    #[test]
    fn refuses_incomplete_and_incorrect_origin() {
        let mut incomplete = fake(); incomplete.ready = false;
        assert!(matches!(KerberosMicBridge::bind(incomplete, &key()), Err(KerberosMicError::IncompleteContext)));
        let mut altered = key(); altered.targetname = "sip/other.example.com".into();
        assert!(matches!(KerberosMicBridge::bind(fake(), &altered), Err(KerberosMicError::OriginMismatch)));
        altered = key(); altered.scheme = "TLS-DSK".into();
        assert!(matches!(KerberosMicBridge::bind(fake(), &altered), Err(KerberosMicError::OriginMismatch)));
    }
    #[test]
    fn accepts_only_valid_mic_and_bounded_material() {
        let adapter = KerberosMicBridge::bind(fake(), &key()).unwrap();
        let signed = b"<Kerberos><abcdef01><1><SIP Communications Service><sip/server.example.com><callid><1><REGISTER>";
        let mic = adapter.sign(signed).unwrap();
        assert!(adapter.verify(signed, &mic));
        let changed = b"<Kerberos><abcdef01><1><SIP Communications Service><sip/server.example.com><altered><1><REGISTER>";
        assert!(!adapter.verify(changed, &mic));
        assert!(!adapter.verify(signed, &[]));
        assert!(!adapter.verify(signed, &vec![0; MAX_MIC_TOKEN + 1]));
        let wrong = b"<Kerberos><abcdef01><1><SIP Communications Service><sip/other.example.com><callid>";
        assert!(!adapter.verify(wrong, &mic));
        assert_eq!(adapter.sign(wrong), Err(SecurityError::SigningFailed));
        assert_eq!(adapter.sign(&vec![b'A'; MAX_MIC_INPUT + 1]), Err(SecurityError::SigningFailed));
        assert!(!format!("{adapter:?}").contains("sip/server.example.com"));
    }
}
