# MS-PRES 13.0 §§3.9–3.10 and §§6.1–6.14 — delegates, UCS migration and actual on-wire XSD contract

**Original edition:** [MS-PRES] 13.0, **2025-05-20**, SHA-256 `f1e0ce966ab242cf09b57e4403f9cfa7aec9c5aa883325f1e4ac91ad1ff5fa79`, official PDF https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-PRES/%5BMS-PRES%5D.pdf . Directly checked complete source text §§3.9.1–3.9.7,3.10–3.10.6,6.1–6.7,6.10–6.14 and selected §§1.3.4–1.3.5,2.2.2.10; original PDF pp131–134 and pp202–213, including p211 **visually rendered** to verify namespace discrepancy. Also cross-reference **[MS-SIP] 8.0 v20170601** §3.7 for roster/ACL, **[MS-SIPREGE] 17.1 v20250819** for REGISTER presence state and **[MS-OCER] 11.0** §7.3 errors. Historic [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) (SIP2002), [RFC3265](https://www.rfc-editor.org/rfc/rfc3265) events2002 superseded by RFC6665; no live servers or parsing code.

**Evidence:** P = exactly what official edition prescribes with actor and condition; S = derived cross-layer safety claim; U = unresolved source/edition ambiguity. A XML Schema statement and prose MUST may be **independent constraints**, and **XML namespaces are case-sensitive**.

## A. Delegates — a separate optimistic version state (not privacy-container membership)

| ID | Clause | Source-specific behavior |
|---|---|---|
| DEL-01 | §3.9.1 | Presence server maintains **DelegateSet per delegator**, keyed by **delegate SIP URI** and with **one version number for the whole set**. Each DelegateEntry has **publish** and **redelegate** permission Booleans. Neither is the contact list deltaNum nor category instance version. |
| DEL-02 | §§3.9.5.1–3.9.5.2 | Delegator submits SIP **SERVICE** `setDelegate` and operates on **one delegate per request**. Server MUST verify **From URI = To URI**, use this identity to select its DelegateSet, validate XML and require submitted **version = current set version**. On first empty list server version is **0**. |
| DEL-03 | §3.9.5.2 | Delegate action **`add`, `modify` or `remove` only**. Delegator **cannot delegate to their own URI**; specified delegate URI **MUST represent valid presentity in deployment**. |
| DEL-04 | §§3.9.5.2,6.14 | `publishOperation` and `redelegateOperation` each accept `authorization=allow|deny`. **Absent operation element means deny**, not an implied inherited authorization or grant. |
| DEL-05 | §3.9.5.2 | **`add` on an already existing delegate is valid, updates it**; **`modify` on a missing delegate is also valid, creates it**. `remove` on a nonexisting delegate **MUST be rejected**. These are not generic HTTP PUT/PATCH semantics; treat behavior exactly as specified. |
| DEL-06 | §3.9.5.2 | Server SHOULD enforce administratively configurable delegate limit; increments **DelegateSet version only after successful processing**, MUST NOT commit changes before full processing, returns SIP **200 OK** on success. |
| DEL-07 | §§3.9.5.3,6.5,6.7 | After successful change server MUST send **NOTIFY or BENOTIFY to each matching self-subscription** whose scope includes delegates, **including sender endpoint**. The `roaming-self+xml` `delegates` child **MUST be complete new set**, **not a partial per-delegate delta**; when empty, send empty delegates body with **updated version**. |
| DEL-08 | §3.9.5.4 | Errors: **400** invalid XML, delegate equals delegator; **403 SHOULD** From/To mismatch; **403 MUST** remove missing delegate; **404 MUST** invalid presentity URI; **409 MUST** stale version; **413 MUST** quota exceeded. **Lync Server 2010 note <48>** varies From/To mismatch response, so cannot apply 403 to every historical product. |
| DEL-09 | §3.9.2,3.9.6 | Delegate management has **no separate protocol-defined timers** in its subsystem. SIP transactions, authentication SA expiry and self subscriptions retain their independent timers. |
| DEL-10 | §6.14 | XSD **`<setDelegates version=unsignedInt><delegate uri=anyURI action=add|remove|modify>`**, optional operation elements each with required `authorization=allow|deny`. This schema is **separate** from `<setContainerMembers>` and MS-SIP `setContact`. |

## B. Unified Contact Store (UCS) is a conditional migration state machine

| ID | Clause | Exact source and lifecycle |
|---|---|---|
| UCS-01 | §§1.3.5,3.10 intro | UCS migrates groups/roster **from enhanced presence server to email server contact store**; it is neither an MS-ABS downloadable global address book nor a SIP category container. Server uses MS-SIP §3.7 legacy contact/group data model. |
| UCS-02 | §3.10.4, product note <49> | A SIP client able to connect to email server **SHOULD** send roaming-contact `SUBSCRIBE` with `Supported: ms-ucs-ready` to signal migration readiness. Server queues user for migration; **exact start time is implementation-specific**. Product note says this behavior is **only supported by Lync Server 2013** in specified overview application. Do not infer every deployment supports it. |
| UCS-03 | §§1.3.5,3.10.4 | **During migration** the contact/group list becomes **read only**. Server rejects **SERVICE mutations**. A failed `setContact` in this phase is not proof authentication has expired, and retrying rapidly cannot override server migration state. |
| UCS-04 | §3.10.4 | **After migration**, SIP endpoints that cannot access email server **MUST NOT send ms-ucs in SUBSCRIBE**. Presence server proxies contact list updates; whether full snapshots vs deltas, reads only vs reads+writes are **implementation-specific**. Clients with email-server connectivity **SHOULD include ms-ucs supported**. |
| UCS-05 | §3.10.4.1 | UCS failures add **MS-OCER §7.3** diagnostic errors on top of **MS-SIP §3.7** contact-list errors. No new UCS-only timer is defined (§3.10.2/.5). |
| UCS-06 | §§3.10.4, MS-SIP §3.7 | Client-side contact cache **cannot treat a read-only/migration rejection as authorization to delete contacts or groups**. A full snapshot may legitimately replace delta stream post-migration; cross-protocol reconciliation is **S**, not a fixed RFC event algorithm. |

## C. Original XML Schema reconciliation: field types and namespace are wire identity

| ID | Primary XSD | Normative structural finding |
|---|---|---|
| XSD-01 | §6.1 | `rpc:ContainerId` = **xs:unsignedShort ≤32767**; `rpc:ContainerIdForSetContainer` = **1..32767** (minExclusive0/maxInclusive32767). Thus **container0 cannot be edited** at XSD level, before server authorization rules. |
| XSD-02 | §6.2 | In `categories` **`uri:xs:anyURI` required**, individual `category name` required; **instance, publishTime, container, version, expireType, endpointId, expires are optional in the general categories-notification schema**. But §2.2.2.1 separately mandates attributes under particular responses; do not let broad XSD optionality override those procedural MUSTs. |
| XSD-03 | §6.3 | `<containers><container id=rpc:ContainerId version=unsignedInt><member type=... value?>`; member types **user/domain/sameEnterprise/federated/publicCloud/everyone**, member `value` optional in base schema; per-type semantic validation remains protocol-driven. |
| XSD-04 | §§6.6,6.7 | Request root `roamingList` in namespace **`http://schemas.microsoft.com/2006/09/sip/roaming-self`**; required **≥1 `roaming`** from categories/containers/subscribers, optional later **rex:roamingEx** allowing delegates and extension values. Response root **`roamingData`** permits optional **categories, containers, subscribers, delegates** in that order and respective imported namespaces. |
| XSD-05 | §6.10 | Publication root **`publish`** contains exactly **one `publications`** child with **≥1 `publication`**; `publications uri` required. Per-publication **categoryName:string, instance:unsignedInt, version:unsignedInt, container:rpc:ContainerId, expireType enum are required**, `expires:unsignedInt` optional but only legal for `time` per message-processing prose. `xs:any` permits one custom category payload; extension wildcard does not exempt required attributes. |
| XSD-06 | §6.11 | Container mutation root **`setContainerMembers`** has ≥1 container; each has ≥1 member, container id **1..32767** and **version unsignedInt required**. Member action enum **`add|remove`**, **`add` default**, type required; **NOT `delete`**. Any previous research table asserting `action=add|delete` for this exact XML schema is incorrect and **MUST be amended**. |
| XSD-07 | §6.12 | `setSubscribers` schema requires **exactly ONE** subscriber element, `user` required and `acknowledged:boolean` required **fixed true**. This syntactically enforces one acknowledgement per SERVICE; cannot send `acknowledged=false` or a batch. |
| XSD-08 | §6.5 | Self-notification `delegates` has **version unsignedInt required** and delegate entries with **uri anyURI, publish boolean, redelegate boolean all required**. This notification differs from mutation schema's `authorization allow/deny` attributes and add/modify/remove action. |
| XSD-09 | §6.13 | **Internal namespace inconsistency in original:** Section's heading `Location:` says **`.../2008/09/sip/subscription-context` (lowercase)**, but the literal XSD **targetNamespace and xmlns:tns are `.../2008/09/sip/SubscriptionContext` (capital S/C)**; namespaces are case-sensitive. Both exact spellings appear on **PDF p211** (visually confirmed), and cannot be silently normalized. Wire-level interoperability needs authoritative erratum and original example cross-check; **OPEN-NORMATIVE-CONFLICT**. |
| XSD-10 | §6.13 | Subscription context shape can include watcher contactList, optional delegator list, protocol extension `ct:delimiter`, `ct:end`, `ct:extension`; these **do not** establish that any arbitrary contact roster subdata is safe to render as public watcher data. Input context scopes/ACL separately required. |
| XSD-11 | §6.14 | Delegation request uses **`action=add|remove|modify`**, version unsignedInt, `delegate uri` required, operation `authorization=allow|deny`, optional publish/redelegate operation; `modify` missing/delegate and `add` existing behavior is application semantics (3.9), not XSD cardinality. |
| XSD-12 | §§6.2,6.7,6.10 | **Response `categories`** and request `publish/publications` use **different namespace/document roots** and different attribute optionality. A generic `category` XML deserializer that requires request fields in every response will reject allowed protocol notification data. |

## D. Exact key client/state distinction (S)

~~~mermaid
flowchart TD
 A[Authenticated registered endpoint] --> B[Self SUBSCRIBE includes delegates scope]
 B --> C[Full delegates snapshot/version]
 C --> D[SERVICE setDelegate version and single delegate]
 D --> E{Server validates URI, self, quota and version}
 E -->|400 403 404 409 413| F[No version advance; domain-specific error]
 E -->|200| G[DelegateSet version advances, atomic commit]
 G --> H[NOTIFY/BENOTIFY COMPLETE delegate list to each self endpoint]
 A --> I[Roaming-contacts SUBSCRIBE with conditional ms-ucs-ready]
 I --> J{Server UCS migration state}
 J -->|Not migrating| K[MS-SIP roster snapshots/deltas]
 J -->|Migrating| L[Read-only contact/groups, mutation rejected]
 J -->|Migrated| M[Email-store direct or proxy full/delta by policy]
~~~

**XSD separation:** `setDelegates`, `setContainerMembers`, `setSubscribers` and `publish` are **distinct XML namespaces, request cardinalities, auth scopes and version domains**; a common 200 SIP status does not fuse them. The order of self NOTIFY across multiple devices and migration-to-email contacts update remains unproven.

## E. Source conflicts / precise proof tasks blocking code

- **D-X01** The **MS-PRES 13.0 §6.13** namespace `Location` versus literal `targetNamespace` casing difference requires official Microsoft erratum or authoritative packet example; **do not silently choose a normalization**.
- **D-X02** The original **MS-PRES §3.9.5.2** spells MIME **`application/msrtc-setDelegate +xml`** with a space before `+xml` in prose, while other material denotes `setDelegate` independently. **Do not treat the apparent whitespace typo as a verified literal HTTP/SIP Content-Type**. Source examples/schema and erratum required.
- **D-X03** An **older research table** claimed container member action `add|delete`. Actual original **§6.11** unambiguously specifies `add|remove`; update historical research artifact to prevent misleading future coders. Current document uses correct schema only.
- **D-X04** In §3.9.5.4 a malformed **delegate SERVICE** is described as invalid roaming-self XML; this may be a source wording mismatch with dedicated delegate-management XSD §6.14. **Actual target parser error schema not yet confirmed**.
- **D-X05** Historical **UCS** §3.10 note <49> says only Lync Server 2013 supports specified behavior; target SfB 2015/2019/server SE applicability remains **U**, not universally mandated.
- **D-X06** UCS `ms-ucs-ready` supported header wording differs from §3.10.4's later `ms-ucs` wording. Need inspect exact raw SIP example and support token registration rather than invent two distinct tokens or silently treating them synonyms.
- **D-X07** Delegator ACL/grant intersection with MS-PRES container permission, SIP impersonation, default `deny`, MS-SIP legacy ACL and conference delegation require complete first-party §2.2.2.10 and MS-SIPAE delegated publishing specification.

**Status: selected §3.9–3.10 and 6.1–6.14 XML contract audited, not all 257 pages or every MS-PRES use case. No implementation, server connection, interop tests, main merges, or releases.**
