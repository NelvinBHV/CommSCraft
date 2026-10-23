# MS-PRES 13.0 §3.8 — reconstructed aggregation transformations, privacy output and device-service selection

**CommSCraft original-source research, 2026-10-23. Source:** Microsoft [MS-PRES] Presence Protocol **rev13.0 / v20250520 (2025-05-20)**; original PDF SHA-256 `f1e0ce966ab242cf09b57e4403f9cfa7aec9c5aa883325f1e4ac91ad1ff5fa79`; official original: https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-PRES/%5BMS-PRES%5D.pdf . Primary source **§§3.8–3.8.7, 2.2.2.7.1–2.2.2.7.3 and §4.3**; **PDF pp114–130**, especially source **Figures 3–12**, including Figure 8 lastActive flowchart **visually rendered and directly inspected**, and multi-page table pp123–125. Direct prior integration: [MS-PRES earlier publication / version / subscriber review](ms-pres13-category-container-lifetimes-version-conflicts-original-2026-10-23.md), [MS-SIP rev8 roaming contacts](ms-sip8-contact-groups-roaming-synchronization-2017-original-2026-10-23.md), MS-SIPREGE rev17.1. Historical SIP events [RFC3265](https://www.rfc-editor.org/rfc/rfc3265), now superseded by RFC6665; Microsoft category aggregation is **not** prescribed by a generic SIP RFC. Source labels **P** = exact Microsoft section for named actor; **S** = explicitly identified cross-source synthesis; **U** = still unverified target Windows Full Client or source ambiguity.

## 1. Central finding: the server is a transformation engine, not a last-event relay

| ID | Clause | Normative actor-specific rule |
|---|---|---|
| AG-01 | §3.8 intro | **Server** transforms an array of previously published category instances into a computed output **for each configured output container**. The same transformation function may produce **different results based on that container's inputs**. Clients do not own or directly overwrite read-only computed instances. |
| AG-02 | §3.8 intro | Inputs record **container number, category name, instance number, publish time and category value**. Computed-category configuration is global for all publishers, **not private per publisher**. The server can trigger recomputation on changed dependent publications. |
| AG-03 | §§3.8,3.8.5.1.1 | Clients MUST NOT add **members to private computation input containers 2 or 3** even where the server does not enforce this. Data in those inputs may disclose private per-endpoint state if subscriptions were permitted. Server publishes constrained aggregates to observer-facing containers instead. |
| AG-04 | §3.8.5.1.1 | **Server** computes **aggregateState** and **aggregateMachineState**, and writes special **legacyInterop** categories for PIDF/MSRTC watchers; legacyInterop **MUST be registered as a private category**. Do not expose raw machine data via legacy PIDF notifications. |
| AG-05 | §3.8.5.1.2.1 | For **aggregateMachineState**, server considers only `machineState` category instances whose **expireType is endpoint**. If **none**, aggregateMachineState availability **MUST become 18500 (offline)**. User, static or time-scoped machineState must not win this competition. |
| AG-06 | §3.8.5.1.2.1 | When ≥1 eligible machineState exists, **lowest numeric availability** wins; tie → the **most recently published** machineState, and copy selected machineState attributes into aggregateMachineState. Do not sort descending or simply take latest endpoint regardless of availability. |
| AG-07 | §3.8.5.1.2.2 | For aggregate availability, collect `state` instances from selected container, **exclude** types `machineState`, `aggregateMachineState`, `aggregateState`, then add computed `aggregateMachineState`. This is a **different set** from winner selection above. |
| AG-08 | §3.8.5.1.2.2 | When a state has `manual=true`, discard **other state instances older** than it using `startTime` (or category publicationTime if startTime absent); **aggregateMachineState is exempt** from that filtering. Do not make manual state erase every machine endpoint. |
| AG-09 | §3.8.5.1.2.2 | Base **aggregated availability** is the **largest numeric availability** among the retained states. This intentionally differs from the **lowest numeric** machineState selection. Two-stage reduction is required, not a universal `min` or `max`. |
| AG-10 | §3.8.5.1.2.2 | If the computed **aggregateMachineState** is **Idle availability class 4500–5999** and base aggregated availability is **Busy 6000–7499**, add **1500** to the aggregated availability to produce resulting **aggregateState** availability. This is conditional, not a blanket idle offset. |
| AG-11 | §3.8.5.1.2.3, notes <43>,<44> | Activity selection uses the state set from availability computation; remove activities with **neither token nor custom string** and those outside activity's **minAvailability..maxAvailability**. Winner **MUST have greatest minAvailability**; ties prefer most recent **startTime if present, else publication time**. If none, aggregate activity **SHOULD be empty**, edition note excludes OCS 2007. |
| AG-12 | §3.8.5.1.2.4, **Figure 8** | For `lastActive`, if **new aggregate availability** is **not** Idle, BusyIdle, Away or Offline → set `lastActive` **empty**. If it **is** one of those classes and **old aggregate availability wasn't**, set lastActive to **server current UTC**. If both previous/new are in these classes, preserve existing lastActive. **This is from the figure, not an inferred paragraph.** |
| AG-13 | §3.8.5.1.2.5 | If a single relevant **calendarState** contributes `meetingLocation` or `meetingSubject`, server **MUST use** them; if multiple qualifying calendarState instances exist, aggregated subject/location **MUST be empty**. Never arbitrarily choose first meeting and leak it. |
| AG-14 | §3.8.5.1.2.6, note <45> | When aggregate availability **<12000**, server **SHOULD** copy endpointLocation/time-zone elements/device type from aggregateMachineState. For **>=12000**, server **can ignore** these fields. This is a *SHOULD* and *can*, not a universal MUST. |
| AG-15 | §3.8.5.1.2.7 pp123–125 | Input **container 2** produces **state+legacyInterop** for **100**, **200**, **400**; input **container 3** produces **state+legacyInterop** for **300**. Distinct output detail sets are explicitly defined in multi-page table. |
| AG-16 | §3.8.5.1.2.7 | For **100**, state category exposes **aggregate availability only** (plus xsi:type); for **200**, adds activity token, lastActive, device; for **300**, adds **meetingSubject, meetingLocation, endpointLocation, time-zone/device**; for **400**, adds endpointLocation/time-zone/device but **not meetingSubject/Location**. **Do not merge to a universal rich presence payload.** |
| AG-17 | §3.8.5.1.2.7 | Server also creates **container 2 aggregateMachineState, instance 0x10000000, expireType user** and **container 2/3 aggregateState**. For generated observer aggregates output `expireType user, instance1` if any machineState exists, or `static, instance0` when none. A computed state may persist as a static fallback even with no live endpoint. |
| AG-18 | §3.8.5.1.2.7 | **legacyInterop** per observer container contains aggregate availability only and is used for legacy format generation. **State vs legacyInterop are different category names and may be independently selected** by subscriber protocol. |
| AG-19 | §3.8.5.1.1 | Server can avoid publishing if computed category equals existing instance (optimization). Client MUST NOT infer “no new NOTIFY” means no recomputation happened; distinct timestamps and event versions still require source-backed handling. |

## 2. Aggregating capabilities across simultaneous device endpoints

| ID | Clause | Source rule and error boundary |
|---|---|---|
| AG-20 | §3.8.5.2.1 | Each endpoint publishes **machineState and device capabilities to private input container 2**. Server MUST trigger **services** aggregation on changed state/device category instances. Computed services published for self to **2**, observers to **100,200,300,400**; server MAY optimize unchanged outputs. |
| AG-21 | §3.8.5.2.2 | Create set of **distinct URI attributes** from device capabilities; **one service element per distinct URI** (MUST). Duplicate endpoint URIs do **not** imply two service records, but may contribute competing capabilities. |
| AG-22 | §3.8.5.2.2 | For each URI and capability type, pick winning capability and machineState linked through **endpointId**. Source defines multiple conditions involving `publish`, `render`, `capture`, **capability version**, and **most active endpoint** (lowest availability then newest publishTime); one global boolean `canVideo` is insufficient. |
| AG-23 | §3.8.5.2.2 | For non-publish (`publish=false`) capabilities, winner is added to **observer services** and **self services**; publish-only capabilities are added to **self services**. Winning endpoint's availability becomes `deviceAvailability`, and self view additionally carries `preferredEndpointId`. |
| AG-24 | §3.8.5.2.2 pp128–129 | All 4 observer `services` aggregates use **instance0, expireType user** and carry only **render/capture** capability, each SHOULD have `deviceAvailability`. Self **container 2** includes render/capture/**publish** and SHOULD carry `preferredEndpointId` + `deviceAvailability`. This is *purposefully asymmetric*. |

## 3. Derived workingHours and dndState

| ID | Clause | Source rule |
|---|---|---|
| AG-25 | §3.8.5.3 / note <46> | Client **MUST** publish `calendarData` **instance0 in container1** for workingHours derivation. Client MUST NOT add members to container1, MUST compare new data with prior to avoid unchanged republishing. Server transforms `WorkingHours` element to **workingHours instance0 in container1**, unpublishes when source removed; avoids identical output. Feature **not supported** in OCS/Communicator 2007/2007 R2. |
| AG-26 | §3.8.5.4 / note <47> | Client **MUST** publish `state` to containers **2 and 3**, MUST NOT add members there. Server computes `dndState instance0` into containers **2,0,100,200,400** from input2, and **3,300** from input3. |
| AG-27 | §3.8.5.4 | Server examines `userState` / `presentingState`. If qualifying source availability is in **DND 9000–11999**, published dndState availability MUST be **9500**; otherwise **omit availability element**. This is not a copy of arbitrary requested DND number. Feature excluded from OCS/Communicator 2007/2007 R2 per note <47>. |
| AG-28 | §§3.8.2,3.8.5.5–3.8.7 | Aggregation subsystem declares **no additional timers, error responses or other local events** in its own section. This does **not** negate SERVICE publish/schema/permission errors in §3.2.5.4 or downstream SUBSCRIBE transaction timers. |

## 4. Four independent state layers (S, not one fixed Windows packet trace)

~~~mermaid
flowchart TD
 A[Endpoint registers +sip.instance] --> B[Endpoint publishes machineState and device in private input container2]
 A --> C[Calendar or user/presenting state in designated input container]
 B --> D[Server select most active endpoint min availability; tie latest publish]
 C --> E[Merge manual/calendar/user states and compute aggregate availability]
 D --> E
 E --> F[Optional idle-busy +1500; activity and lastActive rules]
 F --> G[Separate privacy outputs 100,200,300,400 with different fields]
 B --> H[Services aggregation per URI and capability type]
 H --> I[Self service: publish, render, capture, preferredEndpointId]
 H --> J[Observer service: render, capture, deviceAvailability]
 G --> K[Subscriber permissions select observable categories]
 J --> K
 K --> L[NOTIFY/BENOTIFY per category subscription]
~~~

**Architectural meaning:** Presence `machineState` choice uses **min** but overall status reduction uses **max**, after manual state filtering and conditional adjustments. The active endpoint for a specific capability may differ from the endpoint that appears most active in an aggregate status. Therefore **client-visible presence, capability selection, privacy permissions and MPOP endpoint identity remain separate state machines**.

## 5. Current outstanding checks; no unsupported code planning

- **AG-X01** Reconstruct the complete capability winner multi-condition specification as a decision table with actual MS-PRES §2.2.2.7.3 fields and effective `publish` precedence, including equal capability versions; the text's nested bullets in the PDF have potentially ambiguous association.
- **AG-X02** Figure 8 **lastActive** is authoritative; the diagram alone does not define all possible Availability class boundaries. Need category enumerations & source product variation notes, not manufactured idle classes.
- **AG-X03** Audit all `state`, `activity`, `legacyInterop`, `services`, `calendarData` XML body structure and §6 schema references; image-verified container table does not automatically yield full wire XML.
- **AG-X04** Service mutation, permissions and watcher notifications may race with endpoint deregistration; do not invent deterministic cross-server ordering, media availability or global signed-in behavior.
- **AG-X05** Product release notes <42>–<47> explicitly exclude specified OCS 2007/2007 R2 behavior; the **2015/2016 classic Windows client binary edition and configured server** remain a separate applicability proof gate.
- **AG-X06** Section 3.8 “None” timers/errors refers to aggregation procedure only, not SIP publication/in-band subscription logic. Do not suppress explicit 400/403/409/413/488 from MS-PRES §3.2.
- **AG-X07** Modern privacy/per-category container selection, competing membership (federated/domain/everyone/user) and server control of computed read-only category instances require complete §3.5 source analysis and product-version validation.

**Overall CommSCraft Windows Full Client RFC + Microsoft protocol analysis INCOMPLETE. No implementation, no real-server access, no interoperability tests, no merge or releases.**
