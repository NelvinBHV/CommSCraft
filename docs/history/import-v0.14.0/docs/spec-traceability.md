# Specification traceability and review checklist

This document tracks *exact* RFC and Microsoft Open Specification sections to
implemented code. Anything not matched here is **not asserted as compliant**.
These are public Microsoft protocol descriptions, not copied proprietary
Microsoft client code.

| Requirement | Spec / section | Code | Test / review |
|---|---|---|---|
| SIP message framing, Content-Length on streams | RFC 3261 §§7, 18.3 | `sfb-core/src/sip.rs`, `stream.rs` | frame fragmentation, length injection/size checks |
| Non-INVITE reliable client state machine | RFC 3261 §17.1.2 | `transaction.rs` | 100, 401, 200, Timer F and key matching |
| MS SIP discovery SRV query names in parallel | MS-CONMGMT §3.1.5 | `discovery.rs`, `sfb-probe/main.rs` | exact 4 service names, 4 parallel probe workers |
| MS TLS precedence and fallback names:443 | MS-CONMGMT §3.1.5 | `discovery.rs` | TLS-only ordering + fallback unit tests |
| DNS SRV priority then weight selection | RFC 2782, §Priority/Weight | `discovery.rs::order_srv` | 0-weight, unequal priorities, deterministic draws |
| No plaintext or unauthenticated SIP on probe | Security decision exceeding MS-CONMGMT minimum | `sfb-probe` | source scan; runtime audit still required |
| Certificate chain+DNS hostname validation | RFC 3263 §7 / platform CA behavior | `sfb-probe` (native TLS defaults) | needs live test and certificate matrices |
| 401 vs 407 auth challenge headers | MS-SIPAE §§2.2.1, 3.2.5.1 | `challenge.rs` | 2 schemes, quoted comma, 407, redacted Debug |
| MS-SIPAE server security-association identity and states | MS-SIPAE §3.2.1 | `security.rs::SecurityAssociation`, `AssociationTable` (offline only) | state/identity and expiration unit tests |
| Server Authentication-Info, ordered MIC input, signature-before-replay | MS-SIPAE §§2.2.2, 3.2.5.2 | `security.rs` (partial SIP grammar) | synthetic HMAC, replay and tampering tests |
| TLS-DSK HMAC-SHA256 given a trusted server key | MS-SIPAE §3.2.5.2; RFC 2104 | `TlsDskHmacSha256` | exact synthetic fixture, no handshake |
| Outgoing SA MIC input, cnum/crand, Authorization vs Proxy-Authorization | [MS-SIPAE §3.2.4.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/8c2ae856-5438-42b5-8d08-5746b88e707f) / [§3.3.5.3](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/47b1baf7-ceae-40bc-b1bf-8c26a1ec885a), RFC 2104 | `security.rs::client_mic_buffer`, `SecurityAssociation::sign_client_request`, `TlsDskClientHmacSha256` | synthetic exact 15-field vector, separate Python digest, no pre-SA signing, no sign on error; Rust execution unverified |
| Kerberos 401/407 v3 GSS-token challenge handoff (**offline only**) | [MS-SIPAE §3.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9); §4.2; RFC 2743 | `kerberos.rs`, `registration.rs::plan_authentication_retry` | synthetic provider/negative tests, trusted SPN, no actual handshake |
| NTLM/Kerberos/TLS-DSK *native handshake* and actual authenticated network traffic | MS-SIPAE §§3.2.4.1, 3.2.5.1; MS-NLMP, MS-KILE | **NOT IMPLEMENTED** | requires audited SSPI/GSS, TLS-DSK PRF, certificates |
| Offline REGISTER planner, epid, +sip.instance, Event, 423, GRUU, unregistration | RFC 3261 §§10.2–10.3; MS-SIPREGE §§3.4, 4.1.1–4.1.2 | `registration.rs` / `sfb-inspect --register-demo` | 11 offline tests; integration unverified |
| Offline transaction-bound MIC-checked REGISTER 200 (NOT real login) | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c), [MS-SIPREGE §4.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/bc4b0245-339d-490c-ba67-2410e1513ed9), RFC 3261 §§10.2.4/17.1.2 | `registration.rs::observe_mic_checked_200` | `crates/sfb-core/tests/verified_registration.rs` (9 authored tests; execution pending); Contact/GRUU not MIC-protected |
| Real authenticated REGISTER/SA, signed 200, provision, refresh | MS-SIPAE §§3.2.1, 3.3.5; MS-SIPREGE | **NOT IMPLEMENTED** | requires audited GSS adapter, trusted TLS peer and on-prem lab |

## Non-compliance and deliberate restrictions

- MS-CONMGMT also describes `_sipinternal._tcp`, `_sip._tcp`, DHCP Option 120,
  optional host fallbacks and sequential connection fallback. We query TCP SRV
  for discovery visibility but never connect in plaintext. DHCP and proxy
  behavior are omitted. This is an intentional subset, not complete fidelity.
