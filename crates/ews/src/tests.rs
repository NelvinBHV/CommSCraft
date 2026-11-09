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

mod ucs_tests {
    use crate::ucs::*;
    use crate::Error;

    fn id(s: &str) -> ItemId {
        ItemId {
            id: s.into(),
            change_key: Some("EgAAAA==".into()),
        }
    }

    /// The AddImGroup request of [MS-OXWSCOS] §4 (p. 74) and the other operations.
    #[test]
    fn requests() {
        let r = ucs_request(&UcsOp::AddGroup {
            name: "MyCustomGroup".into(),
        })
        .unwrap();
        assert!(r.contains("<t:RequestServerVersion Version=\"Exchange2013\"/>"));
        assert!(
            r.contains("<m:AddImGroup><m:DisplayName>MyCustomGroup</m:DisplayName></m:AddImGroup>")
        );
        let r = ucs_request(&UcsOp::AddContactToGroup {
            contact: id("AAMkAGQ1MjJjMTBkLTc4Y2AA="),
            group: id("AAMkAGQ1MjJjMTBkzzAAAQKAAA="),
        })
        .unwrap();
        assert!(r.contains(
            "<m:ContactId Id=\"AAMkAGQ1MjJjMTBkLTc4Y2AA=\" ChangeKey=\"EgAAAA==\"/><m:GroupId Id=\"AAMkAGQ1MjJjMTBkzzAAAQKAAA=\""
        ));
        let r = ucs_request(&UcsOp::AddNewContact {
            im_address: "sip:bob@contoso.com".into(),
            name: Some("Bob <B>".into()),
            group: None,
        })
        .unwrap();
        assert!(r.contains("<m:ImAddress>sip:bob@contoso.com</m:ImAddress><m:DisplayName>Bob &lt;B&gt;</m:DisplayName></m:AddNewImContactToGroup>"));
        assert!(ucs_request(&UcsOp::GetList)
            .unwrap()
            .contains("<m:GetImItemList></m:GetImItemList>"));
        for op in [
            UcsOp::AddGroup { name: "".into() },
            UcsOp::AddGroup {
                name: "a\u{1}b".into(),
            },
            UcsOp::AddNewContact {
                im_address: "bob@contoso.com".into(),
                name: None,
                group: None,
            },
            UcsOp::AddNewContact {
                im_address: "sip:bob\"@contoso.com".into(),
                name: None,
                group: None,
            },
        ] {
            assert!(ucs_request(&op).is_err(), "{op:?}");
        }
        assert_eq!(
            UcsOp::RemoveContact { contact: id("A") }.soap_action(),
            "\"http://schemas.microsoft.com/exchange/services/2006/messages/RemoveContactFromImList\""
        );
    }

    const LIST: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body>
<GetImItemListResponse ResponseClass="Success" xmlns="http://schemas.microsoft.com/exchange/services/2006/messages">
<ResponseCode>NoError</ResponseCode>
<ImItemList>
 <Groups xmlns="http://schemas.microsoft.com/exchange/services/2006/types">
  <ImGroup><DisplayName>Other Contacts</DisplayName><GroupType>IPM.DistList.MOC.OtherContacts</GroupType>
   <ExchangeStoreId Id="AAMkG1=" ChangeKey="EgAAAA=="/>
   <MemberCorrelationKey><ItemId Id="AAMkP1=" ChangeKey="x"/></MemberCorrelationKey></ImGroup>
  <ImGroup><DisplayName>Team</DisplayName><GroupType>IPM.DistList.MOC.UserGroup</GroupType>
   <ExchangeStoreId Id="AAMkG2="/>
   <MemberCorrelationKey><ItemId Id="AAMkP1="/><ItemId Id="AAMkP2="/></MemberCorrelationKey></ImGroup>
  <ImGroup><GroupType>IPM.DistList.MOC.UserGroup</GroupType></ImGroup>
 </Groups>
 <Personas xmlns="http://schemas.microsoft.com/exchange/services/2006/types">
  <Persona><PersonaId Id="AAMkX1="/><DisplayName>Bob</DisplayName><ImAddress>sip:bob@contoso.com</ImAddress>
   <Attributions><Attribution><Id>0</Id><SourceId Id="AAMkP1=" ChangeKey="EQAAAA=="/><DisplayName>Favorites</DisplayName></Attribution></Attributions>
   <EmailAddress><Name>Bob</Name><EmailAddress>bob@contoso.com</EmailAddress></EmailAddress></Persona>
  <Persona><PersonaId Id="AAMkP2="/><DisplayName>Carol</DisplayName></Persona>
  <Persona><DisplayName>No id</DisplayName></Persona>
  <Persona><PersonaId Id="bad id&lt;"/></Persona>
 </Personas>
</ImItemList></GetImItemListResponse></s:Body></s:Envelope>"#;

