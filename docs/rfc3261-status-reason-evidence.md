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


## Executed CI evidence — 2026-10-16

This follow-up supersedes the earlier **pending** verification instruction only for PR #32 head `eecb7d4827dc1813c48583b83b9bd8e8d7e24356`.

| Requirement / code / test | Actual completed workflow | Observed result |
| --- | --- | --- |
| SIP-STATUS-01/02, `crates/commscraft-core/src/sip.rs::parse_frame`, `response_reason_phrase_rejects_control_bytes_but_allows_utf8` | [Rust workspace PR run 38832357155](https://github.com/NelvinBHV/CommSCraft/actions/runs/38832357155) | **success** (workflow); offline Rust evidence only |
| SIP-STATUS-01/02/03, Rust + RFC/MS historical evidence checks | [CI PR run 38832357153](https://github.com/NelvinBHV/CommSCraft/actions/runs/38832357153) | **success**, Windows job 116753748940 and Ubuntu job 116753749092 both **success** |
| Repository governance, Rust checks | [repository PR run 38832357098](https://github.com/NelvinBHV/CommSCraft/actions/runs/38832357098) | **success** |
| Independent offline registration, presence and router gates | [SIPAE 38832357087](https://github.com/NelvinBHV/CommSCraft/actions/runs/38832357087), [presence-state 38832357079](https://github.com/NelvinBHV/CommSCraft/actions/runs/38832357079), [router 38832357183](https://github.com/NelvinBHV/CommSCraft/actions/runs/38832357183) | All **success**; no authenticated registration or Microsoft server interoperability claim |

The same head also had five successful push-triggered workflows (Rust 38832352889, repository 38832352891, SIPAE 38832353028, presence 38832352887, router 38832352894). This documentation-only follow-up commit **has not passed CI** until its own checks complete. The source change remains governed by [RFC 3261 (June 2002) §§7.2, 25.1](https://www.rfc-editor.org/rfc/rfc3261#section-7.2) and [MS-SIPREGE §3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010); this is **tested-offline**, not independently verified interoperable.

**Integration risk:** parallel security PRs #26–#31 all modify `crates/commscraft-transport/src/sip_certificate.rs`; #26 and #27 also share `docs/rfc5922-strict-san-evidence.md`. Coordinate combined regression coverage before any merge. PR #32 changes a distinct SIP core parser file. No historic evidence/SHA-256 ledger edited; no new workflow runner, credential, production network access or release signoff. Full Windows Full Client parity and v1.0 remain **BLOCKED**.
