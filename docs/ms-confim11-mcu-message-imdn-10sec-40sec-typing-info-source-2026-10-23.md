# MS-CONFIM 11.0 — IM MCU session, MESSAGE 202/IMDN, 10 s client timeout, 40 s history and INFO typing

**Source-only audit 2026-10-23; NO implementation.** Official Microsoft original: **[MS-CONFIM] Centralized Conference Control Protocol: Instant Messaging Extensions, revision 11.0, v20250520, May 20 2025**; original PDF SHA-256 **`a0ec8195aeca242aa926bce23aaaec12ffefbeb27484f43a0326db952c9e4614`** independently verified against 81-page archived PDF; visual check PDF pp13,22–27. Official PDF https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-CONFIM/%5BMS-CONFIM%5D.pdf ; Microsoft Learn [IMDN schema](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confim/48848d12-5e17-4edc-b9aa-bfc8da54abfe). Core original clauses directly examined §§2.2.1–2.2.6,3.1.1.1–3.1.3.5.2,3.2.1.4.1–3.2.2.5.1,3.3.1.2–3.3.2.6.1,3.4.1.4.1–3.4.2.5.2 and §6 schemas partially. Supporting originals [MS-CONFBAS] rev15, [MS-CONFPRO] rev14 (edition check), [MS-SIPRE] rev13, [RFC3261](https://www.rfc-editor.org/rfc/rfc3261.html) June2002, [RFC3428](https://www.rfc-editor.org/rfc/rfc3428.html) Dec2002, [RFC4566](https://www.rfc-editor.org/rfc/rfc4566.html) July2006 SDP, [RFC2045](https://www.rfc-editor.org/rfc/rfc2045.html) November1996 MIME. **P** directly in MS-CONFIM 11.0; **R** IETF original; **S** cross-spec implications; **U** target Windows client/server build and unresolved source error.

## I. Conference IM is not direct user-to-user MESSAGE

| ID | Source section | Actor and exact protocol boundary |
|---|---|---|
| CONFIM-01 | §§1.3,3 preface (P) | IM media **MCU** receives and forwards MESSAGE and INFO on separate **SIP INVITE sessions** established with conference participant. Admission, Focus, roster and conference control belong to **MS-CONFBAS/MS-CONFPRO**, not to this same MCU MESSAGE forwarding algorithm. |
| CONFIM-02 | §§2.2.5,3.1.1.1 (P) | INVITE/200 **SDP** must signal `m=message` and `a=accept-types` for IM media. Client media format capability isn't just presence status or server-wide codec capability. |
| CONFIM-03 | §3.1.2.4.1 (P) | **Dial-in** client INVITE Request-URI MUST be **IM MCU conference URI**, not the contact's personal AoR. **Dial-out** path comes from Focus addUser and MCU-originated INVITE; distinct roles and call establishment state. |
| CONFIM-04 | §§2.2.1,3.1.3.5.2 (P) | MCU publishes participant `endpoint-capabilities` in conference roster notifications including **supported-im-formats, user-agent and Ms-Sender support**. On missing accept-types or certain extension non-support, fallback SHOULD be **text/plain**. |
| CONFIM-05 | §§2.2.3,3.1.3.5.2 (P) | Microsoft **`Ms-Sender`** extension tracks actual human sender when MCU forwards a MESSAGE, as **From** on forwarded message is **conference URI**, not originator SIP URI. Client should not pretend From gives participant identity directly. |
| CONFIM-06 | §3.1.2.5.1 (P) | MCU IM INVITE session established ⇒ client MUST notify higher layer media session established. This is not proof every conference participant can receive selected rich text body or that any later MESSAGE delivered. |
| CONFIM-07 | §§3.2.1.4.1,3.2.2.4.1 (P) | Client leaves IM session with SIP **BYE**. Server-side `deleteUser` can cause MCU to send BYE to client. These are distinct control operations; **not** `REGISTER Expires=0` or deleting conference globally. |

## II. MESSAGE relay, MIME compatibility and per-recipient status

| ID | Clause | Specific state transition |
|---|---|---|
| CONFIM-08 | §3.3.1.4.1 (P) | Client sends SIP **MESSAGE on established IM MCU INVITE dialog**. It SHOULD consult all other participant capabilities; it may send **multipart/alternative**. When multipart sent, **all body parts MUST have `Content-Transfer-Encoding: binary`**, other encodings MUST NOT be used in this source profile. |
| CONFIM-09 | §§2.2.4,3.3.2.1 (P) | MCU holds **Current message ID per conference**, initialized to **1** upon creating conference MCU instance. `Message-Id: 1*DIGIT` is included in MCU response, forwarded MESSAGE and IMDN, **not SIP Call-ID, CSeq or MIME Content-ID**. |
| CONFIM-10 | §3.3.2.5.1 (P) | If sender is **only** client in MCU session, MCU **MUST return 200 OK** and there will be **no separate delivery notification**. A 200 in this special case is not evidence another remote recipient read or even exists. |
| CONFIM-11 | §3.3.2.5.1 (P) | If **other clients** connected, MCU **SHOULD reply 202 Accepted** and later send IMDN. MCU **MUST put Message-Id** in **both 200 and 202** and increment current message ID. It then **MUST forward MESSAGE to each other client**. |
| CONFIM-12 | §3.3.2.5.1 (P) | If recipient **doesn't support Ms-Sender** and incoming content `text/plain`, MCU SHOULD insert human sender URI into forwarded **body**; if non-plain body and no Ms-Sender, SHOULD record **415 unsupported media** for that recipient. |
| CONFIM-13 | §3.3.2.5.1 (P) | If recipient supports Ms-Sender, MCU MUST include actual sender URI in forwarded **Ms-Sender** header. For multipart/alternative sent to recipient that doesn't support multipart, MCU SHOULD select supported part; if none, SHOULD record **415**. Supported multipart recipient receives full multipart. |
| CONFIM-14 | §3.3.1.5.2 (P) | Recipient client receiving forwarded MESSAGE MUST send **SIP 200 OK** and use **Ms-Sender if present** to identify human original sender; incoming **From** is conference-URI. **Not a human read receipt**. |
| CONFIM-15 | §§3.3.2.5.1–3.3.2.5.2 (P) | MCU tracks **per-forward transaction response/error/timeout plus ms-diagnostics**. Once all outgoing forwarded MESSAGE transactions finished, MCU **MUST send IMDN via BENOTIFY** with `Content-Type: application/ms-imdn+xml`. |
| CONFIM-16 | §§2.2.2,3.3.2.5.2 (P) | IMDN has **message-id** and **recipient URI/status error list**, plus optional key/value entries (e.g. diagnostics). **All failed forwards MUST be in IMDN**, successful forwards **MAY** be included. **Zero recipients in IMDN means forwarded successfully to all**. Client MUST NOT rely on optional success entries. |
| CONFIM-17 | §3.3.1.5.1 (P) | **200 OK** to client's outgoing MESSAGE ⇒ client MUST treat forwarding as success **to all MCU participants** as per source. **202** ⇒ client MUST take **Message-Id** from response, with **default 0 if missing**, and wait for correlated IMDN. |
| CONFIM-18 | §3.3.1.6 (P) | Client **MUST maintain sendMessageTimer**, **default 10 seconds**. On 202 it MUST start timer; successful correlated IMDN before timeout stops it. If timer expires, client treats pending IMDN as per-recipient **408**. No sourced claim this 10 s equals SIP transaction Timer F. |
| CONFIM-19 | §§3.3.1.5.1,3.3.2.5.2 (P) | Correlation key is **MCU-assigned Message-Id in 202 and later IMDN**; Message-Id fallback zero is prescribed on absent response header, but operation for multiple *simultaneous* 202 messages missing Message-Id remains **U**. Don't silently use Call-ID or CSeq instead. |
| CONFIM-20 | §§3.3.2.1–3.3.2.2,3.1.3.5.2 (P) | MCU stores incoming messages as **first 40 seconds conference message history** to replay to newly joining clients. After 40s, history clears and **new MESSAGE not added**. The 40-second timer is **server-side** and **not persistent chatroom retention**. |

## III. Typing/idle INFO is not delivery-confirmed MESSAGE

| ID | Clause | Rule |
|---|---|---|
| CONFIM-21 | §3.4.1.4.1 (P) | User typing or idle information uses **SIP INFO** on INVITE/IM media session, not arbitrary application "presence state" publication; MCU forwards body to other clients **without interpretation**. |
| CONFIM-22 | §3.4.2.5.1 (P) | MCU MUST reply **202 Accepted** to INFO. **Forward only** to recipient participant endpoints supporting **Ms-Sender**, adding actual sender URI in **Ms-Sender** header. Must NOT forward INFO to recipient without that extension. |
| CONFIM-23 | §§3.4.1.5.1,3.4.2.5.2 (P) | Client receiving 202 treats INFO **accepted by MCU**, not final recipient delivery. MCU discards downstream INFO response/timeouts and sends **no sender notification**. INFO's 202 has a weaker delivery contract than MESSAGE's 202+IMDN. |
| CONFIM-24 | §3.4.1.5.2 (P + U) | **Source wording inconsistency:** section titled “Client Receives an INFO Forwarded by the MCU” says client **“MUST send a 200 OK response to the incoming MESSAGE request”**, although it is describing an **INFO**, and companion §3.4.2.5.1 forwards INFO. This is a probable document typo, **not authority to silently rewrite normative text**; check official errata/revision and RFC3261 transaction method semantics. Sender URI from **Ms-Sender** is a separate instruction. |

## IV. SIP conference IM state model (S; asynchronous)

~~~mermaid
stateDiagram-v2
 [*] --> ConfAdmitted: Focus admission per MS-CONFBAS
 ConfAdmitted --> McuDialog: MCU INVITE SDP m=message/accept-types and ACK
 McuDialog --> MessageSent: Client MESSAGE to MCU
 MessageSent --> DeliveredNoPeers: MCU 200 if sole participant; no IMDN
 MessageSent --> WaitingIMDN: MCU 202 and Message-Id, start 10s timer
 WaitingIMDN --> ResultSuccess: Matching IMDN BENOTIFY, no recipient errors
 WaitingIMDN --> ResultPartial: Matching IMDN, per-recipient 4xx/5xx/errors
 WaitingIMDN --> ResultTimeout: 10s; synthetic delivery 408
 McuDialog --> TypingInfo: INFO typing/idle
 TypingInfo --> McuDialog: MCU 202 only, no delivery callback
 McuDialog --> Leaving: Client or server BYE
 Leaving --> [*]
~~~

**Server-side parallel state**: per MCU conference message-ID counter, per-participant content-type and Ms-Sender flags, per-forward SIP transaction status, and separate **40s initial-history** lifetime. It can be valid that the MCU's **SIP 202** has happened while one forward fails, a 415 is recorded, and a later IMDN informs the sender. No global single `messageDelivered` boolean captures all this.

### End-to-end sample exchange categories (not invented literal header values)

~~~mermaid
sequenceDiagram
 participant Sender as IM sender UAC
 participant MCU as Conference IM MCU
 participant Bob as Participant B
 participant Carol as Participant C
 Sender->>MCU: MESSAGE on IM dialog; MIME negotiated
 MCU-->>Sender: 202 Accepted + Message-Id N
 MCU->>Bob: MESSAGE + Message-Id N + Ms-Sender Sender
 MCU->>Carol: MESSAGE + Message-Id N + Ms-Sender Sender
 Bob-->>MCU: 200 OK
 Carol-->>MCU: 415 Unsupported Media Type
 MCU-->>Sender: BENOTIFY application/ms-imdn+xml message-id N, Carol error 415
 Note over Sender,MCU: Client correlates N before 10-second timer
~~~

## V. Source/edition conflicts and unresolved cases

- **CIM-X01:** §3.4.1.5.2 inbound **INFO** section says 200 to inbound **MESSAGE**. Need official erratum; treat as unresolved normative typo, not permission to answer to a *different* SIP transaction.
- **CIM-X02:** §3.3.2.5.2 after first "**MUST** add message-id" says "**SHOULD** add message-id", redundant/inconsistent language. Must inspect revision history and which envelope is intended before asserting an extra independent header requirement.
- **CIM-X03:** Section 3.3.1.5.1 defaults absent Message-Id to **0** for 202; concurrent 202 responses with absent header could share correlation key, no dedup algorithm given. Mark race **U**.
- **CIM-X04:** Source **200 means accepted by MCU/no other client** or 200 client treats forwarded all; **RFC3428 §4** direct SIP 200 to MESSAGE is a different application context and *does not mean read*. Never merge all statuses into one user-visible "delivered/read" concept without feature spec.
- **CIM-X05:** MCU **40-second history** is for *conference join race only*. Persistent GroupChat [MS-XCCOSIP] and EWS history have **different retention and access control** and should not be inferred from this.
- **CIM-X06:** **Multiple simultaneous messages**, Message-Id collision/rollover, group admission/ejection race, BENOTIFY loss on transport, duplicate IMDN and timeout interplay with SIP 202 need exact client behavior/underlying [MS-CONFBAS] state and RFC transactions.
- **CIM-X07:** MIME `multipart/alternative`, rich-text `text/rtf`, image/gif, ink and capabilities are source-supported examples but **not universal mandatory support**. Client/server 2015/2016 edition, policies, content sanitization and partial support remain U.
- **CIM-X08:** Original PDF has **81 pages**; only selected §§1–4 and §6 examples/XSD were reviewed, not a full field-by-field XSD audit. Recheck source edition consistency with MS-CONFBAS 2025 and historical [RFC4566] when establishing all modality sessions.

**Research status NOT COMPLETE; no protocol code, connections to actual servers, interop tests, main merges or releases.**
