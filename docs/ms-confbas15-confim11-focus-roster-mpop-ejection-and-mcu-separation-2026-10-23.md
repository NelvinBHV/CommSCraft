# Conference IM integration — MS-CONFBAS rev15 Focus, SIP subscriptions and per-endpoint ejection vs MS-CONFIM rev11

**Research 2026-10-23; code-free, no server traffic.** Microsoft first-party **[MS-CONFBAS] Centralized Conference Control Protocol: Basic Architecture and Signaling**, **revision 15.0**, publication **2025-05-20** (`v20250520`), SHA-256 **8ecae43533f6a7fa391358078093aa52c852b3b66c0b975d1ec06a883c285dfa**, [official PDF](https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-CONFBAS/%5BMS-CONFBAS%5D.pdf); direct original §§**3.2.4.1.1–2,3.2.4.2–3.2.5.2.4,3.4.4.2,3.8.5.1,3.9**. Pair with **[MS-CONFIM] rev11 v20250520** §§3.1–3.4 and [original conference-IM deep audit](ms-confim11-im-mcu-message-imdn-40s-history-typing-sender-rfc-original-2026-10-23.md). RFC references: [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) SIP (§§8,12,15), [RFC3265](https://www.rfc-editor.org/rfc/rfc3265) SIP events legacy, [RFC4028](https://www.rfc-editor.org/rfc/rfc4028) session refresh, [RFC4353](https://www.rfc-editor.org/rfc/rfc4353) conference framework, [RFC4575](https://www.rfc-editor.org/rfc/rfc4575) conference-state event package. Microsoft Learn [§3.2.5.2.2 multiple endpoint roles](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confbas/909fd641-d41c-40d6-bc35-17e51496b9dd) independently verified. **P** explicit actor-conditioned source; **S** composed requirement; **U** unsettled.

## 1. Focus entry, authorization and error precedence

| ID | Source | Verified actor-conditioned behavior |
|---|---|---|
| FCIM-01 | MS-CONFBAS §3.2.4.1.1 | Client's Focus **SIP INVITE MUST include valid `addUser` C3P request** per §§2.2.1,2.2.3.3; user/role/endpoint GUID are not interchangeable with SDP `m=message` media and MCU conference-URI. |
| FCIM-02 | §3.2.4.1.1 | Client joining **on behalf of another user MUST include `p-session-on-behalf-of` SIP header**, and SHOULD include matching C3P `session-on-behalf-of` element. Focus validates both when element present; mismatch SHOULD reject **403** (§3.2.4.2). Authenticated transport alone cannot grant delegated conference identity. |
| FCIM-03 | §3.2.4.1.1 | If client supports managing a conference **lobby**, it sets `lobby-capable=true`; ability to parse roster and ability to manage lobby are different capabilities. |
| FCIM-04 | §3.2.4.1.2 | **Anonymous conference join** is delegated to special [MS-SIPAE] §3.2.4.3, then conference control uses this protocol. **Anonymous conference admission is not enterprise REGISTER identity or general TLS-DSK domain authentication**. |
| FCIM-05 | §3.2.4.2 | Focus SHOULD authorize/confer membership; if unauthorized **MUST 403 Forbidden**; if authorized **SHOULD 200** and SIP dialog. C3P policy/lobby may still distinguish full admission vs limited waiting state under other clauses. |
| FCIM-06 | §3.2.4.2 | Focus receiving **CANCEL** before INVITE final SHOULD treat as participant leave, receiving **BYE** on established dialog SHOULD likewise treat as participant leave; these are SIP dialog events, not the same as IM MESSAGE forward failures. |
| FCIM-07 | §3.2.4.2 | Focus receiving **UPDATE** SHOULD extend SIP dialog lifetime per RFC4028 §9, MAY accept re-INVITE refresh. Source recommends **600 seconds** for re-INVITE-based lifetime; recommendation not global mandatory application timer. |
| FCIM-08 | §3.2.5.1 | Client SHOULD parse role from Focus 200 response and use returned role for remainder; do not trust only the role that client requested in addUser. |
| FCIM-09 | §3.2.5.2.4 | Focus distinguishes **400** malformed headers/body, **403** unauthorized, **404** conference not found, **603** participant size exceeded. These are conference-admission failures, not MCU per-recipient **415/408**, and not SIP transport TLS errors. |
| FCIM-10 | §§3.2.4.2,3.2.5.2.3 | Focus MUST notify subscribed other conference participants of new join; conference state schema/versions under §2.2.4 and RFC4575 govern user+endpoint deltas. This is not MCU forwarding IMDN. |
| FCIM-11 | §3.2.5.2.2 | Focus SHOULD allow **multiple connected endpoints of same participant**; if allowed, roster **MUST list all endpoints under that one user**. Focus **MUST manage each endpoint independently** and one endpoint Focus dialog termination **MUST NOT alter** other endpoints of that user. A single SIP account cannot use one shared endpoint dialog key. |
| FCIM-12 | §3.4.4.2 | Focus acts conference notification service per RFC4353/RFC4575 and SHOULD subscribe to each MCU. Roster is composed of Focus-local state and MCU state; a client's IM MCU receipt of endpoint capability may arrive asynchronously relative to Focus C3P 200. A universal message order is **NOT** specified (S inference). |
| FCIM-13 | §3.8.5.1 | **Eject a participant** (Focus): MUST terminate **all subscription dialogs for all endpoints** of the ejected user with NOTIFY Expires:0; participant removed reason differs from lobby denied reason; this is broader than closing just one IM MCU dialog. |
| FCIM-14 | §3.8.5.1 | Focus MUST terminate **all signaling dialogs of that user for all endpoints**, sending BYE with Reason text **Participant Removed** vs **Participant Denied**. Source recommends subscription termination before signaling termination, not MUST globally serialized. |
| FCIM-15 | §3.8.5.1 | Focus MUST notify **remaining participants** with updated roster and propagate removal command to **all MCUs**, including IM MCU. Thus MS-CONFIM §3.2.2.4.1 MCU must BYE IM session, and §3.2.2.5.1 MCU must send endpoint-deleted notification. |
| FCIM-16 | §3.9 | `deleteConference` against Focus terminates meeting and all users/MCUs but **MUST NOT change conference provisioning record**; to fully remove the scheduled conference use Focus Factory under **MS-CONFPRO**. Ending a session and deprovisioning reservation are different. |
| FCIM-17 | MS-CONFIM §§3.2.1.4.1,3.2.2.5.1 | **Ordinary IM MCU leave** is client **BYE MCU** and endpoint deletion, not automatic Focus ejection or entire account deregistration. Client may remain participant in other modalities only when Focus/MCU policy permits (S, not guaranteed). |
| FCIM-18 | MS-CONFIM §§3.3.2.5.1–2 + MS-CONFBAS §3.8.5.1 | Per-recipient IM forwarding **415/408** and IMDN are **message-delivery results**; Focus 400/403/404/603 and participant removal are **conference authorization/session-state events**; any UI logic that collapses them into generic 'disconnected' loses source-mandated distinctions. |

## 2. Interleaved but not interchangeable dialogs (S)

~~~mermaid
sequenceDiagram
 participant C as Windows client endpoint
 participant F as Conference Focus
 participant M as IM MCU
 participant O as Other endpoint same SIP user
 C->>F: SIP INVITE C3P addUser (role+endpoint)
 F-->>C: 200 role, membership
 F-->>C: SUBSCRIBE/roster conference view
 C->>M: Separate INVITE m=message and accept-types
 M-->>C: SDP 200 + endpoint capability roster
 opt Normal endpoint-level IM media leave
  C->>M: BYE IM MCU
  M-->>F: endpoint-deleted IM media notification
  Note over F,O: Another registered endpoint's Focus dialog remains independent
 end
 opt Administrative Focus deleteUser
  F->>C: NOTIFY Expires:0 / BYE (all ejected user's endpoints)
  F->>O: NOTIFY Expires:0 / BYE (when O is also ejected user's endpoint)
  F->>M: Propagate ejection command
  M->>C: BYE any active IM media leg
  F-->>C: Remaining viewers' roster updated (where applicable)
 end
~~~

**Diagram guard:** It shows separate **possible paths**, not a single mandatory sequence where a client must always receive a NOTIFY before BYE. Note RFC3265 NOTIFY Expires:0 is quoted as **Microsoft source wording**, not an independently proven RFC3265 wire semantic; precise Expires/Subscription-State and Reason compatibility requires actual literal RFC/MS-CONFBAS parser/errata audit before implementation.

## 3. Required error-state separation

| Category | Status/message | User-visible semantic boundary |
|---|---|---|
| C3P/Focus admission | SIP **400/403/404/603** | Invalid request, authorization denied, conference missing, meeting full |
| Conference roster | conference-info version/endpoint events | Participant or endpoint joined/left; must correlate IDs, cannot substitute user account |
| IM MCU modality | INVITE SDP accepted + `BYE` | One specific conference chat media session opened/closed |
| IM delivery | MESSAGE 202 + IMDN statuses 415/408/200 | MCU accepted versus per-recipient forwarding; no proof read |
| Typing | INFO 202 without downstream report | MCU accepted typing/idle; no delivery guarantee |
| Full removal | Focus deleteUser vs deleteConference vs FocusFactory deleteConference | Eject user vs stop conference vs permanently deprovision conference |

## 4. Open gaps

- **FCIM-U01:** Full [MS-CONFBAS] 248-page original §2.2.4 conference-info XSD, roster version deltas, subscription expiry, lobby admission, Focus command and all C3P faults **not exhaustively validated** here. The 18 findings are section-limited.
- **FCIM-U02:** §3.8.5.1 wording `NOTIFY with Expires=0` versus RFC3265 `Subscription-State: terminated;reason` modern semantics must be reconciled from original source. Do not invent a normative field rewrite.
- **FCIM-U03:** MPOP Focus user endpoint independence and IM MCU per-endpoint delivery/IMDN timing after mid-flight ejection remain indeterminate; match exact endpoint GRUUs and transaction identities.
- **FCIM-U04:** Historical 2015 MS-CONFIM **rev5.0** vs rev11.0 2025 and MS-CONFBAS corresponding historical releases not fully diffed; 2015 installed Windows Full Client policy cannot be inferred solely from newest spec.
- **FCIM-U05:** Security inherited from MS-SIPAE/Kerberos/NTLM/TLS, C3P delegation and MCU topology; no server-side shared secret, certificate bypass or media-key behavior assumed.
- **FCIM-U06:** Full client modality matrix and global source closure still incomplete, including outstanding MS-SIPREGE survivable contradiction and MS-SRTP/SRTP suite mismatch.

**Research-only, no new client code, no live-server/interoperability tests, no merge or release.**
