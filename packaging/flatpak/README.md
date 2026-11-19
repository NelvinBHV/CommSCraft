# Flatpak preview build (no public release)

This package is a **CI preview**, not a verified or published Flatpak release.
It uses the Freedesktop 25.08 runtime and its matching Rust SDK extension.
The [Flatpak preview workflow](../../.github/workflows/flatpak.yml) runs on
same-repository pull requests that affect the application or packaging, and can
also be started using **Actions > Flatpak preview > Run workflow**.

The workflow:
1. Generates `cargo-sources.json` from the checked-in `Cargo.lock` using
   `scripts/generate_flatpak_cargo_sources.py`. Crates.io archives are
   checksum-verified against the lock file and the app builds offline.
2. Builds the `commscraft` desktop binary inside the Flatpak SDK.
3. Uploads an `x86_64` `CommSCraft.flatpak` file as a GitHub Actions
   artifact. Artifacts require unpacking the downloaded GitHub ZIP.

The workflow has only read permission for repository content. It does **not**
push commits, publish GitHub Releases, upload to Flathub, or access lab servers.
The CI container requires privileged execution, so this job skips pull requests
from forks.

## Install a successful preview

Download the `.flatpak` artifact from the successful Actions run, unzip it
and run on a Linux desktop with Flatpak:

```sh
flatpak install --user ./CommSCraft.flatpak
flatpak run io.github.NelvinBHV.CommSCraft --demo
```

This is experimental: a successful compilation is **not** proof that
audio/video, Kerberos tickets, the Secret Service keyring, corporate CAs,
camera access, and enterprise networking work from within the sandbox.
Those need separate **owner-authorized**, local checks. No CI job contacts
a real Skype for Business server. This preview grants broad V4L2 device access
(`--device=all`) for camera support; reduce this permission when the
desktop camera portal is implemented.

For local builds, install Flatpak, flatpak-builder and the 25.08 runtime/SDK
and Rust SDK extension. Generate Cargo sources with Python 3.11+:

```sh
python3 scripts/generate_flatpak_cargo_sources.py Cargo.lock packaging/flatpak/cargo-sources.json
flatpak-builder --user --force-clean --install-deps-from=flathub \
  --repo=flatpak-repo flatpak-build \
  packaging/flatpak/io.github.NelvinBHV.CommSCraft.yml
flatpak build-bundle flatpak-repo CommSCraft.flatpak io.github.NelvinBHV.CommSCraft
```

The generated source file and build directories are temporary artifacts;
do not add them to the repository.
