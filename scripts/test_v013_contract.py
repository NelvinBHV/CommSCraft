"""Limited v0.13.0 source/manifest integrity regression tests only."""
import unittest
from pathlib import Path
from check_v013_evidence import ROOT, verify

class RouterSourceTests(unittest.TestCase):
    def test_frozen_hashes_and_rfc_mapping(self):
        result=verify()
        self.assertGreaterEqual(result['requirements'], 8)
    def test_preserved_v012_sources(self):
        from check_v012_evidence import verify as verify_12
        self.assertGreaterEqual(verify_12()['hashed_files'], 7)
    def test_router_is_offline(self):
        code=(ROOT/'crates/commscraft-router/src/lib.rs').read_text()
        self.assertNotIn('TlsConnector',code)
        self.assertNotIn('write_all(',code)
        self.assertIn('pub fn disconnect',code)
    def test_spec_linked_wire_tests_authored(self):
        tests=(ROOT/'crates/commscraft-router/tests/offline_wire.rs').read_text()
        self.assertEqual(tests.count('#[test]'),3)

if __name__ == '__main__':
    unittest.main()
