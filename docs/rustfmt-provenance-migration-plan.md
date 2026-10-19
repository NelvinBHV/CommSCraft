# Issue #9: read-only pre-rustfmt provenance audit

This stage **does not run rustfmt or claim formatting success**. It prepares a verifiable baseline before any formatting migration. The original v0.14.0 manifest has 100 SHA-256 entries in `docs/source-import-manifest.json`. `scripts/check_import_integrity.py` already verifies these against archived or unchanged original files.

## Audit and execution

- `python3 scripts/audit_rustfmt_provenance.py`: checks the original SHA-256 for all 100 entries and reports how many originals reside in the immutable `docs/history/import-v0.14.0/` archive versus unchanged live paths. Fails closed if any baseline is missing or modified.
- `python3 -m unittest discover -s scripts -p 'test*.py'`: runs a positive repository audit and a negative corrupt-manifest fixture in `scripts/test_rustfmt_provenance.py`.
- `python3 scripts/check_standard_runners.py`: verifies standard-runner policy.
- `cargo fmt --all -- --check`: **not a passing gate** in current workflows; `.github/workflows/ci.yml` and `.github/workflows/rust.yml` currently mark this advisory via `continue-on-error: true`.

## Remaining acceptance criteria for Issue #9

1. Record exact failing files from a real `cargo fmt --all -- --check` job, not from assumptions.
2. For **each** live file that rustfmt would modify, archive its **pre-format current bytes**, path and SHA-256 in a separate migration manifest, in addition to the historical v0.14.0 import archive. Never overwrite existing historical copies.
3. Change integrity checkers only with tests proving they verify original historical bytes and current semantic requirements; do not edit `docs/evidence-v*.json` or `docs/source-import-manifest.json`.
4. Format the current workspace, enforce non-advisory rustfmt on **both** `ubuntu-latest` and `windows-latest`, then run offline tests, Clippy, historical evidence and standard-runner checks on the exact commit.
5. Keep Windows Full Client parity and real server interoperability as separate release blockers.

## Evidence classification

This audit is repository provenance checking only, **not** RFC 3261 / RFC 8866 conformance, authenticated sign-in, or interoperability. No production network test, release or merge is authorized.
