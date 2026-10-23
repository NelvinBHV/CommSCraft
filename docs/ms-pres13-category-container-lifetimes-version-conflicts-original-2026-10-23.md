# MS-PRES 13.0 — Enhanced Presence publication, versioning, privacy containers and roaming subscriptions

**Research tranche 2026-10-23. Status: SELECTED PRIMARY CLAUSES VERIFIED; full document (257 PDF pages), client applicability and all schemas NOT YET COMPLETED. No protocol implementation or tests.**

**Original:** Microsoft **[MS-PRES] Presence Protocol**, revision **13.0**, edition **v20250520**, **2025-05-20**, SHA-256 `f1e0ce966ab242cf09b57e4403f9cfa7aec9c5aa883325f1e4ac91ad1ff5fa79`, original PDF https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-PRES/%5BMS-PRES%5D.pdf (original bytes verified against archived inventory; visually rendered/inspected PDF pp34,35,90,93). Clauses reviewed: §§2.2.2.1–2.2.2.7.13,3.2.5.1.2,3.2.5.2–3.2.5.6,3.3.5.1–3.3.5.3,3.4.5.1–3.4.5.3,3.5.5.2,3.6.5.1 and representative §4 examples. References for contextual reconstruction: [MS-SIP] rev8.0 (2017-06-01) batched NOTIFY/BENOTIFY, [MS-SIPREGE] rev17.1 (2025-08-19) REGISTER/GRUU/+sip.instance, [MS-OCER] rev11.0 diagnostic and version ErrorIDs, [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) (2002) SIP dialogs, [RFC3265](https://www.rfc-editor.org/rfc/rfc3265) (2002) event subscription; RFC3265 was subsequently superseded by [RFC6665](https://www.rfc-editor.org/rfc/rfc6665) (2012). **Versioned original citation is not proof all 2025 rules shipped identically in every 2015/2016 Windows build.**

**Evidence labels:** P = directly prescribed for specified actor and condition in cited source, R = historical RFC rule, S = cross-protocol synthesis, U = unverified client build/unsupported branch.

## A. Publication wire syntax and atomic version transaction

| ID | Original clause | Strict source-bound requirement |
|---|---|---|
| PRE-01 | §2.2.2.2.1 | Publisher sends SIP **SERVICE** with `Content-Type: application/msrtc-category-publish+xml`. **To URI, From URI and `<publications uri>` MUST be identical publisher SIP URI**. The XML must conform to source §6.10 schema. This is not SIP PUBLISH from generic RFC3903. |
| PRE-02 | §2.2.2.2.1 | `<publication>` required **categoryName, instance (unsigned int), version (unsigned int), container (unsigned short), expireType**; **instance is scoped by category and container**, not a universal publication ID. |
| PRE-03 | §§2.2.2.1,2.2.2.2.1 | `expireType` values **static, endpoint, user, time**; when time, `expires` UTC required. **`expires=0` unpublishes**; nonzero expires MUST NOT be set except time-bound entry. Don't infer a universal SIP Expires header from this attribute. |
| PRE-04 | §3.2.5.1.2 | Server validates schema and matching actor identity, must disallow read-only container/category combinations, MAY reject unknown category names according to administrative category registry. **Endpoint-bounded publication requires an already registered endpoint**; otherwise source §3.2.5.4 requires SIP **488**. |
| PRE-05 | §3.2.5.1.2 | **Optimistic version check:** existing category `version` MUST equal submitted `version`; if instance absent, submitted version MUST be **0**. Successful operation increments stored version. Multiple instances can each have independent versions. |
| PRE-06 | §3.2.5.1.2 | Server sets `publishTime` to current UTC. A zero expiry deletes, a nonzero expiry updates/creates. **MUST NOT commit any changes until every publication element processed** and MUST roll back all changes if any operation fails. |
| PRE-07 | §§2.2.2.2.2,3.2.5.1.2 | Successful SERVICE responds **200 OK** `Content-Type: application/vnd-microsoft-roaming-self+xml`, carrying a **`categories`** document with authoritative per-instance version/publishTime. A 200 is server transaction acceptance, not proof an external watcher received notification. |
| PRE-08 | §§2.2.2.2.3,3.2.5.4 | Version mismatch => **409 Conflict** and `Content-Type: application/msrtc-fault+xml`, faultcode **`Protocol client.BadCall.WrongDelta`**, with operation **index, submitted version and curVersion**. **All batched publication elements fail together** even if only one mismatches. Retries SHOULD use server `curVersion`, but **not blindly overwrite stale local data** without user/merge policy. |
| PRE-09 | §3.2.5.4 | Distinct server errors: **400** missing/malformed XML, URI/body mismatch, missing time expiry or duplicate category instance within same container; **403** From/To mismatch, read-only container or unregistered category; **409** version conflict; **413 SHOULD** quota violation; **488** endpoint-bound publication without registration. Do not handle all 4xx as generic sign-out. |
| PRE-10 | §§3.2.5.2–3.2.5.6 | Committed category change can generate **self NOTIFY/BENOTIFY** and **category watcher NOTIFY/BENOTIFY**; visibility is filtered by category/container subscriptions. Aggregate `state`/`services` category handling is a **separate server operation** (§3.8), not just copying one endpoint's availability label. |

## B. Self SUBSCRIBE, multi-endpoint sync and authorization

| ID | Clause | Mandatory/conditional semantics |
|---|---|---|
| PRE-11 | §2.2.2.3.1 | Self SUBSCRIBE: SIP `Event: vnd-microsoft-roaming-self`, **`Accept` and `Content-Type` = application/vnd-microsoft-roaming-self+xml**, self To/From URI identity, valid Contact for SIP dialog target. XML `<roamingList>` selects **categories, containers, subscribers, delegates**. |
| PRE-12 | §§2.2.2.3.2,3.3.5.1 | Successful 200 piggyback or following NOTIFY includes `<roamingData>` with potentially four independent XML namespaces/subsections. The source says self-subscribing after **REGISTER** is a sign-in step and cross-endpoint synchronization channel; roster sync and media readiness are not synonymous. |
| PRE-13 | §3.3.5.1 | Sending a **new self SUBSCRIBE within an existing dialog REPLACES the server's roaming type scope**, not union/addition. Example: current categories; refresh only subscribers => resulting scope **subscribers**, not both. **For a refresh, server MUST return all data in newly selected scope.** |
| PRE-14 | §3.3.5.1 | Each endpoint creates **one** self SUBSCRIBE dialog with all four types under usual profile; server SHOULD enforce one per user endpoint by expiring previous dialog with NOTIFY expires 0. **Polling self SUBSCRIBE MUST NOT be stored as a durable SelfSubscriptionDialogEntry.** |
| PRE-15 | §3.3.5.1 | **Survivable Mode** can cause server to **not create** the new self-subscription dialog (MAY). The unresolved MS-SIPREGE §3.1.1.5.1 **self-sub failure/deregister-versus-retry + false-value polarity conflict** remains independent and BLOCKS unconditional recovery logic. |
| PRE-16 | §§2.2.2.3.3,3.2.5.2,3.3.5.2 | When category/container/subscriber/delegate state changes at another endpoint, server emits **roaming-self NOTIFY or BENOTIFY** with only affected subsections as needed. **An unchanged subsection being absent in one event does not mean its cache should be cleared.** |
| PRE-17 | §3.3.5.3 | Self subscription fails **400** invalid/missing roaming-self XML, **400/404** depending invalid From/To user identity, and **401 SHOULD** if endpoint no longer registered. Behavior with Request-URI mismatch against From/To is **unspecified**; do not invent a fixed status. |

## C. Container privacy, subscribers and category-watcher lifetime

| ID | Clause | Verified scope/response |
|---|---|---|
| PRE-18 | §§2.2.2.5.1–3.5.5.2 | `SERVICE Content-Type: application/msrtc-setcontainermembers+xml` changes membership with `<container id version>` and `<member action="add|delete" type="user|domain|sameEnterprise|federated|publicCloud|everyone" value?>`. **Container ID 0 MUST NOT be modified.** |
| PRE-19 | §§2.2.2.5.3,3.5.5.2 | Container membership has a **separate version**, and stale client update receives **409 `application/msrtc-fault+xml`** with submitted/current versions. A category publication version is **not** the container membership version. |
| PRE-20 | §2.2.2.6.2 | Subscriber acknowledgement: `SERVICE Content-Type: application/msrtc-presence-setsubscriber+xml` with **one** `subscriber user acknowledged="true"`; server **MUST NOT support batching multiple subscribers** in one setSubscribers request. |
| PRE-21 | §3.6.5.1 | Server acknowledges matching SubscriberEntry or **400** if absent; on success may **SHOULD remove** entry for supported category subscriptions. **MSRTC/PIDF subscription entries MUST NOT be removed** on this path because their context semantics differ. Quotas can cause **413 SHOULD**. |
| PRE-22 | §§3.4.5.1–3.4.5.3 | Category watcher SUBSCRIBE supports single/batched `application/msrtc-adrl-categorylist+xml` of targeted publishers/categories, distinct from self roaming. A missing resource for a **single** subscribe => **404**, but for **batched** category subscription server sends per-resource rejection/resubscribe hints, **not unconditional whole-batch 404**. |
| PRE-23 | §3.4.5.2 | Category `NOTIFY` version MUST be **0 on first**, and subsequent versions MAY increment by one or remain same within that dialog. This is **notification sequence** not the per-category publication `version`. Source permits using a **registration route set** when subscribed and registered on same server; NOTIFY route need not be identical to SUBSCRIBE route in that case. |
| PRE-24 | §§3.2.5.5–3.2.5.5.1 | Deregister/registration expiry of one endpoint removes **its endpoint-bounded** publications (endpointId from `+sip.instance` UUID). When **last endpoint** unregisters, server also removes **user-bounded** publications; affected watchers notified. **Static publications do not have the same implicit cleanup rule.** |

## D. Partial-order state graph (S synthesis; NOT a captured Windows Full Client trace)

~~~mermaid
flowchart TD
 A[Authenticated REGISTER +sip.instance binding] --> B[Self SUBSCRIBE roaming categories/containers/subscribers/delegates]
 B --> C[Receive snapshot with publication and container versions]
 C --> D[SERVICE batch publish with exact publisher identity and category versions]
 D --> E{All schema and version checks pass?}
 E -->|No| F[400/403/409/413/488; no partial publication commit]
 F --> G[On 409 reconcile current versions and data before policy-governed retry]
 E -->|Yes| H[200 roaming-self categories authoritative versions]
 H --> I[Server sends independent self and watcher NOTIFY/BENOTIFY]
 I --> J[Other user endpoints update caches subject to privacy containers]
 J --> K{Endpoint loss}
 K -->|One endpoint| L[Delete only its endpoint-expiring category instances]
 K -->|Last endpoint| M[Also delete user-bounded instances; notify]
~~~

**Critical non-equivalence:** User A presence publisher's **category publication version** ≠ privacy container membership version ≠ roster contact `deltaNum` ≠ NOTIFY per-dialog sequence ≠ SIP CSeq ≠ REGISTER Expires. Merging counters would cause unauthorized visibility or out-of-order state corruption.

## E. Named gaps and edition conflicts

- **PRE-X01:** Full **§6 XSDs**, type bounds, category schema bodies (state/device/services/userProperties/contactCard/calendarData/workingHours/delegates/chat) and aggregation **§3.8** not yet exhaustively cross-verified. Must complete before implementation.
- **PRE-X02:** Server-only policy decisions, ACL ordering when user/domain/publicCloud membership overlaps, read-only category names and privacy filtering on federation boundary need §3.5–3.10 audit plus concrete Windows user privacy UI. Never infer contact list membership automatically grants all presence categories.
- **PRE-X03:** MS-PRES original rev13 2025 cites historical RFC3265 (superseded by RFC6665); exact NOTIFY/BENOTIFY semantics depend MS-SIP 2017 and client product support. No modern RFC upgrade assumed.
- **PRE-X04:** MPOP race: server snapshot then overlapping self notification, version reconciliation, endpoint loss on one registration while remaining endpoint still active, branch out-of-order notifications. Source only prescribes selected parts; full deterministic client algorithm U.
- **PRE-X05:** RFC3903 SIP `PUBLISH` is NOT Microsoft `SERVICE` publication merely because the concept "publish" overlaps.
- **PRE-X06:** The existing documented MS-SIPREGE rev17.1 survivable-mode **entry/exit false contradiction** is still blocked; MS-PRES provision §3.3.5.1 adds only that server MAY decline persistent self dialog in survivable.
- **PRE-X07:** MS-OCER 11 ErrorIds **2044 publication / 2045 container / 2046 contact / 2047 permission** apply different version domains, not interchangeable `409` handling.

**Full Classic Windows Full Client documentation NOT COMPLETE. Only research files changed; no server calls, no tests, no protocol implementation, no merges/releases.**
