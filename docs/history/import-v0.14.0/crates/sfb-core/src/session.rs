//! Authentication-aware, offline registration coordination.
//!
//! RFC 3261 §§10.2, 17.1.2; MS-SIPAE §§3.2.1, 3.2.5.2;
//! MS-SIPREGE §4.1.1. A SIP 200 OK never itself proves sign-in.
//!
//! This module has NO transport write API and deliberately cannot elevate a
//! synthetic MIC or TLS-independent message to a productive user session.

use crate::registration::{
    MicCheckedRegisterError, MicCheckedRegisterResult, RegisterFlow,
    RegisterOutcome, RegisterState, RegistrationError,
};
use crate::security::{SecurityAssociation, ServerMicVerifier};
use crate::sip::{SipMessage, StartLine};

/// UI-only state machine. All events are untrusted descriptions of a demo.
/// Never interpret this type as a security decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Offline,
    Discovering,
    Connecting,
    AwaitingChallenge,
    Authenticating,
    Registering,
    /// A 200 OK observed by a UI demo; absolutely NOT signed in.
    AcceptanceUnverified,
    Recovering,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    Start,
    DiscoverySucceeded,
    Connected,
    ChallengeReceived,
    CredentialsPrepared,
    /// Deliberately weak: an arbitrary SIP response cannot authorize login.
    RegistrationAccepted,
    TransportLost,
    Retry,
    Failure,
    SignOut,
}

#[derive(Debug, Clone)]
pub struct SessionMachine { state: SessionState }

impl Default for SessionMachine {
    fn default() -> Self { Self { state: SessionState::Offline } }
}

impl SessionMachine {
    pub fn state(&self) -> SessionState { self.state }

    pub fn dispatch(&mut self, event: SessionEvent) -> Result<SessionState, &'static str> {
        use SessionEvent as E;
        use SessionState as S;
        let next = match (self.state, event) {
            (_, E::SignOut) => S::Offline,
            (S::Offline, E::Start) => S::Discovering,
            (S::Discovering, E::DiscoverySucceeded) => S::Connecting,
            (S::Connecting, E::Connected) => S::AwaitingChallenge,
            (S::AwaitingChallenge, E::ChallengeReceived) => S::Authenticating,
            (S::Authenticating, E::CredentialsPrepared) => S::Registering,
            (S::Registering, E::RegistrationAccepted) => S::AcceptanceUnverified,
            (S::AcceptanceUnverified, E::TransportLost) => S::Recovering,
            (S::Registering, E::TransportLost) => S::Recovering,
            (S::Recovering, E::Retry) => S::Discovering,
            (S::Failed, E::Retry) => S::Discovering,
            (_, E::Failure) => S::Failed,
            _ => return Err("event not valid in current session state"),
        };
        self.state = next;
        Ok(next)
    }
}

