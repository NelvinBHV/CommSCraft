use super::*;

fn url(s: &str) -> HttpsUrl {
    parse_https(s).unwrap()
}

/// UW-01: the challenge of the reference ("Create an application"), next to a Bearer challenge.
#[test]
fn oauth_challenge_of_the_reference() {
    let c = oauth_challenge([
        "Bearer trusted_issuers=\"00000002-0000-0ff1-ce00-000000000000@contoso.com\", client_id=\"00000004-0000-0ff1-ce00-000000000000\"",
        "MsRtcOAuth href=\"https://lyncweb.contoso.com/WebTicket/oauthtoken\",grant_type=\"urn:microsoft.rtc:windows,urn:microsoft.rtc:anonmeeting,password\"",
    ])
    .unwrap();
    assert_eq!(c.token_url.host, "lyncweb.contoso.com");
    assert_eq!(c.token_url.path, "/WebTicket/oauthtoken");
    assert!(c.offers_windows());
    assert_eq!(c.grants.len(), 3);
    // Plain HTTP or no challenge: nothing.
    assert!(oauth_challenge([
        "MsRtcOAuth href=\"http://x.contoso.com/t\",grant_type=\"password\""
    ])
    .is_none());
    assert!(oauth_challenge(["Negotiate", "NTLM"]).is_none());
    let pw_only =
        oauth_challenge(["MsRtcOAuth href=\"https://a.contoso.com/t\",grant_type=\"password\""])
            .unwrap();
    assert!(!pw_only.offers_windows());
}

/// UW-02: the token answer of the reference; hostile variants refused.
#[test]
fn token_answer() {
    let t = token(
        br#"{"access_token":"cwt=2YotnFZFEjr1zCsicMWpAA","token_type":"Bearer","expires_in":3600}"#,
    )
    .unwrap();
    assert_eq!(
        t.authorization().as_str(),
        "Bearer cwt=2YotnFZFEjr1zCsicMWpAA"
    );
    assert_eq!(t.expires_in, 3600);
    // The token is never printed.
    assert!(!format!("{t:?}").contains("cwt"));
    // With the extra member of the example and spaces.
    let t = token(b"{ \"access_token\" : \"cwt=A\\/B\", \"expires_in\": 28799, \"ms_rtc_identityscope\":\"local\", \"token_type\":\"Bearer\" }").unwrap();
    assert_eq!(t.access_token.as_str(), "cwt=A/B");
    for bad in [
        &br#"{"access_token":"x","token_type":"MAC"}"#[..],
        br#"{"token_type":"Bearer"}"#,
        br#"{"access_token":"a b","token_type":"Bearer"}"#,
        br#"{"access_token":{"x":1},"token_type":"Bearer"}"#,
        br#"[1,2]"#,
        br#"{"access_token":"x","token_type":"Bearer"} trailing"#,
        b"{\"access_token\":\"\x01\",\"token_type\":\"Bearer\"}",
        b"",
    ] {
        assert!(token(bad).is_err(), "{:?}", String::from_utf8_lossy(bad));
    }
    assert!(token(&vec![b' '; MAX_TOKEN_ANSWER + 1]).is_err());
    assert_eq!(
        windows_token_request(),
        "grant_type=urn:microsoft.rtc:windows"
    );
}

/// UW-04: the User resource with its UCWA links (shape of [MS-OCDISCWS] §4.1).
#[test]
fn user_resource_links() {
    let body = "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?><AutodiscoverResponse AccessLocation=\"Internal\"><User><Link token=\"Self\" href=\"https://fe1.contoso.com/Autodiscover/AutodiscoverService.svc/root/oauth/user\"/><Link token=\"External/Ucwa\" href=\"https://web.contoso.com/ucwa/oauth/v1/applications\"/><Link token=\"Internal/Ucwa\" href=\"https://fe1.contoso.com/ucwa/oauth/v1/applications\"/><Link token=\"Ucwa\" href=\"http://bad.contoso.com/ucwa\"/></User></AutodiscoverResponse>";
    let links = ucwa_links(body.as_bytes()).unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].host, "fe1.contoso.com");
    assert_eq!(links[1].host, "web.contoso.com");
    assert!(ucwa_links(b"<resource/>").is_err());
}

/// UW-05: the application request of the reference, values escaped and bounded.
#[test]
fn application_request_body() {
    let b = application_request("en-US", "123456", "CommSCraft/0.0.1").unwrap();
    assert!(b.contains(&format!("<input xmlns=\"{NS}\">")));
    assert!(b.contains("<property name=\"endpointId\">123456</property>"));
    assert!(application_request("en-US", "a<b", "x")
        .unwrap()
        .contains("a&lt;b"));
    assert!(application_request("", "1", "x").is_none());
    assert!(application_request("en", &"1".repeat(101), "x").is_none());
    assert!(application_request("en", "1", "x\ny").is_none());
}

const APPLICATION: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<resource rel="application" href="/ucwa/oauth/v1/applications/105" xmlns="http://schemas.microsoft.com/rtc/2012/03/ucwa">
  <link rel="policies" href="/ucwa/oauth/v1/applications/105/policies" />
  <property name="culture">en-US</property>
  <resource rel="me" href="/ucwa/oauth/v1/applications/105/me">
    <link rel="photo" href="/ucwa/oauth/v1/applications/105/photos/lenea@contoso.com" />
    <property name="uri">sip:lenea@contoso.com</property>
  </resource>
  <resource rel="people" href="/ucwa/oauth/v1/applications/105/people">
    <link rel="myContacts" href="/ucwa/oauth/v1/applications/105/people/contacts" />
    <link rel="search" href="/ucwa/oauth/v1/applications/105/people/search" />
  </resource>
</resource>"#;

