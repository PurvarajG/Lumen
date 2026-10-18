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

## Wave G2: projector timing, sharper preview, media fill, dockable panels (2026-10-18)

Plan: `docs/superpowers/plans/2026-10-18-wave-g2.md`. QA scripts in `.local/qa/g1/` (`g2-run.sh`, `cmd.sh`, `cadence-lossless.sh`, `g2-cadence.sh`).

### G2.1 Frame timing set by the audience window

- `MediaPlayer` no longer has a pacer thread. The reader thread still fills a 3-frame channel; `MediaPlayer::frames()` returns a cloneable `FrameSource` whose `frame_at(now, refresh)` picks the newest frame due at `now` (frame `n` is due at `start + n/fps`, `start` anchored a quarter refresh before the first call, after a resume and after a stall of more than 250 ms). The audience window pulls with its own `stable_dt` and uploads into the shared `lumen-video` texture; with no audience the root does it in `MediaState::poll`. `Published.video` is `Some` only while the audience is open, which is how only one side pulls (no separate bool).
- `lumen-video` is created as a 2×2 black texture when the player loads. `release_media_wait` now asks `FrameSource::has_frame()` so the previous picture is held until a real frame exists.
- `is_finished()` now means the reader has ended and the buffered frames were all handed out. Playback only advances while something pulls frames, so the player tests pull.
- **Plan deviation (root repaint):** while a video plays with the audience open the console repaints on a 16 ms timer instead of `request_repaint()`. With the console redrawing at its own 120 Hz, the audience window's passes landed 9–17 ms apart (it shares the main thread); with the timer they sit at 12–15 ms and there is exactly one pass per 75 Hz refresh. The LIVE monitor still shows 29.5 distinct frames per second.
- **Plan deviation (measuring):** `screencapture -V` is too noisy to judge this. A synthetic audience that flips black/white every 3rd pass, which the video code cannot influence, already shows 35% of its pictures as 27/53 ms splits with that tool. The numbers below use `cadence-lossless.sh`: an unencoded avfoundation capture of the whole display, cropped and shrunk in the filter, counting how many refreshes each distinct picture stayed up. Its own floor on the same synthetic flip is 89% exact.

