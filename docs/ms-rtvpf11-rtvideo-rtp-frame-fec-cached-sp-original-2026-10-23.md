# MS-RTVPF 11.0 — RTVideo RTP formats, frame reference, forward-error correction and forwarding state

**Original-source research, 2026-10-23. STATUS: partial video subsystem, not an implementation specification.** Primary Microsoft **[MS-RTVPF] RTP Payload Format for RT Video Streams Extensions**, revision **11.0**, release **2025-05-20**, version **v20250520**, SHA-256 **2ea77af992e1f2935878134886884bbafe90ef0f045aab31ef52de1c0c278e6a**, [original Microsoft PDF](https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-RTVPF/%5BMS-RTVPF%5D.pdf). Reviewed original §§1.3–1.7,2.2.1–2.2.5,3.1.4–3.1.5.6,3.2.5 and selected §4 examples; PDF page 11 was directly rendered and inspected against the text extract. Original normative inputs: [RFC3550](https://www.rfc-editor.org/rfc/rfc3550) §5.1 for RTP header and clock semantics, **SMPTE-VC-1** (VC-1 codec and sequence/entry-point headers), **[MS-RTP] rev17** feedback/SSRC state, **[MS-SDPEXT] rev16** negotiated codec and media. **P** = primary source rule with actor, **R** = RFC, **S** = integrated inference, **U** = unresolved client/server profile. A historic Microsoft proprietary RTVC1 codec format is not evidence of a required codec on every Windows deployment.

## 1. Wire format and SDP/RTP frame semantics

| ID | Original clause | Source-qualified obligation |
|---|---|---|
| RTV-01 | §2.2.1 | RTP timestamp **MUST use 90,000 Hz**; marker bit **MUST be 1 only for last RTP packet of a video frame**. When the frame contains trailing FEC metadata, that FEC packet can be the last RTP packet. MS-RTP/RFC3550 RTP sequence remains a **media stream** sequence, not SIP CSeq. |
| RTV-02 | §2.2.2 | **Basic** RTVideo payload header: min **1 byte**, max **65 bytes** including codec headers. First octet flags M=0 mode, C cached, SP super P, L last *data* packet, O=1, I I-frame, S codec header present, F first data packet. Conditional codec-header length **MUST <=63**. |
| RTV-03 | §§2.2.2–2.2.3 | **Extended** header min **4 bytes**, max **68**, mode bit M=1, M2=0, group-of-pictures **FrameCounter/RefFrameCounter**, including B-frame reference-delta semantics. Frame counter resets at start of **each GOP**, not on SIP INVITE or SSRC reset automatically. |
| RTV-04 | §§2.2.4,3.1.5.1 | **Extended 2** min **8 bytes**, max **72**, documented as a format but **MUST NOT be used as an ordinary RTVideo format**: §3.1.5.1 reserves it for future extensions. Do not mistake definition of ABNF fields for negotiated/allowed payload profile. |
| RTV-05 | §§2.2.5,3.1.5.4 | Separate **RTVideo FEC** RTP payload header is **fixed 8 bytes**; special case of Extended 2 with S=0 and E=1. **HiRFC=0**, **RefFrameCounter=0**, and metadata counters/offset fields have meaning specific to FEC. FEC metadata carries repair information, not a renderable image frame by itself. |
| RTV-06 | §2.2.5 | FEC field DV versions **0 or 1 SHOULD**; for DV=1, **FECPacketsNumber** is a five-bit nonzero value with **maximum 31 packets**. For DV!=1, those bits are reserved and sender **MUST set zero**, receiver **MUST ignore**. Conditional reserved-bit interpretation is critical. |
| RTV-07 | §§2.2.2,3.1.5.3 | Codec Headers contain binding byte, VC-1 sequence and entry-point headers. Binding byte **0x25 means B-frames present; 0x27 means absent**. Sequence header **MUST appear in first video data packet of an I-frame**. Display aspect ratio uses VC-1 MAX_CODED_WIDTH/HEIGHT instead of DISP_HORIZ_SIZE/VERT_SIZE under Microsoft's stated extension. |
| RTV-08 | §§3.1.5.1,3.1.5.2.1 | **Extended format SHOULD** be used to detect missing reference frames and avoid rendering artifacts. Basic MAY be used for resource reasons. Every fragment plus **RTP, transport and IP overhead** must fit path MTU; **video fragment MUST be <1200 bytes**, even when link MTU is larger. |
| RTV-09 | §3.1.5.2.2 | When using FEC, all frame-data packets **except last** must have **same payload block size**; last can be shorter. FEC repair is calculated on common-sized blocks after zero-padding as specified §3.1.5.4, padding not necessarily part of displayed original video. |
| RTV-10 | §§3.1.5.4,3.1.5.4.1–2 | FEC metadata RTP packets **MUST follow data packets** and use **the same RTP sequence-number space**; any extra FEC transmitted consumes sequence numbers. One FEC packet SHOULD be constructed when DV=00. Sender derives metadata from both payload header and video data, not just from compressed picture bytes. |
| RTV-11 | §2.2.1 | **Video-forwarding server**, when uplink video packet missing and FEC can reconstruct it, **MUST forward video packets**. For every packet lost on uplink it **MUST forward an RTP packet with valid RTVideo RTP header but empty payload**, preserving the shared RTP sequence space. This is an **MCU/server rule**, not Windows viewer's mandate to emit empty packets. |
| RTV-12 | §3.1.5.5 | Encoder and decoder cache synchronized frames (one cache frame at a time). C-bit marks new cached frame and decoder **MUST cache** it; SP-bit denotes Super-P predicted from latest cached frame, potentially useful on packet loss. Cached/SP state is independent of RTP marker and I-frame status. |
| RTV-13 | §3.1.5.6 | **RTP sequence numbers MUST remain continuous across frames**. First data packet F-bit=1, last data packet L-bit=1, **last RTP packet** M-bit=1. For FEC, PacketNumber, LastPacketLength, DV, EndOffset and frame-counter fields **MUST** be consistent; codec headers present on first I-frame data packet. |
| RTV-14 | RFC3550 §5.1 vs MS-RTVPF §2.2 | RFC3550 defines general RTP sequence/timestamp/marker fields but **does not define** Microsoft's C/SP/L/O/I/S/F codec flags, FEC DV layout or 31-packet count. Applying an H.264 NAL parser to RTVC1 FEC is unsupported. |
| RTV-15 | MS-RTVPF §3.1.5.1 | Normal RTVideo **Extended 2** is forbidden, but **FEC payload is an explicitly defined special case**. That is not a contradiction; the prohibition applies to its ordinary use, and FEC is allowed if the appropriate format negotiated. |
| RTV-16 | MS-RTVPF §2.2.1 / MS-RTP §§3.2.5–3.2.6 | An RTP marker or FEC recovery signal is **not** proof of successful decoder output or recovered ICE consent. RTCP loss-feedback and optional encoder SP/I recovery, SRTP authentication and conference MCU roster are separate gates. |

## 2. Per-frame sender/receiver/relay states (S, not a captured network trace)

~~~mermaid
flowchart TD
  A[SDP negotiates RTVideo media/PT and RTP protection] --> B[Encoder emits VC1 I/P/B/SP frame]
  B --> C[Select Basic/Extended, optional FEC]
  C --> D[Fragment with each video fragment below 1200 bytes]
  D --> E[Set RTP 90kHz timestamp, continuous sequence and F/L/M flags]
  E --> F[Optional FEC appended in same RTP sequence space]
  F --> G[Media transport under separately negotiated ICE/SRTP]
  G --> H{Viewer or forwarding MCU?}
  H -->|Viewer| I[Validate frame headers and reference cache]
  I --> J{Referenced frame available?}
  J -->|Yes| K[Decode/display with VC1 header sizing]
  J -->|No| L[May discard dependent frame, feedback to RTCP/encoder]
  H -->|MCU| M[Account for missing packets and repair FEC]
  M --> N[Forward recovered content and specified empty RTP placeholders]
~~~

**Role caveat:** §2.2.1 mandates a special forwarding-MCU behavior, not every P2P relay. ICE candidate nomination, SSRTP/SRTP authenticity, conference media membership and decoder reference cache must be independently valid.

## 3. Unresolved requirements / edition guard

- **RTV-G01:** Bit-exact contents of all 1/4/8-byte headers, counter wrap and complete §4 network byte examples require table-by-table source extraction. PDF 11 rendered; not all 36 pages visually audited.
- **RTV-G02:** §3.1.5.4 FEC calculation with multiple repair packets (DV=1), all metadata lengths and transport MTU tunnel overhead need packet-level specification; source offers algorithm, but no assistant-run byte tests were done.
- **RTV-G03:** RTVC1/SMPTE-VC-1 decoder is a separate technical dependency; do not claim this document alone specifies video codec compression algorithm.
- **RTV-G04:** Media runtime whether RTVideo still negotiated in specific classic SfB Windows build and server, versus H.264/Scalable H.264, is **U**; user must determine deployment/client build.
- **RTV-G05:** Server-forwarding empty-packet semantics should be reconciled with MS-RTP throttling, RTCP loss request timers, RTP replay/MKI/auth failures, ICE consent and video decode buffer drops.
- **RTV-G06:** SRTP SHA-1/SHA-256 original-source conflict remains **BLOCKED** for any transport of RTVideo under that media security profile; source-defined payload does not justify insecure downgrade.

**Global Full Client normative research NOT complete; no protocol code, live-server/interop tests, main merge or release.**
