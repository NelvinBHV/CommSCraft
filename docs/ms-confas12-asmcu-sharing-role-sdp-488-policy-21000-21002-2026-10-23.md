# MS-CONFAS rev12 — conference desktop/application sharing, role negotiation, MCU roster, ICE and policy failures

**First-party research on 2026-10-23. STATUS: substantial selected-section review, NOT full 51-page norm completeness.** Microsoft **[MS-CONFAS] Centralized Conference Control Protocol: Application Sharing Extensions**, revision **12.0**, release **2025-05-20** (`v20250520`); archived PDF SHA-256 **659ce74ebe8039b04f3bc8b95a7f007bb7675abe471747339633bbbe8f1cc6be**, official Microsoft https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-CONFAS/%5BMS-CONFAS%5D.pdf . Primary clauses §2.2.1–2.2.2, **§§3.2.5.1.1–.6,3.2.5.2.1–.2.2,3.3.1.1,3.3.5.2.1–.2.2,3.3.5.4–.7.3,§4.2**; original PDF p**19** rendered (including server role/SDP rejection table) and official Microsoft Learn [§3.3.5.2.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confas/e00fb832-c86b-4e69-a4e6-4280bc5d4438) and [§4.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confas/23d5eecc-e92d-4011-bac9-c85903831a7e) independently reviewed. Exact direct RFC reference list **[RFC2119](https://www.rfc-editor.org/rfc/rfc2119)**, **[RFC3264](https://www.rfc-editor.org/rfc/rfc3264)**, **[RFC4028](https://www.rfc-editor.org/rfc/rfc4028)**, **[RFC4574](https://www.rfc-editor.org/rfc/rfc4574)**, **[RFC4575](https://www.rfc-editor.org/rfc/rfc4575)** (historic originals). First-party cross dependencies: [MS-CONFBAS], [MS-CONFPRO], [MS-ICE], [MS-ICE2], [MS-RDPBCGR], [MS-RDPEMC], [MS-RTASPF], [MS-RTP], [MS-SDPEXT], [MS-SIPRE]. **P** first-party actor MUST/SHOULD, **R** RFC, **S** cross-spec design constraint, **U** unresolved variant.

## I. Detailed SDP and conferencing user state

| ID | First-party clauses | Client / ASMCU mandatory or conditional action |
|---|---|---|
| AS-01 | §3.2.5.1.1 | Before constructing application-sharing SIP INVITE, **client is assumed already subscribed to conference events** under MS-CONFBAS and knows **ASMCU MCU-Conference-URI from conference document**, with `conf-uris/purpose=applicationsharing`. SIP registration or generic Focus INVITE alone does not supply RDP MCU URI. |
| AS-02 | §§3.2.5.1.2,3.3.1.1 | Client MUST advertise **`a=x-applicationsharing-role:sharer`** or **`:viewer`** in SDP as actual intended role; ASMCU answer **MUST reverse role** (sharer ↔ viewer). MCU roster **`media/status=sendonly`** for sharer, **`recvonly`** for viewer. This is a *role + direction mapping*, not SIP offer being from a local camera. |
| AS-03 | §3.2.5.1.2 | If initial offer has **neither `m=applicationsharing` nor `m=video`**, MCU MUST respond **SIP 488 Not Acceptable Here**. If active media not supported, per-profile rejected SDP slot uses **port=0** under RFC3264 §8.1. |
| AS-04 | §3.2.5.1.2 | Offer may contain max one sharing and one viewing instance; this **client-oriented section** says if extra matching `m=` lines, MCU MUST **reject additional instances port=0**. But **server-oriented §3.3.5.2.1** says if more than one media instance for sharing or viewing, **reject entire offer**. This is a **directly documented conflict**, not a free choice of semantics. |
| AS-05 | §3.2.5.1.2 | With `m=video`, client SHOULD label `applicationsharing-video` and MUST set sendonly/recvonly. When both RDP sharing and video are present, video direction MUST match application-sharing send/receive role; otherwise **entire offer 488**. |
| AS-06 | §3.2.5.1.2 | `a=x-applicationsharing-contentflow` in offer uses **sendonly/inactive** for sharer or **recvonly/inactive** for viewer; if **no active video m-line**, it **MUST NOT be inactive**. Server answer must **omit this attribute if active video is included**, else answer MUST set `inactive`. This is profile branching, not generic RFC3264 direction alone. |
| AS-07 | §§3.2.5.1.3,3.3.5.2.2 | After ICE checks, **final offer/answer with chosen candidates MUST be exchanged** under [MS-ICE] §3.1.4. Initial SIP 200/SDP acceptance does not prove ICE media final pair or RDP content connected. |
| AS-08 | §3.2.5.1.4 | Client renegotiating application-sharing media MUST issue **new SDP offer with new m=applicationsharing media line** and **deactivate existing m-line with port=0**. Source-specific profile—not an arbitrary UPDATE of the previous line. |
| AS-09 | §3.2.5.1.6 | SDP `a=x-capabilities` consists of **name=value** capabilities separated by **semicolons** (SHOULD), names SHOULD lowercase English letters/underscores/hyphens, values SHOULD `sendonly|recvonly|sendrecv|none`, *each* name/value pair MUST have "="; invalid formatting ⇒ **SIP 488**. MCU propagates accepted capabilities via C3P user/media notification (SHOULD). |
| AS-10 | §3.3.1.1; §3.3.5.2.1 | MCU correlates **SDP m-line ↔ per-user media element ↔ conference-medium-type** and SHOULD/MUST update roster for removed `port=0` and re-added slots. For unsupported offered media, **reject port=0**; if *all* media rejected ⇒ SIP 488; server answer includes selected **session-id**. |
| AS-11 | §3.3.5.4 | ASMCU **SIP 200 with SDP answer** must be followed by roster **full user** notification to Focus; after SDP negotiation, must send **`media-state=joining`**, then eventual connected/failed state under media control. Endpoints can show **SIP dialog connected** while appsharing RDP data not yet usable. |
| AS-12 | §3.3.5.5 | Roster signals **media disconnected by OMITTING corresponding `media` child**, not necessarily by removing user from conference. The endpoint itself can remain `status=connected`. Do not terminate Focus membership because RDP media disappeared. |
| AS-13 | §§3.2.5.1.5,3.3.5.6 | When a **new sharer** replaces old, MCU MUST eject **old sharer** by **SIP BYE**, public diagnostic **`ms-diagnostics-public:21000;reason="New sharer joined conference"`**. Server must emit old-media absent notification and `user state=deleted` notification, new-sharer media `joining` then `connected` when ready. Ordering between some notifications can vary (§4.2); no universal single ordered sequence is assumed. |
| AS-14 | §3.3.5.7.2 | When conference **server-mode ≠13 or absent**, ASMCU MUST reject attempted sharer INVITE by a role **attendee** with **ms-diagnostics-public 21002** ("Attendees cannot share in this conference"). Roster permission `AttendeesCanShare` MUST be true when mode=13 and false otherwise. Client cannot assume being in lobby/meeting confers presenter/share permission. |
| AS-15 | §3.3.5.7.3 | MCU SHOULD advertise `supports-application-desktop-sharing` one of **desktop**, **singleApplication**, **none**. Under `singleApplication`, client **MUST NOT initiate desktop-share** (single-application may be allowed); `none` forbids appsharing. This is independent from SDP codec compatibility. |
| AS-16 | §3.2.5.2.1 | RDP `Attendee Connected PDU` **GroupId=3766552189 MUST** under [MS-RDPEMC] §2.2.4.1. This is an RDP-sharing control field, not C3P conference-id, SDP session-id or an ICE candidate identifier. |
| AS-17 | §3.2.2 | Source **client role specifies no independent timers** in §3.2.2. Specific SIP transactions, ICE, SRTP and RDP media algorithms/timers are inherited from other standards, NOT implied to be absent. |
| AS-18 | §§2.2.1.1,3.3.5.4 | MCU roster endpoint XML extends [RFC4575] conferencing via [MS-CONFBAS]; `media-state` and `session-id` are cross-layer **conference view**, not raw RTP header fields, PSOM object identifiers or user-visible share credentials. |

## II. Desktop sharing state separation (S)

~~~mermaid
flowchart TD
 A[Authenticated registered user joined conference Focus and subscribed roster] --> B[Extract ASMCU URI by applicationsharing purpose]
 B --> C{Conference policy and user role permit sharing?}
 C -->|Attendee blocked| D[ms-diagnostics-public 21002, no sharer media]
 C -->|Allowed| E[INVITE SDP x-applicationsharing-role sharer or viewer]
 E --> F{SDP compatible and cardinality valid?}
 F -->|No| G[488 or port0 per source, duplicate case conflicting]
 F -->|Yes| H[ASMCU 200 SDP reverse role and session id]
 H --> I[ICE final offer answer and media parameter negotiation]
 I --> J[ASMCU roster state joining]
 J --> K{RDP/RTASPF and SRTP media connection?}
 K -->|Connected| L[roster media state connected / RDP data flows]
 K -->|Disconnected| M[roster omits media, Focus membership independent]
 L --> N{New sharer takes over?}
 N -->|Yes| O[Old sharer receives SIP BYE public diagnostic 21000]
 O --> M
~~~

**S caution:** The figure is a multi-source conditional transition sketch. §3.2.5.1.2 and §3.3.5.2.1 **disagree** on rejection scope for duplicate media lines. Branch must remain unresolved in client behavior reconstruction rather than resolved by an invented rule. A policy reject 21002 is not a SIP registration 403, and sharer-change BYE 21000 is not an ordinary user hangup.

## III. Source conflict, security boundaries and RFC edition work

- **AS-U1** Dual rule duplicate `m=applicationsharing` stream: **client section** says reject extras with port=0, **MCU section** says reject *entire* offer. Verify official product/edition notes and errata; do not hardcode one as conformance truth.
- **AS-U2** §3.2.5.1.2 uses prose “sharing”/“viewing” in some contentflow examples while enumerating exact wire `a=x-applicationsharing-role:sharer|viewer`. Treat prose words as descriptions, **not additional wire values**, pending source grammar reconciliation.
- **AS-U3** ICE final answer [MS-ICE] vs ICE2 [MS-ICE2], BWM [MS-ICE2BWM], SRTP [MS-SRTP]/[MS-SSRTP] and RDP/RTP packet protection remain separate normative dependency checks. Existing SHA-1/SHA-256 SRTP media conflict unresolved; never weaken crypto to meet sharing UI.
- **AS-U4** Full [MS-RDPEMC], [MS-RDPBCGR], [MS-RTASPF] PDU/session lifetime/remote-control permission model and sharer policy across all Windows builds not exhaustively reviewed.
- **AS-U5** Exact Roster version and ordering/race for media joining/connected/removed/user deleted, late accepted SDP and concurrent re-INVITEs need lower MS-CONFBAS source and RFC3264.
- **AS-U6** Desktop-share capability, singleApplication restrictions and attendee permission scope depend on conference MCU product/profile, **not proof every SfB Windows Full Client supports both UI modes**.

**Overall research INCOMPLETE. No protocol implementation, connection to genuine server, interoperability testing, main merge or release.**
