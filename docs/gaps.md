# Where CommSCraft falls short of the Skype for Business client

> **Last reviewed:** 2026-10-26 · **Last updated:** 2026-10-26 · **Change:** major (first version)

The work list, ranked by how much each item blocks the alpha gate in
[`roadmap.md`](roadmap.md#alpha-gate-core-workflows) (**A** = blocks alpha). Each entry says what is
missing, the evidence, and which document tracks it. Agents: pick from the top, check open PRs
first, and update this file when a gap closes.

At this stage the whole client is missing; the list below covers what stands between the analysis
and the first milestones, followed by the open points found in the analysis.

## Blocking (A)

| # | Gap | Evidence / tracking | Milestone |
|---|---|---|---|
| 1 | No interop test environment (D-08): nothing can be verified against a real server; open points needing captures stay open | [`decisions.md`](decisions.md) | after M0 |
| 2 | Protocol analysis not yet in the repository | analysis outside the repo (52 functions, 39 conflicts, 18 gaps, 22 open points) | M0 |
| 3 | No workspace, no CI, no tests | – | M0 |
| 4 | Sign-in stack: discovery, TLS, NTLM/Kerberos/TLS-DSK, registration, provisioning | functions 1–8 | M1 |
| 5 | Presence and IM | functions 9–19 | M2 |
| 6 | Media stack: MS-ICE2/MS-TURN, SRTP, RTP/RTCP, G.711/G.722 | functions 21–26 | M3 |
| 7 | Conferences (C3P/CCCP, focus, MCUs) | functions 40–42 | M4 |
| 8 | Viewing shared screens (RDP over RTP) | function 43 | M6 |

## Open points from the analysis (need an interop capture by the owner)

| ID | Open point | Status after research |
|---|---|---|
| O-CP-1 | Byte order of the 16-bit "Uncompressed size" in [MS-SIPCOMP] | open (compression is optional; can stay off) |
| O-IC-1 | Duplicate MS-ICE2 connectivity check with the misprinted CRC table | confirmed by a secondary source; interop |
| O-AW-1 | WS-Security header layout when presenting a web ticket | secondary source; WSDL policy is normative |
| O-NL-1 | NTLM flags negotiated by current servers | recomputed for OCS 2007 R2; interop for 2015+ |
| O-CB-1 | Joining a meeting via meet URL from a non-Windows client | secondary source; interop |
| O-TU-1 | Undocumented MS-TURN attribute 0x8006 seen in captures | open; must be ignored safely |
| O-RD-1/2 | Pre-connection PDU and graphics codecs in desktop sharing | narrowed; RemoteFX accepted by Lync 2013 viewer (secondary) |
| O-EX-1…5 | Exchange: free/busy, OOF, conversation history, voicemail list, authentication | partly official, partly secondary |
| O-QE-1 | How the client computes its MOS estimates for QoE reports | ITU basics available; method undocumented |
| L-11, L-16 | Typing indicator body, persistent chat transport | secondary sources; interop |
| L-13 | Call transfer details specific to Skype for Business | official UCMA description; SfB headers via interop |

## Not planned without an owner decision

| Function | Reason |
|---|---|
| RTAudio | Bitstream not public; Microsoft states a licence is required |
| RTVideo | VC-1 standard is paid; frame-caching patent active (until ~2027 per Google Patents) |
| G.722.1 / Siren | ITU reference code by Polycom; licence terms to be cleared |
| SILK | Reference code excludes patent licences |
| File transfer 1:1 | No public specification; only secondary sources |
