# CommSCraft

<img src="assets/branding/CommSCraft-flat-icon.svg" width="152" alt="CommSCraft fox wearing a communications headset" />

CommSCraft is a **clean-room, open-source Linux client for Skype for Business Server** (and Lync
Server 2013 and later), built from Microsoft's public Open Specifications, IETF RFCs and ITU-T
Recommendations. It follows the conventions of the [storytold Crafting Apps](https://github.com/storytold)
(WordCraft, GridCraft, DeckCraft, …): pure Rust where possible, measured parity with the target
application, a ranked work list, and nothing copied from Microsoft products.

> **Stage: M1 (sign-in), M4, M5 (video), M6 (viewing shared screens) and M7 (Exchange and extras) closed, pre-alpha.** Sign-in, presence, instant messaging (1:1 and group),
> address book search, audio and video calls, meetings and telephony (transfer, forwarding, park, delegates,
> emergency calls) exist as building blocks that pass offline tests and whose main flows also ran in the owner's private lab (50 crates,
> 756 tests) and can be used through the command-line tool `commscraft-cli`. `verified-interoperable` stays reserved
> for the owner's runs on authorized servers. The desktop client `commscraft` (egui) signs in and offers chat (1:1 and group), presence, audio and video calls and viewing shared screens through the same session engine, also `tested-in-lab`; persistent chat rooms tested offline; meeting content with polls and voting. Read [`ROADMAP.md`](ROADMAP.md)
> for where the project stands.

CommSCraft is not affiliated with, endorsed by or sponsored by Microsoft. "Skype for Business",
"Lync" and "Microsoft" are trademarks of Microsoft Corporation; they are used here only to describe
what CommSCraft interoperates with.

## What CommSCraft is meant to do

Sign in to an on-premises Skype for Business Server (2015, 2019 or Subscription Edition) from Linux
and take part in daily work the way the Windows desktop client does:

- sign-in (NTLM, Kerberos, TLS-DSK with client certificate), registration and in-band provisioning;
- presence, contact list, address book search, photos;
- instant messaging 1:1 and in conferences;
- audio and video calls (G.711, G.722, H.264 UC) with ICE, TURN and SRTP;
- conferences (join, lobby, roles, A/V, muting), telephony features (forwarding, hold, transfer, park);
- viewing desktop and application sharing;
- Exchange integration (calendar state, out-of-office note, conversation history, voicemail).

What it will **not** do: connect to Skype for Business Online (retired by Microsoft) or Microsoft
Teams; implement codecs that need a Microsoft licence (RTAudio, RTVideo). Details:
[`docs/target-app-parity.md`](docs/target-app-parity.md).

## What works today

Status from [`docs/target-app-parity.md`](docs/target-app-parity.md) (2026-11-21): of 52 client
functions, 38 are implemented and tested offline as a whole, 9 in parts, 0 not started.

