use super::*;

const DEVICE: &str = "{161CCE75-E0C7-5F60-BDD1-054099725B0B}";

fn answer(class: &str, disposition: &str, entity: &str, cert_b64: &str) -> String {
    format!(
        "<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\"><s:Body>\
<GetAndPublishCertResponse ResponseClass=\"{class}\" DeviceId=\"{DEVICE}\" Entity=\"{entity}\" xmlns=\"http://schemas.microsoft.com/OCS/AuthWebServices/\">\
<RequestSecurityTokenResponse xmlns=\"http://docs.oasis-open.org/ws-sx/ws-trust/200512/\">\
<TokenType>{X509V3}</TokenType>\
<DispositionMessage xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/windows/pki/2009/01/enrollment\">{disposition}</DispositionMessage>\
<RequestedSecurityToken><BinarySecurityToken EncodingType=\"{BASE64BINARY}\" ValueType=\"{X509V3}\" xmlns=\"{NS_WSSE}\">\n  {cert_b64}\n</BinarySecurityToken></RequestedSecurityToken>\
</RequestSecurityTokenResponse></GetAndPublishCertResponse></s:Body></s:Envelope>"
    )
}

/// AW-11: the request of the example §4.1.1 (p. 60): device and entity attributes, X509v3 token
/// type, the CSR as base64 with the PKCS10 value type.
#[test]
fn request_form() {
    let r = certificate_request(
        "sip:alice@contoso.com",
        "161cce75-e0c7-5f60-bdd1-054099725b0b",
        &[0x30, 0x03, 0x02, 0x01, 0x00],
        "4792483c-70b5-4591-b138-1a503a26d65b",
    )
    .unwrap();
    assert!(r.contains(&format!(
        "<GetAndPublishCert DeviceId=\"{DEVICE}\" Entity=\"alice@contoso.com\""
    )));
    assert!(r.contains(&format!("ValueType=\"{PKCS10}\"")));
    assert!(r.contains(">MAMCAQA=</BinarySecurityToken>"));
    assert!(r.contains(">4792483c-70b5-4591-b138-1a503a26d65b</RequestID>"));
    // K-AW-2: no SOAP header.
    assert!(!r.contains("Header"));
    commscraft_xml::parse(&r, &Limits::default()).unwrap();
    assert_eq!(
        certificate_request("sip:a\"@b.com", DEVICE, &[], DEVICE),
        Err(Error::BadEntity)
    );
    assert_eq!(
        certificate_request("sip:a@b.com", "not-a-guid", &[], DEVICE),
        Err(Error::BadDeviceId)
    );
}

/// AW-13: an issued certificate for the entity asked for is taken over; anything else is refused.
#[test]
fn certificate_answers() {
    // A minimal DER SEQUENCE stands for the certificate.
    let ok = answer("Success", "Issued", "alice@contoso.com", "MAMCAQA=");
    assert_eq!(
        parse_certificate_answer(&ok, "sip:alice@contoso.com").unwrap(),
        [0x30, 0x03, 0x02, 0x01, 0x00]
    );
    assert!(matches!(
        parse_certificate_answer(
            &answer("Error", "", "alice@contoso.com", ""),
            "sip:alice@contoso.com"
        ),
        Err(Error::Refused(_))
    ));
    assert!(matches!(
        parse_certificate_answer(
            &answer("Success", "Pending", "alice@contoso.com", "MAMCAQA="),
            "sip:alice@contoso.com"
        ),
        Err(Error::Refused(_))
    ));
    assert_eq!(
        parse_certificate_answer(
            &answer("Success", "Issued", "mallory@contoso.com", "MAMCAQA="),
            "sip:alice@contoso.com"
        ),
        Err(Error::WrongDocument("Entity"))
    );
    // Not a whole DER SEQUENCE.
    assert!(parse_certificate_answer(
        &answer("Success", "Issued", "alice@contoso.com", "MAQCAQA="),
        "sip:alice@contoso.com"
    )
    .is_err());
    assert!(parse_certificate_answer("<x/>", "sip:alice@contoso.com").is_err());
}

mod webticket_tests {
    use crate::webticket::*;
    use commscraft_xml::Limits;

    /// Security: `Debug` of a ticket never shows the assertion, only its length.
    #[test]
    fn debug_hides_the_assertion() {
        let t = WebTicket {
            assertion: "<saml:Assertion>SECRET</saml:Assertion>".into(),
            expires: Some(7),
        };
        let shown = format!("{t:?} {t:#?}");
        assert!(!shown.contains("SECRET") && !shown.contains("saml"));
        assert!(shown.contains("39 bytes redacted") && shown.contains("Some(7)"));
    }

    /// [MS-OCAUTHWS] §4.2.2 (pp. 62–64), shortened (signature and key info cut).
    const ANSWER: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <RequestSecurityTokenResponseCollection xmlns="http://docs.oasis-open.org/ws-sx/ws-trust/200512">
      <RequestSecurityTokenResponse Context="2fdf3b92-4341-4eeb-b898-44ef4994cd55">
        <TokenType>http://docs.oasis-open.org/wss/oasis-wss-saml-token-profile-1.1#SAMLV1.1</TokenType>
        <RequestedSecurityToken>
           <saml:Assertion MajorVersion="1" MinorVersion="1" AssertionID="SamlSecurityToken-7e62744e-bb8b-4f79-a4c8-623c866adf8c" Issuer="https://Server.Vdomain.com/webticket/webticketservice.svc" IssueInstant="2010-02-11T21:40:47.004Z" xmlns:saml="urn:oasis:names:tc:SAML:1.0:assertion"><saml:Conditions NotBefore="2010-02-11T21:40:47.004Z" NotOnOrAfter="2010-02-11T22:40:47.004Z"><saml:AudienceRestrictionCondition><saml:Audience>https://pool0.vdomain.com/</saml:Audience></saml:AudienceRestrictionCondition></saml:Conditions></saml:Assertion>
        </RequestedSecurityToken>
        <Lifetime>
          <Created xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">2010-02-11T21:40:47.0048342Z</Created>
          <Expires xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">2010-02-11T22:40:47.0048342Z</Expires>
        </Lifetime>
      </RequestSecurityTokenResponse>
    </RequestSecurityTokenResponseCollection>
  </s:Body>
</s:Envelope>"#;

