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
| CI-10 | Inviting a participant (dial-out): INFO with addUser (`mscp:mcuUri` = IM MCU conference URI, joining-method `dialed-out`, media `chat`) in the focus dialog ⇒ 202 Accepted ⇒ C3P `pending`/`success` arrive as INFO from the focus (answered 200 OK); the IM MCU sends the invitee an INVITE with `Referred-By`, `Ms-Focus-Uri`, `Session-Expires: 600;refresher=uas` and `a=accept-types:*` | §4.1.2 steps 1–15, pp. 41–50 |
| CI-11 | Session timers: the focus answers the join with `Session-Expires: 7200;refresher=uac` + `Require: timer` (the client refreshes); the IM MCU's dial-out INVITE names the invitee as refresher (`refresher=uas`); refresh by empty UPDATE with `Supported: timer` | §4.1.1 step 4, p. 30; §4.1.2 step 9, p. 46; [MS-CONFBAS] §4.2.2, pp. 124–125 |
| CI-12 | Forwarding: for a client without `multipart/alternative` the IM MCU extracts the `text/plain` part; its Content-Type can carry further parameters (`charset=UTF-8;msgr=…`); 202 with `Message-Id` to the sender | §4.2.1 steps 1–4, pp. 53–55 |
| CI-09 | Leave via BYE to the IM MCU; ejection ⇒ BYE from the MCU | §3.2.1.4.1, p. 20; §3.2.2.4.1, p. 21 |

## Conflicts / gaps
- K-CI-1: §3.1.3.4.2 (p. 19) refers to "MS-CONFAV §2.2.3.4" for the implicit addUser – that section is modifyConferenceAnnouncements; §2.2.3.2 (addUser dial-in) is obviously meant. Editorial error.
- K-CI-2: §3.4.1.5.2 (p. 26) "200 OK to the incoming **MESSAGE**" in the section about INFO – INFO is meant.
- K-CI-3: Invitations of other clients as INVITE with `application/ms-conf-invite+xml` are named only in [MS-SIPRE] 20250520 §3.9.5.1 (p. 58: content type and an `audio` element with `available="true"`; full-text search of the package: further hits only as `Supported: ms-conf-invite` in examples). The body structure (`Conferencing`, `focus-uri`, `subject`, `im available`) is secondary evidence only ([SIPE] `sipe-conf.c` l. 70–77, `sipe-incoming.c` l. 415). Implementation: a checked focus URI is reported to the user, joining is a command; interop test IT-20. CommSCraft itself invites normatively by dial-out (CI-10).
- L-11: The **body format of the typing indicator** (INFO content, Content-Type) is not specified anywhere in the Microsoft package (full-text search "typing/KeyboardActivity/iscomposing" without hits except MS-CONFIM).

## Later finding (file 39)
- L-11 **secondary evidence** (OB-16): INFO, `application/xml`, `<KeyboardActivity><status status="type|idle"/></KeyboardActivity>`; two independent secondary sources.
- L-11 narrowed (OB-50): the MCU forwards typing INFO without interpreting the body (§3.4.1.4.1, p. 26); the body format for receiving clients is interop test IT-04.
- Implementation (2026-10-27, `crates/im`): 1:1 IM sessions reuse the message SDP of CI-02 (`m=message 5060 sip null`, `a=accept-types`). That 1:1 sessions use it is documented only as a flow in [MS-OCSPROT] §2.5.9 (pp. 36–37) and confirmed by a secondary source ([SIPE] `sipe-im.c` l. 385–411) – inference + secondary. Design (security): CommSCraft offers and accepts only `text/plain`; other formats are answered with 415 and `Accept: text/plain`; the typing indicator is accepted only as the small `KeyboardActivity` document (OB-16).

## Implementation (2026-10-28, `crates/conf`, `crates/session`)
Group IM: addConference (`openAuthenticated` as in [MS-CONFPRO] §4.1, p. 41, with a 128-bit random conference ID – design), join the focus as presenter (organizer) or attendee, roster subscription (`Event: conference`, piggyback, BENOTIFY), INVITE to the `chat` URI of `conf-uris`, invitations by dial-out (CI-10), 202 + IMDN with the 10 s timer (CI-05), sender from `Ms-Sender` (CI-06), session timers (CI-11), leave (§4.3.2, pp. 67–70). Status: tested-offline; IT-21.
