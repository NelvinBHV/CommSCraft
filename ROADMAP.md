# CommSCraft roadmap

**Stage: M1 (sign-in), offline** · M0 done (analysis imported, decisions taken, test vectors)

> **Last reviewed:** 2026-10-27 · **Last updated:** 2026-10-27 · **Change:** minor (M1 started)

CommSCraft aims at parity with the Skype for Business desktop client for Windows when connected to
an on-premises Skype for Business Server. This page is the summary; the assessment is
[`docs/target-app-parity.md`](docs/target-app-parity.md), the work list is
[`docs/gaps.md`](docs/gaps.md), and milestones and the current focus are in
[`docs/roadmap.md`](docs/roadmap.md).

## Headline numbers (2026-10-27)

| Number | Value | Kind |
| --- | --- | --- |
| Client functions identified | **52** | measured (analysis) |
| Evidence status | **B 24 · B\* 17 · T 9 · N 2** | measured (analysis, see legend) |
| Documented conflicts between specification passages | **46** | measured (analysis) |
| Missing sources (all IETF/ITU gaps closed or narrowed) | **18** listed, 9 closed | measured (analysis) |
| Open points still needing an interop capture | **3** of 24 (plus 3 of 18 gaps and 5 conflicts) and 5 checks from M1: 19 owner test cases, [`docs/interop-tests.md`](docs/interop-tests.md)) | measured (analysis) |
| Specification documents reviewed | 865 Microsoft Open Specifications PDFs, 192 RFCs, 20 Internet-Drafts, ITU-T G.722.1/P.562/P.800.1 | measured |
| Program code | 16 crates + 1 app, 138 tests (specification test vectors, examples, hostile-input tests, offline sign-in, TLS loopback); network code only in the `net` adapter, used only by the owner's `commscraft-cli` runs | measured |
| Owner decisions | **all taken** (D-01 … D-10; [`docs/decisions.md`](docs/decisions.md)) | measured |
| Interop test environment | **none** (D-08): all work is `tested-offline` at best | measured |

Legend: **B** fully documented; **B\*** documented, with conflicts or gaps that need a decision or an
interop check; **T** partly documented; **N** not documented (or not implementable without a
licence).

The two **N** functions are RTVideo (VC-1 based, Microsoft licence and active patents) and 1:1 file
transfer (no public specification). Neither blocks the core workflows: video uses H.264 UC, and file
transfer can come later from interop captures.

## What comes next

1. **M0 – foundations** (current): protocol analysis imported into [`docs/protocol/`](docs/protocol/); set up an empty Rust workspace and CI on
   standard runners.
2. **M1 – sign-in and registration**, **M2 – presence and IM**, **M3 – audio 1:1**,
   **M4 – conferences and telephony**, **M5 – video**, **M6 – viewing shared screens**, **M7 – Exchange
   integration**. Details and gates: [`docs/roadmap.md`](docs/roadmap.md).

Hours are not estimated yet; the first estimate follows once M0 has fixed the architecture.

## Progress log

| Date | Change |
|---|---|
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
