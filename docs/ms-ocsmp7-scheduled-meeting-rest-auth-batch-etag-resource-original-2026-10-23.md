# MS-OCSMP 7.0 — online conference scheduling REST resources, discovery, conditional updates and batch errors

**Research 2026-10-23. STATUS: original-clause audit, not completion of entire 93-page protocol nor proven classic Windows Full Client runtime.** Official primary [MS-OCSMP] *Microsoft Online Conference Scheduling and Management Protocol*, revision **7.0**, original release **2025-05-20**, original PDF `v20250520`, locally verified SHA-256 `a10c1ac1356aeed2d818685279362249a171a0db754a6de10fb49e5a693d48da`. Original: https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OCSMP/%5BMS-OCSMP%5D.pdf . Original pp10,29,35,40,43 directly rendered and examined; full indexed source §§1.3,2.1–2.2.7.7,3.1.1–3.1.5.13 and selected operation tables reviewed. Source's **normative RFCs** [RFC2387](https://www.rfc-editor.org/rfc/rfc2387) (multipart/related), [RFC2616](https://www.rfc-editor.org/rfc/rfc2616) (HTTP/1.1 historical), [RFC2818](https://www.rfc-editor.org/rfc/rfc2818) (HTTPS historical), [RFC2119](https://www.rfc-editor.org/rfc/rfc2119) requirement words; references **[MS-OCDISCWS]** for discovery and **[MS-OCAUTHWS]** for authentication (source §1.2 classification to audit; not infer one interchangeable login token). Microsoft Learn original [§2.2.7.1 Error](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsmp/584c3e40-06b0-47c8-86c2-63e9ce94b6a8). **P** = direct Microsoft source + exact clause; **S** = conditional cross-protocol integration; **U** = target-build/edition or incomplete branch.

> **Scope warning (P, §1.3):** This is a service to create, change, list and cancel *scheduled online conferences*. The source cites a **web or mobile app as a typical client**, **NOT an assertion the classic Skype for Business Windows Full Client directly transmits each REST method**. The existing SIP [MS-CONFPRO] and [MS-CONFBAS] Focus/C3P live meeting admission flows remain independently normative.

## 1. Actor/field/status ledger

| ID | P source clause | Wire/state rule and effect |
|---|---|---|
| OCS-01 | §1.3;§3.1.1.2.3 | Client first discovers user **home server using MS-OCDISCWS** and provides credentials in user context. Server-oriented operations create/update/delete scheduled meeting resources; discovery and authentication precede resource links. This document does not define SIP REGISTER or TLS-DSK GSSAPI MIC. |
| OCS-02 | §2.1 | Transport is HTTP (RFC2616) **or HTTPS** (RFC2818); defaults **80/443 SHOULD**, nonstandard ports MAY. Request/response text **MUST use UTF-8** and **MUST NOT include BOM**. Source permits HTTP; do not infer plaintext appropriate for user credentials/tickets under secure deployment. |
| OCS-03 | §§2.2.1,2.2.3–2.2.7 | Root XML `input`, `resource` and `reason`; resources, links, property and propertyList/collection semantics. Media `application/vnd.microsoft.com.ucwa+xml` or `application/vnd.microsoft.com.ucwa+json` per operation, **NOT SIP XML or Teams Graph JSON**. |
| OCS-04 | §§3.1.1.2.1–2,3.1.1.3.1 | Hypermedia link discovery from Application→onlineMeetings→`myOnlineMeetings`, `onlineMeetingDefaultValues`, `onlineMeetingEligibleValues` and phoneDialInInfo; follow advertised **`href`/rel** rather than inventing fixed permanent REST paths. |
| OCS-05 | §3.1.5.2.1 | `POST applications` takes `ApplicationInput`: `culture`, `endpointId`, `userAgent` all nonempty per field table. **200** if application already resident; **201** newly created; **400** malformed/invalid. A 200 here does **not create a new scheduled meeting**. |
| OCS-06 | §§3.1.5.4–6 | `myAssignedOnlineMeeting` GET retrieves assigned meeting; `myOnlineMeeting` GET/PUT/DELETE acts on one meeting; `myOnlineMeetings` GET/POST lists/creates. A scheduled meeting document has **conferenceId, joinUrl, policies, leaders/attendees, expiration** — it is **not** live Focus/MCU session state. |
| OCS-07 | §2.2.7.2 | `OnlineMeetingInput` includes accessLevel, attendees/leaders, automaticLeaderAssignment, description/subject, entryExitAnnouncement, expirationTime (UTC), lobbyBypassForPhoneUsers, phoneUserAdmission. Presence of fields **does not itself authorize** a participant at conference Focus; policy enforcement occurs separately. |
| OCS-08 | §2.2.7.6 | `OnlineMeetingResource` includes `conferenceId` (PSTN dial-in identifier), `etag`, `joinUrl`, expiry, participants, assigned policies. Do not confuse **`conferenceId`** with SIP Call-ID, ASMCU session-id or ICE ufrag. |
| OCS-09 | §3.1.5.6.2 | `POST myOnlineMeetings` with `OnlineMeetingInput` returns **HTTP 200** on success and **400** invalid. SIP 200 and C3P INFO 202 are different status namespaces. |
| OCS-10 | §3.1.5.5.3 | `PUT myOnlineMeeting` normally follows GET and SHOULD provide **all resource properties, including unknown properties**, to avoid discarding additions from other clients. **200 success; 400 malformed; 404 missing**. Server makes **no automatic property delta reconciliation** (§3.1.1.5.3). |
| OCS-11 | §§3.1.1.5.1,.3 | Omitted optional fields can be defaulted by server; conditional `If-Match` with current `etag` **can be used** on update. Specification passage **does not explicitly assign 412 as mandatory** for all mismatch cases; do not fabricate status/retry. |
| OCS-12 | §3.1.5.5.1 | `DELETE myOnlineMeeting`: success **204** no content; **410** resource gone without forwarding; **404** missing. Do not equate HTTP delete with SIP BYE to active participant or RFC3261 REGISTER Expires=0. |
| OCS-13 | §3.1.1.2.5;§3.1.5.3 | Server **MUST support HTTP batch** of multipart `Content-Type:multipart/batching;boundary=...`. Each part **MUST contain one `application/http;msgtype=request`** embedded HTTP request, response per part `application/http;msgtype=response`. Parts **SHOULD be processed without ordering guarantees**; client **SHOULD keep batches small**. |
| OCS-14 | §3.1.5.3.2 | Outer batch HTTP statuses **200 success / 400 malformed / 429 too many requests**. A successful outer batch **does not imply every embedded response has succeeded**. No source-defined blanket retry/backoff for every 429 in this selected section. |
| OCS-15 | §§3.1.5.6.3,3.1.5.9–10 | Meetings with custom extensions: `POST myOnlineMeetings` **`multipart/related`** (RFC2387), main `OnlineMeetingInput`, each related extension with `Content-Id` and `OnlineMeetingExtensionInput`. Extensions not understood by a joining client **are ignored**, not SIP header negotiation. |
| OCS-16 | §§3.1.5.7–8 | `onlineMeetingDefaultValues` and `onlineMeetingEligibleValues` are **per-user, should NOT be cached** for that user or other users. They inform UI/default/range validation and can change with policy. Static hardcoded conference-role permissions would misrepresent policy. |
| OCS-17 | §§3.1.5.11–13 | Invitation customization contains logo/help/legal URLs and footer; onlineMeetingPolicies is a **user-specific policy resource**, phoneDialInInformation includes dial-in regions/languages/telephone number. These URLs may point outside root origin; security trust assessment, no blindly trusting HTML or arbitrary redirect. |
| OCS-18 | §2.2.7.1 | HTTP failure body can be `ErrorType` with **code, message, subcode, validation parameter list, optional link and debugInfo**. It does not guarantee all failures have this body and cannot be confused with `ms-diagnostics-public` in SIP [MS-OCER]. |
| OCS-19 | §§3.1.1.5.1–5;3.1.5.5 | **S**: concurrent organizer editors can race between GET etag and PUT. `If-Match` is conditional guard; unknown fields must be preserved, and a failed conditional update should refresh before retry rather than silently overwrite others. **Exact status/automatic retry remains U**. |
| OCS-20 | §1.3; MS-OCSPROT §2.5.17; MS-CONFPRO | **S boundary:** scheduled-meeting resource available is not an admitted, running Focus conference. Conference resource/URL, scheduled attendees and lobby policy must be reconciled at join with C3P/Focus membership, AV MCU/ICE/SRTP, not reduced to one Boolean `meetingReady`. |

