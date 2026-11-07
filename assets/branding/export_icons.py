#!/usr/bin/env python3
"""Export the CommSCraft PNG app icons from the SVG masters.

16 px comes from the simplified small icon, the other sizes from the flat icon.
Requires: rsvg-convert (librsvg).
"""
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent
SIZES = {16: "CommSCraft-small-icon.svg"}
DEFAULT = "CommSCraft-flat-icon.svg"

if shutil.which("rsvg-convert") is None:
    sys.exit("rsvg-convert not found (package librsvg)")
for size in (16, 32, 64, 128, 256, 512, 1024):
    output = ROOT / f"CommSCraft-icon-{size}.png"
    subprocess.run(
        ["rsvg-convert", "-w", str(size), "-h", str(size),
         str(ROOT / SIZES.get(size, DEFAULT)), "-o", str(output)],
        check=True,
    )
    print(f"Created {output.relative_to(ROOT)}")
