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

fn ascii_sip_domain(name: &str) -> Option<&str> {
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
    // RFC 5280 §4.2.1.6: uniformResourceIdentifier SAN is IA5String.
    // Do not accept raw non-ASCII even in ignored SIP URI suffixes.
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
            index += 3;
        } else {
            index += 1;
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
                if let Some(host) = sip_uri_host(uri) {
                    has_valid_sip_uri = true;
                    if host.eq_ignore_ascii_case(reference) {
                        uri_matches = true;
                    }
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
        assert!(match_sip_sans(&user, "example.com"));
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
    fn non_ascii_uri_san_suffix_cannot_authorize_sip_identity() {
        for uri in [
            "sip:example.com;opaque=bücher",
            "sip:example.com?subject=grüße",
            "sip:example.com;note=☃",
        ] {
            assert_eq!(sip_uri_host(uri), None, "{uri:?}");
            assert!(!match_sip_sans(&[GeneralName::URI(uri)], "example.com"));
        }
        // ASCII-safe percent-escaping remains possible under the parser's
        // separate escape validation; this is NOT a full URI grammar claim.
        assert_eq!(
            sip_uri_host("sip:example.com?subject=gr%C3%BC%C3%9Fe"),
            Some("example.com")
        );
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
