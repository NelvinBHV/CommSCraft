# Evidence: MS-RTP – RTP/RTCP extensions

Source: [MS-RTP] "Real-time Transport Protocol (RTP) Extensions", edition 20250520, 68 pp. Based on RFC 3550/3551/4585/5506/5761/4571/4733/3389 (obtained, file 38).

## RTP

| ID | Requirement | Evidence |
|---|---|---|
| RT-01 | UDP and TCP (RFC 4571 framing), IPv4/IPv6; one RTP packet per datagram, ≤1500 bytes including header; transport inactive >30 s ⇒ end the session; RTP/RTCP mux SHOULD (RFC 5761) | §2.1, p. 14; §1.4, p. 11 |
| RT-02 | Several RTP sessions on one transport only with disjoint SSRC ranges (SDPEXT x-ssrc-range); drop foreign SSRCs | §2.1, p. 14; §3.1, p. 34 |
| RT-03 | Marker bit: audio = start of a talk spurt; video = last packet of a frame | §2.2.1, p. 14 |
| RT-04 | Static/usual PTs: 0 PCMU, 8 PCMA (10/20/40/60 ms), 9/117 G.722, 13 CN 8k, 118 CN 16k, 103 SILK 8k (60 ms), 104 SILK 16k (20/60/100 ms), 106 OPUS 48k/2, 111 Siren, 112 G.722.1, 114 RTAudio 16k, 115 RTAudio 8k, 116 G.726, 121 RTVideo, 122 H.264, 123 H.264 FEC, 127 x-data; dynamic PTs may differ per direction | §2.2.1, pp. 14–15 |
| RT-05 | G.722 stereo = left frame ‖ right frame, PT 117 | §2.2.1.1, p. 16 |
| RT-06 | SSRC random ≠ 0, do **not** use the RFC 3550 loop detection; do not change the SSRC within 2 s | §2.2.1, p. 15 |
| RT-07 | CSRC list from mixers: MSI instead of SSRC; **first CSRC = dominant speaker** (own SSRC allowed too); receiver: show "is speaking", expires after 3 s | §2.2.1, pp. 15–16; §3.1, p. 34; §3.1.2, p. 35; §3.1.5, p. 37 |
| RT-08 | SSRC and sequence number throttling (algorithm with LastGood/Resync/LastBad, timer ≤2 s) | §3.1–3.1.5, pp. 34–37 |
| RT-09 | Participant timeout 50 s; RTCP BYE timer 20 s | §3.1.2, p. 35; §3.2.2, p. 41 |
| RT-10 | Compute jitter without DTMF packets | §3.1.5, p. 37 |
| RT-11 | Header extension abs-send-time only Microsoft Edge (not SfB) | §2.2.1.2, p. 16; <18>–<19> p. 62 |

## RTCP

| ID | Requirement | Evidence |
|---|---|---|
| RT-20 | Simple (non-compound) SDES/BYE/SR/RR allowed; SDES text NUL-terminated (except PRIV) | §2.2.2, p. 16; §2.2.10, p. 17 |
| RT-21 | With several streams RTCP only with the lowest SSRC; counters summed; at most 10 report blocks | §2.2.8–2.2.9, p. 17; §3.2, p. 37 |
| RT-22 | SDES PRIV "MS-EVT" `v=1 m=<hex8> q=<hex8>` media quality bits (network, device, echo …) | §2.2.10.1, pp. 17–18; §3.2, p. 38 |
| RT-23 | Profile extensions after SR/RR (type, length, max. 20): 1 bandwidth estimate (−3/−5/−6 special values), 4 packet loss (≤ every 500 ms), 5 video preference, 6 padding, 7 policy bandwidth, 8 TURN bandwidth, 9 audio healer, 10 receiver limit, 11 packet train, 12 peer info, 13 congestion notification, 14 modality send limit; skip unknown ones | §2.2.11, pp. 18–26 |
| RT-24 | Bandwidth estimation: probe (SR without blocks) + pair packet back to back; fast mode 250 ms up to 40 pairs; packet train (pair + 5 RR) 1 s, 21 times, afterwards ≤ every 5 s | §2.2.3–2.2.7, pp. 16–17; §3.2–3.2.6, pp. 38–43 |
| RT-25 | Feedback (RFC 4585, **always reduced size RFC 5506**): PLI (PSFB FMT 1, optional FCI with request ID + sync-frame bits per priority ID), VSR (PSFB FMT 15 type 1: MSI, key-frame flag, up to 20 entries of 0x44 bytes: PT, UCConfig mode 1, flags, aspect ratio/resolution mask, max. W×H, bitrate histogram, frame-rate mask, quality histogram, max. pixels), DSH (FMT 15 type 3, dominant speaker + up to 10 history entries) | §2.2.12, pp. 27–33 |
| RT-26 | Feedback retransmission: fast 190 ms ×4, then VSR slow 3 s ×5 | §3.2.2, p. 41; §3.2.6, p. 43 |
| RT-27 | Without VSR configuration: send video preference ≥10×; policy/TURN bandwidth ≥5×; audio healer in every RR | §3.2, pp. 39–40 |
| RT-28 | Bandwidth estimates **MUST** be sent through the profile extension (type 1, 12 or 16 octets with a 4-bit confidence, 0 = least reliable); how a host computes them is outside the specification; received pairs and trains MUST be parsed but MAY not be used for the estimate | §2.2.11.1, pp. 18–19; §3.2, p. 39; §3.2.5, p. 42 |
| RT-29 | Receiver notifications: packet loss notification (type 4) SHOULD be sent when video packet loss is detected, at most every 500 ms; network congestion notification (type 13, bits uncongested/congested by delay and by loss) – the receiver notifies the sender of changes of the congestion state; receiver-side bandwidth limit (type 10) ≥ 5 times per value; peer info (type 12) in every report until a positive estimate for us arrived | §2.2.11.2, p. 19; §2.2.11.8, p. 24; §2.2.11.10–11, p. 25; §3.2, pp. 39–40; §4.11, p. 56 |
| RT-30 | Sender side: a packet loss notification on video SHOULD lead to a recovery (e.g. an I-frame); policy server, TURN server and receiver-side limits and the modality send limit (type 14) bound what is sent | §3.2.5, p. 42; §2.2.11.5–6, 2.2.11.8, 2.2.11.12 |

