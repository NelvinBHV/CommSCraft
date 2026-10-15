"""Stdlib-only tests for the fail-closed parity release gate."""
import copy
import json
import pathlib
import unittest

import check_release


class ParityGateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = json.loads((pathlib.Path(__file__).resolve().parents[1]
                                   / "FEATURE_PARITY.json").read_text(encoding="utf-8"))

    def test_manifest_is_structurally_valid(self):
        self.assertEqual(check_release.validate(self.manifest), [])

    def test_incomplete_windows_client_cannot_be_released(self):
        failures = check_release.check_release(self.manifest)
        self.assertGreaterEqual(len(failures), 1)
        self.assertTrue(any("not verified" in error for error in failures))

    def test_missing_feature_makes_manifest_invalid(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["features"] = [f for f in manifest["features"] if f["id"] != "srtp"]
        self.assertTrue(check_release.validate(manifest))

    def test_duplicate_feature_identifier_is_rejected(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["features"][1]["id"] = manifest["features"][0]["id"]
        self.assertTrue(any("Duplicate" in e for e in check_release.validate(manifest)))

    def test_invented_status_is_rejected(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["features"][0]["status"] = "done-ish"
        self.assertTrue(any("unknown status" in e for e in check_release.validate(manifest)))

    def test_v1_scope_contract_is_valid(self):
        contract = check_release.read_json(check_release.V1_CONTRACT_FILE)
        self.assertEqual(check_release.validate_v1_contract(contract), [])

    def test_v1_contract_cannot_exclude_full_client_features(self):
        contract = check_release.read_json(check_release.V1_CONTRACT_FILE)
        contract["scope"] = "core-features-only"
        self.assertTrue(any("scope" in e for e in check_release.validate_v1_contract(contract)))

    def test_incomplete_behavior_dimensions_rejected(self):
        contract = check_release.read_json(check_release.V1_CONTRACT_FILE)
        contract["mandatory_dimensions"].remove("failure_recovery")
        self.assertTrue(check_release.validate_v1_contract(contract))

    def test_unpinned_reference_build_is_never_accepted(self):
        self.assertFalse(check_release.valid_build("UNPINNED"))
        self.assertFalse(check_release.valid_build("16.0.not-real"))
        self.assertTrue(check_release.valid_build("16.0.12345.6789"))

    def test_raw_artifact_paths_must_be_real(self):
        failure = check_release.evidence_artifacts(
            {"raw_artifacts": ["release/evidence/windows-reference/status-publish/trace.pcapng"]},
            "status-publish")
        self.assertTrue(any("missing or empty" in x for x in failure))
        failure = check_release.evidence_artifacts(
            {"raw_artifacts": ["../../etc/passwd"]}, "status-publish")
        self.assertTrue(any("invalid artifact path" in x for x in failure))

    def test_raw_artifacts_may_not_escape_evidence_directory(self):
        errors = check_release.evidence_artifacts(
            {"raw_artifacts": ["release/evidence/windows-reference/status-publish/../../../README.md"]},
            "status-publish")
        self.assertTrue(any("escapes" in e for e in errors))

    @classmethod
    def reference(cls):
        path = pathlib.Path(__file__).resolve().parents[1] / "WINDOWS_REFERENCE_CASES.json"
        return json.loads(path.read_text(encoding="utf-8"))

    def test_official_windows_reference_is_mapped(self):
        baseline = self.reference()
        self.assertEqual(check_release.validate_windows_reference(baseline, self.manifest), [])
        self.assertGreaterEqual(len(baseline["windows_scenarios"]), 90)

    def test_removing_windows_case_is_rejected(self):
        baseline = self.reference()
        baseline["windows_scenarios"] = [
            row for row in baseline["windows_scenarios"] if row["id"] != "remote-control"
        ]
        self.assertTrue(any("Missing Windows Full client" in error
                            for error in check_release.validate_windows_reference(baseline, self.manifest)))

    def test_unknown_feature_case_mapping_is_rejected(self):
        baseline = self.reference()
        baseline["windows_scenarios"][0]["feature_ids"] = ["invented-feature"]
        self.assertTrue(any("unknown mapped features" in error
                            for error in check_release.validate_windows_reference(baseline, self.manifest)))

    def test_reference_without_real_client_evidence_fails_release(self):
        baseline = self.reference()
        signoff = {"windows_client_build": "16.0.not-real", "server_version": "SE",
                   "windows_feature_inventory_reviewed": False,
                   "parity_differences_resolved": False}
        errors = check_release.check_windows_reference_evidence(baseline, signoff)
        self.assertTrue(any("not pinned" in x for x in errors))
        self.assertTrue(any("missing independent Windows-client" in x for x in errors))

    def test_duplicate_reference_case_id_rejected(self):
        baseline = self.reference()
        baseline["windows_scenarios"][1]["id"] = baseline["windows_scenarios"][0]["id"]
        self.assertTrue(any("Duplicate" in x
                            for x in check_release.validate_windows_reference(baseline, self.manifest)))


if __name__ == "__main__":
    unittest.main()
