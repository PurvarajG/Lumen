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

Package asset validation is documented in `VERIFICATION.md`. Application source and runtime assets remain unchanged; branding integration and native regression checks are pending Claude implementation.
