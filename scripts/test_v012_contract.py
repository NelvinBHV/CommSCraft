"""Source-only regression checks: Rust execution remains separate."""
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from check_v012_evidence import verify

ROOT = Path(__file__).resolve().parent.parent

class InboundTransportContracts(unittest.TestCase):
    def test_v012_evidence_hashes(self):
        self.assertGreaterEqual(verify()['hashed_files'], 4)
    def test_negative_rust_cases_authored(self):
        inbound = (ROOT/'crates/commscraft-core/src/inbound.rs').read_text()
        for test in ('missing_length_fails_closed', 'duplicate_length_fails_closed',
                     'truncated_frame_at_eof_rejected', 'transport_error_poisoned',
                     'fragment_then_timeout_retains_partial_message', 'huge_header_is_rejected'):
            self.assertIn('fn ' + test, inbound)
    def test_external_network_remains_receive_only(self):
        tls = (ROOT/'crates/commscraft-transport/src/lib.rs').read_text()
        self.assertNotIn('pub fn send(', tls)
        self.assertNotIn('pub fn register(', tls)
        self.assertIn('pub fn receive(&mut self)', tls)
    def test_old_evidence_is_preserved(self):
        from check_v011_evidence import verify as old_verify
        self.assertGreaterEqual(old_verify()['verified_files'], 4)

if __name__ == '__main__': unittest.main()
