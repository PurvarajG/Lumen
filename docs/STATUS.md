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

## Release candidate 1.0.0-rc.5 (2026-10-18)

Wave G2 below. `.local/dist/Lumen-Presenter-1.0.0-rc.5-macos-arm64.dmg` (ad-hoc signed, not notarized): `hdiutil verify` and `codesign --verify --deep --strict` pass, Info.plist reports 1.0.0-rc.5, no ChitraUni in the bundle, bundled binary passes `--smoke-test`.

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

### G2.5 Wrap-up and the rc.5 native pass

- One native pass on the final build, laptop plus the L27q at 75 Hz: cadence (30 fps clip on the audience, 2/3 refreshes per picture: 83%, 100%, 98% of pictures; console LIVE monitor 29.3 fps, CPU about 14.5% of one core), the sharp Preview, Fit/Fill/Stretch (a show file with `"fit": "Fill"` loads and fills the L27q with no bars), and the docking checks above.
- Runtime and dependency notes: the only new dependency is `egui_dock =0.18.0` (MIT); FFmpeg, ffprobe and the optional ChitraUni are unchanged. No new runtime requirement.
- **Not verified by hand:** the audience at 60 Hz and 25 fps content beyond the cadence runs above; the undo step and the inspector `Scale` combo of G2.3; a pause/resume of a video through the UI (unit tests cover it); dropping a panel onto a tab bar to stack it (only an edge split was driven); the laptop-only 120 Hz case, where 30 fps video lands at exactly 4 refreshes in only 48–81% of pictures (the old code: 44–56%).
- **Known gaps:** no show-wide fit default and no fit option in Add files; PDF and deck pages always fit; no floating panel windows; the console repaints at about 60 Hz instead of 120 Hz while the audience plays video (deliberate, see G2.1); the video clock is not phase-locked to the display, so a 30 fps clip on a 75 Hz output still has an occasional 1/4-refresh split near the display's drift.
- Version 1.0.0-rc.5. Package: `scripts/package-macos.sh --release` → `.local/dist/Lumen-Presenter-1.0.0-rc.5-macos-arm64.dmg`.

### App icon in every launch mode (after rc.5)

The Dock and window icon was only set by the packaged bundle's `AppIcon.icns`, so a raw `lumen` binary (dev and QA runs) showed none. `main.rs` now passes the embedded `assets/AppIcon-512.png` (from the same icns) to the viewport, which eframe also uses for the macOS Dock. Verified in the Dock with the raw release binary. The rc.5 DMG was rebuilt with it.

## Wave G3: full-resolution output, LED-safe displays, portable shows, books in the app (2026-10-18)

Plan: `docs/superpowers/plans/2026-10-18-wave-g3.md`. QA files in `.local/qa/g3/` (shows, helper scripts, `notes.md` with the raw numbers). Commits: page reuse + `fetch_books`, full-resolution output, displays, portable shows, bundled books, kirtan fixes, rc.6.

**Rig change.** The L27q-4A was not connected today. The displays were the laptop (1512×982 @2x, 120 Hz) and two LG UltraFine (1920×1080 pt @2x = 3840×2160 px, 60 Hz). Audience tests ran on LG UltraFine (1), a real 4K panel, which made it a better 4K test and a worse 2560×1440 one. Every "on the L27q" check below means that display.

### G3.4a Books fetched once per page
- `run_download` keeps the last page and reuses it when the next unit has the same URL (diksha 5 units a page, vato 10, aksharamrutam 10); test: 10 diksha units make 2 requests. `crates/lumen-library/examples/fetch_books.rs <dir>` downloads the catalog one book after another at the existing 1.1 s rate and skips books with a `book.json`.

