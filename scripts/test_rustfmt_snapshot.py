"""Offline regression tests for immutable rustfmt source snapshots."""
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from snapshot_rustfmt_baseline import ROOT, paths_from_plan, snapshot, verify


class RustfmtSnapshotTests(unittest.TestCase):
    def test_committed_archive_matches_original_source_bytes(self):
        # The archive was recorded before any rustfmt changes; verify its
        # byte-level manifest and unchanged original source on this stage.
        archive = ROOT / "docs/history/rustfmt-baseline-2026-10-20"
        manifest = json.loads((archive / "manifest.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["source_commit"], "72f70b268432abdee76d2a007a204cef2f788e13")
        self.assertEqual(verify(archive), 56)
        self.assertEqual(len(manifest["files"]), 56)
        self.assertEqual(
            sorted(row["path"] for row in manifest["files"]),
            paths_from_plan(ROOT),
        )
        for row in manifest["files"]:
            self.assertEqual(
                hashlib.sha256((ROOT / row["path"]).read_bytes()).hexdigest(),
                row["sha256"],
                row["path"],
            )

    def test_real_repository_inventory_snapshots_without_source_mutation(self):
        # Smoke-test all current Rust bytes in an OS temp directory. The
        # repository, imported archives, and evidence hashes remain untouched.
        paths = paths_from_plan(ROOT)
        self.assertEqual(len(paths), 56)
        before = {
            path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
            for path in paths
        }
        with tempfile.TemporaryDirectory() as temp:
            destination = Path(temp) / "snapshot"
            manifest = snapshot(ROOT, destination, paths)
            self.assertEqual(len(manifest["files"]), len(paths))
            self.assertEqual(verify(destination), 56)
            # CI evidence for a blob-preserving permanent Git archive.
            print("RUSTFMT_BASELINE_SHA256_JSON=" + json.dumps(manifest, sort_keys=True))
            self.assertEqual(
                {row["path"]: row["sha256"] for row in manifest["files"]},
                before,
            )
        after = {
            path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
            for path in paths
        }
        self.assertEqual(before, after)

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
