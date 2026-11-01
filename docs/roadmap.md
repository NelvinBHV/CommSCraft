# CommSCraft roadmap: milestones and what's next

> **Last reviewed:** 2026-11-01 · **Last updated:** 2026-11-01 · **Change:** minor (M4: delegates)

Forward-looking plan. The summary is [`ROADMAP.md`](../ROADMAP.md); the ranked work list is
[`gaps.md`](gaps.md); the function list is in [`target-app-parity.md`](target-app-parity.md).
A milestone is done when its gate passes offline; "verified" always means an interop run by the
owner on an authorized server (D-08). Runs in the owner's private lab inform the work but are not
published.

## Current focus (M3 – audio 1:1)

**Offline first** (decision D-08): every gate ends at `tested-offline`; lab runs stay private and
"verified" waits for the owner's runs ([`interop-tests.md`](interop-tests.md)).

| Milestone | State (2026-10-31) | What exists | Missing before the gate |
|---|---|---|---|
| M0 | done | analysis in [`protocol/`](protocol/), decisions, workspace, CI, specification test vectors | – |
| M1 | built, tested-offline | discovery (Autodiscover HTTPS-only, DNS SRV, trust model), TLS adapter, NTLM + MS-SIPAE, REGISTER (GRUU, refresh, redirect, deregister), keep-alive, provisioning, sign-out, `commscraft-cli` | Kerberos, TLS-DSK; owner verification |
| M2 | built, tested-offline | presence (publish, batch subscriptions, pool resubscribe), contact list (read, live), 1:1 IM with typing, group IM (ad-hoc conference, invitations, IMDN), address book search (MS-DLX over NTLM/HTTP), session engine, CLI `interactive` | contact list editing, privacy/blocking, offline address book, photos; owner verification |
| M3 | in progress | G.711, G.722, ALSA/PipeWire adapter (D-11), RTP/RTCP, DTMF, SRTP, SDP, STUN, MS-TURN, MS-AVEDGEA, ICE agent (UDP and TCP relay on 443 with pseudo-TLS), call signalling incl. hold/resume and early media, media core with the MS-RTP RTCP profile, bandwidth management (call admission control) and runtime, CLI `/call` `/answer` `/hangup` `/dtmf` `/mute` `/hold` `/resume` | redundancy, early answer as the callee, TCP-only mode, own music on hold, bandwidth management for video; owner verification (IT-23 … IT-29) |
| M4 | in progress | meetings: scheduling (create, list, change, cancel; options, presenters, static meeting), join (focus URI, meeting link), lobby, roster, conference audio via the A/V MCU (SRTP sent, SRTP/SSRTP received), muting ourselves, presenter commands, announcements, dialling out to phones, PSTN calling, call transfer, forwarding, call park, quality reports, emergency calls with location, team and delegate ringing, delegates and calls on behalf of the boss, meeting options in the running meeting | E911 rest (WLAN/LLDP identifiers), PSTN dial-in details |
| M5 – M7 | not started | building blocks only: PSOM/ABS/MPPC test vectors | everything else |

Next steps in order: telephony (park, team
and delegate calls); the M3 remainders above are optional or need captures.

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
server. Status: **in progress** – the first four workflows exist in `commscraft-cli interactive`
and pass offline tests; none is verified against a real server yet.

| Core workflow | Works end to end? |
|---|---|
| Sign in automatically (discovery, authentication, registration) and sign out | built, tested-offline (NTLM only) |
| See colleagues' presence and set your own | built, tested-offline |
| Chat 1:1 and in a group | built, tested-offline |
| Call a colleague with audio (incl. through the Edge server) | built, tested-offline (UDP loopback test, TCP relay against a TURN test server) |
| Join a scheduled meeting with audio and see who is speaking | no |
| Watch a colleague's shared screen | no |
