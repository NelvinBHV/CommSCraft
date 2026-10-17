#!/usr/bin/env python3
"""Machine-derived feature registry coverage; never a Microsoft interop claim."""
import argparse
import json
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STATUSES = ("not_started", "offline_subset", "in_development", "verified")
AREAS = ("identity", "contacts_presence", "messaging", "voice", "video_media", "meetings_sharing", "client_platform")

def summarize(root=ROOT):
    features = json.loads((root / "FEATURE_PARITY.json").read_text(encoding="utf-8"))["features"]
    cases = json.loads((root / "WINDOWS_REFERENCE_CASES.json").read_text(encoding="utf-8"))["windows_scenarios"]
    for records in (features, cases):
        if not isinstance(records, list) or any(not isinstance(r, dict) or not isinstance(r.get("id"), str) or not r["id"] for r in records):
            raise ValueError("Invalid parity records")
        ids = [r["id"] for r in records]
        if len(ids) != len(set(ids)):
            raise ValueError("Duplicate parity IDs")
    if any(f.get("status") not in STATUSES or f.get("area") not in AREAS for f in features):
        raise ValueError("Unknown status or functional area")
    known_features = {f["id"] for f in features}
    for case in cases:
        references = case.get("feature_ids")
        if not isinstance(references, list) or not references or any(
            not isinstance(fid, str) or fid not in known_features
            for fid in references
        ):
            raise ValueError("Windows reference case has missing or unknown feature IDs")
    for feature in features:
        if feature["status"] == "verified" and not feature.get("interop_evidence"):
            raise ValueError("Verified feature has no independent interop evidence")
    counts = Counter(f["status"] for f in features)
    return {
        "total_features": len(features),
        "status": {key: counts[key] for key in STATUSES},
        "areas": {area: {"total": sum(f["area"] == area for f in features),
            "offline_subset": sum(f["area"] == area and f["status"] == "offline_subset" for f in features),
            "verified": sum(f["area"] == area and f["status"] == "verified" for f in features)}
            for area in AREAS},
        "reference_cases": len(cases),
        "features_with_registered_interop_evidence": sum(bool(f.get("interop_evidence")) for f in features),
        "cases_with_registered_interop_evidence": sum(bool(c.get("interop_evidence")) for c in cases),
    }

def markdown(stats):
    n = stats["total_features"]
    s = stats["status"]
    ratio = lambda x: f"{x / n * 100:.1f}%" if n else "0.0%"
    lines = ["# Skype for Business Windows Full Client — registered feature parity", "",
        "**This is matrix coverage, not a measured interop success rate.**", "",
        f"- Baseline functions: **{n}**.",
        f"- Offline subsets: **{s['offline_subset']} / {n} ({ratio(s['offline_subset'])})**. Partial implementation only.",
        f"- Verified features in registry: **{s['verified']} / {n} ({ratio(s['verified'])})**.",
        f"- Not started: **{s['not_started']}**; in development: **{s['in_development']}**.",
        f"- Windows reference cases: **{stats['reference_cases']}**.",
        f"- Features with registered live interop evidence: **{stats['features_with_registered_interop_evidence']}**.",
        f"- Reference cases with registered live interop evidence: **{stats['cases_with_registered_interop_evidence']}**.",
        "", "| Functional area | Total | Offline partial | Verified |", "| --- | ---: | ---: | ---: |"]
    for area, values in stats["areas"].items():
        lines.append(f"| {area} | {values['total']} | {values['offline_subset']} | {values['verified']} |")
    lines += ["", "Offline subsets are not feature-complete. Source may be ahead of the declared registry; neither source presence nor green CI implies live interoperability.",
        "Exact server/client-build parity, independently reviewed security, policy-denial and recovery scenarios remain required.",
        "Regenerate with: python3 scripts/report_feature_parity.py --markdown or --json.",
        "Inputs: FEATURE_PARITY.json and WINDOWS_REFERENCE_CASES.json.", ""]
    return "\n".join(lines)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--markdown", action="store_true")
    args = parser.parse_args()
    result = summarize()
    print(json.dumps(result, ensure_ascii=False, indent=2) if args.json else markdown(result))
