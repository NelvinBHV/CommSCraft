//! Normative offline tests for Microsoft [MS-SIPREGE] 2.2.1.4 and 3.1.2.5.1.
//! RFC 3261 section 10.3: REGISTER success is NOT itself client authentication.
use commscraft_register_events::{
    parse_presence_state, Cluster, ClusterState, ParseError, RegisterAction,
};

#[test]
fn accepted_registration_has_untrusted_added_hint() {
    let actual = parse_presence_state(200, &[("Presence-State", "register-action=\"added\"")])
        .expect("valid documented header")
        .expect("hint");
    assert_eq!(actual.action, RegisterAction::Added);
    // Note that the result is only a parsed hint; it carries no security context.
}

#[test]
fn fixed_and_remote_cluster_fields_match_microsoft_abnf() {
    let hint = parse_presence_state(200, &[(
        "presence-state",
        "register-action=\"fixed\";primary-cluster-type=\"remote\";is-connected-to-primary=\"no\";user-services-state=unavailable",
    )]).unwrap().unwrap();
    assert_eq!(hint.action, RegisterAction::Fixed);
    assert_eq!(hint.cluster, Some(ClusterState { kind: Cluster::Remote, connected: false }));
    assert!(hint.user_services_unavailable);
}

#[test]
fn unknown_or_non_ok_responses_do_not_create_login_state() {
    assert_eq!(parse_presence_state(401, &[("Presence-State", "register-action=\"added\"")]), Ok(None));
    assert_eq!(parse_presence_state(503, &[("Presence-State", "register-action=\"refreshed\"")]), Ok(None));
}

#[test]
fn duplicated_header_is_rejected() {
    assert_eq!(parse_presence_state(200, &[
        ("Presence-State", "register-action=\"added\""),
        ("pReSeNcE-sTaTe", "register-action=\"refreshed\"")
    ]), Err(ParseError::DuplicateHeader));
}

#[test]
fn unauthorized_extension_and_header_injection_fail_closed() {
    for bad in [
        "register-action=\"added\";user-services-state=unavailable",
        "register-action=\"fixed\";primary-cluster-type=\"remote\"",
        "register-action=\"added\"\r\nAuthorization: Kerberos token",
    ] {
        assert!(parse_presence_state(200, &[("Presence-State", bad)]).is_err(), "{bad}");
    }
}
