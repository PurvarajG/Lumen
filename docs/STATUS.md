# Lumen Presenter status

Objective: standalone Rust alternative to ProPresenter for sabha/mandir AV volunteers (see PRODUCT.md). Plan: docs/superpowers/plans/2026-10-16-finish.md. Orchestrated by Claude Opus 5.5; executed by Codex and Antigravity workers in git worktrees, reviewed and merged by the orchestrator.

## Release candidate 1.0.0-rc.1 (2026-10-16)

`scripts/package-macos.sh --release` → `.local/dist/Lumen-Presenter-1.0.0-rc.1-macos-arm64.dmg` (ad-hoc signed, not notarized). `hdiutil verify` and `codesign --verify --deep --strict` pass; bundle contains no ChitraUni or book text.

Crates: lumen-core (schema v2: text boxes, markup, objects, persisted timers, props, messages, history), lumen-media (FFmpeg video, thumbnails, rodio audio), lumen-text (cosmic-text shaping; Poppins bundled, ChitraUni loaded from installed fonts, Noto Sans Gujarati fallback), lumen-interchange (OpenLyrics, text, experimental .pro text), lumen-control (HTTP token API, OSC, MIDI), lumen-library (anirdesh.com downloader, 15 parsers, search, slides), lumen-app (Present/Prepare console per DESIGN.md).

## Verification at rc.1

- `cargo fmt --all -- --check`, `cargo test --workspace` (194 passed, 0 failed), `cargo clippy --workspace --all-targets -- -D warnings` (clean).
- Native, by the orchestrator with window-only captures: console in Present mode at 1500×982; Gujarati slide shaped correctly in preview, thumbnails and live monitor; Return sends the on-deck cue live and the primary button switches to Next; release bundle launches with the new console.
- Real network: lumen-library downloaded 3 Vachanamrut units and 3 kirtans at ≥ 1.1 s per request.

## Not verified / known gaps

- The Books panel download → browse → add-to-running-order flow and Cmd+F search were not exercised by hand (synthetic input could not reach the app without Accessibility permission); covered by unit tests only.
- Audience/stage windows on a second physical display, fullscreen placement and Identify are implemented but untested on multi-monitor hardware.
- Transitions, props and message banner rendering verified by tests, not by native capture.
- Notarization, Windows/Linux, NDI/Syphon/SDI, DMX, timecode, recording/streaming and cloud sync are out of scope for 1.0 (PARITY.md).
- FFmpeg/ffprobe remain external requirements for video and thumbnails.

## Release candidate 1.0.0-rc.2 (2026-10-16)

Wave E3 below. `scripts/package-macos.sh --release` → `.local/dist/Lumen-Presenter-1.0.0-rc.2-macos-arm64.dmg` (ad-hoc signed, not notarized). `hdiutil verify` and `codesign --verify --deep --strict` pass; Info.plist reports 1.0.0-rc.2; no ChitraUni in the image; bundled binary passes `--smoke-test`.

## Wave E3: decks, captions and overlays (2026-10-16)

