# Evidence: RDP basics for screen/application sharing – MS-RDPBCGR, MS-RDPEMC, MS-RDPEPS

Sources (Windows package): [MS-RDPBCGR] "Remote Desktop Protocol: Basic Connectivity and Graphics Remoting", 444 pp.; [MS-RDPEMC] "Multiparty Virtual Channel Extension", 41 pp.; [MS-RDPEPS] "Session Selection Extension", approx. 15 pp.; also available: MS-RDPEGFX, MS-RDPRFX, MS-RDPNSC, MS-RDPEGDI, MS-RDPEDYC, MS-RDPEMT, MS-RDPEUDP(2), MS-CSSP, MS-RDPELE. SfB embedding: MS-RTASPF (file 17, PL-40–42), MS-CONFAS (file 22), MS-SDPEXT m=applicationsharing (file 14, SD-80/81).

| ID | Requirement | Evidence |
|---|---|---|
| RD-01 | RDP connection sequence (10 phases): X.224 Connection Request/Confirm → MCS Connect Initial/Response (GCC blocks) → Erect Domain, Attach User, Channel Join (incl. static virtual channels) → [Security Exchange] → Client Info → [Auto-Detect] → Licensing → [Multitransport] → Demand/Confirm Active (capabilities) → finalisation (Synchronize, Control Cooperate/Request/Granted, Font List/Map) | RDPBCGR §1.3.1.1, pp. 24–28 |
| RD-02 | In SfB: RDP encryption "None", **the Security Exchange PDU is omitted**, protection by SRTP; compression viewer↔MCU off | RTASPF §1.5, §3.1, pp. 7–9 (file 17 PL-41); RDPBCGR §1.3.1.1 phase 4, p. 26 |
| RD-03 | Multiparty information over the static virtual channel **"encomsp"**: ORDER_HDR (Type, Length, LE), types 0x0001–0x000D (filter, app/window created/removed, show window, participant created/removed, control level change + response, graphics stream paused/resumed, region update) | RDPEMC §1.5, p. 10; §2.1–2.2.1, pp. 12–13 |
| RD-04 | Participant Created PDU: ParticipantId, **GroupId** (SfB: fixed 3766552189), flags MAY_VIEW 1 / MAY_INTERACT 2 / IS_PARTICIPANT 4, FriendlyName (UNICODE_STRING ≤1024 characters; SfB: user@host of the endpoint URI) | RDPEMC §2.2.2, p. 12; §2.2.4.1, pp. 17–18; CONFAS §3.2.5.2.1–3.2.5.2.2, p. 17 (file 22 CS-09) |
| RD-05 | Request control (remote control): Change Participant Control Level PDU (REQUEST_VIEW 1, REQUEST_INTERACT 2, ALLOW_CONTROL_REQUESTS 8) and the response with ReasonCode; participant removed with DiscType/DiscCode | RDPEMC §2.2.4.2–2.2.4.4, pp. 18–20 |
| RD-06 | Pre-Connection PDU (V1 16 bytes: cbSize, flags 0, version 1, Id; V2 + cchPCB/wszPCB) before all RDPBCGR PDUs, 10 s timer; contents (Id/wszPCB) "by convention" | RDPEPS §2.2.1, pp. 9–10; §3.1.2–3.1.5.2, pp. 11–12; RDPEMC §1.5, p. 10 |

## Gaps / open points
- O-RD-1: Whether and with which values SfB sends the Pre-Connection PDU (RDPEPS) inside the RTP transport is **not** stated in MS-RTASPF/MS-CONFAS (RDPEMC mentions it only in general). Interop capture needed.
- O-RD-2: Which graphics capabilities/codecs (bitmap, RemoteFX MS-RDPRFX, graphics pipeline MS-RDPEGFX, NSCodec MS-RDPNSC) SfB sharers offer is not fixed in the SfB specifications; the viewer must support those offered in Demand Active. The specifications themselves are complete; an existing RDP library is a candidate implementation basis (behind an adapter crate, decision D-03).
- Note: MS-CONFAS speaks of an "Attendee Connected PDU", MS-RDPEMC calls the structure "Participant Created PDU" (same section number §2.2.4.1) – naming difference.

## Later findings (file 39)
- O-RD-1 **narrowed** (OB-18): SIPE works without a pre-connection blob (inference: not required).
- O-RD-2 **narrowed** (OB-19): MultifragMaxRequestSize ≤ 0x3EFFFF; (OB-35, FreeRDP PR #3669): the Lync 2013 viewer accepts RemoteFX. Other codecs still open.
