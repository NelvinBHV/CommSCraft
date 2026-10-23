# MS-OCSPROT 9.0 — 21 use-case coverage and client/server state dependency reconstruction

**Research date 2026-10-23. Edition: [MS-OCSPROT] v20250520, revision 9.0, 20 May 2025; PDF original SHA-256 `b0380ba5606072eb071bd67ffa7561e12cd7e55ec9dbd23a6553558438524ff2`.** Official Microsoft PDF https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OCSPROT/%5BMS-OCSPROT%5D.pdf ; official Microsoft [conference use case](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/aa2e218d-9e1a-40d1-9ff4-58c2b7cd7aad). **Primary-original §§1.2,2.1–2.9,2.5.1–2.5.21,3.1–3.6,4–4.1 and §5** assessed; original PDF pp28–29 rendered and visually checked against local text (the GitHub source edition, not a verified 2026 runtime capture). Refs: [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) (2002) §§8,10,12,17,22; [RFC3265](https://www.rfc-editor.org/rfc/rfc3265) historic 2002 events (superseded by [RFC6665](https://www.rfc-editor.org/rfc/rfc6665) in 2012); [RFC3550](https://www.rfc-editor.org/rfc/rfc3550) (2003), [RFC3263](https://www.rfc-editor.org/rfc/rfc3263) (2002). **Scope: architectural overview + explicit individual use cases, NOT all RFC or product implementation compliance.**

**Evidence semantics:** **P** = Microsoft primary overview exact clause; **D** = detailed Microsoft protocol or original RFC to be separately validated; **S** = cross-source logical state decomposition; **U** = unknown client-build/server-profile or incomplete normative interpretation. The use cases in an **overview** are example/reference flows; they are **not** byte-accurate universal packet captures, mandatory fixed ordering or a complete enumeration of every classic Windows Full Client capability. Client MUST in a lower-level spec must not be inferred from an overview *example step*.

## 1. All 21 officially numbered overview use cases mapped to protocol families

| ID | Overview § | UI function and meaningful protocol steps | Overview-explicit direct dependencies | Scope/gates and unresolved issue |
|---|---|---|---|---|
| UC01 | **2.5.1** | Discover via **SIP-URI domain DNS SRV**, select pool FQDN/port, establish TCP, **optional TLS** with server cert validation, optionally request SIP compression only if TLS + policy | MS-CONMGMT, MS-SIPCOMP | **Connected, not authenticated**. Plain example's optional TLS **is not an endorsement** of plaintext for SRTP inline key exchange; policy/certificate identity handled lower level |
| UC02 | **2.5.2** | Initial REGISTER advertises GRUU, enhanced presence resource-list/XML and keepalive capabilities; server SIP **401/407** challenges; client **second REGISTER** with Kerberos/NTLM token; server verifies and returns endpoint GRUU/capabilities and optional server authenticity proof | MS-CONMGMT, MS-PRES, MS-SIPAE, MS-SIPRE, MS-SIPREGE | **REGISTER 200 / binding** ≠ in-band policies, roster or complete post-sign-in state. Third listed **TLS-DSK** from §2.2.2.1 not illustrated by this Kerberos/NTLM example; protocol §3 actor variant needed |
| UC03 | **2.5.3** | Bootstrap **SUBSCRIBE** in-band provisioning, contacts/groups, own presence; **SERVICE** location profile, MCU list and optional A/V relay tokens; **SERVICE** publish presence; batch contact-presence SUBSCRIBE | MS-PRES, MS-SIPREGE, MS-SIPRE, MS-AVEDGEA | **Sixteen example steps may occur in any order** per closing note. A/V Edge token request only with Edge/ICE guards; location profile example assumes EnhancedEmergencyServices and LocationRequired |
| UC04 | **2.5.4** | HTTPS-SOAP `GetLocations` using network identifiers; cache civic address; if multiple select based on user's input | MS-E911WS | Needs URL/network provisioning and server policy; **§4.1 <16>** OCS2007/2007R2 products do not support feature. Not logically required for every successful IM-only sign-in |
| UC05 | **2.5.5** | Overall sign-in comprises **UC01→UC02→UC03→UC04** reference workflows, not one new standalone SIP command | MS-AVEDGEA, MS-CONMGMT, MS-PRES, MS-SIPCOMP, MS-SIPAE, MS-SIPRE, MS-SIPREGE, MS-E911WS | §2.5.4/location condition and Edge topology apply; display signed-in vs every optional feature ready separately |
| UC06 | **2.5.6** | SIP `SERVICE` publish presence, home server `200` updated version, server `NOTIFY` to subscribers | MS-PRES | Category publication optimistic versions, presence containers/privacy and network-available watchers distinct |
| UC07 | **2.5.7** | ABS full/delta server address book files, HTTPS/file transfer to URL discovered in provisioning, subsequent delta refresh | MS-ABS | ABS download/refresh **separate from** user contact roster SUBSCRIBE; HTTP cache and delta error conditions lower protocol |
| UC08 | **2.5.8** | HTTPS-SOAP distribution list expansion request/response | MS-DLX | User directory ACL and Kerberos/NTLM/SSL trust from §2.9.1.2; not SIP `MESSAGE` payload |
| UC09 | **2.5.9** | SIP `INVITE` to IM dialog, `200 OK` acceptance and **ACK** (explicitly explained though omitted from figure), then SIP `MESSAGE`/response `200 OK` | MS-SIPRE | IM dialog confirmed vs individual message accepted, delivery-to-user and typing state separate; user agent must not skip SIP ACK |
| UC10 | **2.5.10** | SIP `SERVICE` add contact, server `200`; changes watcher/subscriber list at peer, sends new presence with `NOTIFY/BENOTIFY` | MS-PRES | **Last two notifications can occur in different order**, therefore no linear waiting chain; contact version and presence container ACL independent |
| UC11 | **2.5.11** | Fork SIP `INVITE` to multiple registered endpoints, elect branch based on published presence/capability, winning `200`, other branches `CANCEL` | MS-PRES, MS-SIPRE | Branch dialog IDs/To-tags, race between 200/CANCEL and independently registered endpoint bindings require exact RFC3261/MS-SIPRE, **no global single conversation state** |
| UC12 | **2.5.12** | A/V Edge candidate/relay allocation **where configured**, SIP `INVITE`+SDP offer/provisional answer, ICE checks and responses, accepted call, bidirectional RTP/RTCP, later SDP selected-ports update | MS-AVEDGEA, MS-ICE, MS-ICE2, MS-ICE2BWM, MS-OCPSTN, MS-RTP, MS-RTPRADEX, MS-SDPEXT, MS-SIPRE, MS-SRTP, MS-SSRTP, MS-TURN, MS-TURNBWM | **20-step example** assumes Edge and ICE support; two-way media can begin **before final updated SDP exchange** in diagram; do not assume ICE consent/SRTP transform conflict resolved |
| UC13 | **2.5.13** | Add video to ongoing voice call using new SDP offer/answer plus media/codec negotiation | MS-AVEDGEA, MS-H264PF, MS-ICE, MS-ICE2, MS-ICE2BWM, MS-OCPSTN, MS-RTP, MS-RTPRADEX, MS-RTVPF, MS-SDPEXT, MS-SIPRE, MS-SRTP, MS-SSRTP, MS-TURN, MS-TURNBWM | RTVideo vs H.264 chosen by actual advertised profile; no inference either codec is always supported; distinguish voice-dialog lifetime from added m-line |
| UC14 | **2.5.14** | Answer an inbound voice call with SIP routing and media checks | MS-AVEDGEA, MS-ICE, MS-ICE2, MS-ICE2BWM, MS-OCPSTN, MS-RTP, MS-RTPRADEX, MS-SDPEXT, MS-SIPRE, MS-SRTP, MS-SSRTP, MS-TURN, MS-TURNBWM | Answered SIP ≠ nominated media/consent; legacy/new ICE deployments differ; complete reject/timeout branches lower docs |
| UC15 | **2.5.15** | SIP `BYE`, proxy forwards, remote accepts, response forwarded to initiating endpoint | MS-SIPRE | User hangup != RTCP BYE, TURN release, RTP timeout or final QoE report; independent cleanup timing |
| UC16 | **2.5.16** | **Each endpoint independently** sends post-call QoE via provisioned SIP server and receives accept response | MS-QoE, MS-SIPRE | **Overview postcondition claims QoE stored in database**, but detailed MS-QoE rev12 §3.2 says **SIP 202 Accepted does not imply processing/storage**. Distinguish normative guarantee from successful-path desired outcome |
| UC17 | **2.5.17** | C3P conference `addConference` → Focus `INVITE addUser` → conference SUBSCRIBE/roster/MCU URI → optional INFO getConference (`202`, final INFO response) → A/V MCU modality `INVITE`/SDP/ICE/SRTP/RTP | MS-CONFPRO, MS-CONFAV, MS-CONFBAS, MS-ICE, MS-ICE2, MS-ICE2BWM, MS-RTP, MS-RTPRADEX, MS-SDPEXT, MS-SIPRE, MS-SRTP, MS-SSRTP | **§2.5.17 explicitly states asynchronous subflows can be reordered**; INFO SIP 202 ≠ C3P completion; Edge/TURN allocation **omitted for clarity** and must be added as conditional |
| UC18 | **2.5.18** | SIP `SUBSCRIBE` conference events, `200` roster, subsequent roster changes via `BENOTIFY` | MS-CONFBAS, MS-SIPRE | SIP accept vs full roster, lobby-admitted endpoint vs others, reconciling conference version and subscription ordering |
| UC19 | **2.5.19** | Conference-member, roster-subscribed prerequisite; ASMCU `INVITE` SDP application sharing, ICE, state-change notification, re-offer/answer, **media-connected** notification, RDP-over-RTP payload | MS-CONFAS, MS-CONFBAS, MS-ICE2, MS-ICE2BWM, MS-RTASPF, MS-RTP, MS-RTPRADEX, MS-SRTP, MS-SSRTP, MS-SIPRE, MS-SDPEXT | Signaling `200` ≠ RDP established; edge port allocation **omitted**; policy sharer/viewer and RDP virtual channel need separate gate; media-connected notification after joining |
| UC20 | **2.5.20** | In-conference whiteboard, SIP `INFO addUser` gets **202**, later response via **server-originated SIP INFO** and client `200`, then separately authenticate and establish **PSOM shared-object** connection | MS-CONFBAS, MS-PSOM | SIP 202 is not PSOM authenticated/data ready; older OCS editions excluded by **§4.1 note <31>** |
| UC21 | **2.5.21** | SIP `INVITE`/`ACK` Group Chat dialog, `INFO cmd:getserverinfo`, `INFO cmd:bjoin`, history `rpl:bccontext`, `INFO grpchat`; each has 200 acknowledgment and separate reply | MS-XCCOSIP | **17-step** protocol example. `INFO 200` means request accepted, not that `rpl:bjoin` or room message is completed; notes **§4.1 <32>** OCS 2007/07R2 and Lync 2010 excluded |

**Coverage definition:** Every row is **a mapped MS-OCSPROT overview use case**, **not** a completed detailed specification. The Microsoft overview expressly notes in §1.2 that links to the *latest* Open Specification may lead to **different section numbers because document release dates differ**. Use pinned versions and the original footer/errata; do not silently borrow 2026 latest clauses for 2015 classic client.

## 2. Architecture roles and trust boundaries, not a monolithic transport

| Plane | Client-owned or server-owned state | Original sources | Strict boundary |
|---|---|---|---|
| **Account transport** | DNS SRV discovery, TCP connection, server-certificate identity, optional negotiated SIPCOMP | MS-OCSPROT §§2.5.1,2.9; MS-CONMGMT; MS-SIPCOMP | A socket connected is not an authenticated SIP principal. Example permits optional TLS; the security posture for media inline keys requires separate rules |
| **SIP principal/endpoint** | REGISTER 401/407 challenge response, MS-SIPAE security context, GRUU/contact binding, expiry and keepalive | §2.5.2; MS-SIPAE/SIPREGE/SIPRE; RFC3261 §§10,22 | Registration 200 authenticates/registers endpoint, not persistent application presence or MCU admission |
| **Provisioned user context** | In-band subscriptions for policies and roster, own presence, service location, MCU list, Edge credential tokens | §2.5.3; MS-PRES/SIPREGE/AVEDGEA | Example calls are **not serialized in mandatory order**. Their progress/failure is separate |
| **Directory/identity services** | ABS full/delta, DL expansion over HTTPS SOAP, active directory source and permissions | §§2.5.7–8,2.3.2; MS-ABS/MS-DLX | Presence contact group list ≠ global directory file ≠ expanded distribution list |
| **Dialog and federation** | IM dialog, MESSAGE acceptance, one or more MPOP forked INVITE branches, SIP BYE | §§2.5.9,2.5.11,2.5.15; RFC3261/MS-SIPRE | MPOP SIP 200/CANCEL race must not assume one in-flight branch |
| **Conference Focus** | Factory addConference, Focus addUser, roster and lobby/subscriber state, C3P INFO command | §§2.5.17–18; MS-CONFPRO/CONFBAS | SIP 202 ≠ C3P result, conference join ≠ A/V or sharing MCU media readiness |
| **Media transport** | Edge credentials, TURN permission, ICE pairs, SDES keys, SRTP/SSRTP crypto/replay | §§2.5.12–14,19,2.9.1; MS-ICE2/AVEDGEA/TURN/SRTP | SIP 200 ≠ media connected; proxy authentication ≠ media MAC or relay permission |
| **Conference media** | A/V/ASMCU dialogs, RTP/RTCP, H264/RTVideo codec, PSOM separate data channel | §§2.5.17,19–20; MS-CONFAV/CONFAS/RTASPF/PSOM | Sharing `media-state=joining` vs RDP ready vs Focus admitted distinct |
| **Telemetry** | QoE post-call report, OCER diagnostic report, conference/mixer quality statistics | §§2.5.16,2.7; MS-QoE/OCER | SERVICE response ≠ archival durability or recovery of original failed operation |

## 3. Causal state diagrams (S; no claim of one fixed Windows-client wire trace)

### 3.1 Discover → Register → Provision → signed in

~~~mermaid
flowchart TD
 A[Client starts with user SIP URI and policy] --> B[MS-CONMGMT DNS SRV discovery]
 B --> C[TCP connection and server identity TLS if profile/policy]
 C --> D{Transport usable?}
 D -->|No| E[Reconnect or fail according to CONMGMT, not yet registered]
 D -->|Yes| F[REGISTER feature advertisement]
 F --> G[401 or 407 supported challenge]
 G --> H[MS-SIPAE Kerberos/NTLM/TLS-DSK as negotiated and permitted]
 H --> I{Binding accepted and GRUU available?}
 I -->|No| J[Registration/authentication-specific failure]
 I -->|Yes| K[Independent subscriptions, policies, contacts, presence]
 K --> L[Conditional E911, MCU discovery and Edge relay tokens]
 L --> M[User context ready per feature, with subscriptions continuing]
~~~

**P/S boundary:** 2.5.2 illustrates Kerberos/NTLM 401/407; §2.2.2.1 additionally lists TLS-DSK. RFC3261 §10.2.8 specifies **423/Min-Expires** for interval too brief but *not in this four-step illustrative chart*. A 200 REGISTER is not proof E911 configured, contact batch fully synced, or successful future media.

### 3.2 Multi-endpoint INVITE versus per-endpoint state

~~~mermaid
sequenceDiagram
 participant A as Caller
 participant P as SIP proxy
 participant B1 as Recipient endpoint 1
 participant B2 as Recipient endpoint 2
 A->>P: INVITE B
 P->>B1: Forked INVITE branch 1
 P->>B2: Forked INVITE branch 2
 B1-->>P: 200 OK as selected endpoint
 P->>B2: CANCEL outstanding branch
 P-->>A: 200 OK selected dialog
 Note over A,B2: §2.5.11 schematic; RFC3261 race/branch transaction exact behavior in MS-SIPRE and RFC
~~~

**U:** §2.5.11 does not fully specify late 200 after CANCEL, ACK/BYE branch disposal, identical user on multiple PCs and authentication token binding across distinct GRUUs. Don't invent a global “last registered device wins” invariant.

### 3.3 Conference admission/MCU independence

~~~mermaid
flowchart TD
 A[Signed in and conferencing available] --> B[Focus Factory addConference, C3P result]
 B --> C[Focus INVITE addUser]
 C --> D[Conference SUBSCRIBE roster and MCU URI]
 D --> E{Lobby / role / authorization}
 E -->|Pending| F[Lobby state with limited roster]
 E -->|Admitted| G[Independent AV MCU or ASMCU INVITE/SDP]
 G --> H[ICE pair nomination and relay permissions]
 H --> I[SRTP/SSRTP security and RTP media]
 G --> J[PSOM data MCU admission for whiteboard]
 I --> K[AV packets or secured RTASPF RDP connection]
 J --> L[Authenticated whiteboard shared-object data]
~~~

**Source notes:** §2.5.17 expressly allows asynchronous branch reorder; §2.5.19 conference-membership and event-subscription prerequisites; §2.5.20 SIP INFO 202 precedes later PSOM authentication. Server-side dialog and media state must not be collapsed into single Boolean.

## 4. Normative/version conflicts found or sharpened by whole-overview cross-check

| ID | Clauses | Evidence fact, conflict or unsupported inference | Required resolution |
|---|---|---|---|
| OV-X01 | MS-OCSPROT §2.5.1 + §2.9.1.5 | TLS **optional in overview discovery example**, while security section recommends SIP **TLS** to exchange SDP media encryption information and notes SDP extension does not itself encrypt those credentials | Distinguish historical deployment choices, threat model, mandatory TLS enterprise policy and server cert validation; **do not downgrade SRTP key confidentiality** from word “optional” |
| OV-X02 | §2.5.2 vs §2.2.2.1 | Example presents only **Kerberos/NTLM** after REGISTER 401/407, but protocol overview explicitly includes **TLS-DSK** in SIPAE scheme list | Each real authentication branch must be resolved from MS-SIPAE 11.0, MS-CONMGMT and deployment topology; example is not exhaustive |
| OV-X03 | §2.5.3 closing note | Sixteen bootstrap SIP exchanges *may occur in any order* | Build partial-order state graph, not fictional globally ordered 16-action handshake |
| OV-X04 | §2.5.10 | Server's subscriber-list notification vs contact presence notification may occur in either order | Contact roster state, subscription and presence notification are separate |
| OV-X05 | §2.5.11 | MPOP invitation is forked and server cancels loser after one endpoint accepts | Need RFC3261 late branch 2xx/CANCEL handling and exact MS-SIPRE GRUU/branch keys; can't deduce one user dialog for all endpoints |
| OV-X06 | §2.5.12 vs §§2.5.17,2.5.19 | Media port and TURN steps included for voice example but **intentionally omitted** from conference/audio/sharing example | No claim that conferences/desktop sharing bypass A/V Edge token/TURN permission |
| OV-X07 | §2.5.16 postcondition vs MS-QoE rev12 §3.2 | Overview successful scenario postcondition describes quality reports “stored in database” while **Monitoring Agent 202 indicates accepted, not necessarily processed** in detailed QoE spec | Treat overview as end-state scenario, 202 only as network acknowledgement; do not claim durable storage guarantee |
| OV-X08 | §2.5.17 preface and steps 8–10 | Independent asynchronous Focus, roster, C3P and MCU subflows; `INFO` commands acknowledge **202** before later C3P result | Never infer C3P operation succeeded because SIP 202 or because AV SDP 200 |
| OV-X09 | §2.5.19 | Sharing `INVITE 200`, ICE response, roster `media-state=joining`, updated SDP and connected-media notification all occur as distinct events | Only authenticated RTP/RTASPF/RDP full path plus permissions establishes usable sharing |
| OV-X10 | §2.5.20 | PSOM whiteboard requires *second channel authentication* after SIP INFO addUser C3P response | No whiteboard readiness from Focus admission or SIP 202 |
| OV-X11 | §2.5.21 | SIP INFO 200 on `cmd:bjoin` precedes server `rpl:bjoin` | Acknowledgement ≠ successful persistent-chat room join; client must correlate result and room identities |
| OV-X12 | §2.6.1 + §4 | Server can reject by client **UserAgent minimum version**; listed applicability includes products from OCS2007 through SfB2019 and Server SE; **§4.1 notes <1>–<32>** carve out historic unsupported cases | Applicability is *not* proof 2015 Windows Full Client supports every listed protocol, TLS1.3 or WebRTC |
| OV-X13 | §2.7 + §2.8 | Overview only gives **categories of errors** and says **no special system coherency requirements** | Neither assertion removes detailed MUST for MS-SIPREGE binding expiration, MS-PRES versioning, ICE consent or per-service timeout |
| OV-X14 | §1.2 | Microsoft links point to **latest** version's section; original document versions updated asynchronously so section labels can drift | Pin original PDF date+revision+SHA and independently confirm referenced exact section/errata before normative compatibility claims |
| OV-X15 | §§2.2,2.5 vs all published MS refs | Functional overview enumerates **21 use cases** but not exhaustive Windows UI features and custom client protocols | Remaining user-facing features require separate functional inventory; mark missing flows U, not absent from product |

## 5. Actual 2025 edition supports a broad *product matrix*, not automatic homogeneous client compatibility

MS-OCSPROT §4 includes OCS 2007, OCS 2007 R2, Lync 2010, Lync Client 2013/Skype for Business, SfB 2016, SfB 2019, Server 2015/2019/Subscription Edition. **§4.1 explicitly excludes** early clients from multiple ICE2/BWM, E911, whiteboard/chat room cases (footnotes **<10>–<32>**), and says there are no overview-level variations beyond the detailed protocol specifications.

**Critical applicability:** Term “Windows Full Client” must ultimately be resolved to the concrete **Skype for Business 2015/2016/2019 installed Windows client build**, 32/64-bit/Office branch and associated server/Edge/MCU topology. This source edition cannot establish which compiled client registers `Supported` tags or uses a given RTP codec.

**Outstanding source closure (not "done"):** Reconcile the 21 examples against all **46** Microsoft document IDs referenced directly by MS-OCSPROT, existing 150-physical-original dependency graph, normative RFC snapshots and edition discrepancies. MS-PSOM, MS-XCCOSIP, MS-OCSMP and exact MS-E911WS errors need complete original-section and message/state audits; no implementation plan is authorized. The full 150-original corpus audit lists **191 referenced Microsoft IDs missing locally, including 68 referenced from normative sections** across its broad corpus, distinct from this document’s directly listed protocol set. Do not treat a 46-ID overview citation list as an exhaustive transitive closure.

**Research status INCOMPLETE. No code, server connections/interoperability tests, merges to main or releases.**