## Conflicts / gaps
- K-RT-1: Product behaviour <10> (p. 62): SILK and OPUS **not** supported up to and including SfB 2019 (client and server). MS-SDPEXT <4> (p. 76) restricts SILK only up to Lync 2010. Contradiction; a client should plan G.711/G.722 as the safe base set, SILK/OPUS only after an interop test.
- K-RT-2: <13>/<14> (p. 62) "H.263 not supported in Lync 2013/SfB", "H.264 only in Lync 2013/SfB" – the wording leaves open whether later SfB versions are included (the product list names SfB 2016/2019/2021 separately).
- L-06: `[MSFT-H264UCConfig]` (UCConfig mode, VSR) was not in the Microsoft package.
- O-RT-2: §2.2.12.2 (p. 32) writes the range of histogram element i as [min + (i − 1) × level, min + i × level]; whether the upper end belongs to it is not stated clearly. Inference: a sender answers at the lower edge of the highest requested element, which lies inside the range on either reading.
- O-RT-1: The SSRC of media source in a VSR is not specified: §2.2.12 refers to [RFC4585] §6.1, where it names the source the feedback is about, but a VSR is sent before the requested source sends (its SSRC is not known yet, only its MSI or SOURCE_ANY). The builder takes it as a parameter; the value used by Microsoft clients and MCUs is an interop question.

## Gap
- L-05: RFC 3550/3551/4585/5506/5761/4571/4733/3389 were not in the Microsoft package.

## Later findings (file 38)
- L-05 **closed**: RFC 3550/3551/4585/5506/5761/4571/4733/3389 obtained.
- L-06 **closed** via archive search: [MSFT-H264UCConfig] V1.1 obtained; "UCConfig mode 1" in the VSR (RT-25) = SVC temporal scalability with hierarchical P (IE-08). Internal reference use only, therefore not redistributed.

## Implementation (2026-10-28, `crates/rtp`)
RTP header parse/build with RFC 3550 A.1 checks (version, CSRC list, extension, padding; ≤ 1500 octets, RT-01), sender with random SSRC ≠ 0 (RT-06) and start sequence 0 … 32767 ([MS-SRTP] §5.1), reception statistics after A.1/A.3/A.8 with jitter excluding telephone events (RT-10), compound RTCP SR/RR + SDES CNAME, BYE, parser for compound and simple packets (RT-20) with at most 10 report blocks (RT-21); telephone events per RFC 4733 §2.3/§2.5.1.2/§2.5.1.4 within the limits of [MS-DTMF] (PL-30: events 0–16, one event per packet, no zero duration, 8000 Hz). Not yet (see below for 2026-10-31): MS-RTP profile extensions (RT-22 … RT-27), SSRC/sequence throttling (RT-08). Status: tested-offline.

