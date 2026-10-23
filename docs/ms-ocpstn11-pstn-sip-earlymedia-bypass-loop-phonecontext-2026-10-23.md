# MS-OCPSTN rev11 — SIP↔PSTN gateway extensions, dial plan, early media and 605→480 loop handling

**Research tranche 2026-10-23, first-party original sections only; NOT a complete Windows Full Client PSTN compliance claim.** Microsoft [MS-OCPSTN] *SIP for PSTN Calls Extensions*, **rev11.0, v20250520, release 2025-05-20**, archived original 63-page PDF SHA-256 **c66e679e049fff858c4b2a988aa2d3c9f07fa9fa13622f630c138cd3f8d63f56**; official source https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OCPSTN/%5BMS-OCPSTN%5D.pdf (direct link was inaccessible to public web fetch on this session, but **local original PDF bytes independently hashed against archive** and original pp13–14 rendered). Primary §§1.3,2.1–2.2.9, **3.1–3.9**, source examples §4.1–4.5 partially crosschecked. Direct Microsoft dependencies **[MS-SDPEXT] rev16**, **[MS-SIPAE] rev11**, **[MS-SIPRE] rev13**, **[MS-SIPREGE] rev17.1**. Principal IETF originals: [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) SIP (2002), [RFC3262](https://www.rfc-editor.org/rfc/rfc3262) reliable provisional PRACK (2002), [RFC3264](https://www.rfc-editor.org/rfc/rfc3264) SDP O/A (2002), [RFC3966](https://www.rfc-editor.org/rfc/rfc3966) TEL URL (2004), [RFC3325](https://www.rfc-editor.org/rfc/rfc3325) asserted identity trusted network, [RFC2045](https://www.rfc-editor.org/rfc/rfc2045) MIME, [RFC2111](https://www.rfc-editor.org/rfc/rfc2111) Content-ID, [RFC5234](https://www.rfc-editor.org/rfc/rfc5234) ABNF. The archive also lists RFC3263,3515,3891,3892,2119. Microsoft Learn official original [PSTN INVITE example](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocpstn/8af9ca0c-f0cf-4301-a47d-ffdaafe92480). **P** Microsoft direct MUST/SHOULD, **R** RFC, **S** cross-spec deduction, **U** feature-gated unresolved.

## 1. Nine wire extensions, role-scoped semantics

| ID | Source | Exact supported role and requirement |
|---|---|---|
| PSTN-01 | §§2.1,2.2.1,3.1 | **`Contact: <sip:...>;isGateway`** Contact parameter: **gateway-role UAC MUST insert**, nongateway UAC **MUST NOT**. Receiving UAS SHOULD render gateway indication to application, application MAY show in UI. This is **not** proof user has PSTN dialing privilege. |
| PSTN-02 | §§2.2.2,3.2; RFC3966 §5.1.5 | `phone-context` retains RFC3966 syntax but extends meaning: **user's provisioned dial-plan location profile name**, not civic/geopolitical location. If dial string not resolved to E.164 based on local provisioning, UAC MUST issue SIP Request-URI with dial-string phone-number and phone-context. If location profile missing and not E.164 MUST use reserved default **`dialstring`**. A non-E.164 number produced by enterprise dialing rules can have **`enterprise`** context. |
| PSTN-03 | §3.2 | For callback on incoming P-Asserted-Identity with private number and phone-context `dialstring` or `enterprise`, supporting UAC **SHOULD populate Request-URI and To** from received PAI. **PAI must be trusted under RFC3325 network identity rules**; don't treat arbitrary untrusted From/PAI as authority for callback costs/permissions. |
| PSTN-04 | §§2.2.3,3.3 | `Ms-Call-Source: ms-rtc` or `non-ms-rtc`. Only **gateway-role UAC** MUST send: to protocol-network UAS `ms-rtc`, toward PSTN `non-ms-rtc`; nongateway MUST NOT insert. Marks a gateway/PBX origin for loop detection, **not** a client-selected media codec. |
| PSTN-05 | §3.3 | A UAS currently **alerting in another dialog** and receiving INVITE with `Ms-Call-Source` **MUST reply 605**. Proxy receiving 605 in response to INVITE with that header **MUST send 480 upstream**. This is explicitly a *loop/alerting-state prevention branch*; **do not expose raw 605 as user-terminal call refusal** when gateway proxy translates it. |
| PSTN-06 | §3.3 | SIP proxy strips `Ms-Call-Source` on forwarding to a **different user** under forwarding rules or user **not provisioned to receive loop calls**. Thus caller may not see full header across network. |
| PSTN-07 | §§1.3,2.2.4,3.4 | `Supported: ms-early-media` advertises that caller accepts SDP answer in **unreliable SIP 183 Session Progress**. UAC supporting it **MUST advertise**; **gateway-role UAS MUST send unreliable 183 with SDP answer** upon matching Supported tag; non-gateway UAS **MAY**. This is a deliberate historical Microsoft extension to RFC3262 reliable provisional-answer recommendation, not a replacement for SIP final INVITE completion. |
| PSTN-08 | §§2.2.5,3.5 | Anonymous phone origin uses **`From: <sip:anonymous@host;user=phone>`**, host actual IP/FQDN/domain context; source expressly **MUST NOT use `anonymous.invalid` host** in this format. This is gateway/application syntax, not permission to bypass SIP authentication/security. |
| PSTN-09 | §§2.2.6,3.6 | `Supported: ms-bypass` gateway media-bypass capability **SHOULD** be advertised by a UA that supports direct PSTN gateway media. Tag advertisement ≠ bypass selected, direct route possible, or media secured; ICE/SDP/SRTP/enterprise policy are separate. |
| PSTN-10 | §§2.2.7,3.7; RFC2045,RFC2111 | For multipart offers, UAC **MUST put Content-ID** on each MIME part of type `application/SDP` and `application/gw-sdp` (per source capitalization). When UAS answer selects an offered SDP with Content-ID, **MUST echo its identifier via `ms-accepted-content-id`**. Correlates negotiated SDP alternative to offer, not media ICE pair or authentication digest. SIP proxy has no special behavior. |
| PSTN-11 | §§2.2.8,3.8 | `ms-trunking-peer: host;trunk=hostname;User-Agent="..."` sent by **gateway-role SIP UA** identifies particular PSTN trunk peer. Not mandatory outgoing Windows client header if it is **not gateway**. |
| PSTN-12 | §§2.2.9,3.9 | `ms-mediation-generated: yes` in **provisional responses** from gateway-role UA signals the response generated locally by mediation, **not forwarded from PSTN gateway**. Distinguish true downstream early progress from synthetic gateway status. |
| PSTN-13 | §2.2.2 vs MS-E911WS §§2.2.4–3.1.4 | `phone-context` is a **dial-plan selector**. The similarly named network/physical location lookup `Location Profile`/LIS (MS-E911WS) and TURNBWM site's location flag are **different namespaces and trust scopes**. Never feed unverified dialing profile strings to emergency street-address fields. |
| PSTN-14 | §1.3; RFC3262/3264 | Early-media 183 carries possible **SDP answer before final dialog confirmation**. SIP final 200+ACK, RTP path/ICE consent, cryptographic media protection, PSTN billing and media-bypass all have **independent state and error paths**. Do not infer answered call from hearing early ringback RTP. |
| PSTN-15 | §4.5.2.1 example | Microsoft original PSTN client INVITE includes **multipart/alternative SDP** and potentially `Supported: ms-early-media`, `Supported: ms-bypass`, signed SIP Proxy-Authorization, multiple audio codecs and ICE candidates. This is an **illustrative product example**; no proof every 2015 installation has same codecs/SRTP offer profile, credentials or optional bypass flag. |

## 2. Conditional SIP/PSTN composition (S = inference from explicit lower source, not capture)

~~~mermaid
flowchart TD
 A[Registered Windows SIP client with provisioned dial plan] --> B[User enters dial string or callback]
 B --> C{Resolve to E164?}
 C -->|Yes| D[Compose SIP Request-URI using resolved E164]
 C -->|No| E[Compose private number and phone-context profile or dialstring]
 D --> F[INVITE through SIP proxy, auth, optional multipart SDP]
 E --> F
 F --> G[Proxy and mediation server select gateway/PSTN trunk]
 G --> H{Early media advertised?}
 H -->|Yes| I[Potential unreliable 183 with SDP answer]
 H -->|No| J[Standard provisional handling per SIP/SDP profile]
 I --> K[Independent media/ICE path and security]
 J --> K
 K --> L{Call answered and final SIP dialog?}
 L -->|No| M[Reject/redirect/cancel/timeout per SIP and routing protocols]
 L -->|Yes| N[Final 200/ACK and media state, with BYE cleanup]
 G --> O{Gateway loop detected by Ms-Call-Source?}
 O -->|Yes| P[UAS 605; upstream proxy translates 480]
~~~

**Gateway loop substate** applies only to gateway-tagged invitation received during another alerting dialog; it **must not be applied globally to any two simultaneous SIP calls**. Client direct SIP UAC should not fabricate gateway-only headers.

## 3. Explicit unresolved source/edition duties

- **PSTN-U1:** Actual 2015/2016 client build support, provisioning dial-plan E164/private dialing resolution, location profile names, callback from PAI, normalization and policy authorization must be derived from product edition/configuration and MS-SIPRE/MS-SIPREGE; no universal feature permission claim.
- **PSTN-U2:** The document's generic §3.x subheadings largely state “None” for independent timers/state; it **inherits RFC3261 INVITE/provisional/SIP session, RFC3264 SDP and MS-SDPEXT/ICE/SRTP**. No basis to claim calls have no timers or all requests follow one fixed sequence.
- **PSTN-U3:** Full multipart/alternative and fallback behavior of `ms-accepted-content-id` when absent, selected SDP RTP/SRTP payload and RFC3262 100rel/PRACK interaction is a detailed open normative linkage.
- **PSTN-U4:** RFC3325 trusted-domain limits on P-Asserted-Identity, number privacy, gateway-authenticity and anonymous phone URI must be audited before any caller-ID/authorization requirements.
- **PSTN-U5:** Public safety emergency route/PSAP and civic address freshness are **not defined** by MS-OCPSTN phone-context or `ms-bypass`. Requires separate emergency application/proxy policy and location protocols.
- **PSTN-U6:** Source PDF original officially archived and SHA-verified locally but public direct PDF URL failed in this web session. Official Microsoft Learn example is public and 2019-era illustrative; original release/edition verified from archive footer, **not asserted latest deployment reality**.

**Full client compliance and overall source closure INCOMPLETE; implementation gate CLOSED. No code, actual servers, assistant interoperability tests, main merge or release.**
