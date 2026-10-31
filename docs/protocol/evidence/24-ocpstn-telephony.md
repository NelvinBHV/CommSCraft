# Evidence: MS-OCPSTN – SIP extensions for telephony (PSTN/PBX)

Source: [MS-OCPSTN] "Session Initiation Protocol (SIP) for PSTN Calls Extensions", edition 20250520, 63 pp. Product list p. 58 up to SfB LTSC 2024. Based on RFC 3261/3263/3325/3515/3891/3892/3966/3262 (obtained, file 38).

| ID | Requirement (client view) | Evidence |
|---|---|---|
| PS-01 | Dialling a number: if the client cannot resolve the dial string to E.164 via dial rules (location profile, MS-SIPRE/MS-SIPREGE, files 04/06) ⇒ Request-URI `sip:<number>;phone-context=<LocationProfileName>@<domain>;user=phone`; without a profile `phone-context=dialstring`; `enterprise` = non-E.164 result of enterprise rules | §2.2.2, p. 13; §3.2, pp. 16–17 |
| PS-02 | Call back: use the P-Asserted-Identity of the incoming call (possibly with phone-context dialstring/enterprise) for Request-URI and To | §3.2, p. 17 |
| PS-03 | From with user=phone: E.164 or a private number with phone-context = own location profile | §3.2, p. 17 |
| PS-04 | The peer is a gateway, recognisable by the Contact parameter `;isGateway` (display in the UI) | §2.2.1, p. 13; §3.1, p. 16 |
| PS-05 | Incoming INVITE with `Ms-Call-Source` (ms-rtc/non-ms-rtc) while another dialog is ringing ⇒ **605** (loop detection); the client itself never sets the header | §2.2.3, p. 14; §3.3, p. 18 |
| PS-06 | Early media: `Supported: ms-early-media` ⇒ the gateway sends the SDP answer in an **unreliable 183**; flow MS-SDPEXT §3.1.5.12 (file 14) | §2.2.4, p. 14; §3.4, p. 19 |
| PS-07 | Anonymous caller = `sip:anonymous@<host>;user=phone` (not anonymous.invalid) | §2.2.5, p. 14; §3.5, p. 20 |
| PS-08 | Media bypass: `Supported: ms-bypass` when supported (from Lync 2010) | §2.2.6, p. 14; §3.6, p. 20 |
| PS-09 | Multiple SDP offer: every application/sdp and application/gw-sdp part gets a **Content-ID**; the answerer sets `ms-accepted-content-id: <Content-ID>` of the chosen part | §2.2.7, p. 14; §3.7, p. 21 |
| PS-10 | Informative only (gateway role): ms-trunking-peer, ms-mediation-generated | §2.2.8–2.2.9, p. 15; §3.8–3.9, pp. 22–23 |

## Notes
- Forwarding, simultaneous ring, team/delegate calls: documented in MS-SIPRE (file 06, RE-50 ff.); safe transfer `ms-safe-transfer` (RE-64); correlation `Ms-Correlation-Id` (RE-62).
- **Call park** is documented in MS-SIPAPP (file 31); music on hold `a=sendonly`+`a=feature:MoH` in MS-SDPEXT (file 14, SD-07).
- **Hold** is documented in MS-SDPEXT: all A/V streams `a=inactive` + Contact `sip.rendering="no"` (file 14, SD-06; feature tag file 06, RE-68).
- **L-13 (partly):** The client flow for **transfer** (blind/consultative: REFER/Replaces/Referred-By) is not described as a separate MS extension in the package (only parts are documented: `ms-referee-uri` AE-14/AE-37, `ms-safe-transfer` RE-64, `Ms-Correlation-Id` RE-62, REFER/Replaces when parking CK-05); otherwise there are only references to RFC 3264/3515/3891/3892 (§1.4, p. 11) and error codes in MS-OCER (e.g. 7016 "Call cannot start on hold", 7017, 24075 "Call park operation failed", call park ErrorIds 35000–35999, §7.25).
- Legal note of all documents (p. 1, "Intellectual Property Rights Notice"): use for implementation is permitted, Microsoft patents may apply; check the Open Specifications Promise/Community Promise per document.

## Later findings (files 38, 39)
- L-13: base RFCs 3515/3891/3892 obtained.
- L-13 **largely documented** (OB-25, official UCMA reference for Lync 2013): attended (completed after a successful REFER NOTIFY), supervised (REFER with Replaces for the consultation call), unattended (ends after REFER Accepted).
- OB-37 (secondary): transferred or forwarded calls from the Mediation Server carry History-Info or Referred-By.
- L-13: the transfer flow is officially documented (OB-25); the Skype for Business specific headers are interop test IT-06.
