"""Regression tests for the read-only rustfmt provenance audit."""
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import audit_rustfmt_provenance


class ProvenanceAuditTests(unittest.TestCase):
    def test_actual_import_manifest_baseline(self):
        result = audit_rustfmt_provenance.audit()
        self.assertEqual(result["manifest_entries_verified"], 100)
        self.assertEqual(
            result["archived_or_moved_entries"] + result["original_in_place_entries"], 100
        )
        self.assertFalse(result["formatting_verified"])
        self.assertFalse(result["rust_tests_executed"])

    def test_corrupt_manifest_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "docs/source-import-manifest.json"
            manifest.parent.mkdir(parents=True)
            manifest.write_text('{"file_count": 1, "files": [{"path": "missing.rs", "sha256": "0"}]}')
            with self.assertRaises(AssertionError):
                audit_rustfmt_provenance.audit(root)


if __name__ == "__main__":
    unittest.main()
