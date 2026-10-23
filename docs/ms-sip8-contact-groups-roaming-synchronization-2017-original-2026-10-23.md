# MS-SIP rev8.0 — Roaming contacts/groups, ACL, delta synchronization and batched presence

**Original specification edition:** Microsoft **[MS-SIP] Session Initiation Protocol Extensions** revision **8.0**, **v20170601**, **2017-06-01**, PDF SHA-256 `6ee1c16c922b7c154373572d5a3341139aaf6d91922a35468581a209f68e9e52`, official PDF https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-SIP/%5BMS-SIP%5D.pdf . Original file/hash verified; PDF pages **17,18,33** directly rendered and checked. Reviewed §§2.2.1–2.2.4.2,3.3.5.1–3.3.5.2,3.6.2–3.6.5.2,3.7–3.7.7, selected §4 examples and contact/ACL schemas in §§8–9. Historic normative [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) 2002 and [RFC3265](https://www.rfc-editor.org/rfc/rfc3265) 2002 (later superseded by RFC6665); Microsoft dependencies [MS-SIPAE] rev11, [MS-SIPREGE] rev17.1, [MS-PRES] rev13, [MS-OCER] rev11. **This is an older 2017 edition**, not a license to merge MS-PRES 2025 rule changes silently. Selected clauses verified; full 93-page detailed applicability remains OPEN. P direct section; S synthesis; U unknown client build/implementation.

## 1. Contact/group wire data and server-version handling

| ID | Original source | Source-bound protocol rule |
|---|---|---|
| SIPC-01 | §2.2.4.1 | `application/vnd-microsoft-roaming-contacts+xml` **full `contactList`** contains `group` records (integer id, name, externalURI) and `contact` records (uri, name, **space-separated group IDs**, subscribed boolean, externalURI and optional XML contactExtension). |
| SIPC-02 | §§2.2.4.1,3.7 | `contactDelta` uses **addedGroup, modifiedGroup, deletedGroup, addedContact, modifiedContact, deletedContact** and **required `prevDeltaNum` and `deltaNum`**. These are user contact/group-list versions, **not presence category versions**. |
| SIPC-03 | §3.7 | Server maintains a **single nonnegative contact/group version per user**, incremented by one on modification. Client must reconcile against the authoritative version from `contactList deltaNum`, not just locally infer from successful outgoing SERVICE. |
| SIPC-04 | §3.7 | **Initial** roaming-contact subscription result is the **full list**; subsequent notifications contain **deltas**. Transport can be **200 OK piggybacked notification, NOTIFY, or BENOTIFY**; first full snapshot and per-event deltas have different processing semantics. |
| SIPC-05 | §3.7.4.1 | Client **SUBSCRIBE Event: vnd-microsoft-roaming-contacts**, `Accept: application/vnd-microsoft-roaming-contacts+xml`, own SIP URI in request/to/from. Note section heading of 2017 source uses singular name in one prose sentence, but **on-wire Event** in its example is plural `vnd-microsoft-roaming-contacts`; use exact original grammar, not guess. |
| SIPC-06 | §2.2.4.2 | Mutating **SIP SERVICE** SOAP/XML primitives: `setContact`, `deleteContact`, `addGroup/modifyGroup`, `deleteGroup`. Every primitive must carry **the current `deltaNum`** (optimistic version). URI of owner in **To/From/Request-URI**, not URI of the newly added contact. |
| SIPC-07 | §2.2.4.2 | `setContact`: uri/displayName/group IDs/subscribed plus optional externalURI/contactExtension. `subscribed=false` is a valid **offline contact** and does not mean deleting it; contact roster membership is distinct from an active presence SUBSCRIBE. |
| SIPC-08 | §3.7.4.4 | Historic group operations use integer IDs **1–63**; group **#1** default, created automatically by server, **MUST NOT be created/deleted by user**; each contact belongs to ≥1 group. Before deleting group all contacts must be removed from it using specified operations. Client cache cannot blindly assume names/IDs stable across full-list re-notification, which §3.7 says may renumber groups. |
| SIPC-09 | §3.7.4.3–4 | Server **200 OK** confirms SOAP SERVICE contact/group mutation. Cross-endpoint change propagation and roster cache reconciliation flow via separate event subscription; a 200 is not equivalent to all endpoints rendering the new group. |
| SIPC-10 | §§3.7.1,3.7.4.2,3.7.5.3 | User ACL and contact/group list have **separate nonnegative sequence numbers**. Every device **SHOULD subscribe** `Event: vnd-microsoft-roaming-ACL`, `Accept: application/vnd-microsoft-roaming-acls+xml`; store full initial ACL and apply later ACL changes. **ACL deltaNum ≠ contact deltaNum**. |
| SIPC-11 | §3.7.5.1 | Legacy ACE entries are two-character right strings for user/domain/everyone. **Server MUST enforce presence/SUBSCRIBE portion**, **client MUST enforce session-invitation/INVITE portion** of legacy ACL, according to this historic profile. Cross-check richer MS-PRES enhanced presence containers before carrying ACL rule to current deployments. |
| SIPC-12 | §3.7 | **Contact management extension is optional** in the documented SIP profile. The overview lists classic UI behavior, but a protocol being optional does not prove every historical or modern server supports every mutation. |
| SIPC-13 | §3.7.3 | Client **SHOULD REGISTER** before roaming contact retrieval/management; still, REGISTER binding expiry/refresh and roster SUBSCRIBE expiry have distinct clocks and failure recovery. |

## 2. Batched category vs legacy batched presence

| ID | Source | Rule and version boundary |
|---|---|---|
| SIPC-14 | §§3.3.5.1–2 | Batched `SUBSCRIBE` over presence can return **multipart/related** with `application/rlmi+xml` resource list, per-resource CID and **`text/xml+msrtc.pidf`** body in legacy presence model. Enhanced MS-PRES §3.4 may instead carry `application/msrtc-event-categories+xml` category information; **not identical media grammars**. |
| SIPC-15 | §3.3.5.2 | If server doesn't support batched SUBSCRIBE, it sends failure; client **MAY disable BatchSubscribeEnabled and fall back to individual SUBSCRIBE**. This is not a protocol MUST to retry every failed subscription or use infinite retries. |
| SIPC-16 | §§3.6.4.1–3.6.5.2 | Auto-extension negotiated with **`Supported: com.microsoft.autoextend`** on SUBSCRIBE and server response. Client MAY set AutoExtendSubscriptionExpireTimerEnabled true only if server returned support, SHOULD reset subscription expiry timer on subsequent NOTIFY; without support, normal RFC3265 refresh remains. |
| SIPC-17 | §§3.6.2,3.7.2 | Subscription expiry uses separate **SubscriptionExpireTimer**, refresh SHOULD be before expiry. MS-SIP contact management specifies **no additional timers** beyond the SIP/RFC3265 subscription model: no invented periodic 30-second full roster re-download. |
| SIPC-18 | §§3.4,3.6 and MS-PRES §§2.2.2.3.2–3 | **Piggyback first NOTIFY** in 200 OK, regular NOTIFY and optional BENOTIFY reflect **distinct negotiated capabilities**. Do not assume every 200 OK body is contact list rather than self categories or another event package; check `Event` and `Content-Type`. |

## 3. Full vs delta, privacy and watch availability model (S synthesis)

~~~mermaid
flowchart TD
 A[REGISTER/SA established; roster feature provisioned] --> B[SUBSCRIBE vnd-microsoft-roaming-contacts]
 B --> C[Initial full contactList + authoritative deltaNum]
 C --> D[Local immutable snapshot indexed by SIP URI]
 D --> E{Event source}
 E -->|Local update| F[SERVICE setContact/deleteContact/group with current deltaNum]
 F --> G{Server 200 and new event?}
 G -->|200| H[Continue version reconciliation through subscribed roster]
 G -->|Version rejection| I[Refresh authoritative snapshot; merge policy unresolved]
 E -->|Remote endpoint update| J[NOTIFY/BENOTIFY contactDelta prevDeltaNum -> deltaNum]
 J --> K{prevDeltaNum matches local?}
 K -->|Yes| L[Apply six delta operations; advance deltaNum]
 K -->|No| M[Mark roster stale and request server snapshot; exact retry rule to source]
 D --> N[Separate subscription for own ACL and per-contact category presence]
 N --> O[Presence categories filtered by containers/authorization]
~~~

**S caveat:** `prevDeltaNum` is an explicit source field, but exact client conflict-resolution behavior on skipped delta/out-of-order notifications, server 409 XML and user edits requires full §3.7 message examples and version-specific registrar/OCER integration. The mismatch→refresh node is a **safety-oriented derived requirement**, not a quoted MS-SIP MUST. Multiple endpoints can edit groups concurrently; no single “last change wins” assumed.

## 4. Error, privacy and applicability backlog

- **SIPC-X01:** Fully extract MS-SIP 2017 §9 Contact Management Schema, all XML namespace required/optional fields and server error response examples; source text alone here does not fully specify complete 409 fault/curDeltaNum wire schema for contact list. Need MS-OCER rev11 **2046 contact version** and **2047 permission** differentiation.
- **SIPC-X02:** Historic legacy roaming ACL server/client responsibility vs modern MS-PRES container membership (IDs 0/100/200/300/400/32000 and version) can coexist by server support, but don't infer same authorization predicate across both without product-era notes.
- **SIPC-X03:** `prevDeltaNum`/fullDelta history continuity, duplicate/out-of-order BENOTIFY, endpoint disconnect, version rollover, and user-enabled `subscribed=false` semantics remain open for exact Windows Full Client behavior.
- **SIPC-X04:** The §3.7 group identifier range 1–63 and legacy default group #1 may not be universal for every modern client/server; constrain claim to **rev8 2017 source profile**, not every 2025 presence model.
- **SIPC-X05:** Batched PIDF, enhanced category subscriptions, self-roaming SUBSCRIBE, contact list SUBSCRIBE, ACL subscription and directory search are **different event, Content-Type and security states**. Need coverage matrix across all.
- **SIPC-X06:** Older RFC3265 event ordering and Microsoft-specific autoextend/benotify/piggyback versus superseding RFC6665 must be audited for historical client's actual SIP event implementation, not auto-upgraded.

**Research NOT COMPLETE. No implementations, live-server or interoperability tests, main merges or releases.**
