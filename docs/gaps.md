# Where CommSCraft falls short of the Skype for Business client

> **Last reviewed:** 2026-11-01 · **Last updated:** 2026-11-01 · **Change:** minor (M5: camera and H.264 codec exist)

The work list, ranked by how much each item blocks the alpha gate in
[`roadmap.md`](roadmap.md#alpha-gate-core-workflows) (**A** = blocks alpha). Each entry says what is
missing, the evidence, and which document tracks it. Agents: pick from the top, check open PRs
first, and update this file when a gap closes.

State of the code (2026-10-31): sign-in, presence, IM (1:1 and group), address book search and
audio calls over UDP exist as tested-offline building blocks and are usable in `commscraft-cli
interactive`; nothing is verified against a real server yet (owner test cases). The list below says
what is still missing per milestone, followed by the open points found in the analysis.

## Blocking (A)

| # | Gap | Evidence / tracking | Milestone |
|---|---|---|---|
| 1 | No published interop results (D-08): lab runs stay private, nothing is marked verified; the remaining questions are owner test cases IT-01 … IT-33 | [`decisions.md`](decisions.md), [`interop-tests.md`](interop-tests.md) | all |
| 2 | Sign-in: Kerberos (function 5) and TLS-DSK with client certificate (function 6) are missing; NTLM, discovery, TLS, registration, provisioning and sign-out exist (tested-offline) | functions 1–8, 50 | M1 |
| 3 | Presence and IM: contact list editing (add/remove contacts and groups), privacy relationships and blocking (12), offline address book (13), contact photos (15), Exchange unified contact store (11); presence, 1:1 and group IM, typing and address book search exist | functions 9–19 | M2 |
| 4 | Audio 1:1: TCP-only mode and the artificial active TCP server-reflexive candidate (MS-ICE2 §3.1.4.8.1.3, SHOULD), streaming our own music on hold (35, optional), the peer info exchange of MS-RTP (needs link bandwidths), bandwidth management for video and PSTN failover (32, M4/M5), redundant audio (MS-RTPRADEX, 30), as the callee a reliable 183 with an early answer (SD-51, SHOULD) and early media of forked calls (SD-50, SHOULD); calls with ICE over UDP and the TCP relay (443), TURN, SRTP, G.722/G.711 and DTMF exist | functions 21–26, 30, 32, 35 | M3 |
| 5 | Conferences and telephony: recording, PSTN dial-in details of meetings, PSTN media bypass, the rest of E911 (WLAN/LLDP identifiers); scheduling meetings (create, list, change, cancel; presenters, invitees and options; the static meeting), joining meetings (address or link), lobby, conference audio (SRTP sent, SRTP/SSRTP received), muting, announcements and their language, dialling phone numbers into meetings, the presenter commands with meeting options in the running meeting, PSTN calling, call transfer, forwarding, call park, quality reports, emergency calls with location and team/delegate ringing, delegates and calls on behalf of the boss exist | functions 33–34, 36–37, 39–42, 45 | M4 |
| 6 | Video H.264 UC: hardware encoding (VA-API), video SDP, video source requests, 1:1 and conference video, simulcast, FEC, picture control (31); the RTP payload of one layer (`h264uc`), camera capture and H.264 encoding/decoding with libx264 (`video-ffmpeg`) exist | functions 28, 31 | M5 |
| 7 | Viewing shared screens (RDP over RTP) | function 43 | M6 |
| 8 | Graphical user interface (egui, D-07); today only the command-line tool exists | [`decisions.md`](decisions.md) | not yet in a milestone |

## Open points from the analysis – closed (2026-10-31)

Full list with status: [`protocol/conflicts-gaps-open-points.md`](protocol/conflicts-gaps-open-points.md);
evidence of the closing pass: [`protocol/evidence/39-evidence-for-open-points.md`](protocol/evidence/39-evidence-for-open-points.md)
(OB-39 … OB-51).

No point needs more document research. Of 25 open points, 22 are closed by a normative source, by
design or as not needed; of 19 gaps, 16 are closed or no evidence problem (counted from the
generated list, 2026-10-31). What remains is server or
Windows-client behaviour that only a capture can show. These are the owner test cases in
[`interop-tests.md`](interop-tests.md). Each one has a default, so none blocks offline work.

| Closed how | Points |
|---|---|
| Normative source found (fourth pass) | O-AW-1 (OB-39), O-TU-1 (OB-41), O-IC-1 (OB-42), O-NL-1 (OB-43), O-DL-1 + K-DL-1 (OB-44), O-RD-2 (OB-45), O-QE-1 (OB-47), O-EX-1/2/4/5 + L-15 (OB-48) |
| Not needed by the client | O-CP-1, O-CP-3 (OB-40: compression is only negotiated by the client; CommSCraft does not) |
| By design or owner decision | O-EX-3 (OB-49), L-03 and L-07 (D-05, OB-51) |
| Earlier passes | O-SIP-1, O-PR-1, O-AD-1, O-AD-2, O-CP-2, O-PV-3, O-BW-1, L-01 … L-10, L-12, L-14, L-17, L-18 |
| Sources obtained during development | L-19 (ITU-T G.711/G.722 incl. digital test sequences, 2026-10-28) |

| Interop test | Point | Default until tested |
|---|---|---|
| IT-01 | O-CB-1 Join Manager with a non-Windows `User-Agent` | Browser page, focus URI from the HTML |
| IT-02 | O-RD-1 pre-connection PDU for desktop sharing | None, configurable |
| IT-03 | O-RD-2 sharer towards a viewer without RemoteFX | Advertise only implemented codecs |
| IT-04 | L-11 typing indicator body | Secondary format (OB-16) |
| IT-05 | L-16 persistent chat INVITE/INFO | Secondary values (OB-17) |
| IT-06 | L-13 SfB transfer headers | RFC flow plus documented MS headers |
| IT-07 … IT-11 | Conflicts K-RE-1, K-CA-1, K-CS-2, K-E9-1, K-CK-1 | Decision recorded in the conflict |
| IT-12, IT-13 | O-CP-1 (optional), O-EX-3 (optional) | Compression off; own layout |
| IT-14 | NTLM flags offered by current servers (OB-43) | Refuse weaker offers |
| IT-15 | Client version policy and the CommSCraft User-Agent (RG-50) | Report the block to the user |
| IT-16 | SIP access points in the unauthenticated Autodiscover Domain resource; HTTPS on lyncdiscover (D-10) | DNS SRV |
| IT-17 | Registrar redirect to the home pool (O-RG-1) | Signed TLS redirect only |
| IT-18 | TLS 1.2 handshake with the pool (SfB 2015 CU9+/2019 CU1+) | No TLS 1.0/1.1 |
| IT-19 | First complete sign-in with `commscraft-cli` | – |
| IT-20 | K-CI-3 invitation INVITE (`application/ms-conf-invite+xml`) of the Windows client | Reported only; join with `/join N` |
| IT-21 | Group IM against the conferencing server | – |
| IT-22 | O-IC-2 length rule for STUN-02 checks with MI and FINGERPRINT | RFC 5389 rule |
| IT-23 | First audio call (inside and through the Edge) | Encryption as provisioned, no plaintext fallback |
| IT-24 | Audio call with UDP blocked (TCP relay on 443) | UDP first, TCP relay as additional candidates |
| IT-25 | Hold and resume in both directions | No audio sent while held |
| IT-26 | Long call (≥ 5 min) with RTCP reports | Peer and server accept our MS-RTP reports |
| IT-27 | Address book search with NTLM channel bindings | Bound to the TLS connection (issue #42) |
| IT-28 | Call with early media (announcement or gateway) | No speech sent before the answer; PRACK for reliable 183 |
| IT-29 | Calls with call admission control enabled | Refused paths not used; reservation cancelled at the end |
| IT-30 | Join a meeting with audio (link, lobby, mute) | Meeting link without credentials; nothing joined without the command |
| IT-31 | PSTN calls (out with a local number, in from a gateway, call back) | Only checked numbers and profile names in SIP URIs |
| IT-32 | Call transfer (blind, with consultation, being transferred) | REFER only in a call dialog; Replaces only for our own calls |
| IT-33 | Call forwarding and simultaneous ring | Only checked targets published; private category |

## Not planned without an owner decision

| Function | Reason |
|---|---|
| RTAudio | Bitstream not public; Microsoft states a licence is required |
| RTVideo | VC-1 standard is paid; frame-caching patent active (until ~2027 per Google Patents) |
| G.722.1 / Siren | ITU reference code by Polycom; licence terms to be cleared |
| SILK | Reference code excludes patent licences |
| File transfer 1:1 | No public specification; only secondary sources |
