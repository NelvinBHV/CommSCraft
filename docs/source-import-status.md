# Source import status — partial root workspace

- **Current GitHub artifact:** root Cargo workspace containing **only** `crates/commscraft-register-events`; see `Cargo.toml`.
- **Prepared historical local source:** CommSCraft v0.14.0 archive, SHA-256 `fce47367eba3cf1e1f416e557028447c61e5f65254e3adbfec8c1153c60bbfad`.
- **Historical source import:** **NOT DONE**. The archive contains additional apps, crates, fixtures, Python tests, feature matrices and history/evidence files. Its existence outside the repo does not prove that the GitHub tree contains those files.
- **Earlier local checks:** 44 Python unit tests and hash checks passed; no Rust toolchain was available locally. These were **not** real server interoperability tests.
- **Current executable scope:** offline REGISTER `Presence-State` parser only, with new conformance integration tests. Successful GitHub CI will prove build/test success for this limited scope only.
- **v1.0 release:** **BLOCKED** pending verified Windows Full Client parity and full historical source import.

## RFC/MS traceability and migration acceptance

Import the unmodified historical `FEATURE_PARITY.json`, `WINDOWS_REFERENCE_CASES.json`, `V1_RELEASE_CONTRACT.json`, `Cargo.toml`, `apps/`, `crates/sfb-*`, `xtask/`, `tests/`, `scripts/` and `docs/evidence-v*.json`. Preserve original SHA-256 evidence, recording renamed files in immutable history instead of rewriting old ledgers.

After import run:
- `python3 scripts/check_commscraft_migration.py`
- `python3 -m unittest discover -s scripts -p 'test*.py'`
- `cargo test --workspace --all-targets` on Linux and Windows
- `cargo clippy --workspace --all-targets -- -D warnings`
- full signed-off Windows-to-Rust, Rust-to-Windows and server authentication interoperability tests

Normative references: [MS-SIPREGE 2.2.1.4](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/affa7b0a-c039-4699-9889-1da2e585ef39), [MS-SIPREGE 3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010), [MS-SIPAE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd), [RFC 3261 §10](https://www.rfc-editor.org/rfc/rfc3261#section-10).
