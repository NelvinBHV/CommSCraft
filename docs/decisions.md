# Decisions

> **Last reviewed:** 2026-10-26 · **Last updated:** 2026-10-26 · **Change:** major (D-01 … D-07 taken; D-08 open)

Decisions that belong to the project owner. Agents propose, the owner decides; a decision is
recorded here with date and reason before work depends on it.

## Open

| ID | Question | Proposal | Why it matters |
|---|---|---|---|
| D-08 | Interop test environment | Owner-run lab server; captures sanitized before they are committed | Only source of `verified-interoperable` |

## Taken

| ID | Date | Decision | Reason |
|---|---|---|---|
| D-01 | 2026-10-26 | Licence: `MIT OR Apache-2.0` at the user's option (`LICENSE-MIT`, `LICENSE-APACHE`) | Same as the storytold Crafting Apps; permits linking Apache-2.0 libraries |
| D-02 | 2026-10-26 | Repository documents are written in English; the German analysis is translated during import | Contributors and reviewers |
| D-03 | 2026-10-26 | Protocol logic in pure Rust; reviewed system/C libraries (Kerberos/GSSAPI, media I/O, RDP decoding) only behind adapter crates | Maturity where reimplementing is risky; `unsafe` stays out of protocol crates |
| D-04 | 2026-10-26 | Published: evidence files, function matrix, conflict/gap/open-point list and source registers, without PDFs or downloaded third-party material. Patent notes are kept, labelled "understanding only, not evidence" | Only our own descriptions with citations are redistributed |
| D-05 | 2026-10-26 | Codecs: G.711, G.722, H.264 UC (licensed or system H.264 implementation). No RTAudio, no RTVideo. G.722.1 and SILK only after legal clearance | Patent and licence risk |
| D-06 | 2026-10-26 | Target servers: Skype for Business Server 2015, 2019 and Subscription Edition, on-premises; Lync Server 2013 best effort | Test matrix |
| D-07 | 2026-10-26 | User interface toolkit: egui | Consistency with the Crafting Apps |
| T-01 | 2026-10-26 | Repository restarted; the previous history is archived outside the repository | Owner decision |
| T-02 | 2026-10-26 | Work on branches and pull requests; no direct pushes or merges to `main` by agents | Owner rule |
| T-03 | 2026-10-26 | Interop tests against real servers are run only by the owner | Owner rule |
