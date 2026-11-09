use super::*;

/// The answer of [MS-OXWSADISC] §4.3 (pp. 81–82), shortened to three of its settings.
const EXAMPLE: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" xmlns:a="http://www.w3.org/2005/08/addressing">
  <s:Header>
    <a:Action s:mustUnderstand="1">http://schemas.microsoft.com/exchange/2010/Autodiscover/Autodiscover/GetUserSettingsResponse</a:Action>
  </s:Header>
  <s:Body>
    <GetUserSettingsResponseMessage xmlns="http://schemas.microsoft.com/exchange/2010/Autodiscover">
      <Response xmlns:i="http://www.w3.org/2001/XMLSchema-instance">
        <ErrorCode>NoError</ErrorCode>
        <ErrorMessage />
        <UserResponses>
          <UserResponse>
            <ErrorCode>NoError</ErrorCode>
            <ErrorMessage>No error.</ErrorMessage>
            <RedirectTarget i:nil="true" />
            <UserSettingErrors />
            <UserSettings>
              <UserSetting i:type="StringSetting">
                <Name>UserDisplayName</Name>
                <Value>Terry Adams</Value>
              </UserSetting>
              <UserSetting i:type="StringSetting">
                <Name>CasVersion</Name>
                <Value>14.01.0063.000</Value>
              </UserSetting>
              <UserSetting i:type="StringSetting">
                <Name>ExternalEwsUrl</Name>
                <Value>https://mail.Contoso.com/EWS/Exchange.asmx</Value>
              </UserSetting>
            </UserSettings>
          </UserResponse>
        </UserResponses>
      </Response>
    </GetUserSettingsResponseMessage>
  </s:Body>
</s:Envelope>"#;

fn answer(user_response: &str) -> String {
    format!(
        "<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\"><s:Body>\
<GetUserSettingsResponseMessage xmlns=\"http://schemas.microsoft.com/exchange/2010/Autodiscover\">\
<Response><ErrorCode>NoError</ErrorCode><UserResponses><UserResponse>{user_response}</UserResponse>\
</UserResponses></Response></GetUserSettingsResponseMessage></s:Body></s:Envelope>"
    )
}

/// EX-08: the endpoints U1, U2 of §1.5 (pp. 11–12); unusable domains give none.
#[test]
fn autodiscover_endpoints() {
    assert_eq!(
        autodiscover_urls("Contoso.com.").unwrap(),
        [
            "https://autodiscover.contoso.com/autodiscover/autodiscover.svc",
            "https://contoso.com/autodiscover/autodiscover.svc"
        ]
    );
    for bad in [
        "",
        "localhost",
        "a..b",
        "-a.com",
        "evil.com/x",
        "a b.com",
        "x.com:444",
    ] {
        assert!(autodiscover_urls(bad).is_none(), "{bad}");
    }
}

/// The request carries the mailbox, the settings and the endpoint, escaped (example §4.3).
#[test]
fn user_settings_request_form() {
    let r = user_settings_request(
        "carol@contoso.com",
        &SETTINGS,
        "https://autodiscover.contoso.com/autodiscover/autodiscover.svc",
    )
    .unwrap();
    assert!(r.contains("<a:Mailbox>carol@contoso.com</a:Mailbox>"));
    assert!(
        r.contains("<a:Setting>InternalEwsUrl</a:Setting><a:Setting>ExternalEwsUrl</a:Setting>")
    );
    assert!(r.contains(
        "<wsa:Action>http://schemas.microsoft.com/exchange/2010/Autodiscover/Autodiscover/GetUserSettings</wsa:Action>"
    ));
    assert!(r.contains(
        "<wsa:To>https://autodiscover.contoso.com/autodiscover/autodiscover.svc</wsa:To>"
    ));
    commscraft_xml::parse(&r, &Limits::default()).unwrap();
    for bad in ["carol", "<x>@a.com", "a@b@c", "a b@c.com", &"x".repeat(300)] {
        assert_eq!(
            user_settings_request(bad, &SETTINGS, "https://x"),
            Err(Error::BadMailbox),
            "{bad}"
        );
    }
    assert!(user_settings_request("a@b.com", &["Ews<Url"], "https://x").is_err());
}

/// The example answer of §4.3 gives its settings; the EWS URL is found.
#[test]
fn example_answer() {
    let s = parse_user_settings(EXAMPLE).unwrap();
    assert_eq!(s.get("UserDisplayName"), Some("Terry Adams"));
    assert_eq!(s.get("CasVersion"), Some("14.01.0063.000"));
    assert_eq!(
        s.https_url(&["InternalEwsUrl", "ExternalEwsUrl"]),
        Some("https://mail.Contoso.com/EWS/Exchange.asmx")
    );
    assert_eq!(s.get("InternalEwsUrl"), None);
}

