# SIP URI SAN duplicate parameter-name policy — RFC/MS traceability

**Status:** code and offline tests authored; CI execution is pending. This document is not an interoperability or authenticated-login claim.

| Evidence | Specification | Implementation | Test |
| --- | --- | --- | --- |
| SAN-DUP-01 | [RFC 3261 §19.1.1](https://www.rfc-editor.org/rfc/rfc3261#section-19.1.1): any given SIP URI parameter-name MUST NOT appear more than once | `crates/commscraft-transport/src/sip_certificate.rs`, `sip_uri_host`: reject repeated semicolon URI parameter names before considering certificate URI SAN as an authorized SIP identity | `duplicate_sip_uri_parameter_names_cannot_authorize_certificate_identity` |
| SAN-DUP-02 | [RFC 3261 §25.1](https://www.rfc-editor.org/rfc/rfc3261#section-25.1): URI parameters precede the '?' header component; header parsing is independent | Duplicate check uses only authority/parameter prefix; distinct names remain accepted; URI headers do not become URI parameters | Same Rust test, positive header and multi-parameter cases |
| SAN-DUP-03 | [RFC 5922 §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1), [RFC 5280 §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280#section-4.2.1.6): certificate SIP URI SAN is an identity input, not a caller-provided trust assertion | Existing platform-verified TLS peer certificate and strict SAN precedence unchanged | Existing DER and SAN precedence tests remain required |
| SAN-DUP-04 | Microsoft [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c): server Security Association, MIC and replay require independent verification | No change to authentication state, REGISTER, SSPI or SIP sending | Existing registration-gate suites remain required |

**Scope notes:** Case-insensitive duplicate-name comparison is an intentionally conservative security policy at this certificate identity boundary. Empty parameter names, malformed parameter syntax and encoded controls are tracked separately in open PRs #26, #27 and #29. This change does not claim complete SIP URI parsing or full RFC 5922 compliance.

**Verification required before merge:** `cargo test -p commscraft-transport --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, existing Python RFC/MS historical evidence checks, Linux and Windows using only standard GitHub-hosted `ubuntu-latest` / `windows-latest`. Preserve historical SHA-256 ledgers. All CI checks must be verified green for the final SHA.

**Release status:** Live SfB Server SE TLS/SSPI Kerberos binding, SIPAE server MIC/replay, real REGISTER and Windows Full Client interoperability remain unverified. v1.0 remains BLOCKED.


## Executed CI and integration follow-up — 2026-10-16

The original *CI pending* status above applies to the initial authored source; it is superseded **for PR #31 head `4b1898aedce5e98929d723e9f58103c81d24fb9e`** by the following observed GitHub Actions results. This new documentation-only commit is not represented as having passed CI until its own checks complete.

| Source-to-test requirement | Workflow / actual job result | Evidence classification |
| --- | --- | --- |
| SAN-DUP-01/02: [RFC 3261 (June 2002) §§19.1.1, 25.1](https://www.rfc-editor.org/rfc/rfc3261#section-19.1.1) → `crates/commscraft-transport/src/sip_certificate.rs::sip_uri_host` → `duplicate_sip_uri_parameter_names_cannot_authorize_certificate_identity` | [Rust workspace PR run 38738002780](https://github.com/NelvinBHV/CommSCraft/actions/runs/38738002780): **success** | CI-tested offline, not a live TLS trust demonstration |
| SAN-DUP-03/04: [RFC 5922 (June 2010) §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1), [RFC 5280 (May 2008) §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280#section-4.2.1.6), [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) | [CI PR run 38738002764](https://github.com/NelvinBHV/CommSCraft/actions/runs/38738002764): Windows job 116432125552 **success**, Ubuntu job 116432125791 **success** | Rust and historical RFC/MS evidence checks; neither authenticated sign-in nor verified SfB interoperation |
| Repository runner/governance and workspace | [Repository run 38738002870](https://github.com/NelvinBHV/CommSCraft/actions/runs/38738002870): Windows job 116432126198 **success**, Ubuntu job 116432126327 **success**, governance job 116432126702 **success** | Hosted standard Windows/Ubuntu offline CI |
| Other PR-triggered gates | [Presence 38738002726](https://github.com/NelvinBHV/CommSCraft/actions/runs/38738002726), [router 38738002790](https://github.com/NelvinBHV/CommSCraft/actions/runs/38738002790), [SIPAE 38738002818](https://github.com/NelvinBHV/CommSCraft/actions/runs/38738002818) | All three workflows **success**; protocol interoperability remains unverified |

All five observed push-triggered workflows for the same head also reported success: 38737994412 (presence), 38737994132 (SIPAE), 38737994435 (router), 38737994069 (Rust), and 38737994458 (repository).

**Integration inventory / next executable task:** Open PRs #26, #27, #28, #29, #30, and #31 all edit `crates/commscraft-transport/src/sip_certificate.rs`, and #26/#27 additionally edit `docs/rfc5922-strict-san-evidence.md`. Passing CI for their *individual* heads does **not** establish the behavior of a combined patch. Before any integration, reconcile URI header syntax, percent-encoded control handling, DNS/IP policy, parameter grammar, URI IA5 bounds, and duplicate parameter rejection on one coordinated branch; run the complete combined negative SAN identity fixtures and Linux/Windows CI. Keep relevant existing PRs open pending this proof; no unapproved merge to `main`.

No historical SHA-256 ledgers, workflows, permissions, TLS bypasses or network credentials changed here. Independent certificate chain/hostname validation, real Microsoft SIPAE MIC/replay/SSPI and full Windows client policy/error/recovery parity still require separate authorization and evidence. CommSCraft v1.0 remains **BLOCKED**.


## Encoded duplicate-name hardening — 2026-10-16

| Requirement | Public normative basis | Source and regression fixture | Execution |
| --- | --- | --- | --- |
| SAN-DUP-05 | [RFC 3261 (June 2002) §19.1.1](https://www.rfc-editor.org/rfc/rfc3261#section-19.1.1): SIP URI parameter names do not repeat; [§19.1.4](https://www.rfc-editor.org/rfc/rfc3261#section-19.1.4): unreserved characters compare equal to percent-encoded representations. | `crates/commscraft-transport/src/sip_certificate.rs::sip_uri_host` canonicalizes percent-encoded unreserved ASCII parameter-name bytes before duplicate-name comparison; reserved encoded characters retain their distinct form. New `percent_encoded_unreserved_duplicate_parameters_are_rejected` covers `transport/%74ransport`, `foo/f%6Fo`, `%6Cr/lr`, case-insensitive hyphen escape, plus distinct-name acceptance. | **Source authored only** at `e3ece71881b96ba68b06d8482affe3e841a6956c`; do not assert success until the final-head Ubuntu/Windows actions finish. |
| SAN-DUP-06 | [RFC 5922 (June 2010) §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1) and [RFC 5280 (May 2008) §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280#section-4.2.1.6), certificate SIP URI SAN identity; [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) independently requires security association, MIC and replay verification. | Offline certificate identity parser only; no new trust bypass, TLS chain policy or SSPI/REGISTER behavior. | Microsoft interoperability **unverified**. |

**Integration warning:** PRs #26–#31 overlap `sip_certificate.rs`; passing individual CI does not prove their combined security behavior. Coordinate URI parsing/test integration before any merge. Historical SHA-256 ledgers unchanged. Only standard GitHub-hosted Ubuntu/Windows runners are permitted; no live authentication or production testing. v1.0 remains **BLOCKED**.


## Verified encoded-alias CI and matcher-level test extension — 2026-10-17

| Requirement and normative reference | Source → regression fixture | Observed executed outcome |
| --- | --- | --- |
| SAN-DUP-05, [RFC 3261 (June 2002) §§19.1.1, 19.1.4](https://www.rfc-editor.org/rfc/rfc3261#section-19.1.4): duplicate SIP URI parameter names and unreserved-percent-escape equivalence | `crates/commscraft-transport/src/sip_certificate.rs::sip_uri_host` → `percent_encoded_unreserved_duplicate_parameters_are_rejected` | Previous head `64938ce30f84c02ddb1602dac0a5c9a95f83bb3d`: [CI run 38899514389](https://github.com/NelvinBHV/CommSCraft/actions/runs/38899514389), Windows job 116967995575 **success**, Ubuntu job 116967995742 **success**; [repository run 38899514446](https://github.com/NelvinBHV/CommSCraft/actions/runs/38899514446), governance 116967995776, Windows workspace 116967995901, Ubuntu workspace 116967996000 all **success**; Rust workspace 38899514384 and SIPAE/presence/router PR gates 38899514420/38899514400/38899514390 also **success**. Five same-head push workflows succeeded. **Offline CI only**. |
| SAN-DUP-07, same RFC 3261 rules plus [RFC 5922 (June 2010) §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1) certificate SIP URI SAN identity | Extended the *same* Rust regression fixture with mixed-case name/%HEX escape aliases and `match_sip_sans` negative assertions, at source commit `1a421ad60692607bea899614f614c06fd320dff0`. | **Source authored; updated-head CI pending.** The prior-head successful runs do not prove these new assertions passed. |
| SAN-DUP-08, [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) | No live Security Association, MIC/replay validation or Kerberos code changes | Real Microsoft interoperability and complete Windows Full Client parity **unverified**. |

The further documentation-only commit requires its own CI. No historical SHA-256 ledgers, trust bypass, runner policy, workflows or network tests were changed. Coordinate overlapping PRs #26–#31 before integration; v1.0 **BLOCKED**.


## Verified matcher CI and further alias regression — 2026-10-17

| Requirement and public specification | Code path and offline fixture | Actual execution |
| --- | --- | --- |
| SAN-DUP-05/07: [RFC 3261 (June 2002) §§19.1.1, 19.1.4](https://www.rfc-editor.org/rfc/rfc3261#section-19.1.4), [RFC 5922 (June 2010) §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1) | `crates/commscraft-transport/src/sip_certificate.rs::sip_uri_host`, `match_sip_sans`, `percent_encoded_unreserved_duplicate_parameters_are_rejected`, including case/HEX aliases | PR #31 head `832513c1e208eb131236a646b89b343c81734125`: [CI run 38994581295](https://github.com/NelvinBHV/CommSCraft/actions/runs/38994581295) Ubuntu job 117284188813 **success**, Windows job 117284188963 **success**; [repository run 38994581445](https://github.com/NelvinBHV/CommSCraft/actions/runs/38994581445) Ubuntu job 117284189489 **success**, Windows job 117284189583 **success**, governance job 117284189707 **success**. Other PR-triggered Rust, presence, SIPAE, router runs and five push-triggered runs all **success**: total **11/11**. |
| SAN-DUP-09: same RFC 3261 rules | The same test now also covers `param1/param%31` and `key/%6bey` aliases, source commit `3d8336b1de995e7693f26ba76aaa34636d57b18e`. | **Source authored; final-head CI pending**; previous green head cannot verify these new assertions. |
| SAN-DUP-08: [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) | Independent server SA/MIC/replay; no new implementation or live connection | Microsoft interoperability **unverified**; Windows Full Client v1.0 **BLOCKED**. |

Only offline regression coverage and evidence were changed; historical SHA-256 records, workflow permissions, runner configuration, and credential handling remain untouched. Since PRs #26–#31 modify the same certificate module, individual successful CI is not integrated full coverage. This documentation commit itself requires updated-head CI confirmation.
