# Evidence: IETF foundations and archive sources (closing gaps L-01 … L-18 as far as IETF is concerned)

Status 2026-10-26. Obtained: **all 188 RFCs** cited in the Microsoft documents used, all **17 cited IETF drafts** (in exactly the cited version), additionally draft-ietf-behave-rfc3489bis-08 (referenced by ice-19), 6 additional codec sources (2 SILK drafts, RFC 4425/5577/6716/7587) and, via archive search, the Microsoft specification **[MSFT-H264UCConfig]**. The documents are kept outside the repository; the register with bytes, SHA-256 and source URL is [`../sources-ietf.tsv`](../sources-ietf.tsv). Search order per document: IETF archive `ietf.org/archive/id` → Datatracker → Wayback Machine (`web.archive.org/…id_/…`) → historical mirrors. All drafts were in the IETF archive; the H.264 specification at the original URL named in MS-RTP (also in the Wayback archive, snapshot 20260218140127).

Page references for drafts/RFCs = section number or line in the text version.

## Closure status of the gaps

| Gap | Missing | Now available | Status |
|---|---|---|---|
| L-01 | TURN/STUN drafts | **draft-rosenberg-midcom-turn-08** (= [IETFDRAFT-TURN-08]; correction: not "draft-ietf-behave-turn-08"), draft-ietf-behave-turn-01/-05, draft-ietf-behave-rfc3489bis-02 | closed |
| L-02 | ICE drafts | draft-ietf-mmusic-ice-19, -ice-06, -ice-tcp-00, -ice-tcp-07; additionally rfc3489bis-08 (referenced by ice-19) | closed |
| L-03 | proprietary codecs | **SILK**: draft-vos-silk-02 (bitstream description + C reference code, appendix A) and draft-spittka-silk-payload-format-00; G.722.1 RTP: RFC 5577; Opus: RFC 6716/7587 | **partly**: RTAudio, RTVideo codec, Siren and the G.722.1 codec itself (ITU-T, not IETF) still missing |
| L-04 | RFC 3711 | available | closed |
| L-05 | RFC 3550/3551/4585/5506/5761/4571/4733/3389 | available | closed |
| L-06 | [MSFT-H264UCConfig] | "Unified Communication Specification for H.264 AVC and SVC UCConfig Modes V1.1", 37 pp. | closed (internal reference use only, see below) |
| L-07 | SMPTE 421M | VC-1 RTP: RFC 4425 | **partly**: SMPTE 421M is paid, not IETF |
| L-08 | draft-…-media-control-12 | draft-levin-mmusic-xml-media-control-12 | closed |
| L-09 | conference/SIP RFCs, draft-sipsoap-00 | RFC 4353/4575/4579/3265/4028/3311/2976 + draft-deason-sip-soap-00 | closed |
| L-13 | RFC 3515/3891/3892 (transfer) | available | foundations closed; SfB-specific transfer flow open (interop) |
| L-14 | RFC 3863/4119/5139 | available | closed (IEEE 802.1AB still not, not IETF) |
| L-18 | RFC 4120/4121/3961/4757/2743/4559 | available | closed |
| other | further drafts from MS-SIPRE/SDPEXT/EUMR | draft-ietf-sip-gruu-10, -sip-outbound-11, -levy-sip-diversion-08, -sipping-sip-offeranswer-08, -sip-199-01, -rajesh-sipping-605-01/-303-01 | obtained |

Not IETF and therefore not part of this closure: L-10, L-11, L-12, L-15, L-16, L-17 (Microsoft-internal formats or product behaviour).

## New evidence from the obtained sources

