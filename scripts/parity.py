#!/usr/bin/env python3
"""Measures the feature parity with the Windows client from the function table of
docs/target-app-parity.md and derives the release version from it.

Per function (one table row): implemented as a whole = 1, in parts = 0.5, not started = 0;
functions that are not planned, need captures or wait for a legal clearance are left out.
Parity = sum / counted functions. Version: 0.<parity in whole percent>.<patch>; 1.0.0 is
reserved for full parity verified on authorized servers. A pre-release suffix (`-alpha`) stays
while nothing is `verified-interoperable`.

Usage: parity.py            prints the measurement
       parity.py --version   prints only the version core 0.<percent>
"""
import pathlib
import re
import sys

TABLE = pathlib.Path(__file__).resolve().parent.parent / "docs" / "target-app-parity.md"
EXCLUDED = ("not planned", "needs captures", "deferred")


def rows(text):
    for line in text.splitlines():
        m = re.match(r"^\|\s*(\d+)\s*\|([^|]*)\|([^|]*)\|([^|]*)\|(.*)\|\s*$", line)
        if m:
            yield int(m.group(1)), m.group(2).strip(), m.group(5).strip()


def score(status):
    s = status.lower()
    if any(s.startswith(x) for x in EXCLUDED):
        return None
    if s in ("–", "-", ""):
        return 0.0
    head = s.split("(", 1)[0]
    return 0.5 if "part" in head else 1.0


def measure(text):
    counted = whole = parts = missing = excluded = 0
    total = 0.0
    for _, _, status in rows(text):
        v = score(status)
        if v is None:
            excluded += 1
            continue
        counted += 1
        total += v
        if v == 1.0:
            whole += 1
        elif v == 0.5:
            parts += 1
        else:
            missing += 1
    return {
        "functions": counted + excluded,
        "counted": counted,
        "whole": whole,
        "parts": parts,
        "missing": missing,
        "excluded": excluded,
        "percent": int(100 * total / counted) if counted else 0,
    }


def main(args):
    m = measure(TABLE.read_text(encoding="utf-8"))
    if "--version" in args:
        print(f"0.{m['percent']}")
        return 0
    print(
        f"{m['functions']} functions, {m['counted']} counted: {m['whole']} as a whole, "
        f"{m['parts']} in parts, {m['missing']} not started, {m['excluded']} left out "
        f"– parity {m['percent']} % → version 0.{m['percent']}.x"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
