# Decisions

> **Last reviewed:** 2026-10-26 · **Last updated:** 2026-10-26 · **Change:** major (first version)

Decisions that belong to the project owner. Agents propose, the owner decides; a decision is
recorded here with date and reason before work depends on it.

## Open

| ID | Question | Proposal | Why it matters |
|---|---|---|---|
| D-01 | Licence | `MIT OR Apache-2.0`, as the storytold Crafting Apps | Determines which libraries may be linked (Apache-2.0 is incompatible with GPL-2.0-only) |
| D-02 | Language of repository documents | English (as storytold); the existing German analysis is translated during import | Contributors and reviewers |
| D-03 | Dependency policy: pure Rust only, or reviewed C libraries via FFI (libnice, FreeRDP, GStreamer, libsrtp, GSSAPI/Kerberos) | Pure Rust for protocol logic; allow reviewed system libraries for Kerberos (GSSAPI), media I/O and RDP decoding behind adapter crates | Effort vs. maturity; `unsafe` policy in `AGENTS.md` |
| D-04 | Which parts of the analysis are published | Evidence files, function matrix, conflict/gap list and source registers (without PDFs); patent notes kept as "understanding only" or left out | Specification packages say "no redistribution"; only our own descriptions with citations are published |
| D-05 | Codec scope | G.711, G.722, H.264 UC (via a licensed or system H.264 implementation); no RTAudio/RTVideo; G.722.1 and SILK only after legal clearance | Patent and licence risk |
| D-06 | Target servers | Skype for Business Server 2015, 2019 and Subscription Edition, on-premises; Lync Server 2013 best effort | Test matrix |
| D-07 | User interface toolkit | egui, as the storytold apps | Consistency with the Crafting Apps |
| D-08 | Interop test environment | Owner-run lab server; captures sanitized before they are committed | Only source of `verified-interoperable` |

## Taken

| ID | Date | Decision | Reason |
|---|---|---|---|
| T-01 | 2026-10-26 | Repository restarted; the previous history is archived outside the repository | Owner decision |
| T-02 | 2026-10-26 | Work on branches and pull requests; no direct pushes or merges to `main` by agents | Owner rule |
| T-03 | 2026-10-26 | Interop tests against real servers are run only by the owner | Owner rule |
