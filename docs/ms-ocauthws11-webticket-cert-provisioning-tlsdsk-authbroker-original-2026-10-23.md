# CommSCraft — OC Authentication Web Services, certificate provisioning, WebTicket, TLS-DSK Authentication Broker

**Research-only, 2026-10-23. Original source: [MS-OCAUTHWS] OC Authentication Web Service Protocol rev 11.0, released 2025-05-20 (v20250520)**, archived PDF SHA-256 **`a57de2759abca016437d728020fcefae4f687512274f3f8eaa9f91aaff6a17a3`**, official original https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OCAUTHWS/%5BMS-OCAUTHWS%5D.pdf and revision history https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocauthws/63749828-ab8d-4b24-941c-b11985d854c5 . Reviewed §§1.3–1.6,2.1,2.2.1–2.2.4.1,3.1.1–3.1.4.1.2.3,3.2.4.1,3.3.4–3.3.4.7; original PDF pp25 and34 rendered and visually checked. **Not every 92-page element type was exhaustively reviewed**. RFC and third-party original inputs: [RFC2818](https://www.rfc-editor.org/rfc/rfc2818) HTTPS (2000), [RFC5280](https://www.rfc-editor.org/rfc/rfc5280) X509 PKI (2008), PKCS#10/PKCS#7 as specified in [MS-WCCE], WS-Trust 1.3 (non-RFC), [MS-WSTEP] (also missing from 150 archived originals), [MS-SIPAE] 11.0, [MS-OCDISCWS]. P = direct original clause, S = source-based inference, U = applicability unproven for target Windows binary. **HTTPS SOAP services are not SIP REGISTER transactions**.

## A. Three distinct HTTPS service functions (MS-OCAUTHWS §1.3, §§3.1–3.3)

| ID | Original section | Actor-specific source fact and boundary |
|---|---|---|
| WEB-01 | §§1.3,2.1 | MS-OCAUTHWS uses **SOAP over HTTPS**, with envelope/WS-Trust rules. This is separate from **SIP messages over TLS**, and also separate from **TLS-DSK TLS handshake bytes carried as SIP authentication tokens**. A single `tlsConnected` state cannot establish all three. |
| WEB-02 | §§1.3.1–1.3.3 | Three roles: **Web Ticket Service** issues security token for later web service authorization; **Certificate Provisioning Service** issues client X.509v3 certificates; **Authentication Broker Service** assists TLS-DSK SIP authentication. A WebTicket ≠ client X509 certificate ≠ established SIPAE Security Association. |
| WEB-03 | §3.1.4.1 | `GetAndPublishCert` is **CertProvisioningService** operation. Server issues an **X.509v3** certificate from a **PKCS#10 CSR** and publishes it to server storage for subsequent verification during authentication. The client's private key is not automatically disclosed to SIP server merely because it supplied CSR. |
| WEB-04 | §3.1.4.1 | On certificate issuance failure, server **MUST respond with `ErrorInfo` in `GetAndPublishCertResponse`**; **SOAP faults SHOULD NOT** be used for these operation errors. Do not treat absence of a SOAP Fault as proof successful certificate issuance. |
| WEB-05 | §3.1.4.1.2.3 | In `GetAndPublishCert`, `wst:RequestSecurityToken` child is **mandatory**; WS-Trust `RequestType` **Issue**, `TokenType` X509v3, `wsse:BinarySecurityToken` **Base64 PKCS#10 CSR**, `EncodingType` base64binary and `ValueType` **`#PKCS10`** per specified URIs. Prefixes are not semantically significant (XML namespace identity is). |
| WEB-06 | §3.1.4.1.2.3 | Server SHOULD support `ValidityPeriod`/`ValidityPeriodUnits` per MS-WCCE; optional extra elements/attributes not named by this clause **SHOULD be ignored**. Not all generic WS-Trust fields are allowed or required as Skype certificate-provisioning inputs. |
| WEB-07 | §§3.2.4.1,2.2 | `IssueToken` is **IWebTicketService** SOAP operation using WS-Trust RST/Issue → RSTRC/IssueFinal actions. WebTicket authorization request is not SIPAE `Authorization: Kerberos`; it may use Integrated Windows auth or certificate at HTTPS layer. |
| WEB-08 | §3.2.4.1 | On Integrated Windows or UAS-issued certificate credential failure, service can use HTTP **`X-Ms-diagnostics`**; Live ID/auth or processing failures may be **SOAP Faults**. **HTTP header X-Ms-diagnostics ≠ SIP header ms-diagnostics from MS-OCER**; parse/status/roles separately. |
| WEB-09 | §3.2.4.1 | HTTP 403 with **ErrorId 28000**: user authenticated but not SIP-enabled/absent from UAS user database. HTTP 500 **28001**: internal Integrated Windows authentication/authorization failure. A 403 here is not evidence an existing SIPAE SA failed MIC validation or that REGISTER was rejected. |
| WEB-10 | §3.2.4.1 | Certificate auth faults **28011** expired, **28012** invalid, **28013** not found, **28014** user not found, **28015** internal cert authorization error. Map separately to failed certificate provisioning, SIP transport TLS cert invalid, and SIP REGISTER 403. |
| WEB-11 | §3.2.4.1 | **ErrorId 28035** indicates requested WebTicket **claim SIP URI does not match** credentials. Correct response is an identity/claim diagnosis; source does not prescribe automatic lower-security fallback or force new Kerberos password prompt. |
| WEB-12 | §3.3.4 | Authentication Broker exposes **CreateAuthBrokerSession**, **AuthBrokerAcquireCredential**, **AuthBrokerNegotiateSecurityAssociation**, **TerminateAuthBrokerSession** plus **V2** variants for create/acquire/negotiate. The V2 forms explicitly **support TLS 1.2**; broker state and call ordering differ from SIPAE's direct GSS context. |
| WEB-13 | §3.3.4 | Broker `AcquireCredential` binds a particular **SIPInstance** to broker session and can be called **once per session**, whereas `NegotiateSecurityAssociation` can be called **multiple times** as successive SIP challenges arrive and delivers GSS response data; on completed handshake it returns client/server authentication keys. |
| WEB-14 | §§3.3.4.1–3.3.4.7 | WSDL interfaces define request/response actions and types, not proof the installed full Windows client always uses the broker instead of local TLS-DSK. Need target product configuration, authentication-policy and provisioning-service discovery evidence. |
| WEB-15 | MS-SIPAE §3.3.5.1 / MS-OCAUTHWS §1.3 | SIPAE server challenge for TLS-DSK **SHOULD** supply **`sts-uri`** pointing to certificate provisioning service. An `sts-uri` in an unauthenticated challenge is a **candidate endpoint**, not authorization to send credentials to arbitrary URL; its TLS certificate, DNS/service identity and enterprise policy must be validated independently. |
| WEB-16 | MS-OCAUTHWS §3.1.1, MS-SIPAE §3.2.5.1 | Server keeps issuer/CA and trusted chain information; TLS-DSK client must validate server certificate and present suitable client certificate. **HTTPS certificate issuing, certificate chain trust, TLS-DSK client cert possession, and SIPAE MIC validity are four gates**, not one. |
| WEB-17 | §2.2.1 and §3.1.4.1.2.3 | Web-service **XML namespace URI and element rules** define encoding. Generic OASIS WS-Trust SOAP handling alone is insufficient because Microsoft constrains CSR, token type and response/error envelope. |
| WEB-18 | §3.2.4.1 / MS-OCER §2.2 | `X-Ms-diagnostics` HTTP 403/500 WebTicket errors and `ms-diagnostics` SIP errors can contain same numerical family **280xx** but **different protocol carriers/roles**; never parse arbitrary HTTP header as authenticated SIP message or treat SOAP fault as SIP status. |

## B. Trust / state diagram (S synthesis, conditional on selecting TLS-DSK/WebTicket functionality)

~~~mermaid
flowchart TD
 A[Discovered service and signed-in Windows identity] --> B{Needed service}
 B -->|TLS-DSK cert| C[HTTPS CertProvisioning GetAndPublishCert PKCS10 CSR]
 C --> D{Signed certificate returned and published?}
 D -->|No| E[GetAndPublishCertResponse ErrorInfo]
 D -->|Yes| F[Usable client X509 credential with private key]
 F --> G[Optional TLS-DSK broker session and AcquireCredential]
 G --> H[Successive SIPAE 401/407 TLS-DSK handshake tokens]
 H --> I{Server certificate verified and handshake Finished checked?}
 I -->|No| J[Authentication failure without invented downgrade]
 I -->|Yes| K[Directional SIPAE client/server HMAC keys and SA]
 B -->|Web service ticket| L[HTTPS IWebTicketService IssueToken WS-Trust]
 L --> M{Token response or error?}
 M -->|Error| N[HTTP X-Ms-diagnostics or SOAP Fault]
 M -->|Success| O[WebTicket for authorized service]
 K --> P[Signed SIP REGISTER/requests, independent GRUU/Expires]
~~~

**Not a required all-calls ordering:** A client may reuse previously provisioned certificates or valid tickets, and may select other SIPAE authentication schemes. The original sources do **not** force GetAndPublishCert before every SIP REGISTER; account/certificate caching and administrative policy need separate evidence.

## C. Document and protocol versions / incomplete normative closure

- **OCA-OPEN-01:** All 92 pages of WSDL element/complex/simple type definitions, XML value bounds and errors have not been exhaustively audited; the typed certificate-issuance and AuthBroker V2 messages require field-by-field analysis with server-only vs client rules.
- **OCA-OPEN-02:** MS-WCCE / MS-WSTEP originals, certificate enrollment request format, CA issuance policy, key ownership and revocation are **transitive normative dependencies**; the existing 150-PDF archive omits some and later modern versions cannot automatically be inserted in a 2015 full client.
- **OCA-OPEN-03:** Mixed feature supports Integrated Windows authentication, certificates, WebTicket, OAuth/Live ID in different deployment generations. **Do not** claim every scheme is mandatory for a specified SfB Windows build or that every service is on path of SIP client login.
- **OCA-OPEN-04:** WebTicket 403 28000, SIPAE authenticated 403, registrar rejected SIP REGISTER, and HTTPS TLS identity failure are separate error domains, requiring distinct action/rollback and source.
- **OCA-OPEN-05:** Sensitive SIP credentials, certificate private key and WebTickets cross **separate network trust boundaries**; source §3.3 AuthBroker does not permit carrying private key or decoded tokens into untrusted logging.
- **OCA-OPEN-06:** Historic certificate profile RFC2818/RFC3280 references vs current SAN, client certificate chains and TLS versions remain version-dependent; don't weaken validation by only accepting obsolete RFC PKI rules.
- **OCA-OPEN-07:** No server/network tests, no creation of real certificates, tokens or challenge emulation were performed; this is source-only documentation research.

**Overall CommSCraft protocol research NOT COMPLETE. Implementation gate CLOSED; no new protocol code, no real servers/interop tests, no main merge or release.**
