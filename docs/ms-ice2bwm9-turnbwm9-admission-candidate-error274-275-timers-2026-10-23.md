# MS-ICE2BWM 9.0 / MS-TURNBWM 9.0 — bandwidth admission, candidate filtering, commit, refresh and STUN 274/275

**Original-doc research 2026-10-23; coverage selected clauses, not complete Windows Full Client readiness.** Primary Microsoft **[MS-ICE2BWM] ICE 2.0 Bandwidth Management Extensions**, **rev9.0 v20250520, 2025-05-20**, SHA-256 **a63bf4dd42c38f22da65eaf418746bb1f3dc34a6310db1df94ebe4d9781c02de**, [official PDF](https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-ICE2BWM/%5BMS-ICE2BWM%5D.pdf) §§2.2.1–2.2.2,3.1.4.1–3.1.6.2; original PDF p14 inspected visually. **[MS-TURNBWM] TURN Bandwidth Management Extensions**, **rev9.0 v20250520, 2025-05-20**, SHA-256 **6beb8a1e1bc8b058c39aec0e724f7ef64353a824f6c06bdfed3209a79ff5e101**, [official PDF](https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-TURNBWM/%5BMS-TURNBWM%5D.pdf) §§2.2.1–2.2.14,3.2.4–3.2.6, selected §4 examples; original PDF p14 inspected visually. **Original dependencies** Microsoft [MS-ICE2] rev19 for base ICE state, [MS-TURN] rev21 for TURN Allocate/128-bit transaction header, [MS-SDPEXT] rev16 for SIP/SDP media bandwidth. [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) SIP, [RFC4566](https://www.rfc-editor.org/rfc/rfc4566) SDP and **historical IETFDRAFT-ICENAT-19 / IETFDRAFT-STUN-02 / IETFDRAFT-TURN-08** cited by sources. Newer [RFC8445](https://www.rfc-editor.org/rfc/rfc8445) / [RFC8656](https://www.rfc-editor.org/rfc/rfc8656) are *edition comparisons* only, not evidence of legacy wire substitution. **P** original direct source (role/condition), **R** IETF source, **S** cross-spec synthesis, **U** unverified product/configuration.

## 1. Protocol objects and admission policy wire format

| ID | Microsoft source/section | Normative, role-qualified behavior |
|---|---|---|
| BWM-01 | MS-TURNBWM §2.2.1 | **Bandwidth Admission Control** uses TURN **Allocate request/response** extension TLV type **0x8056**, length **4 bytes**, 2-byte reserved zero and 2-byte action: **0x0000 Check**, **0x0001 Commit**, **0x0002 Update**. Not SIP INFO or RTCP packet loss feedback. |
| BWM-02 | §§2.2.2,3.2.5.2 | **Reservation ID** TLV **0x8057**, **16 bytes** assigned on Commit success, echoed in subsequent Commit/Update. If bandwidth policy server **not managing resource**, all-zero ID **MUST** be returned. With null ID, MS-ICE2BWM §3.1.5.1.2 says **MUST NOT send further BWM messages**; TURNBWM adds close TURN session when not used to transport other traffic. |
| BWM-03 | MS-TURNBWM §2.2.3 | Bandwidth Amount TLV **0x8058**, **16 bytes**: four 32-bit quantities (min/max send, min/max receive), **kilobits per second**. Required in Check and Commit, and when Update changes reservation. Check expresses range; Commit expresses expected media amount and server **MUST commit maximum indicated**. |
| BWM-04 | MS-TURNBWM §2.2.3 | Update can increase/decrease/keep reservation; server **MUST deny an increase it cannot honor**. **Zero bandwidth Update cancels reservation**, server releases resources and **MUST disconnect** when TURN transport not used for media relay. This is policy reservation lifetime, not an RTP bitrate adaptation proof. |
| BWM-05 | MS-TURNBWM §§2.2.12–2.2.13 | Optional SIP Dialog ID **0x8061** and SIP Call ID **0x8062** TLVs, each **<=256 bytes**, correlate reservation to SIP signaling for **logging**. Not authorization that a claimed Call-ID controls arbitrary other's reservation; TURN identity/authentication remains separate. |
| BWM-06 | MS-TURNBWM §2.2.14 | Location Profile TLV **0x8068**, length 4 bytes, **MUST be sent on Check and Commit**; Peer-Location and Self-Location 0 unknown,1 Internet,2 intranet; Federation 0 none,1 enterprise,2 public cloud; reserved zero on send. This is bandwidth-site classification, **not E911 civic location**. |
| BWM-07 | MS-TURNBWM §§2.2.4–2.2.11 | Site-address and site-address-response TLVs relate **local host, local relay, remote host and remote relay** to allowed bandwidth; some return validity flag and min/max send/receive. Server profile can disable a **site** rather than one particular UDP/IP candidate pair. |
| BWM-08 | MS-ICE2BWM §2.2.1 | Client uses **MS-TURNBWM** transport/actions to ask bandwidth policy server. Same historical MS-TURN legacy transaction/attribute framing as [MS-TURN] rev21, **not** a new RFC8656 Allocate format. |
| BWM-09 | MS-ICE2BWM §2.2.2 | New STUN error codes **274 Disable Candidate** and **275 Disable Candidate Pair**, sent as **STUN Binding error responses**; not SIP status 274/275 or TURN Allocate error from quota service. |

## 2. When admission runs relative to ICE2: disallowed path is never offered/checked

| ID | Source | State obligation |
|---|---|---|
| BWM-10 | MS-ICE2BWM §3.1.4.1 | For **bandwidth-management-aware caller with peer-site information**, request admission **during candidate gathering**. If all possible paths prohibited, **MUST fail media setup and MUST NOT send initial SDP offer**. A rejected route is not an ICE connectivity failure that needs more STUN retries. |
| BWM-11 | MS-ICE2BWM §3.1.4.1 | Policy-denied **local relayed candidate MUST NOT be included in SDP offer**; do not advertise candidate and hope remote side fails later. |
| BWM-12 | MS-ICE2BWM §3.1.4.8.1.1 | Check Local Site Address from local host candidate, Remote Site Address from peer host candidate, optionally peer relay address; when IPv4+IPv6 available **IPv4 MUST populate the BWM site TLV**. This **does not forbid IPv6 ICE candidates universally**. |
| BWM-13 | MS-ICE2BWM §3.1.4.8.1.1 | If both TCP and UDP bandwidth-policy servers configured, **MUST send checks to both**, **first valid response MUST be used** and later replies **MUST ignore**. A later more generous result cannot silently override the first valid policy. |
| BWM-14 | MS-ICE2BWM §§3.1.4.8.2.1,3.1.5.1.1 | Once policy marks local/remote site invalid, all candidate pairs using it are **invalidated**, and ICE connectivity checks **MUST NOT be sent** for such pairs. Responses with valid flag set carry allowed send/receive bandwidth per site; **valid ICE pairing alone cannot overrule site policy**. |
| BWM-15 | MS-ICE2BWM §3.1.4.8.2.2 | After successful ICE connectivity/nomination, **Commit** describes *selected* local/remote addresses, using local/remote **relay site TLVs** if selected candidate relayed and host address as related site. Distinguish site addresses from actual relayed transport endpoint. |
| BWM-16 | MS-ICE2BWM §§3.1.4.8.2.3,3.1.5.1.2 | After authenticated/valid Commit response with nonzero reservation ID, endpoint **MUST carry ID into updates**; on all-zero ID **MUST NOT send further admission messages**. An ICE nominated path can exist without a tracked reservation when policy server explicitly declares none. |

## 3. Error handling, refresh clocks and media survivability

| ID | Source | Failure and time domain |
|---|---|---|
| BWM-17 | MS-ICE2BWM §3.1.5.2.1 | When remote STUN Binding comes from a **policy-restricted remote candidate**, receiver **MUST send 274 Disable Candidate**. When it targets a restricted **local candidate** but remote is unrestricted, receiver **MUST send 275 Disable Candidate Pair**. Microsoft CANDIDATE-IDENTIFIER differentiates remote candidate identity. |
| BWM-18 | §3.1.5.2.2 | On received **valid 274**, ICE endpoint **MUST mark all pairs for that local candidate FAILED**. On valid **275**, **MUST mark only affected candidate pair FAILED**. Wrong scope risks either leaking disallowed traffic or incorrectly killing all media options. |
| BWM-19 | MS-ICE2BWM §3.1.6.1 | **ICE Bandwidth Commit Timer MUST fire every 500 ms** until Commit response; timer initiates request per BWM Commit procedure. Distinct from MS-TURN ordinary Allocate/SetActiveDest 650-ms retransmission guidance and ICE2 consent 5-second period. |
| BWM-20 | MS-ICE2BWM §3.1.6.2 | For unchanged bandwidth after Commit, **Update Timer MUST run every <=19 seconds**. If Update changes amount, MUST run **every 1 second** until response and then return to <=19 seconds. This is **reservation refresh**, not SRTP key rotation, ICE consent or SIP registration binding expiry. |
| BWM-21 | MS-TURNBWM §3.2.5.2 | Valid Commit response **MUST contain both 16-byte Reservation ID and Bandwidth Amount**. Null ID means policy server does not track allocation; if separate TURN session not used for RTP relay, disconnect according to source conditional rule. |
| BWM-22 | MS-TURNBWM §§2.2.1–2.2.3 / MS-ICE2BWM §3.1.6 | **Check → Commit → Update** are policy state transitions. A session may have valid ICE nominated pair and RTCP packet flow while **Bandwidth Reservation Update** fails; the specific admission failure and rollback policy must be sourced, not guessed as SIP BYE or automatic ICE restart. |

## 4. Combined policy/media state model (S)

~~~mermaid
flowchart TD
  A[MS-SIP SDP media request and client site policy capability] --> B[Gather ICE2 candidates and discover peer site]
  B --> C[MS-TURNBWM Allocate Check for range and site addresses]
  C --> D{Allowed paths?}
  D -->|None| E[Fail before SDP initial offer]
  D -->|Some| F[Suppress disallowed relayed and host candidate pairs]
  F --> G[Initial SDP offer; ICE2 checks on remaining candidates]
  G --> H{Candidate-pair approved and nominated?}
  H -->|No| I[ICE path/media failure, independent of policy]
  H -->|Yes| J[Commit selected local/remote site and relay addresses]
  J --> K{Reservation ID from server}
  K -->|Nonzero 16-byte| L[Update every <=19s, or every 1s after bandwidth change]
  K -->|All zero| M[No further BWM messages, release dedicated control TURN session]
  L --> N{Policy changes or STUN 274/275?}
  N -->|274| O[Disable all pairs of candidate]
  N -->|275| P[Disable matching single pair]
  N -->|No| Q[Media consent and SRTP packet integrity separate]
~~~

**S security boundary:** Candidate policy and QoS reservation are **not proof** user has permission to join conference MCU, call PSTN number, or bypass emergency policy. An authenticated SIP dialog is not a bandwidth reservation, and failure of telemetry does not release reserved capacity unless source-defined admission procedure does.

## 5. Compatibility and audit blockers

1. **BWM-X01:** MS-ICE2BWM explicitly versioned for ICE2, **not automatically every MS-ICE 11/client build**. Need client target edition, server topology and provisioning to mark whether enterprise call admission policy applies.
2. **BWM-X02:** Historical IETFDRAFT-TURN-08 TLVs and Microsoft TURN 128-bit transaction ID are **not** RFC8656 or RFC5389 standard-format control messages. Preserve exact framing in future evidence planning.
3. **BWM-X03:** Verify full byte-level TLV fields for all eight site/relay address request/response types, maximum sample figures and permissions from MS-TURNBWM §§2.2.4–11 and 49-page original; only selected values verified.
4. **BWM-X04:** **500ms Commit, 19s Update, 1s change Update** interact with TURN resource quotas, failures, ICE2 30s consent and SIP RFC4028 timers; no global media connected timeout.
5. **BWM-X05:** Reservation amount in **kbps**, dynamic actual codec bitrate/RTCP feedback, congestion loss and server-denied bandwidth increases are separate constraints. Real-time RTCP estimation **does not authorize** exceeding Call Admission Control limit.
6. **BWM-X06:** NULL/all-zero reservation ID changes client behavior; distinguish no management vs reservation failed. Do not invent retry when source says stop sending further messages.
7. **BWM-X07:** Original §3.1.4.8.2.2 requires host-site addresses even when media uses TURN relay. This is *policy topology data*, not license to send geographic civic E911 coordinates.
8. **BWM-X08:** Formal normative RFC reference edition and product notes must be reconciled before any future implementation plan; no real-server testing by assistant.

**Status: primary source clauses verified, full Windows Full Client documentation NOT COMPLETE. No protocol implementation, live-server traffic, interop tests, main merge or release.**
