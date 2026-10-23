# RFC2743/4120 + Windows MS-NLMP 37 / MS-KILE 49: nested NTLM/Kerberos packets, SPN trust and original edition guard for SIPAE 11

**Original archive and RFC audit 2026-10-23.** **[MS-NLMP] rev37.0 v20260330, 2026-03-30**, source SHA256 `00fe14a058134aabf6a6db1bcc1c47b8d45f0656ef7de610c03712906a945a27`, https://winprotocoldocs-bhdugrdyduf5h2e4.b02.azurefd.net/MS-NLMP/%5BMS-NLMP%5D.pdf ; **[MS-KILE] rev49.0 v20260928, 2026-09-28**, SHA256 `8d172e6fb7b760812fba2e41b1bb591dc80fcfce441419153cc93622456e7a01`, https://winprotocoldocs-bhdugrdyduf5h2e4.b02.azurefd.net/MS-KILE/%5BMS-KILE%5D.pdf ; **[MS-SIPAE] rev11.0 v20250520** first-party source https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-SIPAE/%5BMS-SIPAE%5D.pdf . Original [RFC2743](https://www.rfc-editor.org/rfc/rfc2743.html) January 2000 GSS-API §1.1/§2; [RFC4120](https://www.rfc-editor.org/rfc/rfc4120.html) July 2005 §§3.1–3.3,5.4–5.5 Kerberos 5; [RFC3962](https://www.rfc-editor.org/rfc/rfc3962) February 2005 AES Kerberos; [RFC8009](https://www.rfc-editor.org/rfc/rfc8009) October 2016 Kerberos AES-SHA2; [RFC4757](https://www.rfc-editor.org/rfc/rfc4757) December 2006 RC4-HMAC historical; [RFC4559](https://www.rfc-editor.org/rfc/rfc4559.html) June 2006 HTTP SPNEGO, **Informational**; [RFC4178](https://www.rfc-editor.org/rfc/rfc4178) SPNEGO. Original Windows specs read §§2.2,2.2.1.1–3,3.1.5.1.1–2,3.2.5 (NLMP), §§1.3.1–1.3.2,3.1.5.2–3,3.1.5.10–11,3.2.4.1–2 (KILE); PDFs NLMP pp18–25 and KILE pp30,32,35–36 rendered and checked. **Critical source edition warning:** MS-SIPAE 2025 references a *protocol dependency* by ID, but the local KILE/NLMP PDFs are **newer 2026 revisions** and cannot be silently used as an exact historical Windows Full Client conformance target. Microsoft's official [MS-NLMP revision page](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-nlmp/b38c36ed-2804-4868-a9ff-8dd3182128e4) confirms rev36.0 from **2024-04-23** available for historical comparison; [MS-KILE revision page](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-kile/) confirms rev45.0 **2025-08-11**, rev44.0 **2024-06-10** and rev31.0 **2015-10-16**. Original PDFs of those older revisions not yet downloaded/section-diffed in this tranche.

**Notation:** P=direct first-party Microsoft rule with explicit 2026 edition; R=normative RFC; S=conditional cross-document reasoning; U=unknown historic Windows client capability. No tests, code or live identity systems queried.

## I. Nested protocol packet layers; not a single shared header

