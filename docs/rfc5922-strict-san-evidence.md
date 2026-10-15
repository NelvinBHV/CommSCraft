# CommSCraft RFC 5922: strict SAN matching after platform TLS handshake

**Status:** Implementation and offline tests authored; CI evidence required, offline tests require passing GitHub Actions. Not a claim of live Skype for Business Server TLS certificate / Kerberos authentication. **Scope:** additional peer-certificate SAN verification with no trust-bypass, no SIP send API, no session elevation.

## Traceability to RFC and Microsoft Open Specifications

| Evidence | Normative reference | Specific source behavior | Test |
| --- | --- | --- | --- |
| SIPCERT-01 | [RFC 5922 §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1) | DER peer X.509 certificate parsed with `x509-parser`; extract `subjectAltName` `uniformResourceIdentifier` with **sip:** (case-insensitive), without userpart, and DNS names; ignore unrelated SAN types | `actual_leaf_der_valid_sip_uri_san_and_no_canonical_hostname_confusion`, `uri_scheme_no_userinfo_parameters_and_exact_dns_comparison` |
| SIPCERT-02 | [RFC 5922 §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1) | Any valid SIP URI SAN takes precedence over dNSName SAN even when **only the DNSName matches**; never fall back to CN when SAN is present | `real_der_sip_uri_takes_precedence_over_matching_dns_san_and_cn`, `dns_fallback_only_without_acceptable_sip_uri_and_no_wildcards` |
| SIPCERT-03 | [RFC 5922 §7.2](https://www.rfc-editor.org/rfc/rfc5922#section-7.2) and [RFC 4343](https://www.rfc-editor.org/rfc/rfc4343) | DNS labels compared case-insensitively and in their entirety, no wildcard and no suffix matching. Reject malformed DNS / SIP URI names | `actual_wildcard_der_cannot_impersonate_sip_domain`, `reject_malformed_sip_uri_and_excessive_san_cardinality` |
| SIPCERT-04 | [RFC 5922 §7.3](https://www.rfc-editor.org/rfc/rfc5922#section-7.3) and [RFC 3263 §7](https://www.rfc-editor.org/rfc/rfc3263#section-7) | After verified platform TLS handshake against original SIP URI domain, parse **that connection's actual leaf certificate** via `tls.peer_certificate()?.to_der()`; reject on malformed/oversized DER, absent SAN or identity mismatch and close stream | `malformed_and_trailing_der_fail_closed`; `test_sip_san_rules_bound_to_actual_peer_der` |
| SIPCERT-05 | [RFC 5280 §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280#section-4.2.1.6) and RFC 5922 §7.1 | Reject duplicate/invalid/absent SAN; bounds: 64 KiB DER, 128 SAN names; no manual CA/validity override | `malformed_and_trailing_der_fail_closed`; source checks |
| SIPCERT-06 | [MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3) | SRV result controls TCP DNS endpoint, **never** authenticated SIP-domain identity. No plaintext fallback | Existing `tls_certificate_identity_is_original_sip_domain_not_srv_hostname` and TLS contract tests |
| SIPCERT-07 | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) and [MS-SIPREGE §3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010) | TLS certificate name equality **does not** establish an authenticated SIP transaction, matching SSPI/Kerberos SPN or a validated MIC/replay sequence. Receiver remains read-only | Existing `RegisterVerificationGate` / `gated_presence` suites; `test_inbound_only_and_no_trust_bypass` |

## Public test certificates, not trust anchors

`tests/fixtures/*.pem` contains three **public, self-signed Ed25519 leaf certificates** generated only for parsing tests (no private key distributed):
- `sip-uri-san.pem`: certificate SAN `URI:sip:example.com`, `DNS:example.com`, unrelated CN;
- `conflicting-sip-uri.pem`: `URI:sip:evil.example`, `DNS:example.com`, CN=example.com. **Must reject** for `example.com`;
- `wildcard-dns.pem`: `DNS:*.example.com`, CN=example.com. **Must reject** `sip.example.com`.

Their self-signatures **must never** be trusted in a live TLS context. Tests deliberately parse DER **without** performing chain validation; live `TlsSipReceiver::connect` still requires the native TLS/PKIX handshake *before* this code is called. No certificate fixture private keys are committed. An OS trust-store bypass is not permitted.

## Deliberate compatibility limitations and remaining work

- RFC 5922 §7.1 **permits** CN-only fallback only when SAN is absent. CommSCraft deliberately **disallows CN-only certificates** as a local fail-closed security policy.
- Strict matching currently handles **ASCII DNS hostnames and punycode A-labels** but not Unicode U-label/IDN normalization required by RFC 5280 §7.2. DNS names with IPv6 literals are not supported. This is a documented subset, **not full RFC 5922 conformance**.
- Platform TLS may reject URI-only SAN leaf certs because its initial platform hostname verification expects DNS/CN. CommSCraft never bypasses that verification to force acceptance.
- Platform certificate validation (CA/trust chain, EKU, dates and revocation policies), Windows certificate store configuration, and actual SfB pool certificate identity require live authorized lab tests.
- The transport intentionally **cannot** send SIP REGISTER or credentials, and no resulting connection is bound to a trusted Windows SSPI SPN/MS-SIPAE server Security Association. Live production sign-in and Full Windows Client parity remain unverified.
- Historical RFC/MS SHA-256 manifests and immutable imported source are unchanged.
- CI must use only standard GitHub-hosted `ubuntu-latest` and `windows-latest` runners.

**Execution:** `cargo test -p commscraft-transport --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `python3 -m unittest discover -s scripts -p 'test_*.py'`, `python3 scripts/check_import_integrity.py`, `python3 scripts/check_v012_evidence.py`. Do not report these tests as passed until the particular head commit's CI is complete.

**Release policy:** CommSCraft 1.0 remains BLOCKED.


## SIP URI SAN grammar hardening (follow-up)

[RFC 3261 §19.1.1](https://www.rfc-editor.org/rfc/rfc3261#section-19.1.1) defines the SIP URI structure used by [RFC 5922 §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1) for certificate identity extraction. CommSCraft now rejects URI SAN values containing fragments (`#`), whitespace, ASCII control bytes and backslashes **anywhere**, including ignored URI-parameter/header suffixes, before extracting the matching host. This is a deliberate **fail-closed parser restriction**, not a claim of a complete SIP URI grammar parser. Legitimate semicolon URI parameters and question-mark SIP headers remain accepted. Regression test: `malformed_sip_uri_suffix_cannot_authorize_a_domain`, with both positive and negative cases.

This is only an extra check on an already platform-verified peer leaf certificate. It is not independent certificate chain validation and does not change MS-SIPAE MIC/replay or Windows SSPI login boundaries. Execute `cargo test -p commscraft-transport --all-targets` and the strict full-workspace checks on standard GitHub-hosted Linux/Windows runners; CI evidence is only valid after completion.

## RFC 9549 / IDNA A-label validity — October 2026

| Evidence ID | Source | Code path | Positive/negative evidence |
| --- | --- | --- | --- |
| IDNA-01 | [RFC 5890 §2.3.2.1](https://www.rfc-editor.org/rfc/rfc5890#section-2.3.2.1): only valid reversible `xn--` encodings are A-labels | `ascii_sip_domain` calls `idna::domain_to_ascii_strict` and requires ASCII round-trip equality (ignoring case) | `valid_idna_a_labels_match_exact_domain_but_fake_punycode_cannot` |
| IDNA-02 | [RFC 9549 §2.3](https://www.rfc-editor.org/rfc/rfc9549#section-2.3), updating RFC 5280 §7.2: SAN dNSName values carry A-labels and comparisons are full-string case-insensitive | `match_sip_sans` validates encoded SAN domains before DNS or SIP URI identity comparison | `valid_idna_a_labels_match_exact_domain_but_fake_punycode_cannot` |
| IDNA-03 | [RFC 5922 §7.3](https://www.rfc-editor.org/rfc/rfc5922#section-7.3), [MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3): keep trusted original SIP domain and discovered SRV target separate | `TlsEndpoint::dns_name` rejects invalid Punycode labels in both the SIP-domain SNI identity and DNS SRV destination before network I/O | `idna_a_label_validation_is_required_for_tls_sni_and_srv_target` |
| IDNA-04 | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c): cryptographic server MIC and replay validation remain mandatory | IDNA verification never creates a trusted authentication principal or accepted registration state | Existing `RegisterVerificationGate` and `gated_presence` suites unchanged |

**Deliberate fail-closed scope:** `idna` 1.1.0 provides strict UTS #46 / IDNA normalization, but this test does *not* establish complete IDNA2008 / RFC 9549 interoperability for all Unicode scripts. The public transport and certificate parser still **reject Unicode U-label input**, accept validated ASCII A-labels only, and require an unchanged ASCII result (case-insensitive). This prevents silent compatibility-name rewriting at a security boundary. URI-only SAN platform TLS limitations and real Windows Server SE endpoint / Kerberos SPN binding remain open. Historical source ledgers are unmodified.

CI commands: `cargo test -p commscraft-transport --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, and the legacy Python RFC/MS evidence checkers. Linux and Windows only on standard GitHub-hosted runners. This evidence row is not a passed-test claim until GitHub Actions completes.

## RFC 9549 §1 — Unicode Other Symbol (So) exclusion after Punycode decoding

**Security finding:** RFC 9549 §1 explicitly states that Unicode General Category **Other Symbol** (`So`) characters are **DISALLOWED** by IDNA2008, while UTS #46 compatibility handling can still produce A-labels for them. The RFC's example is the black chess king `♚`, encoded as `xn--45h`. The previous CommSCraft `idna::domain_to_ascii_strict` round-trip did not separately enforce this IDNA2008 symbol-category exclusion.

| ID | Normative reference | Enforced source rule | Executable positive/negative tests |
| --- | --- | --- | --- |
| IDNA-05 | [RFC 9549 §1](https://www.rfc-editor.org/rfc/rfc9549#section-1), [RFC 5890 §2.3](https://www.rfc-editor.org/rfc/rfc5890#section-2.3): reject Unicode `So` even if UTS #46 recognizes a Punycode label | Shared `idna_policy::rfc9549_a_label_symbols_allowed` decodes A-label for inspection and rejects Unicode `GeneralCategory::OtherSymbol`; decoded text is **never** used for SNI or identity comparison | `rfc9549_explicit_other_symbol_punycode_is_denied`; `ordinary_and_valid_idn_a_labels_still_pass` |
| IDNA-06 | [RFC 9549 §2.3](https://www.rfc-editor.org/rfc/rfc9549#section-2.3), [RFC 5922 §7.1–7.3](https://www.rfc-editor.org/rfc/rfc5922#section-7.1): strict full A-label certificate identity | `ascii_sip_domain` calls the shared policy **after** its existing strict ASCII/IDNA round-trip; rejects `xn--45h.example.com` in both SIP URI SAN and DNS SAN | `rfc9549_other_symbol_a_label_never_authorizes_sip_certificate_san` |
| IDNA-07 | [RFC 9549 §1](https://www.rfc-editor.org/rfc/rfc9549#section-1), [MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3): reject invalid server DNS identities before connection | `TlsEndpoint::dns_name` applies the same policy to the original SIP domain and discovered SRV target, **before network I/O** | `rfc9549_rejects_other_symbol_a_labels_before_tls_dns_or_sni` |
| IDNA-08 | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c): server MIC, SPN and replay protections remain separate | The helper returns only a boolean for syntactic/domain policy; it creates no authenticated context and grants no registration state | Existing `RegisterVerificationGate` and `gated_presence` suites; `test_rfc9549_other_symbol_a_labels_checked_both_before_tls_and_in_sans` |

**Limitations:** `unicode-general-category` 1.1.0 uses Unicode 16 character-category data. Rejecting `So` is one explicit RFC 9549 / IDNA2008 requirement, **not** complete IDNA2008 Derived Property / CONTEXTJ/CONTEXTO, bidirectional-script or current Unicode version conformance. This change **retains rejection of untrusted Unicode U-label input** and keeps native TLS/PKIX validation, RFC 5922 SAN comparison and MS-SIPAE server MIC/replay as independent prerequisites. No real Windows Skype for Business Server sign-in was performed. The v1.0 release gate remains blocked.

**Verification:** `cargo test -p commscraft-transport --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `python3 -m unittest discover -s scripts -p 'test_*.py'`, immutable historical source SHA-256 / RFC-MS checks on standard GitHub-hosted `ubuntu-latest` and `windows-latest` only. Tests are **not** claimed as executed until the CI for the exact commit is successful.

## Percent-encoded SIP URI SAN components — RFC 3261 follow-up

**Requirement:** [RFC 3261 §25.1](https://www.rfc-editor.org/rfc/rfc3261#section-25.1) defines escaped URI content as `%` followed by two hexadecimal digits, within SIP-URI syntax from [§19.1.1](https://www.rfc-editor.org/rfc/rfc3261#section-19.1.1). [RFC 5922 §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1) uses SIP URI SAN identities for server-domain matching.

**Implementation:** `sip_uri_host` now rejects malformed or truncated percent escapes **anywhere** in the SIP URI (including parameters and headers that are not used for name matching), before granting a SAN identity. Valid hex escapes in parameters and headers continue to be accepted. This is a local fail-closed RFC-3261 grammar check, not a claim of complete SIP URI ABNF parsing.

**Executable evidence:** Rust test `malformed_percent_escapes_in_sip_uri_san_never_authorize_identity` checks seven malformed URI values and three valid percent-escaped parameter/header values. CI requirements remain `cargo test -p commscraft-transport --all-targets`, full workspace Rust/Clippy and historical SHA-256 RFC/MS checks on standard GitHub-hosted Linux/Windows runners only.

**Microsoft boundary:** [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) still requires Security Association/MIC/replay verification; certificate URI validity does **not** authenticate REGISTER or establish a signed-in CommSCraft session. No history ledger or release policy changed.

## Duplicate X.509 extensions — RFC 5280 §4.2 (follow-up)

**Requirement:** [RFC 5280 §4.2](https://www.rfc-editor.org/rfc/rfc5280#section-4.2) disallows multiple certificate extensions with the same OID. A SIP SAN parser must never silently accept a first `subjectAltName` and ignore a conflicting duplicate. RFC 5922 §§7.1–7.2 then governs comparison of the unambiguous SIP URI/DNSName identity.

**Implementation:** `crates/commscraft-transport/src/sip_certificate.rs::matches_sip_identity` invokes `cert.extensions_map()` before reading SAN content and fails closed on a `DuplicateExtensions` error. This rejects *any* duplicate X.509 extension OID, not just duplicate SANs. The platform certificate/chain verification and original SIP domain checks remain mandatory.

**Executable evidence:** `duplicate_subject_alternative_name_extensions_fail_closed` parses a public-only DER fixture `tests/fixtures/duplicate-san.pem` that carries two `subjectAltName` extensions. The self-signed certificate was modified after signing, so its signature is deliberately invalid; it is suitable **only for parser rejection testing** and must never be added to a trust store. The test explicitly confirms that `extensions_map()` reports an error and the SIP identity is rejected.

**Microsoft boundary:** [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) still requires a verified Security Association, server MIC and replay checks independently. No certificate check may promote SIP REGISTER state. CI: full Linux/Windows workspace Rust tests, strict Clippy and preserved historical evidence ledgers, **standard GitHub-hosted runners only**. CommSCraft v1.0 release gate remains blocked.

## RFC 3261 SIP-URI header field syntax (additional certificate gate)

**Normative source:** [RFC 3261 §25.1](https://www.rfc-editor.org/rfc/rfc3261#section-25.1) specifies `headers = "?" header *("&" header)`, `header = hname "=" hvalue`, `hname = 1*(...)`, and `hvalue = *(...)`. This means an empty header name or an absent equals sign is invalid, while an empty header value is valid.

**Implementation:** `crates/commscraft-transport/src/sip_certificate.rs` rejects missing `=`, empty header name, empty header list, and dangling/duplicate ampersand components before using a `sip:` URI SAN to authenticate a domain. It does not use URI header values to select a certificate identity and intentionally remains a narrow fail-closed subset of the full RFC 3261 URI grammar.

**Executable proof:** `malformed_sip_uri_headers_cannot_authorize_certificate_identity` checks seven malformed URI SANs and three valid header/parameter forms, and verifies that invalid URI SANs cannot authorize `example.com` in `match_sip_sans`.

**Microsoft boundary:** [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) still independently requires a verified server Security Association, MIC, and replay checks. Header syntax checking never elevates TLS to an authenticated REGISTER session. No historic RFC/MS evidence ledger changed. Use only standard GitHub-hosted Linux/Windows runners, and count tests as proven only once the exact commit's CI checks are green.
