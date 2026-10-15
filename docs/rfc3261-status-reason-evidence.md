# RFC 3261 SIP response Reason-Phrase validation

**Scope:** offline, bounded SIP parser hardening. No live SIP network traffic or authentication is implemented or proven by this change.

## Requirement → implementation → test

| ID | Authoritative requirement | Implementation | Regression test |
| --- | --- | --- | --- |
| SIP-STATUS-01 | [RFC 3261 §7.2](https://www.rfc-editor.org/rfc/rfc3261#section-7.2): SIP responses carry a Status-Line; no CR/LF except its terminator | `crates/commscraft-core/src/sip.rs`, `parse_frame`: refuse C0 control bytes and DEL in the response Reason-Phrase | `response_reason_phrase_rejects_control_bytes_but_allows_utf8`: NUL, TAB, ESC, DEL denied |
| SIP-STATUS-02 | [RFC 3261 §25.1](https://www.rfc-editor.org/rfc/rfc3261#section-25.1): Reason-Phrase uses descriptive TEXT-UTF8-TRIM, including UTF8-NONASCII | `parse_frame` retains valid Unicode descriptive text after UTF-8 decoding | same test: `Erfolgreich ✓` accepted without treating it as an authentication signal |
| SIP-STATUS-03 | [MS-SIPREGE §3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010): REGISTER processing, security association, server responses | Parser only validates syntax. It does **not** claim Microsoft registration conformance, server MIC or SA verification | existing REGISTER verification gate tests remain separately required |

**Why:** Previously `parse_frame` checked the numeric status code but copied Reason-Phrase from incoming frames without rejecting embedded ASCII controls. This allowed a nonconforming control-bearing status line to be represented as a parsed SIP response. UTF-8 descriptive text remains acceptable, as distinct from control characters.

**Verification:** Run `cargo test -p commscraft-core --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings` and required historical RFC/Microsoft SHA-256 checkers, on ordinary GitHub-hosted `ubuntu-latest` and `windows-latest` only. Do not claim any check passed until the workflow completes for the PR head. Do not alter historical evidence ledgers.

**Limitations:** This is intentionally *not* a complete RFC 3261 ABNF parser. Status text is never authoritative for authentication. MS-SIPAE Kerberos/SSPI, production REGISTER and Windows Full Client parity are unverified; CommSCraft v1.0 remains BLOCKED.
