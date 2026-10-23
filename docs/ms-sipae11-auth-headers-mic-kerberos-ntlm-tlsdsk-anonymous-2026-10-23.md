# MS-SIPAE rev11: complete extended SIP authentication header grammars, MIC scope, challenge branches and anonymous conference proof

**Audit 2026-10-23. Original [MS-SIPAE] rev11.0, v20250520 (May 20 2025)**, SHA256 `6f0337a593233162401f0265e5425ad38d839174d1f78b8031405c5c7de64836`, official https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-SIPAE/%5BMS-SIPAE%5D.pdf . Reviewed §§2.2.1–2.2.6,3.2.1–3.2.5.6,3.3.1–3.3.5.7 and §4 original examples *selectively*, PDF pp13–15,24–25,35,37–38 rendered. Related original [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) §§20,22,25 (June 2002); [RFC2743](https://www.rfc-editor.org/rfc/rfc2743) GSS API (January 2000), [RFC4120](https://www.rfc-editor.org/rfc/rfc4120) Kerberos V5 (July 2005), [RFC5246](https://www.rfc-editor.org/rfc/rfc5246) TLS1.2 (August 2008), [RFC2104](https://www.rfc-editor.org/rfc/rfc2104) HMAC (Feb 1997), FIPS180-4 SHA2; [MS-NLMP] rev37.0 v20260330 and [MS-KILE] rev49.0 v20260928 as **later dependency editions only** (not proof of 2025 client behavior). Original MS-CONMGMT rev10 TLS transport and MS-SIPREGE rev17.1 registration are separate. **P** = original Microsoft actor/section; **R** = RFC; **S** = conditioned cross-spec synthesis; **U** = unresolved deployment capability and version guard.

## A. Exact Microsoft-specific header wire grammar and ABNF

| ID | Direct source | Wire-level fact; not a generic SIP baseline |
|---|---|---|
| SIPAUTH-01 | §2.2.1 vs RFC3261 §25.1 | `WWW-Authenticate` / `Proxy-Authenticate` challenges add exact scheme tokens **`NTLM`**, **`Kerberos`**, **`TLS-DSK`** beside Digest; their `msspi-cln` includes `realm`, `opaque`, **`targetname`**, **`gssapi-data`**, **`version`**, **`sts-uri`**. Header fields are not an opaque interchangeable Digest string. |
| SIPAUTH-02 | §2.2.1 | `targetname` with Kerberos includes **`sip/` service descriptor**; NTLM and TLS-DSK target values are tokens in their different namespaces. Canonical SIP authentication target is not necessarily DNS A address or certificate SAN FQDN. |
| SIPAUTH-03 | §2.2.1 | Digest `algorithm` explicitly admits **MD5**, **MD5-sess** and **SHA256-sess** in 2025 Microsoft extension; generic RFC3261 Digest MD5 examples do not establish all real deployment policies. |
| SIPAUTH-04 | §2.2.2 | Microsoft adds `Proxy-Authentication-Info` response header and **`snum`, `srand`, `realm`, `targetname`, `opaque`, `version`** to server message-proof fields. `snum` decimal (optionally quoted), `srand` 8 hex digits. Verify the actor and SA before trusting server AOR. |
| SIPAUTH-05 | §2.2.2 vs RFC3261 §20 | RFC3261 baseline confines `Authentication-Info` to **200 OK**, forbidding ACK/CANCEL; Microsoft explicitly **allows Authentication-Info in any response and ACK/CANCEL** and **Proxy-Authentication-Info in any request or response**. A rigid baseline SIP parser would reject valid Microsoft messages. |
| SIPAUTH-06 | §2.2.3 | `Authorization` / `Proxy-Authorization` carry schemes plus `realm`, `opaque`, `version`, `targetname`, base64 `gssapi-data`, **`crand`** 8 lowercase hex, **`cnum`** decimal, and signed **`response`** parameter. Target/service token and signature are per-SA state. |
| SIPAUTH-07 | §2.2.3 vs RFC3261 §20 | Microsoft **allows Authorization/Proxy-Authorization in SIP responses and Proxy-Authorization in CANCEL**, despite baseline SIP restrictions. This is an intentional grammar/profile extension, not a harmless parser warning. |
| SIPAUTH-08 | §2.2.4–2.2.6 | Endpoint identification, `Referred-By` modifications and `p-session-on-behalf-of` are extensions that can affect identity **of whom a signed action is on behalf of**. Do not treat a user-entered From URI as proof of delegation. The delegated-request auth path needs dedicated §3.2.5.4/§3.3.5.7 analysis. |

## B. Challenge, context creation, typed crypto and replay

| ID | Primary clause | Authenticated client/server state |
|---|---|---|
| SIPAUTH-09 | §3.2.1 | Client SA keyed by **scheme + realm + targetname/authentication target**, with **establishing/established/expired** phases and context/opaque/version/sequence state. An identical SIP AoR may have distinct proxy/registrar authenticated targets; no global SA per account. |
| SIPAUTH-10 | §§3.2.5.1,3.2.5.5 | On SIP **401** vs **407**, client MUST inspect challenge headers and pick **Authorization** vs **Proxy-Authorization** accordingly. Digest must follow separate §3.2.5.5 branch; normal Kerberos/NTLM/TLS-DSK security-handshake path is **not universal**. |
| SIPAUTH-11 | §3.2.5.1 | Matching established SA challenged after **unsigned request** → **resend signed**; matching SA challenged after **already signed** → **destroy SA and renegotiate context**, not indefinitely repeat same MIC. |
| SIPAUTH-12 | §3.2.5.1 | NTLM challenges carrying `gssapi-data`: **base64 decode**, invoke GSS_Init_sec_context via [MS-NLMP], returning NTLM **AUTHENTICATE_MESSAGE**. NTLM requests **Datagram, Identify, Integrity**, and documented implementation can yield **empty NEGOTIATE_MESSAGE token** in datagram mode; empty `gssapi-data` is not automatically invalid syntax. |
| SIPAUTH-13 | §3.2.5.1 | Kerberos: request a ticket for `targetname` service principal, invoke GSS_Init_sec_context [MS-KILE], ask **Integrity + Identify**. TGS/service-ticket retrieval differs from SIP REGISTER packets and SIP server-authenticated final response. |
| SIPAUTH-14 | §3.2.5.1 | TLS-DSK: embed TLS client_hello and server_hello / certificate / certificate_request / finished exchanges as **base64 gssapi-data tokens inside SIP auth**; validate server cert, obtain client cert, verify finished. **This is distinct from transport TLS on TCP** and also distinct from MS-SRTP media encryption. |
| SIPAUTH-15 | §3.2.5.1 | For completed TLS-DSK use **PRF(master_secret, `client EAP encryption`, client_random || server_random)** up to 128 bytes: **third 32 bytes** give client signature key; **fourth 32 bytes** give server signature key, truncated to negotiated digest length. Not an SRTP SDES inline master key. |
| SIPAUTH-16 | §3.2.5.1 | Outgoing challenge response `qop=auth`, replayed `realm`, `targetname`, `opaque`; add base64 `gssapi-data` if generated token. TLS-DSK may intentionally omit token if none generated, whereas **empty NTLM token field may be present**. Protocol version parameter only if guarded by source's version compatibility rule. |
| SIPAUTH-17 | §3.2.4.1; §3.3.5.3 | Signed SIP request MIC covers **ordered UTF-8 angle-bracket-wrapped textual fields**, not raw SIP datagram. Fields: scheme, crand, cnum, realm, targetname, Call-ID, CSeq number, CSeq method, From URI/tag, conditional To URI, To tag, conditional P-Asserted/Preferred-Identity SIP/tel URI, Expires and response status code on a response. |
| SIPAUTH-18 | §3.3.5.3 | In MIC canonicalization **missing field yields `<>`** if its slot applies; fields conditionally excluded by **protocol version >=3** or response/request type produce **no slot** if condition false. Case and spelling taken as present even for case-insensitive headers. For signed To and P-Asserted identity, this is security critical, not optional whitespace normalization. |
| SIPAUTH-19 | §§3.2.4.1,3.2.5.2,3.3.5.3 | Outbound **cnum** monotonic per client SA and random **32-bit crand**; server **snum** sliding **256-message replay window**, duplicates or outside window discarded. Sequence numbers not SIP CSeq, RTP SN, TURN Session ID or Kerberos ticket serial. |
| SIPAUTH-20 | §3.2.5.2 | On incoming `Authentication-Info` client locates same scheme/realm/target SA, validates **rspauth** cryptographic MIC (GSS_VerifyMic for NTLM/Kerberos or TLS-DSK server HMAC) then anti-replay. Missing SA, MIC failure or replay ⇒ **discard**, not automatically SIP BYE/REGISTER teardown. |
| SIPAUTH-21 | §3.2.5.3 | For **verified server-signed** message and server protocol version >=3, `P-Asserted-Identity` SIP/tel URI determines authenticated AoR when present; older/no-PAI falls back to **From for requests** and **To for responses**. SIP user identity requires server MIC/trusted realm context, not raw unverified header. |
| SIPAUTH-22 | §§3.2.5.5–.6 | **Anonymous conference GRUU** with From user `@anonymous.invalid` may use Digest **MD5-sess or SHA256-sess** and a **conference-key** as password, per MS-CONFBAS. **This is deliberately not the normal NTLM/Kerberos/TLS-DSK account sign-in**, and cannot be used to grant normal signed-in roster access. |
| SIPAUTH-23 | §3.2.2; §3.3.2 | Established SA client expiry: **NTLM 8 h minus buffer**, Kerberos min(service-ticket expiry,8 h) minus buffer, TLS-DSK min(client-certificate expiry,8 h) minus buffer; buffer **SHOULD ≥5min**. Server SA expiry **8h**. Separate from RFC3261 REGISTER Expires and MS-CONMGMT connection keepalive. |
| SIPAUTH-24 | §3.3.5.3 | Server must locate an *established* SA using a combination of Via/Record-Route reference, opaque and endpoint identifier; failing this yields **401/407** for request. A syntactically correct Authorization header **alone does not establish** a valid SA. |
| SIPAUTH-25 | §3.2.5.1 / MS-KILE vs MS-NLMP | **Kerberos default 5min clock skew** is cited; **no shared prescribed maximum for NTLM/TLS-DSK** in this SIPAE clause. Never impose Kerberos skew as global SIP login timer. |

## C. Per-actor annotated sequence (S, not one mandatory on-wire trace)

~~~mermaid
sequenceDiagram
 participant C as Classic Windows client / SIP UAC
 participant P as Registrar or authentication SIP server
 participant A as GSS/SSPI auth mechanism
 C->>P: REGISTER or other SIP request without Authorization
 P-->>C: 401 WWW-Authenticate or 407 Proxy-Authenticate
 C->>C: Select realm/targetname/scheme and SA version
 alt Kerberos
  C->>A: TGS ticket for sip/service and GSS_Init_sec_context
  A-->>C: GSS token / established service context
 else NTLM
  C->>A: GSS datagram context, optional challenge token
  A-->>C: NTLM response and signing session key
 else TLS-DSK
  C->>A: Inner TLS handshake with client certificate
  A-->>C: Mutual TLS-DSK context and directional signature keys
 else Anonymous conference
  C->>A: Separate Digest MD5-sess or SHA256-sess with conference-key
 end
 C->>P: Authorization or Proxy-Authorization; qop=auth; token
 P-->>C: Response and cryptographically authenticated server identity (when available)
 C->>C: Mark SA established; apply cnum/snum+MIC+replay guard
 C->>P: Signed REGISTER refresh, SUBSCRIBE, INVITE or SERVICE per authorization scope
 Note over C,P: SIP cnum/snum state differs from binding, TLS transport, TURN and SRTP
~~~

**Important S caveat:** In challenged Kerberos/NTLM/TLS-DSK flows, intermediate 401/407 and GSS tokens can occupy more than this simplified set of arrows. The sequence is **conditional by server scheme support and client product release**. Never infer TLS-DSK supported on all Windows builds or that an anonymous conference Digest key logs user into home registrar.

## D. Blocking edition, algorithm and source gaps

1. **MS-SIPAE rev11 2025** references [MS-NLMP] and [MS-KILE] without pinning later contents. Local **MS-NLMP rev37 2026-03-30** and **MS-KILE rev49 2026-09-28** cannot prove cryptography or KDC policy on a target *classic* Skype for Business 2015 client. Older historical editions must be retrieved/compared before declaring the exact runtime path. Track in [Windows KILE/NLMP dependency edition study](ms-sipae11-nlmp37-kile49-trust-and-edition-matrix-2026-10-23.md).
2. **GSS vs SIPAE double-layer MIC:** NTLM v2's AUTHENTICATE handshake MIC and SIPAE per-SIP-message `response`/ `rspauth` are **different inputs and authentication stages**. Source lacks evidence a SIP server exposes raw NTLM SSP fields directly at SIP header layer instead of base64 token.
3. **RFC3261 vs SIPAE header placement**: profile deliberately relaxes use of Authorization in responses/CANCEL; generic SIP parser strict RFC-only conformance would reject Microsoft traffic; need exhaustive original §4 and client proxy handling.
4. **Kerberos** SPN must be based on server challenge targetname and authenticated DNS/domain context; do not auto-trust attacker-supplied proxy realm and SPN. Full KDC trust, certificate and proxy discovery policy belongs to source's external environment.
5. **TLS-DSK certificate possession** can depend on separate **GetAndPublishCert PKCS#10 issuance**, but the sources do not prove that every Client must first use that exact web service in every build/topology. Follow [MS-OCAUTHWS certificate/web ticket original audit](ms-ocauthws11-certprovision-webticket-identity-2026-10-23.md).
6. **Overlapping session state:** SA expiration, registration expiry, SIP connection / keepalive, client certificate expiry, web ticket expiry, media SRTP keys and ICE consent all have independent validity clocks. A passing SIPAE MIC does not prove downstream HTTP ticket freshness or RTP packet signature integrity.
7. Must audit **SIPAE protocol version 3/4 signed header field changes**, full To/identity canonicalization, server-originated proxy authentication assertions, cross-realm error recovery, anonymous conference auth and product appendix version support before whole-client conformance decision.

**Overall Full Client analysis INCOMPLETE. No protocol code, no network/server access, no assistant interop tests, no merge/release.**
