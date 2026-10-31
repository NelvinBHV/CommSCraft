# Evidence: MS-SDPEXT – SDP extensions (media negotiation)

Source: [MS-SDPEXT] "Session Description Protocol (SDP) Version 2.0 Extensions", edition 20250520, 82 pp. Based on RFC 4566, 3264, 4568, 4145, 4571, 4574, 4585, 5761, 3605, 3389, ice-06/ice-19/ice-tcp-07 (§1.2.1, §1.4, pp. 12–16).

## General

| ID | Requirement | Evidence |
|---|---|---|
| SD-01 | SDP in the SIP body with `application/sdp` (or `application/gw-sdp` from gateways); SIP over TLS because keys are in the SDP | §2.1, p. 18 |
| SD-02 | `o=`: ignore sess-id; sess-version numeric but **do not increment** it and ignore it when receiving; nettype IN; IP4/IP6; ignore address changes | §3.1.5.19, p. 33 |
| SD-03 | Formats in m= by preference; the answer keeps the relative order and **the same payload numbers** | §3.1.5.3–3.1.5.4, p. 25 |
| SD-04 | `a=label:main-audio` / `main-video` / `panoramic-video` / `applicationsharing-video`; ignore others | §3.1.5.15, p. 30 |
| SD-05 | `a=rtcp:<port>` with port only | §3.1.5.17, p. 31 |
| SD-06 | Hold: all A/V streams `a=inactive` + Contact `sip.rendering="no"`; resume: `sendrecv`, sip.rendering removed | §3.1.5.23, p. 36 |
| SD-07 | Music on hold: `a=sendonly` + `a=feature:MoH` only for m=audio | §3.1.5.27, pp. 40–41 |
| SD-08 | `a=tty` = text telephony optimisation | §3.1.5.11, p. 28 |

## Encryption

| ID | Requirement | Evidence |
|---|---|---|
| SD-10 | `a=crypto:<tag> AES_CM_128_HMAC_SHA1_80 inline:<key‖salt base64>|2^31|<MKI>:1` – the only suite; lifetime always 2^31 (ignore when receiving); MKI SHOULD, length 1 byte; no session params; exactly one space, no tab | §3.1.5.1, pp. 19–20 |
| SD-11 | **Optional encryption**: offer `RTP/AVP` + a=crypto ⇒ answer `RTP/SAVP` + a=crypto (same tag) if supported, otherwise `RTP/AVP` without crypto; afterwards all re-INVITEs SAVP. Mandatory: offer `RTP/SAVP` | §3.1.5.8, pp. 25–26; examples §4.3, pp. 57–61 |
| SD-12 | `a=cryptoscale:<tag> client|server …` for SSRTP (MCU = server, client = client); result table (e.g. offer SRTP+client SSRTP, answer server SSRTP ⇒ client sends SRTP, receives SSRTP); tags unique across crypto/cryptoscale | §3.1.5.2–3.1.5.2.1, pp. 20–22 |
| SD-13 | **Never** renegotiate encryption parameters via re-INVITE | §3.1.5.2.2, p. 22; §3.1.5.14, p. 30 |
| SD-14 | Policy: `ucPC2PCAVEncryption` (Support/Require/DoNotSupport) decides AVP vs. SAVP (relationship is an inference from SIPREGE §2.2.2.5.1) | SIPREGE p. 26 |

## Codecs / payload types