| ID | Source | Wire/state requirement |
|---|---|---|
| ID-01 | MS-SIPAE 2025 §§2.2.1–3,3.2.5.1 | SIP Authorization/Proxy-Authorization `gssapi-data` is **Base64 text** containing a GSS/NTLM/Kerberos handshake token; it is **not an NTLMSSP raw SIP header** and SIP media does not carry KDC tickets directly except via encapsulated token under the authentication mechanism. |
| ID-02 | MS-NLMP 2026 §2.2 | Raw NTLMSSP message begins **8-byte ASCII `NTLMSSP\0`**, followed by **4-byte little-endian MessageType** `0x00000001` Negotiate, `0x00000002` Challenge, `0x00000003` Authenticate; then type-specific header and variable payload referenced by byte offsets. These fields are parsed **inside mechanism token**, not alongside SIP Via/CSeq. |
| ID-03 | MS-NLMP §2.2.1.1 | `NEGOTIATE_MESSAGE` holds **4-byte NegotiateFlags** and **8-byte DomainNameFields / 8-byte WorkstationFields** security buffers with length16,maxlength16,offset32; flags select options. This is not a Microsoft SIP 'version' or SIP registration Supported tag. |
| ID-04 | MS-NLMP §§2.2.1.2–3 | `CHALLENGE_MESSAGE` contains server 8-byte challenge and TargetInfo; `AUTHENTICATE_MESSAGE` contains challenge responses plus optional negotiated fields. Correct offsets/endianness and Unicode/OEM choice are NTLM mechanism matters and must not be guessed from SIP header parser. |
| ID-05 | MS-NLMP §2.2 | Multibyte fields **little-endian**, Unicode strings **UTF-16LE without BOM** (when negotiated), whereas SIPAE MIC text uses **UTF-8** and SIP message-level angle-bracket canonicalization. Character encodings **must not be collapsed into one raw buffer interpretation**. |
| ID-06 | MS-NLMP §§3.1.5.1.2,3.2.5 | Client receives CHALLENGE and **MUST check selected features against client security policy** before generating Authenticate, else return error to upper application. NTLM authentication server validates response and optional per-handshake MIC/channel binding by negotiated policy. Protocol support is not automatic approval of weak settings. |
| ID-07 | MS-NLMP §3.2.5 / §5.1 | NTLM **AUTHENTICATE_MESSAGE MIC**, when indicated by TargetInfo AvFlags, authenticates negotiation/challenge/authenticate with session key (HMAC-MD5 in selected profile); **SIPAE per-message `response` and `rspauth` MIC** later protects a *different* sequence of SIP header fields. Validating only one does not prove the other. |
| ID-08 | MS-NLMP §3.2.5 | **Channel binding** MsvAvChannelBindings and MsvAvTargetName checking are conditional on application's provided binding and policy; do **not** claim SIPAE 2025 mandates a specific channel-binding token because its reviewed §3.2.5.1 requests Datagram, Identify, Integrity without an explicit exported CBT requirement. |
| ID-09 | MS-NLMP §5.1 | Source notes NTLM depends on older MD4/MD5/RC4 cryptographic primitives, lacks modern AES/SHA256 and **recommends not using NTLM for new applications**. Historical client compatibility and modern product security posture are **different questions**; no instruction to weaken security or disable Kerberos. |

## II. Kerberos service identity, tickets and scopes

| ID | First-party and RFC | Role-conditional rule |
|---|---|---|
| ID-10 | RFC4120 §§3.1–3.3 / MS-KILE §§1.3.2,3.2.4.1–2 | Separate **AS** exchange (user to KDC obtaining TGT), **TGS** exchange (TGT obtains service ticket for SPN) and **AP** exchange (ticket + authenticator to service). SIPAE GSS_Init_sec_context wraps/consumes service ticket AP, not an instruction to send raw KRB_AS_REQ to SIP proxy. |
| ID-11 | MS-SIPAE §3.2.5.1 / MS-KILE §3.1.5.11 | SIPAE Kerberos service identity uses challenge `targetname` with **`sip/`** prefix; KILE SPNs generally `serviceclass/hostname[:port][/servicename]`. Full name resolution, SPN registration and mutual authentication are security-sensitive: do **not** substitute untrusted DNS A IP or arbitrary client account SIP URI. |
| ID-12 | MS-KILE §3.2.4.2 | Windows client requesting a resource first gets **service ticket** after TGT and then passes **AP token** to upper application/GSS. KILE source explicitly says Kerberos KRB_SAFE and KRB_PRIV *direct* messages **not supported**, rather than SIP itself implementing arbitrary Kerberos message types. |
| ID-13 | MS-KILE §3.1.5.10 | Windows KILE supports **cross-realm referrals** (RFC4120 §§1.2,3.3.1 and historical Referrals-11). This does not give unqualified permission to accept service ticket from a different untrusted realm; exact authenticated trust chain and ticket target must be validated by Kerberos mechanism. |
| ID-14 | MS-KILE 2026 §3.1.5.2 | Edition 49.0 explicitly **MUST support** etypes AES256/128-CTS-HMAC-SHA1-96 (17/18) and AES128/256-CTS-HMAC-SHA2 per RFC8009 (19/20), and describes RC4-HMAC and DES as SHOULD in source with product exceptions. **These KDC capability rules are for a 2026 Windows spec edition, NOT evidence of exactly which etypes a 2015 Skype client negotiates.** |
| ID-15 | MS-SIPAE §3.2.2; MS-KILE §3.2.4.2 | SIPAE Kerberos signing SA MUST expire no later than `min(8h, service ticket expiry) - safety margin` (SHOULD >=5min). Kerberos ticket lifetime may differ from application SA and from client REGISTER expiry. Never use 8h as a fixed Kerberos ticket TTL. |
| ID-16 | RFC2743 §1.1.3 / MS-SIPAE §3.2.1 | GSS security context is **mechanism-specific**; SIPAE maintains application **SA** metadata `realm/target/snum/cnum`. The GSS context, KDC tickets, SIPAE SA and proxy TCP/TLS connection have related but distinct lifetimes. |

