# Decisions

> **Last reviewed:** 2026-10-26 · **Last updated:** 2026-11-01 · **Change:** minor (D-13: VA-API with one bounded unsafe module)

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
| D-08 | 2026-10-26, changed 2026-10-31 | Development works offline first: specification test vectors, specification examples and self-built fixtures. Since 2026-10-31 the owner operates an isolated lab, and the agent may run tests against that lab on its own. Nothing about the lab is published (hosts, addresses, captures, logs, results); notes stay outside the repository. No connections to any other real server; those tests remain the owner's. Public statuses stay at `tested-offline`; `verified-interoperable` stays reserved for the owner's runs on authorized servers | Owner decision; changed by the owner ("Du darfst wenn die Verbindungen eingerichtet sind, alles selbständig testen … nicht vom Lab in github veröffentlichen") |
| D-09 | 2026-10-26 | CI only on runners that cause no costs: standard GitHub-hosted runners in this public repository. No larger, custom or self-hosted runners, no paid external CI services. If the repository ever becomes private, no workflow runs without new owner approval (private repositories consume paid minutes) | Owner rule |
| D-10 | 2026-10-27 | Autodiscover over HTTPS only, deviating from the MUST in MS-OCDISCWS §3.2.1.1 to also try `http://` (switch `allow_plain_http` stays off) | Plain HTTP would send the SIP URI unencrypted and accept spoofable redirects; owner followed the proposal |
| D-11 | 2026-10-28 | Audio devices on Linux through ALSA/PipeWire: the adapter crate `audio-alsa` uses the ALSA API (crate `alsa` 0.9.1, links libasound); on PipeWire systems the ALSA device `default` is served by PipeWire's ALSA plugin. No other crate links audio libraries | Owner decision; one backend covers ALSA and PipeWire; safe Rust API, `unsafe_code` stays forbidden in CommSCraft code (D-03) |
| D-12 | 2026-11-01 | Video (M5) through the system's FFmpeg libraries: one adapter crate links libavcodec/libavutil/libswscale (H.264 encoding and decoding; the encoder is chosen at run time from what the system offers, e.g. `h264_vaapi`, `libx264`) and libavdevice (camera capture over V4L2). CommSCraft ships no codec and no FFmpeg; it is linked dynamically. A binary built against an FFmpeg configured with `--enable-gpl` is under the GPL as a whole, one built against `--enable-nonfree` must not be redistributed – the project makes no releases (T-02); a future release needs a new owner decision. No other crate links media libraries. The FFmpeg binding is `ffmpeg-next` 9 (safe API); it needs an FFmpeg without the SAND pixel formats of SoC patch sets (Gentoo: USE `-soc`), otherwise it does not compile. Hardware encoding (VA-API) needs FFmpeg's hardware frame context, which the safe binding does not offer | Owner decision; settles the open H.264 implementation of D-05; safe Rust API, `unsafe_code` stays forbidden in CommSCraft code (D-03) |
| D-13 | 2026-11-01 | Hardware video encoding (VA-API) with one exception to D-03: the crate `video-ffmpeg` denies instead of forbids `unsafe_code`, and exactly one module (`hw`) allows it – the VA-API device, the surface pool, attaching it to the encoder and uploading pictures through FFmpeg's hardware context (`ffmpeg-next`'s raw `ffi`). Rules: every `unsafe` block carries a SAFETY comment (clippy `undocumented_unsafe_blocks` denied), pointers come only from FFmpeg's allocation functions and are checked for null, reference-counted buffers are released exactly once, return codes are checked first, no pointer leaves the module. The encoder tries the GPU first and falls back to libx264 if no device works; video is never lost for lack of hardware. All other CommSCraft code keeps `unsafe_code` forbidden | Owner decision; the Windows client also encodes in hardware where available (to be confirmed by evidence); measured: about a quarter of the CPU time of libx264 at 1280×720 |
| T-01 | 2026-10-26 | Repository restarted; the previous history is archived outside the repository | Owner decision |
| T-02 | 2026-10-26, changed 2026-10-31 | Work on branches and pull requests; no direct pushes to `main`. Since 2026-10-31 the agent merges its own pull requests into `main` regularly once CI is green (merge commits, history kept). No releases | Owner rule; changed by the owner ("Merge aber immer wieder alles zusammen") |
| T-04 | 2026-11-01 | Milestone M4 (conferences and telephony) is closed by owner decision with the functions of its scope implemented and `tested-offline`; the gate's "verified" is not claimed publicly (D-08). Remaining optional parts stay in `docs/gaps.md` | Owner decision |
| T-03 | 2026-10-26 | Interop tests against real servers are run only by the owner | Owner rule |
