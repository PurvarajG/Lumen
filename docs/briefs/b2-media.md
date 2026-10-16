# B2: lumen-media thumbnails and audio

Own `crates/lumen-media/**` only (Cargo.lock may update). Do not depend on lumen-core. Read `AGENTS.md`, follow `~/.claude/skills/ponytail/SKILL.md`, read the existing `src/lib.rs`, `src/player.rs`, `src/probe.rs` and reuse their FFmpeg discovery, validation, timeout and child-process cleanup patterns. Write tests first; tests generate media with ffmpeg `lavfi` sources (skip with a clear message only if ffmpeg is missing, matching existing tests).

## Thumbnails

`thumbnail(path: &Path, max_w: u32, max_h: u32) -> Result<Frame, MediaError>`: images and videos (video: frame at min(1 s, 10% of duration)), aspect preserved inside the box, RGBA `Frame` like the player's. One ffmpeg child, hard 10 s timeout, child killed and reaped on timeout/error, output size bounded to `max_w*max_h*4` (reject max dims > 1024). Plus `ThumbnailCache` — LRU by (canonical path, mtime, size, max dims), capacity in entries (default 256), `get_or_load`. Loading must be callable from a worker thread; the cache is `Send`, not required to be `Sync`.

## Audio

Preferred: `rodio` (default-features off; enable only the symphonia decoders for wav, mp3, flac, aac/m4a and the playback feature it needs). If the dependency fails to build on this machine (e.g. needs libclang), fall back to an owned `ffplay -nodisp -autoexit -loglevel error -volume N` child with default device only, and say so in the report.

`AudioPlayer` (one per output bus) with: `output_devices() -> Result<Vec<String>, MediaError>` (free fn), `AudioPlayer::new(device: Option<&str>) -> Result<Self, MediaError>` (None = default, unknown name = error), `play(path, AudioOptions { volume: f32 0..=1, looped: bool, fade_in_ms: u32 }) -> Result<(), MediaError>` replacing whatever is playing, `stop(fade_out_ms: u32)`, `set_volume`, `is_playing`, `position() -> Option<Duration>`, `Drop` stops and releases the device. Invalid volume/paths are errors, not panics. No blocking longer than ~50 ms on the caller thread (decode happens on the audio thread/child).

## Acceptance

- Tests: thumbnail of generated PNG and MP4 has expected aspect-fit dims and non-uniform pixels; missing file / directory / garbage file errors; oversize dims rejected; cache hit returns same frame and invalidates on mtime change; audio: generated 1 s sine WAV plays and `is_playing` goes true then false after it ends (allow CI without an audio device: if `AudioPlayer::new(None)` fails with no device, skip with a message), stop with fade returns promptly, invalid volume rejected.
- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.
- Commit on your branch. Report to `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/b2-media.md` (API, which audio backend, runtime requirements/licences of new deps, limitations). Final reply: 10 lines max.
