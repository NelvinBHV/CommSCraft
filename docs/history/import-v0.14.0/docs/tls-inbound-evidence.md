# v0.12.0 — SIP/TLS inbound-only transport: specification-to-code evidence

**Scope:** RFC/MS-derived implementation and authored negative tests, **not** a certification or interoperability report. The source references were cross-checked 2026-10-15. The 1.0 parity gate remains closed.

| ID | Specification (exact section) | Implemented requirement / deliberate subset | Code | Authored test / verification status |
|---|---|---|---|---|
| TLS-RFC-01 | [RFC 3261 §7.5](https://www.rfc-editor.org/rfc/rfc3261.html#section-7.5) | Ignore leading CRLF on stream; accept fragments and coalesced messages | `crates/commscraft-core/src/stream.rs`, `inbound.rs` | `keepalive_crlf_is_skipped`, `coalesced_frames_returned_without_second_read`, `fragment_then_timeout_retains_partial_message`; Rust **NOT RUN** |
| TLS-RFC-02 | [RFC 3261 §18.3](https://www.rfc-editor.org/rfc/rfc3261.html#section-18.3) and [§20.14](https://www.rfc-editor.org/rfc/rfc3261.html#section-20.14) | Require a single exact `Content-Length` (including compact `l`) for TLS streams; reject missing/duplicate/oversized frames | `crates/commscraft-core/src/sip.rs`, `stream.rs`, `inbound.rs` | `missing_length_fails_closed`, `duplicate_length_fails_closed`, `huge_header_is_rejected`; Rust **NOT RUN** |
| TLS-RFC-03 | [RFC 3261 §18.4](https://www.rfc-editor.org/rfc/rfc3261.html#section-18.4) | Transport errors close/poison the local connection; incomplete EOF is an error, timeouts preserve partial frames | `inbound.rs::SipInbound::receive` | `transport_error_poisoned`, `truncated_frame_at_eof_rejected`, `retryable_io_error_preserves_connection`; Rust **NOT RUN** |
| TLS-MS-01 | [MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3) | TLS for `*_tls` SRV targets; attempt sequential IPs for connection failure; **stop** on TLS/other failure rather than fallback | `crates/commscraft-transport/src/lib.rs::TlsSipReceiver::connect`, `apps/commscraft-probe/src/main.rs` | `validates_subdomain_boundaries_and_ports`, `invalid_timeout_does_not_attempt_network`; Windows/TLS live lab **NOT RUN** |
| TLS-MS-02 | [MS-CONMGMT §1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/29002ce1-4bd1-4721-b979-722a255a2312) | Verify TLS peer hostname and trusted CA via OS trust store (no bypass API) | `commscraft-transport::TlsSipReceiver::connect` via `native_tls::TlsConnector::new` | Source-level review only; negative certificate lab test **NOT RUN** |
| TLS-RFC-04 | [RFC 3263 §7](https://www.rfc-editor.org/rfc/rfc3263.html#section-7) | Avoid DNS redirection outside claimed SIP identity subtree; TLS name validation for selected host. **Restricted compatibility policy**, not a claim of full NAPTR/PKIX service identity handling | `commscraft-transport::TlsEndpoint::from_domain_and_host` | `validates_subdomain_boundaries_and_ports`, `rejects_host_injection_and_invalid_domains`; Rust **NOT RUN** |
| TLS-AUTH-01 | [MS-SIPREGE §3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010), [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) | Verified TLS is NOT a validated Microsoft Security Association; no message-writing or sign-in API enabled | `commscraft-transport::TlsSipReceiver` (receive-only), `registration.rs` | Source static assertion + existing REGISTER negative tests; Kerberos/real registrar **NOT RUN** |

## Behavior boundaries

- No outbound SIP requests, REGISTER, credentials, MIC generation, or GRUU trust promotion occur in `TlsSipReceiver`.
- Connection is made only via `connect` with an explicitly provided, DNS-label-valid target inside the claimed SIP domain. This **deliberately** excludes some legitimate deployments using different host namespaces. The full Microsoft autodiscovery/host/service policy remains an open item.
- `native_tls` delegates TLS certificate-chain and DNS hostname checking to the platform. A successful platform handshake is not a test against a real SfB server and is not equivalent to mutual Kerberos authentication. Platform-specific revocation/service-identity policy needs lab verification.
- `commscraft-core::inbound` is transport-neutral. Only the `commscraft-transport` wrapper couples it to a TLS-decrypted stream; it cannot claim transport trust by itself.
- `SipInbound::receive` performs one bounded `read` per call. It reports `Pending` for timeouts and fragmented frames, `Closed` for clean EOF, `Truncated` for partial-frame EOF, and a permanent failure on malformed input/fatal I/O.
- RFC 3261's entire SIP grammar, Microsoft `ms-keep-alive` negotiation, certificate service-name binding, DNSSEC/DHCP, SRV selection completeness, and authenticated REGISTER are **not** implemented by this change.

## Reproducibility and evidence levels

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/check_v012_evidence.py
cargo test -p commscraft-core inbound
cargo test -p commscraft-transport
cargo test --workspace --all-targets
python3 scripts/check_release.py --release  # MUST fail
```

The Python scripts confirm source hashes and a limited set of source structure contracts; they do NOT compile Rust, verify real TLS certificates, or prove interoperability. Rust toolchain and Windows/domain-joined SfB test environment are unavailable here. **Rust tests authored, not executed in this environment**; native TLS/SSPI and real server tests **not run**; Windows Full client parity **not proven**.

Historical evidence: `docs/evidence-v0.11.0.json` remains unchanged. Original v0.11 README and CI bytes are archived under `docs/history/v0.11.0/` for SHA validation rather than falsifying previous hashes.
