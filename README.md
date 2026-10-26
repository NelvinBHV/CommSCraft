# CommSCraft

CommSCraft is a planned **clean-room, open-source Linux client for Skype for Business Server**
(and Lync Server 2013 and later), built from Microsoft's public Open Specifications and IETF RFCs.
It follows the conventions of the [storytold Crafting Apps](https://github.com/storytold)
(WordCraft, GridCraft, DeckCraft, …): pure Rust where possible, measured parity with the target
application, a ranked work list, and nothing copied from Microsoft products.

> **Stage: planning.** There is no program code yet. The first milestone imports the protocol
> analysis (every client function mapped to the specification sections that define it) and fixes
> the remaining owner decision in [`docs/decisions.md`](docs/decisions.md). Read
> [`ROADMAP.md`](ROADMAP.md) for where the project stands.

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

## How the work is documented

| File | Content |
|---|---|
| [`ROADMAP.md`](ROADMAP.md) | Stage, headline numbers, progress log |
| [`docs/roadmap.md`](docs/roadmap.md) | Milestones, current focus, alpha gate |
| [`docs/gaps.md`](docs/gaps.md) | Ranked list of everything missing or unresolved |
| [`docs/target-app-parity.md`](docs/target-app-parity.md) | The target client and the 52 functions with their evidence status |
| [`docs/architecture.md`](docs/architecture.md) | Planned layers and crates |
| [`docs/decisions.md`](docs/decisions.md) | Decisions the owner has to take, and decisions already taken |
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
  systems.

## License

CommSCraft is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Copyright (c) 2026 the CommSCraft contributors.
