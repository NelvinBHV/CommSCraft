# v0.11.0 — REGISTER response verification: normative traceability

**Status:** offline integration authored; Rust and native execution **NOT verified** in this authoring environment. Real SfB interoperability **NOT verified**. No 1.0 feature parity claimed.

## Direct specifications and captured requirements

| ID | Source / exact subsection | Requirement derived | Implementation | Rust test (authored, NOT executed here) |
|---|---|---|---|---|
| REG-MIC-01 | [RFC 3261 §17.1.2](https://www.rfc-editor.org/rfc/rfc3261) | Non-INVITE REGISTER 200 must match active Via/Call-ID/CSeq transaction | `registration.rs::observe_mic_checked_200` clones and preflights transaction | `a_mismatched_transaction_cannot_consume_a_valid_signed_sequence` |
| REG-MIC-02 | [MS-SIPAE §3.2.5.2 steps 1–4](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) | Match SA identity, construct exact signed buffer, verify MIC, discard on failure | `security.rs::verify_server_message` invoked only AFTER matching transaction and binding preflight | `a_bad_mic_never_consumes_transaction_or_replay_counter`, `missing_authentication_info_is_not_a_signed_acceptance` |
| REG-MIC-03 | [MS-SIPAE §3.2.5.2 steps 5–7](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) | Validate 257-number replay window only AFTER successful MIC, commit established SA after checks | Same helper in `security.rs`; `registration.rs` commits only if verification succeeded | `commits_only_after_transaction_binding_and_mic`, bad MIC and mismatched transaction tests |
| REG-MIC-04 | [RFC 3261 §10.2.4](https://www.rfc-editor.org/rfc/rfc3261) | Refresh only own binding before its expiry; compare `Contact` bindings and expiry | `registration.rs::matching_binding`, monotonic `expires_at_ms`; separate 80%-lifetime **local heuristic** | `commits_only_after_transaction_binding_and_mic`, `rejects_bad_endpoint_binding_without_consuming_sa_replay` |
| REG-MIC-05 | [MS-SIPREGE §4.1.1 Basic Registration](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/bc4b0245-339d-490c-ba67-2410e1513ed9) | Microsoft `+sip.instance` and `gruu` in REGISTER 200; GRUU reported via Contact | Matching self-instance; GRUU returned only as `untrusted_gruu` | `contact_gruu_is_not_signed_by_sipae_and_must_remain_untrusted` |
| REG-MIC-06 | [MS-SIPREGE §4.1.2 Basic Unregistration](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/1178018e-277b-42a3-a5fb-1a63049e0b50) | Explicit self-binding removal with `Expires: 0` | Existing `plan_unregistration`, new MIC-gated response check | `signed_removal_is_not_assumed_from_unsigned_removal` |
| REG-MIC-07 | [MS-SIPAE §3.2.5.2 step 2 fields 15–16](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) | `Expires` and response code are MIC-covered, **but Contact is not** | Reject Contact-specific expiration different from signed `Expires` in the strict supported subset | `rejects_contact_expiry_different_from_signed_expires`, GRUU-tamper test |
| REG-MIC-08 | RFC 3261 §10.2.4 + explicit local policy | Client should refresh its own binding before expiry | local recommended refresh at 80% only, not normative | `commits_only_after_transaction_binding_and_mic` |
| REG-MIC-09 | MS-SIPAE + fail-closed regression requirement | Older unsigned observer must remain untrusted and cannot get promoted by mistake | `RegisterState::UnverifiedAcceptance` separate from `MicCheckedAcceptance` | `bare_observe_200_still_means_unverified_acceptance` |

## Narrow compatibility and security constraints

1. **Contact and GRUU are NOT in the MS-SIPAE signed field list.** An offline HMAC does not certify them. TLS peer identity, full TLS channel integrity, authenticated routing policy, and any Contact/GRUU usage require separate implementation and audit. The test deliberately changes `Contact` while keeping the original valid MIC to demonstrate this property.
2. **Strict supported subset:** for a success with a Contact binding the local parser requires Contact expiration equal to the MIC-protected `Expires`. RFC 3261 permits more general per-Contact behavior; that branch is unsupported rather than guessed. We do not claim full RFC 3261 compliance from this subset.
3. Offline HMAC material in Rust tests is **synthetic**. No real TLS-DSK key derivation, GSS context establishment, service principal validation, authenticated network transaction, presence, or IM has been completed by this integration.
4. Interoperability may differ across MS-SIPAE versions 3 and 4 and Microsoft server builds. The tested documentation baselines are **MS-SIPAE revision 11.0 (2025-05-20)** and **MS-SIPREGE revision 17.1 (2025-08-19)**. For actual network enablement, repeat compatibility tests on authorized SfB Server SE and compare sanitized traffic against a pinned Windows Full-client build.

## Local verification commands and current evidence level

```sh
cargo test -p commscraft-core --test verified_registration
cargo test --workspace --all-targets
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/check_release.py
python3 scripts/check_release.py --release  # EXPECT failure: full 1.0 parity not proven
python3 scripts/check_v011_evidence.py
```

- `scripts/check_v011_evidence.py` verifies file hashes of code/tests, coverage IDs, the declared status, and expected missing runtime evidence. It is **not** a Rust compiler.
- Rust unit and integration tests: **NOT RUN** in this authoring environment (no Rust toolchain).
- Windows SSPI integration: **NOT RUN** (no domain-joined Windows lab).
- Live TLS transport/registrar: **NOT CONNECTED**.
- Windows Full-client parity: **NOT VERIFIED** (1.0 release gate must stay closed).
