# MS-PSOM 11.0: whiteboard/presentation distributed objects, secure DataMCU join and content lifecycle

**Research 2026-10-23. P = primary original source; S = cross-spec synthesis; U = unresolved. INCOMPLETE, no implementation.**

**Original:** Microsoft **[MS-PSOM] PSOM Shared Object Messaging Protocol rev11.0, v20250520, released 2025-05-20**, exact original PDF SHA-256 `6eee9f7f42014a0806d887448bb5837a68eb56a2a5048ebfa9fd9a4814b6e049`, [Microsoft published versions](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-psom/0b9a26ba-b92e-4a71-a1ea-205c5a0c250c); original PDF user-provided `catalog/MS-PSOM.pdf`. Analyzed original **§§1.3–1.7,2.2.1–2.2.2,3.1.2–3.1.7,3.1.4.1.3–3.1.4.1.13,3.2.3.1.1–3.2.4.1.10,3.3.4.1.4–3.3.4.1.10** (selected subclauses); not all 111 PDF pages audited visually. Dependency **[MS-OCSPROT] rev9, v20250520 §2.5.20** (whiteboard use case and diagram); **[MS-CONFBAS] rev15, v20250520 §§2.2.3.17–18,3.5** C3P addUser/return media DataMCU credentials; **[MS-CONFPRO] rev13**, **[MS-OCER] rev11**, relevant RFC **[RFC3261](https://www.rfc-editor.org/rfc/rfc3261)** (conference SIP transport), RFC5246 (TLS1.2 historical), RFC5280 (certificate validation). **The source itself does not establish that all classic Full Client versions expose each distributed object; product notes and roles remain U.**

## A. Exactly where C3P ends and PSOM starts

| ID | Direct original clause | Source-specific role/state |
|---|---|---|
| PSOM-01 | MS-OCSPROT §2.5.20; MS-CONFBAS §§2.2.3.17–18 | Only after Focus conference membership and conference-event subscription can client obtain DataMCU-specific `addUser` response: C3P operation over **SIP INFO**, provisional **202 Accepted** and later **SIP INFO with credentials**, acknowledged by client **SIP 200**. Subsequent PSOM media session uses a separate network path. This is **not** the Focus INVITE nor an IM MCU MESSAGE. |
| PSOM-02 | MS-PSOM §3.2.3.1.1.1 | `AddUser` response supplies **`sAuthId` token**, conditional **`proxyHeader`**, `pwrpc.modes` and `pwrpc.pwsURI`. For `tls`, connect to server FQDN:port directly; for `fwdtls`, connect to one of configured **proxy[i].FQDN:Port**, randomized initial selection/fallback, and send proxyHeader. **MS-SIPAE authentication state is not PSOM AuthId**. |
| PSOM-03 | §3.2.3.1.1.1 | If mode `tls` and `alternativeName` returned, source requires using that value instead of certificate subject **in FQDN validation**. This is not permission to skip certificate chain validation; detailed trust-store and proxy profile remain U. |
| PSOM-04 | §§3.2.3.1.1.1–1.2 | After establishing TLS/proxy: optional network-order 32-bit **proxyHeader length** and bytes; then bytes **`70 77 32 00 00 00 00 00` + 32-bit token-byte-length + `sAuthId`**. Server must reply **`70 77 32 00`** to complete PSOM authentication. Incorrect auth ⇒ server **MUST immediately terminate** connection. |
| PSOM-05 | §3.2.3.1.2 | On channel 0, **ConnMgr** exchanges **version**, **addProtocol(name, version array, interface hashes)** and **doneProtocols**. Client MUST terminate on incompatible hashes; after sent doneProtocols it MUST NOT send any more version/addProtocol/doneProtocols. No permission to ignore failed method-hash compatibility and proceed to unknown whiteboard method. |
| PSOM-06 | §§1.3,2.2.1,3.1.2 | PSOM channel **0** implicit for control/versioning, application distributed objects exchange on channel **2**. **ConnMgr ping every 30 seconds** by both ends is transport keepalive, not SIP OPTIONS or ICE Binding Consent. |
| PSOM-07 | §§3.1.4,3.1.5,3.1.7 | All messages **MUST be processed in arrival order**; unrecognized message **MUST terminate** connection; on connection loss **no in-place recovery**, new **full connect/auth/version/root-distributed-object sequence MUST be repeated**. Timers SHOULD reset to initial state. |
| PSOM-08 | §§2.2.1.1.1–.5 | PSOM record is **1-byte type** and, when defined, 32-bit **big-endian** body length. **Close 0x00** on channel; **SetChannel 0x04** + 4-byte ChannelId; **Break 0x06** + 32-bit ASCII reason length and bytes; **RpcMessage 0x16** + 4-byte body length and RPC; **RPCOpen 0x37** for channel open. **Close other channels before base channel 0**. |
| PSOM-09 | §2.2.2 | Distributed-object **ProxyId** signed 32-bit generic-int, root 0; connecting side assigns increasing positive identifiers, receiver uses **negated** counterpart, so client and server resolve opposite signs of the same object. Treating raw signed value as universal object identity breaks routing. |
| PSOM-10 | §§2.2.2.1–2.2.2.2,3.1.4.1.1 | RPC operation can **connect child**, **disconnect child** or call indexed interface method. Method index starts at **1** and follows declaration ordering under each versioned DOInterface. Name/hash/child hierarchy govern dispatch; it is not SOAP or the SIP C3P XML schema. |

## B. Real conference collaboration content graph and permission checks

| ID | Original clause | Source/role-scoped meaning |
|---|---|---|
| PSOM-11 | §3.1.4.1.6.2, §3.2.4.1.1 | Application root distributed object **Meeting** has ContentManager and ContentUserManager children. Both **MUST connect before cMeetingReady**. Server **MUST send cSetUrlBase** and **cSetServerTime (UTC)** before ready. An authenticated socket does not imply content list ready. |
| PSOM-12 | §§3.1.4.1.4–5,3.2.4.1.3 | ContentManager tracks per-meeting Content objects and **active presenter baton**; **clients MUST reserve content title before creation** for uniqueness, then server calls `cContentCreated` with **server contentId + client cookie**, and `cContentAdded/Removed` for available content. Cookie correlates submission, contentId identifies actual object. |
| PSOM-13 | §§3.1.4.1.4.2,3.1.4.1.5.2 | Client connects named child **`content.<decimal contentId>`**, then **`extendedContent`** for subtype-specific interface. Content method `sPresent/sStopPresenting` and ContentManager active presenter notifications govern presentation status separately from Focus meeting roles and ASMCU sharer. |
| PSOM-14 | §3.1.4.1.5 | Content visibility enum **MeetingOrganizer=0**, **Presenters=1**, **Everyone=2**. `sSetVisibility` is a request governed by server authorization; **UserNotAuthorized=3** may be returned when renaming content. Having a valid AuthId does not give unconditional permission to reveal slides to everyone. |
| PSOM-15 | §§3.1.4.1.8,3.3.4.1.4–5 | **UploadManager** uploads **OC Package** as file; uploading file bytes to server occurs over **PSOM channel UploadStream**, with `sRequestUpload`, server `sWrite` acknowledgement **cWriteComplete**, and client **sUploadFinished(cookie, cancel)**. `sWrite` requires monotonically incrementing **packetNum**. This is **conference content package** transfer, not P2P chat attachment or PSTN data. |
| PSOM-16 | §§3.3.4.1.4,3.3.4.1.5 | Optional **sRequestWebUploadBlob** permits HTTPS Form POST; **sRequestWebDownloadBlob** for clients unable to decrypt may retrieve server-decrypted file. These are **conditionally supported methods** and carry sensitive content; the user-facing protocol evidence does not specify universal HTTPS trust, redirect or upload limits. |
| PSOM-17 | §§3.1.4.1.11,3.2.4.1.10,3.3.4.1.10 | **PptContent** distributes slide count, slide navigation **PptLocation**, thumbnail and content file URL/resource errors to clients; `cConnectCompleted` signals initial slide/location/thumbnail information processed. PPT resource types include **PartialPpt=1, FullPpt=2, NativeFile=3**, deprecated DHTML variants. **Slide location is mutable distributed state**, not H.264 video stream RTP position. |
| PSOM-18 | §§3.1.4.1.12–13,3.2.4.1.8–9,3.3.4.1.8–9 | **WhiteboardContent has no server-side methods**, only client `cConnectCompleted` **exactly once** after child connected; **annotation changes live in AnnotationContainer**, with add/remove/clear, text-delta, telepointer operations. A blank WhiteboardContent interface does **not** mean no collaborative whiteboard actions. |
| PSOM-19 | §3.3.4.1.8 | `sChangeText(id, gen, textVersion, begins, ends, characters)` is optimistic update: missing annotation ⇒ `cErrorChangeText`; **stale textVersion MUST reject silently without error response**; invalid deltas **SHOULD disconnect**; valid deltas increment generation and notify all clients via **cChangeTextBatch**. Race recovery and preserving newest version remain U. |
| PSOM-20 | §§3.2.4.1.8,3.3.4.1.8 | Server can announce annotation image **resource URL, AES-256 key, IV and SHA-1 hash of plaintext file** through `cSetImageFileInfo`. SHA-1 file hash is *not* evidence of a cryptographic MAC or trusted server identity; HTTPS URL/redirect trust, key secrecy, authorization and integrity remain independent research gates. |
| PSOM-21 | §3.3.4.1.8 | `sSetTelepointer(anchor,visible)` updates cursor/visible state to all clients. `sClearAnnotations`, `sRemoveAnnotation`, `sRemoveAnnotations` have different success/failure rules; server broadcasts successful changes and returns error to initiating client when source condition met. |
| PSOM-22 | §§3.1.4.1.14–16,3.3.4.1.11–13 | **PollContent**, **SharedNotesContent**, **QnaContent** are separate distributed object subtypes with per-role methods and product behavior flags. Source expressly has legacy/deprecated and implementation-conditioned variants; **not all are guaranteed present in target Skype for Business Windows build**. |

## C. Cross-service diagram (S, conditional flow, not captured traffic)

~~~mermaid
sequenceDiagram
 participant C as Windows conference client
 participant F as Focus / C3P authority
 participant D as DataMCU PSOM server
 participant W as Resource URL / file service
 C->>F: INFO C3P addUser for data conference
 F-->>C: 202 then INFO C3P result sAuthId/pwrpc options
 C-->>F: SIP 200 for returned INFO
 C->>D: TLS to pwsURI or fwdtls proxy
 C->>D: optional proxy header + pw2 authentication token frame
 D-->>C: pw2 signature accepted
 C->>D: channel0 ConnMgr versions + addProtocol hashes + doneProtocols
 D-->>C: compatible interface versions/doneProtocols
 C->>D: channel2 Meeting/ContentManager/ContentUserManager connect
 D-->>C: UrlBase, ServerTime, cMeetingReady
 C->>D: ReserveTitle, Content request / Content.Whiteboard child
 D-->>C: contentId, annotation child and state
 opt Shared slide or image resources
  C->>D: UploadManager OC Package / UploadStream
  D-->>C: content resource and encrypted-file metadata
  C->>W: Conditional HTTPS resource retrieval
 end
 Note over C,D: 30s ConnMgr ping, failure means full PSOM reconnect
~~~

## D. Blocking requirements and source caveats

1. **PSOM-X01:** Source gives TLS certificate `alternativeName` handling but no comprehensive PKIX chain revocation/pinning/fwdtls authenticated proxy trust policy here. Need [MS-CONFBAS] §2.2.3.17/18 exact credentials, AuthId expiration and security consequences of a compromised DataMCU proxy.
2. **PSOM-X02:** No in-place PSOM reconnect; new full auth/version/root is mandated. Whether contents and annotations persist after DataMCU failover depends on DataMCU and server persistence, not inferred from client reconnect.
3. **PSOM-X03:** User's **file transfer** feature must be separated into PSOM OC Package upload, XCCOSIP persistent chat repository and P2P/SIP file sharing (latter normative spec **NOT YET IDENTIFIED**). These are not one file-transfer protocol.
4. **PSOM-X04:** Distributed object hashing, GenericInt, nested marshaling, RPC error states, ChannelClose/Break priority and Version=2 Meeting versus Version=1 ConnMgr require complete §§2.2 and Appendix A audit; this tranche selected original clauses only.
5. **PSOM-X05:** A SHA-1 hash of decrypted annotation resource alone does **not authenticate** a source nor replace AES256 key confidentiality; do not trust arbitrary URL from unverified PSOM peer or presume HTTPS certificate checks optional.
6. **PSOM-X06:** Polling, shared notes, QnA, Web PowerPoint, file decryption and WOPI depend on other separate docs: **MS-WOPI, MS-OCSPROT §2.5.20 and MS-OCAUTHWS**; current artifact is not exhaustive.
7. **PSOM-X07:** Historic product notes, exact Skype for Business client release, server DataMCU versions and feature gates remain unverified. A 2025 Open Specification does not establish current deployment behavior or end-user UI capability.

**Original-document-only continuation. Complete Windows Full Client normative closure is NOT proven. Implementation planning remains BLOCKED; no protocol code, real-server testing/connection, main merge or release.**
