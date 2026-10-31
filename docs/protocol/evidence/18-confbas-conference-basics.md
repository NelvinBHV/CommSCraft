# Evidence: MS-CONFBAS – conference architecture and signalling

Source: [MS-CONFBAS] "Centralized Conference Control Protocol: Basic Architecture and Signaling Extensions", edition 20250520, 248 pp. Based on RFC 4353/4575/4579/3265/4028/3311/2976 (obtained, file 38). Product behaviour pp. 237–240 names Lync 2013/SfB up to SfB LTSC 2024 explicitly.

## Architecture and flow

| ID | Requirement | Evidence |
|---|---|---|
| CB-01 | Components: **focus factory** (create/modify/query, MS-CONFPRO), **focus** (conference URI, control, roster aggregation), **MCUs** (audio-video, chat, meeting, data-conf, phone-conf, applicationsharing), **join manager** (simple join over HTTP) | §1.3.1, pp. 15–17; §2.2.2.4, p. 29 |
| CB-02 | Flow: (1) create the conference via the focus factory (URI from in-band provisioning, MS-SIPREGE) → (2) INVITE to the focus → (3) activation/MCU bootstrap by the server → (4) SUBSCRIBE roster → (5) join MCUs (MCU-specific) → (6) control commands → (7) leave MCUs → (8) BYE to the focus → (9) deprovisioning | §1.3.2, pp. 17–19 |
| CB-03 | Carriers: focus factory via SIP **SERVICE**, focus control via SIP **INFO** in the INVITE dialog, roster via SUBSCRIBE `Event: conference` (RFC 4575); transport TCP/TLS, IPv4/IPv6 (IPv6 from Lync 2013) | §1.3.3.2–1.3.3.3, p. 20; §2.1, p. 22; <1> p. 237 |
| CB-04 | Version: attribute `C3PVersion="1"`; extensions via the Supported header | §1.7, p. 21 |

## Focus dialog (join / leave)

| ID | Requirement | Evidence |
|---|---|---|
| CB-10 | INVITE to the conference URI (To = conference URI), valid Contact (GRUU), `Content-Type: application/cccp+xml`, body = C3P `addUser` (or `endorseUser`); Session-Expires SHOULD; ms-keep-alive optional | §2.2.1.1–2.2.1.2, pp. 22–23; example §4.2.1, p. 121 |
| CB-11 | addUser: exactly one `user` (entity = From URI) with `roles` (presenter/attendee) and `endpoint entity="{GUID}"`; a lobby-capable client sets `<lobby-capable>true</lobby-capable>` in `clientInfo`; joining on behalf: header `p-session-on-behalf-of` + element `session-on-behalf-of` (mandatory from OC 2007 R2) | §2.2.3.3, p. 38; §3.2.4.1.1, pp. 86–87; <40> p. 239 |
| CB-12 | 200 OK: Contact with `isfocus`, `Allow: INVITE, ACK, BYE, CANCEL, UPDATE, INFO`, Session-Expires `refresher=uac` + `Require: timer`; body = addUser response with the **granted role** (the client adopts it) | §2.2.1.2, p. 23; §3.2.5.1–3.2.5.2.1, pp. 88–89; example p. 122 |
| CB-13 | Refresh the dialog via UPDATE (empty, `Supported: timer`) per RFC 4028; the focus may accept a re-INVITE with a 600 s extension | §2.2.1.3, p. 23; §3.2.4.1, p. 86; §3.2.4.2, p. 88; example §4.2.2, pp. 124–125 |
| CB-14 | Leave via BYE; abort before the final response via CANCEL | §3.2.4.1, p. 86; example §4.2.3, pp. 125–126 |
| CB-15 | Error codes: 400, 403 (not authorised), 404 (unknown conference), 603 (participant limit) | §3.2.5.2.4, p. 89 |
| CB-16 | Anonymous join per MS-SIPAE §3.2.4.3 (see file 03) | §3.2.4.1.2, p. 87; §3.2.4.2.2, p. 88 |
| CB-17 | Several endpoints of the same user allowed, managed independently | §3.2.5.2.2, p. 89 |
| CB-18 | endorseUser (from Lync 2013): INVITE with Call-ID/From tag/To tag of the dialog of the endpoint to endorse (shared secret, e.g. the same device) ⇒ promotion/lobby admission; the focus ends the dialog, the client sends **no** BYE; 401 without permission | §2.2.3.5–2.2.3.6, pp. 39–40; §3.3–3.3.5.2.1, pp. 89–92; <24> p. 239 |

