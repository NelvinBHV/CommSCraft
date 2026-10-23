# MS-CONFAS 12.0 application sharing: SDP sharer/viewer, RDP payload, handoff and policy failures

**2026-10-23 primary-source research, incomplete, no protocol implementation.** Microsoft original [MS-CONFAS] *Centralized Conference Control Protocol: Application Sharing Extensions*, **rev12.0, v20250520 (May 20 2025)**, exact PDF SHA256 `659ce74ebe8039b04f3bc8b95a7f007bb7675abe471747339633bbbe8f1cc6be`, official source https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-CONFAS/%5BMS-CONFAS%5D.pdf. Original §§1.3–1.7,2.2.1–2.2.2,3.1,3.2.5.1–3.2.5.2,3.3.1.1,3.3.5.1–3.3.5.7.3 and examples §§4.1–4.2 inspected; PDF pages 15/17/24/26 rendered. Cross-source [MS-CONFBAS] rev15.0 §§2.2,3.4–3.5,3.11; [MS-CONFPRO] rev13.0 §§2.2.2.1,3.1; [MS-SDPEXT] rev16.0 §3.1.5, [MS-RDPEMC] (RDP Multi-party Conferencing), [MS-RDPBCGR] (RDP Basic Connectivity and Graphics Remoting), [MS-RTASPF] (RTP application sharing packet framing). IETF original [RFC3264](https://www.rfc-editor.org/rfc/rfc3264) §8.1 (reject m-line with port zero), [RFC4028](https://www.rfc-editor.org/rfc/rfc4028) (SIP session refresh), [RFC4575](https://www.rfc-editor.org/rfc/rfc4575) conference notifications. **P** original clause/actor, **R** RFC, **S** cross-layer inference, **U** unknown behavior; every capitalized normative word from its source actor.

## A. Different permissions, transport and actual media readiness

| ID | Source | Normative state/format constraint |
|---|---|---|
| AS-01 | MS-CONFAS §§1.3,3.2.5.1.1 | Application Sharing MCU (**ASMCU**) is a distinct modality from conference Focus, IM and AV MCUs. To construct media INVITE UAC has **already joined Focus and subscribed to conference state**; retrieves ASMCU-specific `MCU-Conference-URI` from conference document `conf-uris/entry` with **purpose=applicationsharing**. No proof SIP Focus membership alone makes sharing active. |
| AS-02 | §3.2.5.1.2 | Client's initial SDP offer selects sharer or viewer via **`a=x-applicationsharing-role:sharer|viewer`** on media instance. MCU's answer **MUST contain reverse role**; offered sharer → answer viewer, vice versa. Source text elsewhere says `sharing/viewing`; exact attribute token mapping must follow MS-SDPEXT grammar and examples, not improvise values. |
| AS-03 | §3.2.5.1.2 | Maximum **one sharer m-line and one viewer m-line** per client negotiated; MCU **MUST reject additional media instances with `port=0`** in their SDP descriptions, under RFC3264 §8.1. Distinct from rejecting entire conference join. |
| AS-04 | §3.2.5.1.2 | Initial SDP lacking **both** `m=applicationsharing` and `m=video` ⇒ reject **488 Not Acceptable Here**. An offer with only `m=video` requires independent modality/profile rules and does **not** automatically prove RDP sharing. |
| AS-05 | §3.2.5.1.2 | If `m=video` offered, SHOULD label **`applicationsharing-video`**; MUST specify sendonly or recvonly. If also `m=applicationsharing`, video direction MUST match sharing role: sharer → `sendonly`, viewer → `recvonly`; mismatch => entire offer **488**. |
| AS-06 | §§3.2.5.1.4,3.3.5.2.3; RFC3264 §8.1 | Renegotiating sharer↔viewer or media replacement uses **SIP re-INVITE on existing ASMCU dialog** with **new `m=applicationsharing` line**, old one disabled with `port=0`. MCU correlates new media instances then answers **200 with SDP**. This is not a new Focus addUser for each role change. |
| AS-07 | §§3.2.5.1.6,3.3.5.7.3 | Optional `a=x-capabilities:capability1="sendrecv";...` SDP must be syntactically correct with name=value pairs, specified casing/token character limits. Malformed capability line => **488**, successful re-INVITE can generate C3P media-capabilities notifications. |
| AS-08 | §3.3.1.1 | MCU correlates conference media instance, endpoint roster media element and SDP `m=` line. `sharer` **MUST map to roster `status=sendonly`**, `viewer` **MUST map `recvonly`**. `media-content-flow` may be sendonly, recvonly or inactive and is **a different attribute**. |
| AS-09 | §3.2.5.2.1; MS-RDPEMC §2.2.4.1 | RDP over RTP application sharing: attendee connected PDU `GroupId` **MUST be numeric 3766552189**. Not an arbitrary user group/roster group ID or SIP Conference-Id. |
| AS-10 | §§3.2.5.2.2,3.3.5.3.1 | RDP `FriendlyName` in Participant-Created PDU **MUST match** user@host of endpoint-uri for conference user; compare against Focus/MCU-authorized endpoint information, not trust an arbitrary user display name. |
| AS-11 | §3.3.5.4 | ASMCU initially sends Focus a **full** user roster state after SIP 200 SDP answer, signifying signaling dialog connected. After SDP negotiation it **MUST send `media-state=joining`**; only **after RDP connection established** (per MS-RDPEMC) is `media-state=connected` notification **SHOULD**. A 200 SIP or SDP accepted is **not** proof usable desktop frames. |
| AS-12 | §§3.3.5.4–5 | Disconnect of an application sharing media instance signaled through roster **omission of the `media` element**, not necessarily removal of conference `user` or `endpoint`. Correlation uses `session-id`, endpoint-uri and media type. Client must not infer global Focus user logout from missing sharing media. |
| AS-13 | §§3.2.5.1.5,3.3.5.6 | **Single active sharer** at ASMCU: when another client negotiates sharer role, MCU **MUST eject previous sharer** from sharing by SIP **BYE** with `ms-diagnostics-public:21000;reason="New sharer joined conference"`; roster notifies new media joining, old missing media. This BYE terminates **that ASMCU dialog**; **does not automatically mean Focus membership is terminated** (cross-spec S). |
| AS-14 | §3.3.5.7.1 | MCU MAY publish permission `AllowUserToScheduleMeetingsWithAppSharing`; false means **MUST reject** incoming sharing conference INVITEs with `ms-diagnostics-public:21001` and documented attendee restriction reason. This is **conference policy**, not SIPAE auth expiry. |
| AS-15 | §3.3.5.7.2 | When CONFPRO `server-mode` **not equal "13" or omitted**, MCU MUST reject incoming sharer SDP offers for conference attendee-role users, diagnostic **21002**; conference roster `AttendeesCanShare` true **only if server-mode=13**. This is an explicit **historical version/deprecation friction**: CONFPRO §§2.2.2.7–8 says server-mode no longer used for MCU types. **Do not generalize all server-mode logic or infer attendee share permission solely from version fields.** |
| AS-16 | §3.3.5.7.3 | `supports-application-desktop-sharing` capability values **desktop|singleApplication|none**. With singleApplication client **MUST NOT** initiate whole-desktop sharing; none forbids sharing; desktop permits desktop/application sharing under other role/authorization gates. Permission and capability independent. |
| AS-17 | §§3.2.2,3.3.2,3.3.6 | MS-CONFAS defines **no app-sharing-specific timers** in its protocol actor details. **SIP session timers (RFC4028), ICE consent (MS-ICE2) and RDP/transport keepalives** remain separate; no universal 10s or 30s sharing connected deadline is proven by this document. |
| AS-18 | §3.2.5.1.2 + MS-SDPEXT §3.1.5.9 | Video screen-sharing `m=video` is an **optional parallel media modality** with matched direction and label; not same as RDP-based `m=applicationsharing`. Original MS-SDPEXT says initial audio/video SDP should not assume TCP ICE path (application sharing TCP only separately). |

## B. Controlled sharer handoff (multi-state synthesis, not arbitrary network trace)

```mermaid
sequenceDiagram
 participant A as Old Sharer A
 participant B as New Sharer B
 participant M as ASMCU
 participant F as Conference Focus
 Note over B,M: Both clients previously joined Focus and have ASMCU session
 B->>M: re-INVITE, new m=applicationsharing sharer; old m-line port 0
 M-->>B: 200 SDP opposite role viewer
 M->>F: MCU roster media-state joining for B
 M->>A: BYE + ms-diagnostics-public 21000
 M->>F: MCU roster no active media for A
 Note over B,M: RDP session proceeds separately from SIP accepted state
 M->>F: MCU roster media-state connected for B (SHOULD after RDP)
 Note over F,B: Focus notifies authorized conference subscriptions
```

The document's §3.3.5.6 narrates media notification sequencing, **not** a strict mandated total order for network-delivered SIP BYE, F→B subscription notifications, ICE selected candidate or MCU 200. The Focus's roster aggregation and ordering depend on distinct C3P dialogs and network delay. This diagram is conceptually causally ordered, not evidence from a genuine production trace.

## C. Error and security trust boundaries

| Condition | Exact response/diagnostic | Layer and scope |
|---|---|---|
| Offer lacking both recognized media m-lines | SIP **488** | Application Sharing MCU SDP validation, not 403 lobby |
| Wrong video share direction | SIP **488** | MCU offer consistency |
| Bad `a=x-capabilities` grammar | SIP **488** | Capability serialization failure |
| Too many sharer/viewer media descriptions | SDP answer `m=` **port 0** for extra lines | Partial media offer rejection, not necessarily overall SIP failed |
| New sharer arrives | SIP **BYE** with diagnostic **21000** | Old ASMCU media/dialog ejected, Focus conference still independently tracked |
| Appsharing meeting not allowed | `ms-diagnostics-public:21001` | Conference/admin policy |
| Attendee attempts to become sharer under configured server-mode | `ms-diagnostics-public:21002` | Participant role and policy |
| `singleApplication` capability | Desktop-sharing initiation forbidden | Negotiated UI/media capability |
| SDP 200 but no RDP connection | Roster may be `joining`, not connected | Media reachability, consent and RDP setup separately |

**Trust constraints:** SIPAE signer validation, valid conference Focus/ASMCU URI from roster, role, meeting privacy policy and RDP packet/session integrity all matter. A spoofed `Ms-Sender`, `endpoint-uri`, `GroupId`, `FriendlyName` or `session-id` must not override authenticated conference identity. MS-RTASPF, MS-RDPEMC and full RTASPF/RDP encoding need direct original analysis; RDP over RTP is not interchangeable with generic browser/WebRTC screensharing.

## D. Normative edition conflicts and unresolved primary work

1. **AS-X01 (server-mode)** CONFPRO rev13 says `server-mode` should not control MCU availability; CONFAS rev12 §3.3.5.7.2 **still conditions attendee share permission on server-mode=13**. These are *different features* and can both be true; full provisioned source and product note flags must be read before claiming a conflict or replacing the rule.
2. **AS-X02 (viewer/sharer token text)** MS-CONFAS §3.2.5.1.2 mentions extra media with role as `"sharing"`/`"viewing"` but mandatory instructions spell `"sharer"`/`"viewer"`. Consult MS-SDPEXT formal SDP ABNF and complete examples; do not send nonstandard token guessed from prose discrepancy.
3. **AS-X03 (connected semantics)** `INVITE 200`, `media-state=joining`, `media-state=connected`, ICE checks/consent success, RDP session connected, SRTP packet authentication are **distinct**. The exact order across streams/forks needs diagrams grounded in MS-RTASPF/MS-ICE2/MS-CONFBAS.
4. **AS-X04 (sharer switch)** Old sharer SIP BYE diagnostic 21000 should not propagate as Focus BYE, user signout or all conference modalities terminated without source; separate concurrent application-sharing video and RDP session cleanup.
5. **AS-X05 (unknown parameters)** RDP `GroupId=3766552189` and FriendlyName matching are partial handshake, not complete authentication/RDP session security. Read MS-RDPEMC, MS-RDPBCGR, MS-RTASPF and potential MS-RTASPF 2025 product notes. This profile may have legacy security/deprecation conditions.
6. **AS-X06 (encryption protocol overlap)** Sharing RTP might use RTP/SRTP on negotiated media, but **2025 MS-SRTP SHA1/256 vs MS-SDPEXT suite** conflict remains **BLOCKED**. Do not select media transform from an offer/answer label alone.
7. **AS-X07 (release/product notes)** Need exact Windows client (SfB 2015, 2016, newer LTSC) and server (on-premises, edge, MCU, firmware) behavior notes for legacy `video-screen-sharing-enabled`, `AttendeesCanShare`, sharer selection and policy diagnostics.
8. **AS-X08 (RFC mismatch)** RFC3264 port-zero SDP and RFC4028 SIP dialog timer are generic IETF rules. Microsoft SDP role/status labels, MCU C3P roster notifications and RDP payload are separate extensions; RFC4575 describes base event documents only.

**Status incomplete, all implementation planning CLOSED until full original cross-spec analysis. No protocol code, live-server connections/interoperability tests, main merge or release.**