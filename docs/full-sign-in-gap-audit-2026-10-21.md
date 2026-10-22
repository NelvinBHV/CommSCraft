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

## Documentation-first clean-room implementation plan (owner testing policy, 2026-10-22)

**Instruction from the project owner:** implement the actual protocols and complete client behavior from public RFCs and Microsoft Open Specifications, **not an offline-only product or an endless sequence of negative fixture tests**. The owner exclusively performs testing against real servers. Automated local/CI checks are development quality gates and do not confer interoperability status. No unsolicited server connections, credentials, or live test traffic.

**Pinned normative document baselines for new protocol work (record specific revision and section per change):** [MS-SIPAE] published revision **11.0, May 20 2025**, [official document](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd); [MS-SIPREGE] published revision **17.1, August 19 2025**, [official document](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d). Validate exact PDF/DOCX revision-specific clauses and applicable errata before claiming support. RFC 3261 (June 2002), RFC 3263 (June 2002), RFC 5922 (June 2010), RFC 2743 (January 2000), RFC 4120 (July 2005) remain normative anchors. A documentation landing page alone is not a substitute for section-level verification.

Implement connected **vertical protocol slices**, not more artificial milestone APIs:

1. **SIP client transaction and message engine:** bounded streaming parser/serializer, connection management, outbound request generation, transaction timers, correlation and compliant retransmission/recovery; map RFC 3261 §§7–8, 10, 17–20 and RFC 3263 §§4–7 to production source. Keep a strict distinction between parser restrictions and generally legal SIP syntax.
2. **REGISTER and authentication exchange:** implement REGISTER 401/407/423 retry chains and challenge selection for MS-SIPAE §§3.2.1, 3.2.5.1–3.2.5.2 and MS-SIPREGE registration exchanges. Correlate each leg with newly generated branch, required CSeq handling, stable identity, and server response. Replace `prepare_fixture` as the functional driver; its synthetic TLS, security and challenge events must never substitute for implemented protocol operations.
3. **Security contexts and signed requests:** implement per-target SA acquisition, challenge token processing, Windows SSPI/Kerberos/NTLM adapters and explicitly specified TLS-DSK mechanics, client signature generation, server MIC/replay verification and cryptographic binding to the actual validated peer. Follow MS-SIPAE, RFC 2743, RFC 4120 and relevant MS-KILE/MS-NLMP specifications; never invent cryptographic semantics or weaken verification.
4. **Registration acceptance and provisioning:** implement the MS-SIPREGE binding, in-band provisioning, policy handling, subscription/refresh/unregister and terminal disconnect/recovery; untrusted Contact/GRUU or metadata must not silently become an authentication grant.
5. **Feature parity after sign-in:** trace and implement required messaging, presence, contacts, meetings and media protocols separately, including error and policy cases. The project is not protocol-complete until every required Windows Full Client capability is mapped and implemented.
6. **Evidence and ownership:** every substantial feature records exact document version, URL, section, production implementation path, deterministic checks and GitHub CI SHA. Only the project owner performs real-server tests and supplies sanitized evidence. Label code `source-implemented`, `CI-checked`, or `owner-verified-interoperability` truthfully; leave real sign-in success and release claims disabled until owner evidence exists.

## Concrete next implementation gap

The existing `crates/commscraft-transport/src/owned_offline_register_attempt.rs` bundles synthetic states but is **not the final protocol client**. Production work must replace caller-originated synthetic completion events with real message-driven transitions, beginning with REGISTER challenge/retry orchestration and the authorized outbound SIP/TLS API. Ensure any prototype can be compiled and tested without contacting servers. That the code is testable locally is not a restriction to an offline-only architecture.

## Normative traceability anchors

- [RFC 3261 (June 2002) §§10.2, 17.1.2, 17.1.3](https://www.rfc-editor.org/rfc/rfc3261) — REGISTER, challenge retries, transaction matching.
- [RFC 3263 (June 2002) §§4–7](https://www.rfc-editor.org/rfc/rfc3263) and [RFC 2782 (February 2000)](https://www.rfc-editor.org/rfc/rfc2782) — SIP server selection / DNS SRV.
- [RFC 5922 (June 2010) §§7.1–7.3](https://www.rfc-editor.org/rfc/rfc5922) — SIP TLS domain identity.
- [RFC 2743 (January 2000)](https://www.rfc-editor.org/rfc/rfc2743) and [RFC 4120 (July 2005)](https://www.rfc-editor.org/rfc/rfc4120) — GSS-API and Kerberos primitives.
- [MS-SIPAE §3.2.5.1 and §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9) — Microsoft challenge negotiation and MIC; re-pin document revision before production change.
- [MS-SIPREGE §4.1.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/bc4b0245-339d-490c-ba67-2410e1513ed9) — Microsoft registration extensions; re-pin revision before implementation.

## Release gate

The existing `README.md` and `docs/architecture.md` explicitly state that CommSCraft is **not a working Skype for Business sign-in client**. Keep that limitation in the UI and documentation until authorized server evidence proves otherwise. Offline CI passing is not equivalent to interoperability.
