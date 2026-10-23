# MS-CONMGMT rev10 — SIP proxy autodiscovery, TLS server identity, keepalive and connection failure states

**Primary-source audit: 2026-10-23. Research only; no implementation, no server tests.** Original Microsoft **[MS-CONMGMT] Connection Management Protocol**, revision **10.0**, version **v20250520**, published **May 20 2025**, SHA-256 `95fee40b4d20e4eece095acf090e3a65abdae711c03200e99cd28afc0f29ce38`; official https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-CONMGMT/%5BMS-CONMGMT%5D.pdf . Directly checked **§§2.1,2.2.1–2.2.2,3.1.1–3.1.7,3.2–3.3,3.4.1–3.4.7,3.5.1–3.5.7**; original PDF **pp14 and 18** rendered/visually reviewed. Microsoft Learn [exact keep-alive ABNF](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/97b11f6f-eccf-4848-a105-cc0c3a4a1763). IETF original editions: [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) (2002) SIP §§6,8,10,17,22, [RFC2131](https://www.rfc-editor.org/rfc/rfc2131) (1997) DHCP §4, [RFC2132](https://www.rfc-editor.org/rfc/rfc2132) (1997) DHCP option 60, [RFC3361](https://www.rfc-editor.org/rfc/rfc3361) (2002) option 120 §3, [RFC5246](https://www.rfc-editor.org/rfc/rfc5246) (2008) TLS1.2 §7, [RFC2459](https://www.rfc-editor.org/rfc/rfc2459) (1999 X.509 profile), [RFC5234](https://www.rfc-editor.org/rfc/rfc5234) (2008) ABNF. Source cross-dependencies **[MS-SIPCOMP] rev10** and **[MS-SIPAE] rev11**, **[MS-SIPREGE] rev17.1**. **P** = direct Microsoft clause, **R** = published IETF clause, **S** = cross-source inference, **U** = unverified Windows build/server setup. Times only express actor-specific MUST/SHOULD.

## I. Client discovery: parallel probes, ordered candidates and failure semantics

| ID | Exact source | Requirement and condition |
|---|---|---|
| CON-01 | §3.1.4 | Client **MUST acquire address-of-record** from user or offline input. The **host part** of that AoR (e.g. `sip:user@example.com`) **MUST be discovery domain**. Not universally the Active Directory DNS suffix, e-mail MX host or preselected SIP server FQDN. |
| CON-02 | §3.1.5 | Client MUST issue **four parallel DNS SRV queries**: `_sipinternaltls._tcp.<domain>` (TLS), `_sipinternal._tcp.<domain>` (TCP), `_sip._tls.<domain>` (TLS) and `_sip._tcp.<domain>` (TCP). Sort records returned by **DNS SRV priority**, not by alphabetic hostname. |
| CON-03 | §§3.1.2,3.1.5–3.1.6 | Client supporting optional DHCP SIP discovery **MUST maintain DHCP discovery timer**, recommended **5 seconds**. DHCP INFORM Option 120 SHOULD be sent including vendor class Option60 `MS-UC-Client`; broadcast to **255.255.255.255 MUST**, on DHCP-enabled interfaces SHOULD. Server addresses returned as raw IP **SHOULD ignore**; two returned results randomly ordered. |
| CON-04 | §3.1.5 | DHCP candidates have **no returned port**, so client MUST use **5061/TLS** and **5060/TCP**; separately, fallback **`sipinternal.<domain>:443` / `sip.<domain>:443` / `sipexternal.<domain>:443`** TLS candidates are SHOULD-appended when absent. **Never apply 5061 by default to a fallback hostname explicitly suffixed :443.** |
| CON-05 | §3.1.5 | Gather in this source-defined sequence: TLS internal SRV, DHCP TLS, TCP internal SRV, DHCP TCP, external TLS SRV, external TCP SRV. Result readiness can depend on which DNS queries completed first; do not assume all 4 queries MUST block before attempting. |
| CON-06 | §3.1.5 | For internal TLS SRV and external TLS SRV records, server domain/subdomain part **SHOULD be matched to AoR domain**, ignoring unrelated hostnames (e.g. `fakecontoso.com` instead of `contoso.com`). **TLS MUST** connect to TLS-service records, **TCP MUST** to TCP-service records. |
| CON-07 | §3.1.5 | Client **SHOULD connect sequentially** to ordered candidates, making a fresh DNS A lookup per candidate and trying returned A addresses in order. DNS-A failure or exhausted TCP connect IPs ⇒ **SHOULD next candidate**. For **all other failures, or no candidates left, MUST fail discovery**. In particular, a failed TLS peer validation cannot automatically justify a silent downgrade to plaintext next entry. |
| CON-08 | §3.1.5 vs RFC3263 | Microsoft's bespoke internal SRV names, DHCP order and fallback `:443` are **not equivalent** to bare RFC3263 generic SIP location algorithm. RFC3263 cannot replace source-specific discovery order. |
| CON-09 | §§3.1.2,3.5.2 | Optional DHCP timer (5s) and **proxy authentication connection timer (32s SHOULD)** are separate, the latter is server-side and has different cancel/restart events. |

## II. TLS identity, certificates and RTP key trust

| ID | Source | Scope |
|---|---|---|
| TLS-01 | §3.2 + §3.3 | Client MUST use **RFC5246 §7** TLS key exchange and certificate-based validation when connecting via TLS. Outbound proxy TLS certificate **MUST include outbound proxy FQDN as subject name** (source references older **RFC2459 §4.1.2.6**). This is a required **identity check in TLS profile**, not only whether the CA root is installed. |
| TLS-02 | §3.1.5 | A source-discovered SRV name using TLS is **bound to TLS**, and TCP records bound to TCP. Source says all other failures after a candidate connection attempt **MUST fail discovery**, rather than treat invalid certificate as a DNS-A miss. |
| TLS-03 | §2.9.1.5 [MS-OCSPROT] | Overview recommends transferring SDP **inline SRTP keys under protected SIP TLS channel**; **MS-SDPEXT by itself does not encrypt media key parameters**. This is a cross-protocol security boundary, NOT proof TLS is mandatory for every 2015 topology merely because one example says optional. |
| TLS-04 | §3.2 + [RFC2459]/[RFC5246] | Target client’s SNI usage, SAN/CN precedence, certificate chain/revocation and modern TLS1.3 negotiation are **not fully resolved by this 2025 Microsoft text**; an RFC2459-era subject-name sentence cannot justify disabling modern server-name or certificate trust validation. Need deployment/version-source audit. |

## III. Negotiated hop-by-hop keepalive (not generic RFC5626 outbound keepalive)

| ID | Microsoft source | Role-specific state rule |
|---|---|---|
| KA-01 | §2.2.1 + §3.4.5.1 | `Ms-Keep-Alive` header has mandatory role `UAC` or `UAS`, capability e.g. `hop-hop=yes`, and optional `timeout=<seconds>`. Only **`hop-hop`** is supported; `end-end` and `tcp` reserved and MUST NOT be negotiated by proxy. |
| KA-02 | §§3.4,3.4.4–3.4.5.1 | Keepalive is **per individual TCP connection**, **hop-by-hop only**, never forwarded through multiple proxies. **Client MUST initiate** negotiation, proxy MUST NOT initiate, after transport/**optional MS-SIPCOMP** setup. It can ride on any normal initial SIP request including REGISTER. |
| KA-03 | §3.4.5.1 | Client initial request header sets role **UAC**, `hop-hop=yes`; does not propose unsupported reserved end-end/tcp; timer not inferred from header omission. |
| KA-04 | §3.4.5.2 | Proxy receiving **multiple client request** Ms-Keep-Alive headers **MUST process first and ignore remainder**, even though ABNF §2.2.1 says header MUST NOT appear more than once (sender vs robust receiver distinction). |
| KA-05 | §3.4.5.2 | A failed SIP request **>=400** that carried keepalive negotiation implies **keepalive negotiation fails and client MUST NOT send** periodic keepalive message on that connection. Proxy accepting sends a success response including **UAS** role, `hop-hop=yes` and **timeout seconds**. |
| KA-06 | §3.4.5.3 | Client receiving **two or more server response** Ms-Keep-Alive headers **MUST ignore all**; missing header, failed response or empty mechanism intersection ⇒ negotiation fails; **MUST NOT transmit periodic keepalives**. This is asymmetrical with server accepting first duplicate *request*. |
| KA-07 | §§3.4.2,3.4.5.3 | Proxy **MUST** maintain expiry timer, resets on **any traffic** on connection; expiry SHOULD negotiated timeout **plus grace >=1 SIP transaction timeout**. Client **MUST** maintain refresh timer; on received `timeout` client **SHOULD use 2/3 of timeout**, reset timer on outgoing traffic. Unit **seconds**, not milliseconds; no invented fixed global period. |
| KA-08 | §§2.2.2,3.4.5.4 | Keepalive on authenticated/negotiated connection consists of **exact CR LF CR LF** (two consecutive empty CRLFs). Client **MUST send** on expired refresh timer **only for enabled connection**. This is **not STUN consent**, RFC5626 CRLF mode inferred from advertisement, TURN pseudo-TLS, or a new SIP request. |
| KA-09 | §3.4.6 | On proxy expiry, if proxy is **also registrar**, it SHOULD remove endpoint/registration binding and SHOULD close stale connection. Registrar **MUST NOT send NOTIFY** when removing the dead endpoint (§3.4.6→MS-SIPREGE §3.2.2.4) because endpoint isn't expected to respond. If proxy is **not registrar**, action is not specified as required. |
| KA-10 | §§3.4.2–3.4.6 | A SIP proxy receiving application traffic may renew keepalive expiry without a separate CRLF; client likewise resets refresh on outbound traffic. **A passed keepalive proves no media path, SRTP MAC, REGISTER period or conference membership**, and policy may disable keepalive. |

## IV. First-hop SIP proxy connection management and duplicate security association

| ID | Clause | Exact actor/timer and failure recovery |
|---|---|---|
| CONN-01 | §3.5.2 | **Proxy** MUST maintain authentication **connection timer**, set at establishment, reset upon **provisional SIP response**, canceled only upon **successful SIP response**; **32 seconds SHOULD**. At expiry unauthenticated connection **MUST close and release state** (resource exhaustion/DoS protection). |
| CONN-02 | §3.5.2 | **Proxy** MUST maintain **idle timer**, **15 minutes 32 seconds SHOULD**, reset when any traffic sent/received; on expiry **SHOULD close/release**. Neither timer is client SIP transaction F nor keepalive negotiated timeout. |
| CONN-03 | §3.5.5 | Once proxy authenticates new **connection for same endpoint using MS-SIPAE**, it **MUST close any previous connection and associated security association state** for that endpoint. Client reconnected as same endpoint does not mean a second independent active security context on proxy. |
| CONN-04 | §3.5.6 | Connection authentication timeout ⇒ MUST close unauthenticated TCP + states. Idle timeout ⇒ SHOULD close stale. This does not prescribe automatic user sign-out or the exact client reconnect backoff; MS-SIPREGE/CONMGMT failover and RFC3261 must be separately reconciled. |
| CONN-05 | §3.4.6 and §3.5 | Client transport dead, server dropped registration, active SIP subscription and RTP/ICE remaining media are **different states**. Losing a first-hop connection can impair signaling while already established media has independent ICE consent; do not synthesize `SIP BYE` not present in source. |

## V. Composite state machine (S, not an observed client packet trace)

~~~mermaid
flowchart TD
 A[User SIP AoR] --> B[4 parallel DNS SRV and optional DHCP 120/60]
 B --> C[Order TLS/TCP endpoints, FQDN and per-record priorities]
 C --> D[Fresh A lookup, sequential IP connect]
 D --> E{Outcome}
 E -->|DNS A or TCP connect fails| F[Next ordered endpoint if available]
 F --> D
 E -->|Other failure or none remain| G[Discovery failure]
 E -->|TCP usable| H{TLS service selected?}
 H -->|Yes| I[Validate certificate identity and TLS handshake]
 I -->|Fail| G
 H -->|No| J[Plain TCP only if selected record and deployment security policy]
 I --> K[Optional SIPCOMP then SIP request]
 J --> K
 K --> L[Client proposes Ms-Keep-Alive UAC hop-hop]
 L --> M{Successful UAS response with one valid matching header?}
 M -->|No| N[No periodic CRLF keepalive on this connection]
 M -->|Yes| O[Client refresh 2/3 server timeout and proxy expiry+grace]
 O --> P[CRLF CRLF on timer; traffic resets timers]
 P --> Q{Proxy expired and registrar?}
 Q -->|Yes| R[Remove dead binding SHOULD; NOTIFY prohibited]
 Q -->|No| S[Connection lifetime/proxy idle independent]
~~~

**Synthesis and security caution:** The client may initially establish transport but remain unauthenticated for SIPAE and unprovisioned for contacts/MCU/media. Proxy 32-second anti-DoS timer does not imply complete Windows client sign-in deadline. The *same* logical user logged in on multiple physical endpoints has **distinct GRUUs and endpoint identities**, so §3.5.5's close-old-connection rule must be matched by *endpoint identity*, not user SIP AoR, otherwise MPOP would be accidentally suppressed.

## VI. Unresolved original facts and version compatibility gates

- **CON-X01:** §3.1.5 permits a mix of TLS and TCP discovery candidates in historical profile; §3.2 mandates RFC5246 certificate authentication **when TLS selected**, and overview §2.9.1.5 recommends protected SIP signaling for SRTP inline keys. Exact enterprise policy forbidding insecure SIP or fallback must be sourced from **target full-client build** and provisioning, not deduced from general presence of TCP candidate.
- **CON-X02:** DNS SRV + DHCP priority with mixed early query completion is a conditional ordering/state question; no fixed sequential 4-query transcript valid. Need special DNS SRV weight/priority/record ordering rule, load balancer and IPv6 behavior in target product.
- **CON-X03:** Validate correct interpretation of §3.1.5 DHCP Option120 records vs RFC3361 address/domain encodings; Microsoft says **SHOULD ignore raw IP addresses returned**. Need full RFC3361 §3 distinction before classifying every implementation variant.
- **CON-X04:** TLS hostname matching: source §3.3 cites **RFC2459** subject FQDN, RFC5246 TLS1.2; target Windows security platform may have newer SAN precedence/revocation and TLS versions. Neither disable those checks nor invent new required cipher profile without original Windows docs and edition facts.
- **CON-X05:** Asymmetric duplicate `Ms-Keep-Alive` request first-only versus response ignore-all, sender MUST NOT duplicate; malformed 2*CRLF framing over MS-SIPCOMP compressed transport and proxy TLS buffering need lower-layer failure analysis.
- **CON-X06:** Proxy close-old-security-association for **same endpoint** after new authentication (§3.5.5) must be reconciled with MS-SIPREGE multiple endpoints/MPOP. Endpoint must not be conflated with human account, and old media sessions may outlive signaling.
- **CON-X07:** Full client transport recovery for DNS failure vs TLS authentication failure, SIP 401/407 repeated challenges, 423 Min-Expires, SUBSCRIBE repopulation, TLS session cache and media ICE paths remains U pending exact MS-SIPREGE/client version. **Do not invent retries**.
- **CON-X08:** Modern [RFC5626](https://www.rfc-editor.org/rfc/rfc5626) SIP Outbound CRLF/STUN framework does not automatically replace Microsoft's own `Ms-Keep-Alive` hop-hop negotiation, even if byte contents overlap.

**Status: MS-CONMGMT 10.0 §§3.1–3.5 actively audited; complete Windows Full Client research NOT COMPLETE. No protocol code, real-server/interoperability tests, main merge or release.**
