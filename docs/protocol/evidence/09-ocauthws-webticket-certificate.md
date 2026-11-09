# Evidence: MS-OCAUTHWS – web ticket service, certificate provisioning (TLS-DSK), auth broker

Source: [MS-OCAUTHWS] "OC Authentication Web Service Protocol", edition 20250520, rev. 11.0, 92 pp. Applies from Lync 2010 up to SfB LTSC 2024 (§7, p. 88).
Dependencies: SOAP 1.1/1.2, WS-Trust 1.3, WS-Security, WS-MetadataExchange, SAML 1.1, PKCS#10/#7, X.509, MS-WSTEP, MS-WCCE, RFC 4559 (HTTP Negotiate) – §1.2, §1.4–1.5, pp. 11–17.

## Purpose in the client

1. **Web ticket** (SAML 1.1 token) for web services: Autodiscover user resource (MS-OCDISCWS), group expansion (MS-DLX), certificate provisioning, other Lync web services.
2. **User certificate** for SIP sign-in via **TLS-DSK** (MS-SIPAE §3.1, §4.4).
3. Auth broker = server-side TLS implementation for clients **without** their own TLS – not needed for a Linux client (inference from §1.3.3, p. 16).

## Web ticket service

| ID | Requirement | Evidence |
|---|---|---|
| AW-01 | Finding the URL: from the MEX metadata of the web service (`wsp:Policy/sp:IssuedToken/sp:Issuer/wsa10:Address`) or from the 401 header `X-MS-WebTicketURL` | §1.3.1.1–1.3.1.2, pp. 14–16; §1.5, p. 17 |
| AW-02 | Client authentication at the ticket service: integrated Windows (Kerberos/NTLM via HTTP Negotiate RFC 4559), OCS-signed certificate (WS-Security X.509), Live ID (SAML), OAuth2 | §3.2, pp. 31–32 |
| AW-03 | `IssueToken` = WS-Trust RST: `@Context` UUID, TokenType `…saml-token-profile-1.1#SAMLV1.1`, RequestType `…/ws-trust/200512/Issue`, `AppliesTo/EndpointReference/Address` = target service URL, `Entropy/BinarySecret` ≥128 bits, KeyType SymmetricKey, optional Claims (dialect `urn:component:Microsoft.Rtc.WebAuthentication.2010:authclaims`, ClaimType uri = SIP URI) | §3.2.4.1.3.3, pp. 36–37 |
| AW-04 | RSTR: `saml:Assertion` (treat as opaque), `Lifetime/Expires`, `AppliesTo` (prefix comparison, which services it is valid for), `RequestedProofToken/ComputedKey` (e.g. PSHA1), server entropy | §3.2.4.1.3.4, pp. 37–38; example pp. 62–63 (1 h validity) |
| AW-05 | Use: for SOAP web services as SAML token in the WS-Security header; for non-SOAP web applications the HTTP header `X-MS-WebTicket: opaque=<base64 SAML>` | §3.2, p. 32 |
| AW-06 | Errors: `X-Ms-diagnostics: id;source;reason;fault` (e.g. 28000 user not SIP-enabled, 28032 invalid, 28033 expired, 28035 SIP URI does not match); SOAP fault `af:OCSDiagnosticsFaultType` | §2.2.4.1, pp. 20–21; §3.2–3.2.4.1, pp. 32–35 |

## Certificate provisioning (for TLS-DSK)