## Roster subscription

| ID | Requirement | Evidence |
|---|---|---|
| CB-20 | SUBSCRIBE only **after** an existing focus dialog; To = conference URI; `Accept: application/conference-info+xml`; `Supported: ms-benotify`, `ms-piggyback-first-notify` (SHOULD), `com.microsoft.autoextend`; `ms-telemetry-id` (GUID) from Lync 2013 | §2.2.2.1, p. 24; §3.4.3.1, p. 94; example §4.3.1, p. 127 |
| CB-21 | First full notification within **32 s** (otherwise the subscription failed); possibly in the 200 OK (piggyback); unsubscribe via SUBSCRIBE `Expires: 0` | §3.4.2.1, p. 94; §3.4.5.1.1, pp. 96–97; §3.4.6.1, p. 97; §3.4.5.2.1, p. 97 |
| CB-22 | From the first full notification: `conf-uris` ⇒ **MCU conference URI table** (purpose: audio-video, chat, meeting, data-conf, phone-conf, applicationsharing, web-internal/-external) | §2.2.2.4, p. 29; §3.4.5.1.1, p. 97; example p. 128 |
| CB-23 | **Lobby**: own focus endpoint `status` = `on-hold` ⇒ waiting; `connected` ⇒ admitted (then a full notification). A lobby client checks `lobby-capable`; missing/false ⇒ end the dialogs immediately; an own timeout is recommended | §2.2.4.2.1, p. 53; §3.4, pp. 92–93; §3.4.4.1, p. 94; §3.4.5.1.1, p. 97 |
| CB-24 | Notification processing per RFC 4575 §3.7 (state full/partial/deleted, version); the roster contains users/endpoint (session-type = MCU type or "focus", epid, endpoint-uri, roles, authMethod enterprise/anonymous/federated, accessMethod internal/external, media) and `conference-view/entity-view` per focus/MCU (entity-capabilities, entity-state: locked, active, userCount, media …); `participant-count` on the users element (from Lync 2013) | §2.2.2.2–2.2.2.7, pp. 24–32; §2.2.4, pp. 51–54; <31> p. 239 |
| CB-25 | Conference description extensions incl. admission-policy (closedAuthenticated/openAuthenticated/anonymous), autopromote (0, 0x8000 company, 0x80000000 everyone), lobby-capable, pstn-access, recording-allowed, join-url, expiry-time | §2.2.2.3, pp. 27–29 |
| CB-26 | Sensitive data (conference key, web URLs) **never** in the roster – only via `getConference` with an own certificate (public-key encryption) | §3.4.4.2, p. 95; §3.12.4.1, p. 114 |
| CB-27 | Web conferencing MCU (PSOM): `supported-content-type`, `data-mcu-state` (hasContent …; without content no PSOM connection), permission-options/permissions | §2.2.2.8–2.2.2.11, pp. 32–34 |

## Control commands (C3P over INFO)

