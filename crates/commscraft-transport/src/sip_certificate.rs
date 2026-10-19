//! Narrow, fail-closed RFC 5922 §§7.1–7.3 SIP domain identity extraction.
//! This never validates a chain, trusted root, validity time, EKU, or key
//! possession on its own. It is ONLY called after native_tls has completed
//! platform TLS/PKIX verification for the original SIP domain.
//! Conservative subset: no legacy CN fallback and no Unicode U-label IDNs.
//! No network, SIP, registration, Kerberos, or credential API is exposed.

use x509_parser::extensions::GeneralName;
use x509_parser::parse_x509_certificate;

const MAX_CERT_DER: usize = 64 * 1024;
const MAX_SAN_NAMES: usize = 128;
// Conservative local bounds for untrusted certificate URI SAN parsing.
const MAX_SIP_SAN_URI_BYTES: usize = 4096;
const MAX_SIP_SAN_URI_COMPONENTS: usize = 64;

fn ascii_sip_domain(name: &str) -> Option<&str> {
    // RFC 5922: numeric IP literals cannot stand in for DNS SIP domains.
    if name.parse::<std::net::Ipv4Addr>().is_ok() { return None; }
    if name.len() > 253 || name.is_empty() || !name.is_ascii()
        || !name.contains('.') || name.contains('*')
        || name.split('.').any(|label| label.is_empty() || label.len() > 63
            || label.starts_with('-') || label.ends_with('-')
            || !label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    {
        return None;
    }
    // RFC 9549 §2.3 / RFC 5890: a syntactically plausible "xn--" label
    // is NOT automatically a valid IDNA A-label. Use strict IDNA decoding
    // and require an unchanged ASCII representation (apart from case).
    // Unicode U-labels must be converted at a separate, trusted ingress.
    if !idna::domain_to_ascii_strict(name)
        .is_ok_and(|canonical| canonical.eq_ignore_ascii_case(name))
    {
        return None;
    }
    if !crate::idna_policy::rfc9549_a_label_symbols_allowed(name) {
        return None;
    }
    Some(name)
}

/// RFC 5922 §7.1: a URI SAN contributes a SIP *domain* only when it has
/// scheme "sip" (case insensitive) and no userinfo component.
/// RFC 5922 §7.2 compares host only and ignores transport/URI parameters.
/// We intentionally support the DNS-host SIP URI subset, not IPv6 literals.
fn sip_uri_host(uri: &str) -> Option<&str> {
    if uri.len() > MAX_SIP_SAN_URI_BYTES { return None; }
    let (scheme, remainder) = uri.split_once(':')?;
    if !scheme.eq_ignore_ascii_case("sip") || remainder.contains('@')
        || remainder.starts_with("//")
    {
        return None;
    }
    // SIP URIs do not admit URI fragments. Reject controls or whitespace
    // in the *whole* SAN URI, including params/headers that we do not match.
    // Otherwise a malformed certificate identity could be accepted simply
    // because its host prefix looks valid.
    if !uri.is_ascii() || remainder.bytes().any(|b| b <= 0x20 || b == 0x7f)
        || remainder.contains('#')
        || remainder.contains('\\')
    {
        return None;
    }
    // RFC 3261 §25.1: escaped URI bytes require '%' followed by exactly
    // two HEXDIG characters, including in ignored URI parameters/headers.
    // Incomplete or invalid escapes cannot authorize a certificate identity.
    let bytes = remainder.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return None;
            }
            // Conservative SAN identity policy: escaped controls cannot
            // authorize a peer even when percent-triplet syntax is valid.
            let hex = |b: u8| -> u8 {
                if b.is_ascii_digit() { b - b'0' }
                else { b.to_ascii_lowercase() - b'a' + 10 }
            };
            let decoded = (hex(bytes[index + 1]) << 4) | hex(bytes[index + 2]);
            if decoded <= 0x1f || decoded == 0x7f { return None; }
            index += 3;
        } else {
            index += 1;
        }
    }
    // RFC 3261 (June 2002) §25.1: apply the ABNF character classes to
    // every URI parameter and header, not just to the host and percent escapes.
    // RFC 5922 (June 2010) §§7.1-7.3: malformed SIP URI SANs cannot
    // authorize a TLS peer identity or enable fallback to a DNS SAN.
    fn component_chars_valid(value: &str, extra: &[u8]) -> bool {
        let bytes = value.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b == b'%' {
                // Percent-triplet syntax and escaped control bytes were
                // checked for the entire URI above.
                i += 3;
                continue;
            }
            if !(b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b)
                || extra.contains(&b)) {
                return false;
            }
            i += 1;
        }
        true
    }
    // paramchar = param-unreserved / unreserved / escaped;
    // param-unreserved = "[" / "]" / "/" / ":" / "&" / "+" / "$".
    if let Some((_, params)) = remainder.split('?').next()?.split_once(';') {
        for param in params.split(';') {
            let (name, value) = param.split_once('=').unwrap_or((param, ""));
            if !component_chars_valid(name, b"[]/:&+$")
                || !component_chars_valid(value, b"[]/:&+$") {
                return None;
            }
        }
    }
    // hname/hvalue = 1*(hnv-unreserved / unreserved / escaped);
    // hnv-unreserved = "[" / "]" / "/" / "?" / ":" / "+" / "$".
    if let Some((_, headers)) = remainder.split_once('?') {
        for header in headers.split('&') {
            let Some((name, value)) = header.split_once('=') else { return None; };
            if !component_chars_valid(name, b"[]/?:+$")
                || !component_chars_valid(value, b"[]/?:+$") {
                return None;
            }
        }
    }
    // RFC 3261 §25.1: URI headers are nonempty hname=hvalue pairs.
    if let Some((_, headers)) = remainder.split_once('?') {
        if headers.split('&').count() > MAX_SIP_SAN_URI_COMPONENTS { return None; }
        if headers.is_empty() || headers.split('&').any(|header| {
            let Some((name, _)) = header.split_once('=') else { return true; };
            name.is_empty()
        }) { return None; }
    }
    // RFC 3261 §19.1.1: a SIP URI parameter name MUST NOT occur
    // more than once. Apply case-insensitive matching at the identity
    // boundary; ignore the separate '?' header portion here.
    let authority_and_parameters = remainder.split('?').next()?;
    if let Some((_, parameters)) = authority_and_parameters.split_once(';') {
        if parameters.split(';').count() > MAX_SIP_SAN_URI_COMPONENTS { return None; }
        let mut seen: Vec<String> = Vec::new();
        for parameter in parameters.split(';') {
            let name = parameter.split('=').next()?;
            if name.is_empty() || parameter.matches('=').count() > 1 {
                return None;
            }
            // RFC 3261 §19.1.4: unreserved URI characters and their
            // percent-encoded forms compare as equal. Normalize names before
            // checking the §19.1.1 no-duplicate requirement.
            let bytes = name.as_bytes();
            let mut canonical = String::with_capacity(name.len());
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'%' && i + 2 < bytes.len() {
                    let hex = |b: u8| (b as char).to_digit(16);
                    let decoded = (hex(bytes[i + 1])? * 16 + hex(bytes[i + 2])?) as u8;
                    if decoded.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&decoded) {
                        canonical.push(decoded as char);
                    } else {
                        // Percent-triplet hex case does not change the octet.
                        // Keep reserved bytes encoded; canonicalize hex digits.
                        canonical.push('%');
                        canonical.push((bytes[i + 1] as char).to_ascii_uppercase());
                        canonical.push((bytes[i + 2] as char).to_ascii_uppercase());
                    }
                    i += 3;
                } else {
                    canonical.push(bytes[i] as char);
                    i += 1;
                }
            }
            if !canonical.is_empty()
                && seen.iter().any(|old| old.eq_ignore_ascii_case(&canonical)) {
                return None;
            }
            seen.push(canonical);
        }
    }
    let host_and_port = remainder.split([';', '?']).next()?;
    let (host, port) = match host_and_port.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (host_and_port, None),
    };
    if let Some(port) = port {
        if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit())
            || !matches!(port.parse::<u16>(), Ok(1..=u16::MAX))
        {
            return None;
        }
    }
    ascii_sip_domain(host)
}

