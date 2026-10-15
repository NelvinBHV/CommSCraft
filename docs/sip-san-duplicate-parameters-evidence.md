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
