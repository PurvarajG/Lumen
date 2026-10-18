# Wave G1: fixes from the first live service (2026-10-17)

Plan for one executor (GPT-5.5 high or equivalent). Orchestrator: Claude Opus. Read this file top to bottom before you start.

## Rules for the executor

- Read and follow `~/.claude/skills/ponytail/SKILL.md` for all code. Read `~/.claude/skills/impeccable/SKILL.md` and `~/.claude/skills/taste-skill/SKILL.md` before the UI phases (G1.4–G1.6), adapted to egui. Follow `AGENTS.md` and `DESIGN.md`.
- Do the phases in order, one commit per phase (`fix(media): …`, `feat(app): …`). Phases G1.1 and G1.2 can run in parallel worktrees: G1.1 touches only `crates/lumen-media` and `crates/lumen-app/src/media.rs`; G1.2 touches neither. Every other phase edits `app.rs` / `panels.rs`, so run those one after another.
- Use a per-task `CARGO_TARGET_DIR` under `.local/targets/` and delete it after the merge (the laptop has about 20 GB free).
- No new crates. Everything below uses what is already in the workspace (egui 0.33.3, rodio 0.21, ffmpeg/ffprobe CLI).
- Before each commit run `cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings`.
- If a step below turns out to be wrong for the code you find, stop and write down why in `docs/STATUS.md`. Do not work around it silently.

## What happened (operator report, 2026-10-17)

1. Video on the audience screen was jittery and did not look natural.
2. Every change was a hard cut. No fades, so the audience could see someone switching.
3. Previews were often blank after dragging files in, so the operator had to guess.
4. After going live, pressing Next did not move on to the next item. The operator had to click the next item every time and did not feel in control of the order.
5. The running-order panel is tiny, and the panels cannot be moved.
6. Audio playback controls are underdeveloped.

## Evidence (reproduced on rc.3, release build)

Test show and captures: `.local/qa/g1/` (`show.lumen.json`, `shots/01…08`, `measure-cadence.sh`). Launch:

```bash
cd .local/qa/g1 && LUMEN_DATA_DIR=$PWD/data ../../../target/release/lumen --project show.lumen.json
```

HTTP control is on in that data dir: `curl -H 'Authorization: Bearer qa-token-g1-0123456789' -d '{"cmd":"Next"}' http://127.0.0.1:47800/command` (also `{"cmd":"Trigger","index":N}`, `GET /status`).

