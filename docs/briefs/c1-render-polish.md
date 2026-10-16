# C1: shaped text in every output + console polish + displays

Own `crates/lumen-app/**`; tiny additions to `lumen-core`/`lumen-text` only if unavoidable (list them). Read `AGENTS.md`, `PRODUCT.md`, `DESIGN.md` (binding), `~/.claude/skills/ponytail/SKILL.md` (follow), `~/.claude/skills/impeccable/SKILL.md` + `reference/craft-floor.md` + `reference/polish.md`, `~/.claude/skills/taste-skill/SKILL.md` §4.5–4.7 and §9. Then `crates/lumen-text/src/lib.rs` and `crates/lumen-app/src/render.rs`. Tests first for pure logic.

## 1. Shaped text everywhere (lumen-text)

Replace egui text drawing in `render.rs::paint_text`/`paint_text_layer` with `lumen_text::TextRenderer` output uploaded as egui textures: slide primary text, Text objects, props, message banner and stage current/next text, in audience, stage, live monitor, preview and thumbnails. One `TextRenderer` owned by the app (behind `RefCell`/`Mutex` as needed), textures cached by request hash with an LRU byte budget (≤ 128 MiB GPU) and freed when evicted; render at the target's physical pixel size (use `pixels_per_point`) so text is crisp on Retina and outputs. Thumbnails render at their small size (do not downscale 1080p images per frame). Keep frame time smooth: never render more than ~4 new text images per frame; show the previous texture until the new one is ready. Gujarati UI strings (document names, search hits, timer/message names typed in Gujarati) also go through lumen-text via a small `gu_label(ui, text, size)` helper when the string contains U+0A80–U+0AFF; Latin-only strings stay egui labels. Settings shows "Gujarati font: ChitraUni" or "…: Noto Sans Gujarati (ChitraUni not installed)" from `gujarati_face()`.

## 2. Live isolation for props and messages

Triggering a prop or message snapshots its rendered content (objects / resolved template text with live timer tokens) into output state, like slides; editing the definition afterwards does not change what is on air until re-triggered. Test it.

## 3. Polish fixes (observed in the B1 screenshot at 1500×982)

- Right column clips content (timer row's Delete cut off). Every row must fit 360 pt: wrap or move secondary actions into a row menu ("⋯" drawn icon, not a glyph). Verify at the 1100×700 minimum and at 1500×982.
- Exactly one saffron primary in the transport: a single 40 pt "Go live" button that becomes "Next" once something from the current document is live (label switches; same position). Previous is a 40 pt secondary at the same height. Clear opens a menu listing F1–F6 actions with key hints; Blackout shows its active state in `live` red.
- Running-order Up/Down/Remove only in Prepare mode (Present shows a clean list; drag-reorder is optional, skip it).
- Header output status as real pills (dot + label, off = text_dim outline, on = ok, error = live) and remove the duplicate "Saved" (keep the status bar one, make header show only "Unsaved changes" when dirty).
- On-deck marker: the 3 pt saffron left rail + soft glow per DESIGN.md on both cue thumbnail and running-order item; LIVE pill on the live cue.
- Empty states per DESIGN.md (empty library, empty document, no timers/messages/props, no audio device).
- Focus rings visible for keyboard navigation (2 pt saffron), text selection colour and caret from palette.
- Replace any Unicode/emoji used as icons with an embedded line-icon font (Lucide or Phosphor, MIT/OFL; add licence + README provenance) at one weight.

## 4. Displays

Outputs window: per output (Audience, Stage) choose display (from `ctx`/winit monitor list via eframe viewport info; show name + resolution), Windowed / Fullscreen on that display, and "Identify" (shows a large number on each display for 3 s). Persist in app settings by display name + resolution; fall back to windowed on the primary display with a notice if it is missing. If exact monitor placement is not exposed by eframe 0.33, use viewport position from the monitor's origin; document what was verified on one display vs untested multi-display.

## Acceptance

- fmt / test / clippy (workspace, `-D warnings`) pass. Do not use `--smoke` as proof of visuals.
- Screenshots (window-only: get the window id with `python3 -I -c "import Quartz…"` CGWindowListCopyWindowInfo by owner PID, then `screencapture -x -o -l <id>`; never full-screen captures) of: Present mode at 1500×982 and 1100×700; Prepare mode; audience output with a Gujarati slide (use `Project::demo()` plus a slide with text "શ્રી સ્વામિનારાયણ\nVachanamrut વચનામૃત"), a prop and a message showing; stage output. Save to `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/c1-shots/`.
- Leave changes uncommitted (parent commits). Report `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/c1.md`, updated as you go. Final reply: 10 lines max.
