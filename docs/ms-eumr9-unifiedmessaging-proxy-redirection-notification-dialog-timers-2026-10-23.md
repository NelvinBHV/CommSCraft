# MS-EUMR rev9 — Exchange UM voicemail routing, redirect caps, proxy-owned notifications and SIP INFO XML

**Research-only first-party deep-clause audit 2026-10-23; not proof that Exchange UM exists in every SfB deployment.** Official Microsoft **[MS-EUMR] Routing to Exchange Unified Messaging Extensions**, **rev9.0, v20250520 (2025-05-20)**, archived original 29-page PDF SHA-256 **483eea91ca631981393ff96e512620051377b1463962b93b1d5f42cbc1157ed3**, original source https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-EUMR/%5BMS-EUMR%5D.pdf (direct public PDF inaccessible via browser this session, original local archive SHA verified), pp15–16 visually rendered. Primary clauses **§§1.3,2.1,2.2.1–2.2.6,3.1.1–3.1.6,3.2.2–3.2.6,4.1–4.3**. Microsoft dependencies **[MS-SIPRE]**, **[MS-SIPREGE]**, **[MS-AVEDGEA]**, **[MS-TURN]**, **[MS-QoE]**. Original RFCs **[RFC2327](https://www.rfc-editor.org/rfc/rfc2327)** legacy SDP, **[RFC2976](https://www.rfc-editor.org/rfc/rfc2976)** original SIP INFO, **[RFC3261](https://www.rfc-editor.org/rfc/rfc3261)** SIP, **[RFC3892](https://www.rfc-editor.org/rfc/rfc3892)** Referred-By, **[RFC5234](https://www.rfc-editor.org/rfc/rfc5234)** ABNF, [RFC2119](https://www.rfc-editor.org/rfc/rfc2119). **P** direct Microsoft original actor clause, **S** cross-source synthesis, **U** unresolved version/topology.

## 1. Outbound voicemail routing is SIP proxy state, not client-initiated arbitrary UM access

| ID | Section | Exact role-scoped rule |
|---|---|---|
| EUM-01 | §§1.3,3.1.1 | **SIP proxy** routes media calls to enabled Exchange Unified Messaging based on user dial-plan mapping, ordered UM servers and caller routing policy. PSTN/media caller may just be an ordinary SIP UA; **routing database and server-selection timer live on proxy**, not Windows client. |
| EUM-02 | §2.2.2 | When proxy forwards INVITE to a UM server, Request-URI **user part = user's dial plan name**, **host = target UM server FQDN**, **port = 5061**, parameters **transport=tls** and **maddr=<UM FQDN>**. Distinct from user-facing `sip:telephone;phone-context=...` under MS-OCPSTN. |
| EUM-03 | §§2.2.1,3.1.5 steps3–4 | Where A/V Edge configured, proxy MUST attach `Ms-Mras-Address: <sip:...>` identifying Edge **GRUU**; UM can obtain Edge relay long-term credentials via **MS-AVEDGEA** and speak MS-TURN to external caller. Edge GRUU is not endpoint user GRUU nor TURN allocation itself. |
| EUM-04 | §3.1.5 steps1–2 | Before forwarding voicemail INVITE, proxy **MUST remove preexisting Diversion headers**; if original Request-URI differs in user/host from From, proxy **MUST add Diversion** for original request user URI sans parameters/headers. Security/privacy of trusted history must be examined across hop boundaries, not blindly relay caller-supplied Diversion. |
| EUM-05 | §3.1.5 steps5–9 | Proxy SHOULD add `Supported: Ms-Fe`, MUST select UM servers from target user's dial plan, restrict to highest server version when mixed, favor front ends/arrays per specified SHOULD, and process UM's `Ms-fe` contact parameter. |
| EUM-06 | §§3.1.2,3.1.5,3.1.6 | Per-UM-server INVITE **routing timer MUST be <180 seconds**, recommended **5 seconds**. On expiry or failure proxy **MUST try next UM server**. These are **proxy-specific** not SIP UAC response timers, MS-CONMGMT 32s anti-DoS deadline or PSOM keepalive. |
| EUM-07 | §§3.1.5.3,.7 | UM may return **302**. Proxy **MUST consume and not forward 302 to caller**, redirect only if response from selected UM, **exactly one Contact**, Contact references UM in user's **dial plan**, and **no more than four previous 302 redirects** for this attempt. Otherwise try next UM. Redirect is **not open arbitrary SIP Contact URL forwarding**. |
| EUM-08 | §§3.1.5.4–.5 | Any time proxy forwards to new UM server, **SHOULD send SIP `101 Progress`** to calling UA. For UM **415 Unsupported Media Type** on multipart MIME INVITE proxy MUST clear its UM ordering and apply MS-SIPRE §3.9.5.5, potentially constructing a new offer before rerouting; no universal resend of same unmodified SDP. |
| EUM-09 | §3.1.5.7 | Exhausting UM list => call **SHOULD terminate with SIP 480**. This is different from MS-OCPSTN **gateway loop** UAS 605 translated 480: the **upstream same 480 code can have different causes**, requiring provenance/diagnostic handling. |
| EUM-10 | §§3.2.2–3.2.3 | **Proxy**, not desktop UA, establishes a **mutually authenticated TLS (MTLS) connection** and dedicated SIP INVITE dialog with UM for user-notification events. MUST reuse existing dialog with an eligible UM server in same dial plan; **10-minute User Notification Inactivity timer**, reset when notification sent, expiry SHOULD terminate dialog. |
| EUM-11 | §§2.2.4.1–3,2.2.5 | Notification dialog has **fixed system identity `sip:A410AA79-D874-4e56-9B46-709BDD0EB850` in From**, Request-URI empty user part, UM FQDN host, `transport=tls;opaque=app:rtcevent`. SDP `m=application 9 SIP *`, sendonly and `a=accept-types:application/ms-rtc-usernotification+xml`; UM 200's SDP recvonly and `ms-rtc-accept-eventtemplates:RtcDefault`. **This is not an A/V RTP stream or a normal voice INVITE from the user.** |
| EUM-12 | §§2.2.3–2.2.3.3 | Proxy event is SIP `INFO` containing **Content-Type: application/ms-rtc-usernotification+xml**; XML requires `User` valid SIP URI, `Time` UTC string, `Template=RtcDefault`, `Event @type=missed|answered|forbidden`, `CallId` original INVITE Call-ID, and `From` original INVITE header. Optional: `EumProxyAddress`, `Subject`, `Priority`, `ConversationID`, `ReferredBy`, `Target`, `AnsweredBy`, `MissedReason`. These fields may carry sensitive call metadata. |
| EUM-13 | §2.2.3.3 | **`Event type=answered`** requires `Target`, `TargetClass` and `AnsweredBy` according to schema prose; **`Event type=forbidden`** requires `Target` and `TargetClass`. Potential **conditional conflict**: §§3.2.4.2.1 only says include `AnsweredBy` if SIP response includes P-Asserted-Identity, whereas 2.2.3.3 says mandatory for any answered Event. Must not fabricate identity if absent; mark unresolved. |
| EUM-14 | §§3.2.4.1,3.2.4.1.1 | Missed call (caller released before response, or all preamble targets negative except privacy exclusion `Ms-Sensitivity:private-no-diversion`) triggers proxy INFO **if UM-enabled**. `MissedReason` = `CallerReleased` or `Declined`; other origin conversation fields echoed only when present. |
| EUM-15 | §§3.2.4.2,.2.1 | `answered` call notification when target other than user registered endpoint/simultaneous ring answered; source includes SIP **200/303/605** as answer event. `TargetClass=secondary` for team/delegate ringing, primary otherwise. This is a **server-side notification convention**, not proof every listed SIP code represents a connected audio media dialog to user. |
| EUM-16 | §§3.2.4.3,.3.1 | `forbidden` event when call forwarded beyond user's registered endpoint and receives **403**; for team/delegate targets proxy **SHOULD NOT** emit forbidden event, but if it does, must classify as secondary. These notifications are **not the same as phone user interface missed-call toast or local call history entry**. |
| EUM-17 | §§3.1.5.1–.2 | UM can access A/V Edge relay tokens when caller external and publish separate **MS-QoE** telemetry at call end. Successful UM routing is not guaranteed QoE persistence; earlier MS-QoE source states accepted/report stored are separate. |

## 2. Server-side route versus notification dialog (S synthesis)

~~~mermaid
flowchart TD
 A[Windows client or PSTN gateway call INVITE] --> B[SIP proxy routing/policy MS-SIPRE]
 B --> C{Route target enabled for UM?}
 C -->|No| D[Ordinary call path; this UM extension not activated]
 C -->|Yes| E[Select eligible UM server by dial plan/version]
 E --> F[Rewrite UM RequestURI user=dialplan host=UM FQDN port5061 TLS]
 F --> G[Start per-server timer recommended 5s <180s]
 G --> H{UM response}
 H -->|302 with one allowed contact and redirect cap| I[Proxy redirects to allowed UM; not caller]
 H -->|415 multipart| J[Reset ordering; process SIPRE fallback and fresh offer if required]
 H -->|timeout/error| K[Try next eligible UM; on exhausted list SHOULD 480]
 H -->|accepted| L[UM voice mail SIP/RTP media distinct from caller state]
 B --> M{UM-enabled call notification-worthy event?}
 M -->|Yes| N[Proxy MTLS separate UM notification INVITE dialog]
 N --> O[INFO XML missed/answered/forbidden; 10min idle timer]
 O --> P[UM generates email outside client SIP]
~~~

**Security boundary:** The UM voice-mail route is **not a new SIP desktop transport**. `101 Progress` on UM reroute is a provisional SIP progress event, not session established or RTP consent. UM's `Ms-Mras-Address` relay authority stays bound to trusted proxy→Edge flow. Email event captures call identities, redirection/permission context and timestamps; no client should synthesize forged notifications as if it were trusted proxy.

## 3. Explicit verification/edition gaps

1. **EUM-U1:** [MS-EUMR] §2.2.3.3 `answered` requires AnsweredBy; §3.2.4.2.1 only directs it if PAI available. Need official error/default handling to avoid inventing participant identity.
2. **EUM-U2:** Original 2025 text uses **RFC2327** and **RFC2976** historic editions; modern RFC4566 / RFC6086 do not silently replace legacy UM event SDP/INFO behavior.
3. **EUM-U3:** Exact UM failure code classes, 302 redirect loop count interpretation, whether 415 multipart fallback applies to Windows client offered SDP across product/server versions must be established from [MS-SIPRE], not guessed from overview.
4. **EUM-U4:** Exchange UM feature applicability is version/topology-dependent (Exchange 2013/2016 on-prem legacy). The 2025 Open Specification is not proof latest Exchange Online or every Skype for Business Server Edition hosts the same UM service.
5. **EUM-U5:** No authority here to emit non-user-initiated emergency calls, divert callers, update Active Directory configuration, or notify live UM; user's explicit restriction prohibits every such test.
6. **EUM-U6:** Complete 29-page source, application schemas §6, lower-level proxy routing script details and legacy Microsoft MS-SIPRE/Edge interplay not exhaustively audited in this tranche.

**Full protocol/Windows Full Client normative closure INCOMPLETE. No code, real server traffic, owner-only interoperability testing, main merge or release.**
