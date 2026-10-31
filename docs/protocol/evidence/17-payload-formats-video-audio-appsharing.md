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

## Later findings (files 38, 39)
- L-07 **partly**: VC-1 RTP RFC 4425 obtained; SMPTE 421M (not IETF, paid) still not (OB-29: SMPTE ST 421:2013 only for purchase).
- L-08 **closed**: draft-levin-mmusic-xml-media-control-12 obtained; INFO body `Content-Type: application/media_control+xml` (IE-07).
- Base RFCs 6184/6190/5109/2198/4733 obtained.
- Comparison of MS-RTVPF §3.1.5.5 (p. 21) with FFmpeg's VC-1 decoder suggests that RTVideo signals cached frames and SP-frames with picture type codes beyond the VC-1 code `1111` (skipped picture); a standard VC-1 decoder would therefore misread them (inference).
