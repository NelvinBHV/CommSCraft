# CommSCraft roadmap

**Stage: M3 (audio 1:1)** · M0 done · M1 and M2 built and tested offline · M3 in progress

> **Last reviewed:** 2026-10-31 · **Last updated:** 2026-10-31 · **Change:** minor (headline numbers and next steps synchronised with the code)

CommSCraft aims at parity with the Skype for Business desktop client for Windows when connected to
an on-premises Skype for Business Server. This page is the summary; the assessment is
[`docs/target-app-parity.md`](docs/target-app-parity.md), the work list is
[`docs/gaps.md`](docs/gaps.md), and milestones and the current focus are in
[`docs/roadmap.md`](docs/roadmap.md).

## Headline numbers (2026-10-31)

| Number | Value | Kind |
| --- | --- | --- |
| Client functions identified | **52** | measured (analysis) |
| Evidence status | **B 24 · B\* 17 · T 9 · N 2** | measured (analysis, see legend) |
| Implementation status of the 52 functions | **10** tested-offline as a whole, **11** in parts, **29** not started, 2 not planned/needs captures | measured ([`docs/target-app-parity.md`](docs/target-app-parity.md)) |
| Conflicts / gaps / open points in the generated list | **53** / **19** (16 closed or no evidence problem) / **25** (22 closed or not needed) | measured ([`docs/protocol/conflicts-gaps-open-points.md`](docs/protocol/conflicts-gaps-open-points.md)) |
| Owner test cases | **29** (IT-01 … IT-29, [`docs/interop-tests.md`](docs/interop-tests.md)) | measured |
| Specification documents reviewed | 865 Microsoft Open Specifications PDFs, 193 RFCs, 20 Internet-Drafts, ITU-T G.711, G.722 (incl. digital test sequences), G.722.1, P.562, P.800.1 | measured |
| Program code | **32 crates + 1 app** (`commscraft-cli`), **274 tests** (specification test vectors and examples, ITU/RFC/libsrtp reference data, hostile-input tests, offline sign-in, TLS and media loopback tests); network and device code only in adapter crates (`net`, `call-media`, `audio-alsa`) | measured |
| Owner decisions | **all taken** (D-01 … D-11; [`docs/decisions.md`](docs/decisions.md)) | measured |
| Interop test environment | **none** (D-08): all work is `tested-offline` at best | measured |

Legend: **B** fully documented; **B\*** documented, with conflicts or gaps that need a decision or an
interop check; **T** partly documented; **N** not documented (or not implementable without a
licence).

The two **N** functions are RTVideo (VC-1 based, Microsoft licence and active patents) and 1:1 file
transfer (no public specification). Neither blocks the core workflows: video uses H.264 UC, and file
transfer can come later from interop captures.

## What comes next

1. **M3 – audio 1:1** (built): the owner's first calls (IT-23 … IT-29); remaining optional
   parts in [`docs/gaps.md`](docs/gaps.md).
2. **M4 – conferences and telephony** (next).
3. **M5 – video**, **M6 – viewing shared screens**,
   **M7 – Exchange integration**; the graphical interface (egui, D-07) is not yet in a milestone.
   Details and gates: [`docs/roadmap.md`](docs/roadmap.md).

Hours are not estimated.

## Progress log