Frame times on the L27q at 75 Hz (distinct pictures in 4 s; the display's real rate measures about 74.8 Hz from pass intervals):

| Clip | Old code, refreshes per picture | New code |
|---|---|---|
| 30 fps | 2/3 in 99%, 100%, 87% of pictures (3 runs) | 2/3 in 88%, 99%, 94% |
| 25 fps | 3 in 78%, 57%, 59% | 3 in 90%, 94%, 72% |

Console LIVE monitor while the audience plays: 118 distinct pictures in 4 s (29.5 fps). Laptop only (120 Hz ProMotion, audience closed), pictures at exactly 4 refreshes: old 45%, 44%, 56%; new 70%, 81%, 48%. With `screencapture -V` on the audience, 30 fps clip: 13 ms ×17–19, 27 ms ×45–50, 40 ms ×41–46, 53 ms ×8–10 (plan's before: 13 ×26, 27 ×43, 40 ×43, 53 ×12). CPU about 14% of one core in all cases.

The plan's 95% targets are not reached on every run. The residue is the display not being an exact multiple of the clip rate (74.8 Hz against 75 nominal, and a ProMotion laptop panel): the phase between frame times and refreshes drifts through the one-quarter-refresh margin every few seconds, where timing noise flips frames. A phase-locked clock that tracks the pass times would fix that; it is not in this wave.

Tip for the operator: a 75 Hz projector can never show 30 fps evenly; the best is alternating 2- and 3-refresh frames. If most content is 30/60 fps, set the L27q to 60 Hz in System Settings → Displays. 25 fps content divides into 75 Hz exactly.

### G2.2 Sharp Preview

- `lumen-media` thumbnails may now be up to 1920×1920 (was 1024). `ThumbnailState` has a second tier: `request_large` / `large_texture`, at most 1280×720, 6 textures (LRU), on the same three workers (each keeps only 2 large frames in its decode cache). Grid and running-order thumbnails still use the 320×180 tier, and `images()` still returns only that tier. Failures and retries follow the small tier's rules.
- The big Preview shows the grid thumbnail at once and swaps to the large one when it arrives; the live-texture rule is unchanged and still comes first.
- Verified natively (laptop window captures, same crop of the Preview, variance of the Laplacian via `magick`): photo cue 1.66e9 → 6.79e9, video cue 1.01e9 → 4.02e9 (about 4× sharper). A capture right after selecting a cue shows the small image (1.66e9), one 0.5 s later the sharp one.

### G2.3 Fit, Fill or Stretch for photos and videos

- `MediaFit { Fit, Fill, Stretch }` (default Fit) on `Background::Image` and `Background::Video` with `#[serde(default)]`, so old files load as Fit and the schema version is unchanged. `render::media_rect_uv` places the picture and its UV crop; `paint_scene` uses it for photos and videos, so audience, LIVE monitor, Preview, grid, running-order thumbnails and transitions follow. PDF and deck pages stay Fit. `MediaState::matches_background` ignores `fit`, so a fit change never restarts a playing video.
- Prepare inspector, Background: a `Scale` combo. Present and Prepare: the right-click menu of a cue tile (that slide) and of a running-order row (every photo and video slide in the document) lists Fit (show bars) / Fill screen (crop) / Stretch with a check, and "Applies the next time it goes live." while that cue is live. A choice marks the project dirty (one undo step through the usual history).
- **Plan deviation (menu shape):** the choices sit under a dim "Scale" caption in the same menu instead of a sub-menu, because egui's sub-menu arrow glyph is missing from the bundled fonts and rendered as "?". The deck "Play in PowerPoint instead" entry shares the same menu.
- Skipped on purpose: a show-wide default fit and a fit option in Add files.
- Verified natively on the L27q (captures in `.local/qa/g1/shots/g23-*`): 4:3 photo — Fit shows bars, picking Fill leaves the live picture as it was, re-sending fills the screen, Stretch fills and distorts; 9:16 clip — Fit bars, Fill picked while it plays does not restart it, re-sending fills. Core tests cover old files loading as Fit and a save/load round trip. Not checked by hand: the undo step and the inspector combo.

### G2.4 Movable, dockable panels

- New dependency `egui_dock =0.18.0` (MIT, `serde` feature); `cargo tree -i egui` still shows one egui, 0.33.3. New `crates/lumen-app/src/dock.rs`: `Pane { RunningOrder, Macros, Preview, Cues, Live, Tools }`, the TabViewer, both default layouts, validation, persistence and the dock style (tokens from `style.rs` only). The left and right `SidePanel`s are gone; the toolbar and status bar stay, `DockArea` fills the `CentralPanel`. Panes are never closeable and have no window surfaces. "Reset layout" (toolbar, after Shortcuts) resets only the current mode.
- `panels.rs` was split, not rewritten: `library` → `running_order` (+ `macros`, which fills its pane; the collapsing header, `bottom_reserve` and its test are deleted), `slides` → `preview` + `cues`, `inspector` → `live` + `tools`. Settings still open inside the Tools pane. While a book passage is previewed, Preview shows it and Cues says so in one line.
- Persistence: `Settings.layouts { present, prepare }`, written through `persist_settings` at a mode switch, on exit, and at most every 5 s while the layout differs from what was last written (compared as serialized JSON). A layout is checked at load (one surface, a well-formed tree, every pane exactly once where a split reaches it, at most 255 nodes); otherwise that mode gets the default and the settings notice reads "Panel layout was reset".
- **Plan deviations:** (1) `layouts` holds `serde_json::Value`, not `DockState<Pane>`: with a typed field one damaged layout would fail the whole settings file and reset the HTTP token and display choices. The rectangles egui_dock measures are zeroed before saving because an unmeasured one is infinite and JSON turns that into `null`. (2) `show_window_close_buttons` / `show_window_collapse_buttons` are deprecated in 0.18 and not called; with no window surfaces there is nothing to hide. (3) Drop targets use `OverlayType::HighlightedAreas` (edge zones) instead of the default small buttons, which matches "drag to an edge". (4) In egui_dock the split fraction is the share of the left or top node (the old node for `right` / `below`, the new one for `left` / `above`), so the Present default passes 0.22 for `split_left`. (5) The Preview pane reserves 72 pt under the picture for its label and notes (`preview_pane_height`); the book preview keeps the old `preview_height`. (6) `LEFT_WIDTH` / `RIGHT_WIDTH` are removed.
- Verified natively on the laptop (captures `.local/qa/g1/shots/g24-*`): Present default matches the old arrangement; Prepare shows a big running order over Macros and a small Preview; a CGEvent drag of the Preview tab onto the right edge of Cues moved it; a separator drag resized; switching modes and back kept each mode's own arrangement; quit (Cmd+Q) and relaunch restored both; `layouts` set to garbage gave the default layout and "Panel layout was reset" in Settings, no crash; Reset layout restored the default; Space and Return sent cues and HTTP `Trigger` / `Next` still worked.
- Known gap, not from this wave: in Prepare the selected running-order row's "Remove from order" button and name field overlap the row (same in rc.4).
