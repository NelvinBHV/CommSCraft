# Releases

> **Last reviewed:** 2026-11-21 · **Last updated:** 2026-11-21 · **Change:** major (release procedure and version scheme)

## Version scheme
The version says how much of the Windows client's function set CommSCraft covers, measured from
the function table of [`target-app-parity.md`](target-app-parity.md) by
[`scripts/parity.py`](../scripts/parity.py):

- per function: implemented as a whole = 1, in parts = ½, not started = 0; functions that are not
  planned, need captures or wait for a legal clearance are left out;
- parity = sum ÷ counted functions, in whole percent;
- version **0.‹parity›.‹patch›** – the patch number counts releases at the same parity;
- **1.0.0** is reserved for full parity verified on authorized servers;
- the suffix **-alpha** stays while nothing is `verified-interoperable`.

`python3 scripts/parity.py` prints the measurement, `--version` the version core.

## Procedure (after changes land on `main`)
1. Measure the parity and set the workspace version in `Cargo.toml` (and the release entry of
   `packaging/flatpak/io.github.NelvinBHV.CommSCraft.metainfo.xml`) in a pull request.
2. Tag `v‹version›` on `main`; the `flatpak` workflow (read-only, free runner) builds
   `CommSCraft.flatpak` as an artifact.
3. Create the GitHub release (pre-release while `-alpha`) with the notes and attach the bundle
   and `components.txt` (the runtime, FFmpeg extension and bundled native builds of this release).

## Flatpak
- The Rust crates are built offline from checksum-verified sources that
  `scripts/generate_flatpak_cargo_sources.py` derives from `Cargo.lock`; MIT Kerberos 1.22.3 (signature checked
  on update; `scripts/check_native_components.py` and the weekly `native-components` workflow
  report newer upstream releases) and craft-fonts are pinned sources too.
- Manifest: [`packaging/flatpak/io.github.NelvinBHV.CommSCraft.yml`](../packaging/flatpak/io.github.NelvinBHV.CommSCraft.yml),
  runtime `org.freedesktop.Platform` 25.08.
- In the bundle: CommSCraft (MIT OR Apache-2.0), its Rust dependencies, MIT Kerberos and the
  craft-fonts faces (pinned commit, required in a release; licences in craft-fonts). FFmpeg comes
  from the runtime; H.264 needs the runtime extension `org.freedesktop.Platform.ffmpeg-full`
  (Flathub, installed with the app or by `flatpak install flathub org.freedesktop.Platform.ffmpeg-full//25.08`).
- Install: `flatpak install --user CommSCraft.flatpak` (the runtime from Flathub), start:
  `flatpak run io.github.NelvinBHV.CommSCraft`.
- Permissions: network, Wayland/X11, PulseAudio, all devices (GPU for VA-API, camera), the keyring
  (Secret Service) and the session's idle time.
