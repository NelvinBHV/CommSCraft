# MS-E911WS rev9 — HTTPS/SOAP location lookup, network identifier precedence, PIDF-LO and errors

**Source-evidence tranche 2026-10-23 — NOT complete E911/PSTN emergency call reconstruction.** Microsoft original **[MS-E911WS] Web Service for E911 Support Protocol**, revision **9.0**, released **2025-05-20** (`v20250520`), archived source PDF SHA-256 **999ebc01f437d6bbe8b1141e479a94d1ecec30c61fa4491367e1c1d5f032920a**, 38 pages, official historical-original link https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-E911WS/%5BMS-E911WS%5D.pdf. Direct original clauses **§§1.3–1.6,2.1,2.2.3–2.2.5.1,3.1.1–3.1.6,5.1**; original PDF pp17–18 rendered and page18 visually inspected. Cited first-party dependencies: **[MS-NLMP]**, **[MS-OCAUTHWS]**; RFC citations **[RFC2616](https://www.rfc-editor.org/rfc/rfc2616)** HTTP1.1, **[RFC2818](https://www.rfc-editor.org/rfc/rfc2818)** historical HTTPS, **[RFC3863](https://www.rfc-editor.org/rfc/rfc3863)** PIDF, **[RFC4119](https://www.rfc-editor.org/rfc/rfc4119)** PIDF Location Object, **[RFC5139](https://www.rfc-editor.org/rfc/rfc5139)** revised civic PIDF-LO, **[RFC4559](https://www.rfc-editor.org/rfc/rfc4559)** SPNEGO HTTP Negotiate; **[RFC2119](https://www.rfc-editor.org/rfc/rfc2119)** requirement words; **[MS-OCSPROT] rev9 §2.5.4** Windows login reference use-case; SOAP1.1/WSDL/IEEE802.1AB also referenced. **P** primary clause, **R** RFC normative layer, **S** logically combined flow, **U** deployment feature/version.

## 1. Message encoding, identities, security and protocol status

| ID | First-party clause | Source-constrained rule |
|---|---|---|
| E911-01 | §§1.3,2.1 | The **Location Information Service (LIS)** answers SOAP 1.1/WSDL messages **over HTTPS**. Network address may come from a different channel including SIP provisioning. LIS is **distinct from SIP Registrar and PSTN gateway**; it does **not** dial a public safety answering point. |
| E911-02 | §3.1.4.1 | Client **MUST establish TCP and negotiate SSL** (historical source terminology) before SOAP HTTP `GetLocationsRequest`. **TLS server-identity, chain trust and auth** require HTTPS + Windows security policy; historic use of word SSL is **not permission** for obsolete insecure SSL ciphers/protocols. |
| E911-03 | §5.1 | LIS accepts authenticated clients via **Kerberos v5 or NTLM**, SPNEGO-based HTTP auth under RFC4559, or custom certificate authentication **MS-OCAUTHWS**. **SIPAE TLS-DSK credential negotiation is not the same** as HTTPS user authentication; no arbitrary token reuse. |
| E911-04 | §3.1.4.1.2.1 | `GetLocationsRequest` SOAP body has **required `Entity` URI**, optional `WAPBSSID`, `RSSI` (unsignedByte), `MAC`, `ChassisID`, `PortID`, `SubnetID` and `IP` in documented XML element order. ChassisID/PortID use IEEE802.1AB binary TLVs. These are **network identifiers** with privacy implications, not mandatory GPS coordinates. |
| E911-05 | §§3.1.1,3.1.4.1 | LIS's conceptual directory matches WLAN BSSID, chassis/port or network addresses. Example server lookup priority **WAPBSSID → ChassisID+PortID → ChassisID → SubnetID → MAC** is expressly **implementation-specific**, and must not become a client-ordered MUST or a guaranteed location accuracy ranking. |
| E911-06 | §3.1.4.1.1.1 | Endpoint can send GetLocations after sign-in or on AP change, but the **exact triggering point is implementation-specific**. A normal IM-only sign-in cannot be universally blocked by LIS response without additional provisioning evidence. |
| E911-07 | §§3.1.4.1.1.2,3.1.4.1.2.2 | `GetLocationsResponse` SOAP body contains **exactly one `ReturnCode`** and optional `presenceList`; failed match means **no presenceList**. LIS response is HTTP **2xx even when the SOAP `ReturnCode` signals failure**; do not equate transport success with correct emergency location. |
| E911-08 | §2.2.5.1 | `ReturnCodeType` is **an enumerated XML string** with four values: `200` success, `400` bad request, `404` not found, `500` internal server error. Not a SIP status, HTTP status or MCU `ms-diagnostics-public` code. |
| E911-09 | §§2.2.4.1,3.1.4.1.1.2 | `presenceList` is 0..N **PIDF `pidf:presence`** records (RFC3863), extended with **GEOPRIV location object** (RFC4119) whose **civic location-info conforms to RFC5139**. Client specifically consumes country/region, A1, A3, PRD, RD, STS, POD, HNO, HNS, LOC, NAM, PC, not arbitrary extension fields. |
| E911-10 | §2.2.4.1 | If the location **cannot be trusted to match network identifiers**, GEOPRIV `method` **MUST be `Manual`**. This is an explicit **source trust-confidence semantic**, *not* blanket authorization to silently guess the precise street or dispatch location. |
| E911-11 | §§3.1.4.2.1.1,3.1.4.2.2.1 | On **user-initiated city search**, `GetLocationsInCityRequest` requires `Entity`, **`Country`** (ISO3166 alpha-2), **`State`**, **`City`**; distinct operation from network-location GetLocations. The city request contains no mandatory WAP/MAC/LLDP identifier. |
| E911-12 | §§3.1.4.2,3.1.4.2.2.2 | City lookup's response likewise includes **required ReturnCode** and **optional PIDF presenceList**, with errors carried in SOAP body inside HTTP **2xx**. If no match, do not infer the civic location from geographical IP text. |
| E911-13 | §§3.1.1–3.1.2 | LIS original conceptual model **does not retain per-client state across requests** and declares **no protocol-specific timers**. Endpoint can still have caching/renewal requirements from client provisioning and operating system; no global fixed LIS refresh timer proven by this document. |
| E911-14 | §§1.6,2.2.4.1; MS-OCSPROT §2.5.4 | Locations can be used for an **E911 call** and optionally published in presence according to policy. The **LIS response is not proof** PSTN route to correct PSAP is accepted or an emergency INVITE has been delivered. Explicit E911 calling/routing and privacy policies are separate. |
| E911-15 | §§3.1.4.1–2,5.1 | HTTPS network TLS failure, client authentication failure, SOAP schema failure, `ReturnCode=404`, and presence candidate selection are **distinct failure classes**. The source has no universal instruction to keep using an old cached location or prompt a user on any specific one without configuration/product evidence. |

## 2. Partial-order state model (S, original operations conditional)

~~~mermaid
flowchart TD
 A[MS-SIPREGE profile: LocationRequired / LIS endpoint if enabled] --> B{Location procedure chosen}
 B -->|Network change or login when applicable| C[GetLocations: Entity and optional BSSID/LLDP/subnet]
 B -->|User city search| D[GetLocationsInCity: Entity Country State City]
 C --> E[HTTPS TLS and authenticated SOAP]
 D --> E
 E --> F{Transport/auth successful?}
 F -->|No| G[LIS transport/auth failure; no invented dispatch result]
 F -->|Yes| H[HTTP 2xx with SOAP response body]
 H --> I{XML ReturnCode == 200?}
 I -->|No| J[SOAP error 400/404/500, presenceList absent]
 I -->|Yes| K[Zero or more returned PIDF civic location records]
 K --> L[Select/confirm location per documented client profile and trust]
 L --> M[Local emergency location state may be used for presence or E911 call]
 M --> N[Separate SIP PSTN emergency routing not specified by E911WS]
~~~

**Critical privacy/security boundary:** These identifiers (WLAN MAC/BSSID/LLDP, IP, civic building details) are sensitive location evidence. The LIS is an authenticated, trusted service and must not be replaced by guessing geolocation based on a reachable HTTP endpoint. A successful certificate handshake alone says nothing about correctness of civic address.

## 3. Product-era and unresolved requirements

1. **E911-U1:** Original [MS-OCSPROT] §4.1 note <16> excludes OCS2007/2007R2; determine supported client edition, on-prem E911 policy and legacy location provisioning. *MS-E911WS describes LIS operation and cannot by itself reconstruct full emergency dialing or PSAP routing.*
2. **E911-U2:** LIS `entity` identity and authorization for network-derived locations, client certificate use and trust of `alternativeName` elsewhere must be audited per HTTP auth/Windows platform. An HTTPS URL is not identity proof without certificate validation.
3. **E911-U3:** IP not included in illustrative server priority list even though request allows `IP`; matching order explicitly implementation-specific. Do not falsely declare IP unsupported.
4. **E911-U4:** Explore how multiple PIDF records, explicit `method=Manual`, expiry and user selection are handled by *specified Windows client product and provisioning*; original LIS alone leaves client choice underdefined.
5. **E911-U5:** Error status 400/404/500 in SOAP result vs HTTP 2xx may lead to stale-state hazards. Need exact failure policy, security audit and privacy publication controls; no safety-critical assumption.
6. **E911-U6:** E911 numeric location info is distinct from **MS-OCPSTN `phone-context`** (dialing profile) and **MS-TURNBWM `Location Profile`** (intranet/Internet/federation site). Same word “location” does not make values interchangeable.
7. **E911-U7:** Full RFC5139 civic namespace/validation and all WSDL types in §6 remain to be fully audited, including schema cardinality, historic publication differences and deployment overrides.

**Status: source-selected sections verified, overall normative Full Client and emergency-service reconstruction INCOMPLETE. No code, live servers, interoperability tests, merge or release.**