    /// GetImItemList (§2.2.4.1–2.2.4.3; personas per [MS-OXWSPERS] §2.2.4.19): groups with their
    /// members, contacts with IM and mail address; entries without a usable identifier dropped.
    #[test]
    fn item_list() {
        let l = parse_im_item_list(LIST).unwrap();
        assert_eq!(l.groups.len(), 2);
        assert_eq!(l.groups[0].kind, GroupKind::OtherContacts);
        assert_eq!(l.groups[1].name, "Team");
        assert_eq!(l.groups[1].members, ["AAMkP1=", "AAMkP2="]);
        assert_eq!(l.groups[1].id.as_ref().unwrap().change_key, None);
        assert_eq!(l.personas.len(), 2);
        assert_eq!(
            l.personas[0].im_address.as_deref(),
            Some("sip:bob@contoso.com")
        );
        assert_eq!(l.personas[0].email.as_deref(), Some("bob@contoso.com"));
        assert_eq!(l.personas[1].im_address, None);
        // K-EX-2: membership through the contact item behind the persona.
        assert_eq!(l.personas[0].contact_id().id, "AAMkP1=");
        assert!(l.personas[0].in_group(&l.groups[0]) && l.personas[0].in_group(&l.groups[1]));
        assert_eq!(l.personas[1].contact_id().id, "AAMkP2=");
        assert!(!l.personas[1].in_group(&l.groups[0]) && l.personas[1].in_group(&l.groups[1]));
    }

    /// Answers of changes: the new group's and contact's identifiers; errors and faults.
    #[test]
    fn answers() {
        let add = UcsOp::AddGroup { name: "G".into() };
        let ok = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body>
<AddImGroupResponse ResponseClass="Success" xmlns="http://schemas.microsoft.com/exchange/services/2006/messages">
<ResponseCode>NoError</ResponseCode><ImGroup><DisplayName xmlns="http://schemas.microsoft.com/exchange/services/2006/types">MyCustomGroup</DisplayName>
<ExchangeStoreId Id="AAMkAGQ1MjJjMTBkLTc4Y2UtNDA5Ny04ZjU5LWI3MAAA=" ChangeKey="EgAAAA==" xmlns="http://schemas.microsoft.com/exchange/services/2006/types"/>
</ImGroup></AddImGroupResponse></s:Body></s:Envelope>"#;
        assert_eq!(
            parse_ucs_answer(&add, ok).unwrap().unwrap().id,
            "AAMkAGQ1MjJjMTBkLTc4Y2UtNDA5Ny04ZjU5LWI3MAAA="
        );
        let err = ok
            .replace("ResponseClass=\"Success\"", "ResponseClass=\"Error\"")
            .replace("<ResponseCode>NoError", "<ResponseCode>ErrorItemNotFound");
        assert_eq!(
            parse_ucs_answer(&add, &err),
            Err(Error::Server("ErrorItemNotFound".into()))
        );
        // The answer of another operation is not taken.
        assert!(parse_ucs_answer(&UcsOp::RemoveGroup { group: id("A") }, ok).is_err());
        assert!(parse_im_item_list("<x/>").is_err());
    }
}

mod calendar_tests {
    use crate::calendar::*;
    use commscraft_xml::{parse_xs_datetime, Limits};

