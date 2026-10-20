//! Offline Kerberos GSS token handoff for SIP REGISTER authentication.
//!
//! Normative mapping: [MS-SIPAE] 3.2.1 and 3.2.5.1, example 4.2;
//! [MS-SIPREGE] registration; RFC 2743 GSS-API; RFC 3261 10.2 and 17.1.2.
//!
//! This module deliberately supplies **no** Kerberos/SSPI/GSS implementation,
//! no credential acquisition, no live transport, and no proof of authentication.
//! It formats a single v3 GSS initiation token from an EXTERNAL provider, then
//! waits for a separate verified MIC before any caller may claim login.
//! A trusted service principal must be pinned by the application, not adopted
//! from an arbitrary network challenge.

use crate::challenge::{parse_challenges, ChallengeSource, Scheme};
use crate::registration::{RegisterFlow, RegisterState};
use crate::sip::{SipMessage, StartLine};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::fmt;

pub const MAX_GSS_TOKEN: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerberosError {
    InvalidTarget,
    State,
    ResponseMismatch,
    UntrustedChallenge,
    AmbiguousChallenge,
    InvalidChallenge,
    InvalidToken,
    ProviderUnavailable,
    ProviderFailed,
    NotComplete,
    InvalidRetry,
    InvalidRequest,
}
impl fmt::Display for KerberosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for KerberosError {}

/// The GSS implementation MUST acquire credentials from the operating system,
/// request integrity and identify properties and validate its SPN. Providers
/// MUST NOT accept certificate errors, downgrade or log raw tokens.
/// This trait is a narrow integration boundary, NOT an authentication method.
pub trait KerberosProvider {
    /// Starts a NEW GSS context against the administrator-approved SPN.
    fn initiate(&mut self, trusted_spn: &str) -> Result<KerberosStep, KerberosError>;
}

/// Raw GSS output is private to the handoff. `complete` alone is NOT a valid
/// server proof: MS-SIPAE still requires signed server-response verification.
pub struct KerberosStep {
    pub token: Vec<u8>,
    pub complete: bool,
    pub integrity: bool,
}
impl fmt::Debug for KerberosStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KerberosStep")
            .field("token", &"[REDACTED]")
            .field("complete", &self.complete)
            .field("integrity", &self.integrity)
            .finish()
    }
}
impl Drop for KerberosStep {
    fn drop(&mut self) {
        self.token.fill(0);
    }
}

/// Explicit denial by default. A native SSPI/GSSAPI provider remains to be
/// implemented and independently reviewed before live registration is enabled.
pub struct NoKerberosProvider;
impl KerberosProvider for NoKerberosProvider {
    fn initiate(&mut self, _: &str) -> Result<KerberosStep, KerberosError> {
        Err(KerberosError::ProviderUnavailable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerberosPhase {
    AwaitingChallenge,
    /// A first, non-empty GSS token was produced by a provider returning
    /// CONTINUE_NEEDED; this is not authentication and cannot establish an SA.
    AwaitingContextContinuation,
    /// A new offline REGISTER draft has been constructed, but NO authentication
    /// and NO server integrity check has taken place.
    AwaitingVerifiedServerResponse,
    Blocked,
}

pub struct KerberosNegotiation {
    trusted_spn: String,
    phase: KerberosPhase,
    source: Option<ChallengeSource>,
}
impl fmt::Debug for KerberosNegotiation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KerberosNegotiation")
            .field("trusted_spn", &"[REDACTED]")
            .field("phase", &self.phase)
            .finish()
    }
}
impl KerberosNegotiation {
    /// `trusted_spn` must come from an out-of-band administrator / trusted
    /// deployment policy. In particular it MUST NOT be copied from the 401.
    pub fn new(trusted_spn: &str) -> Result<Self, KerberosError> {
        let Some(host) = trusted_spn.strip_prefix("sip/") else {
            return Err(KerberosError::InvalidTarget);
        };
        if host.len() > 253
            || !host.contains('.')
            || !host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        {
            return Err(KerberosError::InvalidTarget);
        }
        Ok(Self {
            trusted_spn: trusted_spn.to_owned(),
            phase: KerberosPhase::AwaitingChallenge,
            source: None,
        })
    }
    pub fn phase(&self) -> KerberosPhase {
        self.phase
    }