/// Only valid SIP URI SANs take precedence over DNS SANs, even when the
/// valid SIP URIs *do not match* the expected domain. Otherwise a conflicting
/// DNSName would silently elevate a server identity (RFC 5922 §7.1).
fn match_sip_sans(names: &[GeneralName<'_>], expected_domain: &str) -> bool {
    let Some(reference) = ascii_sip_domain(expected_domain) else {
        return false;
    };
    if names.is_empty() || names.len() > MAX_SAN_NAMES {
        return false;
    }
    let mut has_valid_sip_uri = false;
    let mut uri_matches = false;
    let mut dns_matches = false;
    for name in names {
        match name {
            GeneralName::URI(uri) => {
                // Never downgrade to a DNS SAN if a malformed SIP-scheme
                // URI could conceal a conflicting SIP domain identity.
                // Other URI schemes carry no SIP domain (RFC 5922 §7.1).
                let sip_scheme = uri.split_once(':').is_some_and(|(scheme, _)|
                    scheme.eq_ignore_ascii_case("sip")
                );
                if uri.len() > MAX_SIP_SAN_URI_BYTES {
                    return false;
                }
                if let Some(host) = sip_uri_host(uri) {
                    has_valid_sip_uri = true;
                    if host.eq_ignore_ascii_case(reference) {
                        uri_matches = true;
                    }
                } else if sip_scheme {
                    return false;
                }
            }
            GeneralName::DNSName(dns)
                if ascii_sip_domain(dns)
                    .is_some_and(|name| name.eq_ignore_ascii_case(reference)) =>
            {
                dns_matches = true;
            }
            _ => {} // IP addresses, RFC822 and other names aren't SIP domains
        }
    }
    if has_valid_sip_uri { uri_matches } else { dns_matches }
}

/// Analyze the *actual* peer certificate returned from the TLS stream.
/// No callers can assert "verified" using an arbitrary boolean or SAN list.
/// Success here is an extra name check, NOT independent chain validation.
pub(super) fn matches_sip_identity(leaf_der: &[u8], expected_domain: &str) -> bool {
    if leaf_der.is_empty() || leaf_der.len() > MAX_CERT_DER
        || ascii_sip_domain(expected_domain).is_none()
    {
        return false;
    }
    let Ok((remainder, cert)) = parse_x509_certificate(leaf_der) else {
        return false;
    };
    if !remainder.is_empty() {
        return false;
    }
    // RFC 5280 §4.2: a certificate MUST NOT contain more than one
    // extension with the same OID. Refuse ambiguous X.509 extension sets
    // *before* reading the SAN identity. x509-parser's extensions_map()
    // returns DuplicateExtensions, instead of silently selecting the first.
    if cert.extensions_map().is_err() {
        return false;
    }
    // RFC 5922 permits CN fallback only if SAN is *absent*. We intentionally
    // reject CN-only certificates instead of using platform-specific heuristics.
    let Ok(Some(san)) = cert.subject_alternative_name() else {
        return false;
    };
    match_sip_sans(&san.value.general_names, expected_domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn der(pem: &str) -> Vec<u8> {
        let (remaining, pem) =
            x509_parser::pem::parse_x509_pem(pem.as_bytes()).expect("valid fixture PEM");
        assert!(remaining.iter().all(u8::is_ascii_whitespace));
        pem.contents
    }

    #[test]
    fn actual_leaf_der_valid_sip_uri_san_and_no_canonical_hostname_confusion() {
        // Self-signed fixtures are for parsing only, never for TLS trust.
        let cert = der(include_str!("../tests/fixtures/sip-uri-san.pem"));
        assert!(matches_sip_identity(&cert, "example.com"));
        assert!(matches_sip_identity(&cert, "EXAMPLE.COM"));
        assert!(!matches_sip_identity(&cert, "sipedge.example.com"));
        assert!(!matches_sip_identity(&cert, "evil.example"));
    }

    #[test]
    fn real_der_sip_uri_takes_precedence_over_matching_dns_san_and_cn() {
        let cert = der(include_str!("../tests/fixtures/conflicting-sip-uri.pem"));
        assert!(!matches_sip_identity(&cert, "example.com"));
        assert!(matches_sip_identity(&cert, "evil.example"));
    }

    #[test]
    fn actual_wildcard_der_cannot_impersonate_sip_domain() {
        let cert = der(include_str!("../tests/fixtures/wildcard-dns.pem"));
        assert!(!matches_sip_identity(&cert, "sip.example.com"));
        assert!(!matches_sip_identity(&cert, "example.com"));
    }

    #[test]
    fn duplicate_subject_alternative_name_extensions_fail_closed() {
        // Test-only public certificate: its SAN extension occurs twice.
        // The DER is deliberately modified after signing and MUST NEVER
        // be trusted for a TLS connection. This tests parsing only.
        let duplicated = der(include_str!("../tests/fixtures/duplicate-san.pem"));
        let (_, cert) = parse_x509_certificate(&duplicated).unwrap();
        assert!(cert.extensions_map().is_err());
        assert!(!matches_sip_identity(&duplicated, "example.com"));
    }

    #[test]
    fn malformed_and_trailing_der_fail_closed() {
        let original = der(include_str!("../tests/fixtures/sip-uri-san.pem"));
        assert!(!matches_sip_identity(&[], "example.com"));
        assert!(!matches_sip_identity(&original[..12], "example.com"));
        assert!(!matches_sip_identity(&vec![0; MAX_CERT_DER + 1], "example.com"));
        let mut appended = original.clone();
        appended.extend_from_slice(&[0x00, 0x01]);
        assert!(!matches_sip_identity(&appended, "example.com"));
        for invalid in ["*.example.com", ".example.com", "example.com.evil",
                        "example..com", "example.com:5061", "example.com\r\n"] {
            assert!(!matches_sip_identity(&original, invalid));
        }
    }

    #[test]
    fn uri_scheme_no_userinfo_parameters_and_exact_dns_comparison() {
        let names = [
            GeneralName::URI("SIP:Example.COM;transport=tls"),
            GeneralName::DNSName("unrelated.test"),
        ];
        assert!(match_sip_sans(&names, "example.com"));
        assert!(!match_sip_sans(&names, "sub.example.com"));
        let user = [GeneralName::URI("sip:alice@example.com"),
            GeneralName::DNSName("EXAMPLE.COM")];
        // A SIP-scheme URI with userinfo is malformed for this certificate
        // identity subset and must not silently fall back to DNS SAN.
        assert!(!match_sip_sans(&user, "example.com"));
        let different = [GeneralName::URI("sips:example.com"),
            GeneralName::DNSName("example.com")];
        assert!(match_sip_sans(&different, "example.com")); // non-sip URI ignored
        let unknown = [GeneralName::URI("https://example.com")];
        assert!(!match_sip_sans(&unknown, "example.com"));
    }

    #[test]
    fn dns_fallback_only_without_acceptable_sip_uri_and_no_wildcards() {
        let dns = [GeneralName::DNSName("EXAMPLE.COM")];
        assert!(match_sip_sans(&dns, "example.com"));
        assert!(!match_sip_sans(&dns, "foo.example.com"));
        let wildcard = [GeneralName::DNSName("*.example.com")];
        assert!(!match_sip_sans(&wildcard, "foo.example.com"));
        let suffix = [GeneralName::DNSName(".example.com")];
        assert!(!match_sip_sans(&suffix, "foo.example.com"));
        let preempt = [GeneralName::URI("sip:other.example"),
            GeneralName::DNSName("example.com")];
        assert!(!match_sip_sans(&preempt, "example.com"));
        assert!(match_sip_sans(&preempt, "other.example"));
    }

    #[test]
    fn valid_idna_a_labels_match_exact_domain_but_fake_punycode_cannot() {
        let valid = "xn--bcher-kva.example";
        assert_eq!(ascii_sip_domain(valid), Some(valid));
        assert!(match_sip_sans(
            &[GeneralName::URI("sip:XN--BCHER-KVA.EXAMPLE")], valid
        ));
        assert!(match_sip_sans(
            &[GeneralName::DNSName("xn--bcher-kva.example")], valid
        ));
        assert!(!match_sip_sans(
            &[GeneralName::DNSName("xn--bcher-kva.example")],
            "shop.xn--bcher-kva.example"
        ));
        for invalid in ["xn--a.example", "xn--ab.example", "xn--p.example"] {
            assert_eq!(ascii_sip_domain(invalid), None, "{invalid}");
            assert_eq!(sip_uri_host(&format!("sip:{invalid}")), None);
            assert!(!match_sip_sans(&[GeneralName::DNSName(invalid)], invalid));
        }
        // Certificate URI/dNSName SANs are IA5String: no Unicode U-labels.
        assert!(ascii_sip_domain("bücher.example").is_none());
    }

    #[test]
    fn rfc9549_other_symbol_a_label_never_authorizes_sip_certificate_san() {
        // RFC 9549 §1 explicitly rejects "♚" (Unicode General Category So).
        // xn--45h is the punycode encoding of that character.
        for invalid in ["xn--45h.example.com", "XN--45H.EXAMPLE.COM"] {
            assert_eq!(ascii_sip_domain(invalid), None);
            assert!(!match_sip_sans(
                &[GeneralName::DNSName(invalid)],
                invalid
            ));
            let uri = format!("sip:{invalid}");
            assert_eq!(sip_uri_host(&uri), None);
            assert!(!match_sip_sans(
                &[GeneralName::URI(&uri)],
                invalid
            ));
        }
        let valid = "xn--bcher-kva.example.com";
        assert_eq!(ascii_sip_domain(valid), Some(valid));
        assert!(match_sip_sans(
            &[GeneralName::URI("sip:xn--bcher-kva.example.com")],
            valid
        ));
        assert!(match_sip_sans(
            &[GeneralName::DNSName("XN--BCHER-KVA.EXAMPLE.COM")],
            valid
        ));
    }

    #[test]
    fn rfc3261_uri_san_parameter_and_header_character_classes() {
        // RFC 3261 (June 2002) §25.1; RFC 5922 (June 2010) §§7.1-7.3.
        for uri in [
            "sip:example.com;foo={bar}",
            "sip:example.com;foo=bar|baz",
            "sip:example.com;foo=bar^baz",
            "sip:example.com?subject={bar}",
            "sip:example.com?subject=bar;baz",
            "sip:example.com?subject=bar=again",
            "sip:example.com?bad{name=value",
            "sip:example.com;bad{name=value",
        ] {
            assert_eq!(sip_uri_host(uri), None, "{uri}");
            assert!(!match_sip_sans(&[
                GeneralName::URI(uri), GeneralName::DNSName("example.com"),
            ], "example.com"), "no DNS SAN fallback: {uri}");
        }
        for uri in [
            "sip:example.com;foo=[x]/:&+$",
            "sip:example.com;foo=%7Bbar%7D",
            "sip:example.com?subject=[x]/?:+$",
            "sip:example.com?subject=hello%20world",
            "sip:example.com;transport=tls?subject=ok&priority=high",
        ] {
            assert_eq!(sip_uri_host(uri), Some("example.com"), "{uri}");
            assert!(match_sip_sans(&[GeneralName::URI(uri)], "example.com"));
        }
    }

    #[test]
    fn malformed_percent_escapes_in_sip_uri_san_never_authorize_identity() {
        for uri in [
            "sip:example.com%", "sip:example.com%2", "sip:example.com%GG",
            "sip:example.com;transport=%", "sip:example.com;transport=%G0",
            "sip:example.com?subject=%2", "sip:example.com?subject=ok%zz",
        ] {
            assert_eq!(sip_uri_host(uri), None, "{uri:?}");
            assert!(!match_sip_sans(&[GeneralName::URI(uri)], "example.com"));
        }
        for uri in [
            "sip:example.com;transport=tls?subject=hello%20world",
            "sip:example.com?subject=foo%2Fbar",
            "sip:example.com;opaque=%3a",
        ] {
            assert_eq!(sip_uri_host(uri), Some("example.com"), "{uri:?}");
        }
    }

    #[test]
    fn duplicate_sip_uri_parameter_names_cannot_authorize_certificate_identity() {
        for uri in [
            "sip:example.com;transport=tcp;transport=tls",
            "sip:example.com;Transport=tcp;transport=tls",
            "sip:example.com;lr;LR",
            "sip:example.com;foo=1;foo=2?subject=ok",
        ] {
            assert_eq!(sip_uri_host(uri), None, "{uri:?}");
            assert!(!match_sip_sans(&[GeneralName::URI(uri)], "example.com"));
        }
        for uri in [
            "sip:example.com;transport=tls;lr",
            "sip:example.com;foo=1;bar=2?subject=ok",
            "sip:example.com?subject=ok&subject=again",
        ] {
            assert_eq!(sip_uri_host(uri), Some("example.com"), "{uri:?}");
        }
    }

    #[test]
    fn percent_encoded_unreserved_duplicate_parameters_are_rejected() {
        for uri in [
            "sip:example.com;transport=tcp;%74ransport=tls",
            "sip:example.com;foo=1;f%6Fo=2",
            "sip:example.com;%6Cr;lr",
            "sip:example.com;X%2DTrace=one;x-trace=two",
            "sip:example.com;%54rAnSpOrT=tcp;transport=tls",
            "sip:example.com;f%4Fo=one;F%6fo=two",
            "sip:example.com;param1=one;param%31=two",
            "sip:example.com;key=one;%6bey=two",
            "sip:example.com;foo%2Fbar=one;foo%2fbar=two",
        ] {
            assert_eq!(sip_uri_host(uri), None, "{uri}");
            assert!(!match_sip_sans(&[GeneralName::URI(uri)], "example.com"),
                "duplicate alias must not authorize certificate identity: {uri}");
        }
        let distinct = "sip:example.com;%66oo=1;bar=2";
        assert_eq!(sip_uri_host(distinct), Some("example.com"));
        // Canonicalizing escaped names must not reject distinct parameters.
        assert!(match_sip_sans(&[GeneralName::URI(distinct)], "example.com"));
        assert!(!match_sip_sans(&[GeneralName::URI(distinct)], "other.example"));
        // Reserved octets are not decoded to separators: distinct encoded
        // and literal parameter names must not become false duplicates.
        let encoded_reserved = "sip:example.com;foo%2Fbar=one;foo%3Fbar=two";
        assert_eq!(sip_uri_host(encoded_reserved), Some("example.com"));
        assert!(match_sip_sans(
            &[GeneralName::URI(encoded_reserved)], "example.com"
        ));
        assert!(!match_sip_sans(
            &[GeneralName::URI(encoded_reserved)], "other.example"
        ));
    }

    #[test]
    fn one_valid_sip_uri_does_not_hide_an_invalid_sip_uri() {
        let invalid = [
            "sip:other.example;transport=tls;transport=tcp",
            "SIP:other.example?subject",
            "sip:other.example;opaque=%0d",
            "sip:alice@other.example",
        ];
        for malformed in invalid {
            for names in [
                vec![
                    GeneralName::URI("sip:example.com"),
                    GeneralName::URI(malformed),
                    GeneralName::DNSName("example.com"),
                ],
                vec![
                    GeneralName::URI(malformed),
                    GeneralName::URI("sip:example.com"),
                    GeneralName::DNSName("example.com"),
                ],
            ] {
                assert!(!match_sip_sans(&names, "example.com"), "{malformed:?}");
            }
        }
        // Multiple well-formed SIP URI SANs do not authorize the wrong host.
        let valid = [
            GeneralName::URI("sip:other.example"),
            GeneralName::URI("sip:example.com"),
        ];
        assert!(match_sip_sans(&valid, "example.com"));
        assert!(match_sip_sans(&valid, "other.example"));
        assert!(!match_sip_sans(&valid, "unrelated.example"));
    }

    #[test]
    fn malformed_sip_uri_san_does_not_silently_enable_dns_fallback() {
        for malformed in [
            "sip:evil.example;transport=%0d",
            "sip:evil.example;transport==tls",
            "sip:evil.example?subject",
            "SIP:evil.example;foo=☃",
            "sip:evil.example;transport=tls;transport=tcp",
        ] {
            let mixed = [
                GeneralName::URI(malformed),
                GeneralName::DNSName("example.com"),
            ];
            assert!(!match_sip_sans(&mixed, "example.com"), "{malformed:?}");
        }
        // Unrelated non-SIP URI SANs cannot preempt a valid DNS SAN.
        let other_scheme = [
            GeneralName::URI("https://portal.example.com/profile"),
            GeneralName::DNSName("example.com"),
        ];
        assert!(match_sip_sans(&other_scheme, "example.com"));
        // A valid matching SIP URI SAN still authorizes the exact domain.
        let valid = [
            GeneralName::URI("sip:example.com;transport=tls"),
            GeneralName::DNSName("evil.example"),
        ];
        assert!(match_sip_sans(&valid, "example.com"));
        assert!(!match_sip_sans(&valid, "evil.example"));
    }

    #[test]
    fn oversized_uri_san_cannot_fall_back_to_matching_dns_san() {
        let oversized = format!("sip:other.example;opaque={}", "x".repeat(MAX_SIP_SAN_URI_BYTES));
        let names = [
            GeneralName::URI(&oversized),
            GeneralName::DNSName("example.com"),
        ];
        assert!(!match_sip_sans(&names, "example.com"));
        assert!(!match_sip_sans(&names, "other.example"));
        // Unrelated valid SIP SAN still takes precedence over matching DNS.
        let conflicting = [
            GeneralName::URI("sip:other.example"),
            GeneralName::DNSName("example.com"),
        ];
        assert!(!match_sip_sans(&conflicting, "example.com"));
    }

    #[test]
    fn bounded_sip_certificate_uri_components_fail_closed() {
        let long_uri = format!("sip:example.com;foo={}", "a".repeat(MAX_SIP_SAN_URI_BYTES));
        let too_many_parameters = format!(
            "sip:example.com;{}",
            (0..=MAX_SIP_SAN_URI_COMPONENTS)
                .map(|i| format!("p{i}=v")).collect::<Vec<_>>().join(";")
        );
        let too_many_headers = format!(
            "sip:example.com?{}",
            (0..=MAX_SIP_SAN_URI_COMPONENTS)
                .map(|i| format!("h{i}=v")).collect::<Vec<_>>().join("&")
        );
        for uri in [&long_uri, &too_many_parameters, &too_many_headers] {
            assert_eq!(sip_uri_host(uri), None);
            assert!(!match_sip_sans(&[GeneralName::URI(uri)], "example.com"));
        }
        assert_eq!(
            sip_uri_host("sip:example.com;transport=tls?subject=ok"),
            Some("example.com")
        );
    }

    #[test]
    fn coordinated_sip_san_negative_and_positive_matrix() {
        // RFC 3261 §§19.1.1, 25.1, RFC 5280 §4.2.1.6,
        // RFC 5922 §§7.1-7.2: fail closed at certificate identity boundary.
        for uri in [
            "sip:example.com?", "sip:example.com?subject",
            "sip:example.com?=value", "sip:example.com?subject=x&",
            "sip:example.com?subject=x&&body=y",
            "sip:example.com?subject=x&body",
            "sip:example.com;transport=tls%0d%0aX:yes",
            "sip:example.com?subject=%00", "sip:example.com?subject=%1f",
            "sip:example.com?subject=%7F", "sip:example.com?subject=%09",
            "sip:example.com;", "sip:example.com;;transport=tls",
            "sip:example.com;=tls", "sip:example.com;transport=tls;",
            "sip:example.com;transport==tls",
            "sip:example.com;note=bücher", "sip:example.com?subject=grüße",
            "sip:example.com;note=☃",
            // Cross-feature cases: no independently safe check may mask an
            // invalid second URI component at the certificate boundary.
            "sip:example.com;transport=tls;%74ransport=tcp?subject=ok",
            "sip:example.com;lr?subject=ok&body",
            "sip:example.com;transport=tls?subject=%0d",
        ] {
            assert_eq!(sip_uri_host(uri), None, "{uri:?}");
            assert!(!match_sip_sans(&[GeneralName::URI(uri)], "example.com"));
        }
        for uri in [
            "sip:example.com?subject=",
            "sip:example.com?subject=x&body=y",
            "sip:example.com;transport=tls?subject=hello%20world",
            "sip:example.com?subject=gr%C3%BC%C3%9Fe",
            "sip:example.com;transport=tls",
            "sip:example.com;lr",
        ] {
            assert_eq!(sip_uri_host(uri), Some("example.com"), "{uri:?}");
        }
        for ip in ["127.0.0.1", "192.0.2.5", "255.255.255.255"] {
            assert_eq!(ascii_sip_domain(ip), None);
            assert_eq!(sip_uri_host(&format!("sip:{ip}")), None);
            assert!(!match_sip_sans(&[GeneralName::DNSName(ip)], ip));
        }
    }

    #[test]
    fn malformed_sip_uri_suffix_cannot_authorize_a_domain() {
        for uri in [
            "sip:example.com#fragment", "sip:example.com;transport=tls#fragment",
            "sip:example.com;transport=tls\\r\\nX:1",
            "sip:example.com?subject=foo\\nbar",
            "sip:example.com\\t;transport=tls",
            "sip:example.com\\\\attacker.test",
        ] {
            assert_eq!(sip_uri_host(uri), None, "{uri:?}");
            assert!(!match_sip_sans(&[GeneralName::URI(uri)], "example.com"));
        }
        // Legitimate URI parameters and headers remain supported.
        assert!(match_sip_sans(
            &[GeneralName::URI("sip:example.com:5061;transport=tls?subject=test")],
            "example.com"
        ));
    }

    #[test]
    fn reject_malformed_sip_uri_and_excessive_san_cardinality() {
        for invalid in ["sip:example.com@attacker.test", "sip:example.com:abc",
                        "sip:example.com:0", "sip://example.com",
                        "sip:[::1]", "sip:*.example.com", "sip:example..com",
                        "sip:example.com\r\n"] {
            assert_eq!(sip_uri_host(invalid), None, "{invalid}");
        }
        assert!(match_sip_sans(
            &[GeneralName::URI("sip:example.com:5061;transport=tls")],
            "example.com"
        ));
        let huge = vec![GeneralName::DNSName("example.com"); MAX_SAN_NAMES + 1];
        assert!(!match_sip_sans(&huge, "example.com"));
    }
}
