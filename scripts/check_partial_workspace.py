#!/usr/bin/env python3
"""Partial workspace check; intentionally not evidence of full v0.14.0 import."""
from pathlib import Path
import tomllib
root = Path(__file__).resolve().parent.parent
data = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
assert data["workspace"]["members"] == ["crates/commscraft-register-events"]
assert (root / "crates/commscraft-register-events/src/lib.rs").exists()
assert (root / "crates/commscraft-register-events/tests/conformance.rs").exists()
assert (root / "docs/register-events-conformance.md").exists()
print("PASS: partial root workspace bootstrap present (not full historical import)")
