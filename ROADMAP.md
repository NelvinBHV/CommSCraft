# CommSCraft roadmap

**Stage: planning** · next: M0 (analysis imported, owner decisions taken), then M1 (sign-in)

> **Last reviewed:** 2026-10-27 · **Last updated:** 2026-10-27 · **Change:** minor (open-point list closed)

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
| Documented conflicts between specification passages | **42** | measured (analysis) |
| Missing sources (all IETF/ITU gaps closed or narrowed) | **18** listed, 9 closed | measured (analysis) |
| Open points still needing an interop capture | **2** of 23 (plus 3 of 18 gaps and 5 conflicts: 13 owner test cases, [`docs/interop-tests.md`](docs/interop-tests.md)) | measured (analysis) |
| Specification documents reviewed | 865 Microsoft Open Specifications PDFs, 192 RFCs, 20 Internet-Drafts, ITU-T G.722.1/P.562/P.800.1 | measured |
| Program code | 8 crates, 55 tests (specification test vectors and examples), no network code | measured |
| Owner decisions | **all taken** (D-01 … D-09; [`docs/decisions.md`](docs/decisions.md)) | measured |
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
| 2026-10-27 | Open-point list closed: a fourth pass over all 865 specifications found normative answers for O-AW-1, O-TU-1, O-IC-1, O-NL-1, O-DL-1, O-RD-2, O-QE-1, O-EX-1/2/4/5 and L-15 (OB-39 … OB-51); compression is not needed (client-initiated only); the rest are 13 owner test cases with defaults (`docs/interop-tests.md`). |
| 2026-10-27 | M0 test vectors complete: `sipcomp` (MPPC, RFC 2118/MS-RDPBCGR examples), `abs` (decompresses the compressed example of MS-ABS §3.2), `psom` (complete ConnMgr frames of MS-PSOM §4.1), `turn-bwm` (XOR examples); 55 tests. New findings K-CP-1, K-AB-4, K-PO-3, O-CP-3; K-AB-1 confirmed by the specification example. |
| 2026-10-26 | Rust workspace started: `ntlm`, `ice`, `srtp`, `sip-identity` with 16 tests from published test vectors (MS-NLMP, RFC 3711, MS-SSRTP, MS-SIPRE, IE-05); CI on `ubuntu-latest`. |
| 2026-10-26 | Protocol analysis imported into `docs/protocol/` (English): 39 evidence files, function matrix, conflict/gap/open-point list (generated), source registers. |
| 2026-10-26 | D-08: no interop test environment for now (offline work only); D-09: CI only on runners without costs. |
| 2026-10-26 | Owner decisions D-01 … D-07 taken (licence MIT OR Apache-2.0, English, pure-Rust protocol core, publication scope, codecs, target servers, egui); licence files added. |
| 2026-10-26 | Repository restarted. Previous history (407 commits up to 2026-10-20) archived outside the repository. Project plan, agent rules and progress docs added. |
| 2026-10-26 | Protocol analysis completed outside the repository: 52 functions mapped to specification sections; IETF/ITU sources gathered; open points checked against official Microsoft documentation, web archives, open-source implementations and patents. |
