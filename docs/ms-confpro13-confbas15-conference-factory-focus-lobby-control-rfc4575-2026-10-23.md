# Conference scheduling and Focus join lifecycle: original MS-CONFPRO 13.0 ↔ MS-CONFBAS 15.0 ↔ RFC 4575/4579

**Research date:** 2026-10-23. **Status:** PARTIAL primary original clauses (conference creation, join, roster, C3P control) — **NOT** complete Microsoft Full Client conformance. No implementation or server testing.

**Primary source editions and identifiers:** [MS-CONFPRO] *Centralized Conference Control Protocol: Provisioning*, **rev13.0, v20250520, May 20 2025**, original PDF SHA256 `e3cf2e50c5e9a0f51225e54cc77009233e5b6579e3d9ff0048e0f6da8948de78`, official https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-CONFPRO/%5BMS-CONFPRO%5D.pdf ; reviewed §§2.2.1–2.2.3.8, 3.1.1–3.1.7, 3.2.1–3.2.6.1, and example §4.1. [MS-CONFBAS] *Centralized Conference Control Protocol: Basic Architecture and Signaling*, **rev15.0, v20250520**, SHA256 `8ecae43533f6a7fa391358078093aa52c852b3b66c0b975d1ec06a883c285dfa`, https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-CONFBAS/%5BMS-CONFBAS%5D.pdf ; reviewed §§2.2.1–2.2.4, 3.1–3.5.6 (focused on 3.2,3.4 and 3.5). Original archived PDF pages visually rendered for MS-CONFPRO 20/32/35/38, MS-CONFBAS 97/100/103. Direct IETF originals [RFC4575](https://www.rfc-editor.org/rfc/rfc4575) (Aug 2006, conference event package), [RFC4579](https://www.rfc-editor.org/rfc/rfc4579) (Aug 2006, conference control BCP119), [RFC3261](https://www.rfc-editor.org/rfc/rfc3261), historical [RFC3265](https://www.rfc-editor.org/rfc/rfc3265), [RFC4028](https://www.rfc-editor.org/rfc/rfc4028), [RFC2976](https://www.rfc-editor.org/rfc/rfc2976). Modern [RFC6665](https://www.rfc-editor.org/rfc/rfc6665) and [RFC6086](https://www.rfc-editor.org/rfc/rfc6086) supersede historical event/INFO editions but do **not** automatically replace Microsoft negotiated extensions.

Classification: **P** = source clause with scoped normative actor; **R** = RFC original; **S** = cross-document inferred state model; **U** = currently not verified. MUST and SHOULD retain actor/condition.

## A. Focus Factory: planning is not admission and not MCU media

| ID | Original exact clause | Documented semantics |
|---|---|---|
| CONF-P01 | CONFPRO §§2.2.2,3.1.3 | C3P provisioning request root `request` has `requestId` unique among same sender's outstanding requests (nonnegative integer), `from` **organizer URI**, `to` **To header SIP URI**, `C3PVersion=1`; exactly one operation child. Client MUST know **organizer SIP URI and Focus Factory URI** before issuing request. |
| CONF-P02 | CONFPRO §4.1, §2.2.2 | Creation uses **SIP `SERVICE`** with `application/cccp+xml`, Request-URI focusfactory service GRUU `opaque=app:conf:focusfactory`. It is **not** the Focus's participant INVITE, not HTTP POST, and not a media MCU offer. |
| CONF-P03 | CONFPRO §§3.1.1–3.1.2,3.1.7.1 | Client conceptually maintains outstanding-request queue keyed by `requestId`; per-request timer **SHOULD 32 s**. A timer exception/retry is not proof the conference was never created server-side; exact duplication/idempotency semantics remain U. |
| CONF-P04 | CONFPRO §§3.1.4.1,2.2.2.1 | Organizer MUST choose unique Conference-Id and issue `addConference`, await corresponding response. `conference-info` entity **empty**, optional state **full**, optional version **1**, `conference-id` **mandatory**; `conference-key` **deprecated and SHOULD NOT** be sent in new request (not proof all legacy participants omit it). |
| CONF-P05 | CONFPRO §3.1.4.1 | **Static** conference: query `getConferences` with `static=true` first; if already present, `getConference` for detail; otherwise `addConference` with static flag. Server limits **one static meeting per user**, as disclosed in error definitions. Static meeting ≠ newly created recurring independent Focus INVITE dialog. |
| CONF-P06 | CONFPRO §§3.1.4.2,3.2.5.3 | Organizer modification MUST fetch current conference and compare version; if local supplied version stale, stop and inform higher layer (possible multi-device race). Send `modifyConference` carrying current version. Server validates version and **increments** stored version on success. No blind unconditional overwrite. |
| CONF-P07 | CONFPRO §§3.1.4.3,3.2.5.4 | `deleteConference`: if active the Focus Factory communicates with Focus to **deactivate** it before removing table entry; static delete requires matching static flag. Deleting scheduling record and leaving one's SIP INVITE/MCU are distinct actions. |
| CONF-P08 | CONFPRO §§2.2.3,3.1.5.1–3.1.5.2 | Successful SIP **200** means Focus Factory processed SIP request; for 200 a **C3P response XML body MUST exist**. XML `response` echoes requestId; swaps from/to; `code=success|failure`, with exactly one operation result. **Logical XML failure and SIP transport status must both be inspected**. |
| CONF-P09 | CONFPRO §3.1.5.2 | Client SHOULD parse XML, validate operation, match/remove requestId in Outstanding Request Queue. Malformed/unknown/correlation failure ⇒ stop processing and report failed request. **An unmatched 200 body must not be treated as verified completion**. |
| CONF-P10 | CONFPRO §§2.2.3.1–3.2,3.2.5.2–3 | Failure `reason` includes `invalidVersion`, `conferenceExistsAlready`, `invalidAdmissionPolicy`, `anonymousUsersNotAllowed`, `federatedUsersNotAllowed`, `mcuTypeNotAvailable`, `invalidEncryptionKeyUsed`, `maxConferencesExceeded`, `maxStaticMeetingsExceeded`, `pstnLobbyBypassNotAllowed`, `pstnBridgeNotEnabled`; **each is a different policy/configuration/version error**, not SIP login error. |
| CONF-P11 | CONFPRO §3.2.5.2 | **Focus Factory server** authorizes organizer's admission policy, participants, available MCU types, notification/roaming/settings data limits, quota and Conference-Id. Failure ends processing with operation-specific `failure` body; not a universal SIP 403. Creation stores conference with **version 1**. |
| CONF-P12 | CONFPRO §§2.2.2.6,2.2.3.6 | `getEncryptionKey` can return **X.509 public certificate** encoded Base64 and `issuing-server` value for protecting **conference sensitive values from intermediate logging**. The public certificate contains **no private key**. The older `conference-key` in addConference is **deprecated**, so treat actual usage as client/server edition gated. |
| CONF-P13 | CONFPRO §3.2.6.1 | **Focus Factory** periodic expiration cleanup considers expiry plus implementation-specific buffer and removes entry only if **not active**. This server cleanup timer is not RFC4028 media/dialog session timer. |
| CONF-P14 | CONFPRO §§2.2.2.7–.8,3.1.4.8 | `getAvailableMcuTypes` and `getConferencingCapabilities` are separate discovery operations. Source says server-mode value no longer selects available MCU types (defaults to prior server mode 14); `getConferencingCapabilities` is guarded by product behavior notes. A client cannot infer AV/AS/IM MCU support solely from organizer auth. |

## B. Focus join, lobby, roster, INFO control (RFC4575/4579 interoperability)

| ID | Original exact clause | Client/Focus-scoped behavior |
|---|---|---|
| CONF-B01 | CONFBAS §§3.1.4–3.2.4.1.1 | Conference activation obtains MCU-Conference-URI for each MCU-Type; **the actual URI acquisition mechanism is implementation-specific** (§3.1.4.1.1). Client joins **Focus** via authenticated SIP **INVITE containing `addUser` C3P XML**. This is not the IM/AV/sharing MCU media INVITE. |
| CONF-B02 | CONFBAS §3.2.4.1.1 | If joining on behalf of another user, client **MUST** include authenticated `p-session-on-behalf-of` and SHOULD provide matching `session-on-behalf-of` in addUser XML; must not forge delegation by editing `From` string. `lobby-capable=true` is an optional client feature signal. |
| CONF-B03 | CONFBAS §§3.2.5.1–3.2.5.2.4 | On accepted Focus SIP 200 client **SHOULD parse the role returned**. Focus joining error table: **400** malformed headers/body; **403** conference policy denied; **404** conference not found; **603** meeting capacity exceeded. No universal 403=Kerberos failure assumption. |
| CONF-B04 | CONFBAS §3.2.5.2.2 | Focus SHOULD permit **multiple endpoints of the same user**; if permitted, each MUST appear under that user in roster and MUST be independently tracked; ending one Focus dialog MUST NOT disconnect the user's other endpoints. |
| CONF-B05 | CONFBAS §§3.4.3.1,3.4.4.1 | Client **MUST join Focus first**, then establish conference-event SUBSCRIBE (RFC4575 plus historical RFC3265). **Lobby clients receive limited roster** (no other participants or MCU information); when admitted they receive full notification. This is a privacy/authorization gate, not an empty conference. |
| CONF-B06 | CONFBAS §3.4.4.1 | Lobby client reads `lobby-capable`: true allows waiting; if absent/false client **SHOULD immediately terminate Focus and subscription**. Presence of lobby state is **not** proof that ordinary media MCU INVITEs may start. |
| CONF-B07 | CONFBAS §§3.4.2.1,3.4.5.1.1,3.4.6.1 | Client MAY start **32-second** first-full-notification timer after subscription. Upon first full notify **MUST stop timer**, process conference-info per **RFC4575 §3.7**, derive `conf-uris` table and **MUST extract own endpoint admission/lobby state**. On expiry **SHOULD fail subscription** and inform UI. |
| CONF-B08 | CONFBAS §3.4.5.2.1 | Focus **MUST immediately send full conference-info notification**; if `ms-piggyback-first-notify` supported, SHOULD put full document in **SUBSCRIBE 200 body**. Thus first roster may be 200 piggyback **or separate NOTIFY/BENOTIFY** according to negotiated extension. Do not assume a NOTIFY always follows SUBSCRIBE before UI can learn own state. |
| CONF-B09 | CONFBAS §§3.4.4.2–3.4.5.2.1 | Focus SHOULD aggregate independent MCU rosters with own state; publication of `conf-uris`, participants and `conference-view` depends on actual Focus/MCU activation. The roster is **not** proof that each participant already negotiated active media or that all MCUs have connected. |
| CONF-B10 | CONFBAS §§3.5.2.1,3.5.4.1.1 | **After valid Focus signaling dialog and MCU URI table**, client sends command C3P **inside SIP INFO** with `application/cccp+xml`, starts **32-second initial command timer**. A C3P `pending` response **SHOULD extend it by 180 seconds**. If no further answer before timeout, command **MUST** be considered timed-out. Not the same as RFC3261 Timer F/UM route/ICE. |
| CONF-B11 | CONFBAS §§3.5.4.2.1–2 | Focus **MUST check** that INFO belongs to known signaling dialog **not in lobby**. It SHOULD parse command then **202 Accepted** while processing later; conceptual role authorization: attendee is denied **conference-level** changes and third-party **user-level** changes. SIP 202 means queue accepted, not C3P success. |
| CONF-B12 | CONFBAS §§3.5.5.1.1–4 | Client on SIP **202** can wait for later command response via **Focus-originated INFO**; reply **200** to that incoming INFO, then match C3P **requestId** in PendingRequestTable. SIP **200 with response body** may complete inline. C3P `code=pending` extends timer; `success|failure` removes pending request; unmatched late response **SHOULD silently drop**. |
| CONF-B13 | CONFBAS §§3.5.6.1–2 | On unanswered command client SHOULD treat as failure; Focus processing timer emits `pending` as needed and then resets internal timer **28 seconds**, separately from client's 180-second wait. **Do not merge timers of both actors**. |
| CONF-B14 | CONFBAS §3.6 | `modifyConferenceLock` changes **global focus lock**, prohibiting new joins, while **MCUs may report their lock state independently**. UI should distinguish authoritative Focus global state and individual MCU state; local display is not a reason to presume an MCU was shut down. |

## C. Cross-role / cross-RFC concurrency and privacy model (S)

```mermaid
sequenceDiagram
 participant U as Organizer UAC
 participant FF as Focus Factory
 participant F as Conference Focus
 participant M as IM/AV/Sharing MCU
 U->>FF: SERVICE addConference requestId and conference-id
 FF-->>U: SIP 200 with C3P success/failure and conference URI
 U->>F: INVITE Focus with addUser C3P
 F-->>U: SIP 200 with role or 4xx/603
 U->>F: SUBSCRIBE conference event
 alt Piggyback extension selected
 F-->>U: 200 with full conference-info
 else Separate notification
 F-->>U: 200; then NOTIFY/BENOTIFY full roster
 end
 Note over U,F: Verify own admission vs lobby and available conf-uris
 opt Admitted and eligible
  U->>F: INFO C3P modifyUserRoles/lock/addUser/etc
  F-->>U: 202 Accepted
  F->>U: INFO with C3P pending or final result
  U-->>F: 200 SIP
 end
 opt Authorized MCU session
  U->>M: INVITE to advertised MCU URI, modality-specific SDP
  M-->>U: 200 or modality failure
 end
```

**Synthesis warning:** `SERVICE addConference` is not required before every **join existing meeting**. Organizer may schedule, participants join independently later; Focus activation may be separate. The arrows omit SIP intermediaries, TLS/SIPAE challenges and BYE; a response should only be considered authentic when relevant SIPAE MIC/transport guards pass. MCU invitations are per modality and may happen in concurrent order after actual roster/admission.

## D. Standards edition/security/conflict ledger

1. **CONF-SEC01:** Sensitive conference-key is **deprecated** for addConference (§2.2.2.1), but `getEncryptionKey` retains public X.509 model. This is a **version/capability-dependent** security provision, not a universal requirement to send conference key or private certificate to Focus Factory.
2. **CONF-SEC02:** Provisioning `requestId` (Focus Factory SERVICE) and control command `requestId` (Focus INFO dialog) have **separate correlation tables**. Do not merge them with SIP CSeq, REGISTER binding or MCU IMDN Message-Id. TLS and SIPAE authentication does not replace C3P policy-based authorization.
3. **CONF-SEC03:** RFC4575 describes a conference event package; Microsoft extends conference roster, and **MS-CONFBAS requires Focus join before SUBSCRIBE**. Do not implement generic RFC4575 subscribe-outside-conference semantics without checking this Microsoft extension.
4. **CONF-SEC04:** RFC2976/6086 `INFO` general SIP method does **not** define Microsoft C3P `202→separate INFO response` semantics. Also distinguish MCU IM typing INFO and UM server notification INFO: same SIP verb, *different application protocol*.
5. **CONF-SEC05:** Lobby cannot see participant/MCU roster data or send control. `lobby-capable` determines waiting behavior, not the ability to evade meeting ACL, role authorization or conference lock. The exact lobby-to-admitted transition by Focus notification and control rights should be separately tracked.
6. **CONF-SEC06:** Focus/MCU policy, endpoint independence and role distribution imply more than one state machine; a successful Focus INVITE may precede failure of IM MCU, AV MCU, AS MCU, ICE, SRTP or presenter lock. Do not use single `conferenceConnected=true` flag in research model.
7. **CONF-SEC07:** MS-CONFPRO `modifyConference` version is optimistic concurrency, different from MS-PRES category/roster versions. Conflict may arise from two devices even when SIP security association and registration are valid.
8. **CONF-SEC08:** Historical RFC3265 event edition vs RFC6665 successor and optional BENOTIFY/piggyback first notify remain version-gated; modern normative update does not imply all legacy Windows deployments adopt modern behavior.
9. **CONF-OPEN09:** Full C3P XSD request/response schema and operation-specific failure codes of modifyConferenceLock/deleteUser/setLobbyAccess/modifyEndpoint/modifyConference, focus status policy, anonymous access and PSTN conference capabilities need more primary original clause review.
10. **CONF-OPEN10:** Focus Factory `GetConferencingCapabilities` and conference-creation option `server-mode` product guards; current Windows target release and server capability negotiation not yet proven. The actual Focus Factory/proxy discovery URL/GRUU requires initial provisioning evidence.

**The technical behavior of the entire classic Windows Full Client remains NOT COMPLETE. No protocol implementations, live-server/interop tests, merge or release.**