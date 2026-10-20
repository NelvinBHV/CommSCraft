#!/usr/bin/env python3
"""Snapshot CURRENT pre-rustfmt files without changing v0.14.0 history or source."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import tempfile

ROOT = Path(__file__).resolve().parent.parent
PLAN = Path("docs/rustfmt-provenance-migration-plan.md")
DEST = Path("docs/history/rustfmt-baseline-2026-10-20")
SECTION = "## Exact file inventory from latest Ubuntu + Windows diagnostics"


def paths_from_plan(root: Path) -> list[str]:
    text = (root / PLAN).read_text(encoding="utf-8")
    if text.count(SECTION) != 1:
        raise ValueError("unique rustfmt inventory section required")
    section = text.split(SECTION, 1)[1].split("\n## ", 1)[0]
    paths = []
    for line in section.splitlines():
        if line.startswith("- `") and line.endswith("`"):
            paths.append(line[3:-1])
    if len(paths) != 56 or len(set(paths)) != 56 or paths != sorted(paths):
        raise ValueError("expected 56 unique sorted Rust source paths")
    for item in paths:
        p = PurePosixPath(item)
        if p.is_absolute() or ".." in p.parts or p.suffix != ".rs":
            raise ValueError(f"unsafe inventory path: {item}")
    return paths


def _contents(root: Path, relative: str) -> bytes:
    src = root / relative
    if src.is_symlink() or not src.is_file() or not src.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"unsafe/missing source: {relative}")
    return src.read_bytes()


def snapshot(root: Path, destination: Path, paths: list[str]) -> dict:
    """Create destination atomically. Never overwrite a previous snapshot."""
    if destination.exists() or destination.is_symlink():
        raise FileExistsError(str(destination))
    destination.parent.mkdir(parents=True, exist_ok=True)
    staged = Path(tempfile.mkdtemp(prefix=".rustfmt-staging-", dir=destination.parent))
    try:
        rows = []
        for relative in paths:
            p = PurePosixPath(relative)
            if p.is_absolute() or ".." in p.parts or p.suffix != ".rs":
                raise ValueError(f"unsafe source path: {relative}")
            data = _contents(root, relative)
            target = staged / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
            rows.append({"path": relative, "sha256": hashlib.sha256(data).hexdigest(), "size": len(data)})
        manifest = {"schema": 1, "source": "current pre-rustfmt working tree", "files": rows}
        (staged / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        verify(staged)
        if destination.exists() or destination.is_symlink():
            raise FileExistsError(str(destination))
        staged.rename(destination)
        return manifest
    finally:
        if staged.exists():
            shutil.rmtree(staged)


def verify(destination: Path) -> int:
    manifest = json.loads((destination / "manifest.json").read_text(encoding="utf-8"))
    rows = manifest["files"]
    if len(rows) != len({row["path"] for row in rows}):
        raise ValueError("duplicate manifest path")
    for row in rows:
        relative = row["path"]
        p = PurePosixPath(relative)
        if p.is_absolute() or ".." in p.parts or p.suffix != ".rs":
            raise ValueError(f"unsafe manifest path: {relative}")
        data = _contents(destination, relative)
        if len(data) != row["size"] or hashlib.sha256(data).hexdigest() != row["sha256"]:
            raise ValueError(f"baseline mismatch: {relative}")
    return len(rows)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify", action="store_true", help="verify an existing immutable baseline")
    args = parser.parse_args()
    dest = ROOT / DEST
    if args.verify:
        print(f"Verified {verify(dest)} archived pre-rustfmt source files")
    else:
        paths = paths_from_plan(ROOT)
        manifest = snapshot(ROOT, dest, paths)
        print(f"Archived {len(manifest['files'])} pre-rustfmt files to {DEST}")


if __name__ == "__main__":
    main()
