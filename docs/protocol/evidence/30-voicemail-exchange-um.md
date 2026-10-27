# Evidence: voicemail / Exchange Unified Messaging – MS-OCEXUM, MS-EUMR, MS-EUMSDP

Sources: [MS-OCEXUM] "Call Control for Exchange Unified Messaging Protocol Extensions", edition 20250520, 20 pp.; [MS-EUMR] "Routing to Exchange Unified Messaging Extensions", edition 20250520, 29 pp.; [MS-EUMSDP] "Exchange Unified Messaging Session Description Protocol Extension", edition 20250520, 17 pp.

## Client-relevant

| ID | Requirement | Evidence |
|---|---|---|
| UM-01 | Listen to voicemail (subscriber access): INVITE to the own voicemail GRUU `sip:<me>;opaque=app:voicemail` (syntax voice-mail-gruu: MS-SIPRE §2.2.3, file 06) over TLS with **`Ms-Exchange-Command: skip-pin`** (SHOULD) ⇒ no PIN prompt | OCEXUM §2.2.1, p. 8; §3.1, p. 10; example §4.1, p. 13; EUMR §3.1, p. 15 |
| UM-02 | Play on phone: UM calls the user with `Ms-Sensitivity: private-no-diversion`; From = To (own URI) ⇒ the client rings **even when "Appear offline"**; no diversion | OCEXUM §2.2.2, pp. 8–9; §3.2, p. 11 |
| UM-03 | SDP towards UM: m=audio only (video/app sharing rejected), the o-line version may increase, 18x and final answer identical; ignored: x-bypassid/x-bypass/x-mediasettings, rtcp-rsize/rtcp-fb, x-ssrc-range, x-source(-streamid); DTMF per MS-DTMF (file 17) | EUMSDP §1.3, p. 7; §3.1.5, p. 10; example §4, p. 12 |
| UM-04 | Provisional response `101 Progress Report` possible when forwarding to the next UM server (the client ignores/logs it, cf. MS-OCER) | EUMR §3.1.5.4, p. 17 |

## Server side only (for context)
- UM-10: Proxy routing to UM (Request-URI `<dial plan>@<UM FQDN>:5061;transport=tls;maddr=…`, Diversion, `Ms-Mras-Address`, option tag `Ms-Fe`, 302/415 handling, timer ≤180 s, recommended 5 s) – EUMR §2.2.1–2.2.6, pp. 11–14; §3.1.5, pp. 15–17.
- UM-11: Notification mails (missed/answered/forbidden) via INFO `application/ms-rtc-usernotification+xml` from the proxy to UM (From `sip:A410AA79-D874-4e56-9B46-709BDD0EB850`) – EUMR §2.2.3–2.2.5, §3.2, pp. 11–20.

## Gaps
- L-15: The voicemail **list**/settings run over Exchange web services (MS-OXWUMS "Voice Mail Settings Web Service", reference OCEXUM §1.2.1, p. 6) – part of the Exchange integration (file 37).

## Later finding (file 39)
- L-15 **narrowed** (OB-23): voicemail message classes `IPM.Note.Microsoft.Voicemail.UM(.CA)` (MS-OXOUM §2.2.2.1, p. 10); MS-OXWUMS only covers playing on the phone.
- L-15 **closed** (OB-48): the voicemail list is an EWS FindItem on the documented message class (MS-OXOUM, OB-23) at the Autodiscover `EwsUrl`.
