"""CommSCraft security source contract: transport must authenticate SIP origin, not SRV target."""
import unittest
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class TlsSipIdentityContract(unittest.TestCase):
    def test_tls_origin_identity_not_dns_srv_target(self):
        source = (ROOT / "crates/commscraft-transport/src/lib.rs").read_text()
        self.assertIn("tls.connect(peer.tls_reference_identity(), tcp)", source)
        self.assertIn("pub fn tls_reference_identity(&self)", source)
        self.assertIn("pub fn sip_domain(&self)", source)
        self.assertIn("tls.peer_certificate()", re.sub(r"\s+", "", source))
        self.assertIn("cert.to_der()", source)
        self.assertIn("idna::domain_to_ascii_strict(name)", source)
        self.assertIn("sip_certificate::matches_sip_identity(", source)
        self.assertNotIn("if cert.is_none()", source)
        self.assertNotIn("tls.connect(peer.host(), tcp)", source)

    def test_sip_san_rules_bound_to_actual_peer_der(self):
        cert = (ROOT / "crates/commscraft-transport/src/sip_certificate.rs").read_text()
        for phrase in (
            "parse_x509_certificate(leaf_der)",
            "cert.subject_alternative_name()",
            "GeneralName::URI(uri)",
            "GeneralName::DNSName(dns)",
            "if has_valid_sip_uri { uri_matches } else { dns_matches }",
            "if !remainder.is_empty()",
            "fn ascii_sip_domain(name: &str)",
        ):
            self.assertIn(re.sub(r'\s+', '', phrase), re.sub(r'\s+', '', cert))
        for fixture in ("sip-uri-san.pem", "conflicting-sip-uri.pem", "wildcard-dns.pem"):
            pem = ROOT / "crates/commscraft-transport/tests/fixtures" / fixture
            self.assertTrue(pem.is_file(), fixture)
            self.assertIn("-----BEGIN CERTIFICATE-----", pem.read_text())
            self.assertNotIn("PRIVATE KEY", pem.read_text())

    def test_invalid_punycode_rejected_for_cert_sans_and_tls_endpoints(self):
        parser = (ROOT / "crates/commscraft-transport/src/sip_certificate.rs").read_text()
        manifest = (ROOT / "crates/commscraft-transport/Cargo.toml").read_text()
        self.assertIn("idna::domain_to_ascii_strict(name)", parser)
        self.assertIn("valid_idna_a_labels_match_exact_domain_but_fake_punycode_cannot", parser)
        self.assertIn('idna = "1.1.0"', manifest)

    def test_rfc9549_other_symbol_a_labels_checked_both_before_tls_and_in_sans(self):
        transport = (ROOT / "crates/commscraft-transport/src/lib.rs").read_text()
        cert = (ROOT / "crates/commscraft-transport/src/sip_certificate.rs").read_text()
        policy = (ROOT / "crates/commscraft-transport/src/idna_policy.rs").read_text()
        manifest = (ROOT / "crates/commscraft-transport/Cargo.toml").read_text()
        self.assertIn("rfc9549_a_label_symbols_allowed(&canonical)", transport)
        self.assertIn("rfc9549_a_label_symbols_allowed(name)", cert)
        self.assertIn("GeneralCategory::OtherSymbol", policy)
        self.assertIn("idna::domain_to_unicode", policy)
        self.assertIn('unicode-general-category = "1.1.0"', manifest)
        self.assertIn("rfc9549_rejects_other_symbol_a_labels_before_tls_dns_or_sni", transport)
        self.assertIn("rfc9549_other_symbol_a_label_never_authorizes_sip_certificate_san", cert)

    def test_inbound_only_and_no_trust_bypass(self):
        source = (ROOT / "crates/commscraft-transport/src/lib.rs").read_text()
        self.assertIn("TlsConnector::new()", source)
        self.assertNotIn("danger_accept_invalid_certs", source)
        self.assertNotIn("danger_accept_invalid_hostnames", source)
        self.assertNotIn("pub fn register(", source)
        self.assertNotIn("pub fn send(", source)


if __name__ == "__main__":
    unittest.main()
