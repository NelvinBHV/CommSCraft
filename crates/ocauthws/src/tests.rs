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
