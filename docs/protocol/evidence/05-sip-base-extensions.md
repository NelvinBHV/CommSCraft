# Evidence: MS-SIP – SIP base extensions (batch SUBSCRIBE, piggyback, BENOTIFY, auto-extend, contact list)

Source: [MS-SIP] "Session Initiation Protocol Extensions", Windows package, edition 20170601, 93 pp.

**Important restriction:** According to Appendix F (p. 89) this document applies only to Windows 2000 SP4, Windows XP, Windows Server 2003 (Windows Messenger/RTC client). It is still authoritative for Skype for Business because MS-SIPREGE defines the following mechanisms explicitly by reference to MS-SIP (SIPREGE §1.4 p. 13; §2.2.1.5 p. 15; §2.2.2.2 p. 20; §3.1.1.5.1 p. 76; §4.2 p. 97). The presence document format (`msrtc.pidf`) and `setPresence`/`getPresence` of this document are the **old** model; SfB uses MS-PRES (categories) instead. Only the mechanisms adopted by reference are recorded here as relevant for SfB.

| ID | Mechanism | Requirement | Evidence |
|---|---|---|---|
| SIP-01 | Batch SUBSCRIBE | SUBSCRIBE to the own AOR (Request-URI = To = From = watcher), `Event: presence`, `Require: adhoclist`, `Supported: eventlist` (both needed), `Content-Type: application/adrl+xml`, body `<adhoclist uri=… name=…><create|add|delete><resource uri="…"/>…`; namespace `urn:ietf:params:xml:ns:adrl`; flat list, server limit typically 100–250 | §3.3.4.1, pp. 30–33 |
| SIP-02 | Batch NOTIFY | `multipart/related; type="application/rlmi+xml"`, first part the RLMI list (`version`, `fullState`, `resource`/`instance cid`), further parts per contact by Content-ID; the client SHOULD ignore CSeq order and use `version` | §3.3.5.1, pp. 33–35 |
| SIP-03 | Batch fallback | Error response ⇒ individual SUBSCRIBEs | §3.3.5.2, p. 35 |
| SIP-04 | Piggyback | `Supported: ms-piggyback-first-notify` in the SUBSCRIBE; if it appears in the 200, treat the 200 content like the first NOTIFY | §3.4.4.1, §3.4.5.1, pp. 36–37 |
| SIP-05 | BENOTIFY | `Supported: ms-benotify` + `Proxy-Require: ms-benotify`; **never answer** a BENOTIFY, process its content like a NOTIFY; keep accepting regular NOTIFYs; on error retry without the headers | §3.5.1.1–3.5.5.3, pp. 38–39 |
| SIP-06 | Auto-extend | `Supported: com.microsoft.autoextend`; if confirmed in the 200, every NOTIFY resets the subscription expiry timer | §3.6.4.1–3.6.5.2, pp. 40–41 |
| SIP-07 | Subscribe to the contact list (old model) | `Event: vnd-microsoft-roaming-contacts`, `Accept: application/vnd-microsoft-roaming-contacts+xml`; `<contactList deltaNum=…>` with `group id/name/externalURI` and `contact uri/name/groups/subscribed/externalURI`; deltas `contactDelta` with `deltaNum`/`prevDeltaNum` | §2.2.4.1, pp. 17–18; §3.7.4.1, p. 43; §3.7.5.2, pp. 44–45 |
| SIP-08 | Change contacts/groups (old model) | SOAP `setContact`, `deleteContact`, `addGroup`/`modifyGroup`, `deleteGroup` in a SERVICE to the own AOR, each with the current `deltaNum`; group ID 1–63, group 1 is the default and cannot be deleted | §2.2.4.2, pp. 18–19; §3.7.4.3–3.7.4.4, p. 44 |
| SIP-09 | ACL (old model) | `Event: vnd-microsoft-roaming-ACL`, `application/vnd-microsoft-roaming-acls+xml`; ACE rights as 2 characters (see, communicate) from A/D/P/B; **the client enforces the communication right (INVITE)**, the server the presence right | §2.2.3, pp. 16–17; §3.7.4.2, pp. 43–44; §3.7.5.1, p. 44 |
| SIP-10 | getPresence (polling) | SOAP `getPresence` in a SERVICE, namespace `http://schemas.microsoft.com/winrtc/2002/11/sip`; 403 when blocked by ACL | §3.2.4/§3.2.5.1, pp. 28–29 |

## Open question
- O-SIP-1: Whether an SfB client still manages contacts via SIP-07/SIP-08 (roaming-contacts, SOAP) or only via MS-PRES (categories `contacts`/`groups`, self subscription) is not decided here. To be clarified in MS-PRES (evidence file 07).

## Later finding (file 39)
- O-SIP-1 **closed** (OB-03, normative: PR-50/PR-51).
