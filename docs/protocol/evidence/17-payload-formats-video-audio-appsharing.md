# Evidence: payload formats – MS-H264PF, MS-RTVPF, MS-RTPRADEX, MS-DTMF, MS-RTASPF, MS-XMLMC

Sources (all edition 20250520): [MS-H264PF] 33 pp.; [MS-RTVPF] 36 pp.; [MS-RTPRADEX] 16 pp.; [MS-DTMF] 18 pp.; [MS-RTASPF] 16 pp.; [MS-XMLMC] 20 pp.

## H.264UC (MS-H264PF)

| ID | Requirement | Evidence |
|---|---|---|
| PL-01 | Based on RFC 6184 (H.264) + RFC 6190 (SVC); only **MST** (multi-session) and **NI-TC**; clock 90 kHz; marker = last packet of a layer of an access unit | H264PF §1.3, §2.2.1–2.2.3, pp. 8–10 |
| PL-02 | Only single NAL, STAP-A, FU-A (no STAP-B/MTAP); one RTP session per layer (same PRID); **PACSI NAL as the first NAL of each layer of each AU**, never fragmented | §2.2.4, p. 10; §3.1.5.1, p. 21 |
| PL-03 | SEI in the PACSI (user data unregistered, payloadType 5): stream layout UUID `139FB1A9-446A-4DEC-8CBF-65B1E12D2CFD` (layer presence bytes LPB0–7, layer descriptions of 16 bytes: coded/display W×H, bitrate, FPSIdx 0–6 = 7.5/12.5/15/25/30/50/60, layer type, PRID, CB), cropping info UUID `BB7FC1A0-6986-4052-90F0-0929217539CF`, bitstream info UUID `05FBC6B9-5A80-40E5-A22A-AB4020267E26` (ref_frm_cnt, NAL count) | §2.2.5–2.2.7, pp. 10–17 |
| PL-04 | First NAL = full stream layout; the receiver drops packets without a preceding full layout or without a PACSI in first position | §2.2.5.1, p. 13; §3.1.5.3, p. 23; §3.2.5.1, p. 24 |
| PL-05 | Simulcast (several resolutions/codecs), at most one RTVideo simulcast stream with its own SSRC; the mixer forwards exactly one | §3.1.5.3, p. 23 |
| PL-06 | FEC (x-ulpfecuc) per RFC 5109 with its own SN offset instead of SN base and an FEC level extension header (HR1/HR2, count, index); XOR with 1 FEC packet, otherwise Reed-Solomon (not specified) | §2.2.8, pp. 17–20; §3.1.5.2, pp. 22–23; §3.2.5.2, pp. 25–26 |
| PL-07 | Consistency check of example §4.1 (p. 27): payloadSize 0x3A = 58 = 16 (UUID) + 8 (LPB) + 1 + 1 + 2×16 ✓ | p. 27 |

## RTVideo (MS-RTVPF)

| ID | Requirement | Evidence |
|---|---|---|
| PL-10 | RTVC1 bitstream = **VC-1 (SMPTE 421M)** with cached-frame/SP-frame extensions; packet formats basic (M=0), extended (M=1, M2=0, 10-bit frame counter), extended 2 (reserved, do not use), FEC (XOR, DV 00/01, up to 31 FEC packets) | RTVPF §1.3–1.5, p. 8; §2.2–2.2.5, pp. 10–18; §3.1.5.1, p. 19 |
| PL-11 | Sequence header in the first packet of every I-frame; binding byte 0x25 (with B-frames) / 0x27; fragments < 1200 bytes; aspect ratio from MAX_CODED_WIDTH/HEIGHT | §2.2.2, pp. 11–12; §3.1.5.2–3.1.5.3, p. 20 |
| PL-12 | Receiving: drop a frame if packets are missing and FEC does not help; resolution above the negotiated one ⇒ drop | §3.2.4–3.2.5, pp. 22–23 |
- L-07: SMPTE 421M (VC-1) is not in the package; the SP/cached-frame semantics are only described at packet level – an interoperable RTVideo **codec** cannot be built from the package alone.

