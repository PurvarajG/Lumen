# E3a: captions over anything, lower thirds and logos

User feedback (2026-10-16, photo of a real two-screen run): "the captions from anirdesh need to be put at the bottom of the screen, not bang in the middle. Give it a black gradient from the bottom so they can be seen and so they can be put as overlays on any video or PowerPoint. I need the ability to put up lower thirds overlays and logo overlays too."

Today every slide replaces the whole picture: a book slide paints its own solid background, so it cannot sit on a playing video or a deck page. Props already exist as a layer above slides, but they are a raw object editor nobody will find in a service.

Own `crates/lumen-core/**`, `crates/lumen-app/src/{render.rs,books.rs,tools.rs,inspector.rs,app.rs,panels.rs,style.rs}` (a parallel worker E3b edits `deck.rs`, `file.rs`, `add.rs` and the `Document` struct: do not touch those). Read `AGENTS.md`, `PRODUCT.md`, `DESIGN.md` (binding), `~/.claude/skills/ponytail/SKILL.md` at **ultra**, and for the UI parts `~/.claude/skills/impeccable/SKILL.md` + `~/.claude/skills/taste-skill/SKILL.md` adapted to egui. Tests first for core logic.

## 1. Core: transparent slides keep what is underneath

- A slide whose background is `Background::Solid(c)` with `c[3] == 0` is **transparent**. No new background variant, no schema bump.
- `OutputState.underlay: Option<Background>` (`#[serde(default)]`). In `Runtime::trigger`: if the new slide is transparent, `underlay` = the previous *effective* background (previous underlay if the previous live slide was transparent, else the previous live slide's background; `None` if nothing was live or media was cleared); otherwise `None`.
- `OutputState::effective_background(&self) -> Option<&Background>`: underlay when the live slide is transparent, else the live slide's background. ClearSlide / ClearAll clear `underlay`; ClearMedia keeps the existing `media_cleared` behaviour.
- `Theme.backdrop: Backdrop` (`#[serde(default)]`, enum `None` (default) | `BottomGradient`). Update `Theme::default` and every literal.
- `CueAction::HideProp { id }` removes one live prop (and its snapshots); validated like `TriggerProp`.
- Tests: caption over video keeps the video as underlay across several captions (Next/Previous too); opaque slide drops it; ClearSlide drops it; old project JSON without the new fields loads; HideProp leaves other props live.

## 2. Render

- `sync_media` and the renderer use `effective_background()`. A video under captions must keep playing without reload or flicker when captions change.
- Paint order bottom→top: effective background (solid / PDF page / image / video, letterboxed as today) → slide layer (backdrop, text, objects) → overlays (props) → message banner. A transparent slide with nothing under it shows black.
- Transitions: when previous and next effective backgrounds are equal, draw the background once at full opacity and transition only the slide layer (caption fades must not dip the video).
- `Backdrop::BottomGradient`: full canvas width, black from alpha ≈ 0.85 at the canvas bottom to 0 at (top of the text rect − 6% of canvas height), never shorter than 30% of the canvas; egui `Mesh` with per-vertex colours, scaled with the canvas so audience, live monitor, previews and thumbnails match.
- Previews/thumbnails of transparent slides: dark neutral canvas from `style.rs` so the gradient and text read.

## 3. Books produce captions

- Slides generated from anirdesh books use one project theme named "Caption" (find by name, else create): white text, `TextAlign::Center`, `VerticalAlign::Bottom`, `Backdrop::BottomGradient`, transparent `background_color`, font size chosen so two Gujarati + English lines fit comfortably at 1920×1080. Slide background `Solid([0,0,0,0])`, `text_rect` a bottom band (≈ x 0.06, y 0.60, w 0.88, h 0.34), `text_style.shadow = true`. Lower `max_chars` if needed so a caption never exceeds about 3 lines per language.
- Saved projects with old book documents are left as they are.
- Inspector: a "Bottom gradient" toggle on the theme so operators can make any text slide a caption.

## 4. Overlays (lower thirds and logos)

Keep the core `Prop` type; in the UI call them **Overlays**.

- **+ Lower third**: inline fields "Name" and "Second line" → a prop named after the name with: dark plate `Shape` (rgba 0,0,0,180, radius 6) at about x 0.05, y 0.74, w 0.42, h 0.14; a thin saffron accent bar on its left edge; name `Text` (semibold, ≈ 44 px at 1920 wide) and second line `Text` (≈ 30 px). Normalised rects only.
- **+ Logo**: image picker (png/jpg/webp) → corner (top-right default; four corners) and size S/M/L (8/12/18% of canvas width) → one `Image` object whose rect keeps the image aspect (read dimensions with the existing `image` crate) with a 3% margin. Image objects must paint aspect-fit inside their rect (check `paint_object`).
- Each overlay row: name, an **On/Off** switch showing live state (On = TriggerProp, Off = HideProp), collapsed "Edit" (existing objects inspector), delete (existing rules). "All overlays off" = ClearProps.
- Overlays must be reachable in **Present** mode without switching to Prepare: a compact Overlays section with one on/off chip per overlay, placed per DESIGN.md next to messages/timers. Creating and editing can stay in Prepare.
- Overlays are not shown while a deck plays inside PowerPoint/Keynote (that app owns the screen); say so in one line in the Overlays section only while `output.deck` is set.

## Acceptance

- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.
- Do not launch the app or take screenshots; the orchestrator verifies natively. Report `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/e3a.md` (updated as you go): API changes, render order, tests, limits. Final reply: 10 lines max.