| ID | Codec | SDP | Evidence |
|---|---|---|---|
| SD-20 | RTAudio | `x-msrta/16000` bitrate=29000; `x-msrta/8000` bitrate=11800 | §3.1.5.3, p. 23 |
| SD-21 | G.722 stereo | `G722/8000/2` bitrate=128000 (real rate 16 kHz) – from Lync 2013 | p. 24; <3> p. 76 |
| SD-22 | SILK | `SILK/16000`, `SILK/8000`, fmtp `usedtx=0; useinbandfec=0|1` – from Lync 2013 | p. 24; <4> p. 76 |
| SD-23 | RED | `RED/8000`, name case-sensitive "RED", no fmtp in the offer | p. 23; §3.1.5.6, p. 25 |
| SD-24 | Comfort noise | `CN/8000`, `CN/16000` | §3.1.5.7, p. 25 |
| SD-25 | DTMF | `telephone-event/8000`, PT **101**, fmtp `0-16` | §3.1.5.5, p. 25; example p. 55 |
| SD-26 | RTVideo | `x-rtvc1/90000` (PT 121 with older servers), fmtp e.g. `CIF=15;VGA=15;PANO=15` | p. 23; <6> p. 76; example p. 40 |
| SD-27 | H.264UC | `X-H264UC/90000`, fmtp **must** be `packetization-mode=1;mst-mode=NI-TC`, plus `a=x-ssrc-range` (length ≥30), otherwise ignore – from Lync 2013 | pp. 23–24; <2> p. 76 |
| SD-28 | ULPFEC-UC | `x-ulpfecuc/90000`; whoever supports H.264UC SHOULD support ULPFEC-UC too | pp. 24–25 |
| SD-29 | RTData (application sharing) | `x-data/90000` | p. 23 |
| SD-30 | Additionally offered in the examples: G722/8000, SIREN/16000, G7221/16000 (bitrate=24000), AAL2-G726-32/8000, PCMA, PCMU, `a=ptime:20`, `a=maxptime:200` | examples pp. 40, 52–55 |

## ICE mapping in the SDP

| ID | Requirement | Evidence |
|---|---|---|
| SD-40 | ICE-06 format: `a=candidate:<id> <comp> <pwd> UDP|TCP <prio 0..1> <ip> <port>`, IPv4 only; **the ICE re-INVITE (final offer) always comes from the caller**, also for streams added by the callee; no candidate updates before completion | §3.1.5.20, pp. 33–34; example p. 52 |
| SD-41 | ICE-19 format: `a=ice-ufrag`, `a=ice-pwd`, `a=candidate:<f> <comp> UDP|TCP-ACT|TCP-PASS <prio> <ip> <port> typ host|srflx|relay [raddr rport]`, final: `a=remote-candidates:` | §3.1.5.21, pp. 34–36; examples pp. 54–56 |
| SD-42 | IPv6 candidates towards unknown peers as `a=x-candidate-ipv6` | §3.1.5.21.1, pp. 34–35 |
| SD-43 | No ICE lite, no `ice-options`, no `ice-mismatch` | §3.1.5.21.2–3.1.5.21.4, p. 35 |
| SD-44 | Base64 detection of ufrag/pwd by length: in the offer do **not** use ufrag=6 and pwd=32 bytes in plain text | §3.1.5.21.5, pp. 35–36 |
| SD-45 | Audio/video: default candidate never TCP; no passive TCP host candidates; first offer/answer never a TCP m= line (reject) | §3.1.5.9, pp. 26–27; §3.1.5.22, p. 36 |
| SD-46 | `a=setup` only active/passive (special case: initial offer active ⇒ answer active too); `a=connection:existing` (new only for m=applicationsharing) | §3.1.5.10, pp. 27–28 |
| SD-47 | RTP/RTCP mux `a=rtcp-mux` only with ICE-19 (from SfB 2015 server) | §3.1.5.38, p. 51; <66> p. 79 |

## Early media

| ID | Requirement | Evidence |
|---|---|---|
| SD-50 | Only with ICE; all 18x responses identical; with forking (`Ms-Forking`) start receiving, send only DTMF after the first received packet; speech only after 200 | §3.1.5.12, pp. 28–29 |
| SD-51 | 100rel: never `Require: 100rel` in requests; 183 with `Require: 100rel` and SDP for early connectivity checks; only one offer/answer before acceptance; no SDP in PRACK; 199 cleans up early media | §3.1.5.13, pp. 29–30 |

## Video, bandwidth and conference attributes

