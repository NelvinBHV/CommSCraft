# Evidence: MS-SIPAPP – call park (ms-call-park)

Source: [MS-SIPAPP] "Session Initiation Protocol (SIP) Application Protocol", edition 20250520, 40 pp. Based on RFC 3515 (REFER), 3891 (Replaces), 3840 (feature tags) (obtained, file 38).

| ID | Requirement (client = parker or retriever) | Evidence |
|---|---|---|
| CK-01 | Park: new **INVITE** to the call park server URI from in-band provisioning (`callParkServerUri`, MS-SIPREGE §2.2.2.5.1, file 04) with `Content-Type: application/ms-call-park+xml`, body `<park-request version="1.0" request-id="…"><audio><dialog-info><call-id/><from-tag/><to-tag/></dialog-info><target>GRUU of the peer</target></audio></park-request>` (namespace `http://schemas.microsoft.com/rtc/2008/12/callpark`) | §2.1.1, p. 10; §2.2.1.1, pp. 10–11; §3.1.5.1, pp. 15–16; schema §6.1; example pp. 21–22 |
| CK-02 | The CPS replaces the peer's call via an **INVITE with Replaces** (Contact with feature tag `isCps`, SDP `a=sendonly` + `a=feature:MoH`); success ⇒ 200 OK to the parker with `<park-response>`: `orbit` (1–9 digits or `*`/`#` + 1–8 digits) and `ms-parked-call` | §2.2.1.2, pp. 11–12; §3.1.5.1–3.1.5.2, p. 16; example pp. 22–24 |
| CK-03 | Errors: 415 (schema), 488 + ms-diagnostics 35008 (major version ≠ 1), 500 + 35002 (no free orbit), 408 + 35007 (replace failed) | §3.1.5.1, §3.1.5.3, p. 16 |
| CK-04 | The parker↔CPS dialog stays; feedback via **INFO** `<unpark-notification>` with reason retrieval/hang-up/ringback/fallback/drop (+ target), then BYE from the CPS (ms-diagnostics 35014); a BYE from the parker only ends the notifications | §2.2.1.3, pp. 12–13; §3.1.5.4–3.1.5.12, pp. 16–19 |
| CK-05 | Retrieve: a normal audio INVITE to the orbit number (like a phone number with phone-context, file 24); the CPS answers, then a **REFER with Replaces** to the retriever (Refer-To = GRUU of the parked party) ⇒ the client must support REFER/Replaces; 404 + 35000 for an empty orbit, 488 during a retrieval in progress | §3.1.5.6, p. 17; example §4.1.2, pp. 24 ff. |
| CK-06 | Ringback (auto ringback): the CPS sends a REFER to the parked party with Refer-To = parker and an embedded header `ms-sensitivity=private-no-diversion` | §3.1.6, p. 19 |

## Relevance for L-13 (file 24)
Call park is therefore **documented**. The flows also show that an SfB-compatible client must handle an **INVITE with Replaces** (as the parked party) and a **REFER with Replaces** (as the retriever); "hold" is documented in MS-SDPEXT (`a=inactive` + `sip.rendering="no"`, file 14 SD-06; the CPS itself uses `a=sendonly` + `a=feature:MoH`, SD-07).

## Conflicts
- K-CK-1: The park-request example (pp. 21–22) has no namespace declaration, the park-response and the schema use `http://schemas.microsoft.com/rtc/2008/12/callpark`. Implementation: set the namespace; check by interop.
- K-CK-2: In the park response example the Request-URI of the INVITE is `sip:callparkservice.contoso.com@contoso.com;gruu;opaque=srvr:Microsoft.Rtc.Applications.Cps:…` – the URI comes from provisioning, do not hard-code it.
