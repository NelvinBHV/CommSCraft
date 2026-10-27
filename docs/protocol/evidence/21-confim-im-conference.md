# Evidence: MS-CONFIM – instant messaging conference (IM MCU)

Source: [MS-CONFIM] "Centralized Conference Control Protocol: Instant Messaging Extensions", edition 20250520, 78 pp. Extends MS-CONFBAS (file 18). Based on RFC 3428 (MESSAGE), 2976 (INFO), 4028, 3311, 2045/2046 (obtained, file 38).

| ID | Requirement | Evidence |
|---|---|---|
| CI-01 | Join: INVITE to the IM MCU URI (purpose `chat`) (dial-in) or the MCU calls the client (dial-out, with an `Ms-Focus-Uri` header that the client ignores); session timer RFC 4028, refresh via UPDATE | §3.1, p. 16; §3.1.2.4–3.1.2.4.1, p. 17; §3.1.3.4.1, p. 18 |
| CI-02 | SDP: `m=message 5060 sip null` (port meaningless), `a=accept-types:<type/subtype …>`; without accept-types only text/plain; the client must **not** offer `*`; `*` from the server = all offered types | §2.2.5, p. 14; §3.1.1.1, pp. 16–17; §3.1.2.5, p. 17 |
| CI-03 | `Supported: ms-sender` in the client's INVITE/200 OK (SHOULD); otherwise the client only gets text/plain with an inserted sender and **no** forwarded INFOs | §3.1.1.2, p. 17; §3.3.2.5.1, p. 24; §3.4.2.5.1, p. 27 |
| CI-04 | multipart/alternative SHOULD be supported; body parts only with binary Content-Transfer-Encoding; choose the format by the `supported-im-formats` of the other participants (roster `msim:endpoint-capabilities`, incl. user-agent) | §2.2.1, p. 12; §3.1.2.5.2, p. 18; §3.3.1.4.1, p. 22 |
| CI-05 | Sending: MESSAGE in the MCU dialog. Response 200 OK ⇒ delivered to all; **202 Accepted** ⇒ remember the `Message-Id` header (digits, missing ⇒ 0), start sendMessageTimer **10 s**, wait for the **IMDN** (BENOTIFY, `Content-Type: application/ms-imdn+xml`, namespace `http://schemas.microsoft.com/rtc/2005/08/imdn`: message-id, recipient@uri + status SIP code + ms-diagnostics); timer expired ⇒ treat as 408 for all | §2.2.2, p. 13; §2.2.4, p. 14; §3.3.1.5.1, p. 22; §3.3.2.5.2, pp. 24–25; schema §6.2, p. 75 |
| CI-06 | Receiving: answer a forwarded MESSAGE with 200 OK; sender from `Ms-Sender` (From = conference URI) | §2.2.3, p. 13; §3.3.1.5.2, p. 22 |
| CI-07 | The MCU forwards messages sent in the first 40 s after conference start to later participants (history) | §3.3.2, p. 23; §3.1.3.5.2, p. 19 |
| CI-08 | Typing/idle indicator via INFO in the MCU dialog; the MCU answers 202, forwards unchanged with Ms-Sender, no feedback | §3.4–3.4.2.5.2, pp. 25–27 |
| CI-09 | Leave via BYE to the IM MCU; ejection ⇒ BYE from the MCU | §3.2.1.4.1, p. 20; §3.2.2.4.1, p. 21 |

## Conflicts / gaps
- K-CI-1: §3.1.3.4.2 (p. 19) refers to "MS-CONFAV §2.2.3.4" for the implicit addUser – that section is modifyConferenceAnnouncements; §2.2.3.2 (addUser dial-in) is obviously meant. Editorial error.
- K-CI-2: §3.4.1.5.2 (p. 26) "200 OK to the incoming **MESSAGE**" in the section about INFO – INFO is meant.
- L-11: The **body format of the typing indicator** (INFO content, Content-Type) is not specified anywhere in the Microsoft package (full-text search "typing/KeyboardActivity/iscomposing" without hits except MS-CONFIM).

## Later finding (file 39)
- L-11 **secondary evidence** (OB-16): INFO, `application/xml`, `<KeyboardActivity><status status="type|idle"/></KeyboardActivity>`; two independent secondary sources.
- L-11 narrowed (OB-50): the MCU forwards typing INFO without interpreting the body (§3.4.1.4.1, p. 26); the body format for receiving clients is interop test IT-04.
