# Evidence: MS-OCER – diagnostic headers, error reports to the server, error display

Source: [MS-OCER] "Client Error Reporting Protocol", edition 20250520, 213 pp.

| ID | Requirement | Evidence |
|---|---|---|
| ER-01 | The server supplies `ms-diagnostics: <ErrorId>;reason="…";source="…"*(;param)` (at most one per response, also in BYE/CANCEL); the client evaluates the **ErrorId**, not the reason text; the client itself never sends this header | §2.2.1–2.2.1.1, pp. 11–12 |
| ER-02 | `ms-diagnostics-public` (without source) for federated/anonymous users; evaluated the same way | §2.2.1.2, p. 13 |
| ER-03 | Generic parameter `Setup_time` (ms, measured locally, only for established calls) | §2.2.1.1, p. 12 |
| ER-04 | The client can collect ms-diagnostics from provisional responses (e.g. `101 Progress Report`) and report a failure via **SERVICE** to the own URI: `Content-Type: application/msrtc-reporterror+xml`, `<reportError xmlns="http://schemas.microsoft.com/2006/09/sip/error-reporting"><error callId requestType responseCode [fromUri toUri fromTag toTag contentType]><diagHeader/><progressReports><progressReport><diagHeader/>…` (length limits: requestType ≤33, contentType ≤257, diagHeader ≤65535, URIs+tags+Call-ID ≤669 characters) | §2.2.2, pp. 14–15; §3.1.4.1, pp. 16–17; example §4.1, pp. 19–21 |
| ER-05 | Responses to reportError: 200 OK; 503 without body or with ms-diagnostics 2019 ⇒ do not retry in this session; 503 with Retry-After ⇒ later; 413 ⇒ do not retry; REGISTER first | §3.1.3, p. 16; §3.1.5, p. 17 |
| ER-06 | Error code catalogues per component (Lync 2010 to SfB 2019): 0–39 public (incl. 4/5/6 bandwidth, 21–39 media connectivity), 1000+ SIP stack, presence, conferencing, IM MCU, AV MCU, AV Edge auth, mediation, VoIP routing, app sharing MCU (incl. 21024–21026 VBSS→RDP fallback), UCMA, call park (35000–35999) etc. | §7.1–7.26, pp. 25–127; §8 (older versions), pp. 128–167 |
| ER-07 | Client display logic (Lync 2013 ff.): table mode (conference/IM/voice/sharing)/ErrorId → message text; unhandled codes ⇒ a generic message with "error identifier [ID] (source ID 243)" | §9.1–9.3.2, pp. 168–192; older versions §10, pp. 193–204 |

## Notes
- The catalogues (ER-06/ER-07) are directly usable as a data table for CommSCraft's error display; the English texts are Microsoft product texts – adopt only the meaning and write our own texts.
- For interop tests the ms-diagnostics codes are the most important source of error information; the client should log them completely.