From user feedback on a real two-screen run (laptop + 27" external): PowerPoint mirrored a plain show on both screens with no presenter view, Keynote decks did nothing, book captions sat mid-screen, and there were no lower thirds or logos.

- Decks: `.pptx/.ppt` (and `.key` with Keynote) export to PDF through their own app into its sandbox container, cached by path/size/mtime, one PDF slide per page with speaker notes. They fill the audience output and overlays sit on top; the console (live, up next, notes) is the presenter view. Context menu "Play in PowerPoint/Keynote instead" keeps animations and now starts PowerPoint in presenter mode (presenter view on the laptop, full-screen show on the external — verified by screenshots). Keynote is not installed on the test Mac: `.key` shows "Keynote isn't installed" with a Get Keynote button; the Keynote export script is untested.
- PDF render fix: `CGPDFPageGetDrawingTransform` never scales up, so a 720×405 pt slide was drawn at 1:1 in the middle of a 2560 px output. Pages now scale to fill; regression test renders above natural size.
- Captions: a slide with a fully transparent background keeps the background underneath (video keeps playing, deck page stays). Book slides use a "Caption" theme: bottom-aligned, bottom black gradient, text shadow. Clear Slide hides the caption and keeps the underlay.
- Overlays: lower-third and logo presets with per-overlay On/Off (`CueAction::HideProp`), chips in Present mode.

Verified: fmt, clippy -D warnings, 235 workspace tests. Native on the external display via the HTTP API: video → caption over playing video → deck page full-screen → lower third + logo → caption over the deck; operator console shows the Overlays chips. The PowerPoint export script was run against a real deck (closed and already-open cases).

Not verified: drag-and-drop/Add files of a deck through the UI and the context-menu Play path (no synthetic clicks); Keynote; logo picking opens the file dialog on the UI thread, so outputs pause while it is open (set logos up before the service).

## Branding handoff (2026-10-16)

Prepared `docs/design/lumen-brand-v1/` and sibling ZIP for Claude to integrate the selected saffron L/projected-light identity. Includes editable vector redraws, outlined Poppins lockups, transparent PNGs, macOS ICNS/iconset, offline proof sheet, concept references, scoped implementation instructions, colour/sizing tokens and a ready-to-paste Claude prompt. The current console layout and full feature set are authoritative; generated mockups are illustrative. Remove the prominent Present-mode show heading while preserving editable/persisted project names in Prepare mode.

Package asset validation is documented in `VERIFICATION.md`. Integrated in wave F1 below.

## Release candidate 1.0.0-rc.3 (2026-10-16)

Wave F1 below. `.local/dist/Lumen-Presenter-1.0.0-rc.3-macos-arm64.dmg` (ad-hoc signed, not notarized): `hdiutil verify` and `codesign --verify --deep --strict` pass, Info.plist reports 1.0.0-rc.3, bundled AppIcon.icns matches the generated icon, no ChitraUni, bundled binary passes `--smoke-test`.

## Wave F1: brand lockup and app icon (2026-10-16)

Integrated the approved saffron L / projected-light identity as a small branding change; the console layout is unchanged.

- Header: the 20 pt show-name heading in Present is replaced by the lockup (28 pt mark from embedded 28/56 px PNGs, cached once; "Lumen" Poppins SemiBold 18; "PRESENTER" Poppins Medium 11, TEXT_DIM, tracked). It is static, with tooltip "Lumen Presenter · <show>". Present steps down to mark + Lumen below a 1200 pt first row and to the mark below 1000 pt; Prepare shows mark + Lumen (mark only below 1200 pt), a separator, then the existing show-name field (160 pt, "Show name" hint, same `project.name` binding). Everything after it in row 1 and all of row 2 are untouched. Demo data and saved names are unchanged.
- Icon: `assets/icon.svg` and `assets/lumen-mark.svg` are the single geometry source. `scripts/generate-icon.sh` renders them with AppKit (no flame/diya code, no new tools) into `AppIcon.icns` (10 sizes) and the two mark PNGs. The packaged bundle's `AppIcon.icns` is byte-identical to the generated one.

Verified: fmt, clippy -D warnings, 237 workspace tests. Window-only before/after captures (`.local/qa/f1/shots`) in Present and Prepare at 1500, 1280 and 1024 pt on the Retina laptop (2x), and 1500 pt on the L27q (1x): the mark is crisp with its diagonal gap at both scales, and control order and row 2 match the before shots. The header row is about 3 pt taller because the mark is 28 pt.

Not verified by hand: typing a new show name and saving (synthetic keystrokes and value setting do not reach egui text fields here; the field keeps the same binding and dirty tracking as before), the brand tooltip, and the Dock/Finder rendering of the installed app (Quick Look thumbnailing hung).

## Release candidate 1.0.0-rc.4 (2026-10-18)

Wave G1 below. `.local/dist/Lumen-Presenter-1.0.0-rc.4-macos-arm64.dmg` (ad-hoc signed, not notarized): `hdiutil verify` and `codesign --verify --deep --strict` pass, Info.plist reports 1.0.0-rc.4, no ChitraUni in the bundle, bundled binary passes `--smoke-test`.

## Wave G1: fixes from the first live service (2026-10-17/18)

Plan and evidence: `docs/superpowers/plans/2026-10-17-live-feedback.md`; captures and scripts in `.local/qa/g1/`. Executed by Sonnet 5.5 workers, reviewed and merged by the orchestrator.

- Video (G1.1): frames are paced on a steady clock instead of `ffmpeg -re`, decoded at the source rate (≤ 60 fps) and size (≤ 1080p), and the console and audience/stage windows repaint every frame while video or a transition plays (was every 80 ms). A 30 fps clip measured 69 → 122–132 distinct frames in 4 s (mean 58.6 → 30–33 ms, worst 83 → 42 ms) on the live monitor and the audience window; about 15–19 % of one core for Lumen plus ffmpeg's share.
- Repeated running-order items (G1.2): Next used to loop back to the first copy of a document that appears twice; the runtime now tracks the live occurrence (`Runtime::cursor`), rows light LIVE/up-next by row, HTTP Trigger and `/status` use the exact occurrence.
- Transitions (G1.3): show-wide transition (default 400 ms fade, saved in the project, Present "Transition" picker); a slide's Cut means "use the show transition". Clears fade, Blackout stays instant. A new video/photo holds the previous picture until its first frame (no black flash, also with Cut). Smoothstep easing. Fades measured at about 60 fps.
- Cue flow (G1.4): in Present, going live arms the next cue (selection follows the order across documents), so Space, Return and the button always send the next thing; the button reads Next, or "Go live: <cue>" when the operator picked something else; the Preview heading reads "Up next". Next at the end of the order does nothing.
- Running order (G1.5): rail 300 pt, resizable 240–520 pt; the list takes the full height (12 rows at 1500×949, was about 7); Macros collapse in Present; New document is Prepare only.
- Previews (G1.6): three thumbnail workers, failed thumbnails retry after 10 s (missing files do not), every preview surface shows Loading / "Can't preview" / "File not found" instead of black; the Preview shows live media only when the selected cue is the live one.
- Audio/video transport (G1.7): now-playing strip under the transport for audio cues and unmuted video: play/pause, elapsed / remaining, progress, stop (audio fades 300 ms), live volume for audio. Fixed: ffmpeg-made `.m4a` files were rejected by the decoder ("Unrecognized format").

Verified: fmt, clippy -D warnings, 261 workspace tests. Native on the laptop display with window-only captures and HTTP control: cadence numbers above, fade frames, eight-step Next walk across document boundaries and the repeated item, click-to-arm then Space, 12-row rail and 520 pt drag, loading/missing/corrupt placeholders, pause/resume/stop for audio and video.

Not verified by hand: the external display (not connected during this wave; laptop ProMotion + 60 Hz external is the case to re-measure with `.local/qa/g1/measure-cadence.sh`), Add files… through the file dialog (shows were built from JSON), book-preview flows after arming, grid scroll-into-view, gapless audio looping. Known gaps: the big Preview is a stretched 320×180 thumbnail (soft); video volume is not adjustable live; no seeking; a slide cannot force a hard cut while the show fades; panels are resizable but not dockable (pushed back in the plan).