    /// [MS-OXWAVLS] §4.2 (pp. 47–50), the second FreeBusyResponse (Detailed) first and with a
    /// merged string as DetailedMerged adds it (§4.3: one digit per block), shortened.
    const AVAILABILITY: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <GetUserAvailabilityResponse xmlns="http://schemas.microsoft.com/exchange/services/2006/messages">
      <FreeBusyResponseArray>
        <FreeBusyResponse>
          <ResponseMessage ResponseClass="Success"><ResponseCode>NoError</ResponseCode></ResponseMessage>
          <FreeBusyView>
            <FreeBusyViewType xmlns="http://schemas.microsoft.com/exchange/services/2006/types">DetailedMerged</FreeBusyViewType>
            <MergedFreeBusy xmlns="http://schemas.microsoft.com/exchange/services/2006/types">000000000000332000000000</MergedFreeBusy>
            <CalendarEventArray xmlns="http://schemas.microsoft.com/exchange/services/2006/types">
              <CalendarEvent>
                <StartTime>2008-01-21T08:00:00</StartTime>
                <EndTime>2008-01-21T09:00:00</EndTime>
                <BusyType>Tentative</BusyType>
                <CalendarEventDetails>
                  <ID>00000000CEB2AC9C</ID>
                  <Subject>Meeting1</Subject>
                  <Location>Location1</Location>
                  <IsMeeting>true</IsMeeting>
                  <IsPrivate>false</IsPrivate>
                </CalendarEventDetails>
              </CalendarEvent>
              <CalendarEvent>
                <StartTime>2008-01-21T13:00:00</StartTime>
                <EndTime>2008-01-21T14:00:00</EndTime>
                <BusyType>Busy</BusyType>
                <CalendarEventDetails>
                  <Subject>Meeting2</Subject>
                  <Location>Location2</Location>
                  <IsMeeting>true</IsMeeting>
                  <IsPrivate>false</IsPrivate>
                </CalendarEventDetails>
              </CalendarEvent>
              <CalendarEvent>
                <StartTime>2008-01-21T13:30:00</StartTime>
                <EndTime>2008-01-21T15:00:00</EndTime>
                <BusyType>Busy</BusyType>
                <CalendarEventDetails>
                  <Subject>Secret</Subject>
                  <Location>Somewhere</Location>
                  <IsPrivate>true</IsPrivate>
                </CalendarEventDetails>
              </CalendarEvent>
            </CalendarEventArray>
            <WorkingHours xmlns="http://schemas.microsoft.com/exchange/services/2006/types">
              <TimeZone>
                <Bias>480</Bias>
                <StandardTime><Bias>0</Bias><Time>02:00:00</Time><DayOrder>1</DayOrder><Month>11</Month><DayOfWeek>Sunday</DayOfWeek></StandardTime>
                <DaylightTime><Bias>-60</Bias><Time>02:00:00</Time><DayOrder>2</DayOrder><Month>3</Month><DayOfWeek>Sunday</DayOfWeek></DaylightTime>
              </TimeZone>
              <WorkingPeriodArray>
                <WorkingPeriod>
                  <DayOfWeek>Monday Tuesday Wednesday Thursday Friday</DayOfWeek>
                  <StartTimeInMinutes>480</StartTimeInMinutes>
                  <EndTimeInMinutes>1020</EndTimeInMinutes>
                </WorkingPeriod>
              </WorkingPeriodArray>
            </WorkingHours>
          </FreeBusyView>
        </FreeBusyResponse>
      </FreeBusyResponseArray>
    </GetUserAvailabilityResponse>
  </soap:Body>
</soap:Envelope>"#;

    #[test]
    fn availability_answer() {
        let t = |s: &str| parse_xs_datetime(s).unwrap();
        let a = parse_availability(AVAILABILITY, t("2008-01-21T00:00:00Z")).unwrap();
        assert_eq!(a.slots.len(), 24);
        assert_eq!(a.slots[12], FreeBusy::Oof);
        assert_eq!(a.slots[14], FreeBusy::Busy);
        assert_eq!(a.events.len(), 3);
        assert_eq!(a.events[0].busy, FreeBusy::Tentative);
        // A private event keeps its time but not its subject and location.
        assert_eq!(a.events[2].subject, None);
        assert_eq!(a.events[2].location, None);
        // Tentative is no meeting; of two busy ones the later start wins.
        assert_eq!(a.current_meeting(t("2008-01-21T08:30:00Z")), None);
        let m = a.current_meeting(t("2008-01-21T13:10:00Z")).unwrap();
        assert_eq!(m.subject.as_deref(), Some("Meeting2"));
        assert_eq!(m.location.as_deref(), Some("Location2"));
        assert_eq!(
            a.current_meeting(t("2008-01-21T13:45:00Z"))
                .unwrap()
                .subject,
            None
        );
        assert_eq!(a.current_meeting(t("2008-01-21T15:00:00Z")), None);
        // The working hours are written back as MS-PRES §2.2.2.7.8 carries them.
        let wh = a.working_hours.unwrap().to_xml();
        assert!(wh.starts_with("<WorkingHours xmlns=\"http://schemas.microsoft.com/exchange/services/2006/types\"><TimeZone><Bias>480</Bias><StandardTime><Bias>0</Bias><Time>02:00:00</Time><DayOrder>1</DayOrder><Month>11</Month>"));
        assert!(wh.contains("<WorkingPeriod><DayOfWeek>Monday Tuesday Wednesday Thursday Friday</DayOfWeek><StartTimeInMinutes>480</StartTimeInMinutes><EndTimeInMinutes>1020</EndTimeInMinutes></WorkingPeriod>"));
        commscraft_xml::parse(&wh, &Limits::default()).unwrap();
    }

