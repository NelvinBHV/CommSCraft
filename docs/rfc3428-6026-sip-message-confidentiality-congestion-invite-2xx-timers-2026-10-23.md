# IETF RFC3428/6026 original normative pass — MESSAGE confidentiality, congestion and INVITE accepted-state security

**Research-only, 2026-10-23. NO code/network/server tests.** Original IETF [RFC3428](https://www.rfc-editor.org/rfc/rfc3428.html) **December 2002** §§4–9,11–11.5, status **Standards Track**, and [RFC6026](https://www.rfc-editor.org/rfc/rfc6026.html) **September 2010** §§3–10, **Standards Track, explicitly Updates RFC3261**. RFC3261 original [June 2002](https://www.rfc-editor.org/rfc/rfc3261.html) §§9,12–13,16–17,18,26. These normative RFC pages were read directly on **RFC Editor HTML**, not second-hand summaries. Cross-spec Microsoft primary [MS-SIPRE] rev13 2025-05-20 §§2.2.18–19,3.2–3.4,3.7, [MS-SIPAE] rev11 2025-05-20 §§3.2.4–3.2.5, [MS-CONFIM] rev11 2025-05-20 §§2.2.1–6,3.3–3.4, [MS-OCSPROT] rev9 §§2.5.9,2.5.11. **R** = direct RFC normative clause, **S** = cross-document logical boundary, **U** = unknown legacy Windows client/server implementation and profile. A later RFC update applies to normative current IETF SIP behavior, but Microsoft client edition/platform support remains to be proven independently.

## RFC 3428: MESSAGE is payload-carrying SIP, not plain SIP signaling

| ID | Exact RFC | Normative actor/source requirement |
|---|---|---|
| MSG-R01 | RFC3428 §4 | UAC that supports MESSAGE **MUST send text/plain** and MAY support message/cpim. MESSAGE doesn't itself create SIP dialog and may operate within a dialog created by INVITE. |
| MSG-R02 | §4 / §7 | Request **MUST NOT contain Contact**; 2xx response likewise **MUST NOT contain Contact or message body**. Sender's GUI cannot derive stable recipient endpoint contact from 2xx of MESSAGE. |
| MSG-R03 | §4 / §7 | **200 OK** means sender may treat as reached final SIP destination, **not human read**. 202 means intermediate gateway/storer accepted, **not guaranteed delivered**. 4xx/5xx not delivered, 6xx in this RFC represents delivered but refused. |
| MSG-R04 | §§4,6 | Proxy may fork **MESSAGE itself**, and then **returns only one final response upstream** even when several endpoints received it successfully. **This is not identical** to forked **INVITE**, which can generate multiple 2xx/dialogs seen by caller. One response cannot be used to infer only one MESSAGE recipient. |
| MSG-R05 | §4 | To limit validity UAC **MAY add Expires**. When nonzero Expires included, UAC **SHOULD add Date**. UAS expired MESSAGE can make policy-dependent decision (§7). These do not replace REGISTER binding Expires nor conference IMDN sendMessageTimer. |
| MSG-R06 | §5 | `im:` URI may be resolved to SIP Request-URI; Record-Route/Route **MUST NOT contain IM URI**; From/To may use SIP or IM URI per available mapping. Direct `im:` address has no guarantee Microsoft classic Windows UI exposes all IETF URI syntax. |
| MSG-R07 | §8 | **Out-of-media-session MESSAGE size MUST NOT exceed 1300 bytes**, except when UAC positively knows every hop safe from congestion OR size at least 200 bytes below minimum path MTU. **Exception for MESSAGE inside media session** exists; do NOT impose unconditional 1300-byte cap on all conference MCU messages. |
| MSG-R08 | §8 | UAC **MUST NOT start second out-of-dialog MESSAGE** for same URI while one already pending; **SHOULD NOT overlap in-dialog** unless each route-set hop congestion controlled (then concurrency MAY be permitted). This is **application transaction concurrency**, not SIP 10-second IMDN timer. |
| MSG-R09 | §§8–9 | Congestion-controlled transport like TCP/SCTP SHOULD be used, but UAC's TCP choice does **not** prove every downstream hop avoids UDP. MESSAGE is a new uppercase case-sensitive SIP method; its 1xx provisional treatment follows non-INVITE method rules of RFC3261. |
| MSG-R10 | §11 | Because message body carries actual conversation content, UAs **MUST implement end-to-end authentication, body integrity and body confidentiality mechanisms**. **Hop-by-hop SIP TLS alone is not an end-to-end body protection** when proxies see plaintext. |
| MSG-R11 | §§11.2–11.3 | RFC3428 says where such end-to-end protection is sought **MESSAGE bodies MUST be secured with S/MIME**, and endpoints **MUST support CMS enveloped encryption and CMS signed data**. In a Microsoft deployment, **SIPAE MIC authenticates canonical SIP headers**, and does **not by itself demonstrate RFC3428 S/MIME protection of IM payload**. Client product support and alternate MIME security profiles remain **U** — do not assert either compliance or noncompliance from these documents alone. |
| MSG-R12 | §11.4 | **Signed MESSAGE** and response MUST include **signature-covered Date** to prevent replay; stale/future dates SHOULD get **400** unless repeated sender abuse allows silent drop. Special store-forward cases may justify policy-controlled exceptions. SIPAE 256-slot `snum` replay is a **different security-layer mechanism**; don't conflate it with body/cert Date coverage. |
| MSG-R13 | §11.5 | `message/cpim` can protect IM metadata across gateway boundary with S/MIME, e.g. sender details; it is an **independent MIME content model**, not MS-CONFIM IMDN or `Ms-Sender` transport header. |

## RFC 6026 correct INVITE transaction life cycle (updates RFC3261)

| ID | Clause | Source requirement |
|---|---|---|
| INV-R01 | RFC6026 abstract, §§3,7 | Direct normative **Update RFC3261** repairs malformed handling of retransmitted 2xx INVITEs and previously forwarding “stray” 2xx, which is a security issue. This change is not an optional new media codec or proxy option tag. |
| INV-R02 | §7.2, §8.4 | On first 2xx while client INVITE transaction Calling/Proceeding, enter **Accepted**, pass 2xx to Transaction User, set **Timer M=64*T1**. Additional 2xx (including distinct To-tag from fork) accepted/passed to Transaction User while timer running. |
| INV-R03 | §7.2 | **Transaction layer MUST NOT generate ACK for 2xx**; **UAC core/Transaction User** must generate the distinct-dialog ACK. **INVITE 300–699 non-2xx ACK** follows different transaction-layer rules inherited from RFC3261. |
| INV-R04 | §7.1, §§8.5–8.7 | Server INVITE transaction adds own **Accepted state / Timer L=64*T1**, absorbing incoming INVITE retransmissions and forwarding ACK **to TU**; 2xx retransmissions remain handled by UAS core/transaction-user behavior. The client **M** and server **L** timers have different roles despite same expression. |
| INV-R05 | §7.2, §8.4 | On Timer M expiry Accepted→Terminated, remove transaction. Unmatched stray response at a **stateful** element MUST be discarded per revised RFC6026 routing rules, except explicit stateless-proxy case; do not forward fake 2xx via arbitrary Via, preventing reflection abuse. |
| INV-R06 | §§6–7, §10 | Retaining accepted INVITE transaction state costs extra memory/time (for default RFC3261 T1=500ms, **64*T1=32s**, not a universal hard-coded value). RFC6026 explicitly discusses denial-of-service trade-off of more retained state against preventing stray-response abuse. |
| INV-R07 | RFC3261 §§9,13.2.2.4 & RFC6026 §7.2 | Proxy **CANCEL** for losing branch can race with **2xx success**, yielding multiple distinct dialogs. CANCEL 200 (for CANCEL request itself) is **not** success response to original INVITE. UAC handles each 2xx and unwanted accepted dialog may require BYE. |
| INV-R08 | MS-SIPRE §3.7 vs RFC6026 (S) | **Dialog route recovery timer** for 430 `P-Dialog-Recovery-Action`, RFC6026 **Timer M/L**, SIPAE **SA expiry**, registrar Expires, and MS-CONFIM **10s IMDN**/MCU **40s history** must be modeled **independently**, even if same user's UI reports one conversation. |

## Two materially unresolved cross-spec issues

**RISK-01 (privacy and trust):** RFC3428 §11 mandates implementations supporting end-to-end body auth/integrity/confidentiality. Microsoft MS-SIPAE MIC and transport TLS cover **different** aspects. MS-CONFIM rich-text `multipart/alternative` and MCU recipient-specific content adaptation requires plaintext interpretation by MCU in documented paths, which is **not generally compatible with arbitrary opaque end-to-end encrypted MIME bodies without an explicitly supported decrypt/re-encrypt or alternate security model**. This is a **technically grounded conflict of trust assumptions (S)**, NOT proof that classic client implements S/MIME or any particular IM encryption. Need historical Microsoft security/product documentation and explicit server trust boundary.

**RISK-02 (transaction exactness):** RFC6026 is a **normative IETF RFC3261 update**, but [MS-SIPRE] rev13 source refers mainly to RFC3261 and Microsoft-specific recovery. Full client on-wire/client-process behavior under fork/late-200 and stateful proxy state handling needs historically accurate Windows build evidence. Do not label the 2025 Microsoft SIPRE source as explicit RFC6026 adoption merely because modern IETF revision exists.

### State machines (R source + S association)

~~~mermaid
stateDiagram-v2
 [*] --> Calling: INVITE client transaction
 Calling --> Proceeding: provisional 1xx
 Calling --> Accepted: first 2xx and Timer M=64*T1
 Proceeding --> Accepted: 2xx and Timer M
 Calling --> Completed: non2xx 300-699; transaction ACK
 Proceeding --> Completed: non2xx 300-699; transaction ACK
 Accepted --> Accepted: any matched late/retransmitted 2xx delivered to TU
 Accepted --> Terminated: Timer M fires
 Completed --> Terminated: Timer D fires per RFC3261 profile
 Terminated --> [*]
~~~

~~~mermaid
flowchart LR
 A[MESSAGE body with MIME] --> B{Trust and delivery model}
 B -->|RFC3428 direct| C[200 final destination not read, or 202 intermediary]
 B -->|MS-CONFIM MCU| D[202 MCU + Message-Id]
 D --> E[Per-recipient MESSAGE forwards and 415 errors]
 E --> F[IMDN BENOTIFY or 10s 408]
 A --> G[Legacy RFC3428 §11 end-to-end MIME security requirement]
 G --> H[Microsoft product S/MIME support and MCU decryption profile OPEN]
~~~

**Overall research remains INCOMPLETE.** No protocol implementations, live server connections, interoperability tests, main merges or releases.
