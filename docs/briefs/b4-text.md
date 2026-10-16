# B4: lumen-text — shaped slide text rasterizer (Gujarati-correct)

Why: egui does not shape complex scripts, so Gujarati conjuncts/matras render wrong. Slides (audience/stage outputs, previews, thumbnails) will render their text through this crate into RGBA images that the app uploads as textures.

Own `crates/lumen-text/**` only (Cargo.lock may update). Depend on `lumen-core` (for `markup::Span`, `TextAlign`, `VerticalAlign`) and `cosmic-text` (current release; it shapes with harfrust/rustybuzz and rasterizes with swash). Read `AGENTS.md`, follow `~/.claude/skills/ponytail/SKILL.md`. Write tests first.

## Fonts

User decision: **Poppins** for English/Latin, **ChitraUni** for Gujarati.

- Poppins (OFL): download Regular, Medium, SemiBold, Bold, Italic, BoldItalic static TTFs plus `OFL.txt` from the official google/fonts repo (`ofl/poppins/`), put them in `crates/lumen-text/fonts/` with `fonts/README.md` listing source URL, commit and SHA-256 of each file, and embed with `include_bytes!`.
- ChitraUni is commercial (© 2008 Summit Information Technologies, no redistribution licence). NEVER copy it into the repo, a fixture, or the bundle. Load it at runtime: look for files whose name table family is `ChitraUni` in `~/Library/Fonts` and `/Library/Fonts` only (do not scan the whole system font DB; read at most the files whose filename contains `chitra`, case-insensitive, e.g. `CHITRAUNI REGULAR.TTF`, `CHITRAUNI-BOLD .TTF`, `CHITRAUNI-ITALIC .TTF`, `CHITRAUNI-BOLDITALIC BOLD ITALIC.TTF`), each ≤ 8 MiB. Expose `TextRenderer::gujarati_face() -> &'static str` returning "ChitraUni" or "Noto Sans Gujarati".
- Fallback: bundle Noto Sans Gujarati Regular+Bold (OFL, google/fonts `ofl/notosansgujarati/`, same provenance rules) for machines without ChitraUni.
- Families: one `Family` is enough for 1.0 — text runs in Latin use Poppins, Gujarati runs use ChitraUni (or the Noto fallback). Verify the per-script fallback really happens in a mixed line "Vachanamrut વચનામૃત". ChitraUni has real italic and bold-italic faces; use them.
- Tests must pass on machines without ChitraUni (CI): assert shaping with whichever Gujarati face is active, and add one `#[ignore]` test that requires ChitraUni and checks glyphs come from it.

## API

```rust
pub struct TextRequest<'a> {
    pub spans: &'a [lumen_core::markup::Span],
    pub width_px: u32, pub height_px: u32,      // target box, each 1..=4096
    pub font_px: f32,                          // 4..=600
    pub color: [u8; 4],
    pub outline: Option<(f32, [u8; 4])>,       // px width 0..=20
    pub shadow: bool,                          // soft drop shadow, fixed offset/blur proportional to font_px
    pub align: TextAlign, pub valign: VerticalAlign,
    pub line_height: f32,                      // multiplier, default 1.2
    pub shrink_to_fit: bool,                   // reduce font_px (down to 40 %) until text fits the box
}
pub struct RgbaImage { pub width: u32, pub height: u32, pub pixels: Vec<u8> } // premultiplied? state which; match egui ColorImage::from_rgba_unmultiplied
pub struct TextRenderer { .. }  // owns FontSystem + SwashCache + LRU cache of rendered images (capacity by bytes, default 64 MiB)
impl TextRenderer {
    pub fn new() -> Self;
    pub fn render(&mut self, req: &TextRequest) -> Result<std::sync::Arc<RgbaImage>, TextError>; // cached by request hash
    pub fn measure(&mut self, req: &TextRequest) -> (f32 /*used font_px*/, u32 /*text height px*/);
}
```

Span styling: bold → bold weight; underline drawn as a rule under the glyph run; colour overrides `color`; italic → use an italic face if present, otherwise render upright and document it (no fake skew). Newlines are hard breaks; words wrap at box width. Invalid sizes → `TextError`, never panic.

## Acceptance

- Tests: Gujarati shaping really happens — shaping "ક્ષ" and "શ્રી" yields fewer glyphs than code points and the glyph ids come from the Noto Gujarati face; mixed Latin+Gujarati line uses both faces; wrapping respects width; shrink_to_fit reduces size and fits; alignment (left/centre/right, top/middle/bottom) puts ink bounding box where expected; outline and shadow change pixels outside glyph ink; bold span differs from regular; colour span pixels carry that colour; cache returns same `Arc` for equal requests and evicts by byte budget; invalid inputs error.
- Bench-style test (ignored by default) printing ms to render a 1920×1080 box with 6 lines of Gujarati at 72 px; target < 40 ms in `--release`. Report the number.
- Write a tiny example `examples/render_png.rs` that writes a PNG (use the `image` crate as a dev-dependency only) of a Gujarati sample so the parent can look at it; put one output in `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/b4-sample.png`.
- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.
- Report `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/b4-text.md`: API, font provenance, timings, limitations. Final reply: 10 lines max.