- RFC 3263 specifies certificate/name bindings for service discovery. A
  successful handshake to a hostname does **not** verify the whole SIP service
  identity policy for production login. `sfb-probe` confines targets to the
  identity domain subtree for lab diagnostics only.
- MS-SIPAE requires managing a stateful security association, choosing or
  verifying a mechanism, signing messages and checking server authentication.
  Parsing a challenge is not implementation of these requirements.
- RFC 2782 has subtleties for zero-weight records and uniform random choice
  over an **inclusive** range. Keep fixed-seed ordering regression tests.
- A release must not be called conformant until feature coverage, full
  threat analysis, RFC/MS-version pinning and interoperability tests are
  recorded. This repository is currently a prototype.

## Canonical links

- [MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3)
- [MS-SIPAE §2.2.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/e60fa322-b73b-4e83-9ddb-4b20bd17a4a1)
- [MS-SIPAE §3.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9)
- [MS-OCSPROT §2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/45e475a0-b534-469a-892e-6bd6ae42fc6e)
- [RFC 3261](https://www.rfc-editor.org/rfc/rfc3261)
- [RFC 3263](https://www.rfc-editor.org/rfc/rfc3263)
- [RFC 2782](https://www.rfc-editor.org/rfc/rfc2782)

See [MS-SIPAE security association mapping](sipae-security-association.md) for the exact normative links and unsupported protocol branches.

## 0.6.0 offline-only extensions

| Requirement / reference | Module | Implemented subset | Explicit missing work |
|---|---|---|---|
| [RFC 6665 §4.1.2](https://www.rfc-editor.org/rfc/rfc6665) (SIP event packages) | `presence.rs` | Pending/active/terminated NOTIFY state, dialog/CSeq checks; initial NOTIFY may precede SUBSCRIBE final response | Established SIP transport and authenticated subscription lifecycle |
| [RFC 3856](https://www.rfc-editor.org/rfc/rfc3856), [MS-PRES §2.2.2.8.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/89e5df81-2176-49da-9456-438e70d40464) | `presence.rs` | PIDF Accept, Event: presence, Contact and body MIME checks | Enhanced categories, XML schemas, containers, publication and BENOTIFY |
| [RFC 3261 §12](https://www.rfc-editor.org/rfc/rfc3261), [MS-OCSPROT §2.5.9](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/f27a1335-dfb8-43b3-80d3-5354757c7c0c) | `im.rs` | Offline accepted INVITE response, ACK prerequisite, MESSAGE planning and correlation, text/plain inbound checks | INVITE/ACK transaction, route set, signed messages, real delivery, multimodal IM |
| [RFC 8866](https://www.rfc-editor.org/rfc/rfc8866), [MS-SDPEXT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/cd17a549-b948-42a6-aa6b-fa707710faac) | `sdp.rs` | Bounded v/o/s/t/m field structure inspection | RFC 3264 offer/answer, ICE, DTLS/SRTP, codecs/media transport |
| [Microsoft Skype for Business Server 2019/SE desktop comparison](https://learn.microsoft.com/en-us/skypeforbusiness/plan/feature-comparison) | `FEATURE_PARITY.json`, `scripts/check_release.py` | Explicit **122-feature / 93 Windows-reference-case** acceptance inventory and fail-closed v1.0 release policy | Actual complete implementation and authorized Windows/server lab signoff |

These subsets are not a transport-safe implementation, no SIP message is
signed for real delivery, and UI demo states provide no server evidence.

## v0.7.0 change log for traceability

The new outgoing signer is ONLY an offline, separately keyed cryptographic
primitive. Test success does not establish user identity, service identity,
SSPI/GSS context, TLS-DSK handshake, a live REGISTER response or Windows parity.
The MS-SIPAE 11.0 client signature input includes SIP URI and selected header
values with case preserved, not a generic RFC 3261 SIP Digest calculation.
See [client MIC notes](client-mic-signing.md) and the synthetic test vector.

## v0.9.0 Windows Kerberos SSPI (experimental)

- Windows `AcquireCredentialsHandleW`/`InitializeSecurityContextW`: `crates/sfb-core/src/windows_sspi.rs`, `#[cfg(windows)]`; see [SSPI mapping](windows-sspi-kerberos.md).
- [MS-SIPAE] §3.2.5.1 initial GSS context may need further processing: `KerberosPhase::AwaitingContextContinuation`; no login status promoted.
- Neither the native Windows provider nor server GSS MIC integration has been built or tested against a server in this authoring environment; they are **not v1.0 evidence**.

## v0.10.0 Windows SSPI MIC bridge

A detailed mapping of exact requirements, Rust functions, offline checks,
Windows API documentation, and **missing interoperability evidence** is in
[Kerberos MIC evidence ledger](kerberos-mic-evidence.md). The existence of an
FFI entry point does **not** prove native Kerberos MIC correctness. This is
not a working server login and does not satisfy full-client parity.
