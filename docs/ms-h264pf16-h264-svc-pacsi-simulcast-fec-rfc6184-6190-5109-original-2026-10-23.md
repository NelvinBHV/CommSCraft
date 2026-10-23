# MS-H264PF 16.0 — H.264/SVC RTP packetization, PACSI layout, FEC repair, simulcast and MCU forwarding

**Primary-document research 2026-10-23. STATUS INCOMPLETE, no code or server traffic.** Microsoft **[MS-H264PF] RTP Payload Format for H.264 Video Streams Extensions**, revision **16.0**, edition **v20250520**, published **2025-05-20**, SHA-256 **9f15e958bf6eee08e6996d2dd94a7a005489e802be08d985ba4a835b14f88f1a**, [official Microsoft PDF](https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-H264PF/%5BMS-H264PF%5D.pdf). Read original §§1.3–1.7,2.2.1–2.2.8.1.4,3.1.4–3.1.5.3.1,3.2.5.1–3.2.5.2.2; original PDF p18 directly rendered. **Primary normative IETF editions** [RFC6184](https://www.rfc-editor.org/rfc/rfc6184) (May 2011) H.264 RTP §§5–7, [RFC6190](https://www.rfc-editor.org/rfc/rfc6190) (May 2011) SVC RTP §§4–5, [RFC5109](https://www.rfc-editor.org/rfc/rfc5109) (Dec 2007) unequal FEC §7, [RFC3550](https://www.rfc-editor.org/rfc/rfc3550) (July 2003) RTP §5.1, ISO/IEC 14496-10:2014 reference. Other Microsoft sources **[MS-SDP]**, **[MS-SDPEXT] rev16**, **[MS-RTP] rev17**, **[MS-RTVPF] rev11**. **P** direct first-party and role qualified; **R** direct RFC; **S** cross-spec inference; **U** unproven Windows full-client capability. **H.264 payload format is not H.264 decoder normative content**, which is ISO/IEC/ITU codec text.

## 1. Packetization rules and necessary decoder preconditions

| ID | Direct clause | P/R rule and consequence |
|---|---|---|
| H264-01 | MS-H264PF §2.2.1 | Every RTP packet carrying the **last packet of one layer of an access unit** MUST set RTP marker **1**; this may be VCL NAL or associated FEC packet. **Clock frequency MUST be 90,000 Hz**. All packets of same access unit within same simulcast stream have same timestamp, but different simulcast streams **need not** share timestamp. |
| H264-02 | §2.2.2, RFC6190 §4.4 | Protocol **only supports multi-session transmission (MST)** for SVC. The layer/session mapping must not be conflated with generic single-session SVC or arbitrary simulcast SSRC layout. |
| H264-03 | §3.1.5.1 and RFC6184 §6.3 | Sender **MUST use non-interleaved packetization mode** (mode 1); **single NAL packets, STAP-A and FU-A only**. STAP-B, MTAP (and source NI-MTAP) MUST NOT be sent. **RFC6184 itself allows other modes**; do not assume those alternatives are supported by Microsoft. |
| H264-04 | §§2.2.4,3.1.5.1 | Every layer of each access unit **MUST start with PACSI NAL**, exactly **one PACSI** per layer/access unit; optionally PACSI first inside STAP-A, **MUST NOT fragment PACSI**. A NAL exceeding MTU **MUST fragment into FU-A**. Aggregated STAP-A MUST fit MTU. |
| H264-05 | §3.1.5.1 | One RTP session **MUST carry one layer PRID value per coded sequence**. Stream layout SEI message **MUST be inside PACSI**. Do not combine independent layers in same RTP session merely to minimize SSRC count. |
| H264-06 | §2.2.5.1 | Full Stream Layout SEI: **P=1**, at least one Layer Description matching every layer presence bit, and **first nontrivial NAL of session MUST be a full-layout PACSI**. Base layer of first simulcast stream of IDR access unit MUST include full layout; other streams MAY repeat. |
| H264-07 | §§2.2.5.1,3.1.5.3 | Update layout **P=0** omits Layer Description and **only toggles presence bits for previously declared layers**. Cannot announce a never-defined layer (full-layout PACSI required). Full/update layout **MUST precede** any changed stream, and no packet of removed layer may follow removal notice. |
| H264-08 | §3.2.5.1 | On reception, **per-PRID layer buffer**. If first RTP packet of one SSRC/timestamp is neither PACSI nor STAP-A starting with PACSI, receiver **MUST discard all packets for that SSRC/timestamp**. No full Stream Layout PACSI yet, or layer missing/not described, => **MUST discard** dependent received packets. |
| H264-09 | §2.2.8, RFC5109 §7 | H.264 FEC is built on **RFC5109** unequal packet-level FEC with Microsoft **additional level-extension header and SN Offset changes**, not the RTVideo proprietary 8-byte FEC metadata header of MS-RTVPF. These are **different FEC wire formats**. |
| H264-10 | §2.2.8.1.1 | FEC for each layer/access unit **MUST follow media data packets**; last FEC packet marker 1; same **SSRC**, **RTP sequence space**, **timestamp** as protected media but **different dynamic payload type** selected out-of-band. Dynamic payload numbers cannot be globally hardcoded. |
| H264-11 | §§2.2.8.1.2,3.1.5.2.1 | Microsoft RFC5109 FEC header replaces SN base by **SN Offset, 16-bit field**: offset between FEC packet RTP seq and lowest protected packet seq with wrap accounted. The source wording says unsigned **32-bit** computed difference though represented in a **2-byte field** — **source width ambiguity**, must not invent truncation/overflow rule. |
| H264-12 | §3.1.5.2.1 | FEC protected **64-bit** header bitstring includes 2 zero bits, P, X, four zeros, marker, PT, 32 zeros, 16-bit **RTP media payload length** excluding fixed 12-byte RTP header, CSRC list, RTP extension and padding. Feed to RFC5109 protection operation, then distribute recovery bits to FEC and level extension. |
| H264-13 | §§2.2.8.1.2–3,3.1.5.2.1 | FEC **E flag MUST 1**, L flag MUST 1 if >16 media packets else 0; level extension **V/C/Reserved MUST zero**, **FEC Count** total and **FEC Index** zero-based; RFC5109 mask/protection length retained. This is not the TURN/RTCP 16-bit Type+Length extension format. |
| H264-14 | §3.1.5.2.2 | With only one FEC packet, forward error correction uses **XOR**; source also **permits Reed–Solomon**, but explicitly places Reed–Solomon algorithm details **outside this document**. No claim of self-sufficient complete decoder from MS-H264PF alone. |
| H264-15 | §3.2.5.2.1–2 | FEC receiver restores lost RTP header fields from recovered bits, sets version 2, derives lost SN via FEC SN/offset/bit index, and copies SSRC/CSRC from FEC; payload recovery zero-pads shorter protected blocks and then cuts recovered payload to restored original **Length Recovery**. Correct length is needed to avoid decoder trailing bytes. |
| H264-16 | §3.1.5.3.1 | **RTVideo simulcast can coexist with H.264** but at most **one RTVideo simulcast stream** and its own SSRC; unlike H.264 layers, RTVideo presence is indicated by **RTP payload type** negotiated via MS-SDP, not by H.264 Stream Layout PACSI. |
| H264-17 | §3.1.5.3.1 | **Mixer MUST forward at most one simulcast stream** downstream. If H.264 available mixer chooses one H.264 stream; otherwise may forward RTVideo. Server switching stream is not client encoding two profiles on same RTP stream. |
| H264-18 | MS-H264PF §§2.2.1,3.1.5.3 / RFC3550 | Codec/layer frame completion markers, RTCP bandwidth feedback, ICE nomination, MS-SRTP/SSRTP replay/authentication and conference endpoint admission all represent **independent states**. Correct PACSI alone does not prove authenticated renderable video. |

## 2. Multistream signaling and failure state sketch (S, not a mandatory observed packet trace)

~~~mermaid
sequenceDiagram
 participant E as H264/SVC video encoder
 participant M as Conference video mixer
 participant V as Viewer decoder
 Note over E,M: SDP negotiates payload types and per-layer sessions
 E->>M: Full Stream Layout PACSI first, PRID/layers
 E->>M: Single NAL, STAP-A or FU-A fragments, 90kHz RTP
 opt FEC enabled
  E->>M: RFC5109-compatible Microsoft FEC packets after data
 end
 M->>M: Validate selected simulcast stream, forwarding profile
 M->>V: At most one selected simulcast RTP stream
 V->>V: Validate first PACSI of each SSRC/timestamp
 alt Full layout missing or unknown layer
  V->>V: Discard affected packets
 else Valid layer and intact or FEC-recovered frames
  V->>V: Assemble NAL units, decode after SRTP security validation
 end
 opt Simulcast layout change
  E->>M: Full or update PACSI before add/remove of layers
 end
~~~

**S guard:** Source does not imply every Full Client must transmit H.264 scalable layers; endpoint codec capability, selected payload type, advertised number of layers, deployment MCUs and Windows release are all conditional. A successful SIP 200 with SDP does not mean the first PACSI or next IDR was decoded.

## 3. Cross-RFC/version and security blockers

1. **H264-X01:** Microsoft's *MST-only, noninterleaved mode 1* is a proper subset of RFC6184/RFC6190. Never accept every RFC-valid packetization mode as Microsoft-client compliant without feature negotiation.
2. **H264-X02:** §2.2.8.1.2 2-byte SN Offset yet 'unsigned 32-bit value' prose inconsistency. Extract original PDF field diagram with full context and check formal errata/packet examples; no assumed wrapping rule.
3. **H264-X03:** XOR and Reed–Solomon different FEC recovery algorithms; source intentionally does not detail latter. A complete FEC decoder needs additional normative source.
4. **H264-X04:** Per-stream decoder cache, full/update PACSI errors, RTP sequence rollover, packet loss and layout races must be reconciled with MS-RTP §3.1.5 throttling and RTCP video-loss notifications.
5. **H264-X05:** Video FEC dynamic payload, PACSI NAL profile, compatible RTVideo stream and mixer selection need exact MS-SDP §3.1 and historical client product notes; no invented PT numbers.
6. **H264-X06:** The unresolved **MS-SRTP rev14 HMAC-SHA256 versus MS-SDPEXT rev16 suite HMAC-SHA1** conflict applies to protected H.264 too. Do not disable encryption to render video.
7. **H264-X07:** All precise bitfield/coding SEI types, original full 33 pages, RFC6190 scalability extensions and codec specification ISO/IEC 14496-10:2014 have **not** been exhaustively verified; evidence status partial.

**The classic Windows Full Client entire protocol analysis remains NOT COMPLETE. No code, live-server connections, interoperability tests, main merge or release.**