    /// Consume a CHALLENGE to a previously validated REGISTER transaction,
    /// selecting exactly one trusted Kerberos offer. The `retry` must be a
    /// separate REGISTER transaction (incremented CSeq and fresh Via branch)
    /// generated by `RegisterFlow::plan_authentication_retry()`.
    /// No network I/O occurs. The returned draft is not yet signed with an SA.
    pub fn respond<P: KerberosProvider>(
        &mut self,
        challenged: &RegisterFlow,
        response: &SipMessage,
        retry: &RegisterFlow,
        provider: &mut P,
    ) -> Result<SipMessage, KerberosError> {
        if self.phase != KerberosPhase::AwaitingChallenge {
            return Err(KerberosError::State);
        }
        // An error is terminal for this context. A fresh negotiation requires
        // a fresh instance; never reuse a potentially failed GSS context.
        self.phase = KerberosPhase::Blocked;
        if challenged.state() != RegisterState::NeedsAuthentication
            || retry.state() != RegisterState::AwaitingResponse
            || retry.cseq()
                != challenged
                    .cseq()
                    .checked_add(1)
                    .ok_or(KerberosError::InvalidRetry)?
            || challenged.request().header("Call-ID") != retry.request().header("Call-ID")
            || challenged.request().header("Via") == retry.request().header("Via")
        {
            return Err(KerberosError::InvalidRetry);
        }
        match &response.start_line {
            StartLine::Response {
                code: 401 | 407, ..
            } => {}
            _ => return Err(KerberosError::InvalidChallenge),
        }
        // Bind the supplied response to the request whose state was already
        // updated via RegisterFlow::observe. A different response must not be
        // able to inject its own authentication target or GSS token.
        for key in ["Call-ID", "CSeq"] {
            if unique(response, key)? != unique(challenged.request(), key)? {
                return Err(KerberosError::ResponseMismatch);
            }
        }
        if branch(unique(response, "Via")?)? != branch(unique(challenged.request(), "Via")?)? {
            return Err(KerberosError::ResponseMismatch);
        }
        for key in ["From", "To", "Contact", "Event", "Supported"] {
            if unique(challenged.request(), key)? != unique(retry.request(), key)? {
                return Err(KerberosError::InvalidRetry);
            }
        }
        let offers = parse_challenges(response).map_err(|_| KerberosError::InvalidChallenge)?;
        let kerberos: Vec<_> = offers
            .iter()
            .filter(|o| o.scheme == Scheme::Kerberos)
            .collect();
        if kerberos.len() != 1 {
            return Err(KerberosError::AmbiguousChallenge);
        }
        let chosen = kerberos[0];
        if chosen.targetname.as_deref() != Some(self.trusted_spn.as_str()) {
            // Refuse silent Kerberos -> NTLM / Digest fallback or target substitution.
            return Err(KerberosError::UntrustedChallenge);
        }
        let realm = chosen
            .realm
            .as_ref()
            .ok_or(KerberosError::InvalidChallenge)?;
        if realm.is_empty()
            || realm.len() > 256
            || !realm.is_ascii()
            || realm.bytes().any(|b| b.is_ascii_control())
            || !matches!(chosen.version, Some(3 | 4))
            || chosen.gssapi_data.is_some()
        {
            return Err(KerberosError::InvalidChallenge);
        }
        // This implementation negotiates protocol version 3 only. As specified
        // by MS-SIPAE 3.2.5.1, a v3 client may respond version=3 to v4 offer.
        // Validate header material BEFORE invoking credential-bearing GSS.
        let realm_quoted = quote(realm)?;
        let target_quoted = quote(&self.trusted_spn)?;
        let opaque_quoted = chosen.opaque.as_deref().map(quote).transpose()?;
        let step = provider.initiate(&self.trusted_spn)?;
        // GSS/SSPI is a multi-step API (RFC 2743): an initial AP_REQ may be
        // sent even when the context returns CONTINUE_NEEDED. Requiring
        // "complete" here would make native Kerberos initiation impossible.
        // It still MUST NOT authorize registration or server-side trust.
        if !step.integrity {
            return Err(KerberosError::NotComplete);
        }
        if step.token.is_empty() || step.token.len() > MAX_GSS_TOKEN {
            return Err(KerberosError::InvalidToken);
        }
        let mut value = format!(
            "Kerberos qop=\"auth\", realm=\"{}\", targetname=\"{}\"",
            realm_quoted, target_quoted
        );
        if let Some(opaque) = opaque_quoted {
            value.push_str(&format!(", opaque=\"{}\"", opaque));
        }
        value.push_str(&format!(
            ", gssapi-data=\"{}\", version=3",
            STANDARD.encode(&step.token)
        ));
        let header = match chosen.source {
            ChallengeSource::Registrar => "Authorization",
            ChallengeSource::Proxy => "Proxy-Authorization",
        };
        let mut draft = retry.request().clone();
        if draft.headers.iter().any(|(k, _)| {
            k.eq_ignore_ascii_case("Authorization") || k.eq_ignore_ascii_case("Proxy-Authorization")
        }) {
            return Err(KerberosError::InvalidRequest);
        }
        draft.headers.push((header.to_owned(), value));
        draft.encode().map_err(|_| KerberosError::InvalidRequest)?;
        self.source = Some(chosen.source);
        self.phase = if step.complete {
            KerberosPhase::AwaitingVerifiedServerResponse
        } else {
            KerberosPhase::AwaitingContextContinuation
        };
        Ok(draft)
    }

