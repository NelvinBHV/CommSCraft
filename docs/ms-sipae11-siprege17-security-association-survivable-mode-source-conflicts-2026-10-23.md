# MS-SIPAE rev11 / MS-SIPREGE rev17.1 / MS-CONMGMT rev10 — authenticated REGISTER to survivable presence state

**Research date: 2026-10-23. INCOMPLETE; documentation only.** Pin **[MS-SIPAE] rev11.0, 2025-05-20**, **[MS-SIPREGE] rev17.1, 2025-08-19**, **[MS-CONMGMT] rev10.0, 2025-05-20**. Official original PDFs:
- https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-SIPAE/%5BMS-SIPAE%5D.pdf — archive SHA-256 `e7d143657a688111fdd900098727918e74dc59727e77cebd248562634580d347` **(hash check pending independent manifest reconciliation, do not treat guessed SHA as verified)**.
- https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-SIPREGE/%5BMS-SIPREGE%5D.pdf — archive edition in inventory and section footer independently verified, cryptographic hash requires recheck.
- https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-CONMGMT/%5BMS-CONMGMT%5D.pdf — original 2025 PDF. 

**Source assessment:** MS-SIPAE §§3.2.1–3.2.5.2,3.3.2–3.3.5; MS-SIPREGE §§3.1.1.1–3.1.1.6,3.2.1.5,3.2.2.4.1,3.3.1.4–3.3.2.4; MS-CONMGMT §§3.1.5,3.4.5–3.5.6. Original MS-SIPAE PDF pp19,20,24 and MS-SIPREGE pp75,76 were rendered and checked against text. Exact normative source refs: [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) §§10.2.2,10.2.4,10.2.7,22; [RFC2743](https://www.rfc-editor.org/rfc/rfc2743) §§2.2–2.3 GSS API; [RFC5246](https://www.rfc-editor.org/rfc/rfc5246) TLS1.2 §7; historical RFC2246/4346 and [RFC3548](https://www.rfc-editor.org/rfc/rfc3548) Base16/Base64; [MS-KILE], [MS-NLMP] as separate identity/authentication dependencies. **P** exact first-party clause; **R** RFC; **S** conditional synthesis; **U** unresolved; historical 2025 edition ≠ a proof of a Windows binary's runtime.

## Security association, challenge, signing and clock state

| ID | Source | Direct protocol-state finding |
|---|---|---|
| AUTH-01 | MS-SIPAE §3.2.1 | A client keeps **one or more security associations (SA)** per **challenging server**, indexed by **realm + authentication target**, with proxy vs UAS distinction from Proxy-Authenticate vs WWW-Authenticate. A user account is **not** a globally shared SA; realm/target and transport role matter. |
| AUTH-02 | §3.2.1 | SA state phases: **establishing → established → expired**, plus authentication context, nonce/opaque, client cnum, server snum highest and **256-message sliding replay window**. An unexpired SIP registrar binding does **not** guarantee every SA remains valid. |
| AUTH-03 | §3.2.5.1 | On **401** (WWW-Authenticate) or **407** (Proxy-Authenticate), client MUST inspect appropriate challenges, match existing SA; special **Digest** or **anonymous-conference** branches have separate processing, so the normal three-scheme flow is not exhaustive. |
| AUTH-04 | §3.2.5.1 | Server's **Date** and client's clock can affect Kerberos. Microsoft cites a default acceptable Kerberos skew of **5 minutes**, but **no maximum skew defined for NTLM/TLS-DSK**. Do not impose Kerberos value universally. |
| AUTH-05 | §3.2.5.1 | A challenge against **already established SA**: if original request was unsigned, **resend signed**, else if already signed **destroy the challenged SA and reestablish**. Never blindly loop by retransmitting same signed message forever. |
| AUTH-06 | §3.2.5.1 | NTLM uses base64 GSSAPI data under [MS-NLMP]; Kerberos obtains **sip/targetname service ticket** under [MS-KILE] and GSS_Init_sec_context; TLS-DSK runs embedded certificate-based **TLS handshake tokens over SIP authentication challenges**, derives distinct client/server signing keys, and is *not identical* to hop-by-hop SIP TLS transport. |
| AUTH-07 | §3.2.5.1 | TLS-DSK derives 128 PRF bytes from negotiated master secret, label **client EAP encryption**, client_hello.random||server_hello.random; client key from third 32-byte segment, server key from fourth 32-byte segment, **truncated to negotiated digest key length**. Algorithm selection inherits negotiated TLS cipher hash and must not be confused with SRTP HMAC-SHA1/SHA256 disputed media suite. |
| AUTH-08 | §3.2.4.1 | Established per-server SA signs outgoing SIP messages with per-SA incrementing **cnum**, fresh **32-bit crand** as 8 hexadecimal digits; MIC input canonicalization is **UTF-8 angled-bracket field sequence**, including request Call-ID, CSeq, method, From and conditional identity/To/Expires fields by version. Generic signing of raw SIP bytes is wrong. |
| AUTH-09 | §3.2.5.2 | Incoming authenticated server messages require matching SA, `rspauth` Base16 decode, GSS_VerifyMic (NTLM/Kerberos) or TLS-DSK HMAC with **server** key, then replay check. On missing SA, MIC failure, **snum outside 256-message window** or duplicate, message is **discarded**, without automatically triggering SIP BYE or a new REGISTER. |
| AUTH-10 | §3.2.5.2 | A valid 403 during handshake destroys SA and asks for other credentials; otherwise a valid authenticated server message establishes SA and destroys competing associations for the same realm/target. **Authenticated 403** and missing/integrity-invalid server message are not one failure. |
| AUTH-11 | §3.2.2 | Client SA renewal deadline: NTLM **8 h minus safety buffer**; Kerberos **min(8 h, service-ticket expiry) minus buffer**; TLS-DSK **min(8 h, certificate expiry) minus buffer**. Client MUST budget enough time and buffer **SHOULD ≥5 min**. Server §3.3.2 fixed **8 h** SA expiry. |
| AUTH-12 | §3.3.2 | Server's SA **idle** timer has its own priority: successful REGISTER Expires first, if not that then successful INVITE/UPDATE Session-Expires, otherwise **900 s SHOULD**. SA idle is not registration expiry, MS-CONMGMT 32 s unauthenticated proxy timer, or hop-by-hop keepalive. |

## REGISTER, provisioning, differentiated failure and survivable mode

| ID | Primary clause | Actor/conditional requirement |
|---|---|---|
| REG-01 | MS-SIPREGE §3.1.1.4.1 | Client MUST insert **Epid and SIP.INSTANCE** from MS-SIPRE; SHOULD supply User-Agent and discover Ms-Subnet, Ms-Device-Info. Source-specific subnet tied to actual TCP interface; do not fabricate MAC/interface identifiers. |
| REG-02 | §3.1.1.5.1 | On successful registration client SHOULD extract **GRUU** and MUST use expiry from 200 response; refresh time MUST be **strictly less than granted expiry**. This differs from fixed authentication-SA 8 h expiry. |
| REG-03 | §3.1.1.5.1 | Client SHOULD parse MS-CONMGMT **Ms-Keep-Alive** response (if present) and initialize per-TCP-connection keepalive, which is independent from REGISTER refresh and SA expiry. |
| REG-04 | §3.1.1.5.1 | On **REGISTER refresh**, Presence-State `Register-Action=added|fixed` means original binding was not just refreshed: MUST treat as registration failure, existing dialogs SHOULD be considered ended, SHOULD restart registration. A SIP 200 on unexpected registration replacement is not straightforward success. |
| REG-05 | §3.1.1.5.1 | Presence-State cluster fields `Primary-Cluster-Type=central|remote`, `Is-Connected-to-Primary=yes|no` update home/primary server state; source explicitly leaves local abstract representation unconstrained while external behavior must match. |
| REG-06 | §3.1.1.5.1 | `user-services-state=unavailable` → Presence-Service-Available SHOULD false; absence of field → SHOULD true. **No universal true rule for other arbitrary values** stated in this reviewed passage; do not infer availability from REGISTER 200 alone. |
| REG-07 | §3.1.1.5.1 | Initial registration 200: SHOULD start **self-presence SUBSCRIBE** (MS-PRES §3.3), **contact/group subscription** (MS-SIP §3.3), depending provisioning state. Success of either does **not imply** success of the other. |
| REG-08 | §3.1.1.5.1 | If Presence-Service-Available=false, client SHOULD enter **survivable mode** and **SHOULD NOT** start subscriptions **except self-subscription**. Contact roster empty because unavailable in this mode is not proof contact list was deleted. |
| REG-09 | §3.1.1.5.1 | On self-subscription failure, text says UAC **SHOULD send REGISTER Expiry=0 to remove binding**. Separately: if **not** survivable, SHOULD **retry self-subscription at randomized interval**; if survivable, SHOULD **defer retry until survivable mode ends**. The text gives **no precedence/order or cancellation guidance** for these simultaneous SHOULDs. This is **UNRESOLVED**, *not* a documented mutually exclusive decision tree. |
| REG-10 | §3.1.1.5.1 | The same passage describes leaving survivable mode after a future REGISTER response with Presence-Service-Available **false**, but earlier defines **false as unavailable / survivable**. Apparent **value-polarity contradiction**, verify official errata and product applicability before implementing recovery. |
| REG-11 | §§3.1.1.5.1–3.1.1.6 | Contact/group subscription failure means **contacts/groups unavailable**, not automatic self-subscription failure. REGISTER refresh **SHOULD** occur at expiry timer, **MUST use same TCP connection** as dialogue-creating original REGISTER (per wording); if TCP dies, a new REGISTER connection requires separate source restart rules, not an impossible reuse of closed socket. |
| REG-12 | §3.2.1.5 | If server `NOTIFY` carries **P-Preferred-Registrar**, UAC SHOULD attempt registration on specified registrar. This **registrar-directed handoff** differs from simple DNS failover and from ms-diagnostics-public error. |
| REG-13 | §3.2.2.4.1 | Server-created **deregister NOTIFY** is addressed to the specific deregistered endpoint, reusing local/remote tags and original REGISTER call-id/CSeq as described. Under **MS-CONMGMT §3.4.6** when dead endpoint removed after proxy keepalive expiry, registrar **MUST NOT** send NOTIFY: these are different eviction circumstances. |
| REG-14 | §§3.3.1.4,3.3.2.4.1 | In-band provisioning uses **SUBSCRIBE** (not REGISTER body), groups/parameters in XML, server validates schema and replies with supported parameter groups. Synchronous SIP 200 vs full policy content and status require MS-SIPREGE §2.2.2 and [MS-PRES] versions. |
| REG-15 | MS-CONMGMT §§3.4.5.1–3.4.6 | Negotiated `Ms-Keep-Alive` is **per connection**, UAC-initiated; if successful sends CRLF CRLF, default refresh SHOULD 2/3 negotiated server timeout; **any traffic resets**. On proxy expiry and concurrent registrar role, registrar SHOULD remove dead binding and **MUST NOT send NOTIFY** to disconnected endpoint. |
| REG-16 | MS-CONMGMT §3.5.5 | When authenticating new connection for **same endpoint** server MUST close previous connection/SA state; do not collapse all registrations of same **user** into same endpoint or break MPOP multiple registered devices. |

## Two independent state machines and point of composition (S)

~~~mermaid
stateDiagram-v2
  [*] --> TransportDiscovered
  TransportDiscovered --> TransportSecured: TLS cert/identity accepted if TLS profile
  TransportSecured --> SAEstablishing: 401/407 challenge per realm/target
  SAEstablishing --> SAEstablished: Verified handshake/MIC
  SAEstablished --> SAExpired: Min of 8h/ticket/cert minus buffer
  SAExpired --> SAEstablishing: Reauthenticate permitted branch
  SAEstablishing --> AuthFailed: Invalid creds/403 or policy error
~~~

~~~mermaid
stateDiagram-v2
  [*] --> BindingAbsent
  BindingAbsent --> Registered: Authenticated REGISTER 200 + GRUU/Expires
  Registered --> Provisioning: Independent SUBSCRIBE self and contacts/groups
  Provisioning --> NormalService: Authorized subscriptions succeed
  Provisioning --> Survivable: Presence service unavailable
  Provisioning --> SelfSubscriptionFailed: Self SUBSCRIBE failure
  SelfSubscriptionFailed --> SourceAmbiguity: Unregister SHOULD vs retry SHOULD
  Survivable --> SourceAmbiguity: End-of-survivable false-value contradiction
  Registered --> BindingRefresh: Refresh on original TCP flow
  BindingRefresh --> Registered: Existing binding refreshed
  BindingRefresh --> BindingUnknown: Register-Action added/fixed unexpectedly
  BindingUnknown --> BindingAbsent: Restart/resolve per source
  Registered --> BindingAbsent: Explicit Expiry=0 or registrar binding deletion
~~~

**S diagrams** deliberately show unresolved branch `SourceAmbiguity`, rather than forcing a false deterministic algorithm. Real client may have parallel realm/target SAs and registration to servers/Edge; a single serialized chart is an explanatory partial order only.

## Distinct timers (only source values, no invented retry)

| Timer | Source | Scope |
|---|---|---|
| MS-SIPAE SA security expiry | §3.2.2; §3.3.2 | client <8h or ticket/cert; server 8h |
| MS-SIPREGE registration refresh | §3.1.1.5.1/.6 and RFC3261 §10.2.4 | shorter than REGISTER 200 granted expiry; same initial TCP |
| MS-CONMGMT CRLF refresh | §§3.4.5–.6 | per hop, typically 2/3 negotiated timeout |
| MS-CONMGMT proxy auth deadline | §3.5.2 | proxy **32s SHOULD**, not client REGISTER timeout |
| MS-CONMGMT proxy idle | §3.5.2 | proxy **15min32s SHOULD**, not client RTP state |
| MS-SIPAE server SA idle | §3.3.2 | REGISTER Expires, else SIP session Expires, else 900s |
| Presence SUBSCRIBE randomized retry | MS-SIPREGE §3.1.1.5.1 | randomized interval **no numeric period in source passage**, unless survivable postpones; precedence unresolved |
| Media ICE2 consent / TURN relay | MS-ICE2 §3.1.6.5 / MS-TURN | separate 5s/30s, allocation/retransmit, no SIP SA conclusion |

## Documented open normative questions blocking code

- **CRIT-A:** Confirm MS-SIPREGE rev17.1 **same-paragraph self-SUBSCRIBE SHOULD unregistration and SHOULD retry** precedence. The original PDF §3.1.1.5.1 and product notes <151>–<153> do not give a general, unambiguous Windows Full Client resolution.
- **CRIT-B:** Resolve explicit **Presence-Service-Available=false used both as entry and supposed end of survivable mode** in MS-SIPREGE §3.1.1.5.1. Do not rewrite to true without official erratum.
- **CRIT-C:** Differentiate **transport TLS identity validation** (MS-CONMGMT) and **TLS-DSK certificate handshake embedded in SIP challenge** (MS-SIPAE), and **SIPAE HMAC/GSS MIC** vs **SDP/SRTP media HMAC dispute**.
- **CRIT-D:** Full signature input field order/empty fields for versions 3 and 4, server replay and authentication response challenges, digest/anonymous branch, server ticket policies and certificate provisioning [MS-OCAUTHWS], details of [MS-KILE]/[MS-NLMP] remain only partially reviewed.
- **CRIT-E:** Discovery candidates, MPOP GRUU, registrar route changes, reconnect/backoff and subscriptions must be proven for target Windows client edition and server topology. The original 2025 Open Specifications do **not** define a universal runtime ordering across all deployment variants.

**Overall normative analysis NOT COMPLETE. No protocol implementation, connection to any server, owner-only tests, merge or release.**