## Redundant audio (MS-RTPRADEX)

| ID | Requirement | Evidence |
|---|---|---|
| PL-20 | RFC 2198, but at most **one** redundancy block + primary block per packet, distance ≤3, same codec as the primary block, dynamic symmetric PT ("RED"), not for DTMF | RTPRADEX §2.2, p. 8; §3.1–3.2, pp. 9–10 |

## DTMF (MS-DTMF)

| ID | Requirement | Evidence |
|---|---|---|
| PL-30 | RFC 4733, events 0–16, clock fixed at 8000 Hz, one symmetric PT (101), out-of-band only, no redundancy, one event per packet, duration ≠0 | DTMF §2.2.1, p. 8; §3.1–3.3, pp. 10–12; example SDP pp. 8–9 |

## Application sharing transport (MS-RTASPF)

| ID | Requirement | Evidence |
|---|---|---|
| PL-40 | RDP data stream (MS-RDPBCGR) + multiparty channel (MS-RDPEMC) as RTP payload **PT 127**, marker 0, only over TCP (ICE2 TCP), RTP packets ≤1500 bytes, reassemble by sequence number when receiving | RTASPF §1.3–1.4, pp. 6–7; §2.2, p. 8; §3.1.5, p. 9 |
| PL-41 | RDP encryption "None" (protected by SRTP), bulk compression viewer↔MCU off, sharer↔MCU on; omit the Security Exchange PDU; FriendlyName in the Participant Created PDU = own SIP URI | §1.5, p. 7; §3.1, §3.1.5, p. 9 |
| PL-42 | Packet loss ⇒ stop sending; version 0x00080004 | §3.1.5, p. 9; §1.7, p. 7 |

## Video control via SIP INFO (MS-XMLMC)

| ID | Requirement | Evidence |
|---|---|---|
| PL-50 | `<media_control><vc_primitive><to_encoder><picture_freeze/>` (from the MCU) ⇒ pause sending video, keep RTCP; `picture_fast_update` ⇒ send again + key frame; never an error on a well-formed picture_freeze | XMLMC §2.2.1, p. 9; §3.1.5, pp. 10–11; example p. 14 |
- L-08: Base schema draft-ietf-mmusic-xml-schema-for-media-control-12 and the Content-Type of the INFO body are not in the package.

## Audio codecs G.711 and G.722 (ITU-T, obtained 2026-10-28)

The base codec set of M3 (K-RT-1: G.711/G.722 are the safe base) was not in the Microsoft package (gap L-12, closed): [ITU-T G.711] (11/1988) and [ITU-T G.722] (09/2012) incl. the electronic attachment with the digital test sequences were obtained from itu.int (`sources-secondary.tsv`), kept outside the repository.