    /// Safe status query; even a finished GSS exchange MUST NOT be interpreted
    /// as login until MS-SIPAE verifies the server's Authentication-Info MIC.
    pub fn awaits_server_mic(&self) -> bool {
        self.phase == KerberosPhase::AwaitingVerifiedServerResponse
    }
    pub fn challenge_source(&self) -> Option<ChallengeSource> {
        self.source
    }
}

fn unique<'a>(m: &'a SipMessage, name: &str) -> Result<&'a str, KerberosError> {
    let mut iter = m
        .headers
        .iter()
        .filter(|(n, _)| n.eq_ignore_ascii_case(name));
    let first = iter.next().ok_or(KerberosError::ResponseMismatch)?;
    if iter.next().is_some() {
        return Err(KerberosError::ResponseMismatch);
    }
    Ok(first.1.as_str())
}
fn branch(via: &str) -> Result<&str, KerberosError> {
    let mut found = None;
    for param in via.split(';').skip(1) {
        let Some((name, val)) = param.trim().split_once('=') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("branch")
            && (found.replace(val.trim()).is_some() || !val.starts_with("z9hG4bK"))
        {
            return Err(KerberosError::ResponseMismatch);
        }
    }
    found.ok_or(KerberosError::ResponseMismatch)
}
fn quote(value: &str) -> Result<String, KerberosError> {
    if value.len() > 512 || !value.is_ascii() || value.bytes().any(|b| b.is_ascii_control()) {
        return Err(KerberosError::InvalidChallenge);
    }
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registration::Endpoint;

    fn register() -> RegisterFlow {
        RegisterFlow::new(
            Endpoint {
                aor: "sip:alice@example.com".into(),
                sent_by: "192.0.2.50:5061".into(),
                contact_uri: "sip:192.0.2.50:5061;transport=tls".into(),
                instance_uuid: "b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97".into(),
                epid: "492a7ce35f".into(),
                from_tag: "2d0be03279".into(),
                call_id: "74c55d45a6ee404680aa55c8fe126f11".into(),
            },
            5,
            "z9hG4bK-initial-long-token",
            3600,
            0,
        )
        .unwrap()
    }
    fn challenge(flow: &RegisterFlow, code: u16, auth: &[&str]) -> SipMessage {
        let mut headers: Vec<(String, String)> = ["Via", "Call-ID", "CSeq"]
            .iter()
            .map(|name| ((*name).into(), flow.request().header(name).unwrap().into()))
            .collect();
        let key = if code == 401 {
            "WWW-Authenticate"
        } else {
            "Proxy-Authenticate"
        };
        headers.extend(auth.iter().map(|v| (key.into(), (*v).into())));
        SipMessage {
            start_line: StartLine::Response {
                code,
                reason: "Test".into(),
            },
            headers,
            body: vec![],
        }
    }
    const GOOD: &str = "Kerberos realm=\"SIP Communications Service\", targetname=\"sip/server.example.com\", version=4";
    struct MockProvider {
        invoked: bool,
    }
    impl KerberosProvider for MockProvider {
        fn initiate(&mut self, target: &str) -> Result<KerberosStep, KerberosError> {
            assert_eq!(target, "sip/server.example.com");
            self.invoked = true;
            Ok(KerberosStep {
                token: vec![1, 2, 3],
                complete: true,
                integrity: true,
            })
        }
    }
    fn prepare(code: u16, values: &[&str]) -> (RegisterFlow, SipMessage, RegisterFlow) {
        let mut f = register();
        let c = challenge(&f, code, values);
        f.observe(&c, 10).unwrap();
        let retry = f
            .plan_authentication_retry("z9hG4bK-new-long-token", 20)
            .unwrap();
        (f, c, retry)
    }
    #[test]
    fn microsoft_kerberos_v4_challenge_creates_v3_draft_with_new_cseq_and_branch() {
        let (f, c, r) = prepare(
            401,
            &[
                "NTLM realm=\"a\", targetname=\"server.example.com\", version=3",
                GOOD,
            ],
        );
        let mut n = KerberosNegotiation::new("sip/server.example.com").unwrap();
        let mut provider = MockProvider { invoked: false };
        let output = n.respond(&f, &c, &r, &mut provider).unwrap();
        let value = output.header("Authorization").unwrap();
        assert!(value.contains("gssapi-data=\"AQID\""));
        assert!(value.contains("version=3"));
        assert_eq!(output.header("CSeq"), Some("6 REGISTER"));
        assert_ne!(output.header("Via"), f.request().header("Via"));
        assert!(provider.invoked && n.awaits_server_mic());
        assert_eq!(
            n.respond(&f, &c, &r, &mut provider).unwrap_err(),
            KerberosError::State
        );
    }
    #[test]
    fn proxy_challenge_uses_proxy_authorization() {
        let (f, c, r) = prepare(407, &[GOOD]);
        let mut n = KerberosNegotiation::new("sip/server.example.com").unwrap();
        let output = n
            .respond(&f, &c, &r, &mut MockProvider { invoked: false })
            .unwrap();
        assert!(output.header("Proxy-Authorization").is_some());
        assert!(output.header("Authorization").is_none());
        assert_eq!(n.challenge_source(), Some(ChallengeSource::Proxy));
    }
    #[test]
    fn rejects_untrusted_target_without_calling_provider_or_downgrading() {
        let (f, c, r) = prepare(
            401,
            &[
                "NTLM realm=\"a\", targetname=\"evil.example.com\", version=4",
                "Kerberos realm=\"a\", targetname=\"sip/evil.example.com\", version=4",
            ],
        );
        let mut n = KerberosNegotiation::new("sip/server.example.com").unwrap();
        let mut provider = MockProvider { invoked: false };
        assert_eq!(
            n.respond(&f, &c, &r, &mut provider).unwrap_err(),
            KerberosError::UntrustedChallenge
        );
        assert!(!provider.invoked);
        assert_eq!(n.phase(), KerberosPhase::Blocked);
    }
    #[test]
    fn rejects_replaced_response_even_after_observing_original() {
        let (f, mut c, r) = prepare(401, &[GOOD]);
        c.headers
            .iter_mut()
            .find(|(k, _)| k == "Call-ID")
            .unwrap()
            .1 = "wrong".into();
        let mut n = KerberosNegotiation::new("sip/server.example.com").unwrap();
        assert_eq!(
            n.respond(&f, &c, &r, &mut MockProvider { invoked: false })
                .unwrap_err(),
            KerberosError::ResponseMismatch
        );
    }
    #[test]
    fn rejects_ambiguous_kerberos_challenges() {
        let (f, c, r) = prepare(401, &[GOOD, GOOD]);
        let mut n = KerberosNegotiation::new("sip/server.example.com").unwrap();
        assert_eq!(
            n.respond(&f, &c, &r, &mut MockProvider { invoked: false })
                .unwrap_err(),
            KerberosError::AmbiguousChallenge
        );
    }
    #[test]
    fn no_kerberos_provider_fails_closed() {
        let (f, c, r) = prepare(401, &[GOOD]);
        let mut n = KerberosNegotiation::new("sip/server.example.com").unwrap();
        assert_eq!(
            n.respond(&f, &c, &r, &mut NoKerberosProvider).unwrap_err(),
            KerberosError::ProviderUnavailable
        );
        assert_eq!(n.phase(), KerberosPhase::Blocked);
    }
    #[test]
    fn incomplete_gss_context_produces_initial_token_without_claiming_authentication() {
        struct Incomplete;
        impl KerberosProvider for Incomplete {
            fn initiate(&mut self, _: &str) -> Result<KerberosStep, KerberosError> {
                Ok(KerberosStep {
                    token: vec![3],
                    complete: false,
                    integrity: true,
                })
            }
        }
        let (f, c, r) = prepare(401, &[GOOD]);
        let mut n = KerberosNegotiation::new("sip/server.example.com").unwrap();
        let draft = n.respond(&f, &c, &r, &mut Incomplete).unwrap();
        assert!(draft.header("Authorization").is_some());
        assert_eq!(n.phase(), KerberosPhase::AwaitingContextContinuation);
        assert!(!n.awaits_server_mic());
    }
    #[test]
    fn refuses_intermediate_context_without_integrity() {
        struct MissingIntegrity;
        impl KerberosProvider for MissingIntegrity {
            fn initiate(&mut self, _: &str) -> Result<KerberosStep, KerberosError> {
                Ok(KerberosStep {
                    token: vec![3],
                    complete: false,
                    integrity: false,
                })
            }
        }
        let (f, c, r) = prepare(401, &[GOOD]);
        let mut n = KerberosNegotiation::new("sip/server.example.com").unwrap();
        assert_eq!(
            n.respond(&f, &c, &r, &mut MissingIntegrity).unwrap_err(),
            KerberosError::NotComplete
        );
        assert_eq!(n.phase(), KerberosPhase::Blocked);
    }
    #[test]
    fn validates_service_principal() {
        for principal in [
            "server.example.com",
            "sip/not-a-fqdn",
            "sip/evil@example.com",
            "sip/server.example.com\r\n",
        ] {
            assert_eq!(
                KerberosNegotiation::new(principal).unwrap_err(),
                KerberosError::InvalidTarget
            );
        }
    }
}
