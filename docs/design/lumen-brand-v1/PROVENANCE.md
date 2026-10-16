# Asset provenance

Prepared 16 October 2026 in the PROPRESENTER workspace.

- `references/logo-concept.png`: original concept generated with the built-in image_gen tool in this chat. Source file: `$CODEX_HOME/generated_images/01a14560-f0f4-7b10-8e69-b399c311a22e/exec-a08d7668-50b9-4252-bfc2-6e395bcdd85d.png`.
- `references/in-app-direction.png`: revised generated mockup from the same tool. Source: the same generated-images directory, `exec-d966c207-dd68-4024-9ab1-d857d41a9338.png`. Its reference was the actual app capture plus original logo concept. It is a conceptual approximation, not a captured implementation.
- The earlier `exec-3795744f-759c-43e3-bb30-4d29cea37b47.png` layout was rejected by the user and is excluded.
- `references/existing-console.png`: copied from `.local/reports/release-shots/Lumen_Presenter.png`. This is a historical repository capture, not a newly captured full-feature test. A fresh native window was inspected with cua.getApp/getScreenshot before preparing the revised mockup; no project data was edited.
- `assets/*.svg`: manually constructed, normalized redraw of the selected L/projected-light idea. Two original filled paths are defined in `tools/build_assets.py`; PNGs and ICNS are derived from these, not AI-upscaled/cropped logos.
- Wordmark outlines: project's existing Poppins SemiBold/Medium, bundled at `crates/lumen-app/assets/fonts/`, licensed under the included SIL OFL. The AI-generated lettering is not a licensed external font source and was not traced.
- Brand values and layout: existing `DESIGN.md`, `style.rs`, `app.rs::toolbar` and user instructions. No ProPresenter executable, decompiled code or proprietary creative asset was used.

Generation brief summary: original identity requested a simple saffron geometric L with a projected-light trapezoid on charcoal, including a macOS icon and monochrome lockup. Revised UI brief requested using the actual app screenshot, removing the prominent Sunday Gathering heading, adding the compact logo and preserving the two-row toolbar, all three columns, Macros, Books, Props, full timers, slides and status bar. These are brief summaries, not verbatim generation logs.

The production redraw is intentionally flat and reproducible. The mockup's gradients, slightly larger logo, altered column proportions and moved mode/status controls are not approved implementation changes. See IMPLEMENTATION.md for the authoritative integration boundaries.
