//! Additional RFC 9549 §1 A-label security restriction shared by the
//! actual TLS reference identity, discovered DNS destination and certificate
//! SIP URI / dNSName SAN matching.
//!
//! RFC 9549 explicitly excludes Unicode General Category "Other Symbol" (So)
//! from IDNA2008 even if UTS #46's strict conversion accepts its Punycode.
//! Callers MUST also run their existing strict ASCII/IDNA round-trip checks.
//! The Unicode representation is *only* inspected for disallowed properties;
//! it is never used as the TLS SNI name or for certificate equality.

use unicode_general_category::{get_general_category, GeneralCategory};

pub(crate) fn rfc9549_a_label_symbols_allowed(ascii_domain: &str) -> bool {
    if ascii_domain.is_empty() || ascii_domain.len() > 253 || !ascii_domain.is_ascii() {
        return false;
    }
    let (unicode_name, status) = idna::domain_to_unicode(ascii_domain);
    status.is_ok()
        && !unicode_name
            .chars()
            .any(|character| get_general_category(character) == GeneralCategory::OtherSymbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc9549_explicit_other_symbol_punycode_is_denied() {
        assert_eq!(get_general_category('♚'), GeneralCategory::OtherSymbol);
        assert!(!rfc9549_a_label_symbols_allowed("xn--45h.example"));
        assert!(!rfc9549_a_label_symbols_allowed("XN--45H.EXAMPLE"));
    }

    #[test]
    fn ordinary_and_valid_idn_a_labels_still_pass() {
        assert!(rfc9549_a_label_symbols_allowed("example.com"));
        assert!(rfc9549_a_label_symbols_allowed("XN--BCHER-KVA.EXAMPLE"));
        assert!(!rfc9549_a_label_symbols_allowed("bücher.example"));
        assert!(!rfc9549_a_label_symbols_allowed(""));
    }
}
