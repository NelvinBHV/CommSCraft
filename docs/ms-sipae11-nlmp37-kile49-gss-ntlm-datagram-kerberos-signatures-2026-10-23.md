# CommSCraft — SIPAE / Windows Kerberos / NTLM authentication boundary audit (draft)

**Research 2026-10-23 | NO PROTOCOL IMPLEMENTATION | Full Windows SfB client gate: OPEN.** Sources are the original Microsoft PDF files from the existing 150-document archive and the specified RFC Editor publications. A later Windows-authentication specification must **not** silently rewrite requirements of the 2025 SIP client specification or prove which algorithm a particular installed Windows client uses.

## Original source identity and section basis

| Source | Edition, date and SHA-256 (original PDF) | First-party URL | Clauses examined |
|---|---|---|---|
| **MS-SIPAE** Session Initiation Protocol Authentication Extensions | **11.0** / 2025-05-20 / `6f0337a593233162401f0265e5425ad38d839174d1f78b8031405c5c7de64836` | https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-SIPAE/%5BMS-SIPAE%5D.pdf | §§2.2.3, 3.2.1–3.2.6, 3.3.2, 3.3.5.1–3.3.5.3; original PDF pp19,21,27 rendered |
| **MS-NLMP** NT LAN Manager (NTLM) Authentication Protocol | **37.0** / 2026-03-30 / `00fe14a058134aabf6a6db1bcc1c47b8d45f0656ef7de610c03712906a945a27` | https://winprotocoldocs-bhdugrdyduf5h2e4.b02.azurefd.net/MS-NLMP/%5BMS-NLMP%5D.pdf | §§2.1, 2.2.1.1–2.2.1.3, 3.1.1.2, 3.1.5.1–3.1.5.2.1, 3.2.5.1–3.2.5.2.2; original PDF pp47–48 rendered |
| **MS-KILE** Kerberos Protocol Extensions | **49.0** / 2026-09-28 / `8d172e6fb7b760812fba2e41b1bb591dc80fcfce441419153cc93622456e7a01` | https://winprotocoldocs-bhdugrdyduf5h2e4.b02.azurefd.net/MS-KILE/%5BMS-KILE%5D.pdf | §§3.1.1.3,3.1.5.1,3.2.5.5,3.2.5.7–3.2.5.8, 3.3.5.7 and product notes §4; original PDF pp39,58 rendered |

