# Media adapter contract

Own crates/lumen-media/** only. Native Rust media adapter for the Lumen presentation workbench. Parent owns manifest/git/core/app. Read AGENTS.md. Cargo tools are in /opt/homebrew/opt/rustup/bin. FFmpeg, ffprobe and ffplay are available in /opt/homebrew/bin. Do not copy sample reference-app media; generate fixtures in temp dirs with FFmpeg lavfi. No UI/core dependencies.

## Public API (re-export from lib.rs)

- `MediaError` implements std::error::Error/Display, with descriptive path/process/format errors.
- `MediaInfo { pub width:u32, pub height:u32, pub duration_secs:Option<f64>, pub has_audio:bool }`, Clone/Debug.
- `pub fn probe(path:&Path)->Result<MediaInfo,MediaError>` ffprobe with JSON parsing, no shell; validate regular file and finite sane stream metadata. Missing file/executable, invalid data and nonzero subprocess exit must be errors, never hangs. Bound metadata/stdout and subprocess time (10 seconds). Include stderr reason bounded to a few KiB.
- `Frame { pub width:u32, pub height:u32, pub rgba:Vec<u8>, pub sequence:u64 }`, Clone/Debug.
- `PlaybackOptions { pub width:u32, pub height:u32, pub fps:u32, pub looped:bool, pub muted:bool }`, Default 1280×720, 30fps, looped=true, muted=true. Limit dimensions to 1..=3840 by 1..=2160 and fps to 1..=60 with checked multiplication before allocating. Decode preserving original aspect ratio with letterbox padding.
- `MediaPlayer::open(path:&Path,options:PlaybackOptions)->Result<Self,MediaError>` preflight path/options, launch worker/owned ffmpeg and optionally ffplay audio (if unmuted and stream has audio). Decoder runs off the UI thread, raw RGBA at requested size/fps, paced in real time. No shell interpolation. Starting/open may probe (up to 10s); app must call it on its own loader thread.
- `latest_frame(&self)->Option<Frame>` returns newest complete frame, never partial. Keep only one retained frame; no queue growth. Access must not block behind I/O or a worker join.
- `error(&self)->Option<String>` reports decode/audio runtime errors; stderr drains continuously into bounded retained text.
- `is_finished(&self)->bool`, `is_paused(&self)->bool`, `set_paused(&mut self,paused:bool)->Result<(),MediaError>`.
- `stop(&mut self)` idempotent, terminates and reaps owned children, unblocks reads, joins worker. Drop calls stop. Handle child already exited, EOF midframe, failed launch of second child, poisoned locks without panic. No detached permanent readers/threads.
- `pub fn tool_availability()->Vec<(String,bool)>` FFmpeg/ffprobe/ffplay availability using command -version (no shell).

## Decisions

Use owned child handles and stdout pipes. Keep lifecycle coordination separate from frame mutex; never hold the frame lock during blocking reads. Pace with FFmpeg -re or equivalent. On macOS/Unix pause/resume may use libc SIGSTOP/SIGCONT only on owned currently-live child handles, guarded against exit/reaping/PID reuse; stop must resume then kill/reap a paused child as needed. Non-Unix can return unsupported pause clearly if no portable solution. Always terminate both video and audio when dropping. Synchronization of independent audio and video processes is approximate and must be recorded honestly; strict production sync is a later milestone. No seek claim in this adapter.

Error handling should preserve a last good frame if decoding later fails. EOF of a non-looping clip marks finished. The `Frame.sequence` grows for each newly published frame so the UI only uploads changed pixels.

## Tests and report

Test-first: invalid options/missing media/invalid media, real probe of generated short test pattern, wait (bounded) for a correctly sized complete first frame, playback completion for non-loop clip, stop/drop and pause cleanup with a generated longer clip. Skip real-tool tests only if tools unavailable and report skips; do not fake a working decoder. Tests must not require audio hardware (mute fixtures). Test paths with spaces and prove no command-shell injection. Record actual initial failure/final test commands and process cleanup observations in .local/reports/media.md. Return concise status and limitations. No git commits, workspace edits or lengthy chat output.
