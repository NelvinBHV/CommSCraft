# DNS-based SIP identity: exclude numeric IPv4 literals (local policy)

**Status:** source change and offline regression tests; GitHub Actions must pass before any tested claim. No network communication, sign-in, or Windows parity is asserted.

| Requirement | Evidence and scope | Source | Test |
| --- | --- | --- | --- |
| DNS-ID-01 | [RFC 5922 §7.2](https://www.rfc-editor.org/rfc/rfc5922#section-7.2) compares complete DNS-domain certificate identities, not DNS-discovered endpoint addresses. **Local strict policy:** reject numeric IPv4 host literals in this DNS-only matching subset | `crates/commscraft-transport/src/sip_certificate.rs` `ascii_sip_domain` | `numeric_ipv4_sans_cannot_authorize_dns_sip_domains` |
| DNS-ID-02 | [RFC 3261 §19.1.1](https://www.rfc-editor.org/rfc/rfc3261#section-19.1.1) **allows numeric IP hosts** in general SIP URIs. Rejecting them here is deliberate subset policy, **not** an RFC 3261 prohibition | `sip_uri_host` uses `ascii_sip_domain` | `numeric_ipv4_sans_cannot_authorize_dns_sip_domains` |
| DNS-ID-03 | [MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3) discovers server addresses from the *original SIP domain* through DNS SRV. Numeric IPv4 literals cannot enter this restricted DNS-domain/TLS identity constructor, but normal DNS names containing numeric labels remain valid | `crates/commscraft-transport/src/lib.rs` `dns_name`, `TlsEndpoint::from_domain_and_host` | `numeric_ipv4_hosts_are_not_dns_sip_tls_identities` |
| DNS-ID-04 | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) requires separate Security Association, server MIC, and replay checking. DNS/TLS name validation does not satisfy authentication | No change to SSPI, REGISTER, MIC, or authenticated state | Existing registration-gate tests remain required |

**Tests to execute:** `cargo test -p commscraft-transport --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, and established RFC/Microsoft evidence scripts on **standard** `ubuntu-latest` and `windows-latest` GitHub-hosted runners. This document does not claim these checks have passed.

**Boundaries:** Deliberately no IPv4 literal support at this identity interface; other RFC 3261 URI handling is unaffected. IPv6 literals were already excluded. The TLS platform certificate checks remain required and never bypassed. Historical SHA-256 ledgers remain untouched. CommSCraft v1.0 remains blocked pending authorized, complete Windows Full Client interoperability.
