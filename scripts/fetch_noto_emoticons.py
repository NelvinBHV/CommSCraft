#!/usr/bin/env python3
"""Fetches the coloured pictures of the emoticons in crates/ui/src/emoticons.rs from Noto Color
Emoji (googlefonts/noto-emoji at a pinned commit, image resources Apache-2.0), 72 px PNG, into
assets/emoticons/, and writes the table apps/commscraft/src/emoticon_images.rs that embeds them.

Usage: fetch_noto_emoticons.py   (from the repository root; needs network access once)
"""
import hashlib
import pathlib
import re
import sys
import urllib.request

COMMIT = "99701e07d3d1fe1bdbcdb14964547f0fd592966f"
BASE = f"https://raw.githubusercontent.com/googlefonts/noto-emoji/{COMMIT}/2D/png/72/"
ROOT = pathlib.Path(__file__).resolve().parent.parent
TABLE = ROOT / "crates/ui/src/emoticons.rs"
OUT = ROOT / "assets/emoticons"
RUST = ROOT / "apps/commscraft/src/emoticon_images.rs"
# A PNG of the set is small; anything larger is refused.
MAX_PNG = 64 * 1024


def emoji_of_table():
    text = TABLE.read_text(encoding="utf-8")
    body = text.split("pub const EMOTICONS", 1)[1].split("];", 1)[0]
    seen = []
    for m in re.finditer(r'\("((?:[^"\\]|\\.)*)",\s*"([^"]+)"', body):
        e = m.group(2)
        if e not in seen:
            seen.append(e)
    return seen


def file_name(emoji):
    # Noto names files by the code points without the emoji presentation selector.
    cps = [f"{ord(c):04x}" for c in emoji if ord(c) != 0xFE0F]
    return "emoji_u" + "_".join(cps) + ".png"


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    rows = []
    for e in emoji_of_table():
        name = file_name(e)
        target = OUT / name
        if not target.exists():
            req = urllib.request.Request(BASE + name, headers={"User-Agent": "emoticon-fetch"})
            with urllib.request.urlopen(req, timeout=30) as r:
                data = r.read(MAX_PNG + 1)
            if len(data) > MAX_PNG or not data.startswith(b"\x89PNG"):
                sys.exit(f"{name}: not a small PNG")
            target.write_bytes(data)
        rows.append((e, name, hashlib.sha256(target.read_bytes()).hexdigest()))
    lines = [
        "//! The coloured emoticon pictures (Noto Color Emoji, googlefonts/noto-emoji",
        f"//! {COMMIT}, image resources Apache-2.0, see ATTRIBUTION.md), written by",
        "//! scripts/fetch_noto_emoticons.py from the emoticons of crates/ui/src/emoticons.rs.",
        "",
        "/// Emoji and its 72 px PNG.",
        "pub const IMAGES: &[(&str, &[u8])] = &[",
    ]
    for e, name, _ in rows:
        lines.append(f'    ("{e}", include_bytes!("../../../assets/emoticons/{name}")),')
    lines.append("];")
    RUST.write_text("\n".join(lines) + "\n", encoding="utf-8")
    for e, name, digest in rows:
        print(f"{name} {digest}")


if __name__ == "__main__":
    main()
