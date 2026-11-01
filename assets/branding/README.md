# CommSCraft brand assets

This directory contains the independent fox-with-headset identity for CommSCraft. The **original detailed fox illustration** was generated with ChatGPT for the project user; the simplified vector versions were derived from its visual concept. This is not Microsoft/Skype for Business artwork.

## Files in the repository

| File | Use |
| --- | --- |
| [CommSCraft-flat-icon.svg](CommSCraft-flat-icon.svg) | Main flat icon: rounded teal background |
| [CommSCraft-flat-transparent.svg](CommSCraft-flat-transparent.svg) | Vector logo with transparent background |
| [CommSCraft-monochrome.svg](CommSCraft-monochrome.svg) | Monochrome/vector asset |
| [export_icons.py](export_icons.py) | Generate common-size PNG app icons locally from the flat SVG |

**Important:** The original detailed raster illustration and the high-resolution transparent raster are not yet committed here. They have been prepared outside GitHub; see [tracking issue #85](https://github.com/NelvinBHV/CommSCraft/issues/85). The vector designs are simplified derivatives, not pixel-identical replicas.

## Brand colors

- Teal: `#007F89`
- Ink: `#10242A`
- Cream: `#FFF4DE`

## Export PNG app icons

Install CairoSVG (for example, `python -m pip install cairosvg`) and run:

```bash
python assets/branding/export_icons.py
```

The script exports 16, 32, 64, 128, 256, 512 and 1024 px icons next to the SVG.

## Rights and usage

Provenance: original logo artwork generated through ChatGPT for the project user. Do not assume that the source-code license automatically applies to these branding assets; maintainers should publish a separate explicit logo/brand use policy before release. AI-generated visual output is not automatically guaranteed to be exclusive or copyrightable.

These files are **not** copied from or endorsed by Microsoft.
