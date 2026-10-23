# MS-OCEXUM rev11 — Exchange UM subscriber access skip-PIN, trusted SIP transport and Play-On-Phone no-diversion

**Original source check: 2026-10-23. Not a complete Windows Full Client or Exchange UM deployment claim.** Microsoft **[MS-OCEXUM] Call Control for Exchange Unified Messaging Protocol Extensions**, revision **11.0**, **2025-05-20** (v20250520); local original PDF **20 pages**, SHA-256 **e478ffd207187cbaf6094b6e61c53f2d83db70ee46500ab54d5c58c01964f512** verified from bytes. Official PDF https://msopenspecs.microsoft.com/files/MS-OCEXUM/%5BMS-OCEXUM%5D.pdf (also surfaced in Microsoft public search); Microsoft overview [MS-OCSPROT] §2.2.2.1 / §2.5.2. Review **§§1.3–1.6, 2.1, 2.2.1–2.2.2, 3.1–3.2.7, 4.1–4.2**, with original PDF **page 8** rendered and inspected. Direct Microsoft original dependencies **[MS-EUMR]**, **[MS-SIPRE]**, **[MS-OXWUMS] Voice Mail Settings Web Service Protocol** (not present among the 150 archived Microsoft PDFs, but Microsoft [publishes official editions](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxwums/79ace092-b793-4fa5-82b0-dd30d12c14c2)); RFC **[RFC3261](https://www.rfc-editor.org/rfc/rfc3261)** and **[RFC5234](https://www.rfc-editor.org/rfc/rfc5234)** and [RFC2119](https://www.rfc-editor.org/rfc/rfc2119). **P** original MUST/SHOULD, **S** cross-layer inference, **U** unresolved product profile.

## Findings grounded by clauses and roles

| ID | Clause | Message/state conclusion |
|---|---|---|
| OCX-01 | §2.1 | Custom SIP extension messages use **TCP or TLS**. Section §3.1 specifically requires trusted transport **for PIN-suppression behavior**; generic ability to carry SIP over TCP is **not** permission to accept unauthenticated PIN suppression. |
| OCX-02 | §2.2.1 | `Ms-Exchange-Command: skip-pin` is a single **valueless** extension parameter on an **INVITE** originating from the protocol client. No other `Ms-Exchange-Command` action is defined in this extension. Do not send arbitrary invented command values. |
| OCX-03 | §§2.2.1,3.1 | Protocol client **MUST first authenticate to SIP server** before it can set `skip-pin`; UAC subscriber-access call over trusted transport **SHOULD** add it. UM server **MUST skip PIN prompt if INVITE arrived over trusted transport** and carries this header. This relies on trust delegated by upstream authenticated SIP infrastructure. |
| OCX-04 | §3.1 | “Skip PIN” is the **UM system’s user-experience choice under trusted signaling**, not a general authorization override. Trusting an unverified `Ms-Exchange-Command` header from external proxy/input would cross the authenticated server boundary and is unsupported by source. |
| OCX-05 | §§2.2.2,3.2 | `Ms-Sensitivity: private-no-diversion` applies to **UM-origin Play-On-Phone/Dial Out INVITE** to user's device; **UM SHOULD send** it; proxy receiving such INVITE **MUST NOT apply immediate or unanswered call forwarding** back to voice mail. This avoids redirect loops and allows telephone playback without mailbox diversion. |
| OCX-06 | §3.2 | UM-origin INVITE must have **same address** in **From and To**; described as necessary for protocol client to ring for Play-On-Phone even when user selects **Appear Offline** presence state. This is an explicit special-case, not authorization for all offline-presence calls to bypass privacy rules. |
| OCX-07 | §3.2 | UM Dial Out is triggered by **Exchange Web Service/EWS [MS-OXWUMS]**, not by arbitrary SIP REGISTER 200 or presence SUBSCRIBE. A user requesting Play-On-Phone in some app may cause EWS request → UM-origin INVITE → desktop rings, all conditionally by product. |
| OCX-08 | §§3.1.1–3.1.7,3.2.1–3.2.7 | Both role sections explicitly say **no extension-specific abstract data model, timers, or local event rules**; the SIP INVITE/dialog/auth/transaction lifetimes are **inherited from RFC3261/MS-SIPRE/SIPAE**, not established to be absent. |
| OCX-09 | §3.2; MS-EUMR rev9 §3.2.4.1 | `private-no-diversion` affects both how proxy treats forwarding and whether certain **UM missed-call notifications** are generated. A Play-On-Phone-specific suppress-forward decision is not equivalent to setting blanket DND or normal Presence ‘Appear Offline’. |
| OCX-10 | MS-OCSPROT rev9 §2.2.2.1 | The overview groups OCEXUM in “SIP signaling extensions”; it is not a SOAP service for controlling Outlook mailbox settings. Its web-service dependency **MS-OXWUMS** is a **separate Exchange protocol family**, requiring its own historic/version-specific assessment. |

## Source-constrained call paths (S, separate initiated directions)

```mermaid
sequenceDiagram
 participant W as SfB Windows UAC
 participant F as Trusted SIP Front End
 participant U as Exchange UM
 participant E as Exchange Web Service
 opt Subscriber access inbound to UM
   W->>F: SIP authenticated INVITE, optionally Ms-Exchange-Command skip-pin
   F->>U: Trusted SIP signaling toward UM
   U-->>W: Voicemail audio via separate negotiated SIP/SDP path
   Note over F,U: Skip PIN only on trusted INVITE, not untrusted header
 end
 opt Play-On-Phone dial-out initiated by app
   W->>E: Application-initiated Play-On-Phone (protocol details MS-OXWUMS)
   E->>U: Web service triggers voice-message playback
   U->>F: INVITE From=To; Ms-Sensitivity private-no-diversion
   F->>W: Route call without immediate or unanswered forwarding
   Note over F,W: Appear Offline special ringing behavior is documented
 end
```

**S caveat:** W→E invocation is **one possible initiating application** in the diagram, not a direct normative claim that a particular 2015 SfB executable implements the complete [MS-OXWUMS] SOAP client. Above paths intentionally omit detailed SIP 100/180/183/200/ACK and RTP media messages until those are sourced from MS-SIPRE, EUMR, EUMSDP and RFC3261; no invented packet ordering.

## Critical open questions

- **OCX-U1:** [MS-OXWUMS] official original and historic client-era editions **are in the wider 150-PDF archive as a secondary source but are not indexed in the 54-item primary inventory**. The initial “missing PDF” classification was incorrect; see [MS-OXWUMS rev16 direct original study](ms-oxwums16-ews-playonphone-soap-32s-call-state-source-2026-10-23.md). Its EWS authentication and Play-On-Phone SOAP cases are partly analyzed, full 107-page normative closure remains open.
- **OCX-U2:** Trusted path and `skip-pin` authorization, SIPAE end-to-hop authentication, TLS transport identity, UM user mapping and identity delegation are distinct. No claim of passwordless external voicemail access warranted.
- **OCX-U3:** `Appear Offline` special-case logic must be checked against MS-PRES category privacy and user builds; not evidence generic privacy override.
- **OCX-U4:** To establish exact classic Windows desktop supported modality, use dated client build/Exchange UM edition notes. MS-OCEXUM rev11 (2025) states protocol extension behavior, not that Exchange UM remains available in every contemporary environment.
- **OCX-U5:** Explicit syntax and call sequence examples are tested against the original; full normative dependencies on RFC3261/MS-SIPRE redirect/header trust, Exchange SOAP and later errata remain **OPEN**.

**Overall research incomplete; no implementation, server connection, interoperability test, merge or release.**
