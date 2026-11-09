use super::*;

const GROUP: &str = "0762e30f-a76c-4c75-9390-4c07a9f4e51f";

fn envelope(body: &str) -> String {
    format!("<soap:Envelope xmlns:soap=\"http://schemas.xmlsoap.org/soap/envelope/\"><soap:Body>{body}</soap:Body></soap:Envelope>")
}

/// §4.2 (p. 34): SignIn request and successful answer; §4.3 (p. 35): unsuccessful SignOut.
#[test]
fn sign_in_and_out() {
    let r = request(&Op::SignIn(GROUP.to_uppercase())).unwrap();
    assert!(r.contains(&format!(
        "<SignIn xmlns=\"{NS_RGS}\"><groupId>{GROUP}</groupId></SignIn>"
    )));
    assert_eq!(
        Op::SignIn(GROUP.into()).soap_action(),
        format!("\"{NS_RGS}/SignIn\"")
    );
    let ok = envelope(&format!(
        "<SignInResponse xmlns=\"{NS_RGS}\"><SignInResult>true</SignInResult></SignInResponse>"
    ));
    assert_eq!(parse_bool(&ok, &Op::SignIn(GROUP.into())), Ok(true));
    let no = envelope(&format!("<SignOutResponse xmlns=\"{NS_RGS}\"><SignOutResult>false</SignOutResult></SignOutResponse>"));
    assert_eq!(parse_bool(&no, &Op::SignOut(GROUP.into())), Ok(false));
    // The answer of another operation is not taken.
    assert!(parse_bool(&ok, &Op::SignOut(GROUP.into())).is_err());
    assert_eq!(
        request(&Op::SignIn("x</groupId>".into())),
        Err(Error::BadGroupId)
    );
    assert!(request(&Op::GetGroups)
        .unwrap()
        .contains(&format!("<GetGroups xmlns=\"{NS_RGS}\"/>")));
}

/// §4.1 (p. 34).
#[test]
fn active_response_groups() {
    let xml = envelope(&format!(
        "<GetActiveResponseGroupsResponse xmlns=\"{NS_RGS}\"><GetActiveResponseGroupsResult>\
<ResponseGroupEntry><Uri>sip:helpdesk@contoso.com</Uri><DisplayName>Contoso Helpdesk</DisplayName><IsAnonymized>true</IsAnonymized><IsOutboundAllowed>true</IsOutboundAllowed></ResponseGroupEntry>\
<ResponseGroupEntry><Uri>sip:hr@contoso.com</Uri><DisplayName>Contoso HR</DisplayName><IsAnonymized>true</IsAnonymized><IsOutboundAllowed>false</IsOutboundAllowed></ResponseGroupEntry>\
<ResponseGroupEntry><Uri>javascript:x</Uri></ResponseGroupEntry>\
</GetActiveResponseGroupsResult></GetActiveResponseGroupsResponse>"
    ));
    let l = parse_response_groups(&xml).unwrap();
    assert_eq!(l.len(), 2);
    assert_eq!(l[0].uri, "sip:helpdesk@contoso.com");
    assert_eq!(l[0].display_name.as_deref(), Some("Contoso Helpdesk"));
    assert!(l[0].anonymized && l[0].outbound_allowed);
    assert!(!l[1].outbound_allowed);
}

#[test]
fn groups_and_agent() {
    let xml = envelope(&format!(
        "<GetGroupsResponse xmlns=\"{NS_RGS}\"><GetGroupsResult>\
<AcdGroup><Id>{GROUP}</Id><Name>Support</Name><NumberOfAgents>2</NumberOfAgents><CanSignIn>true</CanSignIn><SignInState>SignedOut</SignInState>\
<AllAgents><AcdAgent><Id>{GROUP}</Id><SipAddress>sip:alice@contoso.com</SipAddress><DisplayName>Alice</DisplayName></AcdAgent></AllAgents>\
<NumberOfCallsWaiting>3</NumberOfCallsWaiting><LongestWaitingTime>95</LongestWaitingTime></AcdGroup>\
<AcdGroup><Id>not-a-guid</Id><Name>Bad</Name></AcdGroup>\
</GetGroupsResult></GetGroupsResponse>"
    ));
    let g = parse_groups(&xml).unwrap();
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].name, "Support");
    assert!(g[0].can_sign_in);
    assert_eq!(g[0].state, SignInState::SignedOut);
    assert_eq!((g[0].calls_waiting, g[0].longest_wait), (3, 95));
    assert_eq!(g[0].members[0].display_name.as_deref(), Some("Alice"));
    let none = envelope(&format!("<GetAgentResponse xmlns=\"{NS_RGS}\"/>"));
    assert_eq!(parse_agent(&none), Ok(None));
    let fault = envelope("<soap:Fault><faultcode>soap:Server</faultcode><faultstring>not an agent</faultstring></soap:Fault>");
    assert_eq!(
        parse_groups(&fault),
        Err(Error::Fault("not an agent".into()))
    );
}
