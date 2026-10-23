# MS-OCAUTHWS rev11: certificate provisioning, WS-Trust Web tickets, token audiences and per-service identity faults

**Research original 2026-10-23; not every implementation path obligatory.** Microsoft **[MS-OCAUTHWS] rev11.0, version v20250520, May 20 2025**, verified SHA256 `a57de2759abca016437d728020fcefae4f687512274f3f8eaa9f91aaff6a17a3`, first-party https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OCAUTHWS/%5BMS-OCAUTHWS%5D.pdf . Original sections §1.3–1.7,§2.1–2.2.6,§3.1.1–3.1.5,§3.1.4.1.1–3.1.4.1.5,§3.2–3.2.4.1.3.4, select §4 examples; PDF pp25–28/30–33/36–37 locally rendered. Dependencies **[MS-WSTEP]** certificate WS-Trust extension, **[MS-WSPOL]** Web policy, **[MS-WCCE]** PKCS#10 Microsoft certificate issuance, **[MS-SIPAE] rev11**, **[MS-SIPRE] rev13**, **[MS-OAUTH2EX]** OAuth profile, **[MS-OCER]** diagnostics; original [RFC2818](https://www.rfc-editor.org/rfc/rfc2818) HTTPS, [RFC4559](https://www.rfc-editor.org/rfc/rfc4559) HTTP SPNEGO Kerberos/NTLM (2006 informational), [RFC5280](https://www.rfc-editor.org/rfc/rfc5280) X.509 2008, [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) SIP, WS-Trust1.3/OASIS, SOAP1.2, WSS 1.1 and SAML. **P** primary Microsoft source clause+actor, **R** first-party RFC, **S** justified dependency relation, **U** unverified historic Windows client edition/runtime. **Never conflate web ticket, SIPAE SA, client certificate and SRTP master key.**

## I. Two different service operations and trust

| ID | Direct section | Protocol/client-server boundary |
|---|---|---|
| WEB-01 | §2.1 | All messages transmitted via **HTTPS with SOAP** (SOAP 1.2/WSDL/XSD). This is **not** SIP REGISTER, SIP SERVICE, SIPAE `gssapi-data` or TURN Allocate even if related identities are the same user. |
| WEB-02 | §§3.1.1–3.1.4.1 | **Certificate Provisioning Service** has `GetAndPublishCert`: server issues **X.509 v3** based on **PKCS#10 CSR**, stores issued cert so later client-certificate proofs can be verified. Server maintains CA issuer and trusted CA collection; **no protocol-specific timers** in §3.1.2. |
| WEB-03 | §§3.1.4.1,3.1.4.1.2.2 | Cert issuance error **MUST** be returned as `GetAndPublishCertResponse/ErrorInfo`; **SOAP fault SHOULD NOT** be used for this operation. Web Ticket `IssueToken` faults are a separate method/path. |
| WEB-04 | §3.1.4.1.2.3 | Request `wst:RequestSecurityToken` MUST be child of `GetAndPublishCert`, `RequestType = .../Issue`, `TokenType = ...#X509v3`; **`wsse:BinarySecurityToken` contains Base64 PKCS#10 CSR**, EncodingType base64binary, **ValueType #PKCS10**. Base64 does not make key/private material confidential. |
| WEB-05 | §3.1.4.1.3.1 | Certificate request envelope **MUST have exactly one RST**, required `DeviceId` string and required `Entity` URI; server SHOULD process requested validity period when supported by MS-WCCE. **Identity attributes are not optional convenience UI fields.** |
| WEB-06 | §3.1.4.1.5.2 | `Entity` MUST be **authenticated user's SIP URI without leading `sip:`**, not any claimed or unknown user's arbitrary URI. The device identifier is separately correlated; user-to-device mapping is authorization-critical. |
| WEB-07 | §3.1.4.1.2.4 | Successful response requires `DispositionMessage = Issued`, `lang=en-US`, X509v3 TokenType and **Base64 X.509 cert**. Returned Subject **Common Name MUST equal Entity** in request, SubjectKeyIdentifier **SHOULD carry DeviceId**. The outgoing response copies request's `DeviceId` and `Entity`. |
| WEB-08 | §3.1.4.1.2.4 | Response includes original request BinarySecurityToken to correlate CSR/certificate; on **error RSTResponse MUST be absent**. Client cannot treat merely successful HTTP status as valid certificate without verifying typed body/cert/key binding. |
| WEB-09 | §§3.1.4.1.3.3,3.1.4.1.4.1 | Error enum includes **InvalidPublicKey**, **InvalidValidityPeriod**, **InvalidEKU**, **InvalidSipUri**, **InvalidCSR**, **DataStoreUnavailable**, **InvalidDeviceId**, **RequestMalformed**, **AccountDisabled**, **UserImproperlyProvisioned**, **InternalError**, plus NoError. No generic "retry all errors" rule follows from an XML enumeration. |
| WEB-10 | §3.2 | **Web Ticket Service** separate `IssueToken` operation with WS-Trust1.3 RST/RSTR, not a certificate from PKCS#10. Client **MUST authenticate** using one supported method: **Integrated Windows (Kerberos/NTLM)**, **OCS-signed certificate**, **Live ID SAML**, or **OAuth2** (with MS-OAUTH2EX profile). Supported method is topology/product-dependent, not a universal four-step login sequence. |