    #[test]
    fn issue_token() {
        let r = issue_token_request(
            "https://pool0.vdomain.com/GroupExpansion/Service.svc",
            "2fdf3b92-4341-4eeb-b898-44ef4994cd55",
            &[7; 32],
        )
        .unwrap();
        let e = commscraft_xml::parse(&r, &Limits::default()).unwrap();
        let rst = e
            .child("Body")
            .unwrap()
            .child("RequestSecurityToken")
            .unwrap();
        assert_eq!(
            rst.attr("Context"),
            Some("2fdf3b92-4341-4eeb-b898-44ef4994cd55")
        );
        assert_eq!(
            rst.child("KeyType").unwrap().text(),
            "http://docs.oasis-open.org/ws-sx/ws-trust/200512/SymmetricKey"
        );
        assert_eq!(
            rst.child("Entropy")
                .unwrap()
                .child("BinarySecret")
                .unwrap()
                .text(),
            "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc="
        );
        assert!(issue_token_request("https://a.b/", "1", &[0; 8]).is_err());
        assert!(
            r.contains("<Address>https://pool0.vdomain.com/GroupExpansion/Service.svc</Address>")
        );
        assert!(issue_token_request("http://x/", "1", &[0; 16]).is_err());
        let t = parse_issue_token_answer(ANSWER).unwrap();
        // Byte for byte as the issuer sent it.
        assert!(t
            .assertion
            .starts_with("<saml:Assertion MajorVersion=\"1\""));
        assert!(t.assertion.ends_with("</saml:Assertion>"));
        assert!(ANSWER.contains(&t.assertion));
        assert_eq!(
            t.expires,
            commscraft_xml::parse_xs_datetime("2010-02-11T22:40:47Z")
        );
        // Something else in its place is refused.
        let other = ANSWER.replace("saml:Assertion", "saml:Attribute");
        assert!(parse_issue_token_answer(&other).is_err());
    }

    #[test]
    fn secured_request() {
        let t = parse_issue_token_answer(ANSWER).unwrap();
        let e = secured_envelope(
            "\"http://schemas.microsoft.com/acd/2007/12/Microsoft.Rtc.Acd.Clients.Proxy/IsAgent\"",
            "https://pool0.contoso.com/RgsClients/AgentService.svc/webticket_bearer",
            "9e1e3a5c-0d36-4f1c-9a52-2b8a4a1b2c3d",
            &t,
            "<IsAgent xmlns=\"http://schemas.microsoft.com/acd/2007/12/Microsoft.Rtc.Acd.Clients.Proxy\"/>",
        )
        .unwrap();
        let x = commscraft_xml::parse(&e, &Limits::default()).unwrap();
        let h = x.child("Header").unwrap();
        assert_eq!(
            h.child("Action").unwrap().text(),
            "http://schemas.microsoft.com/acd/2007/12/Microsoft.Rtc.Acd.Clients.Proxy/IsAgent"
        );
        assert!(h.child("Security").unwrap().child("Assertion").is_some());
        assert!(e.contains(&t.assertion));
    }

    #[test]
    fn metadata() {
        let wsdl = r#"<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/" xmlns:wsp="http://schemas.xmlsoap.org/ws/2004/09/policy" xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd" xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/" xmlns:tns="urn:x">
<wsp:Policy wsu:Id="Bearer_Svc_policy"><wsp:ExactlyOne><wsp:All><sp:SignedSupportingTokens xmlns:sp="http://docs.oasis-open.org/ws-sx/ws-securitypolicy/200702"><wsp:Policy><sp:IssuedToken><Issuer xmlns="http://docs.oasis-open.org/ws-sx/ws-securitypolicy/200702"><Address xmlns="http://www.w3.org/2005/08/addressing">https://pool0.contoso.com/WebTicket/WebTicketService.svc</Address></Issuer></sp:IssuedToken></wsp:Policy></sp:SignedSupportingTokens></wsp:All></wsp:ExactlyOne></wsp:Policy>
<wsdl:service name="Svc"><wsdl:port name="Bearer_Svc" binding="tns:Bearer_Svc"><soap:address location="https://pool0.contoso.com/RgsClients/AgentService.svc/webticket_bearer"/></wsdl:port>
<wsdl:port name="Plain" binding="tns:Plain"><soap:address location="http://pool0.contoso.com/x"/></wsdl:port></wsdl:service></wsdl:definitions>"#;
        let e = parse_metadata(wsdl).unwrap();
        assert_eq!(
            e,
            vec![Endpoint {
                address: "https://pool0.contoso.com/RgsClients/AgentService.svc/webticket_bearer"
                    .into(),
                issuer: Some("https://pool0.contoso.com/WebTicket/WebTicketService.svc".into()),
            }]
        );
    }
}
