"""Fail closed on incomplete or falsely promoted normative RFC fixture attestations."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

def check(root=ROOT):
    data = json.loads((root / "release/rfc-requirement-evidence.json").read_text(encoding="utf-8"))
    features = json.loads((root / "FEATURE_PARITY.json").read_text(encoding="utf-8"))["features"]
    ids = {x["id"] for x in features}
    found = set()
    for item in data["requirements"]:
        assert item["id"] not in found, "duplicate requirement"
        found.add(item["id"])
        assert item["feature"] in ids, "unknown feature"
        authority = item["authority"]
        assert authority["document"].startswith("RFC "), "missing RFC"
        assert authority["edition"] and authority["section"] and authority["url"].startswith("https://www.rfc-editor.org/"), "untraceable norm"
        assert item["verification"] in ("fixture_mapped_execution_pending", "executed_same_sha"), "unsupported attestation"
        assert item["interop"] is False, "offline evidence cannot attest to interop"
        assert "unverified" in item["ms_counterpart"].lower(), "unreviewed MS claim"
        path = root / item["source"]
        assert path.is_file(), "missing source"
        code = path.read_text(encoding="utf-8")
        for key in ("positive_test", "negative_test"):
            assert "fn " + item[key] + "(" in code, "missing test: " + item[key]
        if item["verification"] == "executed_same_sha":
            assert item.get("workflow_sha") and item.get("run_url") and item.get("run_result") == "success", "missing execution evidence"
    return len(found)

class RequirementEvidenceTests(unittest.TestCase):
    def test_traceable_offline_rfc_fixtures(self):
        self.assertEqual(check(), 4)
    def test_no_false_proof_of_server_interop(self):
        entries = json.loads((ROOT / "release/rfc-requirement-evidence.json").read_text(encoding="utf-8"))["requirements"]
        self.assertTrue(all(not entry["interop"] and entry["verification"] == "fixture_mapped_execution_pending" for entry in entries))

if __name__ == "__main__":
    unittest.main()
