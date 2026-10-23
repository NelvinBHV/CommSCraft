# RFC3261/3428 + MS-SIPRE 13.0: SIP dialog, MPOP endpoint GRUU, fork races and 430 route recovery

**Source-only research, 2026-10-23; classic Windows full-client applicability and full normative closure NOT COMPLETE.** Primary Microsoft [MS-SIPRE] **rev 13.0, edition v20250520 (May 20, 2025)**, original PDF SHA-256 `9dc4121780f66dc958f5d9e906594cfbc8b5535410e5f18592a8331227e969ae` verified from archived original bytes, 116-page PDF pp23,39,42,53,54 visually rendered/checked. Official original PDF: https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-SIPRE/%5BMS-SIPRE%5D.pdf . Source sections reviewed: §§2.2.2–2.2.7,2.2.11–2.2.13,2.2.18–2.2.19,3.2.3–3.2.5,3.3.3–3.3.5,3.4.5.1–3.4.5.2,3.6.5.1,3.7.1–3.7.6,3.9.5.2.2–3.9.5.6 and §4 version notes. Microsoft supporting [MS-SIPREGE] rev17.1 August 19 2025 (not same date), [MS-PRES] rev13 May20 2025, [MS-SIPAE] rev11 May20 2025, [MS-OCSPROT] rev9 §§2.5.9,2.5.11. RFC Editor originals: [RFC3261](https://www.rfc-editor.org/rfc/rfc3261.html) June 2002 §§9,12,13,15–17,22; [RFC3428](https://www.rfc-editor.org/rfc/rfc3428.html) December 2002 §§4–7; [RFC4028](https://www.rfc-editor.org/rfc/rfc4028.html) April 2005 §§7–10,13; [RFC6026](https://www.rfc-editor.org/rfc/rfc6026.html) September 2010 §§2–6 (explicitly **changes transaction handling of INVITE 2xx**). Microsoft GRUU reference is **[IETFDRAFT-OUGRUAUSIP-10]** in the pinned PDF, not automatically the modern final [RFC5627](https://www.rfc-editor.org/rfc/rfc5627.html) (2009). **P** Microsoft primary, **R** IETF original, **S** conditional synthesis, **U** unresolved.

## 1. Identity layers and routing — no one-dialog-per-human invariant

| ID | Section | Verified behavior and scope |
|---|---|---|
| ROUTE-01 | MS-SIPRE §2.2.3 (P) | Microsoft defines GRUUs for **user agent**, voicemail, location-profile, conference endpoint and server instance. They have `gruu` parameter and sometimes `opaque`, not the same as an unqualified user AoR. The source refers to IETF draft syntax; final RFC5627 compatibility needs product/edition review. |
| ROUTE-02 | §2.2.5, MS-SIPREGE §3.1.1.4.1 (P) | `Contact: ...;+sip.instance="<urn:uuid:...>"` has **UUID URN** syntax; an `epid` carried in From/To must be consistent with UUID-derived endpoint identity. An endpoint UUID is NOT the human's SIP URI (AoR). |
| ROUTE-03 | §3.2.4.1 (P) | On **every outgoing SIP request**, compliant UA MUST place own `epid` in **From**. Dialog stores remote `epid` independently. For a mid-dialog request targeting an identified remote endpoint, it uses the stored remote `epid` in **To**. |
| ROUTE-04 | §3.2.5.1 (P) | If inbound request `To` contains an `epid` that does **not match own endpoint epid**, the receiving UA **MUST discard** it without processing or responding. Blind 404/481 response would violate this specific rule. |
| ROUTE-05 | §3.3.5.1 (P) | Registrar validates `+sip.instance` UUID-URN and agreement with name-derived `epid`. Mismatch ⇒ **400** for REGISTER; otherwise add UUID to location service. A valid SIPAE token cannot override invalid endpoint binding identity. |
| ROUTE-06 | §3.3.5.2 (P) | Proxy may index by `AoR + sip.instance UUID` and MUST validate UUID before use. If request/response `epid` conflicts, SHOULD return 400. The server's binding lookup, local dialog key, and SIPAE security association index have **different identity scopes**. |
| ROUTE-07 | §3.4.5.1 (P) | Registrar creates GRUU so it routes to proxy with access to location service; it MUST include URI parameter `gruu`, and generated opaque encoding is **not a client-permitted arbitrary opaque-value generator**. |
| ROUTE-08 | §3.4.5.2 (P) | Unknown GRUU on routed in-domain request ⇒ **404**. No selected live registration for a known GRUU mid-dialog ⇒ **480 SHOULD**. Proxy MUST restrict location records to matching GRUU, then copy/create `grid` parameter and possibly Route info. |
| ROUTE-09 | MS-OCSPROT §2.5.11; RFC3261 §§12,13.2.2.4 (P/R) | A caller may reach **multiple registered endpoints** via forked INVITE. A **distinct 2xx with a different To tag creates its own confirmed dialog**. Each 2xx must be independently ACKed; proxy trying to CANCEL losing branch **cannot guarantee** loser won't already have produced another 2xx. |
| ROUTE-10 | MS-SIPRE §2.2.11 (P) | **`Ms-Forking: Active` appears only in 1xx**, indicating proxy may parallel/serial fork so endpoints can gate early media. It does not guarantee only one winning 2xx or unconditionally cancel all remote dialogs. |
| ROUTE-11 | RFC3261 §§9,13.2.2.4,16.7 (R) | CANCEL applies to **outstanding INVITE transaction**, not any **already confirmed dialog**. Pending branch generally yields 487 to INVITE and 200 for CANCEL separately. After 2xx crossed CANCEL, UAC still ACKs and can terminate unwanted confirmed dialog via BYE. |
| ROUTE-12 | RFC6026 §§2–4 (R) | RFC6026 introduces **Accepted state / Timer M** for 2xx processing in SIP INVITE client transaction; it changes older RFC3261 INVITE client transaction behavior. This RFC edition is a **potential dependency requiring feature/version audit**, not proof classic Skype for Business binary implements it. |
| ROUTE-13 | MS-SIPRE §3.9.5.2.2 (P) | SIP proxy **routing scripts** consider privacy-facing published state, delegate/team/work hours/voicemail for **incoming call INVITEs**, with specific order and DND policy. **Do not assume** these audio-call routing rules govern all 1:1 instant-message `MESSAGE` transactions. |
| ROUTE-14 | §3.9.5.6 (P) | On winning 2xx proxy SHOULD generate branch CANCEL with **`Reason` ms-acceptedby** AoR. This metadata describes routing outcome, NOT proof no late 2xx or recipient read receipt. |

## 2. Dialog route-set state: failing the connection does not equal failing SIP principal, content, or RTP

| ID | Clause | Recovery normative contract |
|---|---|---|
| REC-01 | §2.2.18 (P) | `430 Flow Failed` may carry **`P-Dialog-Recovery-Action`** whose recognized list includes `Registration-Route-Set-Update`, `Dialog-Route-Set-Update`, `Wait-For-Session-Update`. They are **different actions**, not interchangeable retry hints. |
| REC-02 | §2.2.19 (P) | Dialog recovery is negotiated by **`Supported: Ms-Dialog-Route-Set-Update`**; absence of support matters. `Ms-Safe-Transfer` is a **separate extension** and shouldn't be collapsed into route recovery. |
| REC-03 | §3.7.4.1 (P) | UA enabling recovery for a **particular dialog** MUST add Supported tag to **all requests on that dialog**, and SHOULD periodically send **target refresh**. INVITE dialog may use RFC4028 session timer, SUBSCRIBE dialog RFC3265 refresh. |
| REC-04 | §3.6.5.1, §3.7.5.1 (P) | Proxy with lost record-route HMAC key may send **481**. If recovery option is absent and routing state lost, SHOULD reject with **481**. If option tag present, proxy **MUST send 430** with explicit recovery action. A generic 481 and actionable 430 are **not identical**. |
| REC-05 | §3.7.5.2 (P) | Registrar seeing dialog-creating request with GRUU and invalid/absent binding or `Routable=FALSE` SHOULD send **430** with **both Registration-Route-Set-Update and Dialog-Route-Set-Update**. This is a registration context problem, not proof remote user blocked the IM. |
| REC-06 | §3.7.5.3.1 (P) | For 430 **Dialog-Route-Set-Update alone**, UA MUST run target refresh and **on success re-send only the original failed request** with updated route set / remote target. If recovery fails MUST report original operation failure. |
| REC-07 | §3.7.5.3.1 (P) | For **Registration-Route-Set-Update** action, refresh endpoint REGISTER; on success MUST recover all recovery-enabled dialogs bound to endpoint, SHOULD terminate/recreate recovery-disabled dialogs, and MAY/SHOULD retry original dialog-creating operation as scenario-specific. A full-register retry is NOT the same as retransmitting original MESSAGE. |
| REC-08 | §§3.7.5.3.1,3.7.5.3.3,3.7.6.1 (P) | **Wait-For-Session-Update** with an existing RFC4028 session timer: **recovery refresh timer** interval SHOULD be at least negotiated session refresh interval, and its expiry MUST run §3.7.5.3.4 procedure. No fixed global retry 5/10/30 seconds in these clauses. |
| REC-09 | §3.7.5.3.1 (P) | After repeating **same** request **two or more times** and receiving 430 again, UA **SHOULD stop retrying and report failure**. Endless 430 retry loop is not sourced. Exact request-side counter reset semantics remain U. |
| REC-10 | §3.7.5.3.2 (P) | REGISTER refresh successful with `Presence-State: Register-Action=added|fixed` can trigger recovery of dialog route sets, and termination/re-creation for dialogs without recovery support. Align with MS-SIPREGE rev17.1's **renewal unexpectedly replaced binding** cases. |
| REC-11 | §3.7.5.3.4 (P) | **Actual dialog recovery** sends target-refresh request (**UPDATE** on INVITE-based dialog or **SUBSCRIBE** on SUBSCRIBE dialog), waits for transaction response, extracts **Contact and Record-Route** URIs and updates local route set and remote target. It may also recover logically-related conference dialogs. |
| REC-12 | §3.7.5.3.3 (P) | On incoming mid-dialog target-refresh, endpoint SHOULD update **route set + remote target** from Contact/Record-Route. If not, later requests can receive **430** even when earlier dialog remains logically established. |

## 3. RFC3428 message handling — 200 SIP response != human acknowledgment

| ID | IETF source | Precise scope |
|---|---|---|
| IM-01 | RFC3428 §§4,7 (R) | SIP `MESSAGE` MAY be associated with existing dialog but **does not initiate a new dialog**; user agents MUST support **text/plain**. `Contact` MUST NOT appear in MESSAGE request or 2xx response. |
| IM-02 | §4 (R) | MESSAGE `200 OK` allows assumption delivered to **final destination**, but explicitly **not read by human**. `202 Accepted` means accepted by intermediate/store-forward/gateway and NOT evidence final endpoint delivery. |
| IM-03 | §7 (R) | UAS should respond promptly; may return 200 without displaying. `4xx/5xx` indicate unsuccessful delivery, but 6xx indicates successful arrival with refusal in this RFC's language; 2xx response MUST NOT include body. |
| IM-04 | RFC3261 §13 and MS-OCSPROT §2.5.9 (R/P) | The MS-OCSPROT sample **INVITE → 200 → ACK → MESSAGE → 200** is one valid *session-based* IM user journey, **not** proof every MESSAGE needs a preceding INVITE by base RFC3428. |
| IM-05 | MS-SIPAE §§3.2.4.1,3.2.5.2 (P) | SIP request/response authentication and canonical `cnum/crand/rspauth/snum` MIC is **separate from** transaction/dialog IDs and recipient read/delivery acknowledgment; authenticated 200 does not introduce a new application-level read receipt. |

## 4. State model: forked call plus routing recovery (S interpretation, not literal capture)

~~~mermaid
stateDiagram-v2
 [*] --> CallerInvite
 CallerInvite --> Forking: Proxy forwards INVITE to registered endpoints
 Forking --> Dialog1: 200 OK from endpoint A, distinct To-tag
 Forking --> Dialog2: 200 OK from endpoint B crossed CANCEL
 Forking --> RejectedBranch: CANCEL and 487 before final success
 Dialog1 --> Dialog1Ack: ACK that 2xx
 Dialog2 --> Dialog2Ack: ACK that 2xx
 Dialog2Ack --> ByeUnwanted: BYE if client does not want second branch
 Dialog1Ack --> MessageTransaction: In-dialog MESSAGE
 MessageTransaction --> DeliveredTransport: 200 OK, no read receipt
 MessageTransaction --> AcceptedIndirect: 202, final delivery not guaranteed
 MessageTransaction --> FlowFailed: 430 with recovery action
 FlowFailed --> RefreshDialog: Dialog-Route-Set-Update
 FlowFailed --> RefreshRegister: Registration-Route-Set-Update
 FlowFailed --> WaitSession: Wait-For-Session-Update
 RefreshDialog --> MessageTransaction: Refresh success; resend original with new route
 RefreshRegister --> RefreshDialog: REGISTER success; recover dialogs
 WaitSession --> RefreshDialog: Negotiated refresh timer
 RefreshDialog --> Failed: Retry threshold or target refresh fails
~~~

**States describe independent per-branch dialogs, per-SIP-request transactions and actor-specific route recovery**. Do not force transport-TCP status, SIPAE SA, conference MCU membership or end-user read status into a single `connected` boolean.

## 5. Conflicts, security, and remaining source closure

- **SIPR-X01 (recovery source wording):** §3.7.5.3.1 refers to "P-Dialog-Recovery-Action tag" although §2.2.18 defines header `P-Dialog-Recovery-Action` and specific **value actions**. Treat the clause as an apparent shorthand/inexact wording; compare official revisions/errata before binding this to any literal token.
- **SIPR-X02 (edition):** MS-SIPRE rev13 2025 vs SIPREGE rev17.1 August 2025, RFC3261 June2002, RFC6026 Sep2010 and historic draft GRUU definitions. Verify target Windows Full Client's supported tags, SIP timer behavior, feature advertisements and redirect.
- **SIPR-X03:** Exact RFC3261 non-INVITE timeout timers (E/F/K), INVITE A/B/D/H/I, reliable TCP handling, retransmission, CANCEL race when proxy has both branches and early media, RFC6026 Accepted state and Timer M need full state transition and product-era audit.
- **SIPR-X04:** The full MS-SIPRE **§3.9 routing script preamble** includes call-specific routing/forwarding and timers. This study maps selected branches; do not claim all every SIP invite/fork, and especially not IM group policy, is fully covered.
- **SIPR-X05:** HMAC Record-Route/Contact key rotation and opaque GRUU privacy/provenance need server-only vs Windows client trust audit; no deliberate fabrication of keys or route recovery packets.
- **SIPR-X06:** MS-CONFIM **IM MCU 202 + later IMDN BENOTIFY** is a **separate delivery contract** from direct RFC3428 MESSAGE and must be tracked with its own Message-Id and timeout.

**Full classic Windows full-client source system INCOMPLETE; no protocol code, live servers, own interop tests, main merge or release.**
