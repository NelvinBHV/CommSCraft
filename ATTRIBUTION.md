# Attribution and provenance of assets

> **Last reviewed:** 2026-11-01 · **Last updated:** 2026-11-07 · **Change:** minor (app icons, brand policy)

Every non-code asset of CommSCraft – images, icons, sounds, fonts – has a row here, added in the
same commit as the asset (rule in `AGENTS.md` / `CLAUDE.md`). No asset is taken from Microsoft
products. The third-party sounds first proposed in issues #82–#84 (Starlight, Simple ringtone, Magical
UI Selection Stinger) were superseded by the owner's own sound pack and are not bundled.

## Images and icons

| File | SHA-256 | Origin | Licence / terms |
|---|---|---|---|
| `assets/branding/CommSCraft-flat-icon.svg` | `88a2cc1eea8ad0401622a721041d3651ed7c7c06b61108c4d1db7766efe6dabf` | Simplified vector derived from the project's fox-with-headset artwork, which was generated with ChatGPT for the project owner (`assets/branding/README.md`); added by the owner, issue #85 | `assets/branding/BRAND-POLICY.md`; the source-code licence does not apply |
| `assets/branding/CommSCraft-flat-transparent.svg` | `7494e13b5963f495bc619438e798cac25221bb909b254010cad0a759249700e5` | as above | as above |
| `assets/branding/CommSCraft-monochrome.svg` | `ee165b8c4e54a4e7a794892746bd63a95aab66c6d0b9698ac1b6724cd6de92f3` | as above | as above |
| `assets/branding/CommSCraft-small-icon.svg` | `74bf4ae3a788959d5229f56bcef3624c6fbb5da5cd84c555a6a4d6804229d7b1` | Simplification of the flat icon for 16 px, drawn by hand-written shapes | `assets/branding/BRAND-POLICY.md` |
| `assets/branding/CommSCraft-icon-16.png` | `cfa13c90db9903255fa550a295d97f0800c907a8dffcceaf373cf6fe4275291d` | Exported by `assets/branding/export_icons.py` | `assets/branding/BRAND-POLICY.md` |
| `assets/branding/CommSCraft-icon-32.png` | `e6faa9c79ec1b200c290a148ee537f45b25f0df893775f50d23f414aee6be03f` | Exported by `assets/branding/export_icons.py` | `assets/branding/BRAND-POLICY.md` |
| `assets/branding/CommSCraft-icon-64.png` | `ffbf9e0015e337bcb34329756cf87c12c1f5b46445adb38cbc79203a07e0cba2` | Exported by `assets/branding/export_icons.py` | `assets/branding/BRAND-POLICY.md` |
| `assets/branding/CommSCraft-icon-128.png` | `815b5aa24f84c72d3f65014429e4c82d9e7cb54053ed0b83f15ce085bd8ff34b` | Exported by `assets/branding/export_icons.py` | `assets/branding/BRAND-POLICY.md` |
| `assets/branding/CommSCraft-icon-256.png` | `88b9a312d820fa41cced0dc8fcc09cff6333dfe939afdaee447a81cd3d03e6eb` | Exported by `assets/branding/export_icons.py` | `assets/branding/BRAND-POLICY.md` |
| `assets/branding/CommSCraft-icon-512.png` | `9ac3413cdf5c3a44f7e7682b5ad10cddb141165f799cff2dc156c0369adbbe05` | Exported by `assets/branding/export_icons.py` | `assets/branding/BRAND-POLICY.md` |
| `assets/branding/CommSCraft-icon-1024.png` | `b03eada0beb6b2e790a9641155b7ab7b436ae0fd0f9bed4eb5df5cf37dd2ed87` | Exported by `assets/branding/export_icons.py` | `assets/branding/BRAND-POLICY.md` |

## Sounds

The owner-approved CommSCraft sound pack (issues #82–#84, `assets/audio/README.md`). The WAV files are embedded in the program; Opus and MP3 are kept in the repository only.

| Sound | File | SHA-256 | Origin | Licence / terms |
|---|---|---|---|---|
| Pulse V5 (music on hold) | `assets/audio/CommsCraft-Pulse-hold-loop.wav` | `e74aee37b484f5b6943d00e63d884d1b0a212085299d3f2eadcb53b564e4c0d4` | Composed as a CommSCraft sound concept for the owner, rendered by `assets/audio/generate_soundpack.py` with FluidSynth; instrument samples from the Fluid (R3) SoundFont (Frank Wen, MIT) | MIT, `assets/audio/LICENSE.md` |
| Beat Call V5 (ringtone) | `assets/audio/CommsCraft-Beat-call-loop.wav` | `f7ea306684eb3495878ba7bc651f741e1a1c511dbee47849fe160def3e1411b8` | as above | as above |
| Signal Message V4 (notification) | `assets/audio/CommsCraft-Signal-message-unchanged.wav` | `3daf8f99f0c06f89b3b1e22a155daad9c5ea7d7718bcecbb62511f62fd5d8296` | as above | as above |

The Opus and MP3 versions are listed in `assets/audio/SHA256SUMS.txt`; the CI checks all of them. A user's own sound (`COMMSCRAFT_RINGTONE`, `COMMSCRAFT_NOTIFY_SOUND`, `COMMSCRAFT_HOLD_MUSIC`) is not distributed with the program.
