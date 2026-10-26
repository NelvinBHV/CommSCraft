# Evidence: MS-AVEDGEA – media relay authentication (TURN credentials)

Source: [MS-AVEDGEA] "Audio Video Edge Authentication Protocol", edition 20250520, 40 pp. Applies up to SfB LTSC 2024 (§7, p. 36).

| ID | Requirement | Strength | Evidence |
|---|---|---|---|
| AV-01 | Target = `mrasUri` from in-band provisioning (form `sip:<fqdn>@<domain>;gruu;opaque=srvr:MRAS:<id>`); transport SIP SERVICE over TLS | MUST | §2.1, p. 11; SIPREGE §2.2.2.5.1, p. 27; example §4.1.1, p. 25 |
| AV-02 | `Content-Type: application/msrtc-media-relay-auth+xml`; namespace `http://schemas.microsoft.com/2006/09/sip/mrasp` | MUST | §2.2, p. 11 |
| AV-03 | `<request requestID version="3.0|2.0|1.0" from="sip:…" to="<mrasUri>" [route="loadbalanced|directip"]>` with 1–100 `<credentialsRequest credentialsRequestID>` (`identity`, optional `location` intranet/internet, `duration` in minutes, default 480) | MUST | §2.2.2, pp. 11–13; §3.1.5.2.1, p. 22 |
| AV-04 | Response `<response … reasonPhrase="OK" serverVersion="3.0">` with `<credentialsResponse>`: `credentials` (username, password – Base64, `duration`, optional `realm`) and `mediaRelayList/mediaRelay` (location, hostName **or** directIPAddress IPv4/IPv6, udpPort default 3478, tcpPort default 443) | MUST/SHOULD | §2.2.3, pp. 13–18; §1.5, p. 9; examples pp. 25–28 |
| AV-05 | Without `location` both intranet and internet relays are returned; `directip` returns direct IP addresses | MUST | §3.1.5.4, pp. 22–23 |
| AV-06 | Token lifetime = min(requested, server configuration); the token contains a hash of identity and lifetime (HMAC-SHA-1/SHA-256 with keys shared between Edge and TURN) – the client treats username/password as opaque | – | §1.3, p. 8; §3.1.5.7, p. 24; §5.1.1, p. 29 |
| AV-07 | Errors: 400 Request Malformed, 403 Forbidden, 413 Request Too Large, 500 Internal Server Error, 501 Version Mismatch / UnsupportedMessageType, 415 UnsupportedContentType (+ `Accept`) | – | §3.1.5.6, pp. 23–24 |
| AV-08 | On a version conflict the server returns the highest supported lower version | SHOULD | §3.1.5.2.2, p. 22 |
| AV-09 | Retrieval during bootstrap (OCS-UC03 step 11) and before every call through the Edge (OCS-UC12 step 1) | – | OCSPROT §2.5.3, p. 31; §2.5.12, p. 40 |

## Conflicts
- K-AV-1: §2.2.2.1.2 (pp. 11–12) lists `route` as an **attribute** of `requestType`; example §4.2.1 (p. 27) shows `<route>directip</route>` as a **child element** of `credentialsRequest`. Likewise `location`/`duration` are called "attribute" once in the table on p. 13 but sent as elements in the example. **Checked against the schema §6 (pp. 30–32):** `route` is an attribute of `request` (optional, default `loadbalanced`); `identity`, `location`, `duration` are elements of `credentialsRequest`. The example §4.2.1 with `<route>` as a child element is therefore invalid against the schema; the implementation follows the schema (`<request … route="directip">`).