## Implementation (2026-10-31, MS-RTP RTCP profile)
`rtp`: all profile extensions of §2.2.11 (types 1, 4–14) built and parsed with the sizes of the specification, walked by their length field (≥ 4, multiple of 4, inside the report), at most 20 per report, unknown types skipped, out-of-range healer values read as 0 (RT-23); SDES PRIV "MS-EVT" `v=1 m=… q=…` without NUL, only the last 8 hex digits evaluated, unknown fields ignored (RT-22); CNAME NUL-terminated with the NUL counted (§2.2.10, RT-20); probe packet, padded pair packet and packet train packets (§2.2.3–2.2.7). `media`: every report is sent as a packet pair (probe + compound, §3.2.6); a report about our stream starts the packet pair fast rate once (250 ms × 40), a positive bandwidth estimate ends it, −6 switches to packet trains (1 s × 21, pair + five equal-size train packets) (§3.2.2–3.2.6, RT-24); audio healer metrics and "MS-EVT" (m = q = 0) in every report (§3.2, RT-27); the TURN server's bandwidth (BANDWIDTH of the Allocate response, turn-08 §8.2, [MS-TURN] §2.2.2.9) in the next five reports (RT-27); policy, TURN, receiver-limit and estimate values of the peer are kept for bandwidth management; LSR/DLSR from the peer's last SR (RFC 3550 §6.4.1). Inference: received probes and pairs are parsed but not used for estimates (§3.2 allows this); because user-space polling cannot time back-to-back packets, we never send −3/−5. Not implemented: video preference (VSR is configured instead). Estimates, peer info, loss and congestion notifications: see 2026-11-01. Status: tested-offline.

Addendum (2026-10-31): the policy server bandwidth extension (type 7) is sent five times once a bandwidth reservation is committed (file 23, RT-27).

## Implementation (2026-11-01, video feedback)
`rtp::feedback` (RT-25, RT-26): standard and extended PLI (request ID, sync-frame bits per priority ID with SFR0 = IDs 0 … 7), VSR with header (AFB type 1, length, MSI or SOURCE_NONE/SOURCE_ANY, request ID, key-frame flag, entry count and size 0x44) and up to 20 entries of 68 bytes, all as reduced-size RTCP ([RFC5506]); `parse_rtcp` reports both as `Rtcp::Feedback`; the retransmission schedule – at once, 4 × 190 ms, then for a VSR 5 × 3 s. The media source SSRC of a VSR is a parameter (O-RT-1). Not yet: DSH (dominant speaker history). Status: tested-offline.

## Implementation (2026-11-01, video core `media::video`)
The video stream of a call, sans-I/O: access units through `h264uc` into SRTP packets (our SSRC = the start of our declared range, SD-64; sequence number 0 never sent, H264PF note <2>), received packets only from the peer's range and payload type after SRTP authentication, depacketized and handed on as access units; a lost access unit sends an extended PLI (RT-25) on the fast schedule (RT-26); a VSR (any source, key frame) on request; the peer's PLI/VSR about our stream become key-frame and source requests; a sender report with our reception report every 5 s. The media source SSRC of our feedback is the peer's stream or, before it sends, the start of its range (inference, O-RT-1). Status: tested-offline.

## Implementation (2026-11-01, bandwidth estimation and rate adaptation)
`media::rate` (RT-28 … RT-30), sans-I/O: the receiver's `Estimator` measures each incoming stream per 1 s interval – received bit rate (with IPv4/UDP headers), loss from the sequence numbers (RFC 3550 §6.4.1 style) and the queueing delay (mean relative one-way delay above the lowest of the last ten intervals). Inference (§3.2.5 leaves the method open; thresholds are design values): before any measurement the estimate is our capacity (our `x-mediabw` receive value) with confidence 2; loss ≥ about 10 % or a queueing delay ≥ 100 ms in two intervals in a row means congestion – the estimate falls to 85 % of what got through; a clean path raises it by 5 % per interval and at least to 1.2 × what it carried, at most the capacity; a sequence number jump above 3000 is a new start (RFC 3550 A.1), the first interval of a stream is not judged for loss (a mixer may start forwarding mid-stream). Audio and video reports carry the estimate (16-octet form) about the peer's stream – for video also before it sends (about the start of its range) –; video reports also carry the receiver-side limit (five times), peer info until the peer reported an estimate for us, the congestion notification on every change and the packet loss notification at once (an early report) at most every 500 ms; each video report goes out as a packet pair with the fast rate of RT-24. The sender's `SendRate` takes the peer's positive estimate for our SSRC and the policy, TURN, receiver-side and video modality limits; a congestion notification takes 30 % off, recovering 5 % per second, never below a quarter; the target is reported to the application when it moved by 10 %; a loss notification asks the encoder for an IDR. The CLI re-encodes accordingly (resolution steps BW-17, a lower bit rate at once, a higher one after 20 % and 2 s; the old encoder is closed before the new one opens; a failed GPU encoder falls back to the CPU; `COMMSCRAFT_VIDEO_HW=0` chooses the CPU). Status: tested-offline.
