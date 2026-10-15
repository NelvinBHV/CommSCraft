# SIP URI SAN parameter-separator validation — RFC/MS evidence

**Scope:** Offline, fail-closed validation of a SIP URI stored in an X.509 subjectAltName, before extracting the SIP domain. Not an authenticated sign-in or complete SIP URI parser. The peer certificate is still obtained only after the platform TLS verifier accepts the live TLS connection. Historical evidence manifests remain unchanged.

| ID | Normative evidence | Implementation | Offline regression |
| --- | --- | --- | --- |
| SANPARAM-01 | [RFC 3261 §25.1](https://www.rfc-editor.org/rfc/rfc3261#section-25.1), `uri-parameters`, `uri-parameter`: each semicolon introduces a parameter whose name must not be empty; `uri-parameter-value` is optional | `crates/commscraft-transport/src/sip_certificate.rs` `sip_uri_host`: reject empty parameter names, double semicolon, trailing semicolon, multiple unescaped equals in one parameter | `malformed_uri_parameter_separators_cannot_authorize_sip_san` negative cases |
| SANPARAM-02 | [RFC 3261 §19.1.1](https://www.rfc-editor.org/rfc/rfc3261#section-19.1.1) and [RFC 5922 §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1): certificate URI SAN may provide SIP-domain identity; validation cannot rely on a valid host prefix alone | Existing `match_sip_sans` delegates to validated `sip_uri_host` | same test; positive `lr`, `transport=tls`, multiple valid parameters |
| SANPARAM-03 | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) server Security Association, MIC and replay validation are independent of certificate-name syntax | No changes to SIP message sending, SSPI, authentication or registration state | Existing security-gate checks continue to be required |

**Proof status:** source change and test functions authored; no claim that CI passed. Verify `cargo test -p commscraft-transport --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, Python evidence checks, and Linux/Windows CI using only standard `ubuntu-latest`/`windows-latest` GitHub-hosted runners. Do not merge until all required jobs pass.

**Limits:** This is a narrow validation of semicolon separators, not complete RFC 3261 parameter-name/parameter-value grammar, nor complete RFC 5922/IDNA coverage. Existing PRs #26–#28 touch some of the same files: reconcile before merge. No live Windows/Kerberos interoperability evidence. CommSCraft 1.0 remains BLOCKED.