    #[test]
    fn availability_errors() {
        let bad_digit = AVAILABILITY.replace("000000000000332000000000", "00x");
        assert!(parse_availability(&bad_digit, 0).is_err());
        let error = AVAILABILITY.replace(
            r#"ResponseClass="Success"><ResponseCode>NoError"#,
            r#"ResponseClass="Error"><ResponseCode>ErrorMailRecipientNotFound"#,
        );
        assert!(matches!(
            parse_availability(&error, 0),
            Err(crate::Error::Server(c)) if c == "ErrorMailRecipientNotFound"
        ));
        // Injected markup in a day name does not reach the presence document.
        let evil = AVAILABILITY.replace(
            "Monday Tuesday Wednesday Thursday Friday",
            "Monday&lt;x/&gt;",
        );
        let a = parse_availability(&evil, 0).unwrap();
        assert!(a.working_hours.unwrap().periods.is_empty());
    }

    #[test]
    fn availability_request_window() {
        let now = parse_xs_datetime("2026-11-09T10:17:00Z").unwrap();
        let (start, end) = window(now);
        assert_eq!(start, parse_xs_datetime("2026-11-09T00:00:00Z").unwrap());
        assert_eq!(end - start, 4 * 86_400);
        let r = availability_request("alice@contoso.com", start, end).unwrap();
        assert!(r.contains(
            "<StartTime>2026-11-09T00:00:00</StartTime><EndTime>2026-11-13T00:00:00</EndTime>"
        ));
        assert!(r.contains("<MergedFreeBusyIntervalInMinutes>15</MergedFreeBusyIntervalInMinutes><RequestedView>DetailedMerged</RequestedView>"));
        assert!(r.contains("<Address>alice@contoso.com</Address>"));
        commscraft_xml::parse(&r, &Limits::default()).unwrap();
        assert!(availability_request("a<b@c.com", start, end).is_err());
    }

    /// [MS-OXWOOF] §4.2 (pp. 28–29).
    const OOF: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <GetUserOofSettingsResponse xmlns="http://schemas.microsoft.com/exchange/services/2006/messages">
      <ResponseMessage ResponseClass="Success"><ResponseCode>NoError</ResponseCode></ResponseMessage>
      <OofSettings xmlns="http://schemas.microsoft.com/exchange/services/2006/types">
        <OofState>Enabled</OofState>
        <ExternalAudience>All</ExternalAudience>
        <Duration>
          <StartTime>2008-02-01T00:00:00</StartTime>
          <EndTime>2008-02-02T00:00:00</EndTime>
        </Duration>
        <InternalReply>
          <Message>I am out of office. This is my internal reply.</Message>
        </InternalReply>
        <ExternalReply>
          <Message>I am out of office. This is my external reply.</Message>
        </ExternalReply>
      </OofSettings>
      <AllowExternalOof>All</AllowExternalOof>
    </GetUserOofSettingsResponse>
  </soap:Body>
</soap:Envelope>"#;

    #[test]
    fn oof_answer() {
        let t = |s: &str| parse_xs_datetime(s).unwrap();
        let o = parse_oof(OOF).unwrap();
        assert_eq!(o.state, OofState::Enabled);
        assert_eq!(o.message, "I am out of office. This is my internal reply.");
        assert!(o.active(0));
        let s = parse_oof(&OOF.replace(">Enabled<", ">Scheduled<")).unwrap();
        assert!(!s.active(t("2008-01-31T23:59:59Z")));
        assert!(s.active(t("2008-02-01T12:00:00Z")));
        assert!(!s.active(t("2008-02-02T00:00:00Z")));
        let d = parse_oof(&OOF.replace(">Enabled<", ">Disabled<")).unwrap();
        assert!(!d.active(t("2008-02-01T12:00:00Z")));
        assert!(parse_oof(&OOF.replace(">Enabled<", ">On<")).is_err());
        let r = oof_request("user@example.com").unwrap();
        assert!(r.contains("<GetUserOofSettingsRequest xmlns=\"http://schemas.microsoft.com/exchange/services/2006/messages\"><Mailbox xmlns=\"http://schemas.microsoft.com/exchange/services/2006/types\"><Address>user@example.com</Address></Mailbox>"));
    }

    /// Exchange keeps replies as HTML; the note gets plain text.
    #[test]
    fn oof_message_as_plain_text() {
        let html = "<html><head><style>p{color:red}</style><title>x</title></head><body><div>Out until&nbsp;Monday &amp; back<br>on Tuesday</div><!-- <b>hidden</b> --><script>alert(1)</script>&#x263A;&#9;\u{7}</body></html>";
        assert_eq!(plain_text(html), "Out until Monday & back on Tuesday ☺");
        assert_eq!(plain_text("a &unknown; b &lt;"), "a &unknown; b <");
        assert_eq!(plain_text(&"x".repeat(5000)).len(), MAX_MESSAGE);
        assert_eq!(plain_text("<b"), "");
    }
}
