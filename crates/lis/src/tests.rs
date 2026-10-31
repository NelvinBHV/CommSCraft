use super::*;
use commscraft_xml::parse;

/// E9-03 ([MS-E911WS] §3.1.4.1.1.1, pp. 16–17; example §4 p. 23): the request with Entity and
/// the identifiers in the schema's order, in the schema's namespace (K-E9-1).
#[test]
fn request_document() {
    let ids = NetworkIds {
        mac: Some("12-22-22-22-22-22".into()),
        subnet: Some("192.168.0.0".parse().unwrap()),
        ip: Some("192.168.0.244".parse().unwrap()),
    };
    let x = get_locations_request("sip:voip_911_user1@contoso.com", &ids).unwrap();
    let r = parse(&x, &Limits::default()).unwrap();
    let req = find(&r, &["Body", "GetLocationsRequest"]).unwrap();
    assert_eq!(req.namespace.as_deref(), Some(NS_LIS));
    let names: Vec<&str> = req.elements().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["Entity", "MAC", "SubnetID", "IP"]);
    assert_eq!(req.child("IP").unwrap().text(), "192.168.0.244");
    // Bad values never leave the client.
    assert!(get_locations_request("sip:a@b\"><x", &NetworkIds::default()).is_err());
    assert!(get_locations_request(
        &format!("sip:{}@b.c", "a".repeat(64)),
        &NetworkIds::default()
    )
    .is_err());
    let bad_mac = NetworkIds {
        mac: Some("12:22:22:22:22:22".into()),
        ..NetworkIds::default()
    };
    assert!(get_locations_request("sip:a@b.c", &bad_mac).is_err());
}

const ANSWER: &str = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body>
<GetLocationsResponse xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <ReturnCode>200</ReturnCode>
  <presenceList>
    <presence entity="sip:voip_911_user1@contoscovdomain.com" xmlns="urn:ietf:params:xml:ns:pidf">
      <tuple id="_LIS:0">
        <status>
          <geopriv xmlns="urn:ietf:params:xml:ns:pidf:geopriv10">
            <location-info>
              <civicAddress xmlns="urn:ietf:params:xml:ns:pidf:geopriv10:civicAddr">
                <country>US</country><A1>WA</A1><A3>Redmond</A3><PRD /><RD>163rd</RD><STS>Ave</STS>
                <POD>NE</POD><HNO>3910</HNO><HNS /><LOC>30/3351</LOC><NAM>Microsoft</NAM><PC>98052</PC>
              </civicAddress>
            </location-info>
          </geopriv>
        </status>
      </tuple>
    </presence>
  </presenceList>
</GetLocationsResponse></soap:Body></soap:Envelope>"#;

/// E9-04 (§2.2.3–2.2.5, example §4 p. 23): the answer's return code and civic address.
#[test]
fn answer_example() {
    let (code, locs) = parse_get_locations(ANSWER).unwrap();
    assert_eq!((code, locs.len()), (200, 1));
    let l = &locs[0];
    assert_eq!(
        (
            l.country.as_str(),
            l.a3.as_str(),
            l.hno.as_str(),
            l.loc.as_str()
        ),
        ("US", "Redmond", "3910", "30/3351")
    );
    assert!(!l.manual);
    assert_eq!(
        l.summary(),
        "163rd Ave NE, 3910, 30/3351, Microsoft, 98052 Redmond, WA US"
    );
    let not_found = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body><GetLocationsResponse><ReturnCode>404</ReturnCode></GetLocationsResponse></soap:Body></soap:Envelope>"#;
    assert_eq!(parse_get_locations(not_found).unwrap(), (404, Vec::new()));
}

