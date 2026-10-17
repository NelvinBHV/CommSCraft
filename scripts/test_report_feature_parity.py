"""Feature-parity inventory reporting remains conservative and deterministic."""
import copy
import json
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from report_feature_parity import summarize, markdown


class ParityReportTest(unittest.TestCase):
    def test_live_registry_never_claims_full_parity(self):
        stats = summarize()
        self.assertEqual(stats["total_features"], sum(stats["status"].values()))
        self.assertEqual(sum(x["total"] for x in stats["areas"].values()), stats["total_features"])
        self.assertEqual(stats["status"]["verified"], 0)
        self.assertEqual(stats["features_with_registered_interop_evidence"], 0)
        self.assertEqual(stats["cases_with_registered_interop_evidence"], 0)
        report = markdown(stats)
        self.assertIn("Offline subsets are not feature-complete", report)

    def test_duplicate_or_unknown_state_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            features = {"features": [
                {"id": "a", "status": "offline_subset", "area": "identity"},
                {"id": "b", "status": "verified", "area": "identity", "interop_evidence": ["reviewed-fixture"]},
            ]}
            cases = {"windows_scenarios": [{"id": "one", "feature_ids": ["a", "b"]}]}
            (root / "WINDOWS_REFERENCE_CASES.json").write_text(json.dumps(cases), encoding="utf-8")
            path = root / "FEATURE_PARITY.json"
            path.write_text(json.dumps(features), encoding="utf-8")
            self.assertEqual(summarize(root)["status"]["verified"], 1)
            self.assertEqual(summarize(root)["features_linked_to_windows_cases"], 2)
            features["features"][1]["id"] = "a"
            path.write_text(json.dumps(features), encoding="utf-8")
            with self.assertRaises(ValueError):
                summarize(root)
            features["features"][1]["id"] = "b"
            cases["windows_scenarios"][0]["feature_ids"] = ["unmapped"]
            (root / "WINDOWS_REFERENCE_CASES.json").write_text(json.dumps(cases), encoding="utf-8")
            with self.assertRaises(ValueError):
                summarize(root)
            cases["windows_scenarios"][0]["feature_ids"] = ["a", "b"]
            (root / "WINDOWS_REFERENCE_CASES.json").write_text(json.dumps(cases), encoding="utf-8")
            features["features"][1]["interop_evidence"] = []
            path.write_text(json.dumps(features), encoding="utf-8")
            with self.assertRaises(ValueError):
                summarize(root)
            features["features"][1]["interop_evidence"] = ["reviewed-fixture"]
            features["features"][1]["status"] = "unknown"
            path.write_text(json.dumps(features), encoding="utf-8")
            with self.assertRaises(ValueError):
                summarize(root)


if __name__ == "__main__":
    unittest.main()
