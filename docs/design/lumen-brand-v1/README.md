# Lumen Presenter — design handoff v1

Prepared 16 October 2026. Scope: the new L/projected-light identity, a restrained in-app header placement, and the matching macOS icon. Application code has not been changed.

**Start with [CLAUDE-PROMPT.md](CLAUDE-PROMPT.md).** It is a self-contained implementation request. Read [IMPLEMENTATION.md](IMPLEMENTATION.md) for placement and regression requirements. Open [preview.html](preview.html) to inspect the production assets at actual UI sizes.

## Design authority

1. The user's requirement to retain the current layout and all existing features.
2. Current Rust code and `../../../DESIGN.md` for layout, controls, typography and palette.
3. `IMPLEMENTATION.md` and `tokens.json` for the small branding change.
4. Supplied SVGs for reproducible logo geometry and wordmark outlines.
5. Generated images as concept references only.

The revised generated mockup is a visual direction, **not a pixel-perfect implementation target**. It enlarged the brand area, shifted the mode switch, widened the left sidebar, introduced gradients, and restyled several controls. Do not copy those changes. The first in-app mockup was rejected and is intentionally excluded.

## Contents

| File / directory | Purpose |
| --- | --- |
| `CLAUDE-PROMPT.md` | Ready-to-paste implementation request |
| `IMPLEMENTATION.md` | Dimensions, behaviour, file map and acceptance checklist |
| `tokens.json` | Existing palette plus scoped brand component values |
| `preview.html` | Offline asset proof sheet at actual and enlarged sizes |
| `assets/lumen-mark-*.svg` | Editable transparent mark in saffron, light and dark |
| `assets/lumen-header-*.svg` | Outlined, font-independent compact header lockups |
| `assets/*.png` | Transparent mark exports, header @1x/@2x/@3x, icon master |
| `assets/fonts/` | Bundled OFL Poppins for a self-contained offline preview |
| `assets/lumen-app-icon.svg` | Editable flat macOS icon master |
| `assets/Lumen.iconset/` | Complete macOS multi-resolution iconset |
| `assets/Lumen-AppIcon.icns` | Packaged macOS icon |
| `references/logo-concept.png` | Original generated identity exploration |
| `references/in-app-direction.png` | Revised generated placement direction |
| `references/existing-console.png` | Previously captured console from this repository; historical baseline |
| `PROVENANCE.md` | Sources and distinction between generated concepts and production redraw |
| `POPPINS-OFL.txt` | License for bundled Poppins used to outline the wordmark |
| `tools/build_assets.py` | Rebuild vector/PNG/ICNS assets inside this package only |
| `MANIFEST.sha256` | File integrity hashes, excluding this manifest itself |

## Asset policy

The production mark is a manually normalized vector redraw of the selected AI-generated concept. It has two filled shapes and true transparent negative space. The production wordmark uses the project's bundled Poppins rather than tracing generated lettering. This gives Claude stable geometry and consistent fonts; it is not an exact trace of the raster concept.

Use the saffron transparent mark and `lumen-header-on-dark@2x.png` in the dark operator UI. Native Poppins text alongside the mark is preferable when integrating accessibility and responsive fallback. SVG wordmarks have outlined text and need no installed font. The icon has a transparent exterior and a charcoal rounded-square plate; the in-app mark has no plate.

All assets are local. No runtime network requests, new commercial fonts or proprietary application assets are needed. These are visual assets, not a trademark clearance report.

## Rebuild

From this repository: `python3 docs/design/lumen-brand-v1/tools/build_assets.py`.

Requires Python with `fontTools`, `rsvg-convert`, and macOS `sips` / `iconutil` for the ICNS. It reads bundled Poppins fonts and writes only within this package. Recompute `MANIFEST.sha256` after rebuilding.
