"""Offline regression tests for immutable rustfmt source snapshots."""
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from snapshot_rustfmt_baseline import snapshot, verify


class RustfmtSnapshotTests(unittest.TestCase):
    def test_exact_bytes_and_repeat_refusal(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "crates" / "core.rs"
            source.parent.mkdir(parents=True)
            original = b"fn demo() {  }\r\n"
            source.write_bytes(original)
            archive = root / "baseline"
            result = snapshot(root, archive, ["crates/core.rs"])
            self.assertEqual(result["files"][0]["sha256"], hashlib.sha256(original).hexdigest())
            self.assertEqual((archive / "crates/core.rs").read_bytes(), original)
            self.assertEqual(verify(archive), 1)
            source.write_bytes(b"changed")
            self.assertEqual(verify(archive), 1)
            with self.assertRaises(FileExistsError):
                snapshot(root, archive, ["crates/core.rs"])

    def test_tamper_and_missing_detection(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "a.rs").write_bytes(b"hello")
            dest = root / "archive"
            snapshot(root, dest, ["a.rs"])
            (dest / "a.rs").write_bytes(b"tampered")
            with self.assertRaises(ValueError):
                verify(dest)

    def test_path_traversal_rejected_without_archive(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            with self.assertRaises(ValueError):
                snapshot(root, root / "archive", ["../outside.rs"])
            self.assertFalse((root / "archive").exists())

    def test_missing_file_fails_closed(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            with self.assertRaises(ValueError):
                snapshot(root, root / "archive", ["missing.rs"])
            self.assertFalse((root / "archive").exists())


if __name__ == "__main__":
    unittest.main()
