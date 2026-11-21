#!/usr/bin/env python3
"""Generate offline Flatpak sources from the checked-in Cargo.lock (crates.io only).

Each archive is verified using Cargo.lock's SHA-256 checksum. No third-party
Python packages, network calls or generated files are committed to the repo.
"""
import json
import pathlib
import sys
import tomllib

REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"


def generate(lock_path: pathlib.Path) -> list[dict]:
    with lock_path.open("rb") as file:
        packages = tomllib.load(file)["package"]

    sources = []
    seen = set()
    for package in sorted(packages, key=lambda item: (item["name"], item["version"])):
        source = package.get("source")
        if source is None:
            continue  # Workspace member, copied with the application sources.
        if source != REGISTRY:
            raise ValueError(f"Unsupported Cargo source for {package['name']}: {source}")
        name, version = package["name"], package["version"]
        checksum = package["checksum"]
        if len(checksum) != 64 or any(c not in "0123456789abcdef" for c in checksum):
            raise ValueError(f"Missing/invalid checksum: {name} {version}")
        key = (name, version)
        if key in seen:
            raise ValueError(f"Duplicate vendored crate: {name} {version}")
        seen.add(key)
        destination = f"cargo/vendor/{name}-{version}"
        sources.append({
            "type": "archive",
            "archive-type": "tar-gzip",
            "url": f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
            "sha256": checksum,
            "dest": destination,
        })
        sources.append({
            "type": "inline",
            "contents": json.dumps({"package": checksum, "files": {}}),
            "dest": destination,
            "dest-filename": ".cargo-checksum.json",
        })

    sources.append({
        "type": "inline",
        "contents": (
            '[source.crates-io]\nreplace-with = "vendored-sources"\n\n'
            '[source.vendored-sources]\ndirectory = "cargo/vendor"\n'
        ),
        "dest": "cargo",
        "dest-filename": "config",
    })
    return sources


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("Usage: generate_flatpak_cargo_sources.py Cargo.lock OUT.json")
    output = pathlib.Path(sys.argv[2])
    output.parent.mkdir(parents=True, exist_ok=True)
    sources = generate(pathlib.Path(sys.argv[1]))
    output.write_text(json.dumps(sources, indent=2) + "\n", encoding="utf-8")
    print(f"Generated {len(sources)} Flatpak source entries at {output}")


if __name__ == "__main__":
    main()
