# Attribution and provenance of assets

> **Last reviewed:** 2026-11-01 · **Last updated:** 2026-11-07 · **Change:** minor (approved sound pack)

Every non-code asset of CommSCraft – images, icons, sounds, fonts – has a row here, added in the
same commit as the asset (rule in `AGENTS.md` / `CLAUDE.md`). No asset is taken from Microsoft
products. The third-party sounds first proposed in issues #82–#84 (Starlight, Simple ringtone, Magical
UI Selection Stinger) were superseded by the owner's own sound pack and are not bundled.

## Images and icons

| File | SHA-256 | Origin | Licence / terms |
|---|---|---|---|
| `assets/branding/CommSCraft-flat-icon.svg` | `88a2cc1eea8ad0401622a721041d3651ed7c7c06b61108c4d1db7766efe6dabf` | Simplified vector derived from the project's fox-with-headset artwork, which was generated with ChatGPT for the project owner (`assets/branding/README.md`); added by the owner, issue #85 | No separate brand-use policy yet; the source-code licence is not assumed to apply |
| `assets/branding/CommSCraft-flat-transparent.svg` | `7494e13b5963f495bc619438e798cac25221bb909b254010cad0a759249700e5` | as above | as above |
| `assets/branding/CommSCraft-monochrome.svg` | `ee165b8c4e54a4e7a794892746bd63a95aab66c6d0b9698ac1b6724cd6de92f3` | as above | as above |

## Sounds

The owner-approved CommSCraft sound pack (issues #82–#84, `assets/audio/README.md`). The WAV files are embedded in the program; Opus and MP3 are kept in the repository only.

| Sound | File | SHA-256 | Origin | Licence / terms |
|---|---|---|---|---|
| Pulse V5 (music on hold) | `assets/audio/CommsCraft-Pulse-hold-loop.wav` | `61004785a19df7fdfcd001ff0cf28d930b78047f721b982eaeaf534f8f7a5a86` | Composed as a CommSCraft sound concept for the owner, rendered by `assets/audio/generate_soundpack.py` with FluidSynth; instrument timbres from TimGM6mb.sf2 (Tim Brechbill and contributors; packaged under GPL-2.0) | The licence of the rendered audio is not stated yet; the source-code licence is not assumed to apply |
| Beat Call V5 (ringtone) | `assets/audio/CommsCraft-Beat-call-loop.wav` | `96cc154c5e93533385cc5686f949dc8c3094f1bbf46bc35cbba44ccbedee4af8` | as above | as above |
| Signal Message V4 (notification) | `assets/audio/CommsCraft-Signal-message-unchanged.wav` | `419cbcde4f89123719b5f7f67e21447a5728c3ec016b491452f6b36dd824a86c` | as above | as above |

The Opus and MP3 versions are listed in `assets/audio/SHA256SUMS.txt`; the CI checks all of them. A user's own sound (`COMMSCRAFT_RINGTONE`, `COMMSCRAFT_NOTIFY_SOUND`, `COMMSCRAFT_HOLD_MUSIC`) is not distributed with the program.
