# CommSCraft migration evidence

CommSCraft continues the existing Skype for Business client Rust research prototype at version **0.14.0**. The public project name and repository URL were changed, without asserting runtime interoperability. The native desktop application has been renamed to **CommSCraft** (`apps/commscraft`, Cargo package `commscraft`, executable `CommSCraft`). Historical internal SIP protocol and diagnostic package identifiers may retain `sfb-*` for API and evidence compatibility.

The following pre-migration source bytes are preserved so the earlier evidence is auditable after rebranding:

- `docs/history/v0.13.0/Cargo.toml` — original workspace repository URL, verified against `docs/evidence-v0.13.0.json`.
- `docs/history/v0.14.0/README.md` — original main README, verified against `docs/evidence-v0.14.0.json`.
- `docs/history/v0.13.0/check_v013_evidence.py` and `docs/history/v0.14.0/check_v014_evidence.py` — original checkers; active checker variants map to immutable archived bytes for migrated paths.

The original per-version ledgers are retained unmodified. The active checkers map changed root paths and their own historical source copies to archived original bytes; no hash digest in the old ledgers was rewritten. The original Rust protocol modules and their MS/RFC evidence have not been altered by the branding update, other than the GUI's visible application title.

**Not verified:** Rust compilation, native SSPI execution, server interoperation, Windows feature parity. The v1.0 release gate must remain closed.

## Desktop product identity migration

The active application is **CommSCraft**, not the historical `sfb-desktop` package. `cargo run -p commscraft` launches the offline desktop prototype; the Cargo `[[bin]]` target is explicitly named `CommSCraft` (Windows artifact `CommSCraft.exe`). The window title and application header already display CommSCraft.

For auditability, `docs/history/import-v0.14.0/apps/sfb-desktop/Cargo.toml` contains the exact historical pre-rename package manifest. The previous desktop `main.rs` was already preserved under the corresponding historical path. `docs/source-import-manifest.json`, previous SHA-256 values and RFC/MS specification records are **not rewritten**. The active integrity checker maps historical file locations to those original bytes; the new desktop package is separately asserted by `scripts/test_commscraft_branding.py`.

This is a **naming and packaging change only**. It does not claim live Kerberos authentication, Microsoft SIP REGISTER interoperability, a working contact/IM service, or Windows Full Client parity.
