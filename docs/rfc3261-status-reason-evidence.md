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


## C0 boundary regression expansion — 2026-10-16

| Requirement | Precise public normative source | Code/test mapping | Actual execution result |
| --- | --- | --- | --- |
| SIP-STATUS-04 | [RFC 3261 (June 2002) §7.2](https://www.rfc-editor.org/rfc/rfc3261#section-7.2) and [§25.1 TEXT-UTF8-TRIM](https://www.rfc-editor.org/rfc/rfc3261#section-25.1): control octets are not valid descriptive characters. RFC permits structured LWS between text tokens; **this parser's stricter HTAB rejection is a local fail-closed subset**, not universal SIP conformance. | `crates/commscraft-core/src/sip.rs::parse_frame`; `response_reason_phrase_rejects_control_bytes_but_allows_utf8`: explicitly generate each C0 byte except CR/LF (handled by start-line framing), plus DEL; preserve UTF-8 positive fixture. | **Source-only, CI pending** for newly amended test on PR #32. Existing CI successes above apply to prior commit `76919d2055565b9101c07531d90ee15952765f6a`, not this change. |
| MS-STATUS-04 | [MS-SIPREGE revision 17.1 (19 August 2025), §3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010): REGISTER processing and server-response semantics, separate from the generic reason-phrase grammar. | No change to Microsoft registration state; only offline inbound status-line negative test. | **No Microsoft sign-in, SA, MIC, replay, real-server or Windows Full Client parity validation.** |

Tests required on this new head: ordinary hosted `ubuntu-latest`/`windows-latest` Rust workspace, strict Clippy and existing RFC/MS evidence checks. No live traffic or credential tests authorized; no historical hash evidence, runner configuration or security bypass changed. v1.0 remains **BLOCKED**.


## CI regression incident and corrective action — 2026-10-16

The previous C0 test expansion at `064b3aa66d2e18733b63eda5fdbee59206b4ec50`, retained in documentation head `fef1c6279e34629a10140b574735ef9318d6b49a`, **failed** actual standard-hosted CI. [CI run 38842793019](https://github.com/NelvinBHV/CommSCraft/actions/runs/38842793019), Ubuntu job 116786462806 and Windows job 116786462598, both failed `Run imported Rust offline tests`; [Rust run 38842793101](https://github.com/NelvinBHV/CommSCraft/actions/runs/38842793101), Ubuntu job 116786463067 and Windows job 116786462834, failed compilation. Ubuntu compiler output was `unexpected closing delimiter: }` at `crates/commscraft-core/src/sip.rs:274`, caused by literal escaped newlines and apostrophes inserted into the loop replacement at line 245, not by protocol behavior. No test pass is claimed on these heads.

**Repair:** `crates/commscraft-core/src/sip.rs::tests::response_reason_phrase_rejects_control_bytes_but_allows_utf8` was corrected in source commit `c0a99ad311f3eea424b0cf322d904f487af2e4bc` to use real Rust lines and valid `b'\\r'` / `b'\\n'` byte literals; the intended C0/DEL negative regression cases remain. Applicable basis unchanged: [RFC 3261 (June 2002) §§7.2, 25.1](https://www.rfc-editor.org/rfc/rfc3261#section-7.2), and [MS-SIPREGE §3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010) as independent Microsoft registration context.

**Verification status:** source fix authored; required Ubuntu/Windows GitHub Actions outcomes for the corrected head are **pending** as of this documentation change. This incident is a new test-source syntax regression, not an established pre-existing baseline error or Microsoft server-interoperability failure. No historic hash artifacts, trust configuration, workflows, credentials or real network traffic changed. CommSCraft v1.0 remains **BLOCKED**.


## Verified resolution of CI syntax regression — 2026-10-16

The corrective Rust test and documentation are included in PR #32 head `7719906d68b016918b13f98ca373495ce842a91f`. GitHub Actions on **this exact head** completed successfully:

| Requirement → code → test | GitHub Actions actual evidence | Outcome |
| --- | --- | --- |
| SIP-STATUS-01/02/04, [RFC 3261 (June 2002) §§7.2, 25.1](https://www.rfc-editor.org/rfc/rfc3261#section-7.2) → `crates/commscraft-core/src/sip.rs::parse_frame` → `response_reason_phrase_rejects_control_bytes_but_allows_utf8` | [Rust workspace PR run 38843129896](https://github.com/NelvinBHV/CommSCraft/actions/runs/38843129896), Ubuntu `test` job 116787522500, Windows `windows-build` job 116787522412 | Both **success**; offline tests/build only |
| SIP-STATUS-01/02/03/04, including [MS-SIPREGE §3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010) as independent registration context | [CI PR run 38843130013](https://github.com/NelvinBHV/CommSCraft/actions/runs/38843130013): Ubuntu job 116787523037, Windows job 116787523254 | Both **success** (Rust and RFC/MS evidence workflow) |
| Standard runners and repository verification | [Repository PR run 38843129985](https://github.com/NelvinBHV/CommSCraft/actions/runs/38843129985): governance job 116787523307, Windows job 116787523501, Ubuntu job 116787523597 | All three **success** |
| Additional registration, presence and router offline gates | PR runs 38843129889, 38843129861, 38843129888 respectively | All **success** |
| Same-head push events | Runs 38843125879, 38843125816, 38843125763, 38843125718, 38843125678 | All five **success** |

The earlier `unexpected closing delimiter` compilation regression on `fef1c6279e34` is therefore **resolved at the verified head**. The extended negative C0/DEL regression test is **tested-offline**, not independently verified Microsoft interoperability. This evidence-only amendment is a **new commit**, so its own CI status is *pending* until it completes. Existing evidence-ledger bytes and historical SHA-256 records remain unchanged. Do not merge without updated-head checks, comprehensive security integration and authorized independent Windows Full Client interop/policy/recovery evidence. v1.0 remains **BLOCKED**.
