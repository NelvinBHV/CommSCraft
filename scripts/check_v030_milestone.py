#!/usr/bin/env python3
"""Offline, fail-closed v0.30.0 readiness gate; not a live interoperability test.

This gate deliberately does not promote green CI, parsed certificates or an
offline MIC to authenticated sign-in. It never replaces human security review.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REQUIRED_CHECKS = frozenset({
    "combined_san_ci_ubuntu",
    "combined_san_ci_windows",
    "combined_san_review",
    "sip_domain_tls_peer_binding",
    "spn_registration_binding",
    "hostile_input_offline_tests",
    "source_evidence_review",
    "provenance_format_review",
    "independent_security_review",
})
REQUIRED_EVIDENCE = frozenset({"commit_sha", "url", "reviewer", "outcome"})
VALID_OUTCOMES = frozenset({"passed", "pending", "failed"})


def validate_manifest(data: object) -> list[str]:
    """Treat malformed or incomplete evidence as an unfulfilled milestone."""
    errors: list[str] = []
    if not isinstance(data, dict):
        return ["Manifest must be an object"]
    if data.get("schema_version") != 1 or data.get("milestone") != "v0.30.0":
        errors.append("Unsupported milestone/schema")
    checks = data.get("checks")
    if not isinstance(checks, dict):
        return errors + ["Missing checks object"]
    missing = REQUIRED_CHECKS - checks.keys()
    extra = checks.keys() - REQUIRED_CHECKS
    if missing:
        errors.append("Missing checks: " + ", ".join(sorted(missing)))
    if extra:
        errors.append("Unknown checks: " + ", ".join(sorted(extra)))
    for key in sorted(REQUIRED_CHECKS & checks.keys()):
        row = checks[key]
        if not isinstance(row, dict):
            errors.append(f"{key}: must be an object")
            continue
        outcome = row.get("outcome")
        if outcome not in VALID_OUTCOMES:
            errors.append(f"{key}: invalid outcome")
        if outcome != "passed":
            errors.append(f"{key}: not independently passed")
            continue
        if not REQUIRED_EVIDENCE <= row.keys():
            errors.append(f"{key}: incomplete passing evidence")
            continue
        sha = row.get("commit_sha")
        if not isinstance(sha, str) or len(sha) != 40 or not all(
            c in "0123456789abcdef" for c in sha
        ):
            errors.append(f"{key}: invalid exact commit SHA")
        url = row.get("url")
        if not isinstance(url, str) or not url.startswith(
            "https://github.com/NelvinBHV/CommSCraft/"
        ):
            errors.append(f"{key}: missing repository evidence URL")
        if not isinstance(row.get("reviewer"), str) or not row["reviewer"].strip():
            errors.append(f"{key}: missing independent reviewer")
    return errors


def milestone_errors(path: Path) -> list[str]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        return [f"Cannot read milestone evidence: {exc}"]
    return validate_manifest(data)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=ROOT / "release" / "v030-milestone.json")
    parser.add_argument("--release", action="store_true",
                        help="Refuse milestone sign-off without all reviewed evidence")
    args = parser.parse_args()
    errors = milestone_errors(args.manifest)
    if errors:
        if args.release:
            for error in errors:
                print(f"BLOCKED v0.30.0: {error}")
            return 1
        print(f"Development snapshot: v0.30.0 NOT READY ({len(errors)} open/invalid gates)")
        return 0
    print("v0.30.0 evidence manifest structurally complete; still requires human review")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
