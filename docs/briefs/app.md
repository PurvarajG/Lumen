# Native application implementation contract

Own crates/lumen-app/**, examples/demo.lumen.json, scripts/run.sh, scripts/package-macos.sh only. Parent owns core/media/manifests/research/git. Workspace /Users/purvarajsinhgohil/Developer/PROPRESENTER. Read root AGENTS.md and the short public exports in core/media rather than the full conversation. Cargo is in /opt/homebrew/opt/rustup/bin. All application code Rust. Use eframe/egui =0.33.3 (official docs verified), image 0.25 (png/jpeg/webp), rfd 0.15, serde_json/uuid via workspace. Prefer default-features=false for eframe with default_fonts,wgpu,accesskit,x11,wayland; macOS builds target-specific dependencies. Add other small crates only if useful and explain. No webview or JS UI. All ownership excludes parent files; do not commit.

## Deliverable

A working native desktop app called Lumen Presenter. Default launch uses Project::demo. Include original sample JSON. Run with `cargo run -p lumen-app`; binary name `lumen`. Support `--project PATH` to load a show, `--export-demo PATH` to write the demo without a GUI, and `--smoke-test` to open audience and stage viewports, trigger the first cue and write basic view lifecycle messages to stderr (do not automatically exit; parent inspects). `--help` lists these exact options and exits without a GUI. Reject unknown flags and invalid project path with nonzero exit, readable message.

Use a modular app: main.rs CLI/launch, app.rs state/update, panels.rs or multiple panels modules, render.rs shared canvas text/image painting, media.rs background loader/texture state, file.rs save/load/unsaved actions. Avoid one huge file. UI code does not need mirror tests; test pure helper behavior (canvas fit, duration parse, text-fit bounds or user-operation state transitions) where it catches a real regression. Parent will visually exercise it.

## Visual direction

A precise production workbench, legible at a glance. Near-black main canvas (#11151b), blue-gray panels (#1b222c), thin borders, off-white main labels and muted secondary labels. Cyan (#66d9de) for selection and application controls, red (#f46d75) for LIVE/BLACKOUT states, amber for warnings. Consistent 8/12/16/24 spacing. Compact rectangular controls, restrained rounding, no decorative gradients. Body 14-15px, section labels 11-12px, hierarchy via typography not giant empty cards.

Default 1500×950 minimum 1100×700. Toolbar (~56px): brand LUMEN / PRESENTER, show name and dirty marker, New/Open/Save/Save As/Import Text, Audience/Stage toggles and a shortcut-help button. Bottom status strip: save/file status, media errors/activity and keyboard hints. No claim of ProPresenter parity or fictitious hardware status in product UI.

Left rail (~220px, resizable): playlist cue-order list with live and selected markers and document slide counts; up/down/remove from playlist; Library lists all documents with add-to-playlist; create/rename/delete documents. Core duplicate playlist occurrences allowed, but UI can add one document multiple times; maintain item identity when reordering. Document delete must remove its playlist refs but must not clear a cloned live snapshot. An obvious confirmation dialog for deleting a whole document is okay if inline and local (not a chat question).

Main area: selected document name + slide count and Add Slide / Duplicate / Delete / Move Up / Move Down. Responsive 16:9 thumbnail grid with 2-4 columns based on available width. Each tile paints actual text/background with slide label and cue number, selected cyan border, live red badge/border. Click selects for edit; double click or a visible Go Live control triggers. Do not trigger on a single selection click. Library/playlist selection must change selected doc and keep valid slide selection. Empty states should offer a working add action.

Right rail (~360px, resizable/scrollable): LIVE preview (16:9 with actual media/text), document/slide label, Previous / Next, Blackout, Clear Text, Clear All. Separate EDIT SELECTED section with multiline text and notes, label, theme selector, font size/color/alignment and vertical placement, background Solid/Image/Video with file picker, loop/mute for video. Make it clear that editing does not update output until Go Live. Theme changes are shared across editable slides; existing live snapshot remains detached. New/duplicate slides must get unique IDs. Clone media paths only; no binary media copying. Expose Cut transition; expose Fade duration only if it is implemented in actual output, otherwise leave the data field preserved and explain in docs.

Bottom dock or tabbed rail: TIMERS, STAGE MESSAGE and MACROS. Timers can be created/named, duration edited in stopped state, start/pause/reset/delete. Invalid duration input shows validation and never creates NaN/negative timers. Stage message field updates explicitly through Send / Clear, not every keystroke. Macro buttons execute Project macros and display failures. Allow creating/editing a bounded macro of supported CueAction values if practical; otherwise demo macro buttons work and document editor limitation. Handle `run_macro` failure visibly, no .unwrap on user input.

## Live canvas and output viewports

Share one renderer between thumbnails/live preview/audience output, so text placement agrees. It fits a 1920×1080 canvas into available space preserving aspect ratio (black letterbox outside). Draw solid/media background, then text at theme size scaled from canvas units; wrap text and shrink to fit actual available height/width. Apply text color and align Left/Center/Right, vertical Top/Middle/Bottom. Padding corresponds to ~90px on canvas. Notes are never in audience output. Blackout paints fully black; Clear Text suppresses text but keeps media. No live slide paints black.

Audience: independent native viewport titled "Lumen — Audience" with windowed/fullscreen option. Stage: independent viewport titled "Lumen — Stage" with current text, next cue from peek_next, notes, all timers and sent operator message. Stage remains useful during audience blackout. Closing either output turns its toggle off without closing operator. Both can be reopened. Default windowed; fullscreen explicit and use Escape only in that viewport to leave fullscreen. Never fullscreen the operator unexpectedly.

eframe deferred viewport closures require 'static owned state. Publish cloned small view state through Arc<Mutex<_>> or use an equivalent pattern; never hold that lock while decoding, rendering or blocking. Shared egui texture IDs are okay. Repaint outputs while timers/media active. Avoid propagating output close requests to the root viewport.

## Media integration

Consume lumen-media API after reading its lib.rs. Loading/probing/opening a video must run off the UI thread. A live cue change replaces the owned player; dropping/stopping it cleans children. Stale background-load results must be ignored/cancelled and dropped if a newer cue wins. Switching slides with the same background may keep media playing; document that policy. Do not restart video every frame. Blackout and clear-text are masks and do not stop media. Clear All stops live media.

Load still images off-thread with image decode limits and a file-size cap (64MiB); surface failed/missing media in the operator status, keep audience on theme/solid fallback, never render file paths/errors to the audience. Texture upload occurs only on a new frame sequence. No one decoder per thumbnail; thumbnail media placeholders are allowed while live preview/output uses the real frame. Expose pause/resume for current live video and show actual finished/loading/error state. ffplay audio only if user explicitly selects unmuted.

## User operations and reliability

- Atomic save/load uses lumen-core; extensions `.lumen.json` / JSON. Keep last chosen path, make Save As available, and mark dirty on all project edits. Timers/stage runtime changes alone do not dirty project.
- Keep audience/stage updates running while file pickers are open: use asynchronous rfd dialogs with polling/completion messages or a supported nonblocking equivalent, rather than blocking the root update loop in pick_file/save_file. Capture the project snapshot/path for asynchronous saves so later edits are not silently marked saved.
- Before New/Open/closing root with dirty edits, show Save / Discard / Cancel. Cancelled Save As or failed save must keep the original project and pending operation blocked. Use ViewportCommand::CancelClose for root while modal active. Do not discard without the user's local UI choice. New starts a fresh empty or original demo project with a clearly reset selection and output.
- Add bounded undo/redo for project edits if possible. If not delivered, explicitly record it instead of exposing dead controls.
- Text import from user file: use document_from_text, name from stem, add document and playlist item, select first slide. Limit imports to 4MiB. Clipboard paste is optional; no network calls.
- Root keyboard shortcuts: Cmd/Ctrl+S Save; Cmd/Ctrl+Shift+S Save As; Cmd/Ctrl+O Open; Cmd/Ctrl+N New. Without active text input or modal: Enter Go Live selected, Space/Right Next, Left Previous, B toggle blackout; digits 1-9 trigger selected-document cues. Keyboard shortcuts must not fire when typing slide text, notes, stage messages or duration/name fields. Display shortcut help.
- Render errors to operator, keep app alive. No panics from missing IDs after deletes. Do not let invalid editor state create an invalid savable project (clamp font sizes, remove dangling refs on deletions).

## Scripts and package

scripts/run.sh resolves project root, adds /opt/homebrew/opt/rustup/bin and /opt/homebrew/bin to PATH, and runs cargo run -p lumen-app -- "$@". Package script builds local macOS `.app` under .local/dist/Lumen Presenter.app from debug build by default (release opt-in), writes Info.plist with unique org.lumen.presenter bundle ID and executable lumen, copies the Rust binary (no proprietary resources), and ad-hoc codesigns when available. Runtime media tools are external prerequisites and launcher must discover /opt/homebrew/bin even when launched from Finder. Keep existing user environment intact. Parent handles README.

## Verification/report

Run cargo fmt, cargo test -p lumen-app, cargo build -p lumen-app and cargo clippy -p lumen-app --all-targets -- -D warnings. First native dependency compile may take minutes; yield commands and provide concise progress messages to parent rather than huge logs. Save report to .local/reports/app.md including delivered features, unimplemented parts, verification commands and any actual launch result. Parent performs visual inspection and full review. No commits or edits outside owned files.
