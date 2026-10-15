# SIP certificate URI SAN: IA5String ASCII boundary

**Scope:** fail-closed local check on a parsed X.509 URI subjectAltName used for SIP domain comparison. Source and offline Rust regression test authored; CI results must be separately verified.

| Evidence | Normative reference | Source | Test |
| --- | --- | --- | --- |
| IA5-01 | [RFC 5280 §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280#section-4.2.1.6): uniformResourceIdentifier in GeneralName is encoded as IA5String. Raw non-ASCII URI characters must not be accepted even when contained in ignored SIP URI parameters or headers | `crates/commscraft-transport/src/sip_certificate.rs` `sip_uri_host` checks `uri.is_ascii()` before host extraction | `non_ascii_uri_san_suffix_cannot_authorize_sip_identity`: raw Unicode in URI parameter/header rejected |
| IA5-02 | [RFC 3261 §25.1](https://www.rfc-editor.org/rfc/rfc3261#section-25.1): SIP URI percent escapes use ASCII hexadecimal representation. This specific test does not establish general URI syntax conformance | Existing percent-escape parser remains active; ASCII percent-encoded example is accepted by this narrow check | `non_ascii_uri_san_suffix_cannot_authorize_sip_identity`: percent-encoded UTF-8 sequence remains accepted at the IA5 boundary |
| IA5-03 | [RFC 5922 §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1): SIP URI certificate identity must be matched to expected SIP domain; validating only the host prefix is insufficient for this local fail-closed profile | `match_sip_sans` delegates to `sip_uri_host`; existing platform TLS trust verification is unchanged | Existing SAN precedence and actual leaf certificate offline tests |
| IA5-04 | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c): server Security Association, message integrity and replay checks remain separate requirements | No changes to SIP transmit, Kerberos, MIC or registration | Existing registration-gate tests still required |

**Verification plan:** `cargo test -p commscraft-transport --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, historical RFC/MS Python evidence checks on standard GitHub-hosted `ubuntu-latest` / `windows-latest` runners only. No success claimed before CI evidence. PRs #26–#29 must be coordinated before merging due to shared code paths.

**Limits:** URI GeneralName ASN.1 parsing is still performed by `x509-parser`. This additional check does not replace full DER/PKIX validation, complete SIP URI grammar, real SfB TLS interoperability, Windows SSPI/SA binding, or full-client feature parity. Historical evidence SHA-256 files untouched. CommSCraft v1.0 release BLOCKED.
