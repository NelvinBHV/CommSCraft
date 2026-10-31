use super::*;

fn ok(body: &str) -> Response {
    Response::Http {
        status: 200,
        location: None,
        content_type: Some("application/vnd.microsoft.rtc.autodiscover+xml; v=1".into()),
        body: body.into(),
    }
}

fn redirect(to: &str) -> Response {
    Response::Http {
        status: 302,
        location: Some(to.into()),
        content_type: None,
        body: String::new(),
    }
}

fn not_found() -> Response {
    Response::Http {
        status: 404,
        location: None,
        content_type: None,
        body: String::new(),
    }
}

/// [MS-OCDISCWS] 20250520 §4.1 (p. 24): first Root response (PDF line breaks joined).
const ROOT: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<AutodiscoverResponse xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" AccessLocation="Internal">
  <Root>
    <Link token="Domain" href="https://contoso.com/Autodiscover/AutodiscoverService.svc/root/domain?originalDomain=contoso.com"/>
    <Link token="User" href="https://contoso.com/Autodiscover/AutodiscoverService.svc/root/user?originalDomain=contoso.com"/>
    <Link token="Self" href="https://contoso.com/Autodiscover/AutodiscoverService.svc/root?originalDomain=contoso.com"/>
    <Link token="OAuth" href="https://contoso.com/Autodiscover/AutodiscoverService.svc/root/oauth/user?originalDomain=contoso.com"/>
  </Root>
</AutodiscoverResponse>"#;

/// §4.1 (pp. 24–25): User resource with a single Redirect link.
const USER_REDIRECT: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<AutodiscoverResponse xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" AccessLocation="Internal">
  <User>
    <Link href="https://pool1.contoso.com/Autodiscover/AutodiscoverService.svc/root?originalDomain=contoso.com" token="Redirect"/>
  </User>
</AutodiscoverResponse>"#;

/// §4.1 (pp. 25–26): final User resource with the web service links.
const USER_FINAL: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<AutodiscoverResponse AccessLocation="Internal" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <User>
    <Link token="Internal/Autodiscover" href="https://pool1.contoso.com/Autodiscover/AutodiscoverService.svc/root"/>
    <Link token="Internal/AuthBroker" href="https://pool1.contoso.com/Reach/sip.svc"/>
    <Link token="Internal/Ucwa" href="https://pool1.contoso.com/Ucwa/oauth/v1/applications"/>
    <Link token="External/Autodiscover" href="https://pool1external.contoso.com/Autodiscover/AutodiscoverService.svc/root"/>
    <Link token="External/AuthBroker" href="https://pool1external.contoso.com/Reach/sip.svc"/>
    <Link token="External/Ucwa" href="https://pool1external.contoso.com/Ucwa/oauth/v1/applications"/>
  </User>
</AutodiscoverResponse>"#;

/// Own fixture per the Domain schema (§2.2.4.2, p. 11; §2.2.4.5, p. 13).
fn domain_doc(location: &str) -> String {
    format!(
        r#"<AutodiscoverResponse AccessLocation="{location}"><Domain>
<SipServerInternalAccess fqdn="pool1.contoso.com" port="5061"/>
<SipClientInternalAccess fqdn="pool1.contoso.com" port="5061"/>
<SipClientExternalAccess fqdn="sip.contoso.com" port="443"/>
<Link token="External/AuthBroker" href="https://pool1external.contoso.com/Reach/sip.svc"/>
<Link token="Bad" href="http://pool1.contoso.com/x"/>
</Domain></AutodiscoverResponse>"#
    )
}

fn ad() -> Autodiscover {
    Autodiscover::new(&Config {
        aor: "sip:john@contoso.com".into(),
        allow_plain_http: false,
    })
    .unwrap()
}

