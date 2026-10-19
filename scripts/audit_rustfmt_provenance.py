#!/usr/bin/env python3
"""Read-only pre-rustfmt audit; no source, archive or evidence mutation."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

from check_import_integrity import MOVED, verify

ROOT = Path(__file__).resolve().parent.parent
HISTORY = ROOT / "docs/history/import-v0.14.0"


def audit(root: Path = ROOT) -> dict[str, object]:
    """Fail closed if any imported source lacks a verifiable pristine baseline."""
    verified = verify(root)
    manifest = json.loads((root / "docs/source-import-manifest.json").read_text(encoding="utf-8"))
    archived = 0
    original_in_place = 0
    for entry in manifest["files"]:
        path = entry["path"]
        historical = root / "docs/history/import-v0.14.0" / path
        if historical.is_file():
            source = historical
            archived += 1
        elif path in MOVED:
            source = root / MOVED[path]
            archived += 1
        else:
            # Historical bytes can be at the original path only if unmodified.
            source = root / path
            original_in_place += 1
        assert source.is_file(), f"missing historical baseline: {path}"
        assert hashlib.sha256(source.read_bytes()).hexdigest() == entry["sha256"], (
            f"historical baseline differs: {path}"
        )
    return {
        "manifest_entries_verified": verified,
        "archived_or_moved_entries": archived,
        "original_in_place_entries": original_in_place,
        "formatting_verified": False,
        "rust_tests_executed": False,
    }


if __name__ == "__main__":
    print(json.dumps(audit(), sort_keys=True))