### G3.1 Full-resolution output
- Video decodes at the audience window's physical pixel size (inner size × pixels per point, published by the audience closure into an atomic), else the largest connected display, capped at 3840×2160 (`media::video_output_size`, tested). `fit_size` still never upscales. A video that is already playing keeps its size.
- Logos and overlay images use full-size textures on the audience, stage and LIVE monitor (`fullres.rs`): decoded on worker threads by the same function `load_image` uses (`media::decode_image`, 64 MiB and 8192 px limits), thumbnail shown until ready, entries evicted when no slide, prop or live output refers to them. Grids keep the thumbnail tier. Cost: every image the project refers to is held at full size (up to 256 MiB for an 8192² image), so a show with many huge logos uses real memory.
- **Plan addition (hardware decode):** 4K software decode in the app cost ffmpeg 49% + Lumen 37% = about 86% of a core (above the 60% bar), so `-hwaccel videotoolbox` is now passed on macOS (ffmpeg falls back to software for codecs it cannot take). With it: ffmpeg 27% + Lumen 38.5% = about 65%. An isolated 8 s decode: 3.0 s CPU in software, 0.85 s with VideoToolbox. No 2560×1440 cap was needed.
- Measured on the 4K display (30 fps clips, 60 Hz, `cadence-lossless.sh`): 4K clip decoded at 3840×2160, 98% of pictures stay up exactly 2 refreshes (96% in software); the 1080p clip with the same binary 85% (both runs show the same capture-start stall, so it is the capture, not the player). The 1080p clip costs about 36% of a core.
- Sharpness (variance of the Laplacian on the same crop of a 3840×2160 window capture; magick's units are arbitrary, the ratio is the point): 300×300 logo 2.88e30 → 8.24e30 (2.9×); 1920×1080 overlay prop text 1.08e30 → 2.87e31 (27×).
- Known gap: before the audience has ever opened, the fallback size is `CGDisplayPixelsWide`, which is the point size on a HiDPI display (1920×1080 for a 3840×2160 panel), so the first video of a session that starts before the audience opens decodes at 1080p.

### G3.2 Displays that survive LED processors and cable drops
- `match_display` order: same name and resolution; same name at any resolution; the only non-primary display. `target_display` adds the primary fallback; `needs_replace(placed, last, now)` is the pure "has the target moved" decision; all tested.
- **Plan deviation:** resetting `audience_placed` was not enough. A fullscreen window ignores a new position (it left fullscreen and came straight back on the same display), so the window is closed for one frame when its target changes and reopens windowed on the new display, then goes fullscreen a frame later, the same path as the first open. Same for the stage window.
- Verified natively with a temporary file-based "hide this display" hook in `enumerate_displays` (deleted): audience fullscreen on LG (1) → hide LG (1): moved fullscreen to LG (2) (the only non-primary left); hide both: windowed on the laptop; unhide: back fullscreen on LG (1). A saved resolution that no longer matches (3840×2160 saved, 1920×1080 reported) still opens on LG (1).
- The missing-display warning stays in the Outputs menu in the warning colour (now "until it returns") and in the status bar; there is no new always-visible element.
- Operator tip for an LED processor: set it to take a 16:9 input at its native resolution (1920×1080 or 3840×2160) at 60 Hz (50 Hz for 25 fps content), full-range RGB. Frame timing already follows whatever refresh the audience display reports.

### G3.3 Shows that move between Macs
- `Project::paths_mut()` visits all seven path fields (tested). `save_project` writes paths inside the show file's folder relative to it; `load_project` resolves relative paths against that folder; paths elsewhere stay absolute, old shows load unchanged; in memory paths are always absolute. A moved show folder still resolves (tests).
- File → **Save with media…** (`file::save_with_media`): name the folder in the save dialog; `<Name>/<Name>.lumen.json` plus `Media/` with one copy per source (` 2`, ` 3` on a name clash), deck PDFs from the cache included, progress "Copying media n of m…" in the status bar, missing files listed ("3 files were missing and not copied: …"), an existing folder is never touched, a failed copy removes the folder it created. After success the show points at its copies and the path is the new show file (if the show was edited meanwhile, only the copy-time snapshot is saved and the edits stay dirty).
- Opening a show counts missing media on a worker: a banner "N media files are missing — Locate folder… / Dismiss". Locate folder… relinks by file name (depth ≤ 4, shallowest first, indexed off the UI thread, 200,000 entries at most), one undo step, dirty, status "Relinked n media files; m still missing".
- `Settings.recent_projects` (8, newest first, canonical paths, updated on open, save and `--project`); File lists them under "Open recent" and Lumen reopens the newest existing one at launch (a show that fails to load: the demo opens and the error is shown; `--project` wins; smoke mode never touches settings). **Plan deviation:** an inline list, not a submenu, because egui's submenu arrow glyph renders as "?" (same issue as G2.3).
- **Found and fixed:** the `Recovery` folder was never created, so every autosave failed ("Couldn't save recovery … No such file or directory") unless the folder happened to exist.
- **Save as:** macOS maps the filter `lumen.json` to `json`, so a typed name "Show" saved as `Show.json`. Filters are now plain `json` (the plan's fallback); Save with media writes `<Name>.lumen.json` itself.
- Verified natively: a show with a photo, video, audio cue, logo prop and a deck page (a PDF from the deck cache): Save with media (20 MB, every path relative), folder moved elsewhere and opened with nothing missing, two files moved away → banner → Locate folder… relinked both, Open recent opened a show, relaunch without arguments reopened the last one.
- Not tested natively: a `.pptx` source copied by Save with media and "Play in PowerPoint" from the copy; Keynote; Save as with the suggested name untouched.

### G3.4 Books in the app
- `Books` has a second read-only root, `Contents/Resources/Books` found from `current_exe()`; per catalog book the user's copy wins, else the bundled one ("Included · n MB", Open book, no Delete). Dev builds without a bundle behave as before. Tests cover bundled only, user only, both (user wins) and neither (`load_installed`).
- `scripts/package-macos.sh` copies each `.local/books/<id>/book.json` and `Licenses/Anirdesh-books.txt` (© Anirdesh.com / BAPS, included with permission, not covered by the MIT licence) into the bundle; a release fails when a book is missing unless `ALLOW_MISSING_BOOKS="id id"` names it (**addition**, it prints a note) or `--without-books` is passed. `docs/RELEASE.md` has the `fetch_books` step; README says what is included.
- **Fetch result: 12 of the 15 books, 47.4 MB of `book.json`** (vachanamrut 274 units, vato 1484, kirtan 2164, diksha 315, kavya 55, chintamani 164, harililamrut 332, pbp 31, asv 39, gita 18, bhagvat 335, shloks 1). **Three failed and are not in the app**, none by a plan error I could fix with a selector:
  - `yogi-gita`, unit `foreword`: "no gu_text or en_text elements found". The real page uses `p.en_title`, `p.en_indent` and `span.gu_inline`; the parser was written from synthetic samples.
  - `karika`, unit `jiva-dhara` (the sixth; the first five parse): "no hi_center verse elements found". This is a prose section using `p.hi_indent` / `p.hi_subtitle`.
  - `aksharamrutam`, first request `…index.php?prakaran=8&start=1&increment=10`: HTTP 400, so the URL parameters are wrong.
  These books never worked against the real site (rc.1 only downloaded 3 Vachanamrut units and 3 kirtans). Parsers were not changed, per the plan; they need a real-sample fix, a good next piece of work.
- **Kirtan Muktavali could never finish downloading, for any user.** The plan asked for part 1 numbers 1..1300 but the site has 1101 there and serves its landing page for anything past that, so the book failed with "Layout error … unit 1-1102"; part 2 had gaps and was cut at 1191 of 1429; part 3 (299) was missing. Fixed: the plan now lists exactly what exists (part 1: 1..1101; part 2: 1..336, 1001..1162, 1164..1429; part 3: 1..299 = 2164 units), a missing number is skipped with a warning as a safety net, and a test checks that every book's plan length equals its catalog unit count (catalog counts for kirtan, kavya 600→55, bhagvat 347→335 and shloks 2→1 were wrong). This is very likely the user's "have to delete, then download" problem for that book.
- Download-bug time box: (b) kill -9 at 12 of 31 units of a book, relaunch, the saved queue resumed 17 → 31 and finished. (a) cancel then Download again was not driven through the UI (synthetic clicks into the Books list did not land); by reading, Cancel keeps the cache and Download resumes. (c) disconnecting Wi-Fi was not done. Added either way: a partly downloaded book shows "Resume · n of N", and each `DownloadEvent::Failed` is appended with time and book id to `Books/download.log`.
- Measured with 12 books: load + index 0.30 s (release, warm cache), first search 19 ms; RSS 138 MB without books, 298 MB with them (+160 MB, budget 400); idle CPU 0%. The load runs on the Books worker; the UI thread only swaps an `Arc`. Budget met, no lazy loader.
- Offline check: the packaged app (empty `LUMEN_DATA_DIR`) launched under `sandbox-exec -p '(version 1)(allow default)(deny network*)'` (curl under the same sandbox returns 000, so nothing could reach the network): Cmd+F "Gadhada I-1" lists the Vachanamrut passages and "Mukta anant" lists Kirtan Muktavali 2-1066, Nishkulanand Kavya and Bhaktachintamani. Some diacritic letters in snippets render as boxes (font coverage, not from this wave).
- UI: the new controls (banner, menu entries, Resume, Included) reuse existing patterns from DESIGN.md (inline `warn` notice naming the problem and the fix, the same buttons); the impeccable and taste skills were skimmed, not run in full, since nothing new was designed.

### G3.5 rc.6
- Version 1.0.0-rc.6. `ALLOW_MISSING_BOOKS="yogi-gita karika aksharamrutam" scripts/package-macos.sh --release` → `.local/dist/Lumen-Presenter-1.0.0-rc.6-macos-arm64.dmg`, **21.3 MB** (rc.5: 10.1 MB; the app is 65 MB). `hdiutil verify` valid; mounted, `codesign --verify --deep --strict` passes; Info.plist 1.0.0-rc.6; Resources/Books has the 12 books; Licenses has `Anirdesh-books.txt`; no ChitraUni; the bundled binary passes `--smoke-test`.
- Checks run on builds of the final code (the packaged binary has only the version number and docs added since): G3.1 sharpness and CPU, G3.2 re-match, G3.3 round trip and relaunch, G3.4 offline search from the bundle.
- Verification gate: `cargo fmt --all -- --check`, `cargo test --workspace` (295 passed, plus one ignored measurement test) and `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **For the user to check by hand:** (1) unplug and replug the external display while the audience is fullscreen (the audience should return to it fullscreen); (2) set the L27q (or any 60 Hz display) to 60 Hz once and run `cadence-lossless.sh` with the 30 fps clip; (3) "Save as…" with the suggested name untouched, and Save with media on a show that has a `.pptx` deck slide, then "Play in PowerPoint" from the copy; (4) Wi-Fi off, press Download on a half-finished book, then turn it on and press Retry (download bug c). **Keep a copy of the Anirdesh/BAPS permission** somewhere safe; the public download now includes the book texts.
- Not published: nothing pushed, no GitHub release; the README Download link still points to rc.5.
- Known gaps: Yogi Gita, Karika and Aksharamrutam cannot be downloaded or bundled; PSD import, LED pixel mapping and output regions are not started (later wave); the first video before the audience has opened decodes at 1080p on a HiDPI display.

### Footnote for future sessions: three books are still missing (decided 2026-10-18)

Yogi Gita, Swaminarayan Siddhant Karika and Aksharamrutam are not bundled and cannot be downloaded: their importers fail against the live site (causes under "G3.4 Books in the app" above). Package releases with `ALLOW_MISSING_BOOKS="yogi-gita karika aksharamrutam" scripts/package-macos.sh --release` until they are fixed. The fix is its own piece of work: fetch each real page once into `.local/`, rewrite the selectors/URL plan, add synthetic-markup tests (never commit book text), correct the catalog unit counts, run `fetch_books` for those books, then drop the allow-list and update README ("12 of 15"). Deferred by the user to a later session.

## Wave H1: Outputs menu and calmer console (2026-10-20)

Plan: `docs/superpowers/plans/2026-10-20-wave-h1.md`. Implemented from `3050514` on `development` by bounded Luna workers, reviewed and integrated one commit per implementation phase. No push or release package.

- Outputs uses inline display radio choices and `CloseOnClickOutside`, removing the nested-popup failure. Saved-display warnings, persistence actions and placement logic are preserved. Display labels use a comma.
- Controls share a 6 pt radius, surfaces/windows 8 pt, status pills fully rounded. Dock panes have rounded PANEL surfaces, 8 pt gaps on CANVAS and saffron drag feedback. Form outlines remain; secondary header/timer/message action rows use scoped ghost buttons. Primary and live actions keep their frames.
- Duplicate Preview/Inspector headings removed; Up next is a saffron caption; LIVE pills unified; selected running-order rows use RAISED while keyboard focus, up-next rails and on-air outlines remain. Timer kind/value share a row; pane sections use one heading scale. No new dependencies, shortcuts or pane arrangement changes.
- Status errors retain full text on hover. Only the confirmed deck-open/recovery-read error formats shorten absolute paths to filenames; unknown formats remain unchanged to preserve message text, spaced filenames and URLs. Regression tests cover these cases.
- Review corrections: retained the existing preview-height reserve (removing its heading already frees height) and removed the temporary ghost-helper dead-code allowance.

Verified on the integrated code: `cargo fmt --all -- --check`; `cargo test --workspace` (**296 passed, 0 failed, 5 ignored**); `cargo clippy --workspace --all-targets -- -D warnings`; final `cargo build -p lumen-app`; native `--smoke-test` exited 0 and logged audience/stage viewport requests plus the first cue trigger. Existing dependency notice: `block` 0.1.6 has future Rust incompatibilities. Logs/reports: `.local/logs/h1-*`; retained test binary: `.local/qa/h1/lumen-after`. Worker launcher restored; wave worktrees and shared build directory removed after verification.

Not verified: **zero before/after PNGs**. H1.0 could not discover a usable window; CUA reported app access was not approved. No alternate UI-access path was attempted after that denial. Native smoke confirms startup/control flow only, not visual correctness. Outputs radio/fullscreen/outside-click/Escape behavior, display selection persistence and physical output placement, 1500/1280 pt layout, pane resizing/drop feedback, Tools/Inspector controls and live timer interaction remain unverified. The requested 30 s interactive scrolling/dragging measurement with audience open was blocked for the same reason: **p50/p95 not measured**; no temporary frame logging was added or retained. Visual and performance acceptance remain outstanding.

For the user: with the L27q attached, select it under **Outputs → Audience**, open the audience output and confirm placement. Also report whether the console feels right, or which pane still feels boxy. Check against the retained test binary, not the installed rc.6 app. Native captures and interactive frame timing should be completed when UI access is available.

## Wave H2: twin-monitor outputs, smooth transitions, PowerPoint inside Lumen (2026-10-20)

Plan: `docs/superpowers/plans/2026-10-20-wave-h2.md`. Done on `development` by one Sonnet 5.5 session, one commit per phase, nothing pushed, no release package. Hardware: built-in 120 Hz console, two LG ULTRAFINE 3840 × 2160 at 60 Hz (audience on LG (1), stage on LG (2)), PowerPoint 16.114. Logs and captures are in `.local/qa/h2/` and `.local/logs/h2-*` (not committed).

### H2.0 `6e13e2a` review fixes and baseline
- Committed the four modified files (hover outlines, hover vs selection, console paced during fades).
- Method: `run-fades.sh` drives 7 × `Next` over HTTP every 1.5 s on `fades.lumen.json` (Fade 400 ms, one Push 500 ms) while `cadence-lossless.sh` captures LG (1) at 60 Hz for 12 s; `summ.py` counts refreshes per picture during fades.
- **Before** (binary of `6e13e2a`, stage open on LG (2)): first show version 105 pictures, 78 stayed 1 refresh, 18 stayed 2, 3 stayed 3 (the first picture of a fade); stage closed: 136 pictures, 94 % at 1 refresh. Re-run on the final show (identical header object): 92 / 97 / 96 / 97 % at 1 refresh, **3-4 pictures per run stayed 3 refreshes** (`before.txt`). The stage window costs the audience cadence.

### H2.1 `38fb40a` stable identity for twin monitors
- `DisplayKey.uuid` (ColorSync UUID via `CGDisplayCreateUUIDFromDisplayID`, raw FFI, no new crate). `match_display` order: same UUID; same name and size; same name; the only non-primary display. `upgraded_key` writes the UUID into an old saved key once, **only** when it was found by name (deviation: not for the "only other display" stand-in, which is not an identity and must not replace the saved display, and not when a saved UUID is merely absent). Tests: swapped suffixes pick by UUID, old keys still match, a vanished UUID falls to name then the sole non-primary.
- Native: an old key without UUID was upgraded in settings on first scan; chose LG (2) under Outputs → Audience, quit, relaunched, opened the audience: it opened on the LG (2) window position again.
- Which monitor is which (by macOS, not by eye): **LG (1)** = UUID `8FF0B586-7C64-4F2B-8EC2-012D1B53892A`, serial 503613, placed right of the built-in (x 1152); **LG (2)** = UUID `B7D965C8-1732-40F8-8EAE-C73CB71F49DD`, serial 503612, placed left (x -768). Outputs → Identify was not looked at by a person.

### H2.2 `39cab99` real pixel sizes
- `enumerate` takes width/height from the current display mode's pixels (`CGDisplayCopyDisplayMode`, fallback `CGDisplayPixelsWide/High`); x/y stay in points. Outputs now reads "LG ULTRAFINE (1), 3840 × 2160" and "Built-in Retina Display, 3024 × 1964" (seen). A saved key in points still matches by name and H2.1's write-back upgrades it (the audience key became 3840 × 2160 with UUID on first launch).
- **The G3.1 "Known gap" (first video before the audience opens decodes at 1080p) no longer applies**: `video_output_size` now sees 3840 × 2160 (clamped to 4K). A 4K clip started with the audience closed and then opened is sharp: Laplacian variance of a 960 × 520 crop 9.9e8 on LG (1) vs 1.1-1.3e9 for 4K reference frames and 8-9e7 for the same frames through 1080p.

### H2.3 `fa0f8b0` smooth transitions
1. Clock starts on the first audience frame: `OutputTransition.started` is an `Arc<OnceLock<Instant>>` (deviation from `Option<Instant>` with a write-back: clones share it, so the audience's start is also the console preview's start, with no mutex round trip). `progress()` is 0 until started; the audience closure calls `start()`; the console starts it when no audience window is open or after 250 ms. `created` replaces the old `started` for the media wait.
2. Unchanged captions and objects stay at full opacity: in the shared-background crossfade the incoming layer paints what looks identical on both slides once at opacity 1 and the outgoing layer skips it (`Common`, `same_caption`, `same_look`; object ids are ignored). Limit: it is per object and per caption block, so a caption whose second line changes still crossfades as one block (a per-line version is not built).
3. **Push text wobble: skipped.** Slide text is a texture painted by `text::paint` (`painter.image`), not epaint glyph text, so `round_text_to_pixels` has no effect on it, and the texture cache is keyed by content, not by position.
- **After** (stage open on LG (2), console on the built-in): 3 runs: 100 %, 97 %, 100 % of fade pictures stayed exactly 1 refresh, **none stayed 3 or more** (95 / 92 / 96 pictures); stage closed: 99 % and 100 %, none ≥ 3 (`after.txt`). Acceptance met by the numbers.
- **User check pending:** "watch one run of the fades show and say whether it looks smooth." No answer was available during this session; it is not recorded as passed.

### H2.4 `23e951c` spike: **GO**
`docs/research/powerpoint-window-capture.md`. Window-mode start without activate keeps Lumen focused; the window is capturable behind the fullscreen audience; `go to next slide` steps builds; 40-47 changed frames/s, 26-78 ms latency, PowerPoint ≤ 23 % CPU; `sourceRect` removes the title bar. Limits: Cmd+H or minimising freezes the capture; the show's end shows a black "End of slide show" screen; max window height is the display height − 30 pt.

### H2.5 PowerPoint inside Lumen (commit `feat(deck): play a PowerPoint deck inside Lumen's audience output by window capture`)
- New action `PlayDeckInLumen { path }` (`lumen-core`, also in the inspector's action list), `OutputState.deck_capture`; triggering any cue clears it (leaving the slide stops the show); StopDeck / Clear slide / Clear all clear it too; `PlayDeck` and `PlayDeckInLumen` replace each other. Test in `runtime.rs`.
- `capture.rs` (macOS, no-op elsewhere): a worker owns the show and an `SCStream` on the slideshow window (`objc2-screen-capture-kit`, `objc2-core-media`, plus `objc2` 0.6, `block2`, `dispatch2` under their own names beside the 0.5 crates; the sample buffer is read by two C calls, so no core-video crate). `open -g` loads the deck, a window-mode AppleScript (no `activate`) sizes the window 16:9 and places it on the audience display (behind the fullscreen audience), `sourceRect` crops to the slide. Frames go through `Latest` (newest only) into the live media texture. Right-click on a deck slide: **Play inside Lumen** and **Play in PowerPoint (fullscreen)** (Keynote keeps its single entry). README documents Screen Recording and the dependency.
- Behaviour: entering the deck holds the previous picture until the first frame, then Lumen's transition runs; Next/Previous from every source go to PowerPoint while it is live; a Next that would show PowerPoint's black end screen is held back (the last slide stays up, the show ends, Next moves on); no permission, no PowerPoint, an old macOS, no frame in 3 s or any start error → one inline `warn` banner (with "Open Privacy settings" for permission) and the deck plays fullscreen in PowerPoint; the show ending or PowerPoint quitting mid-show → last picture holds, status-bar message, Next moves on.
- **Deviations:** the scripts live in `capture.rs` (only `POWERPOINT_NEXT/PREVIOUS/STOP` are shared from `deck.rs`, now `pub(crate)`); capture is 2560 × 1440 at 30 fps rather than the audience's full pixel size: at 3840 × 2160 / 60 fps the release build used about 58 % of a core in Lumen alone, at 2560 × 1440 / 30 fps Lumen uses 25-28 % and PowerPoint peaks at 26 % (total about 54 %, under the 60 % target).
- Verified natively on a release build (smoke mode, audience fullscreen on LG (1)): the 3-slide test deck played inside the audience with both builds, the Push slide and the last slide; the end screen was never shown; Next then reached the next cue; a Fade into the deck waited about 3 s for PowerPoint, then crossfaded in; a Fade out crossfaded the last deck picture to the next cue; ending the show from outside held the last picture and showed "The slideshow ended. The last picture stays up; Next moves on."; a missing deck file produced the fallback banner with the reason.
- **Not verified:** the permission-denied path (`tccutil reset` would have hit the terminal's own permission, and the user was not asked); the first-run Screen Recording prompt; PowerPoint quitting mid-show; Cmd+H/minimise during a live deck; Keynote; a real animated deck with video; behaviour when PowerPoint's AutoUpdate dialog appears. After the fallback's fullscreen start fails (as with the missing file) the audience window comes back windowed, which is the existing deck-failure behaviour and was not changed.

### H2.6 verification
- `cargo fmt --all -- --check`; `cargo test --workspace` **310 passed, 0 failed, 5 ignored**; `cargo clippy --workspace --all-targets -- -D warnings` clean. Native `--smoke-test` exits 0.
- For the user to check by hand: (1) **twin monitors:** pick the monitor you want as Audience under Outputs, quit, relaunch, press Identify and confirm the audience comes back on the same physical screen, also after unplugging and replugging a cable; (2) **the fades show** (`.local/qa/h2/fades.lumen.json`, 120 Hz built-in console, stage open) and say whether it looks smooth; (3) **an animated deck in rehearsal:** right-click a deck slide → Play inside Lumen, step all builds with the clicker, leave with another cue, and try Play inside Lumen once with Screen Recording switched off for Lumen's host app to see the fallback banner.
