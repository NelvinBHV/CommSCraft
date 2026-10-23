# MS-OCER 11.0 — private/public SIP diagnostics, error reporting and presence-version failure map

**Research: 2026-10-23, INCOMPLETE, no implementation or server tests.** Microsoft original **MS-OCER rev11.0, v20250520, May 20 2025**, SHA256 **1625bee265af9b3c6d927728a936667eeb20fe04c3c9b4e0962c31326f402682**; official https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OCER/%5BMS-OCER%5D.pdf . Direct originals §§1.3–1.7,2.1–2.2.2,3.1.4.1–3.1.5,7.3 (2000–2053 selected),9.1–9.2,10.1–10.2; PDF pp11,17 independently rendered/inspected. Compare RFC3261 §§7–8,17,20 and RFC5234; link [MS-PRES] rev13, [MS-SIP] rev8, [MS-SIPREGE] rev17.1 and [MS-SIPAE] rev11. **P** means Microsoft source/actor obligation; **S** cross-document inference; **U** incomplete profile. Source §7.3 is **Presence**, not a stand-alone UCS specification.

## I. Header grammar and security roles

| ID | Source | Exact role-scoped original finding |
|---|---|---|
| OCER-01 | §§1.3,2.1 | Two separate operations: server-to-client SIP response/request **diagnostics headers**, and optional client-to-server error report XML via SIP SERVICE. No new SIP transport and no independent version negotiation (§1.7). |
| OCER-02 | §§2.2.1–2.2.1.1 | SIP **server SHOULD attach one ms-diagnostics header** to relevant response, **MUST NOT attach more than one**, MAY add one to BYE or CANCEL; **client MUST NOT add this server-originated header** to its own messages. |
| OCER-03 | §2.2.1.1 | Private diagnostic ABNF: ms-diagnostics: **ErrorId;reason=quoted;source=quoted;optional generic parameters**. ErrorId is unsigned; server source SHOULD give FQDN/IP. **Client SHOULD base action on ErrorId, SHOULD NOT on free-text reason.** |
| OCER-04 | §§2.2.1–2.2.1.2 | Private ms-diagnostics can reveal internal server names and **SHOULD be removed at enterprise boundary**; federated/unauthenticated peers can receive **ms-diagnostics-public**. Public ABNF **MUST NOT contain source** and server MUST NOT attach more than one; client MUST NOT originate. |
| OCER-05 | §2.2.2 | Client sends SIP **SERVICE with application/msrtc-reporterror+xml**, root reportError and child error, containing details of the original **failed SIP transaction**, not replay of that failed request. |
| OCER-06 | §2.2.2 | Required XML fields: original Call-ID, failed SIP request method **max 33 chars**, SIP response code unsigned; optional original From/To URI, From/To tag, Content-Type **max 257 chars**. Combined From/To/Call-ID/tags **MUST NOT exceed 669 characters**. |
| OCER-07 | §2.2.2 | Optional diagnostic header text and progressReports list permit preserving diagnostics from multiple provisional responses; individual diagHeader length **MUST NOT exceed 65,535 chars**. Report SERVICE has its **own** SIP Call-ID/CSeq; original failed request Call-ID stored inside XML. |
| OCER-08 | §3.1.5 | Server **SHOULD return SIP 200** when report SERVICE processed; 503 without body if service disabled, or 503 with Retry-After when enabled/busy; client **SHOULD wait stated interval** when retrying. |
| OCER-09 | §3.1.5 | Server with unavailable report service SHOULD attach **ErrorId=2019** to 503. If 503 carries 2019 **or no diagnostics header**, client **SHOULD NOT retry within current login session**. Crucial distinction from 503 with busy Retry-After. |
| OCER-10 | §3.1.5 | Oversized report triggers server **413 SHOULD** and client **SHOULD NOT retry**. The QoE Monitoring Agent’s separate **300-KB bound** is NOT the OCER field-length rule. |
| OCER-11 | §§3.1.1–3.1.7 | Protocol does not specify own fixed retry timer or persistent upload queue. SIP RFC semantics/Retry-After are separate from account registration, ICE or RTP recovery. |

## II. First-party ErrorId values for identity, presence and contact synchronization

**Important:** these are **ErrorId numbers in Microsoft diagnostic headers**, NOT standalone SIP status codes. From **MS-OCER rev11 §7.3**:

