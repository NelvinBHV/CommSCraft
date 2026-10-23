# MS-CONFIM 11.0 — Conference instant messaging: SDP, MCU forwarding, IMDN, typing and delivery state

**Research-only original-specification audit 2026-10-23. Overall protocol analysis NOT complete.** Microsoft original **[MS-CONFIM] Centralized Conference Control Protocol: Instant Messaging Extensions, revision 11.0, v20250520, released 2025-05-20**, original PDF SHA-256 `a0ec8195aeca242aa926bce23aaaec12ffefbeb27484f43a0326db952c9e4614`. Full original available at [Microsoft PDF](https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-CONFIM/%5BMS-CONFIM%5D.pdf). **Original portions examined:** §§1.3–1.7, 2.2.1–2.2.6, 3.1.1–3.1.3.5.2, 3.2.1–3.2.2.5, 3.3.1–3.3.2.6, 3.4.1–3.4.2.5 and original examples §4.2.2. Original PDF pages 22, 24, 25 and 26 rendered directly; §3.4.1.5.2 PDF typo verified visually. Normative/supporting originals [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) (June 2002), [RFC3428](https://www.rfc-editor.org/rfc/rfc3428) (December 2002), [RFC4028](https://www.rfc-editor.org/rfc/rfc4028) (April 2005), [RFC2045](https://www.rfc-editor.org/rfc/rfc2045)/[RFC2046](https://www.rfc-editor.org/rfc/rfc2046), [RFC4566](https://www.rfc-editor.org/rfc/rfc4566), legacy [RFC2976](https://www.rfc-editor.org/rfc/rfc2976) (September 2000), [RFC6086](https://www.rfc-editor.org/rfc/rfc6086) (January 2011, obsoletes RFC2976), historical [RFC3265](https://www.rfc-editor.org/rfc/rfc3265) (June 2002; superseded by RFC6665). Microsoft [MS-CONFBAS] rev15 §§3.4.4.2, 3.8 connects conference Focus to MCU state.

Codes: **P** primary normative Microsoft text and actor, **R** RFC original, **S** cross-source synthesis, **U** unresolved edition/actual client behavior. MUST/SHOULD retain their cited scope (client vs MCU vs Focus).

## 1. Session creation and per-participant capabilities

| ID | Edition-qualified evidence | Normative observation and required state separation |
|---|---|---|
| C-IM01 | MS-CONFIM §2.2.5 | A conference IM media session is an **INVITE dialog** and uses SDP **`m=message 5060 sip null`**. Sending endpoint **MUST set port to 5060**; receiving endpoint **MUST ignore that SDP port**. This media line is not RTP or a TCP socket allocation at 5060. |
| C-IM02 | §§2.2.5,3.1.1.1 | SDP `a=accept-types` lists MIME content-types, MUST be associated with `m=message`; absence implies `text/plain`; all clients assumed to support plain text. In negotiated media session clients **MUST convey supported formats**. |
| C-IM03 | §3.1.2.5 | Client **MUST NOT offer `a=accept-types:*`**. A server offer with `*` means server can accept all *formats in original client offer*, not arbitrary unknown MIME. Client **SHOULD** support RFC2046 `multipart/alternative`. |
| C-IM04 | §§2.2.3,3.1.1.2 | Extension `Supported: ms-sender` in client INVITE or 200 enables server forwarding sender identity via `Ms-Sender` header. Clients supporting spec **SHOULD** support ms-sender; actual extension support is per **endpoint**, not per account. |
| C-IM05 | §§2.2.1,3.1.2.5.2,3.1.3.5.2 | MCU **MUST** publish `endpoint-capabilities` (supported IM formats, optional user-agent) in conference-info notification. Client **SHOULD** parse format support and select common type; some peers accept only text/plain. **Roster notification data** comes via MS-CONFBAS and is not a MIME negotiation response by itself. |
| C-IM06 | §§3.1.2.4.1,3.1.3.4.1–.2 | Dial-in UAC INVITE Request-URI targets **IM MCU conference-URI**. Dial-out MCU can INVITE client after Focus `addUser`; MCU **SHOULD** attach `Ms-Focus-Uri`, containing conference-URI when included. Client **MUST ignore Ms-Focus-Uri in SIP request/response** per §3.1.2.4, so it is not an authorization or routing instruction for that client. |
| C-IM07 | §3.1 intro,3.2 intro | MCU **MUST** support RFC4028 session timer; client **SHOULD** use RFC3311 UPDATE for session refresh. Client signout or ejection must terminate **IM MCU INVITE dialog** via BYE; **Focus** conference membership is a distinct lifecycle. |
| C-IM08 | §§3.1.3.5.2,3.3.2.1–.2 | MCU maintains a **40-second post-conference-creation MESSAGE history window**, not an indefinite chat history. On join *during window*, stored MESSAGE requests **MUST** be forwarded to participant; after timer expiry list **MUST** be cleared and no new messages added. This window is **MCU server-side**, not a client's delivery timeout. |

## 2. Send, relay, final receipt and failures (RFC3428 + Microsoft extension)

| ID | Original section | Client/MCU action and failure interpretation |
|---|---|---|
| C-IM09 | §3.3.1.4.1 | Client sends RFC3428 `MESSAGE` **inside established IM INVITE dialog to MCU**. When body is `multipart/alternative`, **all parts MUST use binary Content-Transfer-Encoding**; other transfer encodings MUST NOT be used under source. |
| C-IM10 | §3.3.2.1 | MCU maintains **per-conference increasing numeric Message-Id**, initialized to **1**; returned in 200/202, attached to every forwarded MESSAGE and later IMDN. Correlation is **conference-local message-ID**, not SIP CSeq, SIP transaction branch, or globally unique GUID. |
| C-IM11 | §3.3.2.5.1 | If sender is **only connected participant**, MCU **MUST** send **200 OK**; response means no subsequent delivery notification. If others connected MCU **SHOULD** use **202 Accepted** indicating later IMDN. MCU **MUST** append Message-Id to either 200 or 202 and increment counter. |
| C-IM12 | §3.3.1.5.1 | Client **MUST treat 200** as successful forwarding to all others (source semantic). That is **not independent cryptographic proof of receipt** by each target device when no other conference endpoints exist. |
| C-IM13 | §3.3.1.5.1 | On **202**, client **MUST capture Message-Id**, use **0** when absent (specified fallback, although normal MCU MUST supply header); **MUST start sendMessageTimer**, **10 seconds default**, and await matching IMDN. IMDN match ends wait and reports statuses. Timeout **treated as error 408 for all recipients**. |
| C-IM14 | §§2.2.2,3.3.2.5.2 | MCU emits **BENOTIFY** with **`Content-Type: application/ms-imdn+xml`**, XML `imdn/message-id`, zero or more `recipient(uri,status,entry(key,value))`. **Every forwarding error MUST be reported**. Successful recipients **MAY** appear, but clients **MUST NOT rely on success entries**. Empty recipient list means **all other participants successfully forwarded to**. |
| C-IM15 | §§3.3.2.5.1–.2 | MCU **MUST forward** incoming MESSAGE to each other connected conference client and track outgoing responses/errors/timeouts; after **all forwarded MESSAGE transactions complete**, it **MUST send** IMDN. Transient 202 acceptance is **not equivalent to these individual transaction results**. |
| C-IM16 | §§2.2.3–.4,3.3.1.5.2 | MCU forwarded `MESSAGE` uses **From = conference-IM dialog URI**, not original human sender. Client **MUST read Ms-Sender if present** for actual sender display and return **200 OK** for incoming MESSAGE. UI must distinguish originator `Ms-Sender` from SIP routing From. |
| C-IM17 | §3.3.2.5.1 | For client **without ms-sender**, MCU on **text/plain SHOULD prepend sender URI to body**. Nonplain formats for such clients SHOULD produce forwarding error **415**. For ms-sender-capable clients MCU **MUST add Ms-Sender**. Legacy body prefix is a server fallback, **not permission to trust arbitrary body text as authenticated identity**. |
| C-IM18 | §3.3.2.5.1 | If incoming MIME is multipart/alternative and receiver lacks multipart, MCU **SHOULD choose a supported part**; no supported part → **415 forwarding error**. If receiver supports multipart, MCU **MUST forward original multipart**. Don't conflate MIME 415 with auth/registration errors. |
| C-IM19 | §§3.3.1.5.1, 3.3.2.5.2 | **Two distinct timeout scopes**: client sender's **10s** IMDN wait and MCU per-recipient SIP transaction timeouts. A late IMDN after local timer expiry cannot retroactively be reported as guaranteed on-time completion without an explicit source rule; duplicate/late and `Message-Id=0` collisions remain open. |

## 3. Typing/idle INFO is **not** MESSAGE delivery or C3P control

| ID | Source | Separate behavior |
|---|---|---|
| C-IM20 | §§3.4,3.4.1.4.1 | Client sends legacy **SIP INFO** inside the **IM MCU INVITE dialog** for typing/idle body; MCU **does not interpret** application body and forwards to other clients. **Not C3P Focus INFO** from MS-CONFBAS §3.5. |
| C-IM21 | §§3.4.1.5.1,3.4.2.5.1 | MCU **MUST 202 Accepted** incoming INFO. Client **MUST treat 202 as successfully delivered to MCU only**, with **no individual recipient forward status/IMDN**. |
| C-IM22 | §§3.4.2.5.1–.2 | MCU **MUST forward INFO only to endpoints with ms-sender support** and insert original sender URI in `Ms-Sender`; for unsupported endpoints **MUST NOT forward INFO**. Subsequent downstream success/failure/timeout ignored by MCU; typing delivery is best-effort. |
| C-IM23 | §3.4.1.5.2, original PDF p26 | **Verified source inconsistency:** heading says **'Client Receives an INFO Forwarded by the MCU'**, first sentence says **'MUST send 200 OK response to incoming MESSAGE request'**, immediately then says extract sender for **INFO request**. This is a **literal editorial/protocol-reference mismatch**; do not silently rewrite it to an unverified INFO 200 MUST. Higher-layer RFC2976/6086 receiver semantics and official Microsoft errata required to resolve. |
| C-IM24 | RFC6086 §§1,3 and MS-CONFIM §3.4 | RFC6086 **obsoletes RFC2976** but preserves **legacy INFO** compatible operation. Microsoft revision still cites RFC2976. `Info-Package` must **not** be inserted into legacy INFO by assumption; user typing INFO is not proof of IETF Info Package negotiation. |

## 4. Cross-component flow (S, NOT a complete mandatory packet capture)

```mermaid
sequenceDiagram
 participant A as Sender Windows Client
 participant M as Conference IM MCU
 participant B as Participant endpoint
 participant F as Focus / conference-state notifier
 A->>M: INVITE (SDP m=message / accept-types / ms-sender capability)
 M-->>A: 200 (SDP)
 F-->>A: Conference state with peer IM endpoint-capabilities
 A->>M: MESSAGE on MCU INVITE dialog (Content-Type / optional multipart)
 alt Sender is alone
  M-->>A: 200 OK with Message-Id (no further IMDN)
 else Other participants connected
  M-->>A: 202 Accepted with Message-Id
  M->>B: MESSAGE (conference From, Message-Id, Ms-Sender if supported)
  B-->>M: 200 or error or timer expiry
  M->>A: BENOTIFY application/ms-imdn+xml with message-id and per-recipient failures
  Note over A: 10s sendMessageTimer default after 202; 408 on timeout
 end
 opt Typing or idle
  A->>M: INFO on IM dialog
  M-->>A: 202 Accepted (no IMDN)
  M->>B: INFO, only if ms-sender capable
 end
```

**Not every possible conference signal is shown**: Focus factory/provisioning, roster SUBSCRIBE, C3P `addUser`, direct vs dialout initiation and IM media session BYE are independently specified in MS-CONFBAS. The conceptual notification arrow `F` does not assert a particular MCU-to-Focus transport (out of spec). If a participant receives a duplicated early-history MESSAGE, behavior is deployment/product-specific unless verified otherwise.

## 5. Error, trust and edition conflicts still BLOCKING

1. **DOC-ERR-IM01:** §3.4.1.5.2 wrong request method literal (INFO heading but 200 for MESSAGE), verified on PDF p26. Need current original errata/change history before normalizing.
2. **IM-OPEN02:** Per-recipient IMDN statuses are SIP statuses produced for **MCU→endpoint MESSAGE forwards**, not definitive user UI read receipts. A 200 or empty IMDN does not establish human read/ack.
3. **IM-OPEN03:** §3.3.1.5.1 fallback `Message-Id=0` if response omits header; need concurrency, duplicate matching, invalid range/nondecimal handling, message-ID counter wrap and replay protections.
4. **IM-OPEN04:** RFC3265→RFC6665 event update and RFC2976→6086 legacy INFO profiles; source requires **Microsoft BENOTIFY** (no normal NOTIFY transaction behavior), so RFC6665 rules must not be imposed on BENOTIFY without direct MS-SIP evidence.
5. **IM-OPEN05:** **40-second MCU history** is conference-creation-time limited and may replay earlier messages to joining clients; exact duplicate detection/UI ordering on delayed joins and simultaneous forks remains unproven.
6. **IM-OPEN06:** Sender identity trust: `Ms-Sender` authenticated by SIPAE/hop and MCU authorization context as applicable, vs unauthenticated `From` conference URI and legacy plaintext body prefix. Do not trust strings without security validation.
7. **IM-OPEN07:** Need exact error policy for SIP MIME types, unrecognized `a=accept-types`, SDP `*` expansion, out-of-dialog MESSAGE, and early media/session timeout.
8. **IM-OPEN08:** Session timer RFC4028 and SIP BYE handling with lost IM MCU connection and conference Focus ejection, including retry or history after transport reconnection.
9. **IM-OPEN09:** Differentiation from **1:1 IM**: RFC3428 standalone MESSAGE does not by itself create a SIP dialog, while this MS-CONFIM uses MESSAGE **within established INVITE dialog**. Do not use MCU `Message-Id` and IMDN rules for all one-to-one MESSAGE traffic without source proof.
10. **IM-OPEN10:** Microsoft MS-CONFIM 2025 and Windows product notes §6 must be mapped to actual Skype for Business Windows Full Client versions and MCU servers, not only idealized spec role.

**Analysis phase remains OPEN, and all implementation planning forbidden. No protocol code, real-server/interop tests, main merge or release.**
