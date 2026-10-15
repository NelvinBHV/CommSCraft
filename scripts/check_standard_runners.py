#!/usr/bin/env python3
"""Prevent CommSCraft from using nonstandard GitHub Actions runners.

Structural runner configuration check, NOT a GitHub billing API or a
protocol-conformance test. Reject ambiguous/dynamic runner expressions.
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALLOWED = frozenset(("ubuntu-latest", "windows-latest"))
RUNS_ON = re.compile(r"^\s+runs-on:\s*(.*?)(?:\s+#.*)?$")
MATRIX_OS = re.compile(r"^\s+os:\s*\[([^\]]+)\]\s*(?:#.*)?$")


def verify_repository(root: Path = ROOT) -> int:
    workflow_root = root / ".github" / "workflows"
    files = sorted((*workflow_root.glob("*.yml"), *workflow_root.glob("*.yaml")))
    if not files:
        raise ValueError("No GitHub Actions workflows to verify")

    number_checked = 0
    for path in files:
        lines = path.read_text(encoding="utf-8").splitlines()
        active = [line for line in lines if not line.lstrip().startswith("#")]
        matrix_os = []
        for line in active:
            match = MATRIX_OS.match(line)
            if match:
                labels = [x.strip().strip("'\"") for x in match.group(1).split(",")]
                matrix_os.append(labels)
                if not labels or any(item not in ALLOWED for item in labels):
                    raise ValueError(f"{path}: nonstandard matrix OS labels: {labels!r}")
            # A reusable workflow is a different runner trust/billing boundary.
            if re.match(r"^    uses:\s*", line):
                raise ValueError(f"{path}: reusable job workflows are not allowed")
        count_in_file = 0
        for line in active:
            match = RUNS_ON.match(line)
            if not match:
                continue
            runner = match.group(1).strip()
            if runner in ALLOWED:
                pass
            elif runner == "${{ matrix.os }}" and matrix_os:
                pass
            else:
                raise ValueError(f"{path}: nonstandard or dynamic runs-on: {runner!r}")
            count_in_file += 1
            number_checked += 1
        if count_in_file == 0:
            raise ValueError(f"{path}: workflow has no permitted runs-on entries")
    return number_checked


if __name__ == "__main__":
    print(
        f"PASS: {verify_repository()} runner selections use GitHub-hosted "
        "ubuntu-latest/windows-latest only"
    )
    print("Not a billing, RFC/MS protocol conformance or interoperability test")