/// The gate represents ONE REGISTER transaction on ONE assumed transport
/// generation. The caller MUST discard it on TLS loss/reconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateState {
    AwaitingResponse,
    ChallengeRequired,
    IntervalRetryRequired,
    /// SIPAE MIC, replay and REGISTER matching passed, but not live TLS or GSS.
    MicCheckedNotSignedIn,
    RemovalMicChecked,
    Rejected,
    TimedOut,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateEvent {
    Provisional(u16),
    /// A challenge is an unauthenticated request to begin GSS negotiation,
    /// NOT proof of server identity or of completion of authentication.
    ChallengeRequired(RegisterOutcome),
    IntervalRetryRequired { minimum_seconds: u32 },
    MicChecked(MicCheckedRegisterResult),
    Rejected(u16),
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateError {
    Closed,
    UnmatchedResponse,
    /// A correlated but invalid final answer poisons this exchange.
    InvalidOrUnverifiedFinal,
    Registration(RegistrationError),
    Mic(MicCheckedRegisterError),
}

/// Intentionally not Clone and not a network object: no socket, packet send,
/// credential token or "is_signed_in" method can be obtained from this gate.
pub struct RegisterVerificationGate {
    flow: RegisterFlow,
    association: SecurityAssociation,
    state: GateState,
}

impl RegisterVerificationGate {
    pub fn new(flow: RegisterFlow, association: SecurityAssociation) -> Self {
        Self { flow, association, state: GateState::AwaitingResponse }
    }

    pub fn request(&self) -> &SipMessage { self.flow.request() }
    pub fn state(&self) -> GateState { self.state }
    pub fn observed_register_state(&self) -> RegisterState { self.flow.state() }
    pub fn highest_verified_server_sequence(&self) -> Option<u64> {
        self.association.highest_verified()
    }

    /// Apply a framed, matching response; only a valid Microsoft MIC may
    /// promote 200 OK to MicCheckedNotSignedIn. A failure on a matching 200
    /// aborts this exchange (fail closed); an unrelated response is ignored.
    /// RFC 3261 §17.1.2: 401/407/423 are final and require a new transaction.
    pub fn observe<V: ServerMicVerifier>(
        &mut self, response: &SipMessage, verifier: &V, now_ms: u64,
    ) -> Result<GateEvent, GateError> {
        if self.state != GateState::AwaitingResponse { return Err(GateError::Closed); }
        if self.flow.tick(now_ms).is_some() {
            self.state = GateState::TimedOut;
            return Ok(GateEvent::TimedOut);
        }
        // The REGISTER flow itself owns an RFC 3261 matching transaction.
        // For 200, it checks matching without consuming state, then authenticates.
        let code = match &response.start_line {
            StartLine::Response { code, .. } => *code,
            _ => return Err(GateError::UnmatchedResponse),
        };
        if code == 200 {
            return match self.flow.observe_mic_checked_200(
                response, &mut self.association, verifier, now_ms,
            ) {
                Ok(result) => {
                    self.state = match result {
                        MicCheckedRegisterResult::RegistrarReportedRemoval => GateState::RemovalMicChecked,
                        MicCheckedRegisterResult::RegistrarReportedBinding { .. } => GateState::MicCheckedNotSignedIn,
                    };
                    Ok(GateEvent::MicChecked(result))
                }
                Err(MicCheckedRegisterError::Transaction | MicCheckedRegisterError::NotFinalOk) =>
                    Err(GateError::UnmatchedResponse),
                Err(error) => {
                    self.state = GateState::Rejected;
                    Err(GateError::Mic(error))
                },
            };
        }
        // Non-200 responses cannot imply registration success. Clone to
        // preserve the pending exchange if an unmatched/invalid answer arrives.
        let mut candidate = self.flow.clone();
        let outcome = match candidate.observe(response, now_ms) {
            Ok(outcome) => outcome,
            Err(RegistrationError::Transaction) => return Err(GateError::UnmatchedResponse),
            Err(error) => {
                self.state = GateState::Rejected;
                return Err(GateError::Registration(error));
            }
        };
        match outcome {
            RegisterOutcome::Provisional(code) => {
                self.flow = candidate;
                Ok(GateEvent::Provisional(code))
            }
            RegisterOutcome::Challenge { .. } => {
                self.flow = candidate;
                self.state = GateState::ChallengeRequired;
                Ok(GateEvent::ChallengeRequired(outcome))
            }
            RegisterOutcome::IntervalTooBrief { minimum_seconds } => {
                self.flow = candidate;
                self.state = GateState::IntervalRetryRequired;
                Ok(GateEvent::IntervalRetryRequired { minimum_seconds })
            }
            RegisterOutcome::Rejected(code) => {
                self.flow = candidate;
                self.state = GateState::Rejected;
                Ok(GateEvent::Rejected(code))
            }
            RegisterOutcome::TimedOut => {
                self.flow = candidate;
                self.state = GateState::TimedOut;
                Ok(GateEvent::TimedOut)
            }
            // The only path that accepts a 200 is the MIC-gated one above.
            RegisterOutcome::AcceptanceReported { .. } | RegisterOutcome::UnregistrationReported => {
                self.state = GateState::Rejected;
                Err(GateError::InvalidOrUnverifiedFinal)
            }
        }
    }

    pub fn tick(&mut self, now_ms: u64) -> Option<GateEvent> {
        if self.state == GateState::AwaitingResponse && self.flow.tick(now_ms).is_some() {
            self.state = GateState::TimedOut;
            return Some(GateEvent::TimedOut);
        }
        None
    }

    /// Must be called for EOF, TLS failure or new peer/connection. Old
    /// security associations must never be silently reused across sockets.
    pub fn disconnect(&mut self) {
        self.state = GateState::Disconnected;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::challenge::{ChallengeOffer, ChallengeSource, Scheme};
    use crate::registration::Endpoint;
    use crate::security::TlsDskHmacSha256;
    use crate::sip::SipMessage;

    fn gate() -> RegisterVerificationGate {
        let endpoint = Endpoint {
            aor: "sip:alice@example.com".into(),
            sent_by: "192.0.2.50:5061".into(),
            contact_uri: "sip:192.0.2.50:5061;transport=tls".into(),
            instance_uuid: "b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97".into(),
            epid: "492a7ce35f".into(),
            from_tag: "2d0be03279".into(),
            call_id: "aabbccdd0011@example.com".into(),
        };
        let flow = RegisterFlow::new(endpoint, 8, "z9hG4bK-integrated-000001", 3600, 0).unwrap();
        let association = SecurityAssociation::begin(&ChallengeOffer {
            scheme: Scheme::TlsDsk,
            source: ChallengeSource::Registrar,
            realm: Some("SIP Communications Service".into()),
            targetname: Some("server.example.com".into()),
            opaque: Some("opaque123".into()),
            version: Some(4),
            sts_uri: None,
            gssapi_data: None,
        }, 8_000_000).unwrap();
        RegisterVerificationGate::new(flow, association)
    }
    fn matching_reply(gate: &RegisterVerificationGate, code: u16) -> SipMessage {
        SipMessage {
            start_line: StartLine::Response { code, reason: "Synthetic".into() },
            headers: ["Via", "Call-ID", "CSeq"].iter().map(|name|
                ((*name).into(), gate.request().header(name).unwrap().into())).collect(),
            body: Vec::new(),
        }
    }
    #[test]
    fn demo_200_is_not_a_login() {
        let mut demo = SessionMachine::default();
        for event in [SessionEvent::Start, SessionEvent::DiscoverySucceeded,
            SessionEvent::Connected, SessionEvent::ChallengeReceived,
            SessionEvent::CredentialsPrepared, SessionEvent::RegistrationAccepted] {
            demo.dispatch(event).unwrap();
        }
        assert_eq!(demo.state(), SessionState::AcceptanceUnverified);
        demo.dispatch(SessionEvent::TransportLost).unwrap();
        assert_eq!(demo.dispatch(SessionEvent::Retry).unwrap(), SessionState::Discovering);
    }
    #[test]
    fn untrusted_200_poisoned_without_consuming_synthetic_sa() {
        let mut gate = gate();
        let reply = matching_reply(&gate, 200);
        let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
        assert!(matches!(gate.observe(&reply, &verifier, 100), Err(GateError::Mic(_))));
        assert_eq!(gate.state(), GateState::Rejected);
        assert_eq!(gate.highest_verified_server_sequence(), None);
        assert_eq!(gate.observe(&reply, &verifier, 101), Err(GateError::Closed));
    }
    #[test]
    fn challenge_is_final_and_does_not_authenticate() {
        let mut gate = gate();
        let mut response = matching_reply(&gate, 401);
        response.headers.push(("WWW-Authenticate".into(),
            "Kerberos realm=\"SIP Communications Service\", targetname=\"sip/server.example.com\", version=4".into()));
        let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
        assert!(matches!(gate.observe(&response, &verifier, 100), Ok(GateEvent::ChallengeRequired(_))));
        assert_eq!(gate.state(), GateState::ChallengeRequired);
        assert_eq!(gate.highest_verified_server_sequence(), None);
        assert_eq!(gate.observe(&response, &verifier, 101), Err(GateError::Closed));
    }
    #[test]
    fn unrelated_200_does_not_consume_gate() {
        let mut gate = gate();
        let mut reply = matching_reply(&gate, 200);
        reply.headers.iter_mut().find(|(k, _)| k == "Call-ID").unwrap().1.push_str("-wrong");
        let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
        assert_eq!(gate.observe(&reply, &verifier, 100), Err(GateError::UnmatchedResponse));
        assert_eq!(gate.state(), GateState::AwaitingResponse);
        assert_eq!(gate.highest_verified_server_sequence(), None);
    }
    #[test]
    fn fatal_disconnect_prevents_reuse() {
        let mut gate = gate();
        gate.disconnect();
        let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
        let reply = matching_reply(&gate, 401);
        assert_eq!(gate.observe(&reply, &verifier, 100), Err(GateError::Closed));
    }
    #[test]
    fn timeout_closes_gate() {
        let mut gate = gate();
        assert_eq!(gate.tick(32000), Some(GateEvent::TimedOut));
        assert_eq!(gate.state(), GateState::TimedOut);
    }
    fn signed_200(gate: &RegisterVerificationGate, number: u64) -> SipMessage {
        use crate::security::{parse_authentication_info, server_mic_buffer};
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut reply = matching_reply(gate, 200);
        reply.headers.extend([
            ("From".into(), "<sip:alice@example.com>;tag=2d0be03279;epid=492a7ce35f".into()),
            ("To".into(), "<sip:alice@example.com>;tag=server".into()),
            ("Contact".into(), "<sip:192.0.2.50:5061;transport=tls>;expires=3600;+sip.instance=\"<urn:uuid:b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97>\";gruu=\"sip:alice@example.com;opaque=untrusted;gruu\"".into()),
            ("Expires".into(), "3600".into()),
            ("Authentication-Info".into(), format!(
                "TLS-DSK realm=\"SIP Communications Service\", targetname=\"server.example.com\", opaque=\"opaque123\", qop=auth, snum={number}, srand=abcdef09, version=4, rspauth=\"00\""
            )),
        ]);
        let info = parse_authentication_info(&reply).unwrap();
        let buffer = server_mic_buffer(&reply, &info).unwrap();
        let mut hmac = <Hmac<Sha256> as Mac>::new_from_slice(&[0x5a; 32]).unwrap();
        hmac.update(&buffer);
        let mic: String = hmac.finalize().into_bytes().iter()
            .map(|v| format!("{v:02x}")).collect();
        let signed = reply.header("Authentication-Info").unwrap()
            .replace("rspauth=\"00\"", &format!("rspauth=\"{mic}\""));
        reply.headers.iter_mut().find(|(name, _)| name == "Authentication-Info")
            .unwrap().1 = signed;
        reply
    }
    #[test]
    fn synthetic_mic_checked_200_is_not_signed_in() {
        let mut gate = gate();
        let reply = signed_200(&gate, 7);
        let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
        let result = gate.observe(&reply, &verifier, 5);
        assert!(matches!(result, Ok(GateEvent::MicChecked(
            MicCheckedRegisterResult::RegistrarReportedBinding { .. }))));
        assert_eq!(gate.state(), GateState::MicCheckedNotSignedIn);
        assert_eq!(gate.highest_verified_server_sequence(), Some(7));
        // An old response cannot enter another phase in this gate.
        assert_eq!(gate.observe(&reply, &verifier, 6), Err(GateError::Closed));
    }
    #[test]
    fn bad_mic_in_synthetic_success_fails_closed() {
        let mut gate = gate();
        let mut reply = signed_200(&gate, 9);
        reply.headers.iter_mut().find(|(k,_)| k == "Authentication-Info")
            .unwrap().1.push_str("x");
        let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
        assert!(matches!(gate.observe(&reply, &verifier, 1), Err(GateError::Mic(_))));
        assert_eq!(gate.state(), GateState::Rejected);
        assert_eq!(gate.highest_verified_server_sequence(), None);
    }
    #[test]
    fn challenged_final_response_must_not_be_followed_by_200() {
        let mut gate = gate();
        let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
        let mut challenge = matching_reply(&gate, 401);
        challenge.headers.push(("WWW-Authenticate".into(),
            "Kerberos realm=\"SIP Communications Service\", targetname=\"sip/server.example.com\", version=4".into()));
        assert!(gate.observe(&challenge, &verifier, 1).is_ok());
        let late_200 = signed_200(&gate, 2);
        assert_eq!(gate.observe(&late_200, &verifier, 2), Err(GateError::Closed));
    }

    #[test]
    fn malformed_challenge_closes_matching_exchange() {
        let mut gate = gate();
        let reply = matching_reply(&gate, 401); // matching transaction, missing WWW-Authenticate
        let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
        assert!(matches!(gate.observe(&reply, &verifier, 1), Err(GateError::Registration(_))));
        assert_eq!(gate.state(), GateState::Rejected);
    }
    #[test]
    fn late_reply_does_not_resume_timed_out_exchange() {
        let mut gate = gate();
        let reply = matching_reply(&gate, 401);
        let verifier = TlsDskHmacSha256::from_server_authentication_key([0x5a; 32]);
        assert_eq!(gate.observe(&reply, &verifier, 32_000), Ok(GateEvent::TimedOut));
        assert_eq!(gate.state(), GateState::TimedOut);
    }

}
