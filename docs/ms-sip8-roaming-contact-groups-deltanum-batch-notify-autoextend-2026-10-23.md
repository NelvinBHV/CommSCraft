# MS-SIP rev8 — roaming contacts/groups, deltaNum, batched subscriptions and Microsoft NOTIFY optimizations

**Research date: 2026-10-23. Historic source [MS-SIP] Session Initiation Protocol Extensions revision 8.0, v20170601, release June 1 2017**, SHA256 `6ee1c16c922b7c154373572d5a3341139aaf6d91922a35468581a209f68e9e52`, official original https://winprotocoldocs-bhdugrdyduf5h2e4.b02.azurefd.net/MS-SIP/%5BMS-SIP%5D.pdf . Primary §§2.2.3–2.2.4.2,3.2,3.3.1–3.3.7,3.4.1–3.4.7,3.5.1–3.5.7,3.6.1–3.6.7; PDF p18 and p35 rendered/inspected. Microsoft Learn authoritative [roaming contacts format](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-sip/d2176909-6e70-437f-9afe-bfe7c10c315f) and [roaming XML semantics](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-sip/242e789a-186a-4ed7-8d36-49dc02b0d708). RFC baselines: [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) (June 2002), [RFC3265](https://www.rfc-editor.org/rfc/rfc3265) (June 2002; RFC6665 July 2012 supersession must be treated separately), [RFC4662](https://www.rfc-editor.org/rfc/rfc4662) (Aug 2006 resource lists, RLMI), [RFC4480](https://www.rfc-editor.org/rfc/rfc4480) (2006 RPID, presence). Companion originals **MS-PRES rev13 v20250520**, **MS-SIPREGE rev17.1 v20250819**, **MS-OCER rev11 v20250520**, **MS-ABS rev10.1 v20250819**. **P** original clause, **R** IETF edition, **S** inference, **U** unproved deployed client capability. 2017 document edition is especially pertinent to historical client architecture but not proof of exact installed SfB version.

## I. Contacts/group objects are not presence category instances

| ID | Source clause | Actor-specific rule and data |
|---|---|---|
| CONTACT-01 | MS-SIP §2.2.4.1 | `application/vnd-microsoft-roaming-contacts+xml` supports **full contact/group list** and **partial `contactDelta` list**. Group fields `id` (nonnegative), `name`, optional `externalURI`; contact fields `uri`, `name`, space-separated `groups` IDs, Boolean `subscribed`, optional `contactExtension`. A contact with `subscribed=false` can exist in roster **without presence subscription**. |
| CONTACT-02 | §2.2.4.1 | Full list lists groups before contacts; group IDs are not persistent stable identities across transmissions; contact associations refer to current group IDs. A local cache MUST NOT assume an old numeric group ID is everlasting stable after a full reset. This is a *source data model* statement; validation of a given full document/version is still needed. |
| CONTACT-03 | §2.2.4.1 | `contactDelta` holds six change records: **addedGroup, modifiedGroup, deletedGroup, addedContact, modifiedContact, deletedContact**. Each delta MUST include `deltaNum` (new view version) and **`prevDeltaNum`** (expected prior version). Unlike MS-PRES category `version`, the contact mutation is linked to *list-view sequence*. |
| CONTACT-04 | §2.2.4.2 | Contact/group edits are SOAP primitives within SIP **`SERVICE`**: **setContact/deleteContact/addGroup/modifyGroup/deleteGroup**. Each primitive **must include the current view's `deltaNum`**. Source `subscribed` Boolean controls whether to subscribe to contact presence, distinct from the presence ACL container. |
| CONTACT-05 | §§2.2.4.1–.2 | Contact editing and category publication have **different XML namespaces, version sources and response formats**. Source **MS-OCER §7.3** maps stale **contact version to 2046**, stale **publication to 2044**, **container to 2045**, **permission to 2047**; a global `lastUpdateVersion` field has no normative basis. |
| CONTACT-06 | §2.2.4.1 | Server returns **full roster** for SUBSCRIBE to `Event: vnd-microsoft-roaming-contact`, while changes can arrive as **delta** in SERVICE responses or notifications depending route and extension. **Server list != offline ABS GAL**, even when both include the same SIP URI. |

