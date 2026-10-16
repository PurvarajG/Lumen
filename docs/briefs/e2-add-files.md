# E2: build a show from files — Add files, drag-and-drop, reorder

User feedback: "I don't see an easy way to import my videos, PowerPoints, PNGs, then order. The design isn't built with that in mind." The operator's mental model is: here are my files → put them in order → run the show. Today media is only a per-slide background set in the inspector, and File → Import only takes text/PDF/decks one at a time. Fix the workflow, not the plumbing.

Own `crates/lumen-app/**` only (core/media/interchange/library already provide everything needed). Read `AGENTS.md`, `PRODUCT.md`, `DESIGN.md` (binding), `~/.claude/skills/ponytail/SKILL.md` at **ultra**, `~/.claude/skills/impeccable/SKILL.md` + `reference/craft-floor.md` + `reference/onboard.md`, `~/.claude/skills/taste-skill/SKILL.md` §4.5–4.7, §9. Then `crates/lumen-app/src/{file.rs,panels.rs,app.rs,pdf.rs,deck.rs}` to reuse the existing import, PDF and deck paths. Tests first for the pure logic.

## 1. One way in for every file

- **Add files…** primary button at the top of the Running order (both Present and Prepare) opening a multi-select picker filtered to every supported type; and **drag-and-drop**: files dropped anywhere on the operator window are added (egui `ctx.input(|i| i.raw.dropped_files)`; show a full-window saffron-outlined drop overlay "Drop to add to running order" while `hovered_files` is non-empty).
- Added items go **after the selected running-order item** (or at the end if none), in the order given; a dropped **folder** adds its supported files (non-recursive) sorted by name (natural sort: "2.png" before "10.png").
- One pure function `plan_additions(paths) -> Vec<Addition>` decides by extension (case-insensitive):
  - `png jpg jpeg webp` → one document named after the file stem, one slide, `Background::Image`, empty text.
  - `mp4 mov m4v webm` → one slide, `Background::Video { looped: false, muted: false }`, empty text.
  - `pdf` → existing PDF page import (one slide per page).
  - `pptx ppt key` → existing Play-deck cue document.
  - `txt xml openlyrics pro` → existing interchange import.
  - `mp3 wav flac m4a aac ogg` → one slide with `audio: Some(AudioCue{volume: 1.0})`, solid black background, label = file name.
  - anything else → skipped; after the batch show one inline notice "Added 7 items · skipped 2 (notes.docx, clip.avi)". Never a modal.
- Work that touches disk (PDF page count, interchange parsing, folder listing) runs on the existing file worker thread; the UI shows "Adding 3 of 7…" inline. The whole batch is ONE undo step.
- Media-only slides must fill the output: image/video fit-to-canvas with letterbox (verify `render.rs` already does; fix if not).

## 2. Order by dragging

- Running-order rows are draggable (egui 0.33 `dnd_drag_source` / `dnd_drop_zone` or `Response::dnd_*`) with a visible grip handle (Phosphor `DotsSixVertical`), an insertion line where it will land, and the moved row selected afterwards. Works in Present and Prepare (remove the Prepare-only Up/Down buttons; keep Cmd+↑/↓ to move the selected row and Delete/Backspace to remove it from the running order — not from the library — when no text field has focus). Each move/removal is one undo step. Pure `move_item(list, from, to)` with tests.
- Slides inside a document reorder the same way in the cue grid (drag thumbnails) in Prepare mode.

## 3. Make it legible

- Each running-order row: 16:9 thumbnail of its first slide (media thumbnails via the existing `ThumbnailCache` worker; text slides via the existing renderer), type icon (image / video / PDF / deck / text / audio, Phosphor), name, and "12 slides" / "1:32" (video duration from `probe` if cheap, else omit). Row height ~56 pt, name truncates with tooltip.
- **Empty running order**: a large drop target filling the rail: icon + "Drop videos, pictures, PowerPoints or PDFs here" + **Add files…** button. Empty library/books states unchanged.
- Rename user-facing labels: "On deck" → "Up next".

## Acceptance

- fmt / test / clippy (workspace, `-D warnings`) pass. Tests: `plan_additions` mapping incl. case, unsupported, folder natural sort; insertion position after selection; batch = one undo step; `move_item` bounds and identity; delete removes from running order only.
- Do not take screenshots; the parent verifies natively. Leave changes uncommitted (parent commits). Report `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/e2.md`, updated as you go. Final reply: 10 lines max.
