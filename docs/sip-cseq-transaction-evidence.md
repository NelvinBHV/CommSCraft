# CommSCraft: RFC 3261 CSeq transaction boundary — evidence

**Scope:** The existing offline reliable-transport non-INVITE transaction engine now rejects inconsistent or potentially injected CSeq values at the transaction boundary. It does **not** authenticate a SIP server, open TLS sockets or implement complete SIP ABNF.

| Requirement | Reference | Implementation | Executable test |
| --- | --- | --- | --- |
| UAC CSeq number must fit in unsigned 32-bit AND be strictly below 2^31 | [RFC 3261 §8.1.1.5](https://www.rfc-editor.org/rfc/rfc3261#section-8.1.1.5) | `parse_cseq`: numeric ASCII decimal, `u32` checked, `< 2^31` | `cseq_request_upper_bound_and_correlation`, `malicious_or_out_of_range_cseq_cannot_create_transaction` |
| Case-sensitive CSeq method and exact transaction matching | [RFC 3261 §§17.1.3, 20.16](https://www.rfc-editor.org/rfc/rfc3261#section-20.16) | TransactionKey from request and response; method cannot be silently normalized | `malicious_or_out_of_range_cseq_cannot_create_transaction` and existing `cannot_be_spoofed_by_wrong_branch_or_cseq` |
| Header framing must not be bypassed by constructing `SipMessage` directly | [RFC 3261 §7.3.1](https://www.rfc-editor.org/rfc/rfc3261#section-7.3.1) plus CommSCraft fail-closed policy | Reject CR/LF, NUL, VT, DEL and other control bytes before tokenization; bound CSeq to 128 bytes | `malicious_or_out_of_range_cseq_cannot_create_transaction`, `malformed_response_cseq_leaves_pending_transaction_untouched` |
| Reject malformed response without consuming pending non-INVITE REGISTER transaction | [RFC 3261 §17.1.3](https://www.rfc-editor.org/rfc/rfc3261#section-17.1.3) | `NonInviteClientTransaction::receive_response` performs `TransactionKey` validation prior to state change | `malformed_response_cseq_leaves_pending_transaction_untouched` |
| Authentication remains gated independently | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) and [MS-SIPREGE §2.2.1.4](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/affa7b0a-c039-4699-9889-1da2e585ef39) | No change to `RegisterVerificationGate`, MIC/replay sequencing or `Presence-State` parsing | Existing registration-gate and Microsoft register-events suites |

## Important boundaries

- Requiring only uppercase request-method tokens is an existing **intentional subset**, not a general RFC 3261 ABNF implementation. `REGISTER` is supported.
- The 128-byte CSeq length cap and rejection of already-unfolded header control bytes are local defensive hardening, not separately claimed Microsoft requirements.
- The unmodified pre-change `transaction.rs` is archived byte-for-byte at `docs/history/import-v0.14.0/crates/sfb-core/src/transaction.rs`. Neither `docs/evidence-v0.13.0.json` nor `docs/source-import-manifest.json` is rewritten. Original SHA-256 values continue to be verified against the archived file.
- All CI must run on standard GitHub-hosted `ubuntu-latest` or `windows-latest` runners only (CommSCraft policy).
- **Verification command:** `cargo test -p sfb-core --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `python3 scripts/check_import_integrity.py`, `python3 scripts/check_v013_evidence.py`. Test pass claims require actual CI results.
- There is no live Kerberos, mutual TLS, Skype for Business Server SE sign-in or Windows Full Client parity proof. The v1.0 release gate stays closed.