| ID | Requirement | Evidence |
|---|---|---|
| CB-30 | `<request requestId="<digits>" C3PVersion="1" from=… to=…>` with exactly **one** command; response `<response … code="success|pending|failure" reason=… >` (from/to swapped from the request), optional `diagnostics-info` (ms-diagnostics like MS-OCER) | §2.2.3.1–2.2.3.2.2, pp. 35–37 |
| CB-31 | Flow: INFO → **202 Accepted** (may be ignored) → final C3P response in the 200 OK **or** as INFO from the focus (the client answers 200 OK) | §1.3/§3.5, p. 98; §3.5.5.1.1–3.5.5.1.4, pp. 102–103 |
| CB-32 | Client timer 32 s per command, with "pending" +180 s; match by requestId, drop unknown responses | §3.5.2.1, pp. 99–100; §3.5.5.1.4, pp. 102–103 |
| CB-33 | Authorisation: conference commands presenters only; user commands for third parties presenters only | §3.5.4.2.2, pp. 100–101 |
| CB-34 | Errors: SIP 503 (retryable)/603; C3P unauthorized, timeout, requestMalformed, notSupported, serverBusy, otherFailure, conferenceDoesntExist, userDoesntExist, endpointDoesntExist | §3.5.4.2.3.1–3.5.4.2.3.2, pp. 101–102 |
| CB-35 | Commands: modifyConferenceLock (locked + optional admission-policy/autopromote/pstn-lobby-bypass together), modifyUserRoles, deleteUser (eject; the focus ends subscriptions with reason "ParticipantRemoved"/"Participant Denied" and BYE), deleteConference (ends, does not deprovision), setLobbyAccess (granted/denied; status success/conferenceFull/userDoesntExist/alreadyGranted), modifyEndpoint (e.g. recording state), getConference, modifyConference (MCU state) | §2.2.3.7–2.2.3.28, pp. 40–51; §3.6–3.15, pp. 104–117 |
| CB-36 | **addUser dial-in** (the client joins an MCU): INFO to the focus with `mcuUri` = MCU conference URI, joining-method `dialed-in`, user = own URI; response `connection-info` with `Mcu-Server-Uri`, `Mcu-Conference-Uri` (+ PSOM keys sAuthId, pwrpc.modes tls/fwdtls, pwrpc.port, proxy[N].FQDN/Port, pwrpc.pwsURI, alternativeName); then **INVITE to the MCU** (To/Request-URI = MCU conference URI, body MCU-specific) | §2.2.3.17–2.2.3.18, pp. 46–49; §3.11–3.11.5.1.2, pp. 110–113 |
| CB-37 | **addUser dial-out** (the MCU calls): mcuUri only IMMCU/AVMCU; the MCU sends an INVITE (From = MCU conference URI, Request-URI = refer-to-uri / endpoint-uri / To) | §2.2.3.15–2.2.3.16, pp. 44–46; §3.10, pp. 108–110 |
| CB-42 | **Meeting options** in modifyConferenceLock: after `locked` a `separator`, then `admission-policy` (closedAuthenticated/openAuthenticated/anonymous), `autopromote` (unsigned int: 0 nobody, bit 15 0x8000 company, bit 31 0x80000000 everyone) and `pstn-lobby-bypass` (true/false) – all three or none; `locked` is always included | §2.2.3.9, pp. 41–42 (second example); §2.2.2.3, pp. 26–28; schema `modify-conference-lock-type`, §6.2 |
| CB-43 | The focus reports the options in `conference-description` (`admission-policy`, `lobby-capable` – the admission policy can be changed after activation –, `anonymous-type-allowed`, `autopromote`, `autopromote-allowed`, `pstn-lobby-bypass`, `pstn-lobby-bypass-allowed`) and the lock state in its own `entity-view`/`entity-state/locked` | §2.2.2.3, pp. 26–28; §2.2.4.4.1, p. 49; schema §6.3.2 (confinfoextensions) |

## Simple join

| ID | Requirement | Evidence |
|---|---|---|
| CB-40 | Join URL ⇒ the join manager returns ocsmeet XML (`http://schemas.microsoft.com/rtc/2009/05/simplejoinconfdoc`: conf-uri, conf-key, original-incoming-url, fallback-url, ucwa-url/-ext/-int, telemetry-id); the browser stores an `.ocsmeet` file, the OS starts the registered client | §2.2.6.3, pp. 55–56; §3.2, pp. 84–85 |
| CB-41 | The client can fetch directly via HTTP(S): `Accept: Application/vnd.microsoft.lync.meeting+xml;ver=15.0`, `User-Agent: Lync/15.0 (Windows M.N/XX, YY)` | §2.2.6.1–2.2.6.2, p. 55 |

