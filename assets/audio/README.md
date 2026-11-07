# CommSCraft Soundpack 1.0

The CommSCraft project owner has selected these sounds as the default user-interface audio:

| Purpose | Approved sound | Version | Audio |
| --- | --- | --- | --- |
| Music on hold | **Pulse** | V5 | `CommsCraft-Pulse-hold-loop.{wav,opus,mp3}` |
| Incoming call | **Beat Call** | V5 | `CommsCraft-Beat-call-loop.{wav,opus,mp3}` |
| Chat / event notification | **Signal Message** | V4 | `CommsCraft-Signal-message-unchanged.{wav,opus,mp3}` |

The short notification retains the V4 composition and arrangement. Pulse and Beat Call use a common 112 BPM, 4/4 grid.

The sources for reproducible rendering are in [`generate_soundpack.py`](generate_soundpack.py); the [GitHub Actions workflow](../../.github/workflows/build-soundpack.yml) renders the audio read-only using the supplied sources and offers WAV, Opus, MP3 and SHA-256 checksums as an artifact; a new rendering reaches this directory through a pull request. Re-rendered encodings may differ byte-for-byte from the initially auditioned preview files depending on the installed renderer and codec versions.

## Instrument provenance

- These three arrangements were originally authored as CommSCraft sound concepts in this conversation.
- Instrument samples come from the **Fluid (R3) General MIT SoundFont** by Frank Wen (Debian package `fluid-soundfont-gm`, MIT licence). The sounds were rendered with it again on 2026-11-07, with the same scores and mastering, because the earlier rendering used a SoundFont under GPL-2.0.
- Licence of the sounds: [`LICENSE.md`](LICENSE.md).
- Audio encoding tools: FluidSynth and FFmpeg.

## UI behavior

- Pulse: loop for remote-party music on hold.
- Beat Call: loop during incoming-call alerting; stop on answer/decline.
- Signal Message: play exactly once for incoming instant messages or other suitable events.
- Volume, mute and a custom file per type, by environment variables of `commscraft-cli`: `COMMSCRAFT_HOLD_MUSIC`, `COMMSCRAFT_RINGTONE` and `COMMSCRAFT_NOTIFY_SOUND` take a PCM WAV file or `off`; `COMMSCRAFT_HOLD_VOLUME`, `COMMSCRAFT_RINGTONE_VOLUME` and `COMMSCRAFT_NOTIFY_VOLUME` take 0 to 100.
- The WAV files are embedded in the program (`commscraft_tones::soundpack`) and played as mastered, mono; the alerts at 44.1 kHz where the device offers it, music on hold at the codec's rate.

Implementation tracked in [#82](https://github.com/NelvinBHV/CommSCraft/issues/82), [#83](https://github.com/NelvinBHV/CommSCraft/issues/83) and [#84](https://github.com/NelvinBHV/CommSCraft/issues/84).