## II. Batched presence subscription is a distinct resource-list dialog

| ID | Section | P rule |
|---|---|---|
| SUB-01 | §§3.3.4.1,3.3.5.1 | Client may issue **batched SUBSCRIBE** for multiple contacts using resource-list/extended presence events, avoiding one dialog per contact; first related NOTIFY can have **multipart/related** body with **`application/rlmi+xml`** list plus per-contact `text/xml+msrtc.pidf` presence payload referenced by Content-ID. This is not the roaming contact group list itself. |
| SUB-02 | §3.3.5.1 | Sample RLMI `list version`, `fullState=true`, per-resource `instance` id/state/`cid` join the multipart parts. Per-contact `presentity` can describe availability, activity, devices, device `epid`. **S:** validate CID associations, document-specific full vs partial and privacy before applying, not simple positional ZIP of contacts to MIME parts. |
| SUB-03 | §3.3.5.2 | If server rejects batched SUBSCRIBE because unsupported, **client MAY set `BatchSubscribeEnabled=false`** and fall back to **individual SUBSCRIBE per contact**. This is a fallback between *subscription formats*, not authentication downgrade, TCP plaintext fallback or unconditional user logout. |
| SUB-04 | §3.3.5.1 / RFC4662 | Partial/notified presence inside multipart is **not full group roster**. RLMI RFC4662 complements RFC3265's event framework; generic RFC4662 alone does not describe proprietary `text/xml+msrtc.pidf` bodies and Microsoft flags. |
| SUB-05 | §3.3.4.1 | Subscription choice is conditional on peer/server capability and feature tags; a legacy server can require different flow. User's `contacts loaded` and `all contacts' presence subscribed` states must be separate. |

## III. Piggyback, BENOTIFY and autoextend cannot be collapsed into standard NOTIFY

| ID | Original | P clause |
|---|---|---|
| NOT-01 | MS-SIP §3.4.5.1 | If `Supported: ms-piggyback-first-notify` is negotiated, server can include first NOTIFY **body in SIP 200 OK to SUBSCRIBE**. Client MAY process that 200 as an effective first notification; server delays ordinary first NOTIFY until actual presence change. Do not wait forever for a separate initial NOTIFY when piggyback accepted. |
| NOT-02 | §3.5 | **BENOTIFY** is best-effort, **never answered** with a SIP response; client **MUST NOT send any SIP response to BENOTIFY**. Server ignores any received response. Client SHOULD still process BENOTIFY body with ordinary NOTIFY logic. This differs from RFC3265 standard NOTIFY transaction. |
| NOT-03 | §3.5.5.2–.3 | BENOTIFY feature is **optional and negotiated per subscription**; server may decline for unreliable network connections; client MUST still be prepared to receive ordinary NOTIFY. A generic “always acknowledge every request” SIP dispatcher would violate BENOTIFY rule. |
| NOT-04 | MS-SIP §3.6.4.1–3.6.5.1 | Subscription autoextension optional via **`Supported: com.microsoft.autoextend`** in SUBSCRIBE and in server **200**. Client MAY enable per-dialog flag only when negotiated. It should not globally assume all SUBSCRIBE dialogs auto-renew after any NOTIFY. |
| NOT-05 | §3.6.5.2 | When autoextension negotiated, client SHOULD **reset subscription expiry timer upon incoming NOTIFY**; otherwise client SHOULD refresh via SUBSCRIBE before actual expiry. A received ordinary NOTIFY and BENOTIFY may have different feature semantics depending negotiated profile; do not assume autoextend from a non-negotiated BENOTIFY. |
| NOT-06 | §§3.3.5.2,3.4.5.1,3.5,3.6 | **Batched SUBSCRIBE**, **piggyback-first-NOTIFY**, **BENOTIFY**, and **autoextend** are four *independently optional* protocol extensions. Their booleans/timers are scoped per relevant dialog/feature, not one global `presenceOptimized` flag. |
| NOT-07 | §§3.3.5.1,3.6.2 | SUBSCRIBE refresh/expiry and contact/group list `deltaNum` are distinct timers/sequence numbers; receiving a valid presence change does not by itself acknowledge a concurrent contact roster edit. |

## IV. Cross-RFC event lifecycle example (S; no fixed client trace)

~~~mermaid
flowchart TD
 A[Successful SIP REGISTER with endpoint GRUU] --> B[Retrieve roaming contact/group full list]
 B --> C[Cache contact deltaNum and group relationships]
 C --> D[Client SERVICE setContact/addGroup/... with current deltaNum]
 D --> E{Accepted roster mutation?}
 E -->|Yes| F[Delta with prevDeltaNum -> deltaNum]
 E -->|Conflict| G[Reconcile roster version, no blind overwrite]
 C --> H{Batch presence subscriptions supported?}
 H -->|Yes| I[One batched SUBSCRIBE and multipart RLMI + per-contact presence]
 H -->|No| J[Individual SUBSCRIBE per subscribed contact]
 I --> K{First NOTIFY piggyback capability?}
 K -->|Yes| L[Handle SIP 200 as first notification]
 K -->|No| M[Receive separate NOTIFY]
 L --> N[BENOTIFY no SIP response if negotiated; ordinary NOTIFY normal response]
 M --> N
 N --> O{autoextend negotiated?}
 O -->|Yes| P[Refresh expiry from NOTIFY]
 O -->|No| Q[Independent periodic SUBSCRIBE refresh before expiry]
~~~

**Synthesis qualifier:** SIP REGISTER is not by itself a roster snapshot, a subscription, or permission to see each contact's containers. MS-PRES server privacy selection is applied independently after each subscription. Any roster delta gap must not be filled by invented change records.

## V. Documented gaps / edition variance

- **CONTACT-X01:** MS-SIP is **2017**, MS-PRES **2025**, MS-SIPREGE **Aug 2025**. The 2017 source includes older authentication and presence extensions; current client and server may use different generations, roaming contacts version numbers and UCS migration. Exact target Windows product build and unified contact store profile are unverified.
- **CONTACT-X02:** Reconcile source §2.2.4.1 full group ordering, transient numeric group IDs, delta restart and actual server counter wrap/delta recovery. **No invented retry/backoff** for 409 contact version conflicts from ErrorId 2046 until original roster update procedure and error payload verified.
- **CONTACT-X03:** RFC3265 is **obsoleted by RFC6665** but the historical MS-SIP references RFC3265. Do not assume all 2012 updates apply to a 2017-era Microsoft extension without source corroboration.
- **CONTACT-X04:** **BENOTIFY may be lost** by design because it is best effort with no SIP response; need partial/full roster or subscription refresh recovery proof to prevent permanent divergence.
- **CONTACT-X05:** Full details of ACL `application/vnd-microsoft-roaming-acls+xml` and MS-PRES containers/delegates must be audited for client sharing scope and federation trust. Contact stored != presence subscribed != authorized to view status.
- **CONTACT-X06:** The MS-SIP §3.3.5.1 example shows multiplexed presence in one MIME body but no proof every installed client offers same supported tags. Captured target build compatibility and server product notes still U.
- **CONTACT-X07:** Must audit error/fallback for expired SUBSCRIBE, missing NOTIFY, invalid Content-ID, out-of-order RLMI versions and multi-endpoint roster change. Do not reuse SIP REGISTER `Expires` to derive subscription expiry.

**Global RFC/MS research INCOMPLETE. No protocol code, no connection to real server, no assistant interoperability testing, no merge to main or release.**