## Conflicts / gaps
- K-CB-1: Examples in §4 use the namespace `http://schemas.fabrikam.com/rtc/2005/08/confinfoextensions` (pp. 122, 128), syntax/schema `http://schemas.microsoft.com/rtc/2005/08/confinfoextensions` (pp. 38, 53, schema §6.3.2). Authoritative: the schema (microsoft.com) – the examples are wrongly anonymised.
- K-CB-2: Example §2.2.3.4 (p. 39) is not well-formed (`</endpoint>` without an opening tag, clientInfo directly below user).
- K-CB-3: modifyConferenceLock options: the schema references `msci:admission-policy`, `msci:autopromote`, `msci:pstn-lobby-bypass` (confinfoextensions namespace, §6.2), the examples (p. 42, §4.4.1 p. 132) write them without prefix in the request namespace `urn:ietf:params:xml:ns:cccp`. Authoritative: the schema (as K-CB-1); the client sends the `msci:` form.
- O-CB-1: For simple join on Linux, `.ocsmeet` handling or the HTTP fetch (CB-41) needs an own MIME/URL handler; whether the join manager accepts a non-Windows User-Agent is not documented – interop test.
- L-09: RFC 4353/4575/4579/3265/4028/3311/2976 and draft-sipsoap-00 were not in the package.

## Later findings (files 38, 39)
- L-09 **closed**: RFC 4353/4575/4579/3265/4028/3311/2976 and draft-deason-sip-soap-00 (SERVICE method, IE-06) obtained.
- O-CB-1: `?SL=1` on the meet URL forces the Lync Web App (OB-32, official blog). **Secondary evidence** (OB-33): fetch the meet URL via HTTP GET and take the focus URI `conf:sip:…` from the HTML (if needed via `domainOwnerJoinLauncherUrl`); browser schemes `lync:`/`lync15:confjoin?url=`, `conf:sip:`.
- O-CB-1 reduced to one question: the in-band request and the XML response are normative (CB-41, §2.2.6, p. 55); whether the Join Manager answers a non-Windows `User-Agent` is interop test IT-01.

## Implementation (2026-10-31, meetings and lobby)
Joining a meeting by its focus URI or by its meeting link: the link is fetched over verified TLS with the client Accept header of CB-41 (no credentials, no redirects followed) and the `conf-uri` of the ocsmeet document is taken only if it is a focus URI (CB-40, §2.2.6.3; example recomputed as a test); our own User-Agent is sent (O-CB-1 stays open: interop test IT-01). addUser announces `clientInfo/lobby-capable` as in the example of §3.2.4 (CB-11); our focus endpoint `on-hold` is the lobby, `connected` the admission (CB-23, example §3.4); audio cannot be joined while waiting; after 15 minutes in the lobby we leave (CB-23 recommends an own timeout; design). Status: tested-offline.

## Implementation (2026-10-31, presenter commands)
Over INFO in the focus dialog (CB-31) with the bodies of the specification examples: setLobbyAccess granted/denied for named users or everyone waiting (§2.2.3.21, example §4.4.8), modifyUserRoles (§2.2.3.7), modifyConferenceLock with the lock state only (§2.2.3.9), deleteUser (§2.2.3.11); the client sends them only with the presenter role in the roster (CB-33; the focus decides anyway) and reports every refused command. Not implemented: admission policy/autopromote changes, deleteConference, recording. Status: tested-offline.

## Implementation (2026-10-31, meeting options)
`conf::modify_conference_options` (CB-42, K-CB-3) and `conf::Settings` from the roster (CB-43). Before sending, the session refuses what the focus says it does not allow (`lobby-capable` false, anonymous not allowed, autopromote bits outside `autopromote-allowed`, lobby bypass not allowed); locking sends the known options along so they stay as they are. CLI `/options [open|closed|anonymous] [promote none|company|everyone] [phone bypass|lobby]` and a line per settings change. Not implemented: deleteConference, recording. Status: tested-offline.
