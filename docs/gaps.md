# Where CommSCraft falls short of the Skype for Business client

> **Last reviewed:** 2026-10-27 · **Last updated:** 2026-10-27 · **Change:** minor (M1 building blocks)

The work list, ranked by how much each item blocks the alpha gate in
[`roadmap.md`](roadmap.md#alpha-gate-core-workflows) (**A** = blocks alpha). Each entry says what is
missing, the evidence, and which document tracks it. Agents: pick from the top, check open PRs
first, and update this file when a gap closes.

At this stage the whole client is missing; the list below covers what stands between the analysis
and the first milestones, followed by the open points found in the analysis.

## Blocking (A)

| # | Gap | Evidence / tracking | Milestone |
|---|---|---|---|
| 1 | No interop test environment (D-08): nothing can be verified against a real server; the remaining questions are owner test cases IT-01 … IT-17 | [`decisions.md`](decisions.md), [`interop-tests.md`](interop-tests.md) | after M0 |
| 2 | ~~Protocol analysis not yet in the repository~~ – imported | [`protocol/`](protocol/) | done |
| 3 | ~~Remaining M0 test vectors (SIPCOMP/MPPC, ABS, PSOM, TURNBWM XOR)~~ – done | [`roadmap.md`](roadmap.md) | done |
| 4 | Sign-in stack: discovery, TLS, NTLM/Kerberos/TLS-DSK, registration, provisioning. Done offline: SIP parser/framing, MS-SIPAE signing and verification (NTLM), NTLM handshake messages, REGISTER flow (sign-in, refresh, sign-out, deregister NOTIFY), in-band provisioning, discovery (Autodiscover HTTPS-only, DNS SRV), registrar redirect. Missing: Kerberos/TLS-DSK, discovery, TLS adapter | functions 1–8 | M1 |
| 5 | Presence and IM | functions 9–19 | M2 |
| 6 | Media stack: MS-ICE2/MS-TURN, SRTP, RTP/RTCP, G.711/G.722 | functions 21–26 | M3 |
| 7 | Conferences (C3P/CCCP, focus, MCUs) | functions 40–42 | M4 |
| 8 | Viewing shared screens (RDP over RTP) | function 43 | M6 |

## Open points from the analysis – closed (2026-10-27)

Full list with status: [`protocol/conflicts-gaps-open-points.md`](protocol/conflicts-gaps-open-points.md);
evidence of the closing pass: [`protocol/evidence/39-evidence-for-open-points.md`](protocol/evidence/39-evidence-for-open-points.md)
(OB-39 … OB-51).

No point needs more document research. Of 23 open points, 21 are closed by a normative source, by
design or as not needed; of 18 gaps, 15 are closed or no evidence problem. What remains is server or
Windows-client behaviour that only a capture can show. These are the owner test cases in
[`interop-tests.md`](interop-tests.md). Each one has a default, so none blocks offline work.

| Closed how | Points |
|---|---|
| Normative source found (fourth pass) | O-AW-1 (OB-39), O-TU-1 (OB-41), O-IC-1 (OB-42), O-NL-1 (OB-43), O-DL-1 + K-DL-1 (OB-44), O-RD-2 (OB-45), O-QE-1 (OB-47), O-EX-1/2/4/5 + L-15 (OB-48) |
| Not needed by the client | O-CP-1, O-CP-3 (OB-40: compression is only negotiated by the client; CommSCraft does not) |
| By design or owner decision | O-EX-3 (OB-49), L-03 and L-07 (D-05, OB-51) |
| Earlier passes | O-SIP-1, O-PR-1, O-AD-1, O-AD-2, O-CP-2, O-PV-3, O-BW-1, L-01 … L-10, L-12, L-14, L-17, L-18 |

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

## Not planned without an owner decision

| Function | Reason |
|---|---|
| RTAudio | Bitstream not public; Microsoft states a licence is required |
| RTVideo | VC-1 standard is paid; frame-caching patent active (until ~2027 per Google Patents) |
| G.722.1 / Siren | ITU reference code by Polycom; licence terms to be cleared |
| SILK | Reference code excludes patent licences |
| File transfer 1:1 | No public specification; only secondary sources |
