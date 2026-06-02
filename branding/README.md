# sunscreen — branding assets

Editable vector sources and exported raster files for the sunscreen CLI brand.

## Files

| File | Purpose |
|---|---|
| `logo-mark.svg` | Icon only (sun + horizon). Use for favicons, avatars, square containers. |
| `logo-wordmark.svg` | Wordmark only ("sunscreen"). Use when paired with another mark or in tight headers. |
| `logo-full-dark.svg` | Mark + wordmark, optimized for dark backgrounds (default brand). |
| `logo-full-light.svg` | Mark + wordmark, darker palette for light backgrounds. |
| `logo-mark.png` (+ 512/256/128) | Rasterized mark exports. |
| `logo-full-dark.png`, `logo-full-light.png` | Rasterized full lockups. |

## Editing

The `.svg` files **are the source**. Open them directly in:

- **Adobe Illustrator** — `File → Open` accepts SVG natively and lets you `Save As → .ai`.
- **Figma**, **Inkscape**, **Affinity Designer** — all open SVG directly.

If you need a true `.ai` binary, open `logo-full-dark.svg` in Illustrator and `Save As → Adobe Illustrator (.ai)` into this folder.

## Palette

| Token | Hex | Use |
|---|---|---|
| Sun highlight | `#FBD38D` | Inner sun, rays |
| Sun base | `#F4A340` | Sun body, horizon line (dark theme) |
| Sun deep | `#D97706` | Horizon line (light theme) |
| Ink light | `#E8E6E1` | Wordmark on dark bg |
| Ink dark | `#1A1814` | Wordmark on light bg |

## Re-exporting PNGs (macOS)

```sh
qlmanage -t -s 1024 -o . logo-mark.svg
sips -Z 512 logo-mark.svg.png --out logo-mark-512.png
```
