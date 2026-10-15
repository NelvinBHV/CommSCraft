"""Fail-closed coverage for CommSCraft standard GitHub-hosted runner policy."""
import tempfile
import unittest
from pathlib import Path

from check_standard_runners import verify_repository


class StandardRunnersOnly(unittest.TestCase):
    def fixture(self, text: str):
        temp = tempfile.TemporaryDirectory()
        root = Path(temp.name)
        workflow = root / ".github" / "workflows"
        workflow.mkdir(parents=True)
        (workflow / "ci.yml").write_text(text, encoding="utf-8")
        return temp, root

    def test_standard_linux_and_windows_matrix(self):
        temp, root = self.fixture(
            "jobs:\n  test:\n    strategy:\n      matrix:\n"
            "        os: [ubuntu-latest, windows-latest]\n"
            "    runs-on: ${{ matrix.os }}\n"
        )
        with temp:
            self.assertEqual(verify_repository(root), 1)

    def test_direct_standard_labels(self):
        temp, root = self.fixture(
            "jobs:\n  linux:\n    runs-on: ubuntu-latest\n"
            "  windows:\n    runs-on: windows-latest\n"
        )
        with temp:
            self.assertEqual(verify_repository(root), 2)

    def test_larger_runner_is_rejected(self):
        temp, root = self.fixture("jobs:\n  large:\n    runs-on: ubuntu-24.04-16core\n")
        with temp:
            with self.assertRaises(ValueError):
                verify_repository(root)

    def test_self_hosted_is_rejected(self):
        temp, root = self.fixture("jobs:\n  custom:\n    runs-on: self-hosted\n")
        with temp:
            with self.assertRaises(ValueError):
                verify_repository(root)

    def test_dynamic_expression_is_rejected(self):
        temp, root = self.fixture("jobs:\n  custom:\n    runs-on: ${{ inputs.runner }}\n")
        with temp:
            with self.assertRaises(ValueError):
                verify_repository(root)

    def test_matrix_injection_is_rejected(self):
        temp, root = self.fixture(
            "jobs:\n  test:\n    strategy:\n      matrix:\n"
            "        os: [ubuntu-latest, windows-2022-16core]\n"
            "    runs-on: ${{ matrix.os }}\n"
        )
        with temp:
            with self.assertRaises(ValueError):
                verify_repository(root)

    def test_reusable_workflow_job_is_rejected(self):
        temp, root = self.fixture("jobs:\n  remote:\n    uses: vendor/workflow/.github/workflows/ci.yml@main\n")
        with temp:
            with self.assertRaises(ValueError):
                verify_repository(root)

    def test_missing_runner_is_rejected(self):
        temp, root = self.fixture("name: empty\n")
        with temp:
            with self.assertRaises(ValueError):
                verify_repository(root)


if __name__ == "__main__":
    unittest.main()
