# DNS/SRV and SIP REGISTER: clause-scoped offline audit — 2026-10-18

Scope: PR #35, source-only/offline behavior. This report does not attest to Microsoft Windows Full Client interoperability or successful authenticated sign-in.

## DNS/SRV

* **RFC 2782 (February 2000), §3** — Process SRV records in ascending numeric priority groups, use weighted selection within a group, and treat a single-dot target as absence of service. Source: `crates/commscraft-core/src/discovery.rs`, `order_srv` and `SrvRecord::new`. Fixtures: `weight_and_priority_follow_rfc2782`, `srvs_with_equal_priority_never_precede_a_lower_priority_group` and `dot_target_is_not_a_connectable_endpoint`. Negative defensive fixture: `invalid_rng_draw_fails_closed_without_modulo_bias_or_panic` (invalid RNG contract is an application-specific fail-closed choice, **not** an RFC requirement). URL: https://www.rfc-editor.org/rfc/rfc2782#section-3 .
* **MS-CONMGMT §3.1.5** — Four published Microsoft SRV query names and precedence are represented by `Service::ALL` and `tls_candidates`. Source fixture: `microsoft_names_cover_four_srv_services`; transport-security fixture: `never_includes_plaintext_dns_candidates`. Microsoft clause: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3 . **Partial only**: actual parallel DNS queries, DHCP option 120 discovery, reachability, and TCP group processing are not implemented; TLS-only intentionally omits insecure transports. Version-specific document revision comparison remains pending.

## REGISTER

* **RFC 3261 (June 2002), §§10.2, 10.2.1.1, 10.2.2 and 17.1.2** — Offline construction and observation of REGISTER, CSeq/transaction correlation and retry on 423 Min-Expires. Source: `crates/commscraft-core/src/registration.rs`. Fixtures: `rfc_423_creates_new_transaction_with_new_cseq_and_branch`, `rfc3261_423_rejects_duplicate_min_expires_fail_closed`, `rfc3261_423_rejects_invalid_or_nonincreasing_min_expires`, and `microsoft_register_has_required_identity_and_capability`. URL: https://www.rfc-editor.org/rfc/rfc3261#section-10.2 .
* **MS-SIPREGE 17.1 (2025-08-19), §4.1.1** — Public example confirms `Event: registration`, `+sip.instance`, `epid`, Call-ID and CSeq formatting. This source is **example behavior**, not a complete normative clause-by-clause audit. https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/bc4b0245-339d-490c-ba67-2410e1513ed9 ; revision catalog https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d .
* **MS-SIPAE** — Current code deliberately does not claim completed authentication, peer certificate validation or production registration. Challenge parsing/MIC-only cases are distinct from live sign-in. Fixture `ms_sipae_challenge_cannot_be_auto_authenticated` explicitly checks this restriction.

## Impact, outcomes and gates

Source changes: invalid out-of-range external SRV RNG samples now choose the zero fallback draw without modulo reduction; no plaintext downgrade. Added two discovery fixtures and two REGISTER 423 negative fixtures. These are narrower offline testable assertions, **not a full RFC or Microsoft conformance result**. Keep the immutable historical feature and RFC evidence ledgers unchanged; no increased `normative_requirement_verified` count or interoperability count. Additional tests for complete RFC option combinations, rejected policies, transport/TLS trust, error recovery, DNS wire behavior, MS-CONMGMT DHCP and peer-server operation remain necessary.

Executed tests: **pending GitHub Actions results for exact final commit**. This report lists source-mapped test functions, not fabricated passed executions. No live network exercised. `main` unchanged; v0.30.0 release blocked.
