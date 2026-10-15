# Historical source import: CommSCraft v0.14.0

The prepared v0.14.0 Rust workspace is now published on the feature branch for [Pull Request #8](https://github.com/NelvinBHV/CommSCraft/pull/8).

The source import includes all original Rust crates, apps, CLI tools, Python source-contract checks, test fixtures, Windows Full Client parity inventories and historical Microsoft/RFC evidence ledgers. It is **not yet merged into main** until GitHub Actions checks succeed and the import is reviewed.

See `docs/source-import-manifest.json` for the 100 original SHA-256 file checks. Modified files are compared against immutable historical copies under `docs/history/import-v0.14.0/`; the older evidence ledgers are never rewritten. `scripts/check_import_integrity.py` performs those checks.

The already-merged `commscraft-register-events` crate is now a member of the combined Cargo workspace.

## Verification status
- The SIP core and REGISTER response gate have passing **offline Rust tests on Linux and Windows** in GitHub Actions; consult the individual PR checks for current build and lint status.
- All remaining workspace builds, compiler and lint warnings must be resolved before merging.
- Live SIP REGISTER, Kerberos mutual authentication on a real server, presence/IM interoperability and full Windows-client parity **are not established**.
- `cargo fmt --all -- --check` is a known source migration debt, reported as advisory during import only. The v1.0 release gate still requires it to pass.

Avoid publishing real user credentials, enterprise server identities, unredacted SIP traces or personal contact data.