#[test]
fn https_only_by_default_and_internal_first() {
    let mut a = ad();
    assert_eq!(
        a.start(),
        Step::Fetch("https://lyncdiscoverinternal.contoso.com/?sipuri=john@contoso.com".into())
    );
    assert_eq!(
        a.on_response(Response::Failed),
        Step::Fetch("https://lyncdiscover.contoso.com/?sipuri=john@contoso.com".into())
    );
    assert_eq!(a.on_response(not_found()), Step::Failed(Error::NotFound));

    let mut a = Autodiscover::new(&Config {
        aor: "sip:john@contoso.com".into(),
        allow_plain_http: true,
    })
    .unwrap();
    assert_eq!(
        a.start(),
        Step::Fetch("http://lyncdiscoverinternal.contoso.com/?sipuri=john@contoso.com".into())
    );
}

#[test]
fn example_flow_through_root_and_domain() {
    let mut a = ad();
    a.start();
    assert_eq!(
        a.on_response(ok(ROOT)),
        Step::Fetch("https://contoso.com/Autodiscover/AutodiscoverService.svc/root/domain?originalDomain=contoso.com".into())
    );
    let Step::Done(o) = a.on_response(ok(&domain_doc("internal"))) else {
        panic!()
    };
    assert_eq!(o.access_location, Some(AccessLocation::Internal));
    let hosts: Vec<(&str, u16)> = o
        .targets
        .iter()
        .map(|t| (t.host.as_str(), t.port))
        .collect();
    assert_eq!(
        hosts,
        [("pool1.contoso.com", 5061), ("sip.contoso.com", 443)]
    );
    assert!(o.targets.iter().all(|t| t.in_user_domain));
    assert_eq!(o.links.len(), 1, "the http:// link is dropped");
    assert_eq!(o.links[0].url.host, "pool1external.contoso.com");
}

#[test]
fn external_location_prefers_the_external_access_point() {
    let mut a = ad();
    a.start();
    a.on_response(ok(ROOT));
    // K-AD-1: the examples write "Internal"/"External" in upper case.
    let Step::Done(o) = a.on_response(ok(&domain_doc("External"))) else {
        panic!()
    };
    assert_eq!(o.targets[0].host, "sip.contoso.com");
}

#[test]
fn example_redirect_and_final_links() {
    let mut a = ad();
    a.start();
    a.on_response(ok(ROOT));
    assert_eq!(
        a.on_response(ok(USER_REDIRECT)),
        Step::Fetch("https://pool1.contoso.com/Autodiscover/AutodiscoverService.svc/root?originalDomain=contoso.com".into())
    );
    let Step::Done(o) = a.on_response(ok(USER_FINAL)) else {
        panic!()
    };
    assert!(o.targets.is_empty());
    let tokens: Vec<&str> = o.links.iter().map(|l| l.token.as_str()).collect();
    assert_eq!(tokens.len(), 6);
    assert!(tokens.contains(&"Internal/AuthBroker"));
    assert!(o.links.iter().all(|l| l.in_user_domain));
}

#[test]
fn redirects_are_bounded_and_checked() {
    let mut a = ad();
    a.start();
    let mut step = Step::Failed(Error::NotFound);
    for i in 0..=MAX_REDIRECTS {
        step = a.on_response(redirect(&format!("https://r{i}.contoso.com/")));
    }
    assert_eq!(step, Step::Failed(Error::TooManyRedirects));

    let mut a = ad();
    a.start();
    a.on_response(redirect("https://a.contoso.com/"));
    assert_eq!(
        a.on_response(redirect("https://A.contoso.com/")),
        Step::Failed(Error::RedirectLoop)
    );

    for bad in [
        "http://contoso.com/",
        "https://10.0.0.1/",
        "https://u@contoso.com/",
        "ftp://contoso.com/",
    ] {
        let mut a = ad();
        a.start();
        assert_eq!(
            a.on_response(redirect(bad)),
            Step::Failed(Error::UnsafeUrl),
            "{bad}"
        );
    }
}

