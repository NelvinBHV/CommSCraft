# Evidence: MS-XCCOSIP / MS-OCGCWEB – persistent chat (chat rooms)

Sources: [MS-XCCOSIP] "Extensible Chat Control Over Session Initiation Protocol (SIP)", edition 20250520, 91 pp.; [MS-OCGCWEB] "Persistent Chat Web Protocol", edition 20250520, 15 pp. Pool URIs from in-band provisioning `persistentChatConfiguration` (DefaultPersistentChatPoolUri, EnablePersistentChat, PersistentChatPoolUris, PersistentChatWebManagerUriInt/Ext; MS-SIPREGE §2.2.2.5.11, file 04).

| ID | Requirement | Evidence |
|---|---|---|
| XC-01 | SIP dialog via **INVITE** to the chat server, payload via **SIP INFO** (RFC 6086); the INFO response = delivery status; TLS only | XCCOSIP §1.4, pp. 12–13; §2.1, p. 14 |
| XC-02 | XML namespace `urn:parlano:xml:ns:xccos`; envelope `<xccos ver="1" envid="<monotonic>">` with any number of cmd/rpl/ntc/err/sys/grpchat; every primitive with `seqid` | §2.2.1, p. 14; §2.2.2.2.1, pp. 35–36 |
| XC-03 | Commands (`cmd:`): bjoin, join, part, bccontext (history), bcbydate/bcbymsg (search), chansrch, chaneffquery, chaninfo, chancreate, chanmodify, getassociations, getinv, getpl, getpref/setpref, getserverinfo, requri, usrsrch, getfutok/getfdtok (files), getroomperms, role lists (qeulmem/qeulmgr/qeulvoiced, sacemem/sacemgr/sacevoiced) and others; responses `rpl:` with `commandid` (seqid+envid) and `<resp code="200">`; notifications `ntc:` (join, part, bjoin, kick, invite, chanmodify, usermodify, pl/ploff, quit, chatmodify); `err`, `sys:status` | §2.2.2.2.2–2.2.2.2.8, pp. 36–42 |
| XC-04 | Chat message `<grpchat id="grpchat" seqid chanUri author authdisp alert chatId ts>` with `<chat>` (text) and optional `<rtf>`; when sending chatId 0, authdisp may be empty; the server echoes it with originatingMessageId | §2.2.2.2.9, pp. 42–43; §3.1.13–3.1.14, pp. 54–56 |
| XC-05 | Flow: INVITE to the pool URI → `cmd:requri` ⇒ response uib/@uri = channel server URI → **BYE** and a new dialog there → `cmd:getserverinfo` (domain, infoType bitmap, clientVersion; response incl. pingInterval, limits, roomManagementUrl) → getassociations/bjoin … | §3.1.3–3.1.5, pp. 45–47; §3.1.11, pp. 52–53 |
| XC-06 | Transaction timer 10 s per command and per sent chat message; match responses by (envid, seqid) and the matching rpl name; code ≠ 200 ⇒ error | §3.1.2.1–3.1.2.1.5, pp. 43–46; §3.1.13.2, p. 55 |
| XC-07 | Client state per room: ParticipantList, LastReceivedMessageId (largest chatId) | §3.1.1, p. 43; §3.1.14.5, p. 56 |
| XC-08 | Room management via a web application (OCGCWEB URL formats; URL from getserverinfo or provisioning) | OCGCWEB §1.3, p. 5 |

## Gaps
- L-16: Neither the **Content-Type of the INFO body** nor the **SDP of the INVITE** to the chat server is given in the document; the examples in §4 (pp. 72–76) show only XML payloads.
- Note: persistent chat still exists in SfB Server 2019 but is not used in every deployment – plan it as an **optional** function.

## Later finding (file 39)
- L-16 **secondary evidence** (OB-17): INVITE to `ocschat@<domain>` or the pool URI with an IM SDP; XCCOS in INFO with `Content-Type: text/plain`.
- L-16 stays secondary evidence (OB-17); interop test IT-05.