/// UW-06: the application resource with the embedded `me` and `people` (shape of the reference).
#[test]
fn application_resource() {
    let app = resource(APPLICATION.as_bytes()).unwrap();
    assert_eq!(app.rel, "application");
    let me = app.find("me").unwrap();
    assert_eq!(
        me.link("photo"),
        Some("/ucwa/oauth/v1/applications/105/photos/lenea@contoso.com")
    );
    let people = app.find("people").unwrap();
    assert_eq!(
        people.link("myContacts"),
        Some("/ucwa/oauth/v1/applications/105/people/contacts")
    );
    assert_eq!(
        people.link("search"),
        Some("/ucwa/oauth/v1/applications/105/people/search")
    );
    // Another namespace or element: refused.
    assert!(resource(b"<resource rel=\"x\"/>").is_err());
    assert!(resource(format!("<x xmlns=\"{NS}\"/>").as_bytes()).is_err());
    assert!(resource(&vec![b' '; MAX_DOCUMENT + 1]).is_err());
}

const EXPANDED: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<resource rel="distributionGroup" href="/ucwa/v1/applications/192/groups/distributionGroup" xmlns="http://schemas.microsoft.com/rtc/2012/03/ucwa">
  <link rel="expandDistributionGroup" href="/ucwa/v1/applications/192/groups/distributionGroup/expandDistributionGroup" />
  <property name="uri">sip:mypersonalgroup@contoso.com</property>
  <property name="name">MyPersonalGroup</property>
  <resource rel="contact" href="/ucwa/v1/applications/192/people/282">
    <link rel="contactPhoto" href="/ucwa/v1/applications/192/people/282/contactPhoto" />
    <property name="uri">sip:Alex@contoso.com</property>
  </resource>
  <resource rel="contact" href="/ucwa/v1/applications/192/people/283">
    <property name="uri">sip:nophoto@contoso.com</property>
  </resource>
  <resource rel="distributionGroup" href="/ucwa/v1/applications/192/groups/distributionGroup">
    <link rel="expandDistributionGroup" href="/ucwa/v1/applications/192/groups/inner/expandDistributionGroup" />
    <property name="uri">sip:inner@contoso.com</property>
    <resource rel="contact" href="/ucwa/v1/applications/192/people/284">
      <link rel="contactPhoto" href="/ucwa/v1/applications/192/people/284/contactPhoto" />
      <property name="uri">sip:bea@contoso.com</property>
    </resource>
  </resource>
</resource>"#;

/// UW-07, UW-09: the members of an expanded group with their photo links (also nested groups);
/// a contact without a photo link is left out; the expand link of a group by its address.
#[test]
fn expanded_group_photo_links() {
    let doc = resource(EXPANDED.as_bytes()).unwrap();
    let links = photo_links(&doc);
    assert_eq!(
        links,
        vec![
            PhotoLink {
                uri: "sip:alex@contoso.com".into(),
                href: "/ucwa/v1/applications/192/people/282/contactPhoto".into()
            },
            PhotoLink {
                uri: "sip:bea@contoso.com".into(),
                href: "/ucwa/v1/applications/192/people/284/contactPhoto".into()
            },
        ]
    );
    assert_eq!(
        group_expand_link(&doc, "MyPersonalGroup@contoso.com"),
        Some("/ucwa/v1/applications/192/groups/distributionGroup/expandDistributionGroup".into())
    );
    assert_eq!(
        group_expand_link(&doc, "sip:inner@contoso.com"),
        Some("/ucwa/v1/applications/192/groups/inner/expandDistributionGroup".into())
    );
    assert_eq!(group_expand_link(&doc, "other@contoso.com"), None);
}

/// UW-07, UW-08: query strings for photo and search.
#[test]
fn photo_and_search_queries() {
    assert_eq!(
        photo_query("/ucwa/v1/applications/192/people/282/contactPhoto", 96),
        "/ucwa/v1/applications/192/people/282/contactPhoto?size=HR96X96"
    );
    assert_eq!(
        photo_query("/p/contactPhoto?participantHash=1", 9999),
        "/p/contactPhoto?participantHash=1&size=HR648X648"
    );
    assert_eq!(
        search_query(
            "/ucwa/v1/applications/192/people/search",
            "alex@contoso.com",
            500
        )
        .unwrap(),
        "/ucwa/v1/applications/192/people/search?query=alex@contoso.com&limit=100"
    );
    assert_eq!(
        search_query("/s", "a b&c", 5).unwrap(),
        "/s?query=a%20b%26c&limit=5"
    );
    assert!(search_query("/s", &"x".repeat(257), 5).is_none());
    assert!(search_query("/s", "", 5).is_none());
}

/// UW-10: links are followed only to the UCWA service's own host and port, over HTTPS.
#[test]
fn links_stay_on_the_service() {
    let svc = url("https://fe1.contoso.com/ucwa/oauth/v1/applications");
    let r = resolve(&svc, "/ucwa/oauth/v1/applications/105/people/contacts").unwrap();
    assert_eq!((r.host.as_str(), r.port), ("fe1.contoso.com", 443));
    assert!(resolve(&svc, "https://FE1.contoso.com/ucwa/x").is_some());
    for bad in [
        "https://evil.example/ucwa/x",
        "http://fe1.contoso.com/ucwa/x",
        "https://fe1.contoso.com:8443/ucwa/x",
        "//evil.example/x",
        "/ucwa/x\r\nHost: evil",
        "/ucwa/x y",
    ] {
        assert!(resolve(&svc, bad).is_none(), "{bad}");
    }
    let svc_port = url("https://fe1.contoso.com:4443/ucwa");
    assert_eq!(resolve(&svc_port, "/ucwa/a").unwrap().port, 4443);
}
