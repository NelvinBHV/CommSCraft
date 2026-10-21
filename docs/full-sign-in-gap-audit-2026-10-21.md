# Full sign-in implementation gap audit — 2026-10-21

**Scope:** active CommSCraft Rust workspace at stacked draft PR #51 (parent #50). Source inspection only, with offline CI evidence noted separately. **No real sign-in or interoperability claim.**

## End-to-end acceptance contract

The first real sign-in must traverse **one** owned, revocable session state machine:

`configured SIP identity → authorized discovery → verified TLS peer → REGISTER challenge → authenticated SSPI/GSS or TLS-DSK context → newly correlated, correctly signed REGISTER → validated server MIC/replay → registration binding → provisioning → session usable`.

Every transition must have a failure/timeout/disconnect edge, a bounded input contract, a source requirement, a test fixture, and a CI run. An offline 200 OK, synthetic MIC, or acquired SSPI handle must never set `signed_in`.

## Audited active components and gaps

| Step | Active code | Evidence level | Missing for actual sign-in |
|---|---|---|---|
| Discovery and server selection | `crates/commscraft-core/src/discovery.rs`; `apps/commscraft-probe` | offline planner + opt-in DNS diagnostics | Authorized production discovery, policy, home-pool redirects and service identity binding |
| SIP transport and TLS identity | `crates/commscraft-transport/src/lib.rs`, `sip_certificate.rs` | inbound-only TLS, platform certificate checks, restricted SIP SAN policy | Audited outbound SIP transport, session ownership, connect/close lifecycle and credential-channel binding |
| SIP parser and transaction | `crates/commscraft-core/src/sip.rs`, `transaction.rs`; `crates/commscraft-router` | offline tests | Wire send/receive integration, stream fragmentation, retries and transaction mapping |
| REGISTER construction and challenge | `registration.rs`, `challenge.rs`, `kerberos.rs` | offline planner/tests | Multi-leg real challenge loop, correct per-leg CSeq/branch and authenticated outgoing MIC |
| Native Windows authentication | `windows_sspi.rs`, `kerberos_mic.rs` | native code and offline/Windows CI checks | Authorized domain/server interoperability, context binding, credential lifecycle and end-to-end verification |
| Server MIC and replay | `security.rs`, `session.rs` | offline verification gate | Verified context key and TLS peer binding in same connection/session |
| Transport/REGISTER bridge | `commscraft-transport/src/offline_register_response_pipeline.rs` (PR #50/#51) | offline-only; #51 CI under review | Real transport and no caller-supplied synthetic approval |
| Registration metadata | `crates/commscraft-register-events/src/gated_presence.rs` | offline negative tests | Connect optional untrusted hints to same verified session without granting authorization |
| Desktop session | `apps/commscraft/src/main.rs` | demo only | Production session coordinator and truthful user-facing state |
| Provisioning, presence and messaging | `commscraft-core` offline feature modules | previews and parsers | Server-negotiated policy, subscriptions, contacts and live operations |

## Ordered implementation plan (security-first)

1. **State/ownership contract:** add a single typed, fail-closed sign-in coordinator interface that owns the connection generation, registration transaction and security context. Unit-test disconnect, reconnect, timeout, duplicate, cross-session and replay. No live sign-in flag.
2. **Outbound transport under explicit authorization:** implement bounded SIP-over-TLS writes and framing only after domain/SAN identity authorization. No TLS bypass, no plaintext downgrade; integration tests with a local deterministic TLS fixture.
3. **Complete REGISTER/challenge exchange:** connect `RegisterFlow`, transaction router and challenge parsing; each 401/407/423 requires a new transaction and deadline. Negative tests for unexpected server and wrong transaction.
4. **Authenticated security negotiation:** bind Windows SSPI/GSS context or an independently derived TLS-DSK server key to the validated TLS service identity and same connection generation. Verify signed outgoing requests and server MIC/replay before any registration decision.
5. **Post-registration provisioning:** consume only validated policy and separately marked untrusted Presence-State/GRUU metadata. Verify refresh, sign-out, failure and recovery.
6. **Authorized interoperability:** test with explicitly authorized Skype for Business Server versions and sanitized captures; compare against the exact Microsoft specification revision. Keep sign-in disabled until the full matrix is independently verified.

## Normative traceability anchors

- [RFC 3261 (June 2002) §§10.2, 17.1.2, 17.1.3](https://www.rfc-editor.org/rfc/rfc3261) — REGISTER, challenge retries, transaction matching.
- [RFC 3263 (June 2002) §§4–7](https://www.rfc-editor.org/rfc/rfc3263) and [RFC 2782 (February 2000)](https://www.rfc-editor.org/rfc/rfc2782) — SIP server selection / DNS SRV.
- [RFC 5922 (June 2010) §§7.1–7.3](https://www.rfc-editor.org/rfc/rfc5922) — SIP TLS domain identity.
- [RFC 2743 (January 2000)](https://www.rfc-editor.org/rfc/rfc2743) and [RFC 4120 (July 2005)](https://www.rfc-editor.org/rfc/rfc4120) — GSS-API and Kerberos primitives.
- [MS-SIPAE §3.2.5.1 and §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9) — Microsoft challenge negotiation and MIC; re-pin document revision before production change.
- [MS-SIPREGE §4.1.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/bc4b0245-339d-490c-ba67-2410e1513ed9) — Microsoft registration extensions; re-pin revision before implementation.

## Release gate

The existing `README.md` and `docs/architecture.md` explicitly state that CommSCraft is **not a working Skype for Business sign-in client**. Keep that limitation in the UI and documentation until authorized server evidence proves otherwise. Offline CI passing is not equivalent to interoperability.