#[test]
fn foreign_hosts_are_marked() {
    let mut a = ad();
    a.start();
    a.on_response(ok(ROOT));
    let doc = r#"<AutodiscoverResponse AccessLocation="external"><Domain>
<SipClientExternalAccess fqdn="sip.fabrikam.example" port="443"/>
<Link token="External/Ucwa" href="https://web.fabrikam.example/ucwa"/></Domain></AutodiscoverResponse>"#;
    let Step::Done(o) = a.on_response(ok(doc)) else {
        panic!()
    };
    assert!(!o.targets[0].in_user_domain);
    assert!(!o.links[0].in_user_domain);
}

#[test]
fn bad_documents_are_refused() {
    let cases: [(&str, Response); 6] = [
        (
            "content type",
            Response::Http {
                status: 200,
                location: None,
                content_type: Some("text/html".into()),
                body: ROOT.into(),
            },
        ),
        ("dtd", ok("<!DOCTYPE x><AutodiscoverResponse/>")),
        (
            "namespace",
            ok(r#"<AutodiscoverResponse xmlns="urn:x"><Root/></AutodiscoverResponse>"#),
        ),
        (
            "two kinds",
            ok("<AutodiscoverResponse><Root/><User/></AutodiscoverResponse>"),
        ),
        (
            "location",
            ok(r#"<AutodiscoverResponse AccessLocation="moon"><Domain/></AutodiscoverResponse>"#),
        ),
        (
            "port",
            ok(
                r#"<AutodiscoverResponse><Domain><SipClientInternalAccess fqdn="a.contoso.com" port="0"/></Domain></AutodiscoverResponse>"#,
            ),
        ),
    ];
    for (name, r) in cases {
        let mut a = ad();
        a.start();
        assert!(matches!(a.on_response(r), Step::Failed(_)), "{name}");
    }
    assert_eq!(
        Autodiscover::new(&Config {
            aor: "tel:+1".into(),
            allow_plain_http: false
        })
        .unwrap_err(),
        Error::BadAor
    );
    assert_eq!(
        Autodiscover::new(&Config {
            aor: "sip:a b@contoso.com".into(),
            allow_plain_http: false
        })
        .unwrap_err(),
        Error::BadAor
    );
}

fn srv(priority: u16, port: u16, target: &str) -> SrvRecord {
    SrvRecord {
        priority,
        weight: 0,
        port,
        target: target.into(),
    }
}

#[test]
fn dns_targets_in_order_with_fallbacks() {
    assert_eq!(
        srv_names("contoso.com"),
        [
            "_sipinternaltls._tcp.contoso.com".to_string(),
            "_sip._tls.contoso.com".to_string()
        ]
    );
    let internal = [
        srv(10, 5061, "pool2.contoso.com."),
        srv(0, 5061, "pool1.contoso.com"),
        srv(0, 5061, "evil.example"),
    ];
    let external = [srv(0, 443, "sip.contoso.com"), srv(0, 0, "x.contoso.com")];
    let t = srv_targets("contoso.com", &internal, &external);
    let got: Vec<(&str, u16, Source)> = t
        .iter()
        .map(|t| (t.host.as_str(), t.port, t.source))
        .collect();
    assert_eq!(
        got,
        [
            ("pool1.contoso.com", 5061, Source::SrvInternalTls),
            ("pool2.contoso.com", 5061, Source::SrvInternalTls),
            ("sip.contoso.com", 443, Source::SrvTls),
            ("sipinternal.contoso.com", 443, Source::Fallback),
            ("sipexternal.contoso.com", 443, Source::Fallback),
        ]
    );
    let ad_outcome = Outcome {
        access_location: None,
        targets: vec![SipTarget {
            host: "pool1.contoso.com".into(),
            port: 5061,
            source: Source::Autodiscover,
            in_user_domain: true,
        }],
        links: vec![],
    };
    let all = candidates(Some(&ad_outcome), t);
    assert_eq!(all[0].source, Source::Autodiscover);
    assert_eq!(all.len(), 5, "duplicate pool1 removed");
}
