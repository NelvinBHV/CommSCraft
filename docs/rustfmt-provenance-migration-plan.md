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

## Executed rustfmt diagnostic (GitHub Actions, 2026-10-19)

On PR #38 exact head `34480a954574a68d7e463a6f7eef50d22cac73c9`, standard Ubuntu CI job `118151063724` in [run 39279531812](https://github.com/NelvinBHV/CommSCraft/actions/runs/39279531812) executed `cargo fmt --all -- --check`. The log contains **735 `Diff in` hunks across 56 unique Rust files**, including SIP, SDP, Windows SSPI, transport certificate identity and tests. The step displays `success` because `continue-on-error: true`; this **does not** mean formatting passed. The six workflow conclusions are green for the audit commit, not for strict rustfmt.

Before any formatting, archive **the current pre-format bytes** of all 56 affected Rust files and a manifest of their exact SHA-256 digests; the historical import manifest and evidence ledgers must remain untouched. Then format, enforce strict Linux/Windows checks and rerun all offline/security tests on the formatted commit. Do not silently reclassify an advisory step as a pass.

## Exact file inventory from latest Ubuntu + Windows diagnostics

At head `45d86e620e7bbae03e8a644e134db7637ef8737a`, the ubuntu-latest job [118401065505](https://github.com/NelvinBHV/CommSCraft/actions/runs/39352923944) reported **735 diff hunks across 56 distinct tracked Rust files**. The windows-latest job [118401065211](https://github.com/NelvinBHV/CommSCraft/actions/runs/39352923944) independently reported **735 hunks across 56 files**. The earlier 58-file count was a diagnostic parsing overcount and is superseded by this full-path-normalized inventory. File counts are informational only; no archive or format mutation was performed.

- `apps/commscraft-inspect/src/main.rs`
- `apps/commscraft-probe/src/main.rs`
- `apps/commscraft/src/main.rs`
- `crates/commscraft-core/src/additional_meeting_offline.rs`
- `crates/commscraft-core/src/advanced_client_offline.rs`
- `crates/commscraft-core/src/auxiliary_offline.rs`
- `crates/commscraft-core/src/call_drafts.rs`
- `crates/commscraft-core/src/challenge.rs`
- `crates/commscraft-core/src/client_admin_offline.rs`
- `crates/commscraft-core/src/client_offline.rs`
- `crates/commscraft-core/src/client_operations_offline.rs`
- `crates/commscraft-core/src/client_preferences_offline.rs`
- `crates/commscraft-core/src/client_services_offline.rs`
- `crates/commscraft-core/src/collaboration_staging_offline.rs`
- `crates/commscraft-core/src/communication_drafts_offline.rs`
- `crates/commscraft-core/src/conference_media_previews_offline.rs`
- `crates/commscraft-core/src/contacts_ui_offline.rs`
- `crates/commscraft-core/src/delegate_meetings_offline.rs`
- `crates/commscraft-core/src/discovery.rs`
- `crates/commscraft-core/src/enterprise_meeting_drafts_offline.rs`
- `crates/commscraft-core/src/enterprise_preview_offline.rs`
- `crates/commscraft-core/src/final_inventory_previews_offline.rs`
- `crates/commscraft-core/src/im.rs`
- `crates/commscraft-core/src/inbound.rs`
- `crates/commscraft-core/src/interaction_offline.rs`
- `crates/commscraft-core/src/kerberos.rs`
- `crates/commscraft-core/src/kerberos_mic.rs`
- `crates/commscraft-core/src/lib.rs`
- `crates/commscraft-core/src/local_features.rs`
- `crates/commscraft-core/src/media_preflight_offline.rs`
- `crates/commscraft-core/src/meeting_controls_offline.rs`
- `crates/commscraft-core/src/meeting_entry_offline.rs`
- `crates/commscraft-core/src/preflight_offline.rs`
- `crates/commscraft-core/src/presence.rs`
- `crates/commscraft-core/src/registration.rs`
- `crates/commscraft-core/src/safety_client_previews_offline.rs`
- `crates/commscraft-core/src/sdp.rs`
- `crates/commscraft-core/src/security.rs`
- `crates/commscraft-core/src/session.rs`
- `crates/commscraft-core/src/sip.rs`
- `crates/commscraft-core/src/stream.rs`
- `crates/commscraft-core/src/tls_boundary.rs`
- `crates/commscraft-core/src/transaction.rs`
- `crates/commscraft-core/src/usability_offline.rs`
- `crates/commscraft-core/src/windows_sspi.rs`
- `crates/commscraft-core/tests/authentication_fixtures.rs`
- `crates/commscraft-core/tests/protocol_fixtures.rs`
- `crates/commscraft-core/tests/verified_registration.rs`
- `crates/commscraft-register-events/src/gated_presence.rs`
- `crates/commscraft-register-events/src/lib.rs`
- `crates/commscraft-register-events/src/sip_binding.rs`
- `crates/commscraft-router/src/lib.rs`
- `crates/commscraft-router/tests/offline_wire.rs`
- `crates/commscraft-transport/src/lib.rs`
- `crates/commscraft-transport/src/sip_certificate.rs`
- `tools/commscraft-xtask/src/main.rs`

Before formatting, capture original current bytes and SHA-256 of every listed file as a distinct immutable baseline, then verify them after archiving. Do **not** overwrite existing historical Evidence or the original import manifest. The advisory `cargo fmt` step is not yet a passing gate.

## Snapshot tool (staged, not yet executed)

`scripts/snapshot_rustfmt_baseline.py` reads the 56-path inventory above and creates `docs/history/rustfmt-baseline-2026-10-20/` with byte-for-byte current-source copies and a new independent `manifest.json` (SHA-256 plus byte count). It fails closed on missing sources, path traversal, altered snapshots and existing destination; existing original-import archives and evidence ledgers are not touched. `--verify` checks an existing snapshot without mutation. `scripts/test_rustfmt_snapshot.py` includes positive and negative offline fixtures. **The snapshot has not been generated; new scripts require exact-head CI before use.** No Rust source formatting has occurred.

Do not run the snapshot while concurrent source commits are in flight; pin the source revision first and review the generated archive and manifest before committing it. Only after successful archival and verification may formatting begin.
