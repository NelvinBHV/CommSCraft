# MS-XCCOSIP rev7 — SIP INFO persistent chat commands, asynchronous XML correlation and file-token security

**Original-clause research 2026-10-23. STATUS: selected-clause direct audit, not all 91 pages or Windows binary behavior.** Microsoft primary **[MS-XCCOSIP] Extensible Chat Control Over Session Initiation Protocol (SIP)** revision **7.0**, dated **2024-08-20** (`v20240820`), local original SHA-256 **dedf341e7f1031453bab3bc5a196a8563ad7381895e0458eb51bf870e270077e**, official Microsoft PDF https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-XCCOSIP/%5BMS-XCCOSIP%5D.pdf . Direct original §§2.1,2.2.2.1.27–29,2.2.2.2.1–.9,3.1.1,3.1.2.1.1–7,3.1.5,3.1.6,3.1.7,3.1.13–.15,3.1.16,3.1.19,3.2.1,3.2.4–.6; source PDF pp **35,44,45** rendered and inspected. Actual original direct RFC citations **[RFC2119](https://www.rfc-editor.org/rfc/rfc2119)**, **[RFC3261](https://www.rfc-editor.org/rfc/rfc3261)**, **[RFC6086](https://www.rfc-editor.org/rfc/rfc6086)** SIP INFO package; Microsoft [MS-PRES], [MS-SIP], [MS-SIPRE], [MS-SIPREGE]; overview [MS-OCSPROT] §2.5.21 legacy client feature notes <32>. **P** Microsoft direct actor rule; **R** IETF specification; **S** cross-protocol inference; **U** unresolved product/edition/conflict.

## I. Outer SIP transaction is not the inner XCCOS command result

| ID | Clause | Exact state, wire fields and error rule |
|---|---|---|
| XCC-01 | §2.1; MS-OCSPROT §2.5.21 | Chat messages are carried by **SIP over TLS**, the protocol defines XML `xccos` command/reply/notice semantics but **no new transport**. SIP `INVITE/ACK` creates the chat server dialog; SIP `INFO` conveys XCCOS data. SIP INFO `200 OK` proves SIP request handling only, not a successful channel join/post. |
| XCC-02 | §2.2.2.2 | Five control primitives: client **`cmd`**, server response **`rpl`**, asynchronous server **`ntc`**, error **`err`**, system notification **`sys`**; `grpchat` is a sixth, historically separate, bidirectional message rather than a command/reply primitive. |
| XCC-03 | §2.2.2.2.1–3 | XML root `<xccos ver="1" envid="...">`, namespace **`urn:parlano:xml:ns:xccos`**; `envid` monotonically increases per envelope, `seqid` identifies primitive **within envelope**. The key is **(EnvId, SeqId)**, modelled by **two 64-bit unsigned integers**. A SIP Call-ID or CSeq is **not** a substitute for XCCOS command ID. |
| XCC-04 | §3.1.2.1.3 | Multiple commands MAY occupy the same `xccos` envelope, but each **MUST have unique seqid inside that envelope**. Client creates PendingCommand with CmdId copied from parent envid and cmd seqid. Pending state terminates only by matched reply, timeout, lower-layer failure or local cancel. |
| XCC-05 | §§2.2.2.2.2–4 | Command `id` chooses operation (e.g. `cmd:join`, `cmd:bjoin`, `cmd:bccontext`, `cmd:grpchat` is **not** a generic cmd, rather standalone `grpchat`). Server reply type MUST semantically match command, e.g. **`cmd:join` ↔ `rpl:join`**, not `rpl:bjoin`. `commandid` contains both original envid and seqid. |
| XCC-06 | §2.2.2.1.27 | Inner XML `<resp code=100..699>` is **HTTP-like six-class status**, NOT an HTTP network response or SIP status merely because number is 200 or 4xx. `code` attribute is required in valid ResponseBlock; specific error semantics depend on command and server. |
| XCC-07 | §3.1.2.1.5 | Client MUST validate received XCCOS XML schema, drop invalid. Every incoming `rpl` MUST have **commandid seqid+envid**, match a Pending CmdId, have a response code and **reply id corresponding to originating command**; else **DROP**. Only then cancel command timer and pass to handler. **Inner response code exactly 200** = command success; **any other value** = failure. |
| XCC-08 | §§3.1.2.1.2,.6,.7 | Each outgoing **command** starts an independent **10-second `XccosTransactionTimer`**. Timeout terminates PendingCommand and triggers command-specific error cleanup; failed containing SIP INFO terminates PendingCommand too. Higher-layer cancellation terminates it and suppresses processing of late reply. Source provides **no universal automatic resend** upon timeout. |
| XCC-09 | §2.2.2.2.6; §3.2.1.5 | `err id="error"` primitive references a **seqid** and optionally `commandid` for failed command; server MUST use XccosErrorPrimitive for malformed schema/unsupported command, correlating CmdId. This **does not automatically mean a SIP 400** is sent for every malformed embedded XML. |
| XCC-10 | §§3.1.5.3,.5 | Single-room join MUST use `cmd:join` with **one data block containing exactly one ChannelIdsInformationDataBlock**, channel GUID and server domain. Successful inner reply is parsed for room participants, display name/description and active user IDs mapped to SIP URI; a failed reply MUST NOT mutate participant list. |
| XCC-11 | §§3.1.6.5;3.1.1 | Batch-room join processes multiple ChannelInformationDataBlock items to corresponding `ParticipantList` per room; client SHOULD maintain **ParticipantList** and **LastReceivedMessageId** *separately for each channel*. One global participant list/chat counter corrupts other rooms' history. |
| XCC-12 | §§3.1.7.3–.5 | Chat history `cmd:bccontext` requests **one channel uri** and `bcquery.last cnt`. When the channel window is actively open, **cnt SHOULD 25**; when not actively open **SHOULD 1**. On successful reply, **returned URI MUST match requested URI** or result dropped; client SHOULD advance LastReceivedMessageId only on greater `chatId`. |
| XCC-13 | §§2.2.2.2.9,3.1.13.1–6 | `grpchat` has required `id="grpchat"`, `seqid`, `chanUri`, `author`, `authdisp`, `alert`, `chatId` (client may send **0**, server assigns), ISO8601 `ts`, mandatory child **plain `chat`**, optional `rtf` and `originatingMessageId` in received copies. This is a separate send ledger from CmdId. |
| XCC-14 | §§3.1.13.2,.6; 3.1.14.5 | Each **sent groupchat message** has **10s GroupChatSendTimer**. Timer expiration => sending failed and client SHOULD display error. **Incoming grpchat** with originatingMessageId matching MsgId(EnvId,SeqId) raises **SentGroupChatReceived** for that pending post. Unmatched incoming messages may still display normally as other authors' posts. |
| XCC-15 | §§2.2.2.2.5,3.1.15 | Server **`ntc`** primitives are **asynchronous**, containing required `data`; types include room join/part, kick, permission/participant changes, invite, room info changes and quit. They are not always tied to any PendingCommand, and may arrive in a different order from one-shot `rpl`. |
| XCC-16 | §3.1.16.5 | Room permission query success includes ChannelInformationDataBlock/UserInformationBlock with **`chperms`**; no permission should be inferred just because `cmd:join` had a SIP 200 or because channel URI is known. |
| XCC-17 | §§2.2.2.2.2,3.1.19.3,.5 | File web-service transfer token is **scoped to a file in a specified room and short lifetime**; download/upload requests send file token fields, successful reply MUST include **nonempty token** plus **serveruri**, otherwise reply dropped. Only the provided server URI and token go to Group Chat File Repository service; this is not proof that an arbitrary returned URL should be trusted without TLS/host validation. |
| XCC-18 | §3.1.19.3 vs §2.2.2.2.2 | **Internal primary-source inconsistency**: `cmd:getfutok` is listed for **upload** and `cmd:getfdtok` for **download** in command enum (§2.2.2.2.2); §3.1.19.3 says **`cmd:getfutok` for both upload and download**. This is **U**, not authority to silently substitute one command or assert one must work in a target 2015 client. |
| XCC-19 | §3.2.1.5 | **Server** must validate request command XSD and supported `cmd id`, replying with `err` referencing CmdId on invalid/unsupported cases; client MUST NOT generate a fictitious `rpl:join` success when SIP INFO 200 is received before server result. |
| XCC-21 | §§3.2.1.2,.6 | **Server** has a **25-second default** processing timer, independent of client's **10-second** XccosTransactionTimer. Server timeout **MUST yield SIP 503 for pending INFO request**. A reply after the client command has already timed out or been canceled is not allowed to revive pending application state; this is a **timer-race cross-protocol inference (S)**. |
| XCC-20 | §2.2.2.2.9; §3.1.14.5 | **S security interpretation:** chat author and timestamp appear in submitted payload, but server-supplied `chatId`/identity and authenticated SIP channel are distinct. Do not treat client-set author/chatId=0 as authoritative audit metadata; validate server-origin message and room membership under lower SIP auth/trust rules. |

## II. Two concurrent acknowledgement layers (S)

~~~mermaid
sequenceDiagram
 participant W as Windows client
 participant P as SIP Front End
 participant G as Persistent Chat server
 W->>P: SIP INVITE Group Chat
 P-->>W: 200 OK / ACK completes SIP dialog
 W->>G: SIP INFO with xccos envid 40, cmd join seqid 1
 G-->>W: SIP 200 for INFO transaction
 Note over W,G: Still awaiting XML command result
 G->>W: Separate SIP INFO with rpl join / commandid envid 40 seqid 1 / resp code 200
 W-->>G: SIP 200 for incoming INFO
 Note over W: Mark room joined only after correlated successful XML reply
 W->>G: SIP INFO containing grpchat seqid 2 / chatId 0
 G-->>W: SIP 200 for posted INFO
 G->>W: grpchat carrying assigned chatId and originatingMessageId
 W-->>G: SIP 200
 Note over W: Match sent MsgId independently of pending cmd ledger
~~~

**This is a schematic selected success path grounded in XCCOSIP §§3.1.2/3.1.13–14 and MS-OCSPROT §2.5.21, NOT guaranteed exact 2015 wire capture.** Incoming notices, history and token paths can occur concurrently; a message returned on a SIP transaction has separate application-level validation and timeouts.

## III. Independent state/error matrix (S)

| Layer | Key and scope | Expiry/failure | Do not equate with |
|---|---|---|---|
| SIP dialog | RFC3261 Call-ID + tags / routes | SIP timeout/BYE/re-INVITE | XCCOS command data status |
| SIP INFO transaction | RFC3261 CSeq and SIP response | SIP INFO failure ends embedded pending command | XML `resp=200` |
| XCCOS command | `CmdId=(EnvId,SeqId)` within room/control | 10s timeout, server may still have 25s processing; invalid/foreign `rpl` dropped, any non-200 inner status fails | Distinct chat MsgId confirmation |
| Chat message | `MsgId=(EnvId,SeqId)`, server assigned chatId | 10s SendTimer → user error; origin echo matches | Room join/capability/permission |
| Room membership | channel URI + ParticipantList + independent chatId cursor | `ntc:kick` / `ntc:part` etc server updates | Personal IM MESSAGE recipient |
| Room file transfer | per-channel file token + URL + server URI | short token lifetime; invalid/empty token reply ignored | TURN relay or PSOM file AES key |
| User login | registered SIP principal and auth context | MS-SIPAE and SIPREGE rules | Any room/room server still available |

## IV. Normative/version gaps still open

- **XCC-U01:** Resolve command conflict `getfutok` vs `getfdtok` by checking source errata, historical editions and target client variant; cannot treat example as conformance test.
- **XCC-U02:** Original complete ~91-page XML schemas for ACL `ace`, query/search pagination, mutations, user preferences, notification ordering and room admin roles **not exhaustively reviewed** in this tranche.
- **XCC-U03:** Confirm RFC6086 INFO package negotiation and deployed SIP transport/TLS errors in MS-SIPRE; cannot assume any SIP server supports Chat XCCOS commands.
- **XCC-U04:** MS-OCSPROT §4.1 note <32> excludes earliest OCS generations; 2024 original reflects features beyond particular Skype for Business 2015 Windows binary, and optional Persistent Chat Server is deployment-specific.
- **XCC-U05:** Correlate `ntc:kick`, permission changes, failed sends, history gaps and reconnect with no synthetic rejoin without authoritative lower source.
- **XCC-U06:** File token `serveruri` must be trusted/authorized; lifetime and replay across room/file require official Group Chat File Repository web service specification, not inferred from XCCOS reply alone.

**Full protocol analysis NOT COMPLETE; no implementation code, live servers, interoperability tests, merge or release.**