| # | Observed | Root cause in the code |
|---|---|---|
| 1 | A 30 fps clip (`motion30.mp4`) on the live monitor updated only **69 times in 4 s (about 17 fps)**. Gaps between frames ranged from 17 to 83 ms. | (a) While video plays, the app asks for a repaint only every **80 ms** (`app.rs` end of `update`, and the audience/stage viewport closures). The decoder thread never wakes the UI, so a new frame is shown only when a repaint happens to come. (b) `ffmpeg -re` paces itself loosely, and a single "latest frame" slot drops or repeats frames unevenly. (c) Every clip is resampled to **30 fps at 1280×720**, which causes judder on 25/50/60 fps footage and looks soft on a 2560 px projector. (d) Each frame is cloned (`latest_frame()`) and premultiplied pixel by pixel on the UI thread (`from_rgba_unmultiplied`). |
| 2 | Every new slide defaults to `Transition::Cut` (`model.rs` `Slide::new`). The only way to change it is per slide, in the Prepare inspector. Clear actions always cut (`clear_layer` sets `self.transition = None`). When a video starts, the screen goes black until the decoder's first frame arrives, because `sync_media` calls `media.stop()` first. Fades also run at 12.5 fps because of the 80 ms repaint, and the motion is linear. | No show-wide transition. Fades don't wait for the new media. Repaint rate too low. |
| 3 | The big Preview, the running-order rows and the grid tiles show plain black while a thumbnail loads **and** after it fails. The grid text says "Media unavailable / loading" in both cases. | `ThumbnailState` has **one** worker thread, so dropping 20 videos queues 20 × (ffprobe + ffmpeg). A failure (including a 10 s timeout while busy) is cached forever and never retried. There is no visible loading or error state outside the grid. `thumbnails.images()` clones the whole map once **per tile** in `panels::slides`. |
| 4 | Confirmed in `shots/03-next1.png`: after Go live → Next, "Video" stays selected, the Preview still shows it, and the primary button says **"Go live"**, so pressing it re-sends the old cue. | The primary button says "Next" only while the live cue is in the *selected document* (`panels.rs` `live_in_document`). Dropped files become one document each, so every advance crosses a document boundary and flips the button back to "Go live <old selection>". `trigger`/`navigate` never move the selection. |
| 4b | **New, not reported but show-breaking:** with the same document twice in the running order (`Captions` in the test show), both rows show LIVE, and **Next loops back to the first copy forever** (measured live_index 8 → 1 → 2 → 3 → 4 … 8 → 1). | `Runtime::navigate` and `peek_next` find the current cue with `position()` on `(doc, slide)`, which always returns the first occurrence. |
| 5 | The left rail is capped at **310 pt** (`width_range(220.0..=310.0)`). Macros and "New document" take the bottom quarter, leaving about 7 rows for the running order. | Layout constants. |
| 6 | The Audio tab has Add, a "New file volume" slider, Stop, and a device picker. There is no now-playing display, no pause, and no elapsed/remaining time. Video sound has no controls at all: `MediaPlayer::set_paused` exists but the app never calls it. | Missing UI. `AudioPlayer` has no pause and no duration. |

## Pushback (ponytail ultra): what this plan does not do

- **Dockable or floating panels** ("windows aren't movable"): egui has no docking, and adding `egui_dock` means rewriting the shell. The real complaint is that the order list is too small. G1.5 makes the rail resizable up to 520 pt and gives the list the full height. The audience and stage outputs are already separate movable windows. Revisit docking only if the operator still asks after G1.5.
- **More transition types** (wipes, zooms, 3D): Fade, Fade through black and Push already exist. The problem was that nothing used them. G1.3 makes a fade the default.
- **A frame-accurate display-link video engine** (AVFoundation/CVDisplayLink): too big for now. G1.1 gets even pacing with the existing ffmpeg pipe. If measured cadence still fails on the external display after G1.1, record that, and consider the native engine as its own wave.
- **Seeking, a live video volume slider, audio looping, a persisted audio bin**: none were asked for. G1.6 adds play/pause/stop, time and volume only.
- **A higher-resolution Preview** (the big Preview is a stretched 320×180 thumbnail and looks soft): not the reported problem. Note it in STATUS as a known gap.

---

## G1.1 Smooth video playback (P0)

Files: `crates/lumen-media/src/probe.rs`, `crates/lumen-media/src/player.rs`, `crates/lumen-media/tests/media.rs`, `crates/lumen-app/src/media.rs`.

1. **Probe the frame rate.** In `probe_with_program`, add `avg_frame_rate` to `-show_entries` (`stream=codec_type,width,height,avg_frame_rate`). Parse `"num/den"` from the video stream into `MediaInfo { fps: Option<f64> }`. Use `None` for `0/0`, missing values, or values outside 1–240. Fix every `MediaInfo { .. }` constructor and test fixture.
2. **Pace frames in our code, not with `-re`.**
   - Remove `-re` from the ffmpeg args.
   - Compute the output rate once: `fps = info.fps.map(|f| f.round()).unwrap_or(30.0).clamp(1.0, options.fps as f64)`. `options.fps` now means a *maximum*, and the app passes 60. Use the computed value in the `fps=` filter.
   - Split `decode` into a reader and a pacer. The reader thread reads whole frames into `std::sync::mpsc::sync_channel::<Vec<u8>>(3)`. The pacer thread receives each frame, sleeps until `start + n / fps`, then stores it in `latest`.
   - Drift and pause rule, one line: if `now > due + 250 ms`, set `start = now - n / fps`. This one rule recovers after a pause (SIGSTOP stalls the reader), a loop restart, or a slow decode, without bursting frames. Comment it: `// shortcut: rebase the clock when >250 ms late (pause, stall); upgrade to PTS-driven timing if A/V drift is reported.`
   - Keep every existing error, EOF, mid-frame and stop/reap behavior. `stop()` must still join both threads. Dropping the channel's receiver unblocks the reader.
