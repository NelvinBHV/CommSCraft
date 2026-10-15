"""Source-only regression checks, not execution of Rust protocol behavior."""
from pathlib import Path
import unittest
from check_v014_evidence import ROOT, LEDGER, verify
import json

class GateContract(unittest.TestCase):
    def test_current_evidence_and_historical_ledgers(self):
        self.assertEqual(verify()['requirements'], 8)

    def test_demo_200_cannot_claim_sign_in(self):
        source = (ROOT/'crates/commscraft-core/src/session.rs').read_text()
        self.assertIn('AcceptanceUnverified', source)
        self.assertNotIn('SessionState::Registered', source)
        self.assertIn('fn demo_200_is_not_a_login()', source)

    def test_only_mic_path_can_accept_200(self):
        source = (ROOT/'crates/commscraft-core/src/session.rs').read_text()
        self.assertIn('if code == 200 {', source)
        self.assertIn('observe_mic_checked_200(', source)
        self.assertIn('GateState::MicCheckedNotSignedIn', source)
        self.assertIn('fn synthetic_mic_checked_200_is_not_signed_in()', source)
        self.assertIn('fn bad_mic_in_synthetic_success_fails_closed()', source)

    def test_no_sip_network_send_and_release_contract_not_weakened(self):
        source = (ROOT/'crates/commscraft-core/src/session.rs').read_text()
        self.assertNotIn('TcpStream', source)
        self.assertNotIn('TlsStream', source)
        self.assertNotIn('pub fn send(', source)
        data = json.loads(LEDGER.read_text())
        self.assertFalse(data['v1_release_ready'])
        self.assertFalse(data['real_sfb_interoperability_tested'])

if __name__ == '__main__':
    unittest.main()
