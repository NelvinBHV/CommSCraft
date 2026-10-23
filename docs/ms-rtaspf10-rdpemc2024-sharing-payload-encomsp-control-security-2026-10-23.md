# RTP application-sharing payload and RDP multiparty virtual-channel evidence

**Original source audit 2026-10-23. NOT conformance-ready.** [MS-RTASPF] rev10.0 v20250520 May 20 2025, local original PDF SHA256 2f836fe6d7f0e9fc3dab09b5998ea4460f4b2496d3ff140bd6b703cae0c57eb2, inspected §§1.3–1.7, 2.1–2.2, 3.1–3.2 and example §4, PDF pp7–10 rendered. Official https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-RTASPF/%5BMS-RTASPF%5D.pdf . [MS-RDPEMC] original v20240423 (April 23 2024), owner archive PDF SHA256 17658ee28cef61d0ba11beb69be0e1fbe9e0b0f6aa7c146d5bc4100f64c0985b, inspected §§1.3–1.7,2.1–2.2.5,3.1.2–3.1.5,3.2.5–3.3.5.2.3, PDF pp10/12/17/28/29 rendered; official edition-specific URL and revision NOT verified. [MS-CONFAS] rev12 §§3.2.5.2,3.3.5.3–7, [MS-RDPBCGR] rev62.0 v20260309 cross-referenced only, [MS-RTP], [MS-SRTP] and [MS-ICE2], original [RFC3550](https://www.rfc-editor.org/rfc/rfc3550). **P** = original clause, **S** = cross-spec synthesis, **U** = unverified.

## I. RTASPF – transport/SRTP security and RTP wire behavior

| ID | Precise original | Scoped evidence |
|---|---|---|
| RTASP-01 | MS-RTASPF §§1.3–1.4 | RDP graphics/extension payload rides RTP over **TCP mandatory**. RDP internally describes message lengths and multiple RDP messages can be part of RTP-framed stream. Not generic standalone RDP TCP/3389 service. |
| RTASP-02 | §1.5,§3.1.5 | **RDP encryption level must be None**, **RDP Security Exchange PDU omitted** in this RTP profile; **§1.6 independently requires SRTP encryption** for transferred data. Removing inner RDP encryption **does not allow unprotected RTP on the network**. |
| RTASP-03 | §1.5 | RDP Bulk Data Compression **off** for viewer–MCU and **on** for sharer–MCU, governed by MS-RDPBCGR §3.1.8. This is NOT MS-SIPCOMP negotiated LZ77-8K dictionary on SIP TLS. |
| RTASP-04 | §1.7 | Described RDP protocol version 0x00080004. RTASPF itself has no payload-level version negotiation; SIP/SDP handles app-sharing capabilities at higher layer. |
| RTASP-05 | §2.2 | Maximum **1500 bytes total**, including link/network/transport/RTP headers and payload. Sender must split packets accordingly; violating bound tears down RTP connection. Do not assume 1500 RTP-payload bytes alone fit. |
| RTASP-06 | §§3.1.5,3.2.5 | **RTP payload type MUST be 127 (0x7F), marker bit MUST be 0**; SSRC/CSRC/sequence governed by MS-RTP and RFC3550. Receiver **must order/reassemble by RTP sequence** before RDP interpretation. |
| RTASP-07 | §§3.1.5,3.2.5 | RDP connection sequence **must omit** the RDPBCGR Security Exchange PDU (§2.2.1.10.1), a deliberate profile override protected by outer SRTP. |
| RTASP-08 | §§3.1.7,3.2.7 | On **detected packet loss** source states protocol **stops sending data**. No specified auto-retry/new-ICE timer here; complete MS-RTP loss detection and resumption semantics still unverified. |
| RTASP-09 | §§3.1–3.2 | Peer-to-peer has sharer+viewer; multiparty has sharer+multiple viewers via ASMCU. Peer-to-peer participant FriendlyName MUST be local SIP URI. No independent timers in §§3.1.2,3.2.2. |

## II. RDPEMC – virtual channel and viewer/remote-control permissions

| ID | Original clause | Scope |
|---|---|---|
| RDP-M01 | MS-RDPEMC §§1.3–1.5,2.1 | Multiparty control runs on established RDP static virtual channel **encomsp**. Channel must exist first, and termination stops extension communications. This is not SIP INFO. |
| RDP-M02 | §1.7 | No version negotiation of its own. Known messages don't require a reply, unknown types ignored; do not infer all new extensions accepted. |
| RDP-M03 | §2.2.1 | Each message starts 4-byte ORDER_HDR: 16-bit **Type** and 16-bit **Length**. Multiple extension PDUs may be embedded in one virtual-channel payload. Receiver validates message boundaries. |
| RDP-M04 | §§2.2.3–2.2.5 | Types include application/window created/removed/show (0x0002–0x0006), participant removed/created (0x0007–0x0008), change control level (0x0009), graphics paused/resumed (0x000A–0x000B), region update (0x000C), control change response (0x000D). |
| RDP-M05 | §3.1.5.1 | Unknown PDU type SHOULD be ignored; known type must use type-specific length validation. Truncated recognized PDUs can require connection termination under handlers. |
| RDP-M06 | §3.1.5.2 | UNICODE_STRING cchString is character count; receiver **MUST multiply by two bytes** and ensure remaining packet includes string plus subsequent fields. Not UTF-8 length. |
| RDP-M07 | §§1.3.1.1,3.1.5.3 | Application filtering tracks an application's present and later windows, explicit window filtering requires separately selected windows. AppId/WndId lists follow creation/removal events, not SIP conference participant IDs. |
| RDP-M08 | §§2.2.4.1,3.2.5.2.1 | Participant-Created includes ParticipantId/GroupId/Flags/FriendlyName; also updates changed control permissions. MS-CONFAS §3.2.5.2.1 fixes ASMCU Attendee-Connected PDU GroupId **3766552189**. |
| RDP-M09 | §§2.2.4.3–4,3.3.5.2.3 | Participant requests mouse/keyboard **control level**; **sharing manager SHOULD verify authorization** before granting and send a Participant-Created update if permitted. View-only participant asking control is not authorization. |
| RDP-M10 | §§2.2.3.6,3.3.5.1.1 | Show Window PDU references WndId; sharing manager SHOULD validate existence and entitlement. Truncated message SHOULD terminate connection. |
| RDP-M11 | §§1.3.1.3,2.2.5.1–2 | Graphics Stream Paused/Resumed allows suspending sensitive graphics (e.g. password prompts), **not** removing Focus conference membership. |
| RDP-M12 | §§3.1.2,3.2.2,3.3.2 | No timers specific to this extension. RDP transport/RTP/ICE/RTCP and SIP dialog timeouts independent. |

## III. Packet and security layering (synthesis, not a mandated one-path trace)

Conceptual chain: **Focus admitted → ASMCU sharer/viewer SDP offer-answer → ICE2 TCP / selected SRTP keys → MS-RTASPF RTP PT=127 with ordered RDP payload → MS-RDPBCGR RDP stream (no inner encryption/security exchange) → static encomsp channel → MS-RDPEMC typed application/window/participant/control PDUs**.

A successful SIP INVITE 200 is insufficient to call screen sharing ready. SIP authorization, role/policy, ICE consent, SRTP packet integrity, RDP channel and display-state readiness remain distinct. Removing internal RDP encryption makes the **unresolved MS-SRTP 2025 SHA1/SHA256 crypto-suite inconsistency** directly relevant to desktop privacy. Compromised group/window identifiers and control PDU names can affect user-visible permissions; authorization must be checked by the actual sharing manager.

## IV. Primary blockers remaining

- **RDP-X01:** Editions: RDPEMC 2024, RTASPF 2025, RDPBCGR 2026. Need historical RDPBCGR matching actual classic Windows/MCU product, not silently assume new 2026 RDP service behavior.
- **RDP-X02:** Full RDPBCGR initial connection/capabilities and negotiated security profile, encomsp channel error handling, and any additional RDP security layers are not fully audited.
- **RDP-X03:** Exactly how MS-RTP defines packet loss/restart after stop-sending is unresolved. No invented automatic retry timer or resumption.
- **RDP-X04:** TCP framing and RTP total 1500-byte bound with transport/link/IP v4/v6/relay overhead must be reconciled before interpreting acceptable payload sizes.
- **RDP-X05:** Complete PDU fields/endianness, PDU-length validation, input entitlement and remote-control lifecycle across conference roles remain open.
- **RDP-X06:** SRTP key negotiation/integrity conflict in previous cryptographic studies remains BLOCKED; desktop shares must not be treated as secure until actual negotiated media protection is proved.
- **RDP-X07:** Graphics pause, channel connected, SIP MCU dialog connected, media-state joining/connected and ongoing Focus membership can differ independently.

**No implementation code, real-server or owner interoperability tests, merge/release. Global research NOT COMPLETE.**