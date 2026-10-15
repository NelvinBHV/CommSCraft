"""CommSCraft security source contract: transport must authenticate SIP origin, not SRV target."""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class TlsSipIdentityContract(unittest.TestCase):
    def test_tls_origin_identity_not_dns_srv_target(self):
        source = (ROOT / "crates/commscraft-transport/src/lib.rs").read_text()
        self.assertIn("tls.connect(peer.tls_reference_identity(), tcp)", source)
        self.assertIn("pub fn tls_reference_identity(&self)", source)
        self.assertIn("pub fn sip_domain(&self)", source)
        self.assertIn("tls.peer_certificate()", source)
        self.assertNotIn("tls.connect(peer.host(), tcp)", source)

    def test_inbound_only_and_no_trust_bypass(self):
        source = (ROOT / "crates/commscraft-transport/src/lib.rs").read_text()
        self.assertIn("TlsConnector::new()", source)
        self.assertNotIn("danger_accept_invalid_certs", source)
        self.assertNotIn("danger_accept_invalid_hostnames", source)
        self.assertNotIn("pub fn register(", source)
        self.assertNotIn("pub fn send(", source)


if __name__ == "__main__":
    unittest.main()