3. **Cap the decode size, don't fix it at 720p.** In `media.rs` `load_video`, pass `width: 1920, height: 1080, fps: 60`. In `player.rs`, decode at `min(source, requested)` while keeping the aspect ratio (the probe already gives the source size). That means `scale=w:h` to the fitted size, not pad-to-box. `paint_scene` already letterboxes. A 720p source therefore stays 720p, and 4K comes down to 1080p.
4. **Stop copying frames.**
   - Replace `latest_frame()` (clone) with `take_frame(&self) -> Option<Frame>` (`lock(&self.latest).take()`). Drop the `sequence` comparison in `MediaState::poll`, since `Some` now means a new frame. Update the `lumen-media` tests that call `latest_frame`.
   - ffmpeg emits opaque RGBA, so use `ColorImage::from_rgba_premultiplied` in `MediaState::poll` (no per-pixel premultiply).
5. **Wake the UI per frame, not every 80 ms.**
   - In `app.rs` `update`, split the final repaint condition: while `self.media.player` is playing (not finished, not paused) or `self.transition.is_some()`, call `ctx.request_repaint()` (vsync-paced). Keep the 80 ms fallback for timers, deck, stage, message and remote.
   - Do the same in the audience viewport closure (`state.media_animating || state.transition.is_some()` → `ctx.request_repaint()`) and the stage closure (`media_animating` → `request_repaint()`).
   - Only if the measurement below fails, add `PlaybackOptions.on_frame: Option<Arc<dyn Fn() + Send + Sync>>` (write a manual `Debug`). Call it from the pacer after each store, and pass a closure that calls `request_repaint()` on the root context and `request_repaint_of` the audience viewport.
6. **Tests (deterministic, no hardware):**
   - Probe: parse `"30000/1001"` → 29.97, `"0/0"` → None, `"25/1"` → 25.
   - Pacer: give the pacing function a fake clock (`fn due(start, n, fps) -> Instant` plus the rebase rule as a pure function) and assert there is no burst after a 2 s stall.
   - Player integration (the existing ffmpeg-gated tests): `take_frame` returns `Some` and then `None` until the next frame arrives.

**Acceptance (measured, not eyeballed).** Run the test show, trigger `clip.mp4` (cue index 13, a 1920×1080 30 fps looped `motion30.mp4`), and measure the live monitor:

```bash
.local/qa/g1/measure-cadence.sh <window-id> 680:380:2360:380 4
```

Get the window id with `swift .local/qa/g1/wins.swift <pid>`. The crop is for a 1500×949 pt window at {6,33} on the Retina laptop; check the crop against one extracted frame first.

