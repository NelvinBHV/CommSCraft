# Attribution and provenance of assets

> **Last reviewed:** 2026-11-01 · **Last updated:** 2026-11-01 · **Change:** new (assets register)

Every non-code asset of CommSCraft – images, icons, sounds, fonts – has a row here, added in the
same commit as the asset (rule in `AGENTS.md` / `CLAUDE.md`). No asset is taken from Microsoft
products. Third-party sounds proposed in issues #82–#84 are **not** bundled until their licence
and collecting-society (GEMA) status are documented and the owner has cleared them.

## Images and icons

| File | SHA-256 | Origin | Licence / terms |
|---|---|---|---|
| `assets/branding/CommSCraft-flat-icon.svg` | `88a2cc1eea8ad0401622a721041d3651ed7c7c06b61108c4d1db7766efe6dabf` | Simplified vector derived from the project's fox-with-headset artwork, which was generated with ChatGPT for the project owner (`assets/branding/README.md`); added by the owner, issue #85 | No separate brand-use policy yet (owner decision pending, issue #85); the source-code licence is not assumed to apply |
| `assets/branding/CommSCraft-flat-transparent.svg` | `7494e13b5963f495bc619438e798cac25221bb909b254010cad0a759249700e5` | as above | as above |
| `assets/branding/CommSCraft-monochrome.svg` | `ee165b8c4e54a4e7a794892746bd63a95aab66c6d0b9698ac1b6724cd6de92f3` | as above | as above |

## Sounds

| Sound | Origin | Licence / terms |
|---|---|---|
| Built-in ringtone (`commscraft_tones::ringtone`) | Generated at run time by CommSCraft's own code (sine tones with an envelope); no recording, no third-party material | The project's licence (part of the source code) |
| Built-in notification sound (`commscraft_tones::notification`) | as above | as above |
| Music on hold | None bundled; off unless the user names a WAV file (`COMMSCRAFT_HOLD_MUSIC`) | The user's own file |

## Proposed, not bundled

| Proposal | Licence found (2026-11-01) | Open before bundling |
|---|---|---|
| "Starlight" – Ronald Kah (issue #82) | CC BY 4.0 linked, with extra wishes (no sale, excluded uses) | written confirmation, GEMA status |
| "Simple ringtone" – TheZero, Freesound 273540 (issue #83) | CC0 1.0 | GEMA status |
| "Magical UI Selection Stinger" – TommasoMotteran, Freesound 875968 (issue #84) | CC BY 4.0 | GEMA status, attribution text |
