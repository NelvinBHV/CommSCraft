#!/usr/bin/env python3
"""Source migration integrity, NOT RFC conformance or Rust runtime evidence."""
from __future__ import annotations
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "docs/source-import-manifest.json"
MOVED = {
    ".github/workflows/ci.yml": "docs/history/import-v0.14.0/.github/workflows/ci.yml",
    ".github/workflows/rust.yml": "docs/history/import-v0.14.0/.github/workflows/rust.yml",
    "Cargo.toml": "docs/history/import-v0.14.0/Cargo.toml",
    "crates/sfb-core/src/sip.rs": "docs/history/import-v0.14.0/crates/sfb-core/src/sip.rs",
    "crates/sfb-core/src/security.rs": "docs/history/import-v0.14.0/crates/sfb-core/src/security.rs",
    "crates/sfb-router/src/lib.rs": "docs/history/import-v0.14.0/crates/sfb-router/src/lib.rs",
    "crates/sfb-core/src/sdp.rs": "docs/history/import-v0.14.0/crates/sfb-core/src/sdp.rs",
}

def verify(root: Path = ROOT) -> int:
    manifest = json.loads((root / "docs/source-import-manifest.json").read_text(encoding="utf-8"))
    files = manifest["files"]
    assert len(files) == manifest["file_count"] == 100
    assert len({row["path"] for row in files}) == len(files)
    for row in files:
        path = row["path"]
        assert not path.startswith("/") and ".." not in Path(path).parts
        src = (root / MOVED.get(path, path)).resolve()
        assert src.is_relative_to(root.resolve()) and src.is_file(), path
        actual = hashlib.sha256(src.read_bytes()).hexdigest()
        assert actual == row["sha256"], f"source hash mismatch: {path}"
    return len(files)

if __name__ == "__main__":
    print(f"Historical CommSCraft source SHA-256 verified for {verify()} files (not a build/interop result)")
