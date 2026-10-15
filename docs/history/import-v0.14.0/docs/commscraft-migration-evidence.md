# CommSCraft migration evidence

CommSCraft continues the existing Skype for Business client Rust research prototype at version **0.14.0**. The public project name and repository URL were changed, without asserting runtime interoperability. Internal `sfb-*` packages remain stable.

The following pre-migration source bytes are preserved so the earlier evidence is auditable after rebranding:

- `docs/history/v0.13.0/Cargo.toml` — original workspace repository URL, verified against `docs/evidence-v0.13.0.json`.
- `docs/history/v0.14.0/README.md` — original main README, verified against `docs/evidence-v0.14.0.json`.
- `docs/history/v0.13.0/check_v013_evidence.py` and `docs/history/v0.14.0/check_v014_evidence.py` — original checkers; active checker variants map to immutable archived bytes for migrated paths.

The original per-version ledgers are retained unmodified. The active checkers map changed root paths and their own historical source copies to archived original bytes; no hash digest in the old ledgers was rewritten. The original Rust protocol modules and their MS/RFC evidence have not been altered by the branding update, other than the GUI's visible application title.

**Not verified:** Rust compilation, native SSPI execution, server interoperation, Windows feature parity. The v1.0 release gate must remain closed.
