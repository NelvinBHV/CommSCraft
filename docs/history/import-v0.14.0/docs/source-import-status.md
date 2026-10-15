# CommSCraft historical source import: complete in PR #8

**Scope:** The original CommSCraft v0.14.0 Rust workspace was imported into [PR #8](https://github.com/NelvinBHV/CommSCraft/pull/8) and reconciled with the existing `commscraft-register-events` module from the main branch. The live workspace contains eight packages across three main protocol crates, one Microsoft REGISTER-events crate, three application binaries and `xtask`.

**Source provenance:** `docs/source-import-manifest.json` lists the original 100 source/test/documentation files and SHA-256 digests. Earlier RFC/MS evidence ledgers are unchanged. Source changes made for compilation, tests, linting and merge integration are preserved with immutable historical snapshots under `docs/history/import-v0.14.0/`. Verify using `python3 scripts/check_import_integrity.py`.

**Verification:** The import branch ran compiled offline Rust protocol tests on Linux and Windows. See [PR #8 Actions](https://github.com/NelvinBHV/CommSCraft/pull/8/checks) for exact results and the relevant commit SHA. `cargo fmt --all -- --check` remains a known quality task in [Issue #9](https://github.com/NelvinBHV/CommSCraft/issues/9): its advisory status is not proof of passing formatting.

**Normative sources:** [RFC 3261 §10](https://www.rfc-editor.org/rfc/rfc3261#section-10) on REGISTER and [§17.1.3](https://www.rfc-editor.org/rfc/rfc3261#section-17.1.3) on response matching; [MS-SIPREGE §2.2.1.4](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/affa7b0a-c039-4699-9889-1da2e585ef39) on `Presence-State`; [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) on MIC and replay processing.

**Safety and release:** No live end-to-end MS-SIPAE/Kerberos registration, actual SfB Server SE interoperability, working calls or full Windows-client feature parity has been established. Version 1.0 remains blocked by the strict evidence gate. The 122-case feature inventory and 93 Windows-reference tests remain a coverage **plan**, not completed functions.

This replaces the previous partial-workspace bootstrap status while retaining its historical content in Git history.

## Current CommSCraft desktop identity

The previously named desktop prototype now lives at `apps/commscraft/`, uses the Cargo package `commscraft` and builds the binary `CommSCraft`. The user-facing application is **CommSCraft**. See `docs/commscraft-migration-evidence.md` for preserved historical paths and hashes. This package rename does not validate a live Skype for Business sign-in.
