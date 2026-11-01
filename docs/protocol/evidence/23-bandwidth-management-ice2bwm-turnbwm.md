# Evidence: bandwidth management (call admission control) – MS-ICE2BWM, MS-TURNBWM

Sources: [MS-ICE2BWM] "ICE Extensions 2.0: Bandwidth Management Extensions", edition 20250520, 32 pp.; [MS-TURNBWM] "TURN Bandwidth Management Extensions", edition 20250520, 49 pp. They extend MS-ICE2 (file 13) and MS-TURN (file 12).

## ICE side (MS-ICE2BWM)

| ID | Requirement | Evidence |
|---|---|---|
| BW-01 | The bandwidth-managing endpoint is usually the **callee** (knows the peer's candidates); for ICE→non-ICE the caller; independent per media stream; the application can configure "report only" | §1.3, pp. 11–12; §3.1.4, p. 15 |
| BW-02 | Check during candidate gathering: LSA = one local host candidate, RSA = one host candidate of the peer, RRA = relay candidate of the peer; with IPv4+IPv6 use IPv4; send to the UDP and TCP policy servers, the first valid response counts | §3.1.4.8.1.1, p. 17 |
| BW-03 | All paths blocked ⇒ the session fails (no offer/answer); local relay blocked ⇒ do not offer the relay candidate; do not check blocked pairs | §3.1.4.1–3.1.4.2, pp. 15–16; §3.1.4.8.2.1, p. 18 |
| BW-04 | New STUN error codes: **274 Disable Candidate** (all pairs of the local candidate ⇒ Failed), **275 Disable Candidate Pair** (this pair ⇒ Failed); the remote candidate is identified via CANDIDATE-IDENTIFIER | §2.2.2, p. 14; §3.1.5.2.1–3.1.5.2.2, pp. 19–20 |
| BW-05 | After the checks a **commit** (timer 500 ms until the response; LSA/LRA/RSA/RRA according to the chosen pair), then an **update** every ≤19 s (on bandwidth change every second until a response); the UDP policy server is preferred unless media run over a TCP relay; reservation ID null ⇒ no further messages | §3.1.4.8.2–3.1.4.8.2.3, pp. 17–18; §3.1.5.1.2, p. 19; §3.1.6.1–3.1.6.2, p. 20 |
| BW-06 | Messages ≤ 1500 bytes | §2.1, p. 14 |

## TURN side (MS-TURNBWM)

| ID | Requirement | Evidence |
|---|---|---|
| BW-10 | All BAC attributes in **authenticated Allocate requests/responses** (MS-TURN), SHOULD in the first authenticated Allocate | §3.1, p. 25 |
| BW-11 | Attributes (TLV): 0x8056 BAC Message (reserved 2 B + type: 0 check, 1 commit, 2 update); 0x8057 Reservation Id (16 B; all 0 ⇒ not managed); 0x8058 Reservation Amount (4×4 B kbit/s: min/max send, min/max receive); 0x8059 Remote Site, 0x805A Remote Relay Site, 0x805B Local Site, 0x805C Local Relay Site (reserved, family 1/2, X-Port, X-IP); 0x805D–0x8060 responses (V flag, for site responses additionally F = PSTN failover; max. send/receive); 0x8061 SIP Dialog Id, 0x8062 SIP Call Id (≤256 B); 0x8068 Location Profile (peer, self: 0 unknown/1 internet/2 intranet; federation 0/1/2) | §2.2.1–2.2.14, pp. 13–24 |
| BW-12 | X-Port = port XOR top 16 bits of the transaction ID; X-IP IPv4 = address XOR top 32 bits; IPv6 XOR the 128-bit transaction ID | §2.2.4, pp. 15–16 |
| BW-13 | Check: BAC=Check, RSA, RRA if any, LRA if any (only if already allocated), LSA, Amount, SHOULD MS-Service-Quality (otherwise audio), SHOULD SIP Call Id, MUST Location Profile | §3.2.4.1, p. 26 |
| BW-14 | Commit: BAC=Commit, RSA, LSA, RRA/LRA only with a used relay, Amount, SHOULD Service-Quality and SIP Dialog Id, MUST Location Profile; update: BAC=Update, Reservation Id, Amount on change (0 ⇒ cancel) | §3.2.4.2–3.2.4.3, pp. 26–27; §2.2.3, p. 14 |
| BW-15 | Evaluation: path LSA↔RSA only if both V=1; F=1 ⇒ PSTN rerouting allowed; relay paths analogously; do not use invalid paths | §3.2.5.1, p. 28 |
| BW-16 | Client update timer: update at least every **60 s** (otherwise the server discards the reservation) | §3.2.2, pp. 25–26; §3.3.2, p. 29 |
| BW-17 | Rate adaptation of Skype for Business endpoints (official documentation, not a protocol rule): media endpoints "can adapt to varying network conditions"; per H.264 resolution a maximum and a minimum payload bit rate, the minimum being "the bit rate below which a Skype for Business Server endpoint will switch to the next lower resolution" (e.g. 1280x720 2500/700, 640x480 and 640x360 800/300, 424x240 and 320x240 350/100, 320x180 250/15 kbit/s); in conferences the sender decides its streams "based on CPU capability, available uplink bandwidth, and the number of receiving clients requesting a certain video stream"; aggregated video at most 8000 kbit/s; FEC is used dynamically on packet loss | [SFB-NETREQ] (Microsoft Learn, section "Network bandwidth requirements for media traffic", retrieved 2026-11-01) |

### Recomputed
The XOR examples of §2.2.4 (pp. 15–16) are correct: 0x1122⊕0x4455 = 0x5577; 0x11223344⊕0xaabbccdd = 0xbb99ff99; IPv6 example = 0x31233efc444444cccccccc44444444cc ✓.

## Conflicts / gaps
- K-BW-1 (not a real contradiction): ICE2BWM requires updates ≤19 s, TURNBWM only ≤60 s – implementation: 19 s.
- K-BW-2: Example ICE2BWM §4 (p. 27): the answer to the final offer contains `m=audio 52714 5 RTP/SAVP` and `a=remote-candidates:1 101.0.57 52732` – the example is wrong; in addition the profile changes from RTP/AVP to RTP/SAVP between initial and final offer.
- O-BW-1: How the client learns address/port of the bandwidth policy server is described only abstractly ("configured", ICE2BWM §1.5, p. 12). Only the switches `enableBWPolicyCheck` (ServerConfiguration, MS-SIPREGE, file 04) and `enableBWPolicyOverride` (ucPolicy) are documented.

## Later finding (file 39)
- O-BW-1 **closed** (OB-10, MS-TURNBWM §1.3, pp. 9–10): the bandwidth check runs in the Allocate request to the client's own TURN server ⇒ the policy server is the TURN server obtained via MRAS.
- The three XOR examples of MS-TURNBWM §2.2.4 (p. 16) are automated tests in `crates/turn-bwm` (2026-10-27).

## Implementation (2026-10-31, building blocks)
`turn-bwm`: Check, Commit and Update attributes in the order and with the lengths of §2.2/§3.2.4 (site addresses XOR-coded with the request's transaction ID, BW-12; SIP call/dialog IDs ≤ 256 octets; Location Profile always in Check and Commit, BW-13/BW-14); response evaluation per §3.2.5.1–3.2.5.3 (required site responses, V/F flags – F only in the remote and local site responses, the bit is reserved in the relay ones –, path rules incl. full check failure and PSTN failover, reservation ID all zero = not managed). `turn`: a bandwidth request is an authenticated Allocate with MS-Service-Quality on the existing allocation (§3.1), one at a time; the authenticated answer is reported with its attributes, errors and timeouts separately from the allocation (BW-10). `ice-agent`: refused sites (local/remote site and relay site, §3.1.5.1.1) fail their pairs, which are never checked; requests on them get signed 274/275 (§3.1.5.2.1); 274/275 received fail the candidate's pairs or the pair (§3.1.5.2.2; BW-03, BW-04). Status: tested-offline.

## Implementation (2026-10-31, in calls)
`call-media` (module `bwm`), switched on only when provisioning enables `enableBWPolicyCheck` (file 04): the callee is the managing endpoint (BW-01) and sends the check while gathering, once per policy server (the first UDP relay and the TCP relay), with one IPv4 host candidate of ours and of the caller, the caller's relayed candidate and our relay (BW-02, BW-13); the first valid answer counts, gathering waits for it at most 3 s (design); a refused local relay is not offered, a policy without any path ends the call before the answer is sent (BW-03), the other restrictions go to the ICE agent; after ICE selection the selected path is committed every 500 ms until answered (sites per §3.1.4.8.2.2; the UDP policy server unless media runs over the TCP relay), then updated every 19 s (K-BW-1: ≤ 19 s), a null reservation ID ends all messages (BW-05, BW-14, BW-16); towards a peer without ICE the caller commits and updates (§3.1.4.4.2); at the end an update with amount 0 cancels the reservation; the reserved send bandwidth goes to the peer as the MS-RTP policy server bandwidth extension (five reports, RT-27). CLI: Call-ID and offer of the ringing call, federation from the SIP domains. Inferences: amount 85 kbit/s (G.722/G.711 20 ms with RTP, SRTP, UDP, IPv4 headers), locations "unknown", a peer outside our SIP domain counts as enterprise federation, the policy extension carries the reserved maximum send bandwidth, commits are given up after 120 attempts. Not implemented: video streams (M5), PSTN failover routing (M4), SIP dialog ID (SHOULD), "report only" configuration. Status: tested-offline (UDP test TURN server: check with refused relay, blocked policy, commit, update, cancel).


## Implementation (2026-11-01, rate adaptation)
`media::rate`: the resolution steps of BW-17 (`STEPS`, `step_of`). The CLI picks, within a video source request, the largest size whose minimum bit rate the target reaches and keeps the bit rate below the size's maximum; the target is the smaller of the request and what the path allows (RT-30). Inference: our sizes are those of the table up to VGA (the camera's size); 160x120 takes the rates of 320x180. Status: tested-offline.
