# B1: operator console shell + Wave A integration

Own `crates/lumen-app/**`; small listed additions to `crates/lumen-core/**` are allowed (below). Do not touch other crates. Read first, in this order: `AGENTS.md`, `PRODUCT.md`, `DESIGN.md` (binding), `~/.claude/skills/ponytail/SKILL.md` (follow it: smallest complete change, reuse existing helpers), `~/.claude/skills/impeccable/SKILL.md` + `reference/mode-operate.md` + `reference/craft-floor.md`, `~/.claude/skills/taste-skill/SKILL.md` sections 0, 1, 4.1–4.7, 9 (apply to egui; ignore web/GSAP parts), `docs/research/propresenter-feature-surface.md`, then the app source. Write tests first for every pure function you add (layout math, shortcut mapping, recovery decisions, transition progress).

Slide text keeps rendering with egui text for now — a separate task (B4, `lumen-text`) will replace text rasterization later, so keep text drawing behind ONE function in `render.rs` that takes the slide's spans/rect/style and paints them.

## Core additions (lumen-core, with tests)

- `CueAction`: `ClearSlide` (= today's ClearText(true) semantics; keep ClearText for compatibility), `ClearMedia`, `ClearAudio`, `ClearMessages`. `OutputState` gains `media_cleared: bool` and `audio_clear_serial: u64` as needed; ClearAll does all.
- `Transition`: keep `Cut` and `Fade { millis }` (= dissolve); add `FadeThroughBlack { millis }` and `Push { millis, direction: PushDirection { Left, Right, Up, Down } }`. Validate millis 0..=10000.
- `TimerDef.warn_at_secs: Option<f64>` (serde default) — display turns `warn` colour at/below it, `live` colour at/after zero.

## Shell (per DESIGN.md)

- `style.rs` holds every colour/size token from DESIGN.md; no colour literals elsewhere. Replace Geist with Poppins for the UI: download Poppins Regular/Medium/SemiBold TTF + OFL.txt from google/fonts `ofl/poppins` into `crates/lumen-app/assets/fonts/` (keep license, record SHA-256 in `assets/fonts/README.md`), remove Geist from the app, update the bundle script if it copies font licences.
- Header: project name, Prepare/Present segmented switch (default Present), output status pills (Audience, Stage, Remote: off/on/error), save state ("Saved" / "Unsaved changes").
- Left rail: Search field (filters library + playlist by name and slide text for now; books join later), Running order / Library tabs.
- Centre: large preview of the selected cue, cue grid with the saffron on-deck rail for the cue Next will send and red LIVE pill/outline on the live cue. Media-background cues show real thumbnails via `lumen_media::ThumbnailCache` loaded on a worker thread (never on the UI thread); placeholder while loading.
- Right column: live output monitor (exact audience render, red LIVE caption), transport (Previous · Go live/Next 40 pt saffron primary · Clear menu with F-key hints · Blackout), tabs Timers · Messages · Props · Audio · Stage. Prepare mode swaps tabs for the slide inspector.

## Tools

- **Timers:** list project timers with live value (tabular numerals, warn/live colours), Start/Pause/Reset, add/rename/delete, kind (Countdown duration · Countdown to time HH:MM · Elapsed), overrun toggle, warn threshold. Pass real local seconds-of-day (add `chrono` with `clock` feature only, or `time` with local offset — pick the one that works reliably on macOS and say why).
- **Messages:** template list, edit text with "Insert timer" helper for `{timer:<id>}`, Show/Clear; shown as a lower-third banner on audience output (theme-consistent, readable at a distance).
- **Props:** list, add/delete, edit name and objects (Text/Shape/Image with numeric rect fields), Trigger/Clear; rendered above slide and below message.
- **Audio:** slide audio cue (path, volume) plays on trigger via `lumen_media::AudioPlayer` (create lazily; device errors shown inline, never panic); audio bin = list of files with play/stop and volume; output device picker; ClearAudio stops with 300 ms fade.
- **Stage tab:** stage output options: current text, next text, clock, chosen timer, stage message, notes — checkboxes persisted in app settings (not the project).
- **Slide inspector (Prepare):** label, text (markup hint line: `**bold**  *italic*  __underline__  [#F2A33A]colour[/]`), text box rect (x/y/w/h %), style overrides (size, colour, outline, shadow), background, transition (kind + duration), audio cue, objects list (add/remove/edit), cue actions editor (add/remove/reorder any CueAction incl. new ones). Macros get the same actions editor.

## Rendering (audience/stage/previews)

`render.rs`: background → slide text (markup spans via egui LayoutJob for now) in `text_rect` with style overrides → slide objects → active props → message banner. Transitions: keep the previous live frame state and blend by elapsed/millis for Fade, FadeThroughBlack (out to black then in), Push (offset both layers); Cut instant. Pure function `transition_frame(kind, t) -> (prev_alpha, next_alpha, prev_offset, next_offset)` with tests. Repaint only while a transition/timer/video needs it.

## Editing safety

- Undo/redo with `lumen_core::history::History<Project>`: record before each discrete edit; coalesce continuous text editing into one entry per focus session. Cmd+Z / Cmd+Shift+Z only when no text field has focus (egui text fields keep their own undo).
- Autosave/recovery: every 30 s while dirty, write `~/Library/Application Support/Lumen Presenter/Recovery/<file-stem or "untitled">-<hash of project path>.json` via `save_project` (atomic). On launch / open, if a recovery file is newer than the project file, show one non-blocking banner "Recovered unsaved changes from <time> — Restore · Discard". Delete the recovery file after a successful save or a clean quit with nothing unsaved.
- Keyboard map exactly as DESIGN.md (F1–F6, B, arrows/space/page keys, Return, Cmd+F); never while a text field has focus. Pure mapping function with tests.

## Interchange + remote

- File menu/buttons: Import… (txt, xml/OpenLyrics, pro) through `lumen_interchange::import_file` on a worker thread (reuse `file.rs` pattern); success adds the document to library + playlist and shows warnings in an inline notice; Export OpenLyrics… for the selected document.
- Settings sheet (a side panel, not a modal): Remote control on/off, bind address (default 127.0.0.1:47800), token (generate 24 random URL-safe chars with `uuid` v4 bytes if no RNG crate; show/copy/regenerate), OSC port on/off (default 127.0.0.1:47801), MIDI input port picker + note→macro mappings. Start/stop `lumen_control` servers; map `ControlCommand` to the same functions the UI buttons call (one code path); publish `StatusSnapshot` when it changes. Store settings in `~/Library/Application Support/Lumen Presenter/settings.json` (atomic write, validated, defaults on parse error with a notice).

## Acceptance

- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo run -p lumen-app -- --smoke` (or the existing smoke mode) still exits cleanly.
- Native check: launch the app; capture screenshots with `screencapture -l <windowid>` or `screencapture -x` of the operator window in Present and Prepare modes, plus audience output during a Fade transition and with a prop + message showing; save PNGs to `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/b1-shots/`. If screen capture is denied, say so; do not claim visual verification.
- Leave changes uncommitted (parent commits). Report `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/b1-console.md`: what changed per area, tests added, new deps + licences, screenshots list, known gaps. Update it as you go so a quota stop loses nothing. Final reply: 10 lines max.