| ID | Statement | Evidence |
|---|---|---|
| IE-01 | MS-TURN corresponds to draft-rosenberg-midcom-turn-08: message types 0x0003/0x0103/0x0113 Allocate, 0x0004/0x0104/0x0114 Send, 0x0115 Data Indication, 0x0006/0x0106/0x0116 Set Active Destination; attributes 0x000D LIFETIME … 0x0015 REALM; **MAGIC-COOKIE 0x72c64bc6**; errors 434–440 (incl. 439 "Transitioning", 3 s) – consistent with TU-10…TU-12 | turn-08 §9.1–9.2.10 |
| IE-02 | **"MESSAGE-INTEGRITY in the old STUN format"** (IC-13, TU-14) = HMAC-SHA1 over the message up to before MI, padded with zeros to a multiple of **64 bytes**; MI is the last attribute | rfc3489bis-02 §10.2.8 |
| IE-03 | Normal FINGERPRINT (mandatory for ICE-19): CRC-32 per ITU V.42 over the message up to before FINGERPRINT, **XOR 0x5354554E**, last attribute | ice-19 §7.1.1; rfc3489bis-08 §14.5 |
| IE-04 | Candidate priority = 2^24·type preference + 2^8·local preference + (256 − component); for TCP local preference = 2^12·transport + 2^9·direction + other (recommended UDP 15, TCP 6; active 5, passive 2, simultaneous-open 7) | ice-19 §4.1.2.1; ice-tcp-07 §3.2 |
| IE-05 | **Recomputed** from SfB example SDPs (MS-ICE2BWM, MS-EUMSDP, MS-SIPAPP): 2130706431 = host(126)/0xFFFF/component 1; 16648703 = relay(0), UDP(15), direction 7; 7076351/1684797951 = TCP(6), **active(5)**, type relay(0) or srflx(100); 6556159 = TCP(6), **passive(2)** – all values explained | own decomposition |
| IE-06 | SIP method **SERVICE** (focus factory, MRAS, QoE, reportError, presence): body usually XML, response with standard SIP codes; the same Call-ID per client recommended, CSeq increasing; a new dialog branch | draft-deason-sip-soap-00 §4 |
| IE-07 | Video control (MS-XMLMC) via SIP INFO with **`Content-Type: application/media_control+xml`** | draft-levin-mmusic-xml-media-control-12 §8 |
| IE-08 | **UCConfig modes**: 0 = H.264 AVC single layer with simulcast; 1 = SVC temporal (hierarchical P); 2q = temporal + SNR; 2s = temporal + spatial (factor 1.5 or 2); 3 = full SVC; higher modes include lower ones; toolsets UC Constrained High (profile_idc 100), UC Scalable Constrained High/Baseline (86/66); Annex B byte stream; 7.5–60 fps dyadic; max. 1080p (16:9), VGA (4:3), Pano288p (20:3); minimum resolutions per mode (table 1, level 3 or 3.1) | MSFT-H264UCConfig §5–§7, pp. 4–36 |
| IE-09 | SRTP key derivation (labels 0/1/2, kdr = 0) recomputed against **RFC 3711 appendix B.3**: cipher key `C61E7A93744F39EE10734AFE3FF7A087`, salt `30CBBC08863D8C85D49DB34A9AE1`, auth key `CEBE321F…` ✓ – independent confirmation of SS-05 | RFC 3711 §4.3, B.3 |
| IE-10 | SILK SDP: `a=rtpmap:<pt> SILK/<clock>`, fmtp `maxaveragebitrate`, `usedtx` (`useinbandfec` from MS-SDPEXT SD-22 is **not** defined in draft -00) | draft-spittka-silk-payload-format-00 §6–7 |

## Remaining or narrowed open points
- **O-IC-1 (narrowed):** The "normal" FINGERPRINT is now documented (IE-03). The table printed in MS-ICE2 is identical to the standard CRC-32 table except **entry 90** (`8BBE8EA` instead of `8BBEB8EA`). **Inference:** if the table were correct, the "additional" packet would be bit-identical to the original and pointless; the printed value 0x08BBE8EA probably reproduces the faulty table of old implementations. Whether it is also XORed with 0x5354554E was later confirmed by a secondary source (file 39, OB-09).
- **SILK:** The reference code is under a BSD-like Skype licence that explicitly excludes patent licences ("NO EXPRESS OR IMPLIED LICENSES TO ANY PARTY'S PATENT RIGHTS", draft-vos-silk-02 appendix A) – check before use (decision D-05).
- **[MSFT-H264UCConfig]**: terms of use "You may copy and use this document for your internal, reference purposes" (p. 1) – internal use only, not redistributed.
- RFCs/drafts are under the terms of the IETF Trust; like all other sources they are kept outside the repository.