## II. Ticket message contract, audience, actor and authorization

| ID | Original section | Source-exact behavior |
|---|---|---|
| WEB-11 | §3.2.4.1 | `IssueToken` WSDL operation sends WS-Trust action `.../RST/Issue` and receives `.../RSTRC/IssueFinal`; HTTP SOAP and WS-Trust faults are expected, **not SIP error 401/407**. |
| WEB-12 | §3.2.4.1.3.3 | RST `Context` **MUST UUID**; `TokenType` **MUST SAML 1.1**, `RequestType` Issue, `KeyType` SymmetricKey. `AppliesTo/EndpointReference/Address` **MUST be target service HTTP URL** and server **MUST validate membership in own server farm**. A token for one service must not be automatically accepted by unrelated HTTP origin. |
| WEB-13 | §3.2.4.1.3.3 | RST `Entropy/BinarySecret` required Base64 cryptographically random bytes; key length from **WS-Policy**, **SHOULD >=128 bits**, entropy SHOULD match key length. Source describes cryptographic entropy within WS-Trust, not a SIPAE cnum/crand field or SRTP KDR. |
| WEB-14 | §3.2.4.1.3.3 | Optional `Claims` Dialect `urn:component:Microsoft.Rtc.WebAuthentication.2010:authclaims`; optional SIP URI claim **MUST match authenticated credential principal**. On mismatch server SHOULD return **wst:RequestFailed**; malformed required fields SHOULD yield **wst:InvalidRequest**. |
| WEB-15 | §3.2 | For Integrated Windows auth use **RFC4559 SPNEGO over HTTP**, NOT SIPAE NTLM/Kerberos header grammar. Certificate auth uses **WS-Security X.509 token**; Live ID uses **SAML**; OAuth2 requires trusted actor token/signing cert/audience with `00000004-0000-0ff1-ce00-000000000000/<host_fqdn>@<realm>` and identity claims matching **exactly one** user in UAS database. Microsoft 2025 source includes modern/federated variants but does **not prove** all 2015 Windows build support. |
| WEB-16 | §3.2 | Once Web ticket issued, web service client **MUST attach ticket as SAML token to SOAP request** to participating web service. Tickets may be **signed security token** or **proof-of-possession token**; proof tickets can be **bound to same web server** at which they were requested. |
| WEB-17 | §3.2 | For participating non-SOAP web applications, client **MUST send HTTP `X-MS-WebTicket: opaque=<base64 SAML>`** (optional extensions allowed); source **requires signed ticket**, not proof-of-possession-only token on arbitrary HTTP origin. Not an Authorization: NTLM or SIPAE header. |
| WEB-18 | §3.2 | Web ticket validation errors: **28032 invalid**, **28033 expired**, **28034 proof ticket valid only for same web server** (SOAP fault for SOAP app); non-web **HTTP X-Ms-diagnostics** error for invalid/expired. These errors are **not** MS-OCER SIP diagnostics or SIP REGISTER expiry values. |
| WEB-19 | §3.2.4.1 | `IssueToken` Web service authentication failure distinguishes **HTTP X-Ms-diagnostics** (for Integrated Windows/UAS-signed cert failures) from **SOAP OCSDiagnosticsFaultType** (Live ID, certain cert failures or failed request processing). **403/28000** means authenticated but not SIP enabled; **500/28001** means unexpected integrated-auth/internal failure. |
| WEB-20 | §§3.1.5,3.2.2 | Neither Certificate Provisioning Server nor Web Ticket Service defines its own timer algorithm in inspected sections. **Certificate expiry, token lifetime, SIP SA expiry and HTTP 401 handshakes remain independent**; do not invent fixed automatic web-ticket refresh interval. |

