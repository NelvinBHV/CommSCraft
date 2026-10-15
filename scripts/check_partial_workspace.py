#!/usr/bin/env python3
"""Assert the complete imported Rust workspace structure, not interop."""
from pathlib import Path
import json
import tomllib

root = Path(__file__).resolve().parent.parent
data = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
expected = {
    "crates/sfb-core", "crates/sfb-transport", "crates/sfb-router",
    "crates/commscraft-register-events", "apps/commscraft",
    "apps/sfb-inspect", "apps/sfb-probe", "xtask",
}
actual = data["workspace"]["members"]
assert len(actual) == len(set(actual)) == 8, actual
assert set(actual) == expected, (set(actual) ^ expected)
for member in actual:
    assert (root / member / "Cargo.toml").is_file(), member
desktop = tomllib.loads((root / "apps/commscraft/Cargo.toml").read_text())
assert desktop["package"]["name"] == "commscraft"
assert desktop["bin"][0]["name"] == "CommSCraft"
for path in (
    "crates/commscraft-register-events/tests/conformance.rs",
    "docs/register-events-conformance.md",
    "docs/source-import-manifest.json",
    "FEATURE_PARITY.json",
    "WINDOWS_REFERENCE_CASES.json",
    "V1_RELEASE_CONTRACT.json",
    "scripts/check_import_integrity.py",
):
    assert (root / path).is_file(), path
ledger = json.loads((root / "docs/source-import-manifest.json").read_text())
assert ledger["file_count"] == len(ledger["files"]) == 100
print("PASS: eight-package imported Rust workspace and original evidence manifest present")
print("NOT a Skype for Business server interoperability or v1.0 parity test")
