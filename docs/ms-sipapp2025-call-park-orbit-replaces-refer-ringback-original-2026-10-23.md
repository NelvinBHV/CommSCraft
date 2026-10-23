# MS-SIPAPP 2025: Call Park service, parked audio dialog, orbit reuse, REFER/Replaces, ringback and fallback

**Primary original analysis 2026-10-23. STATUS: selected exact Call Park clauses verified, full telephony study still INCOMPLETE.** Microsoft **[MS-SIPAPP] Session Initiation Protocol (SIP) Application Protocol, v20250520 released 2025-05-20**, original owner PDF SHA-256 `9ba105dd85f05861b528f4fb5f527da65acda466b0480e5fb67d50dd64ac428a`; Microsoft official [Call Park example](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipapp/59cff24c-77ab-48dd-b398-17a2c556c075) and [SIP 200 park-response wire](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipapp/0c415b45-5a52-47c2-8b8b-6a7cf900adfd). Original sections **§§1.3–1.7,2.2.1.1–3,3.1.1–3.1.7,4.1.1–4.1.5**; 40-page PDF selectively audited. Dependencies [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) §§13–15, [RFC3515](https://www.rfc-editor.org/rfc/rfc3515) REFER, [RFC3891](https://www.rfc-editor.org/rfc/rfc3891) Replaces, [RFC3840](https://www.rfc-editor.org/rfc/rfc3840) `isCps` SIP Contact tag, [RFC4566](https://www.rfc-editor.org/rfc/rfc4566) SDP, [MS-SIPRE] rev13 §§2.2.10,3.9.5.2.1, [MS-SIPREGE] rev17.1 §2.2.2.5.1 (Call Park service URI provisioning), [MS-SDPEXT] rev16 §3.1.5.27 (MOH), [MS-OCER] rev11 error IDs. **P** direct original normative role; **S** synthesized lifecycle; **U** unresolved configuration/build.

## A. Original client and CPS message and state rules

| ID | Edition-qualified source | Mandatory/optional source behavior |
|---|---|---|
| PARK-01 | MS-SIPAPP §§2.2.1.1,3.1.5.1 | Parker **UAC** initiates a **new INVITE dialog to provisioned Call Park Service URI**, `Content-Type: application/ms-call-park+xml`, body **`park-request version="1.0" request-id="..."`**. XML has exactly one mandatory **audio** element with **dialog-info(call-id,from-tag,to-tag)** of *existing voice dialog* plus **target=other party's GRUU**. SIP Call-ID of **new CPS INVITE** and XML old-dialog call-id **MUST NOT be confused**. |
| PARK-02 | §2.2.1.1 | `version` formatted major.minor (digits); for client source requires **1.0**; request-id arbitrary correlation string. Request audio XML may contain optional additional modality fields but **CPS MUST ignore** non-audio media. No proof park automatically preserves video, RDP app-sharing or conference Focus. |
| PARK-03 | §§3.1.5.1–3.1.5.2 | CPS chooses free **orbit** from its pool; on success initiates **SIP INVITE with Replaces** to the parkee's GRUU, with **audio-only SDP** that SHOULD indicate hold/music-on-hold. If successfully replaced CPS returns parker's INVITE **200 OK** with `park-response` XML and `isCps` Contact feature tag. A CPS response before Replaces succeeds is **not** a confirmed successful park. |
| PARK-04 | §2.2.1.2 | XML `park-response` contains **orbit** dial string and **ms-parked-call** unique identifier, with request-id echoed and version 1.0. Orbit syntax **1–9 digits**, or '*'/'#' followed by **1–8 digits**; pool must avoid enterprise extension overlap SHOULD. **Orbit may be reused**, so unique ms-parked-call disambiguates over time. |
| PARK-05 | §§3.1.5.1–3.1.5.3 | Invalid park XML/schema ⇒ CPS **415**; unsupported major version may lead **488** with diagnostic **35008 SHOULD**; no free orbit ⇒ **500** with diagnostic **35002 SHOULD**; failing CPS-to-parkee Replaces ⇒ parker **408** with diagnostic **35007 SHOULD**. These are **CPS/application-specific**, not general account login failure. |
| PARK-06 | §§3.1.2,3.1.5.2 | After successfully parking, CPS starts **per-parked-call auto-ringback timer**; Microsoft source **does not specify numeric timer duration** (deployment adjustable). **Do not invent a default like 30s or 60s**. This timer is not RFC4028 dialog session or TURN allocation lifetime. |
| PARK-07 | §3.1.5.4 | CPS receiving **BYE on parker↔CPS control dialog MUST NOT automatically end parkee↔CPS parked voice dialog**. However subsequent unpark notifications to closed parker control dialog **MUST NOT be sent**. These are **different dialogs**. |
| PARK-08 | §3.1.5.5 | If parkee hangs up and park control dialog still present (and not in transfer branch), CPS sends parker a SIP **INFO** with XML `unpark-notification reason=hang-up` (**no target**), then CPS BYE control dialog and returns orbit to pool. This SIP INFO is **not conference C3P INFO** or XCCOS group chat INFO. |
| PARK-09 | §3.1.5.6 | Retriever calls **orbit** with INVITE/audio SDP; SIP proxy routes orbit-pool number to CPS; no call for that orbit ⇒ CPS **404** and diag **35000 SHOULD**. Orbit busy during park/retrieval/ringback/fallback ⇒ CPS **488 MUST**. |
| PARK-10 | §3.1.5.6 | CPS accepts valid unpark INVITE and identifies with `Contact;isCps`. It cancels ringback timer and sends **REFER + Replaces** to retriever, `Refer-To` parkee GRUU and Replaces dialog-info for parkee↔CPS dialog. Source RFC3515 + RFC3891 role precise; **CPS does not directly send RTP between final reconnected users indefinitely**. |
| PARK-11 | §3.1.5.7 | On successful retrieval CPS sends parker, if control dialog still active, `INFO application/ms-call-park+xml` with **reason=retrieval** and **target=AoR of retrieving user**, then BYE control dialog with diagnostic **35014 SHOULD**; orbit/resource release. |
| PARK-12 | §3.1.5.8 | On failed retriever transfer CPS **BYE retriever↔CPS** and, if parked voice dialog remains, **restarts auto-ringback timer**. If parkee has disconnected then sends hang-up notification/cleanup if parker control dialog survives. Not a mandatory restart of caller's original SIP dialog. |
| PARK-13 | §§2.2.1.3,3.1.5.8–12 | `unpark-notification` reason enum exactly **retrieval | hang-up | ringback | fallback | drop**. For retrieval/fallback `target` **required**, ringback **optional**, hang-up/drop `target MUST NOT be present`. Client should map by same request-id/version; status semantics from these values not SIP response codes. |
| PARK-14 | §§3.1.6,3.1.5.9–12 | On auto-ringback timer expiry CPS sends **REFER** to parkee with target parker's **AoR**, including escaped `ms-sensitivity=private-no-diversion`; if repeated ringback unsuccessful it may REFER parkee to configured fallback, else **drop** and release call. This **privacy-sensitive header** is a Microsoft routing rule, not generic RFC3515 syntax alone. |
| PARK-15 | §§3.1.5.10–12 | Successful fallback ⇒ **INFO reason=fallback**, target SHOULD fallback URI; no fallback / fallback transfer failure ⇒ **INFO reason=drop** without target, CPS BYE parkee and parker control, release orbit. CPS diagnostics **35014 SHOULD** on cleanup BYE. |
| PARK-16 | §§3.1.1–3.1.3 | CPS can persist parked-call state over server restarts (**MAY**), does not require every deployment to restore. Failed/restarted CPS, stale orbit and wrong ms-parked-call identity need product/data persistence evidence; do not invent guarantee parked calls survive restart. |
| PARK-17 | §4.1 example + MS-SIPREGE §2.2.2.5.1 | Call Park availability/URI is conditional on **in-band provisioning**. Original example `park-response` shows separate `orbit` and `ms-parked-call`; example NTLM SIP Security/route tokens belong to MS-SIPAE/MS-SIPRE, not an inherent CPS-only security mechanism. |
| PARK-18 | §§3.1.5.7–12 | Completing CPS control dialog and releasing orbit is separate from future regular user-to-user RTP/ICE/SRTP and SIP dialogue. A `reason=retrieval` report from CPS does **not** prove post-transfer media consent/audio usability, which requires independent MS-ICE2 and MS-SDPEXT rules. |

## B. Causal call park, retrieve and failure state model (S, not production packet capture)

~~~mermaid
sequenceDiagram
 participant A as Parker Windows UAC
 participant B as Parkee endpoint
 participant CPS as Call Park Service
 participant R as Retriever Windows UAC
 Note over A,B: Ordinary audio call already exists
 A->>CPS: INVITE park-request XML old dialog-info and B GRUU
 CPS->>B: INVITE Replaces for parked audio, optional music-on-hold
 B-->>CPS: 200 or failure
 alt Replaces fails
  CPS-->>A: SIP 408, no completed park
 else Replaces succeeded
  CPS-->>A: SIP 200 park-response orbit and ms-parked-call
  Note over CPS: Start per-call configurable auto-ringback timer
  R->>CPS: INVITE to orbit, with audio SDP
  alt Empty or busy orbit
   CPS-->>R: 404 empty or 488 busy
  else Available parked call
   CPS-->>R: 200 with Contact isCps
   CPS->>R: REFER with Replaces of B-CPS dialog
   R->>B: INVITE Replaces via SIP routing
   alt Transfer completes
    CPS->>A: INFO unpark-notification reason retrieval
    CPS->>A: BYE control dialogue
   else Transfer fails and B remains
    CPS->>R: BYE
    Note over CPS: Restart auto-ringback timer
   end
  end
 end
~~~

**Not universal order:** The example omits SIP interim replies, NOTIFY from REFER, ICE re-INVITEs and proxy hops (per source §4.1), and these omissions cannot imply messages do not occur.

## C. Independent identities/timers and unresolved research

- **PARK-U01:** `park-request.request-id` correlates XML, `orbit` is dialable/reusable number, `ms-parked-call` is unique per parked episode, XML original `dialog-info` is a different SIP call than parker-CPS control dialog. These MUST remain separate from conference Focus call/meeting IDs, SIPAE SA counters, RTP SSRC and XCCOS chat CmdId.
- **PARK-U02:** Exact `auto-ringback timer` duration, number of fallback cycles and provisioning permissions are server-configurable, **not specified as fixed wire constants**. Further original product notes/server policy required.
- **PARK-U03:** `ms-sensitivity=private-no-diversion` is routed inside escaped REFER target, but full privacy of all gateways/proxies and P-Asserted-Identity trust under RFC3325 need MS-SIPRE §3.9.5.2.1 direct cross-audit.
- **PARK-U04:** RFC3515 and RFC3891 REFER/Replaces NOTIFY subscription transitions, call transfer failures, RFC3261 fork/cancel/ACK and SDP hold/media transfer must be incorporated to reach true call-level guarantee; this artifact is CPS-specific.
- **PARK-U05:** CPS server-only `isCps`, orbit allocation, 350xx diagnostics and fallback state are not UI/wire obligations of an ordinary client; must avoid role leakage.
- **PARK-U06:** Feature may be disabled by deployment and is different from standard call hold, transfer or conference bridging. Version-specific SfB Windows client enablement still unverified.
- **PARK-U07:** Some CPS example fields use old RTCC client/server versions; primary PDF v20250520 itself does not prove particular deployed build or sign-in method. No assistant interop tests.
- **PARK-U08:** Complete 40-page original not exhaustively line-by-line audited, especially product notes and RFC3515 NOTIFY, all diagnostics and transfer edge case races; no full call feature completion claim.

**Research incomplete, all implementation planning CLOSED. No protocol code, server connections, interoperability tests, main merge or release.**
