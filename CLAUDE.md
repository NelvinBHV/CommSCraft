# CommSCraft — instructions for Claude (same rules as AGENTS.md)

CommSCraft is a clean-room, open-source Linux client for Skype for Business Server, built from
Microsoft Open Specifications, IETF RFCs and ITU-T Recommendations. Conventions follow the storytold
Crafting Apps (WordCraft, GridCraft, DeckCraft).

## Start every session here
1. Read [`ROADMAP.md`](ROADMAP.md) (stage, progress log), then **Current focus** in
   [`docs/roadmap.md`](docs/roadmap.md) and the matching entry in [`docs/gaps.md`](docs/gaps.md).
   A local, gitignored `plan/` directory may hold longer notes; it is never committed.
2. Check open PRs (`gh pr list`) before starting overlapping work.
3. Stop and ask the owner only for owner decisions: licensing, publishing, pushing to new remotes,
   secrets, anything listed as open in [`docs/decisions.md`](docs/decisions.md), and any test that
   touches a real server outside the owner's lab (D-08).

## Evidence first
- Every protocol behaviour is backed by a citation: document, edition/version, section, PDF page
  (e.g. `[MS-SIPAE] 20250520 §3.3.4.1, p. 33`). Code that implements it carries the evidence ID.
- Keep four kinds of statement apart and label them: **documented** (normative source),
  **recomputed** (checked against a published test vector), **inference** (our reasoning) and
  **open** (needs an interop capture). Never invent missing protocol details.
- Source ranking: (1) Microsoft Open Specifications, IETF, ITU-T; (2) official Microsoft
  documentation (Microsoft Learn, archived SDK docs); (3) secondary sources (open-source
  implementations, observation notes) — confirm only, never copy; (4) patents — understanding only,
  never evidence, and patent notes are not committed (decision D-04).
- Every normative requirement maps to source path, test fixture and executed test result.
  Published test vectors (MS-NLMP, MS-SRTP, …) become automated tests.
- Distinguish `source-only`, `tested-offline` and `verified-interoperable` status. Only the owner's
  interop runs on authorized servers can grant `verified-interoperable`.

## Clean room
- Never read, disassemble, decompile or copy anything from Microsoft client or server binaries.
  Never commit files, captures or screenshots produced by Microsoft software unless the owner made
  them on an authorized system and sanitized them.
- Never commit specification PDFs or other third-party documents; cite them.
- Never copy code from GPL/LGPL/AGPL projects (e.g. SIPE) or from projects without a licence.
  Reading them to confirm behaviour is allowed; cite file and line as secondary evidence.
- No Microsoft iconography, images, sounds, fonts or other assets. Every asset is original or
  openly licensed and gets a row in `ATTRIBUTION.md` in the same commit.
- Codecs: do not implement RTAudio or RTVideo (Microsoft licence required, active patents). Patent
  texts may be read for understanding only; nothing from them is implemented without the owner's
  legal clearance.

## Security
Security by design: threat model and the binding rules per crate are in
[`docs/security.md`](docs/security.md); every new parser or protocol state machine follows them.

- Treat all SIP, SDP, XML, HTTP, STUN/TURN, RTP and RDP input as hostile: bound memory and
  recursion, checked arithmetic, fail closed. XML parsers never resolve external entities.
- Never disable TLS certificate validation, never accept a silent plaintext downgrade, never log
  passwords, Kerberos tickets, NTLM hashes, session keys or SRTP keys.
- A parsed `200 OK`, an offline signature check or an acquired credential handle is not an
  authenticated sign-in.
- Validate Autodiscover redirects against the user's SIP domain before sending credentials.
- Network tests run only with explicit owner authorization and produce sanitized evidence.

## Never crash (once code exists)
- No panics in non-test code: no `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`,
  `unimplemented!`; `unsafe_code = "forbid"` unless an owner decision allows a reviewed FFI crate.
- Errors are `Result<T, E>`; unfinished features return an error or are disabled.
- Every crash fix lands with a regression test.

## Architecture
See [`docs/architecture.md`](docs/architecture.md). Protocol state machines never depend on
sockets, the OS or the UI; transport and OS adapters are separate crates.

## Quality gates and CI
- Before every commit (once code exists): `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `cargo test`.
- **Never use runners or services that cause costs** (decision D-09): only standard GitHub-hosted
  runners (`ubuntu-latest`, `windows-latest`) in this public repository. No larger, custom or
  self-hosted runners, no paid external CI. If the repository becomes private, run no workflow
  without new owner approval.
- Interop testing (decision D-08, changed 2026-10-31): the owner operates an isolated lab; you may
  test against that lab on your own once its connections are set up. Never publish anything about
  the lab (hosts, addresses, captures, logs, results); keep notes in the gitignored `plan/`. Never
  connect to any other real server. Public statuses stay at `tested-offline`.
- Work on branches and open pull requests; never push to `main` directly. Merge your own pull
  requests into `main` regularly once CI is green (owner instruction T-02, 2026-10-31); no releases.

## Progress docs
With every change that lands, bring the four status documents in line with the actual code (owner
rule, 2026-10-31): `README.md` (stage, what works today), `ROADMAP.md` (stage, headline numbers
measured from the repository, progress log), `docs/target-app-parity.md` (the function rows and
the totals – the feature matrix) and `docs/gaps.md` (what is still missing per milestone), plus the
current focus in `docs/roadmap.md`. Numbers are measured (crates, tests, generated open-point
list), never estimated. Bump the touched docs' status line.
