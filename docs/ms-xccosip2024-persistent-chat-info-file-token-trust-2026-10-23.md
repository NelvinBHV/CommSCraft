# MS-XCCOSIP 2024: persistent group chat SIP INFO, message correlation, room membership and file-repository tokens

**Original-document investigation 2026-10-23. Scope: classic persistent Group Chat only, not all SfB client personal file transfers. Research INCOMPLETE.**

**Originals:** [MS-XCCOSIP] *Extensible Chat Control Over Session Initiation Protocol (SIP)*, **v20240820, 2024-08-20**, source PDF SHA-256 `dedf341e7f1031453bab3bc5a196a8563ad7381895e0458eb51bf870e270077e`, owner original `catalog/MS-XCCOSIP.pdf`, official title/editions [Microsoft Open Specifications](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-xccosip/); direct **§§2.2.2.1.24–25,2.2.2.2.1–2.2.2.2.9,3.1.1–3.1.8,3.1.19,3.2.1**. [MS-OCSPROT] *Skype for Business and Skype for Business Server Protocols Overview* rev9.0 **v20250520** §2.5.21 for an end-to-end group-chat use case. RFCs [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) §§12–15, [RFC2976](https://www.rfc-editor.org/rfc/rfc2976) legacy SIP INFO; successor [RFC6086](https://www.rfc-editor.org/rfc/rfc6086) requires edition-gated comparison, not retrofit. Class **P** Microsoft original actor/section, **S** cross-source sequencing, **U** exact Windows deployment/HTTP File Repository wire contract unresolved.

## A. Group-chat control channel and two independent acknowledgement layers

| ID | Original section | Direct bounded behavior |
|---|---|---|
| XCC-01 | MS-OCSPROT §2.5.21 | After signed-in client SIP **INVITE** to Group Chat Server and **200/ACK**, server-info sequence is **client SIP INFO `cmd:getserverinfo` → immediate SIP 200 → server SIP INFO `rpl:getserverinfo` → client SIP 200**. Then group join **`cmd:bjoin`** repeats separate SIP INFO/200 plus later server INFO/200 with `rpl:bjoin`. Therefore SIP INFO 200 **is not** the final XCCOS command success. |
| XCC-02 | MS-XCCOSIP §§3.1.1,3.1.2 | Each room maintains **ParticipantList** (SIP URIs) and **LastReceivedMessageId**; XccosControlPrimitive embeds transactional `XccosCommandPrimitive` except for direct GroupChatDataBlock chat messages. This protocol differs from **personal SIP MESSAGE**, **conference IM MCU Message-Id**, and **PSOM RPC**. |
| XCC-03 | §3.1.2.1.1–3 | Command identifier consists of **EnvId uint64 + SeqId uint64**. EnvId taken from outer `xccos.envid`; when multiple commands in same envelope, each **seqid MUST be unique**. These correlate **application command replies**, not SIP transaction CSeq/Call-ID. |
| XCC-04 | §§3.1.2.1.2,.5–6 | Each command has **XccosTransactionTimer = 10 seconds**, started on send; timeout ends command. Client cancels timer on matched reply or cancellation; a late unmatched reply **must be discarded** after command no longer pending under validation rules. No source-proven infinite retry. |
| XCC-05 | §3.1.2.1.5 | Incoming XccosControlPrimitive failing schema validation **dropped**. For each reply client must validate commandid + seqid + envid + pending CmdId + response code + corresponding `rpl:...` ID (e.g. `cmd:join` must not match `rpl:bjoin`). Missing or mismatched fields ⇒ drop. **ResponseBlock.code=200 only** is application success; other codes failure. |
| XCC-06 | §§3.2.1.1–3.2.1.6 | Server CmdId uses same EnvId+SeqId; server timeout has **25-second default** and on expiry **MUST generate SIP 503** to the pending INFO request. **10-second client application timer vs 25-second server processing timer** is an actual cross-actor timebase difference; not proof all valid later replies should be accepted by timed-out client. |
| XCC-07 | §§3.1.4.3–5 | Client sends `cmd:getserverinfo`, then reads server display name and room-management URL; command failure entails **client MUST tear down SIP INVITE dialog**. Room-management URL is **not** an authentication authority or automatic safe browser destination without relevant policy. |
| XCC-08 | §§3.1.5.3–5 | One-room join `cmd:join`: exactly one XccosCommandDataBlock / ChannelIdsInformationDataBlock with **room GUID extracted from channel URI**, plus server domain. Success ChannelInformationDataBlock maps transient user IDs→SIP URIs and active-user IDs to ParticipantList, and optionally name/description. |
| XCC-09 | §§3.1.6.3–5 | Multi-room join `cmd:bjoin` with **comma-separated GUIDs** and server domain. Success returns multiple ChannelInformationDataBlock; independent ParticipantList by room. Do not collapse room participants into current account's presence category subscriptions. |
| XCC-10 | §§3.1.7–3.1.8 | After join client **SHOULD retrieve recent channel history**. Searching chat history `cmd:bcbydate` takes filter AND/OR, count limit, date interval and optional specific author SIP URIs; multi-room query **MUST be repeated once per room**, not one global server search assumed. |
| XCC-11 | §2.2.2.2.9 | `GroupChatDataBlock` (`grpchat`) for actual messages includes `chanUri`, `author SIP URI`, `authdisp`, `alert`, `chatId` (server-generated, client may send zero), ISO8601 `ts`, plain `chat` and optional `rtf`, optional originatingMessageId. **Not the same XML as XccosCommand/Reply**. |
| XCC-12 | §§3.1.1,2.2.2.2.9 | `chatId` is unique **within room**; client maintains **LastReceivedMessageId** by room. Server-assigned IDs/timestamps and client-originated placeholders differ; messages should not be globally sorted by EnvId/SeqId nor their client local timestamps without source proof. |

## B. Persistent room file upload/download token is not the actual upload transport

| ID | Direct clause | Role and security semantics |
|---|---|---|
| XCC-F01 | §§2.2.2.1.24,3.1.19.3 | `FileTokenDataBlock` element `ftdb` includes **channelUri** e.g. `ma-chan://domain/GUID` and **fileUrl** e.g. `ma-filelink://domain/channelGUID/fileGUID.../filename`. For both **upload and download** requests, source explicitly says construct command **`cmd:getfutok`** with exactly one CommandDataBlock and exactly one ftdb. **Do not invent a different command or assume two token names**. |
| XCC-F02 | §§2.2.2.1.25,3.1.19.5 | On success client finds `TokenDataBlock` `token` and **`serveruri`** (the web service endpoint honoring token); **empty token means discard**. Save both to perform later **Group Chat File Repository Web Service** requests. Example `serveruri` is an HTTPS `MGCWebService.asmx` endpoint, not SIP URI. |
| XCC-F03 | §3.1.19.5 | Token is a short-lived **file AND channel-scoped bearer capability**; it is provided as parameter on later upload/download Web Service calls and **cannot be assumed usable for any other file/channel**. Exact lifetime numeric duration **not stated** in examined source. |
| XCC-F04 | §§3.1.19.1–7 | File-token exchange itself lists **no own separate timers or higher-layer events**, but generic **10-second XCCOS command timer** applies. Later HTTP transfer retry, TLS certificate validation, request methods, file limits, scanning and content integrity are **out of scope of checked MS-XCCOSIP clauses**, requiring File Repository Web Service protocol originals. |
| XCC-F05 | §§3.1.19,2.2.2.1.24–25 | **Trust boundary:** Authenticated SIP Group Chat server grants a token and directs client to a URL. Never equate token possession with proof URL is a trusted host, or treat arbitrary returned `serveruri` as safe without TLS/allow-list/redirect/credential-leak assessment; this is a justified **S security requirement**, not a Microsoft token validation MUST quoted from source. |
| XCC-F06 | MS-OCSPROT §2.5.21 / MS-XCCOSIP §3.1.19 | This applies to **persistent Group Chat File Repository** specifically, **not** direct 1:1 chat file transfer, conference PSOM OC Package, WebRTC DataChannel or server-managed document sharing. Those separate transfer profiles are still OPEN. |

## C. Wire and application state (S: command/reply + file HTTP bridge)

~~~mermaid
sequenceDiagram
 participant C as Signed-in Windows Group Chat client
 participant G as Group Chat SIP server
 participant W as Group Chat File Repository web service
 C->>G: INVITE Group Chat service
 G-->>C: 200 OK
 C->>G: ACK
 C->>G: INFO xccos cmd:getserverinfo (EnvId,SeqId)
 G-->>C: SIP 200 for incoming INFO
 G->>C: INFO xccos rpl:getserverinfo (correlated)
 C-->>G: SIP 200
 C->>G: INFO xccos cmd:bjoin (GUID list)
 G-->>C: SIP 200
 G->>C: INFO xccos rpl:bjoin with room state
 C-->>G: SIP 200
 opt File upload/download for joined channel
  C->>G: INFO xccos cmd:getfutok + ftdb channelUri/fileUrl
  G-->>C: SIP 200 transaction reply
  G->>C: INFO xccos rpl:getfutok with token + serveruri
  C-->>G: SIP 200
  C->>W: Separate HTTPS web service operation using scoped token
 end
~~~

**S diagram caveat:** The source's example does not define an end-to-end file transfer ACK/virus scan/receipt in the Group Chat Web Service. No claim of full file receipt or UI success follows solely from SIP 200, rpl:getfutok success or token acquisition.

## D. Gaps and version conflicts for Windows Full Client

1. **CHAT-U01:** Full 91-page MS-XCCOSIP chat history, moderation, room invitations/access control and edit/retention events not exhaustively analyzed, nor complete XCCOS XML schema.
2. **CHAT-U02:** Exact **Group Chat File Repository Web Service** WSDL/HTTP request and response protocol original and auth/secure token exchange not yet indexed; MS-XCCOSIP only reaches token issuance.
3. **CHAT-U03:** Historical product difference MS-XCCOSIP v20240820 vs MS-OCSPROT rev9 v20250520; target Windows Full Client build may not support persistent Group Chat or room file posting in all server editions. No inferred feature support from overview alone.
4. **CHAT-U04:** Client command 10s vs server 25s timeout can race; source requires pending-reply match and discard stale response. Exact SIP INFO timeout semantics under RFC3261 and whether late server SIP 503 or new INFO arrives require scenario-specific proof.
5. **CHAT-U05:** Author SIP URI in `grpchat` is a protocol field, not automatically authenticated just because it's present in XML. Need server authorization/identity validation and SIPAE MIC/trust boundary.
6. **CHAT-U06:** Server-returned HTTPS URL/token can cross service trust boundary. Actual certificate validation, URL whitelisting, redirects, MIME handling and privilege semantics remain unknown.
7. **CHAT-U07:** Historical RFC2976 legacy INFO vs RFC6086 Info Package and SIP dialog transport negotiation must be version-gated, not automatically replaced by modern generic.
8. **CHAT-U08:** **Direct personal file transfer normative sources remain NOT IDENTIFIED** by this one study. The user-specified entire Windows client transfer behavior cannot be marked complete.

**Research incomplete. No protocol implementation, real server/interop testing, main merge or release.**