| ID | Requirement | Evidence |
|---|---|---|
| G7-01 | 8000 samples/s, 8 bits per sample, A-law and µ-law; bit 1 (polarity) transmitted first | G.711 §2–4, p. 1 |
| G7-02 | A-law: 13 segments (segment 1: 32 × 2, segments 2–7: 16 × 2^s from 2^(s+4)), decision values x_n, quantized value y_n = (x_{n−1} + x_n)/2, character 128 + (n − 1) positive, n − 1 negative, **even bits inverted** (0x55) | Tables 1a/1b, pp. 3–4, Notes 2–3 |
| G7-03 | µ-law: segment 1: 1 × 1 + 15 × 2, segments 2–8: 16 × 2^s from 2^(s+4) − 33; y_0 = 0, y_n = (x_n + x_{n+1})/2; character 255 − n positive, 127 − n negative | Tables 2a/2b, pp. 5–6, Notes 2–3 |
| G7-04 | µ-A/A-µ conversion tables; deliberate deviations µ-80 → A-81 and A-80 → µ-79 | Tables 3/4, pp. 7–9 and Notes |
| G7-05 | Digital milliwatt: periodic sequences of Tables 5/6 = 1 kHz at 0 dBm0; T_max +3.14 dBm0 (A) / +3.17 dBm0 (µ) | §5, p. 2; Tables 5/6, p. 10 |
| G2-01 | SB-ADPCM: transmit/receive QMF (24 coefficients, Table 11), lower sub-band 6-bit ADPCM (Modes 1/2/3 = 6/5/4 bits in the decoder), higher sub-band 2-bit ADPCM | G.722 §1.4–1.5, pp. 4–7; §5, pp. 21–27 |
| G2-02 | Octet format IH1 IH2 IL1 … IL6, IH1 first | §1.4.4, p. 5 |
| G2-03 | ADPCM blocks 1L … 6L, 1H … 5H with 16-bit saturating arithmetic, `*` = (A·B) >> 15; constants Tables 14/15, codeword tables 16–21; scale factor Method 1 (353-entry ILA) or Method 2 (32-entry ILB) | §6.2, pp. 28–39 |
| G2-04 | QMF: accumulators ≥ 24 bits, truncation or rounding – therefore **no** test sequences for the QMF | §5.2 Note 1, p. 22 |
| G2-05 | Digital test sequences (Configuration 1 encoder, Configuration 2 decoder Modes 1–3; 17 files, ASCII format with checksums, reset by RSS) | Appendix II |
| G2-06 | RTP: payload type 9, clock rate 8000 although sampled at 16 kHz; [MS-RTP] PT 9/117 (stereo) | RFC 3551 §4.5.2; [MS-RTP] §2.2.1, pp. 14–15 |

### Recomputed
- G.711: the generated decision values and quantized values match every segment end point of Tables 1a/1b/2a/2b; all 88 rows of Table 3 and 98 rows of Table 4 are reproduced by "nearest quantized value on a logarithmic scale" with the physical scaling 4096 ↔ +3.14 dBm0, 8159 ↔ +3.17 dBm0, except exactly the two deviations of the Notes (inference about how the tables were built, recomputed); the sequences of Tables 5/6 decode to 0 dBm0 ± 0.1 dB.
- G.722: the ADPCM implementation (`crates/g722`, Method 2) reproduces **all 14 comparison files** of Appendix II bit-exactly (T2R1, T2R2 from T1C1/T1C2; T3L1–3.RC1–3 and T3H1–3.RC0 from T2R1/T2R2/T1D3). The test sequences are not redistributed; CI runs a pinned regression value recorded from this implementation.

## Conflicts / gaps (codecs)
- K-G2-1: Table 14 prints QQ4 and WL on addresses 2 … 9, while INVQAL/LOGSCL address them with IL4 = 0 … 7 (Table 17). Resolved: address = IL4; confirmed by the test sequences.
- K-G2-2: INFA (Appendix II) writes `XL = S# >> 1`; X# is meant (editorial).
- L-12: ITU-T G.711 and G.722 were not in the Microsoft package – **closed** (obtained 2026-10-28).

## Later findings (files 38, 39)
- L-07 **partly**: VC-1 RTP RFC 4425 obtained; SMPTE 421M (not IETF, paid) still not (OB-29: SMPTE ST 421:2013 only for purchase).
- L-08 **closed**: draft-levin-mmusic-xml-media-control-12 obtained; INFO body `Content-Type: application/media_control+xml` (IE-07).
- Base RFCs 6184/6190/5109/2198/4733 obtained.
- Comparison of MS-RTVPF §3.1.5.5 (p. 21) with FFmpeg's VC-1 decoder suggests that RTVideo signals cached frames and SP-frames with picture type codes beyond the VC-1 code `1111` (skipped picture); a standard VC-1 decoder would therefore misread them (inference).
- L-07 **closed by decision** D-05 (OB-51): no RTVideo/VC-1 codec.
