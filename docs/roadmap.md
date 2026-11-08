# CommSCraft roadmap: milestones and what's next

> **Last reviewed:** 2026-11-01 · **Last updated:** 2026-11-08 · **Change:** minor (M2 gaps closed except Exchange)

Forward-looking plan. The summary is [`ROADMAP.md`](../ROADMAP.md); the ranked work list is
[`gaps.md`](gaps.md); the function list is in [`target-app-parity.md`](target-app-parity.md).
A milestone is done when its gate passes offline; "verified" always means an interop run by the
owner on an authorized server. Runs in the owner's private lab are shown as `tested-in-lab`; their
details are not published.

## Current focus (M5 – video)

**Offline first**: every gate ends at `tested-offline`; what also ran in the owner's lab is `tested-in-lab`, and
"verified" waits for the owner's runs ([`interop-tests.md`](interop-tests.md)).

| Milestone | State (2026-11-08) | What exists | Missing before the gate |
|---|---|---|---|
| M0 | done | analysis in [`protocol/`](protocol/), workspace, CI, specification test vectors | – |
| M1 | built, tested-offline, tested-in-lab | discovery (Autodiscover HTTPS-only, DNS SRV, trust model), TLS adapter, NTLM + MS-SIPAE, REGISTER (GRUU, refresh, redirect, deregister), keep-alive, provisioning, sign-out, `commscraft-cli` | Kerberos, TLS-DSK; owner verification |
| M2 | built, tested-offline, tested-in-lab | presence (publish, batch subscriptions, pool resubscribe, container manifest), contact list (read, live, editing), privacy relationships and blocking, 1:1 IM with typing, group IM (ad-hoc conference, invitations, IMDN), offline address book (full and delta files, opt-in cache, normalization rules), address book search and distribution list expansion (MS-DLX over NTLM/HTTP), directory photos, error diagnostics (MS-OCER), session engine, CLI `interactive` | the Exchange parts (unified contact store, function 11; photos from Exchange, function 15), which need an Exchange server; owner verification |
| M3 | in progress | G.711, G.722, ALSA/PipeWire adapter, RTP/RTCP, DTMF, SRTP, SDP, STUN, MS-TURN, MS-AVEDGEA, ICE agent (UDP and TCP relay on 443 with pseudo-TLS), call signalling incl. hold/resume and early media, media core with the MS-RTP RTCP profile, bandwidth management (call admission control) and runtime, CLI `/call` `/answer` `/hangup` `/dtmf` `/mute` `/hold` `/resume` | redundancy, early answer as the callee, TCP-only mode, own music on hold, bandwidth management for video; owner verification (IT-23 … IT-29) |
| M4 | done (tested-offline, tested-in-lab) | meetings: scheduling (create, list, change, cancel; options, presenters, static meeting), join (focus URI, meeting link), lobby, roster, conference audio via the A/V MCU (SRTP sent, SRTP/SSRTP received), muting ourselves, presenter commands, announcements, dialling out to phones, PSTN calling, call transfer, forwarding, call park, quality reports, emergency calls with location, team and delegate ringing, delegates and calls on behalf of the boss, meeting options in the running meeting | E911 rest (WLAN/LLDP identifiers), PSTN dial-in details |
| M5 | in progress | H.264 UC RTP payload (`h264uc`), camera capture, H.264 encoding on the GPU or CPU and decoding (`video-ffmpeg`), video SDP (`sdp::video`), PLI/VSR (`rtp::feedback`), video core (`media::video`) and transport (`call-media`), 1:1 video calls (session and CLI), video in meetings, bandwidth estimation and rate adaptation (`media::rate`), picture control (function 31) | hardware decoding, 1:1 and conference video |
| M6 – M7 | not started | building blocks only: PSOM/ABS/MPPC test vectors | everything else |

Next steps in order: a system call filter for the video window process; the Exchange parts of M2 once an Exchange server is available; the M3 remainders above are optional or need captures.

## Milestones

| ID | Milestone | Functions (target-app-parity #) | Gate |
|---|---|---|---|
| M0 | Foundations | – | Analysis imported; workspace builds; test-vector tests green |
| M1 | Sign-in and registration | 1–8, 50 | Discovery → TLS → NTLM/Kerberos/TLS-DSK → REGISTER → provisioning → sign-out, offline against recorded fixtures; then verified |
| M2 | Presence, contacts, IM | 9–15, 17–19, 46 | Publish/subscribe presence, contact list, address book search, 1:1 and group IM; verified |
| M3 | Audio 1:1 | 21–26, 30, 32, 35 | Call with G.711/G.722 over ICE/TURN with SRTP, hold/resume, DTMF; verified incl. relay through the Edge |
| M4 | Conferences and telephony | 40–42, 33–34, 36–37, 39, 45 | Schedule/join/lobby/mute; transfer, forwarding, park, PSTN, E911, QoE report; verified |
| M5 | Video | 28, 31 | H.264 UC send/receive in 1:1 and conference; verified |
| M6 | Viewing shared screens | 43 | View a desktop/application share (RDP over RTP); verified |
| M7 | Exchange and extras | 16, 20, 38, 44, 47–49, 51 | Calendar state, OOF note, conversation history, voicemail, persistent chat, PSOM content; verified where documented |

Functions 27 (G.722.1 only, after licence clearance), 29 (RTVideo — not planned) and 52 (file
transfer — needs captures) stay outside the milestones for now.

## Alpha gate (core workflows)

What a typical Skype for Business user does every day, end to end on Linux against an on-premises
server. Status: **in progress** – the first five workflows exist in `commscraft-cli interactive`;
none is `verified-interoperable` yet.

| Core workflow | Works end to end? |
|---|---|
| Sign in automatically (discovery, authentication, registration) and sign out | built, tested-offline (NTLM only), sign-in tested-in-lab |
| See colleagues' presence and set your own | built, tested-offline, own state tested-in-lab |
| Chat 1:1 and in a group | built, tested-offline, tested-in-lab |
| Call a colleague with audio (incl. through the Edge server) | built, tested-offline (UDP loopback test, TCP relay against a TURN test server), direct calls tested-in-lab |
| Join a scheduled meeting with audio and see who is in it | built, tested-offline, tested-in-lab |
| Watch a colleague's shared screen | no |
