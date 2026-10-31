# CommSCraft roadmap: milestones and what's next

> **Last reviewed:** 2026-10-27 · **Last updated:** 2026-10-27 · **Change:** minor (M2 started)

Forward-looking plan. The summary is [`ROADMAP.md`](../ROADMAP.md); the ranked work list is
[`gaps.md`](gaps.md); the function list is in [`target-app-parity.md`](target-app-parity.md).
A milestone is done when its gate passes offline; "verified" always means an interop run by the
owner on an authorized server and is deferred until such an environment exists (D-08).

## Current focus (M0 – foundations)

1. **Offline first** (decision D-08): there is no interop test environment yet, so every gate below
   ends at `tested-offline`; "verified" rows wait until the owner provides an environment.
2. ~~Import the protocol analysis~~ – done: [`protocol/`](protocol/) (39 evidence files, function
   matrix, conflict/gap/open-point list, source registers; English, no PDFs, no patent notes).
3. ~~Workspace skeleton~~ – done: Rust workspace with the first crates (`ntlm`, `ice`, `srtp`,
   `sip-identity`), lints from [`AGENTS.md`](../AGENTS.md), CI on `ubuntu-latest` (D-09).
4. ~~Test vectors as tests~~ – done: MS-NLMP NTLMv2 (§4.2.4), RFC 3711 B.3 and MS-SSRTP §4
   (key derivation and packet transform), MS-SIPRE §4.2 (EPID → +sip.instance), MS-ICE2 CRC table
   and the SfB candidate priorities (IE-05), MPPC per RFC 2118/MS-RDPBCGR (`sipcomp`), the
   compressed MS-ABS example and file-name dates (`abs`), the MS-PSOM ConnMgr frames (`psom`) and
   the MS-TURNBWM XOR examples (`turn-bwm`).
5. ~~Close the open-point list~~ – done: normative answers or defaults for every point; the
   remaining server behaviour is the owner's test catalogue [`interop-tests.md`](interop-tests.md).
6. **M1 in progress** – sign-in and registration, offline against specification examples and
   self-built fixtures. Done: SIP message parser and stream framing (`sip-msg`), MS-SIPAE signing
   and verification with NTLM (`sipae`), NTLM CHALLENGE/AUTHENTICATE messages (`ntlm::messages`,
   example §4.2.4.3 byte for byte), the REGISTER flow as a sans-I/O state machine (`register`,
   offline sign-in against a test registrar). In-band provisioning (`provisioning`,
   bounded `xml` reader) and the deregister NOTIFY are done. Discovery (`discovery`, `url`) and the
   registrar redirect are done. Keep-alive, the TLS adapter (`net`) and `commscraft-cli`
   are done: **M1 is ready for the owner's first interop run (IT-19)**. Discovery is wired into the CLI
   (`--discover-only` for IT-16).
7. **M2 in progress** – presence and IM. Done offline: contact list, one-time batch presence
   queries, own state publication scoped to the endpoint (`presence`, CLI `presence`), 1:1 IM
   (`im`), the session engine with long-lived subscriptions and conversations (`session`).
   Next: the CLI on top of the engine (interactive mode), address book search (MS-DLX), group IM
   (MS-CONFIM), then M3.

## Milestones

| ID | Milestone | Functions (target-app-parity #) | Gate |
|---|---|---|---|
| M0 | Foundations | – | Decisions taken; analysis imported; workspace builds; test-vector tests green |
| M1 | Sign-in and registration | 1–8, 50 | Discovery → TLS → NTLM/Kerberos/TLS-DSK → REGISTER → provisioning → sign-out, offline against recorded fixtures; then verified |
| M2 | Presence, contacts, IM | 9–15, 17–19, 46 | Publish/subscribe presence, contact list, address book search, 1:1 and group IM; verified |
| M3 | Audio 1:1 | 21–26, 30, 32, 35 | Call with G.711/G.722 over ICE/TURN with SRTP, hold/resume, DTMF; verified incl. relay through the Edge |
| M4 | Conferences and telephony | 40–42, 33–34, 36–37, 39, 45 | Schedule/join/lobby/mute; transfer, forwarding, park, PSTN, E911, QoE report; verified |
| M5 | Video | 28, 31 | H.264 UC send/receive in 1:1 and conference; verified |
| M6 | Viewing shared screens | 43 | View a desktop/application share (RDP over RTP); verified |
| M7 | Exchange and extras | 16, 20, 38, 44, 47–49, 51 | Calendar state, OOF note, conversation history, voicemail, persistent chat, PSOM content; verified where documented |

Functions 27 (G.722.1 only, after licence clearance), 29 (RTVideo — not planned) and 52 (file
transfer — needs captures) stay outside the milestones until the matching decisions are taken.

## Alpha gate (core workflows)

What a typical Skype for Business user does every day, end to end on Linux against an on-premises
server. Status: **not started**.

| Core workflow | Works end to end? |
|---|---|
| Sign in automatically (discovery, authentication, registration) and sign out | no |
| See colleagues' presence and set your own | no |
| Chat 1:1 and in a group | no |
| Call a colleague with audio (incl. through the Edge server) | no |
| Join a scheduled meeting with audio and see who is speaking | no |
| Watch a colleague's shared screen | no |
