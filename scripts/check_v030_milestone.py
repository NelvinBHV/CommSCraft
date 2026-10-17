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
        if not isinstance(outcome, str) or outcome not in VALID_OUTCOMES:
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
    # Linux and Windows evidence must refer to the same integrated revision.
    linux = checks.get("combined_san_ci_ubuntu")
    windows = checks.get("combined_san_ci_windows")
    if isinstance(linux, dict) and isinstance(windows, dict) and (
        linux.get("outcome") == "passed" and windows.get("outcome") == "passed"
        and linux.get("commit_sha") != windows.get("commit_sha")
    ):
        errors.append("Ubuntu/Windows CI evidence refers to different commits")
    # Release CI claims must link to a concrete Actions run, not an arbitrary
    # repository URL that happens to satisfy the generic evidence prefix.
    for key in ("combined_san_ci_ubuntu", "combined_san_ci_windows"):
        row = checks.get(key)
        if isinstance(row, dict) and row.get("outcome") == "passed":
            url = row.get("url")
            prefix = "https://github.com/NelvinBHV/CommSCraft/actions/runs/"
            if not isinstance(url, str) or not url.startswith(prefix) or not (
                url[len(prefix):].split("/", 1)[0].isdigit()
            ):
                errors.append(f"{key}: URL must reference a concrete Actions run")
    # A syntactically plausible evidence URI is not independently verified
    # interoperability. CI gates may be mechanically cross-checked, but any
    # human sign-off must still be independently inspected by reviewers.
    for key in ("combined_san_ci_ubuntu", "combined_san_ci_windows"):
        row = checks.get(key)
        if not isinstance(row, dict) or row.get("outcome") != "passed":
            continue
        url = row.get("url", "")
        prefix = "https://github.com/NelvinBHV/CommSCraft/actions/runs/"
        if isinstance(url, str) and url.startswith(prefix):
            suffix = url[len(prefix):]
            pieces = suffix.split("/")
            if len(pieces) > 1 and (len(pieces) != 3 or pieces[1] != "job" or not pieces[2].isdigit()):
                errors.append(f"{key}: invalid GitHub Actions job link")
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
    # Development may have intentionally pending gates, but a malformed,
    # untrusted or contradictory manifest must fail even without --release.
    invalid = [error for error in errors if not error.endswith(": not independently passed")]
    if invalid and not args.release:
        for error in invalid:
            print(f"INVALID v0.30.0 evidence: {error}")
        return 1
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