| Date | Change |
|---|---|
| 2026-10-31 | Bandwidth management in calls: the callee checks while gathering (refused relay not offered, blocked policy ends the call, restrictions to ICE), commit every 500 ms until answered, updates every 19 s, cancel at the end; caller towards a peer without ICE commits; policy bandwidth to the peer in RTCP; CLI with `enableBWPolicyCheck`; interop test IT-29. |
| 2026-10-31 | Bandwidth management building blocks (MS-TURNBWM/MS-ICE2BWM): Check/Commit/Update attributes and their responses with the path rules (`turn-bwm`), bandwidth requests as authenticated Allocates on the TURN allocation (`turn`), refused sites and 274/275 in the ICE agent; feature matrix 10/11/29. |
| 2026-10-31 | Early media as the caller (MS-SDPEXT §3.1.5.12–3.1.5.13): `Supported: 100rel`/`ms-early-media`, early dialogs, PRACK with RAck in RSeq order, early answer with ICE starts media (receive, DTMF only after reception, no speech before 200), 199 ends it, the 200 continues it; 420 for an INVITE requiring 100rel; interop test IT-28. |
| 2026-10-31 | Security audit issue #42 closed: strict HTTP framing (a kept-alive chunked response used to leave its final CRLF behind), CLI output filtered for control and bidi characters, NTLM over HTTP bound to the TLS connection (`tls-server-end-point`, RFC 4121/5929), no ICE checks or media to loopback/link-local peers, one bounded way into the TCP relay buffer, terminal echo restored on every path; interop test IT-27. |
| 2026-10-31 | MS-RTP RTCP profile: all profile extensions, SDES "MS-EVT", probe/pair/train packets and their sending rates, audio healer metrics, TURN bandwidth (from the Allocate response) to the peer, LSR/DLSR, NUL-terminated CNAME; interop test IT-26. |
| 2026-10-31 | Hold and resume (MS-SDPEXT §3.1.5.23): re-INVITE with inactive/sendrecv and `+sip.rendering="no"` (encoding researched: RFC 4235 §5.2, RFC 3840 §9), incoming hold and music on hold recognised, no RTP while held; CLI `/hold` `/resume`; interop test IT-25. |
| 2026-10-31 | TCP relay through the Edge on 443 (MS-ICE2 regular mode): pseudo-TLS, MS-TURN over TCP, active/passive relayed candidates, ice-tcp-07 pairing, RFC 4571 framing; tested against a TURN test server; interop test IT-24. |
| 2026-10-31 | Status documents synchronised with the code (owner rule): README (stage, what works, build and run), headline numbers measured (32 crates + 1 app, 242 tests, 10/10/30 functions), feature matrix rows and totals, gap list per milestone incl. IT-20 … IT-23, current focus and alpha gate. |
| 2026-10-31 | CLI `interactive` calls: `/call`, `/answer`, `/hangup`, `/dtmf`, `/mute`; one media thread per call with ALSA capture/playback threads and bounded queues; final offer after ICE. Interop test IT-23. |
| 2026-10-31 | Call media runtime (`call-media`): UDP sockets, TURN relays, ICE, audio core; loopback test with real sockets: ICE selection, final offer, encrypted G.722, DTMF, RTCP BYE. |
| 2026-10-31 | Audio media core (`media`): codec, RTP, SRTP, jitter buffer with concealment, DTMF, RTCP, packet classification (RFC 5761 §4, STUN-02 §10.1). |
| 2026-10-31 | Call signalling in the engine: multipart offer (MS-SIPRE §3.15), ringing, answer, final offer with remote-candidates, CANCEL, BYE, incoming calls with accept/decline/timeout. |
| 2026-10-31 | ICE agent (`ice-agent`, MS-ICE2 over UDP): checks in both STUN formats, nomination, keep-alive, consent freshness; K-IC-2 (FINGERPRINT in responses). |
| 2026-10-31 | Media relay credentials (`avedgea`, MS-AVEDGEA) fetched by the engine at bootstrap from `mrasUri` and refreshed at 90 % of the lifetime. |
| 2026-10-31 | MS-TURN client (`turn`): allocation with digest challenge and SHA-256, sequence numbers, Send/Data Indication, active destination, refresh, teardown, TCP framing, pseudo-TLS. |
| 2026-10-31 | Owner rule T-02 changed: PRs are merged into `main` regularly once CI is green; stack #1 … #27 merged. STUN codec (`stun`) for RFC 5389 and STUN-02/MS-TURN, all RFC 5769 vectors verified; O-IC-2 / IT-22. |
| 2026-10-28 | SDP for audio (`sdp`, MS-SDPEXT): offer/answer, a=crypto, ICE-19 attributes; checked against the §4.1.3 offer; K-SD-3 (truncated keys in the §4.3 answer examples). |
| 2026-10-28 | SRTP/SRTCP contexts (MS-SRTP profile; identical to RFC 3711 B.2 and the libsrtp reference packets) and `rtp` (RTP/RTCP, reception statistics, DTMF per RFC 4733/MS-DTMF). |
| 2026-10-28 | Decision D-11 (owner): audio through ALSA/PipeWire. Adapter `audio-alsa` (ALSA API; PipeWire via its ALSA plugin) and the local device check `commscraft-cli audio-check` (tone, recording, playback through G.722 or G.711, no network). |
| 2026-10-28 | M3 started: codecs G.711 (`g711`, generated from the decision values, Tables 1–6 incl. all conversion rows) and G.722 (`g722`, ADPCM bit-exact against all 14 ITU comparison files, QMF round trip); gap L-19 closed by obtaining ITU-T G.711/G.722. |
| 2026-10-28 | Group IM (`conf`, MS-CONFPRO/CONFBAS/CONFIM): ad-hoc conference, focus join, roster, IM MCU session, dial-out invitations, 202 + IMDN, Ms-Sender, RFC 4028 session timers, leave; invitations of other clients reported (secondary evidence, K-CI-3, IT-20); CLI `/group`, `/invite`, `/join`. |
| 2026-10-27 | Address book search (`dlx`, MS-DLX SearchAbEntry): SOAP request, bounded answer parser with SIP URI check; connection-oriented NTLM over HTTP (MS-NTHT, MIC over NEGOTIATE ‖ CHALLENGE ‖ AUTHENTICATE) on one kept-alive TLS connection; the session engine fetches provisioning; CLI `/search`. |
| 2026-10-27 | CLI `interactive`: stays signed in on top of the session engine – live presence of the contacts, own state, several conversations (`/contacts`, `/status`, `/chat`, `/to`, `/end`, `/quit`). |
| 2026-10-27 | Session engine (`session`, L4): bootstrap with endpoint publication, long-lived contact list and presence subscriptions (NOTIFY, refresh, resubscribe), incoming and outgoing IM conversations, timeouts; UAS dialogs in `sip-msg`; the test registrar of `register` is available to other crates (feature `testing`). |
| 2026-10-27 | 1:1 IM: `im` crate (message-session SDP, ACK, MESSAGE, typing INFO, peer requests; plain text only) and CLI command `im`. |
| 2026-10-27 | Presence of contacts on other pools: `resubscribe` answers are followed to the pool GRUU (`poolFqdn`) or asked one by one (MS-PRES §3.4.5.1.3.1). |
| 2026-10-27 | SIP dialogs (RFC 3261 §12: route set, in-dialog requests, peer requests) in `sip-msg`; the contact list is now fetched with a real subscription (piggyback or NOTIFY) and the subscription is ended again. |
| 2026-10-27 | M2 started: `presence` crate (contact list, one-time batch presence queries, own state publication scoped to the endpoint, 409 retry), CLI command `presence` with signed request/answer handling; REGISTER announces `msrtc-event-categories` and `adhoclist`. New conflict K-PR-1. |
| 2026-10-27 | Fix: MS-SIPAE version 4 – the request with the final NTLM token is now signed (§3.2.5.1 step 6.8, `cnum` 1); answered version capped at 4; test registrar enforces it. |
| 2026-10-27 | NTLM diagnostics for interop runs: non-secret facts of the handshake with `--verbose` (offered/chosen flags, target-info names, MS-SIPAE version), auth headers shown with their non-secret parameters, `--ntlm-legacy` (NTLMv2 without the optional MIC/channel-binding additions). |
| 2026-10-27 | Trust model for SIP servers named by Autodiscover in another DNS domain (`--trust-domain`, interactive question, remembered; like `TrustModelData`, [SFB-TRUST]); SRV records are printed. |
| 2026-10-27 | Discovery in `commscraft-cli`: HTTPS GET (`net::http`, strict parser, loopback test) and DNS SRV (`net::dns`, ID/question checks, bounded parser, mutation test); `--discover-only` runs discovery without a password. |
| 2026-10-27 | M1 ready for the owner's first run: keep-alive negotiation (K-CON-1), `net` adapter (rustls, TLS 1.2/1.3, system trust store + enterprise CA, loopback tests with a throw-away CA), `commscraft-cli sign-in` (requires `--authorized`). D-10 taken. Research: TLS 1.2 support needs SfB 2015 CU9 / 2019 CU1 ([SFB-TLS]); IT-18, IT-19. |
| 2026-10-27 | Discovery: `discovery` (Autodiscover over HTTPS only via the unauthenticated Domain resource, ≤ 10 redirects, loop detection; DNS SRV/fallback ordering, TLS only, domain filter) and `url`; signed registrar redirect (O-RG-1, researched in secondary sources). New: K-AD-1, O-RG-1, decision proposal D-10, IT-16/IT-17. |
| 2026-10-27 | Provisioning and server NOTIFYs: bounded `xml` reader (no DTD/entities), `provisioning` (SUBSCRIBE, parser for the three group shapes of MS-SIPREGE §4.2.2, ServerConfiguration with HTTPS/domain/port checks), deregister and survivable mode NOTIFY in `register`. New conflict K-RG-4 (malformed examples, inconsistent element names). |
| 2026-10-27 | Offline sign-in: `register` state machine (REGISTER → NTLM → signed 200 OK → GRUU → refresh → sign-out) against a test registrar; strict base64; client version block (RG-50). Findings: the MS-SIPAE signature does not cover Contact/GRUU; the MS-SIPREGE example uses NTLMv1. New interop tests IT-14, IT-15. |
| 2026-10-27 | NTLM handshake messages: CHALLENGE parsing with bounds checks, AUTHENTICATE construction (reproduces MS-NLMP §4.2.4.3 byte for byte), MIC/channel bindings/no LM response per §3.1.5.2.1, flag policy that refuses anything weaker than NTLMv2 + ESS + 128-bit + key exchange. |
| 2026-10-27 | M1 started: `sip-msg` (bounded SIP parser, stream framing, injection-safe serializer, mutation test) and `sipae` (MS-SIPAE signature buffers recomputed from the examples §4.1/§4.2, NTLM signing/verification with seq 100, replay window of 256, version ≥ 3 policy). New conflict K-AE-4 (version 4 example). Security rules in `docs/security.md`. |
| 2026-10-27 | Open-point list closed: a fourth pass over all 865 specifications found normative answers for O-AW-1, O-TU-1, O-IC-1, O-NL-1, O-DL-1, O-RD-2, O-QE-1, O-EX-1/2/4/5 and L-15 (OB-39 … OB-51); compression is not needed (client-initiated only); the rest are 13 owner test cases with defaults (`docs/interop-tests.md`). |
| 2026-10-27 | M0 test vectors complete: `sipcomp` (MPPC, RFC 2118/MS-RDPBCGR examples), `abs` (decompresses the compressed example of MS-ABS §3.2), `psom` (complete ConnMgr frames of MS-PSOM §4.1), `turn-bwm` (XOR examples); 55 tests. New findings K-CP-1, K-AB-4, K-PO-3, O-CP-3; K-AB-1 confirmed by the specification example. |
| 2026-10-26 | Rust workspace started: `ntlm`, `ice`, `srtp`, `sip-identity` with 16 tests from published test vectors (MS-NLMP, RFC 3711, MS-SSRTP, MS-SIPRE, IE-05); CI on `ubuntu-latest`. |
| 2026-10-26 | Protocol analysis imported into `docs/protocol/` (English): 39 evidence files, function matrix, conflict/gap/open-point list (generated), source registers. |
| 2026-10-26 | D-08: no interop test environment for now (offline work only); D-09: CI only on runners without costs. |
| 2026-10-26 | Owner decisions D-01 … D-07 taken (licence MIT OR Apache-2.0, English, pure-Rust protocol core, publication scope, codecs, target servers, egui); licence files added. |
| 2026-10-26 | Repository restarted. Previous history (407 commits up to 2026-10-20) archived outside the repository. Project plan, agent rules and progress docs added. |
| 2026-10-26 | Protocol analysis completed outside the repository: 52 functions mapped to specification sections; IETF/ITU sources gathered; open points checked against official Microsoft documentation, web archives, open-source implementations and patents. |
