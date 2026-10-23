# MS-SIPREGE registration lifecycle: Basic Unregistration evidence and security dependency

**Research tranche 2026-10-22.** This is a directly sourced, narrow examination of **MS-SIPREGE §4.1.2 (Basic Unregistration)** and the published-version ledger; it is NOT a completed review of MS-SIPREGE or the Windows Full Client.

## Exact published source and version discipline

- [MS-SIPREGE published specification overview](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d): published revision **17.1, 2025-08-19 (minor)**. Previous rev 17.0 (2025-05-20), rev 16.0 (2025-02-18) and rev 15.0 (2021-08-17). The revision history also includes Communicator-era editions going back to 2009–2010. Full PDF edition-to-section comparison is NOT yet performed.
- [MS-SIPREGE §4.1.2 Basic Unregistration](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/1178018e-277b-42a3-a5fb-1a63049e0b50): original Microsoft example, HTML **last updated 2019-02-14**. This is an **informative example**, not alone a comprehensive normative statement applying unchanged to rev 17.1. It describes an old Office Communicator / RTC version; no assumption it exactly matches all Skype for Business Windows Full Client releases.
- [MS-SIPREGE glossary](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/e1215cef-8cb2-4594-8b35-660566271a83): EPID, GRUU, in-band provisioning, SIP registrar, security association and other terms.
- [RFC 3261 §10.2.2](https://www.rfc-editor.org/rfc/rfc3261), client REGISTER bindings removal; [§10.2.3](https://www.rfc-editor.org/rfc/rfc3261) binding query, [§10.2.4](https://www.rfc-editor.org/rfc/rfc3261) binding refresh, [§17.1.2](https://www.rfc-editor.org/rfc/rfc3261) non-INVITE transaction.
- [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c), authentication of server responses and replay checks.

## Observed wire format: §4.1.2 (redacted descriptions, not copied credentials)

The published example explicitly shows a REGISTER request with **Expires: 0** and a subsequent **SIP/2.0 200 OK** with Expires: 0. It shows the following relevant request fields (all values are sample-only):

| Field / feature | Observed example | Research consequence |
|---|---|---|
| Request-URI | sip:example-domain (in published example: fabrikam.com) | Registrar's SIP domain; not necessarily the TLS endpoint address |
| Via | SIP/2.0/TLS host:port | Transport description; no claim that TLS negotiation itself authenticates the user |
| From and To | Same user AoR, From includes `tag` and `epid` | Endpoint identity may need to be retained for unregister |
| Call-ID, CSeq | REGISTER with numeric CSeq (example 3) | Transaction identity must be correlated across request/response |
| Contact | URI plus `transport=tls`, `ms-opaque`, `methods`, `proxy=replace`, `+sip.instance` | Endpoint instance and route/contact metadata need source-specific grammar and registration-binding semantics |
| Authorization | NTLM example with `qop=auth`, realm, targetname, opaque, version 3, gssapi-data | Unregistration may carry authentication. **Do not copy or reuse original example token.** |
| Event | `registration` | Microsoft profile metadata; applicability across versions to be verified |
| Expires | **0** | Request expresses binding removal; distinguish from unrelated disconnect or TLS close |
| Allow-Events and Supported | Microsoft presence/provisioning/roaming packages, `gruu-10`, `adhoclist`, `msrtc-event-categories` | This is evidence of negotiated feature inventory; not proof all features are enabled |
| Content-Length | 0 | Example is bodyless REGISTER |

Observed response includes **Authentication-Info: NTLM** with `rspauth`, `srand`, `snum`, realm, targetname and opaque; unchanged Call-ID/CSeq; `Expires: 0`; `ms-keep-alive`, `Allow-Events`, and `Supported` headers. **Do not regard 200 alone as proof of authenticated sign-out**: the MS-SIPAE MIC/SA validation and RFC 3261 transaction checks form independent prerequisites for interpreting a protected reply.

## Layered end-to-end unregistration model

```mermaid
sequenceDiagram
 participant U as SIP client
 participant T as RFC3261 transaction layer
 participant S as registrar
 U->>T: REGISTER with Contact + Expires 0; authorization where applicable
 T->>S: send over selected transport
 S-->>T: example 200 OK with matching Call-ID/CSeq and Authentication-Info
 T-->>U: correlated response
 Note over U: verify protected response with MS-SIPAE SA and replay rules
 Note over U: then apply MS-SIPREGE registration binding lifecycle
 Note over U: connected dialogs, presence and provisioned services need separate cleanup rules
```

**Classification:** the sample REGISTER/200 fields come directly from §4.1.2; correlation and MIC verification are derived from independently documented RFC 3261/MS-SIPAE layers. The last cleanup note is *an open cross-document requirement*, not an already-proven exact Microsoft state transition. The diagram omits challenge/retry variants and failures.

## Failure, renewal and multi-endpoint questions

1. **Binding removal scope:** RFC 3261 §10.2.2 distinguishes per-Contact removal from wildcard Contact `*` with expiration zero. The Microsoft example uses a specific Contact plus `Expires: 0`. Determine whether all registration bindings or just the intended endpoint are affected, subject to registrar behavior and Contact matching clauses; do not presume it unregisters every endpoint of the account.
2. **Response security:** if the returned protected response fails matching SA, MIC or replay validation, the client must not treat the example 200 as authoritative completion. See MS-SIPAE §3.2.5.2.
3. **Presence and provisioning:** MS-PRES category expireType=endpoint, expireType=user and registration-bound subscription lifetimes imply that unregister triggers dependent lifecycle considerations. The exact server-driven notifications and client cleanup are **not yet determined**.
4. **Registration refresh:** unregistration vs ordinary refresh under RFC 3261 §10.2.4 and MS-SIPREGE must be represented as distinct operations.
5. **Auth and retry:** what if the unregister itself receives 401/407, 423, timeout, non-2xx or a home-pool redirect? Resolve via the exact target-revision MS-SIPREGE and MS-SIPAE clauses, not by extrapolating from the successful example.
6. **Connection semantics:** `Expires: 0` requests removal of a SIP registration binding; it does not automatically prove TCP/TLS connection closed, Windows credentials disposed or all remote dialogs terminated.
7. **Version specificity:** published example identifies older Office Communicator user-agent and protocol features, while this project's target is Skype for Business Windows Full Client; build a feature/version applicability matrix before adoption as universal behavior.

## Documented vs inferred vs missing

**Directly observed:** registration example request/response fields and sequence from official §4.1.2, official rev 17.1 publication metadata, Microsoft definitions of GRUU and in-band provisioning.

**Inferred using cross-spec sources:** transaction matching and per-SA server response verification are separate gates; endpoint identity and Contact scope must be preserved across unregister.

**Missing:** published rev 17.1 full-body extraction; normative MS-SIPREGE §2/§3 method processing, fully enumerated in-band provisioning, authentication challenge chains and failure recovery; all mandatory Windows Full Client functional flows; historical-version comparison.

**Status: PARTIAL.** No implementation changes, no real-server tests, no sign-in/out interoperability claims. Owner exclusively performs tests with real servers.