/// Redirects are reported (RedirectUrl only to HTTPS); error codes and faults are errors.
#[test]
fn redirects_and_errors() {
    let r = |code: &str, target: &str| {
        parse_user_settings(&answer(&format!(
            "<ErrorCode>{code}</ErrorCode><RedirectTarget>{target}</RedirectTarget>"
        )))
    };
    assert_eq!(
        r("RedirectAddress", "carol@fabrikam.com"),
        Ok(UserSettings::RedirectAddress("carol@fabrikam.com".into()))
    );
    assert_eq!(
        r(
            "RedirectUrl",
            "https://ad.fabrikam.com/autodiscover/autodiscover.svc"
        ),
        Ok(UserSettings::RedirectUrl(
            "https://ad.fabrikam.com/autodiscover/autodiscover.svc".into()
        ))
    );
    assert!(r("RedirectUrl", "http://ad.fabrikam.com/x").is_err());
    assert!(r("RedirectAddress", "not-an-address").is_err());
    assert_eq!(
        r("InvalidUser", ""),
        Err(Error::Server("InvalidUser".into()))
    );
    let fault = "<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\"><s:Body><s:Fault>\
<faultcode>a:ActionNotSupported</faultcode><faultstring>no&#9;such action</faultstring></s:Fault></s:Body></s:Envelope>";
    assert_eq!(
        parse_user_settings(fault),
        Err(Error::Fault("nosuch action".into()))
    );
}

/// Hostile answers: wrong document, wrong namespace, too many settings, a DTD.
#[test]
fn hostile_answers() {
    assert!(parse_user_settings("<x/>").is_err());
    assert!(parse_user_settings(&EXAMPLE.replace("2010/Autodiscover\"", "2010/Other\"")).is_err());
    let many: String = (0..200)
        .map(|i| format!("<UserSetting><Name>S{i}</Name><Value>v</Value></UserSetting>"))
        .collect();
    let s = parse_user_settings(&answer(&format!(
        "<ErrorCode>NoError</ErrorCode><UserSettings>{many}</UserSettings>"
    )))
    .unwrap();
    assert!(matches!(s, UserSettings::Settings(ref v) if v.len() == MAX_SETTINGS));
    assert!(parse_user_settings("<!DOCTYPE x [<!ENTITY a \"b\">]><x>&a;</x>").is_err());
}

/// EX-06: the URL of the example of [MS-OXWSPHOTO] §4 (p. 11) from the EWS URL.
#[test]
fn photo_url_form() {
    assert_eq!(
        photo_url(
            "https://outlook.office365.com/ews/Exchange.asmx",
            "user1@contoso.com",
            PhotoSize::Hr120
        )
        .unwrap(),
        "https://outlook.office365.com/ews/Exchange.asmx/s/GetUserPhoto?email=user1%40contoso.com&size=HR120x120"
    );
    // K-EX-1: a photos URL that already ends in /s.
    assert_eq!(
        photo_url(
            "https://ex.contoso.com/EWS/Exchange.asmx/s/",
            "a@b.com",
            PhotoSize::Hr48
        )
        .unwrap(),
        "https://ex.contoso.com/EWS/Exchange.asmx/s/GetUserPhoto?email=a%40b.com&size=HR48x48"
    );
    assert!(photo_url("http://x/ews/Exchange.asmx", "a@b.com", PhotoSize::Hr48).is_err());
    assert!(photo_url(
        "https://x/ews/Exchange.asmx?a=b",
        "a@b.com",
        PhotoSize::Hr48
    )
    .is_err());
    assert!(photo_url(
        "https://x/ews/Exchange.asmx",
        "a&size=1@b.com",
        PhotoSize::Hr48
    )
    .is_err());
    assert_eq!(PhotoSize::Hr648.code(), "HR648x648");
}

/// §3.1.5: 200 with type and ETag, 304, 404, 401, others; a malformed ETag is dropped.
#[test]
fn photo_answers() {
    assert_eq!(
        photo_answer(200, Some("image/jpeg"), Some("\"889B7442\"")),
        PhotoAnswer::Image {
            content_type: Some("image/jpeg".into()),
            etag: Some("\"889B7442\"".into())
        }
    );
    assert_eq!(
        photo_answer(200, None, Some("889B7442\r\nX: y")),
        PhotoAnswer::Image {
            content_type: None,
            etag: None
        }
    );
    assert_eq!(photo_answer(304, None, None), PhotoAnswer::Unchanged);
    assert_eq!(photo_answer(404, None, None), PhotoAnswer::NoPhoto);
    assert_eq!(photo_answer(401, None, None), PhotoAnswer::Unauthorized);
    assert_eq!(photo_answer(400, None, None), PhotoAnswer::Failed(400));
}
