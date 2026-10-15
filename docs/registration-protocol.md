# Offline REGISTER planner — RFC/MS comparison (v0.4)

This document describes **only** the offline behaviour of `crates/commscraft-core/src/registration.rs`.
No network REGISTER, real sign-in, GSSAPI credential exchange, SA, MIC verification, provisioning, presence or telephone support is implemented. This is intentionally *not* a conformance statement.

## Source pins and rationale

| Behaviour | Specification | Scope in this repository |
|---|---|---|
| REGISTER construction, AoR To, stable Call-ID, increasing CSeq, Contact, Expires | [RFC 3261 §§10.2.1–10.2.4](https://www.rfc-editor.org/rfc/rfc3261.html#section-10.2) | Constructed offline; identifier generation delegated to trusted caller |
| Match final response to non-INVITE transaction, timer F | [RFC 3261 §17.1.2](https://www.rfc-editor.org/rfc/rfc3261.html#section-17.1.2) | Uses existing `NonInviteClientTransaction`; reliable transport subset only |
| Handle `423 Interval Too Brief`, `Min-Expires` | [RFC 3261 §10.3](https://www.rfc-editor.org/rfc/rfc3261.html#section-10.3) | New offline transaction with increased CSeq and different Via branch; bounds 1–86400s are local safety policy |
| SIP REGISTER From epid, `+sip.instance`, Event: registration, `gruu-10` | [MS-SIPREGE §4.1.1 Basic Registration](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/bc4b0245-339d-490c-ba67-2410e1513ed9) and [§3.4 request processing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010) | Partial registration header model, not complete Microsoft SIP header / routing handling |
| Microsoft `gruu=` Contact parameter, `expires=` and instance binding | [MS-SIPREGE §4.1.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/bc4b0245-339d-490c-ba67-2410e1513ed9) | Bounded Contact parser for synthetic server responses; no full RFC URI equivalence |
| Only remove own Contact with `Expires: 0` (never `Contact: *`) | [MS-SIPREGE §4.1.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/1178018e-277b-42a3-a5fb-1a63049e0b50) | Offline unregistration plan; sending prohibited until authenticated integration |
| Distinguish 401 / 407 from 200 OK; no silent credential retry | [MS-SIPAE §3.3.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/aa0ce59d-a5e7-47c9-8a36-f2a2d32bc08c) | Read-only offers and schemes, no auth tokens or signed responses |
| SA establishment, replay window, signed response (`rspauth`) validation | [MS-SIPAE §3.2.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/82a3b226-f1e4-460f-b823-ca42cc251410) and [§3.3.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ec772ab7-96e7-4b54-ac9b-84cd0de2a6c3) | **NOT IMPLEMENTED — a 200 remains UnverifiedAcceptance** |
| Microsoft-specific GRUU predates RFC 5627 pub-gruu/temp-gruu scheme | [MS-SIPREGE §4.1.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/bc4b0245-339d-490c-ba67-2410e1513ed9), [RFC 5627](https://www.rfc-editor.org/info/rfc5627) | Do not conflate Microsoft `gruu=` with RFC 5627 `pub-gruu=` |

## Lifecycle (offline only)

```text
New, configured Endpoint
   |
   v
REGISTER plan / NonInviteClientTransaction [AwaitingResponse]
   |-- 1xx --------------> remain AwaitingResponse
   |-- 401/407 ----------> NeedsAuthentication (no credential response)
   |-- 423 Min-Expires --> NeedsIntervalRetry --new branch+CSeq--> new REGISTER plan
   |-- 200 w/ matching --> UnverifiedAcceptance (not signed in!)
   |                         |-- plan_refresh():   CSeq+1, same Call-ID, branch new
   |                         '-- plan_unregistration(): Expires: 0, own Contact
   |-- other final ------> Rejected
   '-- Timer F ----------> TimedOut
```

**Threat model:** A forged or replayed 200 response could appear to accept a
binding. Transaction matching, a verified TLS connection, or even a legitimate
server-issued GRUU alone cannot replace the cryptographic response integrity
mechanisms in MS-SIPAE. The safe state is *UnverifiedAcceptance*, never
*Registered*. The untrusted GRUU must not be used for privileged operations.

### Remaining before first real login

1. A Microsoft-compatible GSSAPI/NTLM/Kerberos/TLS-DSK adapter with security-association bookkeeping, anti-replay and signed SIP verification according to MS-SIPAE.
2. Service-identity authorization and TLS routing through MS-CONMGMT / RFC 3263, not merely hostname validation.
3. Secure CSPRNG identity / instance persistence, unique branches, local routing Contact selection (IPv4/IPv6), explicit user consent, credential storage.
4. Complete REGISTER exchange with provisioned/redirected home pool, signed 200 OK verification, refresh scheduling and real sign-out.
5. Tests against a licensed and authorized Skype for Business Server 2015/2019/Subscription Edition environment; capture and redact differences from spec examples.
