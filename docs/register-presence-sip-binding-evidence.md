# CommSCraft REGISTER Presence-State SIP binding: offline evidence

**Scope:** `crates/commscraft-register-events/src/sip_binding.rs` is a strict **untrusted response parser**, not live registration or user authentication.

| Requirement | Source | Rust check | Authored test |
| --- | --- | --- | --- |
| Parse response, not request | [RFC 3261 §8.1.3](https://www.rfc-editor.org/rfc/rfc3261#section-8.1.3) | `StartLine::Response` check | `request_never_becomes_a_register_success` |
| CSeq is single, valid numeric and associates response with REGISTER method | [RFC 3261 §§8.1.1.5, 20.16](https://www.rfc-editor.org/rfc/rfc3261#section-20.16) | CSeq cardinality, decimal range below 2^31 (outgoing request constraint), exact method | `malformed_and_duplicate_cseq_fail_closed` (including 2^31 boundary), `only_register_200_returns_hint` |
| Microsoft `Presence-State` only interpreted for successful REGISTER | [MS-SIPREGE §2.2.1.4](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/affa7b0a-c039-4699-9889-1da2e585ef39) | Delegate to bounded parser on REGISTER 200 | `duplicate_presence_state_is_rejected` |

**Security boundary:** The SIP response is still untrusted. CSeq does not prove an outstanding transaction; the caller must separately verify RFC 3261 transaction matching and the trusted TLS channel, and use [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) MIC/replay checks before any authenticated state transition. The Microsoft Presence-State fields are not certified by these tests.

The public crate API exposes `parse_register_presence_response` and `BindingError`; the security boundary remains untrusted.

**Test command:** `cargo test -p commscraft-register-events --all-targets` on Linux and Windows. CI execution evidence is obtained only after GitHub checks complete. This patch does not alter the historical SHA-256 source evidence or the v1.0 release gate.

## RFC 3261 CSeq upper boundary

RFC 3261 [§8.1.1.5](https://www.rfc-editor.org/rfc/rfc3261#section-8.1.1.5) requires UAC request CSeq values to be less than `2^31`; [§20.16](https://www.rfc-editor.org/rfc/rfc3261#section-20.16) requires an unsigned 32-bit decimal representation and a case-sensitive method. The new offline response binding applies the stricter **corresponding request** rule: `2147483647 REGISTER` is accepted, `2147483648 REGISTER` is rejected. No claim is made that these checks authenticate a SIP transaction.

The strict Clippy failure on a nested duplicate-CSeq guard was corrected without weakening duplicate detection. Run `cargo clippy --workspace --all-targets -- -D warnings` in Linux/Windows CI.
