# CommSCraft v0.14.0 GitHub source import — evidence

**Source:** previously prepared CommSCraft v0.14.0 project tree. This import preserves 100 historical source/test/evidence files; the live workspace manifest adds the already-merged `commscraft-register-events` crate. The unmodified historical `Cargo.toml` is stored at `docs/history/import-v0.14.0/Cargo.toml`, and `docs/source-import-manifest.json` provides the original SHA-256 hashes.

## Imported code and normative sources

| Layer | Imported path | Normative source | Proof standard |
| --- | --- | --- | --- |
| SIP transaction routing | `crates/sfb-core/src/transaction.rs`, `crates/sfb-router/` | [RFC 3261 §17.1.3](https://www.rfc-editor.org/rfc/rfc3261#section-17.1.3) | Offline test must execute on Linux/Windows |
| REGISTER states/response gate | `crates/sfb-core/src/registration.rs`, `session.rs` | [RFC 3261 §10](https://www.rfc-editor.org/rfc/rfc3261#section-10), [MS-SIPREGE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/) | Server+client validation still missing |
| Microsoft security associations | `crates/sfb-core/src/security.rs`, `windows_sspi.rs` | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c), [RFC 2743](https://www.rfc-editor.org/rfc/rfc2743) | Windows SSPI runtime and mutual auth still missing |
| TLS transport | `crates/sfb-transport/` | [RFC 3261 §18](https://www.rfc-editor.org/rfc/rfc3261#section-18), [RFC 3263](https://www.rfc-editor.org/rfc/rfc3263) | Real SfB transport interoperability still missing |
| REGISTER Presence-State parser | `crates/commscraft-register-events/` (preexisting) | [MS-SIPREGE §2.2.1.4](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/affa7b0a-c039-4699-9889-1da2e585ef39) | Separately compiled Linux/Windows tests previously confirmed |

## Evidence policy

- The original `docs/evidence-v0.10.0.json` through `docs/evidence-v0.14.0.json` are carried forward **byte-for-byte**, not updated to claim tests that did not run when authored.
- Historical source/README/checker copies remain in `docs/history/`. The original v0.14.0 root workspace manifest is also preserved.
- The added `scripts/check_import_integrity.py` compares SHA-256 of every imported source file with the original, consulting the immutable history copy for the one intentional Cargo workspace change.
- `python3 -m unittest discover -s scripts -p 'test*.py'` previously passed 44/44 on the local prepared tree; this is **source-contract testing, not Rust compilation or server interoperability**.
- A GitHub Actions Rust build and workspace tests are REQUIRED to convert the status from `source-only` to `compiled/offline-tested`. Until a green relevant job exists, they are **unverified**.
- MS-SIPAE server authentication, a live REGISTER and complete Windows Full Client feature parity remain **unverified and nonfunctional end-to-end**. Do not tag v1.0.

## Public document revision references

Microsoft's published [MS-SIPAE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd) version 11.0 (2025-05-20) and [MS-SIPREGE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d) version 17.1 (2025-08-19) are the modern protocol reference starting points. Earlier Microsoft-specific draft-based behavior must be traced to the particular historical implementation rather than assumed to match newer RFCs.

This import does not independently attest that every previous implementation is normatively correct.
