# Decisions

> **Last reviewed:** 2026-10-26 · **Last updated:** 2026-10-28 · **Change:** minor (D-11 taken)

Decisions that belong to the project owner. Agents propose, the owner decides; a decision is
recorded here with date and reason before work depends on it.

## Open

| ID | Question | Proposal | Why it matters |
|---|---|---|---|
| – | – | – | – |

## Taken

| ID | Date | Decision | Reason |
|---|---|---|---|
| D-01 | 2026-10-26 | Licence: `MIT OR Apache-2.0` at the user's option (`LICENSE-MIT`, `LICENSE-APACHE`) | Same as the storytold Crafting Apps; permits linking Apache-2.0 libraries |
| D-02 | 2026-10-26 | Repository documents are written in English; the German analysis is translated during import | Contributors and reviewers |
| D-03 | 2026-10-26 | Protocol logic in pure Rust; reviewed system/C libraries (Kerberos/GSSAPI, media I/O, RDP decoding) only behind adapter crates | Maturity where reimplementing is risky; `unsafe` stays out of protocol crates |
| D-04 | 2026-10-26 | Published: evidence files, function matrix, conflict/gap/open-point list and source registers, without PDFs or downloaded third-party material. Patent notes are **not** published | Only our own descriptions with citations are redistributed |
| D-05 | 2026-10-26 | Codecs: G.711, G.722, H.264 UC (licensed or system H.264 implementation). No RTAudio, no RTVideo. G.722.1 and SILK only after legal clearance | Patent and licence risk |
| D-06 | 2026-10-26 | Target servers: Skype for Business Server 2015, 2019 and Subscription Edition, on-premises; Lync Server 2013 best effort | Test matrix |
| D-07 | 2026-10-26 | User interface toolkit: egui | Consistency with the Crafting Apps |
| D-08 | 2026-10-26 | No interop test environment for now. Work proceeds offline only: specification test vectors, specification examples and self-built fixtures. No status can reach `verified-interoperable` until the owner provides an environment; milestone gates end at `tested-offline` | Owner decision |
| D-09 | 2026-10-26 | CI only on runners that cause no costs: standard GitHub-hosted runners in this public repository. No larger, custom or self-hosted runners, no paid external CI services. If the repository ever becomes private, no workflow runs without new owner approval (private repositories consume paid minutes) | Owner rule |
| D-10 | 2026-10-27 | Autodiscover over HTTPS only, deviating from the MUST in MS-OCDISCWS §3.2.1.1 to also try `http://` (switch `allow_plain_http` stays off) | Plain HTTP would send the SIP URI unencrypted and accept spoofable redirects; owner followed the proposal |
| D-11 | 2026-10-28 | Audio devices on Linux through ALSA/PipeWire: the adapter crate `audio-alsa` uses the ALSA API (crate `alsa` 0.9.1, links libasound); on PipeWire systems the ALSA device `default` is served by PipeWire's ALSA plugin. No other crate links audio libraries | Owner decision; one backend covers ALSA and PipeWire; safe Rust API, `unsafe_code` stays forbidden in CommSCraft code (D-03) |
| T-01 | 2026-10-26 | Repository restarted; the previous history is archived outside the repository | Owner decision |
| T-02 | 2026-10-26 | Work on branches and pull requests; no direct pushes or merges to `main` by agents | Owner rule |
| T-03 | 2026-10-26 | Interop tests against real servers are run only by the owner | Owner rule |