| Area | In the code (tested offline) | Not yet |
|---|---|---|
| Sign-in | Autodiscover (HTTPS only), DNS SRV, TLS 1.2/1.3, NTLM, Kerberos and TLS-DSK with a user certificate, each with MS-SIPAE signing, registration with GRUU, home-pool redirect, keep-alive, in-band provisioning, sign-out | – |
| Presence and contacts | own state, contacts' presence (live), contact list (read and edit, on the server or in Exchange), privacy relationships and blocking, address book search, distribution lists, offline address book, contact photos (directory and Exchange) | calendar and out-of-office state |
| Instant messaging | 1:1 with typing indicator, group conversations (ad-hoc conference, invitations, delivery reports), persistent chat rooms in the CLI and the desktop client (tested offline; Skype for Business Server 2019 has none) | rich text, file transfer |
| Audio calls | G.722 and G.711, SRTP, ICE, MS-TURN relay over UDP and over TCP 443 (pseudo-TLS), DTMF, mute, hold/resume, early media, call admission control, ALSA/PipeWire devices; ringing, notification sound and music on hold (the approved CommSCraft sound pack, or a WAV file of the user's; `off` and a volume per sound) | – |
| Meetings | schedule, join by address or meeting link, lobby, roster, audio and video in the A/V MCU (SRTP/SSRTP), mute yourself; view what others share (screen or window); whiteboards, polls (with voting) and slides of the meeting; as presenter: admit, roles, lock, remove, mute others or the audience | sharing one's own screen, drawing whiteboards and slides, creating content |
| Telephony | phone numbers with the dial plan (phone-context), calls from gateways with caller number, call back, anonymous callers, early media, call transfer (blind and with consultation), call forwarding and simultaneous ring, team and delegate calls, call park, emergency calls with location, voicemail (list, listening), response group agent | – |
| Video, screen sharing | video calls and meeting video (H.264), viewing shared screens (1:1 and in meetings) and windows shared over RDP (1:1 and in meetings), sharing one's own screen or a window 1:1 | sharing one's own screen in meetings |
| Exchange | contact photos, contact list (unified contact store), calendar state and out-of-office note (own and contacts'), conversation history, voicemail list and listening | – |

## Building and running

Requirements: Rust (stable) and the ALSA development files (`libasound2-dev` / `alsa-lib`).
The Flatpak takes the camera through the desktop's Camera portal: PipeWire needs camera
support (standard in most distributions; on Gentoo `media-video/pipewire[v4l]`).
The notification area icon needs a StatusNotifierItem host: built into KDE Plasma, Xfce,
Cinnamon, MATE, Budgie, LXQt and waybar; on GNOME the extension “AppIndicator and
KStatusNotifierItem Support”.

```
cargo build --release
./target/release/commscraft-cli audio-check            # local microphone/speaker check, no network
./target/release/commscraft-cli interactive --aor sip:USER@DOMAIN --user NTLM_USER --domain NTLM_DOMAIN --authorized
```

`commscraft-cli` connects to a server only with `--authorized`: it is meant for runs on systems you
are allowed to test. In `interactive` mode the commands are `/contacts`, `/status`, `/chat`,
`/group`, `/invite`, `/join`, `/search`, `/call`, `/answer`, `/hangup`, `/dtmf`, `/mute`, `/to`,
`/end`, `/quit`; `commscraft-cli` without arguments prints the full usage.

The desktop client is in progress: it signs in from its sign-in screen (NTLM, Kerberos, TLS-DSK;
Autodiscover or a given server) and offers chat, chat rooms, presence, audio and video calls,
viewing shared screens and meetings in the operating structure of the Windows client (a narrow
main window with tabs, conversations and calls in windows of their own, options) with its own
look; `--demo` shows the interface
with made-up data and no network connection at all:

```
./target/release/commscraft                              # sign-in screen; --ca-file PEM for an enterprise CA
./target/release/commscraft --demo                       # German or English from LANG; --lang de|en
```

Fonts: built with `CRAFT_FONTS_DIR=/path/to/craft-fonts` (https://github.com/storytold/craft-fonts;
release builds always) it embeds Inter and the script and symbol faces of craft-fonts; without it,
it uses a system sans font (fontconfig). Emoji come from the bundled Noto Emoji, the icons from the
bundled Lucide subset. Screens and the capability gates are described in
[`docs/ui/README.md`](docs/ui/README.md).

## How the work is documented

| File | Content |
|---|---|
| [`ROADMAP.md`](ROADMAP.md) | Stage, headline numbers, progress log |
| [`docs/roadmap.md`](docs/roadmap.md) | Milestones, current focus, alpha gate |
| [`docs/releases.md`](docs/releases.md) | Releases: version from the measured feature parity with the Windows client (now 90 %, version 0.90.9-alpha), Flatpak bundle and how to install it |
| [`docs/gaps.md`](docs/gaps.md) | Ranked list of everything missing or unresolved |
| [`docs/interop-tests.md`](docs/interop-tests.md) | Test cases only the owner runs against a real server, each with a default |
| [`docs/security.md`](docs/security.md) | Security by design: threat model and the rules every crate follows |
| [`docs/target-app-parity.md`](docs/target-app-parity.md) | The target client and the 52 functions with their evidence status |
| [`docs/architecture.md`](docs/architecture.md) | Layers and crates |
| [`docs/protocol/`](docs/protocol/) | The protocol analysis: evidence per protocol, function matrix, conflicts/gaps/open points, source registers |
| [`AGENTS.md`](AGENTS.md) | Rules for contributors and AI agents (clean room, evidence, security) |

## Clean room and sources

- Protocol behaviour comes from Microsoft Open Specifications ([MS-SIPAE], [MS-SIPREGE], [MS-PRES],
  [MS-ICE2], [MS-SDPEXT], …), IETF RFCs and Internet-Drafts, and ITU-T Recommendations. Every
  requirement is cited by document, edition, section and page.
- Specification PDFs are **not** stored in this repository; only our own descriptions with citations.
- No code, assets, icons or text are taken from Microsoft products. Third-party open-source
  implementations are read only to confirm behaviour and are cited as secondary evidence; their code
  is not copied.
- Interoperability tests against real servers are run only by the project owner on authorized
  systems ([`docs/interop-tests.md`](docs/interop-tests.md)); their results are not published.
  All work in this repository is verified offline.

## License

CommSCraft is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Copyright (c) 2026 the CommSCraft contributors.