## III. HTTP Kerberos/NTLM differs from SIP Kerberos/NTLM

| ID | Source | Strict actor/transport separation |
|---|---|---|
| ID-17 | RFC4559 §§4–5 / MS-OCAUTHWS §3.2 | **HTTP `WWW-Authenticate: Negotiate`** is SPNEGO carrying Kerberos/NTLM GSS tokens over HTTPS to the **Web Ticket Service**, not Microsoft SIPAE `WWW-Authenticate: Kerberos|NTLM` with `realm`/`targetname`/`gssapi-data`. |
| ID-18 | MS-OCAUTHWS §§3.1,3.2 | Even with successful Windows Integrated Auth, Web Ticket `IssueToken` applies **AppliesTo target, client entropy, identity claims and HTTPS server-farm policy** before issuing a SAML token. HTTP Negotiate success ≠ SIP REGISTER GRUU nor WEB ticket audience permission. |
| ID-19 | MS-SIPAE §3.2.5.5 / MS-CONFBAS | **Anonymous conference Digest** may use MD5-sess or SHA256-sess with conference key; not a form of Kerberos ticket exchange, NLMP session key or HTTP Web Ticket. |

## IV. Dependency edition timeline and explicitly outstanding historical originals

| Source | Edition physically in 150-PDF archive | Official earlier edition index (not yet full section comparison) | Interpretation |
|---|---|---|---|
| MS-SIPAE | rev11.0 / 2025-05-20 | Prior client-era versions need separately pinning | Primary 2025 description for this audit |
| MS-NLMP | rev37.0 / **2026-03-30** | rev36.0 2024-04-23; rev35.0 2022-04-29; older editions listed on Microsoft Learn | **Later** mechanism document; prohibit retroactive runtime inference |
| MS-KILE | rev49.0 / **2026-09-28** | rev45.0 2025-08-11; rev44.0 2024-06-10; rev31.0 2015-10-16; official rev history verified | **Later** Kerberos KDC profile; full 2015 vs 2026 AES/RC4 policy diff OPEN |
| MS-OCAUTHWS | rev11.0 / 2025-05-20 | Historical web-service behavior and product notes not fully reconciled | Separate HTTP/WS-Trust trust model |

## V. Error and risk gates

- **ED-A:** NTLM client security policy can refuse server feature flags; not every server 401 or NTLM CHALLENGE must be accepted.
- **ED-B:** SIP authentication 401/407 and HTTP Negotiate 401 are *different* messages and SA ownership. One must not transfer `Opaque`/`cnum` between them.
- **ED-C:** Per-handshake NTLM MIC and per-message SIPAE MIC are cryptographically and semantically different. NTLM-era historical MD5 cannot be replaced by SHA256 unilaterally without protocol/profile change.
- **ED-D:** Microsoft 2026 KILE's AES-SHA2 etypes and Microsoft 2026 NLMP/CBT rules cannot establish client 2015 feature coverage without actual earlier docs/revision delta/target binaries.
- **ED-E:** KILE cross-realm referrals vs SIP proxy realm vs TLS FQDN require authenticated target policy and no DNS redirection trust shortcut.
- **ED-F:** Official Microsoft historical PDFs must be retrieved and side-by-side normative section diffed before marking actual **Windows Full Client** vs Windows domain authentication compatibility. Current archive is a **supporting dependency**, not a full historical proof.
- **ED-G:** MS-OCAUTHWS web ticket, TLS-DSK X509 client cert and SRTP media credentials distinct; the previously proven MS-SRTP rev14 SHA256 / SDP AES_CM_128_HMAC_SHA1_80 **remains BLOCKED**.

**Global research INCOMPLETE; no implementation, live server test/interoperability, merge or release.**
