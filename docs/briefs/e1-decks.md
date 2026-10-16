# E1: vector PDF slides, Play-in-PowerPoint/Keynote cue, OGG audio

User intent: a deck someone worked hard on must look exactly as authored. Static decks → PDF pages rendered from vectors at each output's real pixel size (never JPEG, never upscaled bitmaps). Animated decks → the author's own app (PowerPoint/Keynote) plays the slideshow while Lumen's Next/Previous/remote drive it. Do NOT parse or render PPTX/Keynote files yourself.

Own `crates/lumen-core/**`, `crates/lumen-media/**`, `crates/lumen-app/**`. Read `AGENTS.md`, `DESIGN.md`, follow `~/.claude/skills/ponytail/SKILL.md` at **ultra** (smallest complete change, question any extra). Tests first for pure logic. macOS system frameworks only for PDF; no new rendering dependency beyond objc2 framework bindings matching the existing objc2 0.5.x / 0.2.x family.

## 1. PDF pages as slides

- Core: `Background::Pdf { path: PathBuf, page: u32 }` (1-based; validate ≥ 1). Additive to schema v2 (no version bump).
- Media (`lumen-media`, `#[cfg(target_os = "macos")]`, clear `Unsupported` error elsewhere): `pdf_page_count(path) -> Result<u32>` and `render_pdf_page(path, page, max_w, max_h) -> Result<Frame>` using CoreGraphics (`CGPDFDocument` / `CGPDFPage` drawn into an RGBA bitmap context with the page's crop box scaled to fit, aspect preserved, white page background as authored) or PDFKit — your choice, state why. Bounds: file ≤ 512 MiB, pages ≤ 2000, output ≤ 8192×8192, page out of range → error. Must be callable from a worker thread. Add PDF to `ThumbnailCache` keying (path, mtime, page, dims).
- App: Import… accepts `.pdf` → one document named after the file, one slide per page (label "Page N", background Pdf, empty text), added to library + running order. Rendering: audience/stage/live monitor request the page at that output's physical pixel size (pixels_per_point × rect) on a worker thread, letterboxed on black; previews/thumbnails at their own size; show the last good frame while a new size renders; re-render on output resize. No per-frame decoding.
- Tests: generate a tiny valid PDF in the test (handwritten minimal PDF bytes with 2 pages of vector rectangles/text, or `cupsfilter` if present — hermetic preferred), check page count, rendered dims fit-and-preserve aspect, non-uniform pixels, out-of-range page, missing file, garbage file.

## 2. Play in PowerPoint / Keynote cue

- Core: `CueAction::PlayDeck { path: PathBuf }`; `OutputState.deck: Option<PathBuf>` (set by PlayDeck, cleared by ClearSlide/ClearAll/StopDeck); `CueAction::StopDeck`. Validate path non-empty.
- App module `deck.rs`: pick app by extension (`.key` → Keynote `com.apple.iWork.Keynote`; `.ppt`/`.pptx` → PowerPoint `com.microsoft.Powerpoint`, else Keynote if installed) using NSWorkspace URLForApplicationWithBundleIdentifier; neither installed → inline error "Install PowerPoint or Keynote to play this deck, or export it as PDF". Control via `osascript` child processes on a worker thread, 10 s timeout, never on the UI thread. **Pass the file path only as an argv item** (`osascript -e 'on run argv … end run' -- <path>`); never interpolate paths into script text. Commands: open + start slideshow from slide 1, next, previous, stop. Keep the scripts as `const` strings; unit-test the argv builder and app selection.
- Behaviour: triggering a PlayDeck cue starts the deck; while `deck` is active, Lumen's Next/Previous (buttons, Space/arrows/Page keys, HTTP/OSC/MIDI commands — same code path) go to the deck instead of Lumen cues; Lumen's audience window hides (or goes to the back) so the slideshow is visible on that display, and returns when the deck stops. Live monitor shows "Playing in PowerPoint — use Next/Previous" with a Stop button. F2/F1 stop the deck. If the slideshow app quits or errors, clear `deck` and show the error inline.
- Import… accepts `.pptx`, `.ppt`, `.key` → a document named after the file with one slide whose label is "Play <file>" and whose actions are `[PlayDeck{path}]`, plus a short note in the slide notes: "Plays in PowerPoint/Keynote; export as PDF for still pages".
- macOS will ask once for Automation permission; add `NSAppleEventsUsageDescription` = "Lumen controls PowerPoint or Keynote slideshows when you play a deck." to the bundle Info.plist in `scripts/package-macos.sh` (only that key).

## 3. OGG audio

Enable OGG/Vorbis decoding in rodio (e.g. `symphonia-vorbis` + ogg container, or rodio's `vorbis` feature — whichever 0.21 provides; smallest). Test: generate a 1 s OGG with `ffmpeg -f lavfi -i sine=… -c:a vorbis -strict -2` and decode it (skip with a message if ffmpeg lacks the encoder). Audio picker keeps `ogg` only if this works; otherwise remove it from the filter.

## Acceptance

- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.
- Do not attempt screenshots or launching PowerPoint (the parent verifies natively). Leave changes uncommitted. Report `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/e1.md` (updated as you go): API, framework choice, tests, limits. Final reply: 10 lines max.
