# Evidence: MS-E911WS – location information service for emergency calls

Source: [MS-E911WS] "Web Service for E911 Support Protocol", edition 20250520, 38 pp. Products Lync 2010 up to SfB LTSC 2024 (p. 34). URL from in-band provisioning `lisInternalUrl` (file 04); use of the location in emergency calls (PIDF-LO in the INVITE, locationPolicy) see files 04/06.

| ID | Requirement | Evidence |
|---|---|---|
| E9-01 | SOAP 1.1 over **HTTPS**; port type ILIService; namespace `urn:schema:Microsoft.Rtc.WebComponent.Lis.2010`; actions `LIService/GetLocations`, `LIService/GetLocationsInCity` | §1.4, p. 9; §2.1–2.2.1, p. 11; §3.1, §3.1.4.1, pp. 15–16; §3.1.4.2, p. 18 |
| E9-02 | Authentication: Kerberos/NTLM (HTTP Negotiate, RFC 4559) or certificate-based per MS-OCAUTHWS (file 09) | §5.1, p. 25 |
| E9-03 | **GetLocationsRequest**: Entity (own SIP URI, 1–64 characters), optional WAPBSSID, RSSI, MAC, ChassisID/PortID (LLDP TLV as Base64, IEEE 802.1AB), SubnetID, IP – in this order; send after sign-in and when the WLAN access point changes (implementation-dependent) | §3.1.1, p. 15; §3.1.4.1.1.1–3.1.4.1.2.1, pp. 16–17 |
| E9-04 | Response: ReturnCode 200/400/404/500 (always inside HTTP 2xx) + presenceList of `pidf:presence` with a GEOPRIV location object, civicAddress (RFC 5139); the client evaluates only country, A1, A3, PRD, RD, STS, POD, HNO, HNS, LOC, NAM, PC; method "Manual" ⇒ the address is not verified by the network | §2.2.3–2.2.5.1, pp. 12–14; §3.1.4.1.2.2, p. 18; example §4, p. 23 |
| E9-05 | **GetLocationsInCityRequest**: Entity, Country (ISO 3166 alpha-2), State (exactly 2 characters), City (1–64) – for manual location choice | §3.1.4.2–3.1.4.2.4.2, pp. 18–21 |
| E9-06 | Locations may also be used for presence publication | §1.6, p. 10 |

## Conflicts / gaps
- K-E9-1: The examples (§4, pp. 23–24) show GetLocationsRequest without a target namespace (no xmlns on tns), the schema defines `urn:schema:Microsoft.Rtc.WebComponent.Lis.2010`. Implementation: use the schema namespace, check by interop.
- L-14: RFC 3863/4119/5139 (PIDF, GEOPRIV, civic address) and IEEE 802.1AB (LLDP) were not in the package. On Linux, LLDP neighbour information (chassis/port) and the BSSID have to be obtained from the system (e.g. lldpd, NetworkManager) – an implementation detail, not protocol-relevant.

## Later finding (file 38)
- L-14 **closed** for IETF: RFC 3863/4119/5139 obtained; IEEE 802.1AB (not IETF) still not.
