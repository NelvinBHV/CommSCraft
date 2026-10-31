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

## Conflicts / gaps
- K-RT-1: Product behaviour <10> (p. 62): SILK and OPUS **not** supported up to and including SfB 2019 (client and server). MS-SDPEXT <4> (p. 76) restricts SILK only up to Lync 2010. Contradiction; a client should plan G.711/G.722 as the safe base set, SILK/OPUS only after an interop test.
- K-RT-2: <13>/<14> (p. 62) "H.263 not supported in Lync 2013/SfB", "H.264 only in Lync 2013/SfB" – the wording leaves open whether later SfB versions are included (the product list names SfB 2016/2019/2021 separately).
- L-06: `[MSFT-H264UCConfig]` (UCConfig mode, VSR) was not in the Microsoft package.

## Gap
- L-05: RFC 3550/3551/4585/5506/5761/4571/4733/3389 were not in the Microsoft package.

## Later findings (file 38)
- L-05 **closed**: RFC 3550/3551/4585/5506/5761/4571/4733/3389 obtained.
- L-06 **closed** via archive search: [MSFT-H264UCConfig] V1.1 obtained; "UCConfig mode 1" in the VSR (RT-25) = SVC temporal scalability with hierarchical P (IE-08). Internal reference use only, therefore not redistributed.
