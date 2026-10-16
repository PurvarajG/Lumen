# A2: lumen-core schema v2

Own `crates/lumen-core/**` only. Do not edit other crates; the app is updated by a later task, but `cargo build --workspace` must still pass. Where a core change breaks lumen-app compilation (new struct fields, the replaced `Timer` type), make the minimal mechanical fix in lumen-app — no new UI, no behaviour beyond keeping today's countdown working — and list each app edit in your report. That is the only allowed app edit. Read first: `AGENTS.md`, `~/.claude/skills/ponytail/SKILL.md` (follow it), `crates/lumen-core/src/*`, `crates/lumen-core/tests/behavior.rs`. Write tests first. No new dependencies except none — std + existing serde/uuid/thiserror only.

## Schema (SCHEMA_VERSION = 2)

`load_project` accepts version 1 and 2; v1 is migrated in memory (new fields get defaults, `schema_version` becomes 2). Save always writes 2. Versions >2 or 0 still error. All new fields `#[serde(default)]` so v1 JSON deserializes.

New types (all `Clone, Debug, Serialize, Deserialize, PartialEq`):

- `Rect { x: f32, y: f32, w: f32, h: f32 }` normalized 0..=1 canvas fractions. `Default` = `{0.05, 0.05, 0.9, 0.9}`. Valid when finite, x,y >= 0, w,h > 0, x+w <= 1.0001, y+h <= 1.0001.
- `TextStyle { font_size: Option<f32>, color: Option<[u8;4]>, outline: Option<(f32,[u8;4])>, shadow: bool }` Default all None/false. Option font_size validated like theme (8..=400).
- `SlideObject` enum, each variant has `id: Uuid, rect: Rect`:
  - `Text { id, rect, text: String, style: TextStyle, align: TextAlign }`
  - `Image { id, rect, path: PathBuf }`
  - `Shape { id, rect, color: [u8;4], corner_radius: f32 }` (radius finite, 0..=0.5)
  - add `fn id(&self) -> Uuid` and `fn rect(&self) -> Rect`.
- `TimerKind { Countdown { duration_secs: f64 }, CountdownTo { hour: u8, minute: u8, second: u8 }, Elapsed }` (validate duration finite >= 0 and <= 360000; hour<24, minute<60, second<60).
- `TimerDef { id, name: String, kind: TimerKind, allow_overrun: bool }`
- `Prop { id, name, objects: Vec<SlideObject> }`
- `MessageTemplate { id, name, text: String }` — text may contain `{timer:<uuid>}` tokens.
- `AudioCue { path: PathBuf, volume: f32 }` volume finite 0..=1.

Changes:

- `Slide` adds `text_rect: Rect`, `text_style: TextStyle`, `objects: Vec<SlideObject>`, `audio: Option<AudioCue>`. `slide.text` remains the primary text and stays plain-compatible (see markup below).
- `Project` adds `timers: Vec<TimerDef>`, `props: Vec<Prop>`, `messages: Vec<MessageTemplate>`. `Project::new`/blank constructor (whatever exists) seeds one `Countdown` 300 s timer named "Countdown".
- `CueAction` adds `TriggerProp { id }`, `ClearProps`, `ShowMessage { id }`, `ClearMessage`, `StopAudio`. Existing Start/Pause/ResetTimer ids now refer to `project.timers`.
- Validation (`Project::validate` or existing equivalent) also rejects: duplicate ids across slides/objects/timers/props/messages (extend the existing duplicate check), invalid rects/styles/volumes/timer kinds, more than 32 objects per slide or prop, cue/macro actions referencing missing timer/prop/message ids. Add `CoreError` variants with clear messages.

## Markup (new module `markup.rs`)

Primary text and Text objects use a tiny inline markup: `**bold**`, `*italic*`, `__underline__`, `[#RRGGBB]coloured[/]`, backslash escapes `\*`, `\_`, `\[`, `\\`. Unclosed markers render literally (never error). Public:

- `Span { text: String, bold: bool, italic: bool, underline: bool, color: Option<[u8;4]> }`
- `parse_markup(&str) -> Vec<Span>` (newlines preserved inside span text; adjacent same-style spans merged)
- `strip_markup(&str) -> String` (plain text, escapes resolved)
- `escape_markup(&str) -> String` so `strip_markup(&escape_markup(s)) == s` for any s (property-style test over a few hundred generated strings without new deps).
- `document_from_text` must escape imported text so literal `*`/`_`/`[` survive.

## Runtime

- Timer runtime state lives in `Runtime`, keyed by `TimerDef.id`: `TimerState { running: bool, value_secs: f64 }` (remaining for Countdown, elapsed for Elapsed; CountdownTo derives from wall clock). Replace the old self-contained `Timer`; keep a `display` helper: `Runtime::timer_display(&self, def: &TimerDef, now_secs_of_day: u32) -> String` giving `MM:SS` / `HH:MM:SS`, prefixed `-` when overrun.
- `Runtime::tick(&mut self, project: &Project, elapsed: Duration)` advances running timers. Countdown without overrun stops at 0; with overrun continues negative. CountdownTo remaining = target − now (seconds of local day, caller-supplied); after target: negative if allow_overrun else 0. Elapsed counts up. Start/pause/reset semantics as before; reset Countdown→duration, Elapsed→0.
- `Runtime::sync_timers(&mut self, project)` adds state for new defs and drops removed ones (call it from tick and the timer actions so edits never panic).
- `OutputState` adds `props: Vec<Uuid>` (active, ordered, no duplicates), `message: Option<Uuid>` (active audience announcement), `stop_audio_serial: u64` (incremented by StopAudio so the app can react). ClearAll also clears props and message.
- `Runtime::render_message(&self, project, id, now_secs_of_day) -> Option<String>` replaces `{timer:<uuid>}` tokens with that timer's display; unknown tokens left literal.
- `LiveSlide` snapshot carries the new slide fields (it clones the slide, so edits stay isolated — keep the existing isolation tests green and add one for objects/text_rect).

## History (new module `history.rs`)

`History<T: Clone>` with capacity (default 200): `record(&mut self, before: T)` pushes onto undo and clears redo; `undo(&mut self, current: T) -> Option<T>`; `redo(&mut self, current: T) -> Option<T>`; `can_undo/can_redo`; `clear`. Oldest entries dropped at capacity. Pure, deterministic tests.

## Acceptance

- Tests: v1 fixture JSON (write one inline in the test) loads and migrates; v2 round trip; each new validation rejection; markup parse/strip/escape incl. unclosed markers, nested bold+italic, colour, escapes, Unicode; all timer kinds incl. overrun and CountdownTo across the target; props/message actions; ClearAll; history capacity/undo/redo/branch clear; edit/live isolation for new fields.
- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` all pass.
- Commit on your branch with a conventional message. Save a report to `.local/reports/a2-core-v2.md`: public API summary, tests added, any app edits, limitations. Final reply: 10 lines max.
