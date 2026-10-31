# Evidence: MS-SRTP and MS-SSRTP – media encryption

Sources:
- [MS-SRTP] "Secure Real-time Transport Protocol (SRTP) Profile", edition 20250520, 19 pp. – strict subset of RFC 3711.
- [MS-SSRTP] "Scale Secure Real-time Transport Protocol (SSRTP) Extensions", edition 20250520, 24 pp. – for MCU→client (server flavour); the client sends SRTP.

## MS-SRTP

| ID | Requirement | Evidence |
|---|---|---|
| SR-01 | AES-CM-128 (no f8, no NULL), salt 112 bits, prefix 0 | §3.1.3.2–3.1.3.3.1, pp. 11–12 |
| SR-02 | Authentication always, key 160 bits, tag 80 bits | §3.1.3.3.2, p. 12 |
| SR-03 | MKI mandatory, 1 byte; key derivation rate 0; master key 128 bits; lifetime 2^48−1 (RTP) / 2^31−1 (RTCP) | §3.1.3.2, p. 11 |
| SR-04 | Replay list 64 entries | §3.1.1.1, p. 10; §3.1.5.1.2, p. 12 |
| SR-05 | **Two** cryptographic contexts per session (send/receive) for all streams; per SSRC its own ROC, s_l, replay list | §3.1.3.1, pp. 10–11 |
| SR-06 | RTP and RTCP always encrypted+authenticated; **ignore the E bit** of incoming SRTCP; SRTCP index shared by all streams of one direction | §3.1.5.1–3.1.5.2, pp. 12–13 |
| SR-07 | Master keys random, send ≠ receive; start sequence number random but not close to 65535 (server: 0–32767); do not abort on replay hits (DTMF repeats packets) | §5.1, p. 15 |

## MS-SSRTP

| ID | Requirement | Evidence |
|---|---|---|
| SS-01 | Packet: RTP header ‖ encrypted payload (+padding+count) ‖ **ESN 48 bits** ‖ MKI 1 byte ‖ auth tag 10 bytes | §2.2.1, pp. 10–11 |
| SS-02 | Authenticated block (internal only): CSRC ‖ encrypted payload ‖ ESN ‖ zero padding to a 64-byte boundary ‖ RTP header without CSRC ‖ ROC (4 bytes) | §2.2.1.2, pp. 11–12 |
| SS-03 | IV = (k_s·2^16) XOR ((ESN>>16)·2^64) XOR (ESN·2^16) | §3.1.5.1.2, p. 16 |
| SS-04 | Initialise ESN randomly 0…2^47−1, +1 per packet, skip end byte 0; ESN shared by all streams; not used for replay protection | §3.1.3.2, p. 14; §3.1.5.1.1, p. 14; §3.1.5.1.3.1, p. 15 |
| SS-05 | HMAC-SHA1, tag 80 bits; key derivation RFC 3711 §4.3 with kdr=0; RTCP like SRTCP | §3.1.3.3.2–3.1.3.4, p. 14; §3.1.5.2, p. 16 |
| SS-06 | Receiving: estimate index/ROC from ESN + s_l, check replay, check tag, decrypt, remove ESN/MKI/tag | §3.1.5.1.3.2, pp. 16–17 |

### Recomputed (test vectors §4, pp. 17–19)
Own implementation of the RFC 3711 KDF (labels 0–5, kdr = 0), SS-03 (IV) and SS-02 (authenticated block):
- SSRTP enc `C3FCC67B…59CD`, auth `23B8B2D9…5694`, salt `929B3AD0…2C8D`, SSRTCP enc `122E3C94…E0BB`, auth `999BDAC0…BAA6`, salt `839D2707…4E` – **all identical** to the document.
- Ciphertext starts `C1D49FFD5B845AAC…`, ends `…B0A91D520EC9` – identical.
- Authenticated block 208 bytes – identical.
- Tag with **HMAC-SHA1**: `2FA5BAC13AC58423BE4A` – identical; HMAC-SHA256 would give `F59819F2…` (wrong).

These checks become automated tests in M0.

## Conflicts
- **K-SR-1 (documented):** MS-SRTP §3.1.3.2/§3.1.3.3.2 (pp. 11–12) names HMAC-**SHA-256** and claims this is the RFC 3711 default. MS-SSRTP requires parameters identical to MS-SRTP (§3.1.3.2, p. 13) but names HMAC-**SHA1**; MS-SDPEXT allows only `AES_CM_128_HMAC_SHA1_80` (§3.1.5.1, p. 19); the SSRTP test vectors only work with SHA1. A key length of 160 bits only fits SHA1. ⇒ Implementation: **HMAC-SHA1-80**. The SHA-256 statement is a document error.

## Gap
- L-04: RFC 3711 (SRTP) was not in the Microsoft package.

## Later finding (file 38)
- L-04 **closed**: RFC 3711 obtained; key derivation additionally recomputed against RFC 3711 appendix B.3 (IE-09).
