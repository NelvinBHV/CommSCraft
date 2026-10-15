# CommSCraft: original SIP-domain TLS reference identity — test evidence

**Status:** Source implementation and offline tests; live certificate interop NOT verified. These changes do not enable SIP transmission, credential exchange or sign-in.

## Normative mapping

| ID | Requirement and authority | Implementation | Executable check |
| --- | --- | --- | --- |
| TLSDOM-01 | [RFC 5922 §7.3](https://www.rfc-editor.org/rfc/rfc5922#section-7.3): authenticate certificate identity against the **original SIP AUS domain**, not an SRV hostname | `TlsEndpoint::tls_reference_identity` uses canonical SIP domain; `TlsSipReceiver::connect` calls `tls.connect(peer.tls_reference_identity(), tcp)` | `tls_certificate_identity_is_original_sip_domain_not_srv_hostname` |
| TLSDOM-02 | [RFC 5922 §7.2](https://www.rfc-editor.org/rfc/rfc5922#section-7.2): compare full SIP DNS identity, not suffixes or wildcards | DNS labels are normalized and target host is restricted to the expected subtree, without changing the certificate reference | `validates_subdomain_boundaries_and_ports`, `dns_root_dot_and_forged_suffixes_fail_closed` |
| TLSDOM-03 | [MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3): SRV lookup provides TLS destination; no fallback after certificate failure in current diagnostic | DNS/port connection uses SRV target; TLS certificate reference is SIP origin; missing peer cert fails closed | `tls_certificate_identity_is_original_sip_domain_not_srv_hostname`, existing timeout/endpoint tests |
| TLSDOM-04 | [native-tls connector API](https://docs.rs/native-tls/0.2.18/native_tls/struct.TlsConnector.html): `connect(domain, stream)` uses domain for SNI and platform hostname checking | Default system roots + hostname checks (no bypass builder); `tls.peer_certificate()` must return Some | `scripts/test_tls_identity_contract.py` (source contract; does **not** verify a real certificate) |
| TLSDOM-05 | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) and [MS-SIPREGE §3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010): SIP security association and REGISTER verification remain additional prerequisites | Transport stays read-only; no registration state exposed | `test_external_network_remains_receive_only`, existing MIC/replay gate tests |

## Security boundaries and unresolved RFC requirements

- RFC 5922 **does not allow wildcard identity matching** and defines scheme-aware SAN precedence. The platform TLS verifier can implement **HTTPS-style wildcard or CN fallback**; this change **does not implement** full RFC 5922 §§7.1–7.2 X.509 SAN parsing. Consequently a successful diagnostic connection must not be asserted as a fully RFC-5922-authenticated SIP domain.
- The SIP domain must originate from the application's trusted SIP identity, not be taken from an untrusted `Authentication-Info`, `Contact`, redirect or SRV target. This is the responsibility of the caller that constructs `TlsEndpoint`.
- A legitimate deployment may present a certificate for its pool FQDN but not its SIP-domain origin; this strict diagnostic will fail closed. A conformant service-name/pool certificate policy and real Windows integration testing remain mandatory before expanding the transport.
- `peer_certificate()` checks certificate presence but **does not** independently verify certificate identity; that is the responsibility of the platform TLS handshake.
- The previous diagnostic printed that a destination hostname was verified. It now distinguishes the original SIP domain from the SRV host and refuses to imply a signed-in state.
- Existing historical RFC/MS hash ledgers and archived original source files are unchanged. The active v0.12 source checker now verifies the **new** transport reference identity while historical SHA-256 checks still use original archived bytes.
- Only standard GitHub-hosted `ubuntu-latest` and `windows-latest` runners are permitted.

## Executable evidence and release constraints

`cargo test -p commscraft-transport --all-targets`; `cargo test --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `python3 scripts/check_v012_evidence.py`; `python3 -m unittest discover -s scripts -p 'test*.py'`.

Do **not** count any new test as passed until GitHub Actions finishes. No real PKIX certificate test, MS-SIPAE Kerberos-to-TLS binding, Skype for Business Server SE interaction or Full Windows Client parity is established. The CommSCraft 1.0 release gate stays fail-closed.
