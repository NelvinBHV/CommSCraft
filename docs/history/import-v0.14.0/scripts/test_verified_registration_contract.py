"""Static guards for evidence metadata; no claim that Rust tests have run."""
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from check_v011_evidence import verify, LEDGER

class ContractChecks(unittest.TestCase):
    def test_evidence_hashes_and_runtime_disclosures(self):
        self.assertGreaterEqual(verify()['verified_files'], 4)

    def test_public_rust_tests_present(self):
        p = Path(__file__).resolve().parent.parent / 'crates/sfb-core/tests/verified_registration.rs'
        src = p.read_text()
        self.assertEqual(src.count('#[test]'), 9)
        for test in (
            'a_bad_mic_never_consumes_transaction_or_replay_counter',
            'a_mismatched_transaction_cannot_consume_a_valid_signed_sequence',
            'contact_gruu_is_not_signed_by_sipae_and_must_remain_untrusted',
            'signed_removal_is_not_assumed_from_unsigned_removal',
        ):
            self.assertIn(f'fn {test}', src)

    def test_legacy_unsigned_path_distinct(self):
        src = (Path(__file__).resolve().parent.parent / 'crates/sfb-core/src/registration.rs').read_text()
        self.assertIn('RegisterState::UnverifiedAcceptance', src)
        self.assertIn('RegisterState::MicCheckedAcceptance', src)

if __name__ == '__main__':
    unittest.main()