/// Hostile answers: wrong document, missing code, entities, too many locations.
#[test]
fn hostile_answers() {
    assert!(parse_get_locations("<x/>").is_err());
    assert!(parse_get_locations(r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body><GetLocationsResponse/></soap:Body></soap:Envelope>"#).is_err());
    assert!(parse_get_locations(&format!("<!DOCTYPE x [<!ENTITY a \"b\">]>{ANSWER}")).is_err());
    let tuple = ANSWER
        .split("<tuple")
        .nth(1)
        .and_then(|t| t.split("</tuple>").next())
        .map(|t| format!("<tuple{t}</tuple>"))
        .unwrap();
    let many = ANSWER.replace(&tuple, &tuple.repeat(40));
    assert_eq!(parse_get_locations(&many).unwrap().1.len(), MAX_LOCATIONS);
    let long = ANSWER.replace("Microsoft", &"m".repeat(1000));
    assert_eq!(
        parse_get_locations(&long).unwrap().1[0].nam.len(),
        MAX_FIELD
    );
}

/// RE-67 ([MS-SIPRE] §2.2.23, example §4.14 p. 94): the PIDF-LO with civic address, usage rules,
/// `Manual` for a typed address, the Microsoft extension and the policy tag; values escaped.
#[test]
fn pidf_lo_document() {
    let mut loc = parse_get_locations(ANSWER).unwrap().1.remove(0);
    loc.nam = "Contoso <Corp> & Co".into();
    loc.manual = true;
    let ext = Extension {
        conference_uri: Some("sip:+14255550199@contoso.com;user=phone".into()),
        conference_mode: Some("twoway".into()),
        tag_id: Some("user-tagid".into()),
    };
    let x = pidf_lo("sip:voip_911_user1@contoso.com", &loc, &ext, 1_136_913_107).unwrap();
    let r = parse(&x, &Limits::default()).unwrap();
    assert_eq!(
        (r.name.as_str(), r.namespace.as_deref()),
        ("presence", Some(NS_PIDF))
    );
    assert_eq!(r.attr("entity"), Some("sip:voip_911_user1@contoso.com"));
    let status = find(&r, &["tuple", "status"]).unwrap();
    let gp = status.child("geopriv").unwrap();
    let ca = find(gp, &["location-info", "civicAddress"]).unwrap();
    assert_eq!(ca.namespace.as_deref(), Some(NS_CA));
    assert_eq!(ca.child("NAM").unwrap().text(), "Contoso <Corp> & Co");
    assert!(ca.child("PRD").is_none(), "empty fields are left out");
    assert_eq!(gp.child("method").unwrap().text(), "Manual");
    let ms = status.child("msftE911PidfExtn").unwrap();
    assert_eq!(ms.child("ConferenceMode").unwrap().text(), "twoway");
    assert_eq!(
        ms.child("LocationPolicyTagID")
            .unwrap()
            .namespace
            .as_deref(),
        Some(NS_TAG)
    );
    assert_eq!(
        find(&r, &["tuple", "timestamp"]).unwrap().text(),
        "2006-01-10T17:11:47Z"
    );
    assert_eq!(
        content_id("sip:voip_911_user1@contoso.com"),
        "voip_911_user1@contoso.com"
    );
}

/// E9-08 (§3.1.4.2, pp. 18–20): GetLocationsInCity request with escaped state and city and the
/// schema bounds; the answer is read like GetLocations, under its own element name.
#[test]
fn locations_in_city() {
    let x = get_locations_in_city_request("sip:alice@contoso.com", "US", "WA", "Redmond & <Co>")
        .unwrap();
    assert!(
        x.contains("<GetLocationsInCityRequest xmlns=\"http://schemas.microsoft.com/2007/06/lis\"")
            || x.contains("<GetLocationsInCityRequest xmlns="),
        "{x}"
    );
    assert!(x.contains("<Entity>sip:alice@contoso.com</Entity><Country>US</Country><State>WA</State><City>Redmond &amp; &lt;Co&gt;</City>"), "{x}");
    for (c, s, city) in [
        ("us", "WA", "X"),
        ("USA", "WA", "X"),
        ("US", "W", "X"),
        ("US", "WA", ""),
        ("US", "WA", "a\r\nb"),
    ] {
        assert!(
            get_locations_in_city_request("sip:alice@contoso.com", c, s, city).is_err(),
            "{c} {s} {city:?}"
        );
    }
    assert!(
        get_locations_in_city_request("sip:alice@contoso.com", "US", "WA", &"x".repeat(65))
            .is_err()
    );
    let answer = ANSWER.replace("GetLocationsResponse", "GetLocationsInCityResponse");
    let (code, locs) = parse_get_locations_in_city(&answer).unwrap();
    assert_eq!(code, 200);
    assert!(!locs.is_empty());
    assert!(
        parse_get_locations_in_city(ANSWER).is_err(),
        "the other operation's answer"
    );
}