- Baseline was 69 unique frames, mean 58.6 ms.
- Target: **≥ 112 unique frames in 4 s (≥ 28 fps), mean 33 ± 3 ms, worst ≤ 50 ms.**
- Then open Audience on the **external display** (the user's L27q-4A, 2560×1440, 60 Hz) and measure that window the same way. Laptop ProMotion (120 Hz) plus a 60 Hz external, driven from one main thread, is the case most likely to throttle. If it fails only there, try `on_frame` (step 5), then record the numbers in STATUS.
- Check CPU with `top -l 2 -pid <pid>`: note it, and keep it under one core at 1080p30.

## G1.2 Duplicate running-order items (P0, show-breaking)

Files: `crates/lumen-core/src/runtime.rs`, `crates/lumen-core/tests/behavior.rs`, `crates/lumen-app/src/panels.rs` (row LIVE pill only).

1. Add `#[serde(default)] pub cursor: Option<usize>` to `Runtime`: the index into `playlist_cues(project)` of the live cue.
2. Add a private `fn current_index(&self, cues) -> Option<usize>`. It returns `cursor` if `cues.get(cursor)` equals the live `(doc, slide)`. Otherwise it falls back to the first `position()`, as today. `navigate` and `peek_next` both use it.
3. `navigate` sets `cursor = Some(target)` after a successful trigger.
4. `trigger(doc, slide)` (a click or the HTTP index) sets `cursor` to the first matching index **at or after** the current cursor, wrapping to the first match. This needs a cues lookup inside `trigger`, which is fine. Add `pub fn trigger_index(&mut self, project, index)` for `ControlCommand::Trigger { index }` so remote triggers hit the exact occurrence. In `app.rs` `control()`, route the HTTP `Trigger` through it, then call the same after-trigger path the existing code uses.
5. Add `pub fn live_item_index(&self, project) -> Option<usize>`, the playlist row of the cursor. In `panels.rs` `document_rows`, light LIVE (and the up-next rail, via the next cue's row) by playlist **row index**, not by document id.
6. Tests in `behavior.rs`:
   - A playlist `[A, B, A, C]` with one slide each: Next from start walks A, B, A, C, then returns `Ok(false)`, and never loops.
   - `peek_next` agrees with Next at every step.
   - Triggering A while the cursor sits on the second A keeps the second A.
   - A runtime JSON without `cursor` still deserializes.

## G1.3 Show transition and smooth cuts (P0)

Files: `crates/lumen-core/src/model.rs` (+ the validation in `validate`), `crates/lumen-core/tests/*`, `crates/lumen-app/src/app.rs`, `crates/lumen-app/src/render.rs`, `crates/lumen-app/src/inspector.rs`, `crates/lumen-app/src/panels.rs` (transport row).

1. **Show-wide default.** Add `#[serde(default = "default_show_transition")] pub transition: Transition` to `Project`, with the default `Fade { millis: 400 }`. Old files load with a fade, so no schema bump is needed. Validate it with the same 10 000 ms limit as slides.
   - Meaning: a slide's `Transition::Cut` now means **"use the show transition"**. Any other value overrides it.
   - In `app.rs`, add `fn effective_transition(&self, slide) -> Transition` and use it in `after_trigger`.
   - Comment: `// shortcut: per-slide Cut = show default, so one slide can't force a hard cut while the show fades; add Option<Transition> if operators ask.`
   - In `inspector.rs`, rename the slide combo's "Cut" entry to "Show default".
2. **Operator control in Present.** In the transport row in `panels.rs` (next to Clear / Blackout), add one `ComboBox` labelled `Transition`: Cut · Fade · Fade through black · Push left. Put the duration presets 0.25 s / 0.5 s / 1 s in the same popup. It edits `app.project.transition` and calls `app.mark()`.
3. **Clears fade too.** In `clear_layer`, stop setting `self.transition = None`. Capture `previous` and `previous_media` as `trigger` does, apply the action, then start an `OutputTransition` with the show transition. **Blackout stays instant** (a safety action). F1–F6 behave as before, just eased.
4. **Hold the old picture until the new media is ready.**
   - Add `waiting_for_media: bool` to `OutputTransition`.
   - In `after_trigger`, if the new live background is `Video` or `Image` and `self.media.pending` (or the video has no texture yet), create the transition even for `Cut` (with millis 0) and set `waiting_for_media = true`.
   - In `update`, after `self.media.poll(ctx)`: if waiting and (`self.media.image_texture.is_some() && !self.media.pending`, or 1.5 s have passed), set `started = Instant::now()` and `waiting_for_media = false`.
   - The expiry check in `update` and the `active_transition` filter in `render::paint_audience` must treat a waiting transition as active, with progress 0. This removes the black flash on video and photo cues, including with Cut.
5. **Ease the motion.** In `render::transition_frame`, apply smoothstep `t = t * t * (3.0 - 2.0 * t)` after the clamp. Update the existing numeric assertions in `render.rs` tests (0.25 → 0.15625).
6. **Repaint per frame during transitions.** This is covered by G1.1 step 5. If G1.1 is not merged yet, make the change here.
7. **Tests:**
   - Core: a project without `transition` loads as `Fade 400`; 10 001 ms is rejected.
   - App: `effective_transition` returns the show value for a slide with `Cut`, and the slide's own value otherwise.
   - Render: smoothstep values; a waiting transition paints the previous scene (progress 0).

**Acceptance:** with the test show and Audience open, trigger `photo.png` → `clip.mp4` → a caption → Clear slide (F2). Capture the four switches with `screencapture -V 3` and confirm with `measure-cadence.sh` that each fade shows about 12 or more distinct frames over 400 ms, with no all-black frame between photo and video. Then set Transition to Cut and confirm the switch happens in one frame with no black frame.

## G1.4 Next always means the next thing (P0)

Files: `crates/lumen-app/src/app.rs`, `crates/lumen-app/src/panels.rs`, `DESIGN.md` (Keyboard and Layout sections).

The model in one sentence: **the selected cue is the armed cue; going live sends it, and then the cue after it is armed automatically.** The operator only clicks to break the order.

1. Add `fn arm_next(&mut self)` in `app.rs`. In Present mode only (`self.present`), set `selected_doc` and `selected_slide` to `self.runtime.peek_next(&self.project)`. At the end of the order, select the live cue.
   - Call it at the end of a successful `trigger` and of `navigate` (both directions), and from `books::add_and_go_live` (after its trigger).
   - Do not call it in Prepare: moving the selection while someone edits would be worse.
   - Call `crate::books::close_preview(self)` first only if the preview is showing a book result the operator did not send. Read `books.rs` `has_preview` and follow its existing rules; do not invent new ones.
2. Primary button (`panels.rs` inspector transport):
   - Label: `Go live` when not live; `Next` when the armed cue == `peek_next`; `Go live: <label>` when the operator armed something else. Truncate the label to the button width.
   - Action: always `trigger(selected)`, except when a deck is playing (keep `navigate`).
   - Delete `transport_primary_label(live_in_document)` and its test, and replace the test with one for the new three-way label function.
3. Keyboard: Space / → / Page Down send the armed cue, the same action as the button. ← / Page Up still go to the cue before the live one (`navigate(false)`) and then arm the next. Return stays "go live with selection", which is now the same as Space. Update the Keyboard line in DESIGN.md.
4. The Preview heading in `panels::slides` reads **"Up next"** when the selection == `peek_next` and live is not empty; otherwise it reads "Preview". This tells the operator that what they see is what the button sends.
5. When the selected document changes because of arming, scroll the running-order row and the grid tile into view (`response.scroll_to_me(Some(egui::Align::Center))` once, only on the frame the arm happened; store a one-shot flag).
6. Tests: `LumenApp::new` needs an eframe `CreationContext`, so there is no app test constructor. Do not add one. Put the decision in two pure functions and test those:
   - `fn armed_after_send(runtime: &Runtime, project: &Project, present: bool) -> Option<(Uuid, Uuid)>`, which `arm_next` calls. Tests: with a playlist of three one-slide documents, after triggering the first it returns the second, after the second the third, and at the end the live cue. With `present = false` it returns `None` (no move).
   - `fn primary_label(armed, next, live, deck) -> String`: `Go live`, `Next`, `Go live: <label>`, and the deck case.

**Acceptance:** with the test show in Present, press Space eight times from the start. Every press advances exactly one cue across document boundaries, the button reads `Next` the whole time, and the Preview heading reads "Up next". Click `photo.png`: the button reads `Go live: photo.png`, Space sends it, and the order continues from there.

## G1.5 Bigger running order (P1)

Files: `crates/lumen-app/src/app.rs` (left `SidePanel`), `crates/lumen-app/src/panels.rs` (`library`, `document_rows`), `crates/lumen-app/src/style.rs` (`LEFT_WIDTH`), `DESIGN.md` (layout widths).

1. Left rail: `default_width(300.0)`, `width_range(240.0..=520.0)`. Right inspector: unchanged.
2. Running order list: replace `max_height(available - 210)` with the full remaining height, minus one collapsed header.
3. Macros move into `egui::CollapsingHeader::new("Macros")` at the bottom of the rail, `default_open(!app.present)`, so it is collapsed in Present and open in Prepare. Keep the macro buttons and their editing exactly as they are.
4. Hide "New document" in Present (an editing action; PRODUCT.md: impossible to confuse with editing). Keep it in Prepare.
5. Rows stay 56 pt. When the rail is wider than 380 pt, the thumbnail may grow to 64 pt tall. Optional; skip it if it complicates hit-testing.

**Acceptance:** at a 1500×949 window in Present, the running order shows at least 12 rows with Macros collapsed. Dragging the divider widens the rail to 520 pt. Prepare still shows Macros open and New document. Take before/after window captures to `.local/qa/g1/shots/`.

## G1.6 Previews that never show a blank (P1)

Files: `crates/lumen-app/src/thumbnails.rs`, `crates/lumen-app/src/render.rs`, `crates/lumen-app/src/panels.rs`, `crates/lumen-app/src/books.rs` only if it paints media previews.

1. **Three workers.** Wrap the request receiver in `Arc<Mutex<Receiver<Request>>>` and spawn `THUMBNAIL_WORKERS = 3` threads. Each one has its own `ThumbnailCache`; the UI-side `entries` map already dedupes. Shutdown and Drop keep working for all three.
2. **Retry failures.** Store `failed_at: Option<Instant>` on `ThumbnailEntry`. In `request()`, re-queue an entry whose failure is older than 10 s, unless the error is a missing file (`MediaError::InvalidPath`). Check that from the error text the worker sends, or send a `retry: bool` alongside the error.
3. **One placeholder for every preview surface.** Add `render::media_placeholder(painter, rect, file_name, state)`, where `state` is `Loading | Failed(reason) | Missing`:
   - Loading: dim panel, a small egui spinner, "Loading preview…", and the file name.
   - Failed: a `warn`-coloured Phosphor warning icon, "Can't preview <file>", and the reason as a tooltip.
   - Missing: the same with "File not found: <file>".
   - Text at `CAPTION` size, centred, truncated.
   - Call it wherever `thumbnail(...)` returns `None` for an `Image` or `Video` background: the grid tile (replacing the current "Media unavailable / loading" text), the running-order row thumbnail (a tiny rect, so icon only), and the big Preview in `panels::slides`.
   - Expose `ThumbnailState::state(path)` to choose the variant.
4. **Live video in the Preview.** When the selected cue's background equals the live background (`app.media.matches_background`), paint `app.media.image_texture` instead of the still thumbnail. This is the existing documented behaviour; confirm it still works after G1.4, where selection usually equals *next*, not live.
5. **Performance:** call `app.thumbnails.images()` once per `slides()` call, not per tile.
6. Tests: a unit test for the retry rule (`should_retry(failed_at, now, missing)`), and a test that three workers drain a queue of 6 synthetic requests (use a fake loader closure if the worker body can take one; otherwise test the pure retry and state functions only).

**Acceptance:** drop 10 videos and 10 photos from `~/Movies` onto the running order. Every tile shows "Loading preview…" with its name within one frame, and all real thumbnails appear. Time this; it should be about 3× faster than before. `missing.mov` shows "File not found". No tile or Preview is ever plain black. Capture it.

## G1.7 Audio and video transport (P1)

Files: `crates/lumen-media/src/audio.rs`, `crates/lumen-media/src/player.rs` (position/duration only), `crates/lumen-app/src/audio.rs`, `crates/lumen-app/src/panels.rs` (inspector, under the transport), `crates/lumen-app/src/app.rs` (pass-through methods only).

1. `AudioPlayer`:
   - `pause()` and `resume()` (`Sink::pause` / `Sink::play`) and `is_paused()`.
   - `duration()`: capture `Source::total_duration()` from the `Decoder` before `append`, and store it as `Option<Duration>`.
   - Store the playing file name.
   - `stop(fade)` is unchanged.
2. `MediaPlayer`: `position_secs()` (frames shown ÷ fps from the G1.1 pacer, as an atomic) and `duration_secs()` (from the probe). For a looped clip, the position is taken modulo the duration.
3. **Now-playing strip** in `panels::inspector`, directly under Clear / Blackout, shown only while something with sound is playing: an audio cue, or a video with `muted: false`. One row per source, at most two (audio + video):
   - The file name, truncated.
   - `1:23 / -2:41` in tabular figures.
   - A thin progress bar (`egui::ProgressBar`, height 4, no text).
   - Pause/Resume (a Phosphor icon button with a tooltip and an accessible label).
   - Stop: for audio, 300 ms fade (existing `CLEAR_FADE_MS`); for video, the same as `ClearMedia`.
   - For audio, a volume slider 0–1 bound to the live sink.
   - Tokens from `style.rs` only. Hit targets at least 28 pt (DESIGN.md control height).
   - Repaint at 4 Hz while it is visible, which is enough for a seconds clock.
4. The Audio tab stays as the bin and device picker. Rename "Stop" there to "Stop audio" and "New file volume" to "Volume for added files".
5. Tests: `AudioPlayer` pause/resume need hardware, so gate them like the existing audio tests. Add a pure `fn format_clock(elapsed, total) -> String` with tests for 0:00, 59:59, hours, and an unknown total ("1:23").

**Acceptance:** an audio cue in the running order goes live, and the strip appears with a moving clock. Pause freezes the clock and the sound; Resume continues; Stop fades. A `muted: false` video shows its own row, and Pause freezes both picture and sound. Capture it.

## G1.8 Wrap-up

- Run the full verification commands. Then do one native pass on two screens (laptop + L27q): G1.1 cadence numbers on the external display, G1.3 fades, G1.4 eight-Space walk, G1.5 and G1.6 captures, and G1.7 strip. Use window-only captures in `.local/qa/g1/shots/`.
- `docs/STATUS.md`: add a "Wave G1" section with what changed, the measured numbers (before → after), and what was not verified by hand. Note the known gaps: soft Preview, video volume not adjustable live, no seeking, per-slide Cut means show default.
- Update `DESIGN.md`: Keyboard, Layout (rail 300 pt, 240–520), the transport row (Transition picker, now-playing strip), and Motion (transitions are eased; the show default is a 400 ms fade).
- Bump the version to `1.0.0-rc.4` and package with `scripts/package-macos.sh --release`, following the rc.3 checks in STATUS.

## Order and sizing

| Phase | Priority | Size | Can run alongside |
|---|---|---|---|
| G1.1 video pacing | P0 | M | G1.2 |
| G1.2 duplicate items | P0 | S | G1.1 |
| G1.3 transitions | P0 | M | — (after G1.1, which shares the repaint change) |
| G1.4 arm next | P0 | S | — |
| G1.5 rail layout | P1 | S | — |
| G1.6 previews | P1 | M | — |
| G1.7 transport strip | P1 | M | — (after G1.1, which provides position) |
