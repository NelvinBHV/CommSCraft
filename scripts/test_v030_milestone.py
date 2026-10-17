"""Fail-closed tests for v0.30.0 release evidence. No network used."""
import copy
import json
import pathlib
import sys
import subprocess
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from check_v030_milestone import REQUIRED_CHECKS, milestone_errors, validate_manifest


def template():
    return {"schema_version": 1, "milestone": "v0.30.0", "checks": {
        key: {"outcome": "pending"} for key in REQUIRED_CHECKS
    }}


class V030MilestoneTest(unittest.TestCase):
    def test_pending_baseline_fails_closed(self):
        errors = validate_manifest(template())
        self.assertEqual(len(errors), len(REQUIRED_CHECKS))

    def test_each_required_gate_cannot_be_omitted(self):
        for key in REQUIRED_CHECKS:
            data = template()
            del data["checks"][key]
            self.assertTrue(any("Missing checks" in err for err in validate_manifest(data)))

    def test_fake_pass_cannot_omit_proof(self):
        data = template()
        for key in REQUIRED_CHECKS:
            data["checks"][key] = {"outcome": "passed"}
        self.assertTrue(any("incomplete passing evidence" in err for err in validate_manifest(data)))

    def test_positive_schema_requires_per_gate_explicit_proof(self):
        data = template()
        for key in REQUIRED_CHECKS:
            data["checks"][key] = {
                "outcome": "passed", "commit_sha": "a" * 40,
                "url": "https://github.com/NelvinBHV/CommSCraft/actions/runs/1",
                "reviewer": "independent reviewer",
            }
        self.assertEqual(validate_manifest(data), [])
        data["checks"]["combined_san_ci_windows"]["commit_sha"] = "fake"
        self.assertTrue(validate_manifest(data))

    def test_cross_platform_sha_disagreement_blocks_release(self):
        data = template()
        for key in REQUIRED_CHECKS:
            data["checks"][key] = {
                "outcome": "passed", "commit_sha": "a" * 40,
                "url": "https://github.com/NelvinBHV/CommSCraft/actions/runs/123",
                "reviewer": "independent reviewer",
            }
        data["checks"]["combined_san_ci_windows"]["commit_sha"] = "b" * 40
        self.assertTrue(any("different commits" in e for e in validate_manifest(data)))

    def test_ci_requires_actual_actions_run_link(self):
        data = template()
        data["checks"]["combined_san_ci_ubuntu"] = {
            "outcome": "passed", "commit_sha": "a" * 40,
            "url": "https://github.com/NelvinBHV/CommSCraft/pull/35",
            "reviewer": "independent reviewer",
        }
        self.assertTrue(any("concrete Actions run" in e for e in validate_manifest(data)))

    def test_malformed_actions_job_link_is_rejected(self):
        data = template()
        data["checks"]["combined_san_ci_windows"] = {
            "outcome": "passed",
            "commit_sha": "a" * 40,
            "url": "https://github.com/NelvinBHV/CommSCraft/actions/runs/12/garbage",
            "reviewer": "independent reviewer",
        }
        self.assertTrue(any("invalid GitHub Actions job link" in e for e in validate_manifest(data)))

    def test_unsupported_and_untrusted_manifest_rejected(self):
        data = template()
        data["milestone"] = "v1.0.0"
        self.assertTrue(validate_manifest(data))
        data = template()
        data["checks"]["unexpected"] = {"outcome": "passed"}
        self.assertTrue(validate_manifest(data))
        self.assertTrue(validate_manifest([]))

    def test_release_cli_fails_closed_for_current_pending_manifest(self):
        script = pathlib.Path(__file__).with_name("check_v030_milestone.py")
        result = subprocess.run(
            [sys.executable, str(script), "--release"],
            capture_output=True, text=True, check=False, timeout=15,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("BLOCKED v0.30.0", result.stdout)
        self.assertNotIn("structurally complete", result.stdout)

    def test_development_cli_does_not_mislabel_release(self):
        script = pathlib.Path(__file__).with_name("check_v030_milestone.py")
        result = subprocess.run(
            [sys.executable, str(script)],
            capture_output=True, text=True, check=False, timeout=15,
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("NOT READY", result.stdout)

    def test_malformed_file_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "manifest.json"
            path.write_text("{bad", encoding="utf-8")
            self.assertTrue(milestone_errors(path))
            path.write_text(json.dumps(template()), encoding="utf-8")
            self.assertEqual(len(milestone_errors(path)), len(REQUIRED_CHECKS))


if __name__ == "__main__":
    unittest.main()
