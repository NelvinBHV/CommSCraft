# Implementation order (derived from the evidence dependencies)

The milestones in [`../roadmap.md`](../roadmap.md) follow this order. Each stage ends offline (`tested-offline`, decision D-08); interop tests are run by the owner, lab runs stay private (D-08).

| Stage | Content | Basis (evidence IDs) | Checkable without a server by |
|---|---|---|---|
| 0 | Test basis: test vectors as automated tests (EPID→UUID, NTLMv2, SIP NTLM signature, SSRTP, MPPC/SIPCOMP, ABS compression, PSOM encoding, TURNBWM XOR, ICE CRC table, ICE priorities) | K-RE-1, NL-*, OB-11, SS-*, CP-*, AB-*, PO-*, BW-12, IC-14, IE-05 | the documents' test vectors (already recomputed) |
| 1 | Connection & sign-in: Autodiscover/DNS SRV, TLS, keep-alive, REGISTER, NTLM/Kerberos SA with message signing, GRUU, provisioning | AD-*, CON-*, AE-*, RG-*, RE-01…10 | specification examples; captures later |
| 2 | Presence & contacts (server contact list), location profile, error display | PR-*, SIP-*, RE-40…42, ER-* | specification examples |
| 3 | Chat 1:1 and IM conference | OCS-UC09, CB-*, CI-* | specification examples |
| 4 | Media base: MRAS, MS-TURN, MS-ICE2, SDP, SRTP, RTP/RTCP with G.711/G.722, DTMF | AV-*, TU-*, IC-*, SD-*, SR-*, RT-*, PL-30 | packet level against document examples |
| 5 | Telephony: PSTN dialling, hold, forwarding preamble, park, voicemail, emergency location, QoE | PS-*, SD-06/07, RE-50…54, CK-*, UM-*, E9-*, QE-* | specification examples |
| 6 | Video H.264UC/SVC, VSR | PL-01…07, RT-25…27 | specification examples |
| 7 | Schedule/join conferences, A/V conference, lobby, roles, mute | PV-*, CB-*, CA-* | specification examples |
| 8 | Address book/search/photos/UCS via Exchange | AB-*, DL-*, EX-* | specification examples |
| 9 | Screen sharing (viewer first) via RDP in RTP | CS-*, PL-40…42, RD-* | specification examples |
| 10 | Optional: PSOM content, persistent chat, response groups, bandwidth management, compression, TLS-DSK | PO-*, XC-*, WS-01, BW-*, CP-*, AE-26/27, AW-* | specification examples |

Decided before stage 1 (see [`../decisions.md`](../decisions.md)): licence (D-01), dependency policy for TLS, Kerberos/GSS-API, SRTP, H.264 and RDP (D-03), codec scope without RTAudio/RTVideo (D-05).