## 2. Resource state/operation diagram (S, based on source-specific operations)

~~~mermaid
flowchart TD
 A[User sign-in credentials and MS-OCDISCWS autodiscovery] --> B[Application POST: endpointId culture userAgent]
 B --> C[GET application hypermedia links]
 C --> D[GET default and eligible values, user-specific no caching]
 D --> E[POST myOnlineMeetings OnlineMeetingInput]
 E --> F[Scheduled resource: joinUrl etag conferenceId policy]
 F --> G{Organizer operation}
 G -->|GET| H[Read resource and current etag]
 H --> I[PUT full resource preserving unknown fields, optional If-Match]
 I --> F
 G -->|Delete| J[DELETE 204/404/410, independent of live SIP dialogs]
 G -->|Invite or Join| K[Separate SIP Focus conference admission + MCU media]
 E --> L{HTTP resource error}
 L --> M[Error code/subcode/parameters if provided, distinguish from SIP]
 C --> N[Optional multipart/batching independent part responses]
 N --> O{Outer 200/400/429}
 O --> P[Interpret each embedded HTTP response separately]
~~~

## 3. Source/version and security issues NOT RESOLVED

- **OCS-U01** Source calls Web/mobile app typical (§1.3); **whether any given classic SfB Windows Full Client build directly speaks these REST methods is UNVERIFIED**. Mark as supporting optional online-meeting management capability, not mandatory login dependency.
- **OCS-U02** Source permits HTTP (§2.1), but user credentials/Web Tickets and modification requests need correct deployment identity/authentication, redirect policy and TLS trust profile. Reconcile [MS-OCDISCWS] rev10 and [MS-OCAUTHWS] rev11, not unauthenticated HTTP fallback.
- **OCS-U03** Full 93-page XML schemas, extension interfaces, version-specific `etag`/If-Match failure and batch partial-result behavior beyond selected sections not completely audited. Do **not** assume HTTP 412 or a fixed 429 retry interval absent exact source.
- **OCS-U04** Server-side resource deletion, delegated organizers, reassigned meeting, and joinUrl redirection vs Focus membership/active media retirement need cross-spec policy proof. Do not imply deleting schedule always drops active RTP.
- **OCS-U05** RFC2616/2818 are historic originals; replacing their syntax/privacy/connection assumptions by Microsoft Graph `PATCH /onlineMeetings` or modern UCWA Teams semantics changes wire protocol. Exact original edition is normative here.
- **OCS-U06** Historic Windows client build target and product notes needed before including OCSMP as “full-client mandatory.” All transitive references and historical editions not closed.

**Research status INCOMPLETE; implementation plan gate CLOSED. No new protocol code, connection to genuine servers, interoperability test, merge or release.**
