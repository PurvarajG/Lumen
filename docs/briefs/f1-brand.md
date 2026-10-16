# F1: brand lockup in the header and the new app icon

User intent: the approved saffron L / projected-light logo ties the experience together. This is a **small branding integration, not a redesign**. They rejected a mockup that changed the layout and stripped features. Every change must be intentional: do not copy the generated mockup's spacing, centred mode switch, right-pushed pills, widths, gradients or control styles.

Source package (read-only reference): `docs/design/lumen-brand-v1/` — read `README.md` and `IMPLEMENTATION.md`. Also read `AGENTS.md`, `DESIGN.md`, `~/.claude/skills/ponytail/SKILL.md` (full), `~/.claude/skills/impeccable/SKILL.md` + `reference/craft-floor.md` adapted to egui. Own: `crates/lumen-app/src/{app.rs (toolbar only),style.rs}`, a new `crates/lumen-app/src/brand.rs`, `crates/lumen-app/assets/**`, `scripts/generate-icon.sh`. Nothing else.

## Decisions already made by the orchestrator (implement exactly)

1. **Geometry source = SVG files in `crates/lumen-app/assets/`.** Replace `icon.svg` with `docs/design/lumen-brand-v1/assets/lumen-app-icon.svg` and add `lumen-mark.svg` (copy of `lumen-mark-saffron.svg`).
2. **`scripts/generate-icon.sh` renders those SVGs with AppKit** (Swift `NSImage(contentsOf:)` loads SVG natively on this macOS via `_NSSVGImageRep` — verified by the orchestrator; draw into an `NSBitmapImageRep` of the exact pixel size). Delete all old flame/diya drawing code. It writes: the standard iconset sizes from `icon.svg` → `iconutil` → `assets/AppIcon.icns`; and `assets/lumen-mark-28.png` + `assets/lumen-mark-56.png` from `lumen-mark.svg` (transparent background, transparent internal gap). Run it, commit outputs. No new tools/dependencies (no rsvg, no Python).
3. **In-app mark = those two PNGs**, `include_bytes!`, decoded once with the existing `image` crate into egui textures (lazy, cached — e.g. `OnceLock`/`ctx.data` or a field), drawn at 28×28 pt; pick the 56 px texture when `pixels_per_point > 1.5`, else 28 px. No per-frame decode or texture creation, no SVG parsing at runtime.
4. **Lockup (new `brand.rs`, one function used by `toolbar`):** mark 28 pt, 8 pt gap, "Lumen" Poppins SemiBold 18 pt `style::TEXT`, 8 pt gap, "PRESENTER" Poppins Medium 11 pt `style::TEXT_DIM` with ~1.3 pt letter spacing (LayoutJob `extra_letter_spacing`). Optically centred on the first toolbar row. Static: not focusable, no click. Hover tooltip "Lumen Presenter · <show name>" (Gujarati-safe: plain tooltip text is fine; do not break shaping elsewhere). Full ≈188 × 28 pt.
5. **Variants by first-row width** (`ui.available_width()` at the start of the row): Present: full ≥ 1200, mark + Lumen ≥ 1000, else mark only. Prepare: mark + Lumen ≥ 1200, else mark only (the name field needs the room). Pure function `variant(mode, width) -> Variant` with a unit test. Never shrink text below 11 pt.
6. **Present mode:** the brand replaces the 20 pt show-name label. The show name is otherwise unchanged (data, window, saving untouched). `Sunday Gathering` in lumen-core is demo data: do not touch it.
7. **Prepare mode:** brand, then the existing project-name `TextEdit` (same `self.project.name` binding, so rename/dirty/undo/save keep working) with `desired_width(160.0)` and hint "Show name", then the separator and Prepare/Present as today. If the name contains Gujarati, keep the existing behaviour that shapes it (look at how the current code/inspector handles Gujarati fields and reuse it; do not regress).
8. Everything after the brand/name in row 1 (separator, Prepare/Present in that order, separator, Audience/Stage/Remote pills, "Unsaved changes") and the whole second row stay exactly as they are. `horizontal_wrapped` stays.
9. No audience/stage/output changes. No new style tokens beyond, if useful, `BRAND_MARK: f32 = 28.0` in `style.rs`.

## Acceptance

- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.
- `scripts/generate-icon.sh` runs cleanly and regenerates byte-stable-enough outputs; `grep -i -E "flame|diya|glow" scripts/generate-icon.sh` finds nothing.
- Do not launch the app or take screenshots (the orchestrator verifies natively at several widths and scales). Report `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/f1.md`: files changed, decisions, anything you could not do. Final reply ≤ 8 lines.