| ID | Attribute | Evidence |
|---|---|---|
| SD-60 | `a=x-caps:<pt> id:w:h:fps:maxbr:…;…` (QCIF/CIF/VGA mandatory for main-video; HD720, HD1080; missing ⇒ CIF15+VGA15); not for H.264UC/ULPFEC | §3.1.5.24, pp. 36–38 |
| SD-61 | `a=x-mediabw:<label> send=<kbps>;recv=<kbps>` (session level; main-audio is ignored) | §3.1.5.28, pp. 41–42; <55> p. 79 |
| SD-62 | `a=x-devicecaps:audio:send,recv;video:send,recv` (session level, UI hint e.g. camera icon) | §3.1.5.29, pp. 42–43 |
| SD-63 | `a=rtcp-fb:* x-message app send:… recv:…` with dsh/src/x-pli + mandatory `a=rtcp-rsize` | §3.1.5.30, pp. 43–45 |
| SD-64 | `a=x-ssrc-range:a-b` (audio size 1, video 100; non-overlapping; immutable) | §3.1.5.31, pp. 45–46 |
| SD-65 | `a=x-source-streamid` (MSI, assigned by the MCU), `a=x-source:<name>` (own source, e.g. camera) | §3.1.5.32–3.1.5.33, pp. 46–47 |
| SD-66 | Multiplexed video channels (same addresses/ports/ufrag/pwd, candidates only in the first m= line, own SSRC ranges, same label); multi-channel main video in conferences; the offer additionally as multipart with a variant without multiplexing | §3.1.5.34–3.1.5.35, pp. 47–49 |
| SD-67 | `a=x-candidate-info:<foundation> network-type=WLAN|WWAN` | §3.1.5.36, pp. 49–50 |
| SD-68 | Panorama: `a=x-sourceid:PanoramicCamera` + `a=label:panoramic-video` | §3.1.5.37, pp. 50–51 |
| SD-69 | Media bypass to gateways: `a=x-bypassid`, `a=x-bypass`, `a=x-mediasettings:holdrtcpunsupported|rtcpunsupported|signalboostunsupported`, multipart with `application/GW-SDP` | §3.1.5.25, pp. 38–39; examples §4.6, pp. 63–66 |
| SD-70 | `a=x-ms-SDP-diagnostics:53000|53001;reason=…` (bandwidth policy) | §3.1.5.26, pp. 39–40 |

## Application sharing (m=applicationsharing)

| ID | Requirement | Evidence |
|---|---|---|
| SD-80 | `m=applicationsharing <port> TCP/RTP/SAVP|AVP <pt>` with `x-data/90000`; TCP candidates only (ICE2 over TCP), default TCP, no early media | §3.1.5.18, p. 31; §3.1.5.9, p. 27; §3.1.5.22, p. 36 |
| SD-81 | `a=x-applicationsharing-session-id`, `-role:sharer|viewer` (answer reversed), `-media-type:rdp`, `a=mid:<n>`, `a=x-applicationsharing-contentflow:sendonly|recvonly|inactive` | §3.1.5.18.1–3.1.5.18.5, pp. 31–32 |

## Conflicts / gaps

- K-SD-1: Example §4.5.1 (p. 61) describes the offerer as "viewer", but the SDP shows `role:sharer` and `contentflow:sendonly`; the answer shows `viewer`. Prose and SDP contradict each other. Normative: role table §3.1.5.18.2.
- K-SD-2: The attribute name in the ABNF §3.1.5.18.5 is `a=x-application-contentflow`, heading/examples use `a=x-applicationsharing-contentflow` (pp. 32, 62).
- **L-03 (essential):** The bitstream formats of the proprietary codecs **RTAudio (x-msrta)**, **RTVideo (x-rtvc1)**, **SIREN**, **G.722.1 (G7221)** are not described in the Microsoft package (MS-RTVPF only describes the RTP packetisation). Interoperably implementable without further sources: G.711 (PCMU/PCMA), G.722, SILK (public specification outside the package), H.264UC (H.264/SVC, MS-H264PF), DTMF, CN, RED.

## Later findings (files 38, 39)
- L-03 **partly closed**: SILK via draft-vos-silk-02 (with reference code, without patent licence) and draft-spittka-silk-payload-format-00 (IE-10; `useinbandfec` not defined there); G.722.1 RTP RFC 5577; Opus RFC 6716/7587.
- References RFC 3264/4566/4568/4574/3262 and draft-ietf-sipping-sip-offeranswer-08/-sip-199-01 obtained.
- OB-20/OB-26: ITU-T G.722.1 including Annex C (Siren14) is freely available from the ITU with reference code and test vectors (package obtained).
- OB-27: RTAudio – Microsoft's official overview contains no bitstream and states that a licence from Microsoft is needed to implement it.
- OB-34: Siren7 **secondary evidence** (libsiren): 2-bit rate code + 4-bit checksum per 20 ms frame; SDP `SIREN/16000 bitrate=16000`.
- L-03 **closed by decision** D-05 (OB-51): RTAudio, RTVideo and Siren are not implemented; G.722.1 only after legal clearance.
