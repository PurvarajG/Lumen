# Core implementation contract

Own only crates/lumen-core/**. Parent owns workspace Cargo.toml, app/media crates and git. This is the portable engine for Lumen Presenter, a standalone native Rust live-presentation app. Read AGENTS.md and the design spec. Implement this task autonomously; resolve routine details yourself. Cargo is /opt/homebrew/opt/rustup/bin/cargo (add that directory to PATH in commands). Rust 1.99.1 is installed. Dependencies can use workspace serde, serde_json, uuid, thiserror, tempfile. Re-export all public types/functions from lib.rs.

Write meaningful tests first, run to observe expected failure, then implement. No UI dependencies. Use small model modules rather than one enormous file. APIs below are the interface other workers will consume; preserve exact names/fields. You may add helpers and derives. All model types Clone/Debug/Serialize/Deserialize and PartialEq where applicable.

## Persisted types

`pub const SCHEMA_VERSION: u32 = 1;`

`Project { pub schema_version: u32, pub name: String, pub documents: Vec<Document>, pub playlist: Vec<PlaylistItem>, pub themes: Vec<Theme>, pub macros: Vec<Macro> }`

`Document { pub id: Uuid, pub name: String, pub slides: Vec<Slide> }`

`PlaylistItem { pub id: Uuid, pub document_id: Uuid }`

`Slide { pub id: Uuid, pub label: String, pub text: String, pub notes: String, pub theme_id: Uuid, pub background: Background, pub transition: Transition, pub actions: Vec<CueAction> }`

`Background` enum: `Solid([u8; 4])`, `Image { path: PathBuf }`, `Video { path: PathBuf, looped: bool, muted: bool }`. Use serde externally tagged defaults (do not invent custom tagging).

`Transition` enum: `Cut`, `Fade { millis: u32 }`.

`Theme { pub id: Uuid, pub name: String, pub font_size: f32, pub text_color: [u8;4], pub background_color: [u8;4], pub align: TextAlign, pub vertical_align: VerticalAlign }`

`TextAlign`: `Left`, `Center`, `Right`. `VerticalAlign`: `Top`, `Middle`, `Bottom`. Default theme: original dark background, near-white text, centered, 64-point canvas font size. Font size is in 1920×1080 canvas units; valid 8..=400 and finite.

`Macro { pub id: Uuid, pub name: String, pub actions: Vec<CueAction> }`

`CueAction` enum: `Blackout(bool)`, `ClearText(bool)`, `ClearAll`, `StageMessage(String)`, `StartTimer { id: Uuid }`, `PauseTimer { id: Uuid }`, `ResetTimer { id: Uuid }`. No arbitrary commands or filesystem actions. At most 64 actions per slide/macro (bounded execution). Timers are runtime-only and references to absent timers return a descriptive error.

Methods:
- `Project::demo() -> Self` original sample show: 3 documents (welcome, song, announcements), 8-12 total slides with stage notes, 3 themes, playlist of those docs, and useful blackout/restore/stage-message macros. No copyrighted lyrics; original generic text.
- `Project::validate(&self) -> Result<(), CoreError>` validates schema, theme count >0, unique IDs within and across documents/slides/themes/playlist items/macros, dangling theme/document refs, finite font sizes and action bound. Empty project/documents/playlist are valid if at least one theme exists. Keep errors human-readable.
- `Project::document(&self,id:Uuid)->Option<&Document>` and `theme(&self,id:Uuid)->Option<&Theme>`.
- `Theme::default() -> Self` new UUID.
- `Slide::new(theme_id:Uuid) -> Self` new UUID, label "New slide", empty text/notes, Solid dark bg, Cut, no actions.
- `Document::new(name:impl Into<String>)->Self` new UUID, empty slides.

## Live state and runtime

`LiveSlide { pub document_id: Uuid, pub document_name: String, pub slide: Slide, pub theme: Theme }` is cloned when triggered. No reference lifetimes back into editable Project.

`OutputState { pub live: Option<LiveSlide>, pub blackout: bool, pub clear_text: bool, pub stage_message: String }` implements Default. Blackout masks output without discarding live content. ClearText masks text only. ClearAll discards live and resets clear_text, preserving blackout and stage message.

`Timer { pub id: Uuid, pub name: String, pub duration_secs: f64, pub remaining_secs: f64, pub running: bool }`
- `Timer::new(name:impl Into<String>,duration_secs:f64)->Self`, sanitizes invalid/negative to zero.
- `start(&mut self)`, `pause(&mut self)`, `reset(&mut self)` (restore duration; stopped), `tick(&mut self, elapsed:Duration)` saturates at zero and stops expired clocks.
- `display(&self)->String` format ceil remaining seconds as HH:MM:SS for >=1h, MM:SS otherwise.

`Runtime { pub output: OutputState, pub timers: Vec<Timer> }` implements Default (one 5-minute timer called "Countdown").
- `trigger(&mut self,project:&Project,document_id:Uuid,slide_id:Uuid)->Result<(),CoreError>` resolve slide/theme, clone snapshot, clear clear_text, leave blackout unchanged, apply slide actions in order. Validate the entire action batch before mutation, so missing timer errors leave prior output/runtime intact.
- `next(&mut self,project:&Project)->Result<bool,CoreError>` / `previous(...)` walk nonempty slides in playlist order. With no live slide next triggers first, previous triggers last. At boundaries return false and keep live unchanged. With no matching live in current playlist use those same start/end behaviors. Skip empty documents. Duplicate playlist doc references are allowed; navigation uses the first matching occurrence for a given current slide (document this choice).
- `peek_next(&self,project:&Project)->Option<LiveSlide>` without mutation.
- `apply_actions(&mut self,actions:&[CueAction])->Result<(),CoreError>` bounded, prevalidate timer refs to guarantee all-or-nothing for invalid batches, apply in order.
- `run_macro(&mut self,project:&Project,id:Uuid)->Result<(),CoreError>` resolve and apply.
- `tick(&mut self,elapsed:Duration)` updates clocks.

## Storage/import

`CoreError` implements std::error::Error and Display (thiserror is fine). Error variants may be chosen freely.

- `pub fn save_project(path:&Path,project:&Project)->Result<(),CoreError>` validate, serialize, enforce 16MiB limit, write unique tempfile in same directory, flush/sync, atomic persist to path. Do not truncate the prior file if any earlier error happens. Support replacement on macOS. Parent dirs must exist (return error otherwise).
- `pub fn load_project(path:&Path)->Result<Project,CoreError>` enforce 16MiB pre-read and bound actual read if file changes; serde_json then validation. Descriptive IO/schema errors.
- `pub fn document_from_text(name:&str,text:&str,theme_id:Uuid)->Document` normalize CRLF/CR, split paragraphs at blank/whitespace-only lines (one paragraph per slide), retain nonblank line breaks, trim outer blank lines and trailing whitespace. Ignore empty paragraphs. Labels "Slide 1", etc. An empty text import yields empty doc.

## Required behavioral coverage

Snapshot remains unchanged when original text/theme changes. Navigation crosses docs, skips empty docs, handles first/last/empty playlist. Timer pause/resume/reset/expiry and fractional-second display. Ordered macro/slide actions and invalid batch atomicity. Duplicate/dangling IDs, schema version/font limits/action caps. Roundtrip save/load; invalid save preserves original bytes; malformed and oversized load fail. CRLF and whitespace text import. Meaningful tests, no field getter mirror tests.

Run cargo fmt (crate files only if other work exists), cargo test -p lumen-core, cargo clippy -p lumen-core --all-targets -- -D warnings. Save report including initial failure, final commands/results and any deviations to .local/reports/core.md; return only status, tests count/summary and concerns. Do not commit or modify workspace manifest.
