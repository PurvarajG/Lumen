# Lumen Presenter design

## Authority and scope

The user requested a full standalone Rust ProPresenter alternative, using REA to understand the installed reference application. They explicitly authorized autonomous decisions without further routine input. This design records the selected decisions rather than imposing another approval gate. Full parity remains the program objective; each milestone must distinguish implemented, tested, inferred and unimplemented capabilities.

## Architecture decision

Choose a native Rust application using eframe/egui, with a UI-independent domain crate and a separate media adapter crate. This keeps the application in Rust, makes cue semantics testable without a GPU and allows independent audience and stage windows. A webview shell would add a second application language and web rendering behavior; platform-specific AppKit UI would slow portability and require more bridging. Neither is selected.

Use the installed FFmpeg/ffplay executables for the first media adapter, managed as owned child processes with bounded frame storage and cleanup. FFmpeg is an explicit development/runtime prerequisite, not a hidden promise of a self-contained media distribution. Later packaged media and broadcast SDK adapters must document their licenses and supported platforms.

## Project model and runtime

Persist a versioned JSON project containing documents, slides, a playlist of document references, reusable themes and macros. UUIDs are stable identifiers. Slides contain text, notes, a label, a theme reference, background media, transition settings and cue actions. Themes hold font size, text/background colors and horizontal/vertical alignment. Media paths are editable references; missing media is a visible recoverable error.

Project validation rejects unsupported schema versions, invalid dimensions, duplicate identifiers, dangling references and non-finite/invalid font sizes. Storage uses a temporary file in the destination directory and an atomic rename, so errors do not truncate an existing project. Load applies a size limit before allocation. Text import separates slides at blank lines and normalizes line endings.

The runtime owns the selected cue, a cloned live slide/theme, blackout, text-clear state, a stage message and countdown timers. Triggering makes a snapshot; subsequent editor changes never mutate it. Next/previous walk playlist order across documents with defined boundaries. Blackout is a reversible output mask, separate from clearing text or all live content. Timers advance from elapsed monotonic durations; paused clocks remain fixed and remaining time saturates at zero. Macros use the same bounded cue action algebra as individual slides.

## Operator interface

A dark, high contrast workbench uses three functional regions: playlist/library on the left, a cue grid in the center, and live preview plus properties on the right. Editing, live preview and output have explicit labels and state; keyboard navigation must respect active text fields. Operator actions include create/open/save, document/slide CRUD, cue reordering, duplicate, text import, theme controls, image/video selection, live trigger, next/previous, blackout, clear text, clear all, timers and stage message.

Audience and stage outputs are separate native viewports using shared immutable render state. Audience output letterboxes to the configured 16:9 canvas and renders media plus text. Stage output adds current/next text, timers and operator message. Windowed preview and fullscreen are explicit settings. Closing an output window must not close the operator window. UI callbacks must not block on file decoding or child processes.

## Milestones and parity criteria

1. **Foundation and working show:** versioned projects; validated storage; playlists/documents/slides; themes; live snapshots; cue navigation; timers/macros; text import; native operator UI; audience/stage windows; image/video backgrounds; owned media lifecycle; keyboard controls; recoverable error messages; launchable macOS app.
2. **Editing and interchange:** layered objects, rich text, text overflow strategies, transitions, undo/redo, media organization, OpenLyrics import/export and evidence-based ProPresenter document import. Do not call protobuf import supported from a library name alone.
3. **Production outputs:** monitor assignment, independent layouts, synchronization/performance measurements, audio routing, capture, recording and streaming; isolate NDI/SDI/Syphon behind capability adapters.
4. **Automation and services:** MIDI/OSC/DMX/timecode where supported, remote API/control, scripture content from licensed/user-provided sources, schedules and external integrations.
5. **Reliability and packaging:** media packaging, crash recovery, benchmarks under long shows, hardware tests, release signing/notarization and Windows/Linux validation.

## Verification

Core tests must prove edit/live isolation, playlist boundary navigation, timer pause/resume/reset/expiry, macro ordering, schema/reference rejection and save/load integrity. Media tests must use a generated local test pattern rather than proprietary footage. Native visual checks must show the operator, audience and stage windows; automated test success alone does not establish live-production readiness. Record exact commands, actual outcomes and remaining limitations in docs/STATUS.md.

## Research strategy

Start with REA bundle inventory, plist, Mach-O metadata, resource/document declarations and bounded string/symbol searches. Use Ghidra for selected questions and small components rather than repeatedly decompiling the entire application. Retain complete evidence on disk, return selected facts to the orchestrator, and mark inferences explicitly. Do not modify or patch the reference application.