| Diagnostic ID | Meaning in the original | Client system state / relevance |
|---|---|---|
| 2003 | Attempt to overwrite read-only category publication | MS-PRES category policy, not stale version |
| 2004 | Subscriber not authorized for presence | Privacy container/ACL; not proof SIPAE credentials invalid |
| 2006 | Container member limit exceeded | Privacy container quota |
| 2007 / 2008 | Contact count / group count exceeded | Legacy roster/service mutation quotas |
| 2013 (public) | msrtc-event-categories extension required | Client feature tag/profile support |
| 2014 (public) | Subscription dialog does not exist | SUBSCRIBE recovery rather than automatic account sign-out |
| 2016 (public) | New subscription required | Must classify dialog-specific re-subscription; no assumed general REGISTER |
| 2017 | Registration expired or unknown | Potential MS-SIPREGE binding recovery |
| 2019 | Report error service not available | Suppress repeat error-report SERVICE this login (§3.1.5) |
| 2035 | Directory search authorization denied | Directory permission, not missing contact |
| 2039 | Contact change authorization denied | Roster/contact mutation policy |
| **2044** | Publication version out of date | MS-PRES category version |
| **2045** | Container version out of date | MS-PRES container membership version |
| **2046** | Contact version out of date | MS-SIP contact deltaNum version |
| **2047** | Permission version out of date | Legacy ACL/permission version, distinct from contact list |
| 2048 / 2049 | Duplicate instance / duplicate container | Batch semantic error, not generic server busy |
| 2052 | Presence document too large | Category/quota error, not QoE 413 |

**New cross-source reconciliation:** MS-PRES and MS-SIP previously appeared to require four version counters. MS-OCER **explicitly assigns four distinct ErrorIds 2044–2047**, confirming their failure states cannot be collapsed to one generic version-conflict recovery action. A bare SIP 409 does not specify which version is stale without body/validated diagnostic and request context.

## III. Product-era error priority and privacy

| ID | Original source | Caveat |
|---|---|---|
| OCER-12 | §9.1.1 | Communicator user-facing priority on simultaneous diagnostics: **Microsoft diagnostic > SIP error > SIP warning > Windows error**, with unknown errors showing mode-specific generic fallback. This is Microsoft-product behavior, NOT a generic RFC3261 UAC MUST. |
| OCER-13 | §9.2 | IM/file transfer errors in IM UI; A/V or collaboration can use conversation notification bar, detail/support link or remediation button based on mode. A support URL from external diagnostic cannot be blindly trusted. |
| OCER-14 | §10.1.1 | Another edition-specific UI/error mapping retains diagnostic priority but differs in details; original **§11 client applicability and product notes** must be checked before declaring exact SfB Windows Full Client wording. |
| OCER-15 | §§2.2.1–2.2.2 | **S trust inference:** Internal server FQDN/IP, original Call-ID and progress diagnostics may be sensitive. Do not forward private diagnostics across federation or upload to unknown remote services; trusted source and telemetry retention policy separate. |

## IV. Error reporting is not session recovery (synthesized diagram)

~~~mermaid
flowchart TD
  A[Failed SIP transaction with optional progress diagnostics] --> B{Header trust and user context}
  B -->|Internal| C[ms-diagnostics ErrorId, optional source]
  B -->|External| D[ms-diagnostics-public without source]
  B -->|No diagnostics| E[SIP status and method only]
  C --> F[Classify SIP response and source-specific ErrorId]
  D --> F
  E --> F
  F --> G{Optional OCER report SERVICE supported?}
  G -->|No| H[Source-specific UI and cleanup]
  G -->|Yes| I[Report original failed CallID/method/code and progress XML]
  I --> J{SERVICE response}
  J -->|200| K[Report processed by reporting service]
  J -->|503 busy Retry-After| L[Wait before possible retry]
  J -->|503 with 2019 or no diagnostics| M[SHOULD NOT retry in this login]
  J -->|413| N[SHOULD NOT retry oversized report]
~~~

**S diagram:** SIP 430 route failure, STUN 430, TURN 401, E911 XML ReturnCode 404, QoE SERVICE 202 and OCER SERVICE 200 remain **different status and authentication namespaces**. Reporting a failure does not repair media, refresh REGISTER or reestablish conference Focus membership.

## V. Unfinished source obligations

- Full ~207-page §§7/8/9/10/11 ErrorId tables, 24xxx client codes, product applicability and per-feature action/recovery matrix: **not exhaustively audited**.
- Complete header ABNF/generic parameter and SIPAE MIC cross-boundary coverage, duplicate input and untrusted reason-string behavior.
- OCER §7.3 beyond selected rows, including UCS migration and presence SERVICE error handling; distinguish 403 authorization from 409 optimistic concurrency and available diagnostic.
- 503 busy vs unavailable, Retry-After absence and RFC3261 transaction rules, without infinite retry.
- Privacy/retention, diagnostic upload enablement and user-visible support links for exact Windows build and enterprise deployment: **unverified**.
- No evidence that optional failure-report upload blocks client logout; do not invent such dependency.

**Overall research INCOMPLETE; no protocol implementation, server traffic/interoperability tests, main merge or release.**
