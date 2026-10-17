"""Audit coverage is complete, but protocol compliance is not asserted."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
class SpecAudit(unittest.TestCase):
    def test_every_declared_partial_has_a_review_record(self):
        base = json.loads((ROOT / "FEATURE_PARITY.json").read_text(encoding="utf-8"))
        overlay = json.loads((ROOT / "release/offline-feature-development.json").read_text(encoding="utf-8"))
        audit = json.loads((ROOT / "release/offline-feature-spec-audit.json").read_text(encoding="utf-8"))
        wanted = {f["id"] for f in base["features"] if f["status"] == "offline_subset"}
        wanted.update(x["id"] for x in overlay["overrides"])
        records = audit["records"]
        self.assertEqual(len(records), len({r["id"] for r in records}))
        self.assertEqual({r["id"] for r in records}, wanted)
        for record in records:
            self.assertTrue(record["source"])
            self.assertTrue(record["tests"])
            self.assertIn(record["normative_claim"], ("none_local_only", "unverified_mapping"))
            self.assertTrue(record["refs"])
            for ref in record["refs"]:
                self.assertTrue(ref["url"].startswith(("https://www.rfc-editor.org/", "https://learn.microsoft.com/")))
                self.assertTrue(ref["edition"])
                self.assertTrue(ref["section"])
                self.assertTrue(ref["review"])
            if record["normative_claim"] == "none_local_only":
                self.assertTrue(all(ref["review"] == "context_only_not_implemented" for ref in record["refs"]))
    def test_no_false_normative_claim(self):
        audit = json.loads((ROOT / "release/offline-feature-spec-audit.json").read_text(encoding="utf-8"))
        self.assertEqual(sum(r["normative_claim"] == "requirement_verified" for r in audit["records"]), 0)

if __name__ == "__main__":
    unittest.main()
