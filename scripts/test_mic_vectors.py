"""Independent test of synthetic MS-SIPAE outgoing MIC test vector.

This exercises HMAC and field order in Python; it does NOT run Rust or
establish a mutual-authenticated Skype for Business security association.
"""
import hashlib
import hmac
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class ClientMicVectorTest(unittest.TestCase):
    def test_ms_order_uses_exact_angle_bracket_fields(self):
        data = json.loads((ROOT / "tests/vectors/client-mic-v4.json").read_text())
        expected = "".join("<" + value + ">" for value in data["fields"])
        self.assertEqual(data["buffer_utf8"], expected)
        self.assertEqual(len(data["fields"]), 15)
        self.assertEqual(data["fields"][0], "TLS-DSK")
        self.assertEqual(data["fields"][1], data["crand"])
        self.assertEqual(data["fields"][2], str(data["cnum"]))
        self.assertEqual(data["fields"][7], "REGISTER")
        self.assertEqual(data["fields"][-1], "3600")
        self.assertTrue(expected.endswith("<3600>"))
        digest = hmac.new(bytes.fromhex(data["key_hex"]),
                          expected.encode("utf-8"), hashlib.sha256).hexdigest()
        self.assertEqual(digest, data["hmac_sha256_hex"])

    def test_change_in_signed_header_changes_mic(self):
        data = json.loads((ROOT / "tests/vectors/client-mic-v4.json").read_text())
        changed = data["buffer_utf8"].replace("<3600>", "<4200>")
        digest = hmac.new(bytes.fromhex(data["key_hex"]),
                          changed.encode("utf-8"), hashlib.sha256).hexdigest()
        self.assertNotEqual(digest, data["hmac_sha256_hex"])

if __name__ == "__main__":
    unittest.main()