## III. Cross-domain identity and readiness gates (S, conditional)

~~~mermaid
flowchart TD
 A[Authenticated user credentials and HTTPS server TLS identity] --> B{Web authentication capability}
 B -->|Integrated Windows| C[HTTP Negotiate Kerberos or NTLM per RFC4559]
 B -->|OCS certificate| D[Certificate previously issued/installed and client proves possession]
 B -->|Federated LiveID| E[SAML/WS-Security]
 B -->|OAuth2| F[Actor token trust and exact audience/user claim]
 C --> G[WS-Trust IssueToken RST with audience and entropy]
 D --> G
 E --> G
 F --> G
 G --> H{Token and authorized user valid?}
 H -->|No| I[HTTP/SOAP diagnostic fault, not SIP logout]
 H -->|Yes| J[Signed or proof SAML Web Ticket]
 J --> K{Target application}
 K -->|SOAP web app| L[WS-Security SAML token]
 K -->|Non-SOAP web app| M[X-MS-WebTicket opaque base64 signed SAML]
 A --> N[Separate PKCS10 GetAndPublishCert/DeviceId/Entity request when provisioned]
 N --> O[Issued X509 cert or typed ErrorInfo]
 O --> D
~~~

The certificate provisioning graph is **not a mandatory precursor** to all Web Ticket service methods. In particular Integrated Windows and OAuth paths can be independently valid; client's **SIPAE TLSD-SK signing SA** and **HTTPS Web Ticket** are different credentials/trust services even when they refer to same user principal. Proof Web Ticket bound to one server cannot be forwarded to another host as if generic bearer.

## IV. Explicit original-edition dependency blockers

- **WEB-X01:** Full WS-Trust1.3, OASIS SAML Security Token Profile, SOAP WSS X.509 and Microsoft **MS-WSTEP/MS-WSPOL/MS-WCCE** namespace/algorithm/HTTP binding and revs have **not** been completely audited. Local extra PDFs exist, but some source dates and 2025 Windows client compatibility need checking. Certificate keypair generation/CSR private key retention/exact trust chains are outside the checked §3.1 clauses.
- **WEB-X02:** HTTP Web Ticket `X-Ms-diagnostics` vs Microsoft SIP `ms-diagnostics` **different transport/header casing/scope and ErrorIds**. Do not treat OAuth2 HTTP 401 and SIPAE 401 same authentication response or automatically force REGISTER reauth.
- **WEB-X03:** Anonymous conference `conference-key` Digest, A/V Edge `SERVICE media-relay-auth` credentials, and Web Ticket are **three separate principals/keys**. A valid X509 client certificate doesn't automatically grant conference Focus/ASMCU roster permission.
- **WEB-X04:** OAuth2 actor token trust and audience in MS-OCAUTHWS 2025 is documented, but **historic legacy Skype for Business Windows Full Client capability** must be evidenced by feature notes/client build, not assumed.
- **WEB-X05:** Certificate Subject CN and SIP Entity equality requirement from 2025 Microsoft text does not override separate RFC5280 SAN or transport peer FQDN rules. Need verify which certificate is for **client TLS-DSK**, which for **HTTPS server** and which for **WS-Security**, and how revocation/deprovisioning works.
- **WEB-X06:** Complete certificate error and SOAP fault table, Web Ticket cryptographic XML signature/proof key and exact metadata discovery authentication bindings remain not fully source-closed. Do not claim Web Ticket issuance or storage complete because HTTP 200 alone.
- **WEB-X07:** WS-Trust `AppliesTo` server-farm validation and proof-only host scope are **security trust boundaries** that must survive redirects and URL changes. No source evidence authorizes returning/proxying tokens to arbitrary host supplied in SDP or diagnostic text.

**Global Full Client normative research incomplete. No new protocol code, real server traffic or owner-only interoperability testing, main merge or release.**
