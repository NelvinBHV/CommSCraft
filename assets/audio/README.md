# CommSCraft Soundpack 1.0

The CommSCraft project owner has selected these sounds as the default user-interface audio:

| Purpose | Approved sound | Version | Audio |
| --- | --- | --- | --- |
| Music on hold | **Pulse** | V5 | `CommsCraft-Pulse-hold-loop.{wav,opus,mp3}` |
| Incoming call | **Beat Call** | V5 | `CommsCraft-Beat-call-loop.{wav,opus,mp3}` |
| Chat / event notification | **Signal Message** | V4 | `CommsCraft-Signal-message-unchanged.{wav,opus,mp3}` |

The short notification retains the V4 composition and arrangement. Pulse and Beat Call use a common 112 BPM, 4/4 grid.

The sources for reproducible rendering are in [`generate_soundpack.py`](generate_soundpack.py); the [GitHub Actions workflow](../../.github/workflows/build-soundpack.yml) renders the audio using the supplied sources and commits WAV, Opus, MP3 and SHA-256 checksums to this directory. Re-rendered encodings may differ byte-for-byte from the initially auditioned preview files depending on the installed renderer and codec versions.

## Instrument provenance

- These three arrangements were originally authored as CommSCraft sound concepts in this conversation.
- Rendered instrument timbres are supplied by **TimGM6mb.sf2**, attributed to Tim Brechbill and contributors; upstream package `timgm6mb-soundfont` is published under **GPL-2.0** (see [Ubuntu package listing](https://packages.ubuntu.com/noble/timgm6mb-soundfont)).
- Audio encoding tools: FluidSynth and FFmpeg.

## UI behavior

- Pulse: loop for remote-party music on hold.
- Beat Call: loop during incoming-call alerting; stop on answer/decline.
- Signal Message: play exactly once for incoming instant messages or other suitable events.
- Expose volume, mute and custom-audio selection for each type.

Implementation tracked in [#82](https://github.com/NelvinBHV/CommSCraft/issues/82), [#83](https://github.com/NelvinBHV/CommSCraft/issues/83) and [#84](https://github.com/NelvinBHV/CommSCraft/issues/84).