**RFC lineage**: [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) (2002) §§8,17,22 for SIP challenges and transactions; [RFC2743](https://www.rfc-editor.org/rfc/rfc2743) (2000) §§2.2–2.3 for GSSAPI establishment and MIC; [RFC4120](https://www.rfc-editor.org/rfc/rfc4120) (2005) §§3.1,3.2,3.3,5 for Kerberos AS/AP/TGS exchanges; [RFC4757](https://www.rfc-editor.org/rfc/rfc4757) (2006) NTLM/GSS Kerberos RC4 considerations; [RFC2104](https://www.rfc-editor.org/rfc/rfc2104) (1997) HMAC; [RFC5246](https://www.rfc-editor.org/rfc/rfc5246) (2008) TLS1.2; [RFC3548](https://www.rfc-editor.org/rfc/rfc3548) (2003) historical Base16/Base64 and [RFC4648](https://www.rfc-editor.org/rfc/rfc4648) (2006) later mapping. Direct RFC link is not evidence that current later algorithms are automatically negotiated by the classic SfB client. The source **MS-SIPAE** explicitly references historical RFCs **2246/4346/5246** for TLS-DSK; **do not** infer TLS1.3 handshake compatibility without actual normative extension.

**Evidence labels:** P = primary exact section for its version and protocol role; R = RFC clause; S = cross-source reasoning that may require additional proof; U = unverified version, feature policy or error path.

## A. Wire-level boundary: SIP headers are the *carrier*, not the NTLM/Kerberos transport

| ID | Section and authority | Direct source fact (actor-qualified) / boundary |
|---|---|---|
| AUTHB-01 | MS-SIPAE §3.3.5.1 (P) | Authenticated SIP server acting as **UAS** SHOULD send `401` with **`WWW-Authenticate`**; as proxy SHOULD send `407` with **`Proxy-Authenticate`**. Normal challenge must include **realm**, **targetname**, **version** (3/4) and Date; may advertise NTLM, Kerberos and TLS-DSK alternatives. A client must not assume one method selected merely because three challenges are offered. |
| AUTHB-02 | MS-SIPAE §3.2.5.1 (P) | The receiving UAC MUST parse 401/407 authentication headers. For **NTLM and TLS-DSK**, SA target key is `targetname`; for Kerberos, the target key incorporates the **`sip/` service descriptor**. Realm+target identify SA; `401` and `407` are different protection/role domains. |
| AUTHB-03 | §3.2.5.1 (P) | If existing established SA challenged after an **unsigned** request, client resends the **signed** request. If it had been signed, destroys established SA and restarts handshake. A repeated 401 does not automatically prove bad username/password. |
| AUTHB-04 | §3.2.5.1 (P) | GSS output bytes or embedded TLS-DSK handshake bytes in **`gssapi-data`** are **Base64** in `Authorization` or `Proxy-Authorization` per challenge role; an empty generated NTLM Datagram token yields **`gssapi-data=""`** rather than the parameter being omitted. For a protocol generating *no token* the parameter can be omitted, per source. |
| AUTHB-05 | §3.2.5.1 (P) | SIPAE authorization challenge response copies **realm**, **targetname**, optional **opaque**, sets **`qop=auth`** and carries the appropriate GSS token when generated. Do not invent HTTP Negotiate semantics or a raw NTLM MIME body without the SIPAE Authorization envelope. |
| AUTHB-06 | MS-NLMP §2.1 (P) | NTLM has **no transport of its own** and is embedded in its *calling application protocol*. For CommSCraft SIPAE is the carrier, NOT SMB or RPC on the client-to-SIP-server application wire. |
| AUTHB-07 | MS-SIPAE §3.2.5.1 + MS-NLMP §3.1.5.2 (P) | SIPAE **NTLM requires Datagram, Identify, Integrity** GSS flags. Source explains Datagram NTLM **does not generate the NEGOTIATE_MESSAGE** in its current implementation; first wire-level NTLM message can be server **CHALLENGE_MESSAGE**, then client **AUTHENTICATE_MESSAGE**. Do not impose the three-message connection-oriented NTLM handshake on SIP. |
| AUTHB-08 | MS-NLMP §§3.1.5.1–3.1.5.2 (P) | Connection-oriented NLMP uses **NEGOTIATE → CHALLENGE → AUTHENTICATE**. Connectionless uses **CHALLENGE → AUTHENTICATE**; the latter still processes negotiated flags and derives exported session keys. This conditional branch depends on **caller-requested NTLM flags**, not TCP vs UDP physical SIP transport. |
| AUTHB-09 | MS-NLMP §3.1.5.2.1 (P, 2026 edition) | Datagram NTLMv2 client receiving CHALLENGE with TargetInfo SHOULD omit LM challenge response, SHOULD include timestamp (preferring server `MsvAvTimestamp`), and SHOULD supply NTLM message MIC if TargetInfo timestamp present. **The NTLM AUTHENTICATE MIC** is **not SIPAE Authorization `response` signature**; there are two different signed objects/layers. |
| AUTHB-10 | MS-NLMP §3.1.5.2.1 (P, 2026 edition) | The **NLMP** source recommends **MsvAvChannelBindings** hash when its calling application supplies channel binding data, and **MsvAvTargetName** when supplied. **MS-SIPAE** reviewed handshake does not independently prove the client supplies those values; do not automatically assert binding of SIP transport TLS to NTLM just because optional NLMP fields exist. |
| AUTHB-11 | MS-NLMP §3.1.5.2.1 (P, 2026 edition) | Policy with `ClientBlocked=TRUE` can return **STATUS_NOT_SUPPORTED** when target not in exceptions; treat as **Windows-authentication policy**, not a SIP 403/404 status. Its exact availability in the **historical 2015 Windows client operating system build remains U**. |

## B. Kerberos service selection and KDC: a separate trust flow

| ID | Primary clause | Verifiable behavior / caveat |
|---|---|---|
| KRB-01 | MS-SIPAE §3.2.5.1 (P) | For Kerberos SIPAE requests a **service ticket for SIP targetname** then establishes `GSS_Init_sec_context` with **Integrity + Identify**. A domain user logon ticket is not itself the final SIP authentication token. |
| KRB-02 | RFC4120 §§3.1/3.3 and MS-KILE §§3.1.1.3,3.2.5.5,3.2.5.7 (R/P) | Existing **TGT** may be cached. If needed, AS-REQ/AS-REP obtain it; KDC TGS-REQ/TGS-REP obtains a **target service ticket**. Do not assert a fresh AS exchange on every SIP 401 or every MESSAGE. Ticket cache, KDC routing and cross-realm referrals can change network order. |
| KRB-03 | MS-SIPAE §3.2.5.1 (P) | The source uses Kerberos target string **`sip/<targetname>`** for GSS context lookup and specifies default acceptable **5-minute** Kerberos clock skew. There is **no documented 5-minute maximum** for NTLM/TLS-DSK in this clause. |
| KRB-04 | MS-KILE §3.2.5.8 (P, 2026 edition) | The Kerberos application client can encounter **KRB_AP_ERR_SKEW**, with a KRB-ERROR carrying server time and source-prescribed possible **AP-REQ retry using error time**. Distinguish this deeper Kerberos exchange from the higher SIPAE instruction to prompt clock synchronization when the authentication server rejects credentials. |
| KRB-05 | MS-KILE §3.2.5.8 (P, 2026 edition) | Kerberos AP authorization-data support includes **channel binding flag** (conditional `ChannelBinding`) when there is an outer secured channel such as TLS. **Neither a Kerberos ticket alone nor SIPAE's protocol name proves channel binding occurred**. Testability and target Windows version are open research fields. |
| KRB-06 | MS-KILE §§3.2.5.7,3.3.5.7 (P, 2026 edition) | TGS exchange includes conditional PAC, policy, domain group and cross-realm trust, FAST and encryption options. These are **KDC/Windows platform roles**, not client SIP headers nor mandatory client-side implementations of whole Active Directory server. |
| KRB-07 | MS-KILE §§3.2.5.5/§4 (P, 2026 edition) | Conditional PA-PAC-OPTIONS, FAST, modern AES-SHA2 suites and RODC policies have OS/AD version notes. **Product-era gating is mandatory**: MS-KILE **Sept 2026** cannot retroactively require a 2015 client to send modern options merely by normative-reference reachability. |

## C. Signed SIP messages: canonical field order, MAC and replay

| ID | Source | Exact client/server security contract |
|---|---|---|
| SIG-01 | MS-SIPAE §3.2.4.1 (P) | Established SA with relevant target signs outgoing SIP message using a monotonically incremented **per-SA `cnum`** and fresh **32-bit `crand`** formatted as eight hexadecimal digits. This is **not** CSeq reuse nor NAT consent transaction ID. |
| SIG-02 | §3.2.4.1 (P) | SIPAE constructs a **UTF-8 canonical buffer of fields enclosed in angle brackets**. The order is **protocol, crand, cnum, realm, targetname, Call-ID, CSeq sequence, CSeq method, From URI, From tag, (To URI if v>=3), To tag, (P-Asserted-/P-Preferred-Identity SIP and TEL if v>=3), Expires, (response code if response)**. Case/grammar are preserved as on wire, even when header names are case-insensitive. Absent required selected fields become `<>`; conditionally omitted fields do not. **Hashing raw SIP packet bytes is NOT equivalent.** |
| SIG-03 | §3.2.4.1 (P) | **NTLM** GSS_GetMIC and **Kerberos** GSS_GetMIC sign canonical buffer. NTLM SSPI uses a **fixed sequence argument 100** for this SIPAE operation. This is **different from per-SA cnum** in the serialized canonical input. |
| SIG-04 | §3.2.4.1 (P) | **TLS-DSK** signs the same canonical input with **RFC2104 HMAC**, cipher-negotiated digest algorithm and **client authentication key** generated by embedded TLS handshake. No mandate to use SRTP media key or transport TLS channel key for this MAC. |
| SIG-05 | MS-SIPAE §3.2.5.2 (P) | Incoming authenticated response/request identifies SA by protocol+realm+targetname, reconstructs canonical buffer using **srand, snum** in place of crand,cnum, and signs using **server key** for TLS-DSK or **GSS_VerifyMic** for NTLM/Kerberos. The **P-Asserted-Identity** v>=3 rule differs from outbound preference that also permits P-Preferred-Identity. |
| SIG-06 | §3.2.5.2 (P) | Receiver Base16-decodes `rspauth`, validates MAC/GSS signature, and only then checks snum against **256-position replay window**. Missing SA, invalid MAC, duplicate or too-old snum ⇒ **discard**; do **not** interpret corrupted message as authenticated SIP 200. |
| SIG-07 | §3.2.5.2 (P) | After successful verification and replay acceptance, an authentic **403** during establishing SA ⇒ destroy SA and request alternate credentials; otherwise transition SA to **established** and destroy competing same-target SAs. **Valid 403**, invalid MAC, and repeated 401 all lead to different state changes. |
| SIG-08 | MS-SIPAE §3.2.2 (P) | Client SA expiry: **NTLM 8 hours minus safety buffer**, Kerberos **min(service-ticket remaining lifetime, 8 h) minus buffer**, TLS-DSK **min(client-cert remaining lifetime, 8 h) minus buffer**; buffer **SHOULD >=5 min**. This is not the same timer as SIP REGISTER Expires, SIP session expiry or MS-CONMGMT CRLF keepalive. |
| SIG-09 | MS-SIPAE §§3.2.5.3, 3.2.4.2 (P) | Authenticated **P-Asserted-Identity** can determine SIP AOR/TEL identity for source protocol version >=3; older profile falls back to From on request and To on response. **Identity trust requires verified server signature**; plain forwarded `P-Asserted-Identity` alone is not signed proof. |

## D. Diagram: two-layer NTLM/Kerberos authentication and signed SIP registration

~~~mermaid
sequenceDiagram
 participant U as SIP UAC Windows Full Client
 participant P as SIP proxy or UAS
 participant K as Windows credential/KDC
 U->>P: SIP REGISTER without usable SA Authorization
 P-->>U: SIP 401 WWW-Authenticate or 407 Proxy-Authenticate (realm,targetname,version,Date)
 alt NTLM Datagram
  U->>K: GSS Init NTLM Datagram/Identify/Integrity (empty first token possible)
  U->>P: SIP Authorization with gssapi-data="" and qop=auth if initial token empty
  P-->>U: 401/407 with Base64 NTLM CHALLENGE_MESSAGE
  U->>K: GSS Init/NTLM CHALLENGE => AUTHENTICATE_MESSAGE
  U->>P: SIP Authorization with Base64 AUTHENTICATE_MESSAGE
 else Kerberos
  U->>K: Get cached TGT or AS-REQ AS-REP if required
  U->>K: TGS-REQ for SIP service ticket / TGS-REP
  U->>K: GSS_Init_sec_context (Integrity,Identify)
  U->>P: SIP Authorization with Kerberos GSS token
 else TLS-DSK
  Note over U,P: Embedded TLS handshake carried in repeated SIP challenge/response tokens
  U->>P: TLS-DSK client_hello token in SIP Authorization
  P-->>U: TLS-DSK server_hello and certificate token
  U->>U: Validate server certificate and provision client certificate if needed
  U->>P: TLS-DSK client certificate / key exchange / finished handshake tokens
 end
 P-->>U: Authenticated SIP response with Authentication-Info rspauth and snum
 U->>U: Verify signature, reject replay, establish SA
 U->>P: Later signed SIP messages with canonical MIC, cnum and crand
 Note over U,P: Exact numbers of SIP 401/407 messages vary with auth mechanism and cached state
~~~

**Diagram S**: It presents classes of exchanges, **not fixed challenge count**. It must not be mistaken for an end-to-end secured media path, full provisioning sequence, or proof SIP TLS certificate and TLS-DSK certificate are the same.

## E. Security and edition risk ledger (unresolved)

- **U-01: OS-version mismatch** — MS-SIPAE rev11 May 2025, MS-NLMP rev37 Mar 2026, MS-KILE rev49 Sep 2026. All are originals, but formal 2015/2016 classic client and Windows operating-system applicability, NTLM policy/algorithm, Kerberos FAST/AES-SHA2 options remain unverified.
- **U-02: NTLM policy** — 2026 NLMP adds blocking exceptions/channel-binding optional fields; the SIPAE caller's provision of correct `ClientChannelBindingsUnhashed`, `ClientSuppliedTargetName` and `UnverifiedTargetName` must be sourced. Do not claim CBT is always present or absent.
- **U-03: GSS channel binding** — TLS transport server identity, NTLM AV channel bindings, Kerberos AP client option and TLS-DSK embedded certificate exchange are **different trust proofs**. Need explicit caller-provided binding and Windows OS evidence before calling a method channel-bound.
- **U-04: Kerberos service target** — Need exact SPN selection under pool redirection, Edge/proxy targetname, DNS aliases, cross-forest trust, pre-existing ticket and referrals. SIPAE `sip/` prefix is proven; exact server-issued target string is deployment data.
- **U-05: MTU / base64 / fragmented GSS tokens** — No invented 4KB max or fixed number of 401/407 iterations; SIP proxy header and UDP/TCP application limits must be verified by original sources.
- **U-06: Replay** — Per-SA received snum window 256 vs SSPI fixed 100 sequencing and cnum/cRAND require section-accurate distinction; whether wrapping causes new SA is not derived from inspected excerpt.
- **U-07: Certificates** — TLS-DSK client certificate acquisition [MS-OCAUTHWS] is a separate SOAP/HTTPS dependency; it is not a Kerberos service ticket nor a SIP transport TLS session.
- **U-08: Failure cleanup** — A SIPAE authenticated 403 may request new credentials, but it does not by itself prove automatic SIP REGISTER deregistration or presence cache reset. Cross-reference MS-SIPREGE and MS-PRES.
- **U-09: Hash algorithms** — SIPAE GSS MIC/HMAC selected by auth context is separate from MS-SRTP vs MS-SDPEXT **SHA1/SHA256 media suite conflict** still unresolved.

**Completion gate**: exact source-by-source historical applicability, official errata, all packet examples and conditional MIC fields, NTLM/Kerberos error code mapping, certificate broker provisioning and trust boundaries. Until then **NOT COMPLETE**; no protocol code, no real servers or own interop tests, no merge/main/release.
