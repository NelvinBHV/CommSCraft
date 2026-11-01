#!/usr/bin/env python3
"""Export CommSCraft PNG app icons from the scalable flat-style SVG.

Requires: python -m pip install cairosvg
"""
from pathlib import Path

from cairosvg import svg2png

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT / "CommSCraft-flat-icon.svg"

for size in (16, 32, 64, 128, 256, 512, 1024):
    output = ROOT / f"CommSCraft-icon-{size}.png"
    svg2png(url=str(SOURCE), write_to=str(output), output_width=size, output_height=size)
    print(f"Created {output.relative_to(ROOT)}")
