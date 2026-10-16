# E3b: decks that fill the audience screen, with a presenter view

User feedback (2026-10-16, photo of a real run on laptop + 27" external): "Keynote presentations have not been connected so they don't work. Lumen was working through the PowerPoint, but I couldn't get the presenter view, and on the bigger audience screen the PowerPoint wasn't taking over the space; it felt like an upsized, mirrored copy." They also want captions, lower thirds and logos on top of PowerPoint (E3a builds the overlay layer; it only works when Lumen draws the deck).

Orchestrator findings on this Mac (PowerPoint 16 installed, **Keynote not installed**, two displays, mirroring off):

- Play mode bug: `run slide show` with the deck's default `show type` (`slide show type speaker`, `show with presenter` false) plays the plain show on both screens. Setting `show type of (slide show settings of p)` to `slide show type presenter` before `run slide show` gives PowerPoint's presenter view on the laptop and a full-screen slideshow on the external display (verified with screenshots).
- PowerPoint exports a deck to PDF from AppleScript with no permission prompt when the output is inside its sandbox container: `save p in (POSIX file "<home>/Library/Containers/com.microsoft.Powerpoint/Data/tmp/x.pdf") as save as PDF` (4 slides, 1.9 s). `out` is a reserved word in PowerPoint's dictionary; do not use it as a variable name. Notes via `content of text range of text frame of place holder 2 of notes page of slide i of p` returned `missing value` on a deck without notes; treat errors and "missing value" as empty.
- `.key` fails today because Keynote is missing and PowerPoint cannot open Keynote files.

Own `crates/lumen-app/src/{deck.rs,file.rs,add.rs,pdf.rs}`, the `Document` struct in `crates/lumen-core/src/model.rs` (and the three `Document {…}` literals in core/interchange/media/books that need the new field), and in `panels.rs` only the running-order/cue context menu and the notes display. A parallel worker E3a edits the rest of core, `render.rs`, `books.rs`, `tools.rs`, `inspector.rs`, `app.rs`: keep your edits there to the minimum and say exactly where in the report. Read `AGENTS.md`, `DESIGN.md`, `~/.claude/skills/ponytail/SKILL.md` at **ultra**, and for UI `~/.claude/skills/impeccable/SKILL.md` + `~/.claude/skills/taste-skill/SKILL.md` adapted to egui. Tests first for pure logic.

## 1. Decks become Lumen slides by default

- Adding/importing `.pptx`/`.ppt` (and `.key` when Keynote is installed) converts the deck to PDF in its own app on the existing file worker thread, then reuses the existing PDF page import: one document named after the file, one slide per page. This is what makes the deck fill the audience output, lets overlays and captions sit on top, and makes Lumen's console (current + up next + notes) the presenter view.
- Conversion (in `deck.rs`, one function per app, scripts as `const` strings, paths **only as argv items**):
  - open with the existing `open_with(bundle_id, path)`; AppleScript gets `[source, output]`; pick the presentation/document whose full name/file matches the source (fallback: the frontmost one); export PDF; collect one notes string per slide (PowerPoint notes placeholder; Keynote `presenter notes of slide i`); close it without saving only if it was not already open before.
  - output path inside the app's sandbox container (`~/Library/Containers/com.microsoft.Powerpoint/Data/tmp/lumen-<uuid>.pdf`, Keynote `~/Library/Containers/com.apple.iWork.Keynote/Data/tmp/…`), then move it to Lumen's data dir `decks/<stem>-<hash(path, len, mtime)>.pdf`; reuse that cached PDF when present.
  - 180 s timeout, inline progress "Converting <file> in PowerPoint…", errors inline (never a modal). Verify the PDF page count matches the slide count reported.
- Notes go into `Slide.notes`. Present mode shows the live slide's notes (small, under the Up next preview) when non-empty, if it does not already.
- `.key` without Keynote: inline error "Keynote isn't installed. Get it free from the App Store, or export the deck as PDF." with a **Get Keynote** button opening `macappstore://apps.apple.com/app/id409183694`. Same for PowerPoint files with no PowerPoint (message: "Install PowerPoint or Keynote, or export the deck as PDF").

## 2. Play in PowerPoint/Keynote stays as the animated option

- `Document.source: Option<PathBuf>` (`#[serde(default)]`) = the original deck path for converted decks.
- Context menu on a deck document (running-order row and its cues): **Play in PowerPoint instead** / **Play in Keynote instead** with the hint "keeps animations; overlays hidden" → applies `PlayDeck { path: source }` live. Existing documents with a PlayDeck slide keep working.
- Fix `POWERPOINT_OPEN`: set `show type` to `slide show type presenter` before `run slide show` (keep the existing retry loop and the comment about not changing the slide range).

## Acceptance

- Unit tests: argv builders keep paths out of script text, cache-path hashing changes with mtime/len, notes parsing (missing value → empty, count mismatch handled), app selection incl. Keynote-missing message.
- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.
- Do not launch PowerPoint/Keynote or the app, no screenshots; the orchestrator verifies natively with a real deck on two displays. Report `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/e3b.md` (updated as you go). Final reply: 10 lines max.
