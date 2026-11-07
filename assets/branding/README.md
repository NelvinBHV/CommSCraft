# CommSCraft brand assets

This directory contains the independent fox-with-headset identity for CommSCraft. The **original detailed fox illustration** was generated with ChatGPT for the project user; the simplified vector versions were derived from its visual concept. This is not Microsoft/Skype for Business artwork.

## Files in the repository

| File | Use |
| --- | --- |
| [CommSCraft-flat-icon.svg](CommSCraft-flat-icon.svg) | Main flat icon: rounded teal background |
| [CommSCraft-flat-transparent.svg](CommSCraft-flat-transparent.svg) | Vector logo with transparent background |
| [CommSCraft-monochrome.svg](CommSCraft-monochrome.svg) | Monochrome/vector asset |
| [CommSCraft-small-icon.svg](CommSCraft-small-icon.svg) | Simplified fox for 16 px, where the flat icon loses its silhouette |
| `CommSCraft-icon-{16,32,64,128,256,512,1024}.png` | App icons (16 px from the small icon, the others from the flat icon) |
| [export_icons.py](export_icons.py) | Generates the PNG app icons from the SVGs |
| [BRAND-POLICY.md](BRAND-POLICY.md) | Terms of use for the logo and the name |

**Important:** The original detailed raster illustration and the high-resolution transparent raster are not yet committed here. They have been prepared outside GitHub; see [tracking issue #85](https://github.com/NelvinBHV/CommSCraft/issues/85). The vector designs are simplified derivatives, not pixel-identical replicas.

## Brand colors

- Teal: `#007F89`
- Ink: `#10242A`
- Cream: `#FFF4DE`

## Export PNG app icons

With `rsvg-convert` (librsvg) installed, run:

```bash
python3 assets/branding/export_icons.py
```

## Backgrounds

Checked on light and dark backgrounds. Use the flat icon on **dark** backgrounds: the transparent and the monochrome version are drawn with dark lines, which disappear there together with the sound waves. They are for light backgrounds only.

## Rights and usage

The terms are in [BRAND-POLICY.md](BRAND-POLICY.md). Provenance: original logo artwork generated through ChatGPT for the project user; the source-code license does not apply to these assets.

These files are **not** copied from or endorsed by Microsoft.