| ID | Requirement | Evidence |
|---|---|---|
| AW-10 | The endpoint URL comes from the TLS-DSK challenge (`sts-uri`, MS-SIPAE §3.3.5.1); example `https://server/CertProv/CertProvisioningService.svc`, action `http://schemas.microsoft.com/OCS/AuthWebServices/GetAndPublishCert` | MS-SIPAE p. 33; §4.1.1, p. 60 |
| AW-11 | Request `GetAndPublishCert DeviceId="{GUID}" Entity="<sip URI without sip:>"` with `wst:RequestSecurityToken`: TokenType X509v3, RequestType Issue, `BinarySecurityToken` = Base64 PKCS#10 CSR, ValueType `http://schemas.microsoft.com/OCS/AuthWebServices.xsd#PKCS10`; validity via ValidityPeriod/Units (MS-WCCE) | §3.1.4.1.2.3, pp. 26–27; §3.1.4.1.5, p. 30; example p. 60 |
| AW-12 | This request is authenticated with a web ticket (request a ticket for the CertProv URL; AppliesTo example §3.2.4.1.3.3) | §1.3.1, p. 13; §3.2.4.1.3.3, p. 37 |
| AW-13 | Response: `ResponseClass`, `DispositionMessage="Issued"`, X.509v3 in `RequestedSecurityToken`; subject CN = Entity; SubjectKeyIdentifier ≈ DeviceId; error codes NoError/InternalError/InvalidPublicKey/InvalidValidityPeriod/InvalidEKU/InvalidSipUri/InvalidCSR/DataStoreUnavailable/InvalidDeviceId/RequestMalformed/AccountDisabled/UserImproperlyProvisioned | §3.1.4.1.2.4, p. 27; §3.1.4.1.4.1, pp. 29–30 |
| AW-14 | DeviceId MUST be convertible to a GUID, unique per device and identical to (or deterministically derivable from) the device identifier used in other services (for SIP: +sip.instance – inference from the example `{161CCE75-E0C7-5F60-…}` = version-5 UUID) | §3.1.4.1.5.1, p. 30 |
| AW-15 | The client needs PKCS#7 and X.509 processing | §1.5, p. 17 |

## Auth broker (for completeness only)

| ID | Statement | Evidence |
|---|---|---|
| AW-20 | `Reach/sip.svc`; operations Create/Terminate Session, AcquireCredential(SIPInstance), NegotiateSecurityAssociation (+V2 for TLS 1.2) produce gssapi-data for TLS-DSK on the server side | §1.3.3, p. 16; §3.3, pp. 38–40 |

## Conflicts / open points
- K-AW-1: Example §4.2.1 (p. 62) uses RequestType `http://schemas.xmlsoap.org/ws/2005/02/trust/Issue`, §3.2.4.1.3.3 (p. 37) requires `…/ws-trust/200512/Issue`. Section 3 is normative.
- O-AW-1: The exact WS-Security header structure when sending the web ticket to SOAP services (signed vs. proof of possession with a PSHA1 key) is only described by reference to WS-Trust/WS-Security; the WSDL in §6 (pp. 74–87) contains the policies and has to be evaluated during implementation.

## Later finding (file 39)
- O-AW-1 **secondary evidence** (OB-08): bearer variant (Timestamp + Assertion/EncryptedData) or proof of possession (P_SHA1 key, HMAC-SHA1 XML signature over `#timestamp`). The WSDL policy in §6 remains authoritative.
- O-AW-1 **closed** (OB-39, normative): the web-ticket policy §6.3 (p. 81) requires a bearer SAML 1.1 token as a signed supporting token over HTTPS, with WS-Addressing and no timestamp requirement; MS-DLX names its binding `AddressBookWebTicketBearer`.

## Implementation (2026-11-09, certificate provisioning)
- **AW-10 … AW-15:** `ocauthws::certificate_request` builds GetAndPublishCert (DeviceId = the endpoint's `+sip.instance` GUID, AW-14; Entity = the SIP URI without `sip:`); `parse_certificate_answer` takes the certificate only with ResponseClass Success, DispositionMessage "Issued", the asked entity and well-formed DER.
- **Request:** the adapter `tlsdsk` makes an RSA 2048 key pair and a PKCS#10 request with CN = entity, key usage digital signature and key encipherment, and extended key usage client authentication, signed with SHA-256 by `ring`.
- **Check:** the issued certificate must carry the requested public key and be valid now.
- **Storage:** the CLI keeps certificate and key in `$XDG_DATA_HOME/commscraft/tlsdsk/<account>` (directory 0700, files 0600) and asks again a day before the certificate ends.
- **No web ticket (inference from the service metadata, `tested-in-lab`):** the service at the challenge's `sts-uri` has a Windows-authentication binding (`CertProvisioningServiceWin`: HTTP Negotiate/NTLM over HTTPS) next to the web-ticket bindings, so the client sends the request with its NTLM HTTP authentication.
- **K-AW-2 (found 2026-11-09):** the example request §4.1.1 (p. 60) carries a SOAP header with `To` and `Action` in the namespace `…/addressing/none` (`mustUnderstand="1"`).
  - With it the service answers 400 without a body; with WS-Addressing 1.0 headers it answers 500.
  - Without a header (the SOAPAction HTTP header names the operation) it issues the certificate (`tested-in-lab`).
  - The client therefore sends no SOAP header.
- **Status:** `tested-offline`, `tested-in-lab`.
