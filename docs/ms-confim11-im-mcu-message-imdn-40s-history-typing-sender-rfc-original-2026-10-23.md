# MS-CONFIM rev11 / original RFCs — conference IM, MCU forwarding, IMDN and INFO state machines

**CommSCraft research-only original-source audit 2026-10-23. Scope:** Microsoft **[MS-CONFIM] Centralized Conference Control Protocol: Instant Messaging Extensions**, revision **11.0** / `v20250520`, released **2025-05-20**, original PDF SHA-256 **a0ec8195aeca242aa926bce23aaaec12ffefbeb27484f43a0326db952c9e4614**, official [Microsoft original PDF](https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-CONFIM/%5BMS-CONFIM%5D.pdf), Microsoft [current source/previous versions listing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confim/6f14d1ec-c854-4360-bdda-a6c61a655a60). Primary **§§1.2–1.7,2.1–2.2.6,3.1–3.4.2.7**, §4 examples selected, §5 and §6 schemas selected; **local original PDF pages 13,14,17,19,21–26 visually rendered**, including the editorial inconsistency in §3.4.1.5.2 p26. Only clauses represented by the numbered findings below are qualified as checked; historical version equivalence and full schema/XSD edge cases remain unverified. **No code, no real server, no interoperability test.**

**Status terminology:** **P** = direct Microsoft source for stated role/condition; **R** = RFC original; **S** = inference composing independent protocols, not mandatory network trace; **U** = source ambiguity/edition/client-build unknown. MUST vs SHOULD vs MAY preserved, actor-specific. **An RFC-valid behavior is NOT automatically Microsoft's conference IM profile.**

## 1. Exact normative and informative reference set — not generic latest RFCs

