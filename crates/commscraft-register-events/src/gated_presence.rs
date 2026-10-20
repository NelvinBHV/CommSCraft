//! Offline-only composition: RFC 3261 matched REGISTER + MS-SIPAE MIC/replay
//! gate, then optional untrusted MS-SIPREGE Presence-State hint.
//!
//! This module does NOT bind the security association to a real TLS peer.
//! No caller may promote these results to an authenticated CommSCraft session.
use crate::sip_binding::{parse_register_presence_response, BindingError};
use crate::PresenceHint;
use commscraft_core::registration::MicCheckedRegisterResult;
use commscraft_core::security::ServerMicVerifier;
use commscraft_core::session::{GateError, GateEvent, RegisterVerificationGate};
use commscraft_core::sip::{SipMessage, StartLine};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatedPresenceError {
    InvalidPresence(BindingError),
    RegistrationGate(GateError),
}

/// An offline gate outcome and optional *untrusted* registrar metadata.
/// No authenticated-user, TLS-binding, or production sign-in claim is carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatedPresenceOutcome {
    pub event: GateEvent,
    pub untrusted_hint: Option<PresenceHint>,
}

/// Validate optional Microsoft metadata before altering any exchange state.
/// Then let the already-correlated REGISTER gate check SIP transaction identity,
/// signed server MIC and replay. Release the hint ONLY after a MIC-checked
/// REGISTER binding; never on challenge, provisional, removal, or invalid MIC.
///
/// A successful result is still an OFFLINE simulation without independent
/// proof of TLS peer identity and completed server security negotiation.
pub fn observe_register_with_presence<V: ServerMicVerifier>(
    gate: &mut RegisterVerificationGate,
    response: &SipMessage,
    verifier: &V,
    now_ms: u64,
) -> Result<GatedPresenceOutcome, GatedPresenceError> {
    // Preflight the unauthenticated optional field without touching the gate.
    // Do not parse or return registration hints for other response codes.
    let candidate_hint = if matches!(response.start_line, StartLine::Response { code: 200, .. }) {
        parse_register_presence_response(response).map_err(GatedPresenceError::InvalidPresence)?
    } else {
        None
    };

    let event = gate
        .observe(response, verifier, now_ms)
        .map_err(GatedPresenceError::RegistrationGate)?;

    let untrusted_hint = if matches!(
        event,
        GateEvent::MicChecked(MicCheckedRegisterResult::RegistrarReportedBinding { .. })
    ) {
        candidate_hint
    } else {
        None
    };
    Ok(GatedPresenceOutcome {
        event,
        untrusted_hint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisterAction;
    use commscraft_core::challenge::{ChallengeOffer, ChallengeSource, Scheme};
    use commscraft_core::registration::{Endpoint, RegisterFlow};
    use commscraft_core::security::{
        parse_authentication_info, server_mic_buffer, SecurityAssociation, TlsDskHmacSha256,
    };
    use commscraft_core::session::GateState;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

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
        let flow = RegisterFlow::new(endpoint, 8, "z9hG4bK-integrated-000001", 3600, 0)
            .expect("valid synthetic REGISTER");
        let association = SecurityAssociation::begin(
            &ChallengeOffer {
                scheme: Scheme::TlsDsk,
                source: ChallengeSource::Registrar,
                realm: Some("SIP Communications Service".into()),
                targetname: Some("server.example.com".into()),
                opaque: Some("opaque123".into()),
                version: Some(4),
                sts_uri: None,
                gssapi_data: None,
            },
            8_000_000,
        )
        .expect("synthetic security association");
        RegisterVerificationGate::new(flow, association)
    }

    fn reply(gate: &RegisterVerificationGate, status: u16) -> SipMessage {
        SipMessage {
            start_line: StartLine::Response {
                code: status,
                reason: "Synthetic".into(),
            },
            headers: ["Via", "Call-ID", "CSeq"]
                .iter()
                .map(|name| {
                    (
                        (*name).into(),
                        gate.request().header(name).expect("request header").into(),
                    )
                })
                .collect(),
            body: Vec::new(),
        }
    }

    fn signed_200(gate: &RegisterVerificationGate, snum: u64) -> SipMessage {
        let mut msg = reply(gate, 200);
        msg.headers.extend([
            ("From".into(), "<sip:alice@example.com>;tag=2d0be03279;epid=492a7ce35f".into()),
            ("To".into(), "<sip:alice@example.com>;tag=server".into()),
            ("Contact".into(), "<sip:192.0.2.50:5061;transport=tls>;expires=3600;+sip.instance=\"<urn:uuid:b43b3d1d-9f8f-5fdc-9f74-3ca273cadb97>\";gruu=\"sip:alice@example.com;opaque=untrusted;gruu\"".into()),
            ("Expires".into(), "3600".into()),
            ("Authentication-Info".into(), format!(
                "TLS-DSK realm=\"SIP Communications Service\", targetname=\"server.example.com\", opaque=\"opaque123\", qop=auth, snum={snum}, srand=abcdef09, version=4, rspauth=\"00\""
            )),
        ]);
        let info = parse_authentication_info(&msg).expect("valid synthetic info");
        let signed_bytes = server_mic_buffer(&msg, &info).expect("valid MIC buffer");
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&[0x5a; 32]).expect("test key");
        mac.update(&signed_bytes);
        let digest: String = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let value = msg
            .header("Authentication-Info")
            .expect("authentication info")
            .replace("rspauth=\"00\"", &format!("rspauth=\"{digest}\""));
        msg.headers
            .iter_mut()
            .find(|(name, _)| name == "Authentication-Info")
            .expect("authentication info header")
            .1 = value;
        msg.headers
            .push(("Presence-State".into(), "register-action=\"added\"".into()));
        msg
    }

    fn verifier() -> TlsDskHmacSha256 {
        TlsDskHmacSha256::from_server_authentication_key([0x5a; 32])
    }

    #[test]
    fn synthetically_signed_correlated_200_may_expose_only_untrusted_hint() {
        let mut gate = gate();
        let msg = signed_200(&gate, 7);
        let got = observe_register_with_presence(&mut gate, &msg, &verifier(), 5)
            .expect("synthetic MIC-checked response");
        assert!(matches!(
            got.event,
            GateEvent::MicChecked(MicCheckedRegisterResult::RegistrarReportedBinding { .. })
        ));
        assert_eq!(
            got.untrusted_hint.expect("header present").action,
            RegisterAction::Added
        );
        assert_eq!(gate.state(), GateState::MicCheckedNotSignedIn);
        assert_eq!(gate.highest_verified_server_sequence(), Some(7));
    }

    #[test]
    fn wrong_transaction_cannot_reveal_hint_or_advance_gate() {
        let mut gate = gate();
        let mut msg = signed_200(&gate, 7);
        msg.headers
            .iter_mut()
            .find(|(name, _)| name == "Call-ID")
            .expect("Call-ID")
            .1
            .push_str("-forged");
        assert_eq!(
            observe_register_with_presence(&mut gate, &msg, &verifier(), 5),
            Err(GatedPresenceError::RegistrationGate(
                GateError::UnmatchedResponse
            ))
        );
        assert_eq!(gate.state(), GateState::AwaitingResponse);
        assert_eq!(gate.highest_verified_server_sequence(), None);
    }

    #[test]
    fn invalid_mic_does_not_release_hint_or_record_replay_number() {
        let mut gate = gate();
        let mut msg = signed_200(&gate, 7);
        msg.headers
            .iter_mut()
            .find(|(name, _)| name == "Authentication-Info")
            .expect("Authentication-Info")
            .1
            .push('x');
        assert!(matches!(
            observe_register_with_presence(&mut gate, &msg, &verifier(), 5),
            Err(GatedPresenceError::RegistrationGate(GateError::Mic(_)))
        ));
        assert_eq!(gate.state(), GateState::Rejected);
        assert_eq!(gate.highest_verified_server_sequence(), None);
    }

    #[test]
    fn invalid_optional_metadata_does_not_consume_valid_mic_or_transaction() {
        let mut gate = gate();
        let mut msg = signed_200(&gate, 7);
        msg.headers
            .push(("presence-state".into(), "register-action=\"fixed\"".into()));
        assert_eq!(
            observe_register_with_presence(&mut gate, &msg, &verifier(), 5),
            Err(GatedPresenceError::InvalidPresence(BindingError::Presence(
                crate::ParseError::DuplicateHeader
            )))
        );
        assert_eq!(gate.state(), GateState::AwaitingResponse);
        assert_eq!(gate.highest_verified_server_sequence(), None);
        msg.headers.pop();
        let good = observe_register_with_presence(&mut gate, &msg, &verifier(), 6);
        assert!(matches!(
            good,
            Ok(GatedPresenceOutcome {
                untrusted_hint: Some(_),
                ..
            })
        ));
    }

    #[test]
    fn challenge_never_exposes_microsoft_presence_hint() {
        let mut gate = gate();
        let mut msg = reply(&gate, 401);
        msg.headers
            .push(("Presence-State".into(), "register-action=\"added\"".into()));
        msg.headers.push(("WWW-Authenticate".into(),
            "Kerberos realm=\"SIP Communications Service\", targetname=\"sip/server.example.com\", version=4".into()));
        let result =
            observe_register_with_presence(&mut gate, &msg, &verifier(), 5).expect("challenge");
        assert!(matches!(result.event, GateEvent::ChallengeRequired(_)));
        assert_eq!(result.untrusted_hint, None);
        assert_eq!(gate.state(), GateState::ChallengeRequired);
    }

    #[test]
    fn timed_out_gate_cannot_become_registered_by_late_mic() {
        let mut gate = gate();
        let msg = signed_200(&gate, 9);
        let event = observe_register_with_presence(&mut gate, &msg, &verifier(), 32_000)
            .expect("timeout event");
        assert_eq!(event.event, GateEvent::TimedOut);
        assert_eq!(event.untrusted_hint, None);
        assert_eq!(gate.state(), GateState::TimedOut);
    }

    #[test]
    fn after_disconnect_a_valid_mic_is_no_longer_accepted() {
        let mut gate = gate();
        let msg = signed_200(&gate, 10);
        gate.disconnect();
        assert_eq!(
            observe_register_with_presence(&mut gate, &msg, &verifier(), 5),
            Err(GatedPresenceError::RegistrationGate(GateError::Closed))
        );
        assert_eq!(gate.highest_verified_server_sequence(), None);
    }
}
