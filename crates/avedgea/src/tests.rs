use super::*;

const MRAS: &str =
    "sip:relay.contoso.com@contoso.com;gruu;opaque=srvr:MRAS:OKPDbAVxIEKtPh2g624vPAAA";

/// [MS-AVEDGEA] §4.1.2 (p. 26) response body.
const RESPONSE_41: &str = r#"<?xml version="1.0"?>
<response xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" requestID="990512" version="2.0" serverVersion="3.0" to="sip:relay.contoso.com@contoso.com;gruu;opaque=srvr:MRAS:OKPDbAVxIEKtPh2g624vPAAA" from="sip:client@contoso.com" reasonPhrase="OK" xmlns="http://schemas.microsoft.com/2006/09/sip/mrasp">
  <credentialsResponse credentialsRequestID="990512">
<credentials>
<username>AQAgAIaoZr4EM8gBrxTJGY83uqdEgRXUunam2c+RID/vAJeJSL4YINbAYMvRAHeANv+Zew==</username>
      <password>35yqSF/p3A8gWXFHOC9YJA2kdvY=</password>
      <duration>480</duration>
    </credentials>
    <mediaRelayList>
      <mediaRelay>
        <location>intranet</location>
        <hostName>relay.contoso.com</hostName>
        <udpPort>3478</udpPort>
        <tcpPort>443</tcpPort>
      </mediaRelay>
    </mediaRelayList>
  </credentialsResponse>
</response>"#;

#[test]
fn request_body() {
    let r = request(990512, "sip:client@contoso.com", MRAS).unwrap();
    let root = commscraft_xml::parse(&r, &Limits::default()).unwrap();
    assert_eq!(
        (root.name.as_str(), root.namespace.as_deref()),
        ("request", Some(NAMESPACE))
    );
    assert_eq!(root.attr("version"), Some("2.0"));
    assert_eq!(root.attr("to"), Some(MRAS));
    let cr = root.child("credentialsRequest").unwrap();
    assert_eq!(cr.attr("credentialsRequestID"), Some("990512"));
    assert_eq!(
        cr.child("identity").unwrap().text(),
        "sip:client@contoso.com"
    );
    assert_eq!(cr.child("duration").unwrap().text(), "480");
    assert!(
        cr.child("location").is_none(),
        "AV-05: both relay locations"
    );
    assert!(request(1, "sip:a@b\"><x", MRAS).is_err());
}

#[test]
fn specification_responses() {
    let c = parse_response(RESPONSE_41, 990512).unwrap();
    assert_eq!(&*c.password, "35yqSF/p3A8gWXFHOC9YJA2kdvY=");
    assert!(c.username.starts_with("AQAgAIaoZr4E"));
    assert_eq!(c.duration_minutes, 480);
    assert_eq!(
        c.relays,
        vec![Relay {
            location: Some(Location::Intranet),
            host: RelayHost::Name("relay.contoso.com".into()),
            udp_port: 3478,
            tcp_port: 443
        }]
    );
    assert!(!format!("{c:?}").contains("35yq"), "password never printed");
    // §4.2.2 (p. 28): direct IP addresses, IPv4 and IPv6.
    let direct = RESPONSE_41
        .replace("<location>intranet</location>\n        <hostName>relay.contoso.com</hostName>", "<location>internet</location><directIPAddress>192.0.2.254</directIPAddress>")
        .replace("</mediaRelay>\n    </mediaRelayList>", "</mediaRelay><mediaRelay><location>internet</location><directIPAddress>2001:0DB8::943c:fa53</directIPAddress><udpPort>3478</udpPort><tcpPort>443</tcpPort></mediaRelay></mediaRelayList>");
    let c = parse_response(&direct, 990512).unwrap();
    assert_eq!(c.relays.len(), 2);
    assert_eq!(
        c.relays[1].host,
        RelayHost::Address("2001:db8::943c:fa53".parse().unwrap())
    );
}

#[test]
fn hostile_or_failed_responses() {
    assert_eq!(
        parse_response(RESPONSE_41, 1).err(),
        Some(Error::WrongDocument("requestID"))
    );
    let refused = RESPONSE_41.replace("reasonPhrase=\"OK\"", "reasonPhrase=\"Forbidden\u{7}\"");
    assert!(parse_response(&refused, 990512).is_err());
    let forbidden = RESPONSE_41.replace("reasonPhrase=\"OK\"", "reasonPhrase=\"Forbidden\"");
    assert_eq!(
        parse_response(&forbidden, 990512).err(),
        Some(Error::Refused("Forbidden".into()))
    );
    let bad_host = RESPONSE_41.replace(
        "relay.contoso.com</hostName>",
        "relay contoso;rm</hostName>",
    );
    assert_eq!(
        parse_response(&bad_host, 990512).err(),
        Some(Error::BadValue("hostName"))
    );
    let bad_pw = RESPONSE_41.replace("35yqSF/p3A8gWXFHOC9YJA2kdvY=", "pw\"with quotes");
    assert!(parse_response(&bad_pw, 990512).is_err());
    let zero_port = RESPONSE_41.replace("<udpPort>3478</udpPort>", "<udpPort>0</udpPort>");
    assert_eq!(
        parse_response(&zero_port, 990512).err(),
        Some(Error::BadValue("port"))
    );
    let long = RESPONSE_41.replace("<duration>480</duration>", "<duration>99999</duration>");
    assert_eq!(parse_response(&long, 990512).unwrap().duration_minutes, 480);
    assert!(matches!(
        parse_response("<!DOCTYPE x><response/>", 1),
        Err(Error::Xml(_))
    ));
}
