# MS-PRES 13.0 — enhanced presence publication, privacy container selection, subscription and endpoint cleanup

**Research audit: 2026-10-23. Not full specification conformance; no code or live tests.** Primary Microsoft original **[MS-PRES] Presence Protocol revision 13.0, v20250520, 2025-05-20**, SHA256 `f1e0ce966ab242cf09b57e4403f9cfa7aec9c5aa883325f1e4ac91ad1ff5fa79`; original PDF: https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-PRES/%5BMS-PRES%5D.pdf . Original full-text sections examined: §§2.2.2.1–2.2.2.6,3.2.5.1.2–3.2.6.2,3.3.5.1–3.3.5.3, with portions of §3.8 still to audit. PDF pp91,94,97 **rendered** to compare authoritative pagination/field language. Compare [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) (SIP2002), historical [RFC3265](https://www.rfc-editor.org/rfc/rfc3265) (events2002, since obsoleted by [RFC6665](https://www.rfc-editor.org/rfc/rfc6665),2012) and [RFC4662](https://www.rfc-editor.org/rfc/rfc4662) (resource lists2006), other Microsoft **MS-SIP rev8 2017-06-01, MS-SIPREGE rev17.1 2025-08-19, MS-SIPRE, MS-OCER rev11 2025-05-20**. Microsoft first-party counterpart: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/6d7de3b4-abdb-4a02-854c-835cccfa6c1b .

Evidence keys: **P** directly inspected Microsoft source actor/condition; **R** explicit historical RFC; **S** cross-source inference; **U** unresolved client-build applicability. Every MUST/SHOULD/MAY retains its original role. Source publication 2025 is not proof every classic 2015 Windows client supports every category and aggregation type.

## I. Publication requests: XML, optimistic versions, atomic service transaction

| ID | MS-PRES original | Direct role-scoped rule |
|---|---|---|
| PRES-01 | §2.2.2.1 | A `categories` XML document has required publisher `uri`; `category` identifies `name`, optional instance (default **0**), container ID, version, publishTime and expireType. **Category `container`/ `expireType` MUST appear in self-subscribe/publication results but MUST NOT appear in category subscriber notification**. This is an intentional privacy distinction, not an XML parser fault. |
| PRES-02 | §2.2.2.1 | Expiry types: **static** until deletion; **endpoint** as long as a particular registered endpoint; **user** until *last* endpoint deregisters; **time** until specified expiry. For endpoint-scoped category `endpointId` UUID derives from **+sip.instance** Contact parameter and MUST be present; time-scoped `expires` required. Publication deletion uses `expires=0`. |
| PRES-03 | §§2.2.2.2.1,3.2.5.1.2 | Client `SERVICE` publication uses **`Content-Type: application/msrtc-category-publish+xml`**, `<publish><publications uri=...><publication categoryName=... instance=... container=... version=... expireType=...>` payload. Server **MUST compare** SIP From URI, To URI and publications URI and must validate XML + schemas before mutation. |
| PRES-04 | §3.2.5.1.2 | For **new category instance** client-supplied version MUST equal **0**; for existing instance requested version MUST equal server's current version. On successful check version increments; server sets `publishTime` UTC. Do not use generic global roster `deltaNum` as publication version. |
| PRES-05 | §3.2.5.1.2 | Processing category mutations is **transactional for the entire `SERVICE` batch**: the server MUST NOT commit before all publication elements processed and **cancels all pending changes if any fails**. After success responds **200** with **`application/vnd-microsoft-roaming-self+xml`** and updated `categories` with current version and publishTime. |
| PRES-06 | §§2.2.2.2.3,3.2.5.4 | On stale category version server **MUST 409 Conflict for whole batch**, with `application/msrtc-fault+xml` containing `Faultcode`, details and current version (`curVersion`) for affected operation(s). Source **§4.2.1** illustrates **MS-OCER ErrorId 2044**. A 409 is not a partial success; no publish notification follows failed batch. |
| PRES-07 | §3.2.5.4 | Error mapping: missing/invalid XML **400**, URI permission mismatch **403** (From/To), To/body URI mismatch **400**; missing `time` expires **400**; read-only or unregistered category **403**; same instance duplicated in batch **400**; size/category quota **413 SHOULD**; endpoint-expire category when publisher endpoint not registered **488 MUST**. SIP 488 is not a media codec-only error in this Microsoft extension. |
| PRES-08 | §3.2.6.2 | Server publishes specific predefined categories on its own periodic publisher timer, e.g. directory-derived `contactCard` to containers **0,100,200,300,400,32000**, `legacyInterop` offline class 18500 to container **32000**, and **read-only `userProperties` in container 1**. Do **not** treat this as the client making an unauthorized self-write or every server-generated property as user input. |
| PRES-09 | §3.2.5.6 | Altered `state` and `services` instances in containers **2 and 3** trigger presence aggregation per **§3.8**. This source audit has **NOT** covered §3.8's full aggregation algorithm; no invented aggregate availability arithmetic is justified. |

## II. Privacy containers: resolution is category-scoped, subscriber-specific and ordered

| ID | Original | P rule |
|---|---|---|
| PRES-10 | §3.2.5.3 | For **each subscribed category of each watcher**, server reevaluates which container controls visibility. It considers only containers containing the **requested category**, then selects by an ordered precedence: **exact user URI** member, **domain** member, **sameEnterprise**, **federated**, **publicCloud**, **default**. Within exact user/domain matching groups select **highest-numbered container**; this does NOT mean pick globally highest container regardless of group. |
| PRES-11 | §3.2.5.3 | If no eligible category container exists, selected publication becomes **null**, and notifier adds an **empty data category** to represent change. A watcher is not necessarily permitted to see all category instances the publisher owns. |
| PRES-12 | §3.2.5.3 | External category subscription NOTIFY/BENOTIFY **MUST use `application/msrtc-event-categories+xml`**; each `category` exposes `name`, `instance`, `publishTime`, body XML but **MUST NOT disclose** container ID, server version, expiration mode, endpointId or expiry. This prevents crossing viewer/publication metadata trust boundary. |
| PRES-13 | §§3.2.5.2,3.2.5.3 | On valid publish, self subscribers within scope MUST receive NOTIFY/BENOTIFY **including publisher endpoint itself**; external subscribers receive updates only if resolved category instance/visibility changed. Visibility change can be notified with empty body; exact watcher order is not a prescribed synchronization barrier. |
| PRES-14 | §§3.2.5.3,3.2.5.5 | Since server may re-evaluate member permissions and category instances, a user can see a contact become less informative without contact removal or SIP registration loss. **S:** client UI must not treat hidden status as confirmed logged-out user (MS-OCER 2004 unauthorized subscription). |

## III. Own-state SUBSCRIBE and multiple endpoints

| ID | Source | P clause |
|---|---|---|
| PRES-15 | §§2.2.2.3.1,3.3.5.1 | After registration self-SUBSCRIBE uses **`Event: vnd-microsoft-roaming-self`**, `Accept` and `Content-Type: application/vnd-microsoft-roaming-self+xml`, body `roamingList`, with options **categories, containers, subscribers, delegates**; SIP From and To **MUST be identical**. A valid Contact supports RFC3261 dialog routing. |
| PRES-16 | §3.3.5.1 | A new SUBSCRIBE within an **existing dialog** replaces **entire roamingList scope**, rather than appending. E.g. refresh requesting only subscribers removes categories from that same dialog scope. Client endpoints usually ask for all four, server SHOULD limit to **one active self dialog per endpoint**, terminating previous with NOTIFY Expires=0. |
| PRES-17 | §3.3.5.1 | Server MUST return all in-scope roaming data **even on refresh SUBSCRIBE**, and successful request returns **200**. In survivable mode server **MAY not create** the dialog despite creation request. A **polling self SUBSCRIBE MUST NOT store SelfSubscriptionDialogEntry** on server. These create/refresh/poll variants need separate state. |
| PRES-18 | §3.3.5.2 | Self-state changes (including changes made by **another endpoint**) cause NOTIFY to every matching self dialog. Notification is sent to **originating endpoint too** when it has a dialog; server may route over registrar route set instead of dialog set where both are on same server. This is shared user-state sync but **not** a guarantee all endpoints have identical device-local `endpoint` publications. |
| PRES-19 | §3.3.5.3 | Invalid roaming-self XML or missing body **400**; if From/To differ, unconfigured target **404** or configured target **400**; deregistered subscription-initiating endpoint **401 SHOULD**. Source does **not** specify behavior for Request-URI differing from From/To — **explicit gap**, not authorization to substitute generic RFC3261 rule without higher spec. |

## IV. Expiry and sign-out cleanup are strictly scoped

| ID | Source | P behavior |
|---|---|---|
| PRES-20 | §3.2.5.5 | Deregister/expire *one endpoint* ⇒ server MUST delete its `expireType=endpoint` category instances identified by endpointId and notify affected self/category subscribers. Do not delete other active devices' `user` publications merely because one endpoint lost TCP. |
| PRES-21 | §3.2.5.5.1 | Only when **last endpoint** deregisters/registration expires ⇒ remove `expireType=user` **as well as** that endpoint's `endpoint` categories and notify subscribers. `static` categories persist unless removed by their own rules. |
| PRES-22 | §3.2.6.1 | Server's periodic publication cleanup removes expired `expireType=time` categories and generates subscriptions updates. It is **not** identical to REGISTER refresh, SIPAE 8-hour SA renewals or a GUI contact refresh timer. |

## V. Synthesis: a per-category, per-endpoint, per-watcher state graph

~~~mermaid
flowchart TD
 A[Authenticated registered endpoint and SIP URI] --> B[Self SUBSCRIBE categories containers subscribers delegates]
 A --> C[SERVICE publish category name instance container version]
 C --> D{Server XML and optimistic version valid for entire batch?}
 D -->|No| E[400 403 409 413 or 488; no partial commit]
 D -->|Yes| F[200 roaming-self updated versions and publishTime]
 F --> G[NOTIFY/BENOTIFY all matching self endpoints]
 F --> H[Reevaluate every subscriber per category and ordered privacy container]
 H --> I[Category NOTIFY/BENOTIFY: no container, version or expiry metadata]
 A --> J{Endpoint expires?}
 J -->|One endpoint| K[Delete only matching endpoint-scoped publications]
 J -->|Last endpoint| L[Delete endpoint and user-scoped publications]
 K --> G
 L --> G
~~~

**Synthesis-only, not an observed packet transcript.** Self-SUBSCRIBE subscription scope is independent of subscriptions to other users; REST, directory ABS, distributed AD property, SIP roster and Presence-State REGISTER all have independent version/role semantics. Normal categories/containers/subscribers/delegates are sensitive and MUST only be exposed according to first-party rules.

## VI. Explicit audit blockers

- **PRES-X01**: server `§3.8` aggregation algorithm and product-specific precedence/weights for state/services **not yet fully inspected**. Do not invent unified status priority.
- **PRES-X02**: all containers/permission XML §2.2.2.5 and delegate permissions §2.2.2.10, exact `setContainerMember`/subscriber version counters 2045/2047, and error batch atomicity per operation are not yet exhaustively audited.
- **PRES-X03**: original [RFC3265] (2002) events vs later RFC6665 (2012) adjustments, [MS-SIP] BENOTIFY and piggyback/autoextend are conditional; do not silently adopt all modern NOTIFY sequencing into historic Microsoft profile.
- **PRES-X04**: exact “last endpoint” semantics for user publications after proxy dead-connection timeout depend on **MS-CONMGMT registrar** and **MS-SIPREGE** branch; dead endpoint may receive **no deregister NOTIFY**, yet other subscriber state must update.
- **PRES-X05**: §3.3.5.1 survivable self-SUBSCRIBE creation MAY fail and [MS-SIPREGE] rev17.1 §3.1.1.5.1 contains contradictory survivable entry/exit and self-subscription re-register vs retry SHOULD; **BLOCKED** until official errata.
- **PRES-X06**: all 257-page original's legacy vs enhanced presence and 2015 Windows build capability notes have not been audited; no universal requirement that every possible category is produced by client.
- **PRES-X07**: watchers external to enterprise must not learn private container/version/endpoint metadata. Whether all in-transit SIP messages were encrypted/signature-validated under legacy topology depends MS-CONMGMT and SIPAE, not this XML schema alone.

**Global Windows full-client research remains INCOMPLETE; no protocol code, live servers or own tests, no merge/main/release.**