**Direct normative [MS-CONFIM] §1.2.1:** **[MS-CONFBAS]** (conference Focus/C3P/roster), [RFC2045](https://www.rfc-editor.org/rfc/rfc2045) MIME multipart transfer encoding (November 1996), [RFC2046](https://www.rfc-editor.org/rfc/rfc2046) MIME media type and multipart/alternative (November 1996), [RFC2119](https://www.rfc-editor.org/rfc/rfc2119) requirements keywords (March 1997), **[RFC2976](https://www.rfc-editor.org/rfc/rfc2976)** legacy SIP INFO (October 2000), **[RFC3261](https://www.rfc-editor.org/rfc/rfc3261)** SIP (June 2002), **[RFC3265](https://www.rfc-editor.org/rfc/rfc3265)** SIP events (June 2002), **[RFC3311](https://www.rfc-editor.org/rfc/rfc3311)** UPDATE (September 2002), **[RFC3428](https://www.rfc-editor.org/rfc/rfc3428)** SIP MESSAGE (December 2002), **[RFC4028](https://www.rfc-editor.org/rfc/rfc4028)** SIP session timers (April 2005), **[RFC4566](https://www.rfc-editor.org/rfc/rfc4566)** SDP (July 2006). **Informative [MS-CONFIM] §1.2.2:** **[MS-CONFAV]** (A/V counterpart), **[RFC5234](https://www.rfc-editor.org/rfc/rfc5234)** ABNF (January 2008).

**Edition warnings:** IETF **RFC2976 is superseded by [RFC6086](https://www.rfc-editor.org/rfc/rfc6086)** (2011), which *retains a legacy INFO mode*; **RFC3265 superseded by [RFC6665](https://www.rfc-editor.org/rfc/rfc6665)** (2012). RFC3428 has later update RFC8591, but the Microsoft original still expressly cites **RFC3428**; likewise RFC4566 predates RFC8866. Don't silently replace historical wire negotiation or Microsoft BENOTIFY with modern SIP Events/INFO packages. **MS-CONFIM itself has no standalone transport, private PKI, direct MSRP session, XCCOS room syntax or WebRTC data channel.**

## 2. Role-qualified wire, state, timing and delivery normative findings

| ID | Source | Exact behavior and consequence |
|---|---|---|
| CI-01 | MS-CONFIM §§1.3–1.5,3.1 | **Separate SIP session to Focus + subscription** established per **MS-CONFBAS**, and **per-media SIP INVITE session to IM MCU**. A Focus C3P addUser result does not imply IM MCU dialogue established, and audio MCU readiness does not imply IM MCU readiness. |
| CI-02 | §§2.1,3.1 | Exchange is SIP over source-permitted **TCP/TLS**, with client/MCU dial-in or MCU dial-out INVITE; UAC/UAS **transaction roles reverse**, so “all INVITEs are initiated by desktop client” is wrong. Server protection/identity requirements inherited from SIPAE/CONMGMT, not specified here. |
| CI-03 | §§2.2.5,3.1.1.1 | SIP SDP media line **MUST be `m=message 5060 sip null`**. SDP receiver **MUST ignore port 5060**. This is symbolic conference IM media description, **not** authorization to open a UDP/TCP socket to remote port5060 as media transport. |
| CI-04 | §§2.2.5,3.1.1.1 | SDP media-level **`a=accept-types:` MUST be associated with `m=message`** and is required in offer/answer exchange to convey supported formats. If omitted, support for `text/plain` is implied; even with advertised list `text/plain` always implicitly supported. |
| CI-05 | §3.1.2.5 | **Client MUST NOT send accept-types=`*`**. If client **receives server SDP offer with `*`**, source interprets server as accepting all original offered content types; this asymmetric wildcard acceptance is not license for a client to propose `*`. |
| CI-06 | §§2.2.1,3.1.2.5.2,3.1.3.5.2 | Client parses **conference-info endpoint-capabilities/supported-im-formats** per participant and SHOULD select common formats. MCU records per-endpoint SDP `accept-types`, can publish it into roster, with `text/plain` fallback for non-ms-sender clients and missing attribute. Capability state is not global per SIP user or all MCU modalities. |
| CI-07 | §§2.2.3,3.1.1.2,3.1.3.5.2 | **`Supported: ms-sender`** in INVITE or 200 OK from endpoint announces support for **`Ms-Sender` header**, which conveys original author URI on forwarded MESSAGE/INFO. Microsoft client **SHOULD support** feature; a receiving endpoint's `From` is typically MCU conference URI, **NOT** original author identity. |
| CI-08 | §§2.2.6,3.1.2.4,3.1.3.4.1 | MCU **SHOULD include** `Ms-Focus-Uri` with conference URI in outbound INVITE; **client MUST ignore the header** if present in any SIP request/response. This is compatible sender/receiver asymmetry, not contradictory role MUSTs. |
| CI-09 | §§3.1.2.4.1,3.1.3.4.1 | Client dial-in INVITE Request-URI MUST target **IM MCU's conference-URI**; MCU dial-out is higher-level **addUser** by Focus. Roles and destination are per IM MCU, not a generic IM contact SIP URI. |
| CI-10 | §§3.1.2.5.1,3.1.3.5.2 | On established IM MCU INVITE session client **MUST notify higher layer**. MCU **MUST publish endpoint-capabilities** into conference state notifications and replay applicable early-message history to joining endpoints. Conference metadata and dialog establishment are distinct transitions. |
| CI-11 | §§3.2,3.2.1.4.1,3.2.1.5.1,3.2.2.4.1 | Client leaving IM conference MUST **BYE IM MCU session**. For server-initiated C3P `deleteUser`, MCU MUST send BYE. Client's IM MCU session termination MUST notify higher layer; MCU MUST issue endpoint-deleted conference-state notification. **BYE IM session ≠ unregister SIP account, delete whole Focus conference, or tear down all other A/V data modalities.** |
| CI-12 | §3.2 intro, RFC4028 | Both MCU and client SHOULD use SIP Session-Expires extension to detect stale INVITE dialog; **§3.2.1.2 client timers 'None' refers only additional timers in this profile**, not RFC3261/4028 core transaction/dialog timers. |
| CI-13 | §§3.3.1.4.1,3.3.1.5.1 | Client sends **SIP MESSAGE on established IM MCU INVITE dialog**; this is allowed by RFC3428 though generic MESSAGE **does not itself open a dialog** (RFC3428 §1), and differs from one-on-one MS-SIPRE peer IM. |
| CI-14 | §3.3.1.4.1 | Multi-format MESSAGE may use **MIME multipart/alternative**; **all parts MUST use `Content-Transfer-Encoding: binary`**, other encodings prohibited for this profile. Client SHOULD base formats on other participants' endpoint capabilities. |
| CI-15 | §3.3.1.5.1 | MESSAGE response **SIP 200 OK** ⇒ sender client **MUST treat** forwarding as successful to all participants. **SIP 202 Accepted** ⇒ client **MUST extract `Message-Id`**, default **0 if header absent**, start **sendMessageTimer** and await IMDN. 200 is a protocol success convention; it does not cryptographically prove recipient read UI. |
| CI-16 | §§2.2.4,3.3.2.1,3.3.2.5.1 | MCU maintains per-conference monotonically incremented integer **Current message ID**, initial value **1**; response 200/202 **MUST include `Message-Id: 1*DIGIT`**, then increment ID, and forwarded MESSAGE also carries same ID. MESSAGE `Call-ID` / `CSeq` are not substitutes for MCU message ID. |
| CI-17 | §§2.2.2,3.3.1.5.1,3.3.1.6 | IMDN is XML `http://schemas.microsoft.com/rtc/2005/08/imdn` with `message-id`, zero or more `recipient uri`, `status` SIP response integer and optional `entry` key/value diagnostics. Client correlates IMDN `message-id` to 202 header; **sendMessageTimer default 10 s**, timer expiry treated as **failure IMDN 408 for all recipients**. A late IMDN should not resurrect a completed/failed UI state without extra rule (S). |
| CI-18 | §§2.2.2,3.3.2.5.2 | IMDN **MUST contain all recipient forwarding failures**, successes MAY be listed; **no recipient entries means all forwards succeeded**. Client MUST NOT infer success/failure by number of roster participants minus IMDN entries without considering only failures guaranteed. |
| CI-19 | §3.3.1.5.2 | On MCU-forwarded MESSAGE receiver **MUST send SIP 200 OK**, inspect `Ms-Sender` if present and display actual sender. Its SIP `From` contains **conference-URI**, not the human original sender; without authenticated Ms-Sender, source falls back to format restrictions and no universal From=original user. |
| CI-20 | §§3.3.2.1–2,3.3.2.5.1,3.3.2.6.1 | IM MCU records **first 40 seconds after conference created** in temporary **MESSAGE history**. Late join during that window receives cached MESSAGE through the ordinary forwarding path; after timer expiry MUST clear list and set flag false. This is **not durable conferencing chat log**, historic Group Chat room history or XCCOS bcontext. |
| CI-21 | §3.3.2.5.1 | If sender **alone on IM MCU**, MCU MUST return **200** (no later delivery status); if others joined, MCU **SHOULD 202** then later IMDN. Source says MCU MUST forward to each *other* joined endpoint; no other recipients means no forwarding. |
| CI-22 | §3.3.2.5.1 | For endpoint **without ms-sender** and plain text, MCU **SHOULD prepend/include sender URI in body** to compensate; for non-plain content with no ms-sender SHOULD add **415 error for that recipient**. For ms-sender recipients, MCU MUST add header and may select supported multipart alternatives; if none supported SHOULD record **415** forwarding failure. |
| CI-23 | §§3.3.2.5.1–2 | MCU tracks each forwarded SIP transaction response/status **plus ms-diagnostics** separately. Only after all forwards completed by response or timeout does MCU MUST send **IMDN BENOTIFY** back to sender, with **Content-Type: application/ms-imdn+xml**; unsuccessful recipients reported by URI/status. This is logically distinct from original upstream 202. |
| CI-24 | §§3.4–3.4.2.5.2 | Typing/idle uses **SIP INFO** on the **same IM MCU INVITE dialog**, not MESSAGE. MCU **MUST return SIP 202 Accepted immediately** and forwards INFO **only** to clients advertising `ms-sender`, adding `Ms-Sender`. Forwarded INFO result or timeout is **ignored**, no IMDN or further report to sender. |
| CI-25 | §3.4.1.5.1 | Client receiving INFO 202 MUST treat as **successfully sent to MCU**, **not delivered** to recipient(s). INFO is thus weaker observable guarantee than 202 MESSAGE + IMDN. |
| CI-26 | §3.4.1.5.2 p26 | **Editorial source inconsistency**: heading and surrounding text say received `INFO`, but required reply sentence says client MUST send **200 OK to incoming `MESSAGE`**. This is **not a proven interoperable INFO response requirement** until errata/version evidence. RFC2976/6086 SIP transaction semantics suggest replying to received method (S), but never rewrite the Microsoft normative sentence as if it were explicit. |
| CI-27 | §§3.4.2.5.1–2 | MCU **MUST NOT forward INFO** to clients lacking ms-sender; when forwarding transaction later succeeds/fails/times out MCU **ignores status**, sends **no IMDN**. A downstream typing-notification UI must therefore tolerate absence even when source INFO 202 succeeded. |
| CI-28 | §§2.2.2,3.3.2.5.2 | **Source ambiguity**: IMDN `message-id` SHOULD match header, client defaults missing 202 header to **0**, while MCU MUST send header. The source does not specify uniqueness/collision recovery for multiple pending requests all defaulted to **0** on malformed/legacy path. Requirement: mark ambiguous correlation **U**, not invent correlation by Call-ID/CSeq. |
| CI-29 | §§2.2.1,3.1.3.5.2 | Endpoint capabilities **`supported-im-formats` list** and `user-agent` appear as **conference-info XML endpoint extensions**; XML schema §6.1 limits supported-im-formats to max **512 chars** and user-agent to **128 chars**. Parsing must not collapse missing to unconditional rich text support. |
| CI-30 | §5.1–5.2 | Both **“Security Considerations for Implementers: None”** and **“Index of Security Parameters: None”** in this extension mean **security inherited from RFC3261, SIPAE, CONFBAS and transport**. They do *not* authorize unauthenticated Ms-Sender, untrusted conference MCU headers, unencrypted SIP and leaking IM text. |

## 3. Distinct state machines and exact source-labeled interaction diagrams

### 3.1 IM MCU session, capability notification, application send

~~~mermaid
sequenceDiagram
 participant U as SfB Windows Full Client
 participant F as Conference Focus
 participant M as IM MCU
 participant V as Other conference client
 U->>F: Existing Focus INVITE/AddUser and SUBSCRIBE (MS-CONFBAS)
 F-->>U: Focus membership and IM MCU URI
 U->>M: INVITE with m=message 5060 sip null, accept-types, Supported ms-sender
 M-->>U: 200 OK with SDP; ACK completed per RFC3261
 M-->>U: Conference state notification, IM endpoint capabilities
 U->>M: MESSAGE on IM MCU INVITE dialog
 alt Only sender connected
  M-->>U: 200 OK; Message-Id
 else Other participants
  M-->>U: 202 Accepted; Message-Id
  M->>V: MESSAGE forwarded; Ms-Sender and Message-Id, format selected per endpoint
  V-->>M: 200 or error/timeout
  M-->>U: BENOTIFY application/ms-imdn+xml after forward completion
  Note over U: Correlate Message-Id; 10s timer if awaiting IMDN
 end
~~~

### 3.2 Counterexample: SIP INFO typing state is not IM delivery confirmation

~~~mermaid
sequenceDiagram
 participant U as Sender
 participant M as IM MCU
 participant V as Receiver with ms-sender
 participant L as Legacy receiver without ms-sender
 U->>M: SIP INFO typing/idle data on IM dialog
 M-->>U: 202 Accepted
 M->>V: Forwarded INFO plus Ms-Sender
 Note over M,L: No forwarding to client lacking ms-sender
 V-->>M: 200 or error
 Note over U,M: No delivery-status IMDN for INFO
~~~

### 3.3 Independent state variables (S; not mandatory internal object model)

| State | Unique key/scope | Transition trigger | Failure path |
|---|---|---|---|
| `FocusMembership` | conference Focus, user endpoint, C3P/roster version | addUser/deleteUser/roster | lobby/role/deletion; independent IM MCU |
| `ImMcuDialog` | SIP Call-ID plus local/remote tags to IM MCU | INVITE/200/ACK, BYE, session timer | SIP dialog lost; only IM modality marked unavailable |
| `ImFormatsByEndpoint` | conference+endpoint | SDP `accept-types`, endpoint roster changes | fallback text/plain; no invented rich text |
| `MessagePending` | IM MCU per-conference Message-Id | SIP 202 accepted, IMDN or 10s deadline | 408 synthesized on timer; ambiguous default0 |
| `RecipientForwardResult` | MCU message-id + recipient URI | downstream MESSAGE 2xx/4xx/timeout | per-recipient 415/408 and diagnostic entries |
| `EarlyMessageHistory` | MCU conference and 40s creation window | cached early MESSAGE, timer cleanup | cache dropped after 40s, not persisted |
| `TypingInfoPending` | SIP INFO transaction | 202 Accepted (MCU only), no IMDN | no recipient guarantee |
| `ConferenceSessionCleanup` | Focus and IM MCU sessions separately | BYE/deleteUser/endpoint-deleted | no account deregistration implicitly |

## 4. RFC-vs-Microsoft differences and open normative conflicts

| Finding | RFC baseline | Microsoft extension/condition | Resolution status |
|---|---|---|---|
| SIP MESSAGE dialog | RFC3428 §1 says MESSAGE alone **does not create dialog** but may be sent in existing dialog | MS-CONFIM §3.3.1.4.1 **MUST send on IM MCU INVITE dialog** | P/R clear, client build/version gate |
| SDP media | RFC4566 general media line semantics | `m=message 5060 sip null`, receiver ignores 5060, `accept-types` type list/default text/plain | P/R clear |
| SEND acknowledgement | RFC3428 SIP request transaction delivery semantics | `Message-Id` in 200/202, per-recipient failures in **IMDN BENOTIFY** | P/R clear; actual *read* receipt unproven |
| Sender identity | RFC3261 From may identify signaling endpoint | Microsoft MCU From=conference URI, original sender in optional `Ms-Sender` extension; body fallback plain text | P clear; security/trust inherited |
| MIME | RFC2045/2046 multipart allows differing body encodings | Microsoft profile demands **binary Content-Transfer-Encoding** all multipart body parts | P/R clear |
| SIP INFO | RFC2976 legacy INFO, obsoleted by RFC6086 (legacy fallback retained) | MCU 202 then **no forwarded status**, only to ms-sender capable receivers | P/R clear; RFC6086 package negotiation not proven |
| SIP events | RFC3265 now obsoleted by RFC6665 | Proprietary **BENOTIFY** used for IMDN and conference states | Version delta open; never infer modern NOTIFY automatically |
| IMDN no recipient | Generic SIP transaction doesn't require this XML | Microsoft IMDN empty list ⇒ all forwarded successfully | P, only forwarding, not user read |
| **CI-X1 INFO response typo** | RFC SIP INFO transaction semantics | §3.4.1.5.2 says reply to incoming MESSAGE in section headed INFO | **U editorial contradiction** |
| **CI-X2 default Message-Id 0** | SIP Call-ID+CSeq separate | §3.3.1.5.1 defaults missing 202 Message-Id to 0, yet server MUST provide ID | **U collision/loss policy** |
| IMDN history vs personal chat history | RFC3428 has no persistent history | 40s MCU buffer only, not a general transcript | P clear |
| Product edition | [MS-CONFIM] overview Microsoft Learn history rev **5.0 March 2015**, **10.0 August 2024**, **11.0 May 2025** | Current revision not automatic capability of 2015 Windows binary or same server topology | **U** historical diff and deployment |

## 5. Explicit research closure blockers

1. **CONFIM-U01** official errata/historical source review for §3.4.1.5.2 incorrect “incoming MESSAGE” in INFO handler. This is a literal normative wording defect visible in original PDF p26, not proof which wire response Windows desktop actually emits.
2. **CONFIM-U02** no protocol-level deterministic correlation rule for multiple 202 MESSAGEs with missing `Message-Id` all mapped to 0. Need historic errata/product note and possibly additional client-specific behavior from official documentation, *not* server interop tests by assistant.
3. **CONFIM-U03** standalone 200 MESSAGE convention says forwarded successfully, but if no other participants this is vacuous. Endpoint read or UI acknowledgement is not proven.
4. **CONFIM-U04** during 40s history window, temporal races between new join, cached message delivery, live message and partial roster ordering require source-based exact event model. Do not invent global total order for IMDN vs joined roster.
5. **CONFIM-U05** full [MS-CONFBAS] Focus/C3P admission, anonymous user, media roster and multiple endpoint behavior must be attached with version-pinned exact source subsections. Old product branch lacks proof.
6. **CONFIM-U06** 2015 historical **[MS-CONFIM] rev5.0** official Microsoft PDF exists in catalog of previous versions, but bytes/sections have **not** been hash-audited in this tranche. The 2025 rev11 profile cannot be asserted identical.
7. **CONFIM-U07** [RFC6086]/[RFC6665] modern supersession vs legacy INFO/BENOTIFY wire conditions must be pinned to Skype for Business Windows installed build; security contexts and allowed rich text MIME require source-specific presence/permission behavior.
8. **CONFIM-U08** complete appendix §6 IMDN XSD namespace/string length, invalid payload processing, formatting/MS diagnostics and XML security risks need exhaustive cross-spec closure.

**This completes a substantive original §2–3 IM MCU packet/state investigation, NOT the full client normative source analysis. Implementation gate remains CLOSED; no protocol code, live-server connections, interoperability tests, merges or releases.**
