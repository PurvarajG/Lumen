# A3: lumen-control crate (remote control seam)

Create `crates/lumen-control/**` only (workspace members are `crates/*`; Cargo.lock may update). Do not depend on or edit lumen-core or lumen-app — the app maps these types later. Read `AGENTS.md` and follow `~/.claude/skills/ponytail/SKILL.md`. Write tests first. Allowed deps: `serde`, `serde_json`, `uuid` (workspace), `thiserror`, `tiny_http` (HTTP), `rosc` (OSC), `midir` (MIDI input). Nothing async, no tokio.

## Types

- `ControlCommand` (serde, tagged `{"cmd": "..."}`): `Next`, `Previous`, `Trigger { index: usize }` (flat playlist cue index), `Blackout { on: bool }`, `ClearText`, `ClearAll`, `RunMacro { name: String }`, `TimerStart { name }`, `TimerPause { name }`, `TimerReset { name }`, `StageMessage { text: String }` (max 500 chars).
- `StatusSnapshot` (serde): `project: String, live_index: Option<usize>, live_text: String, next_text: String, blackout: bool, timers: Vec<(String, String)>`. The app publishes it through `StatusHandle` (`Arc<Mutex<StatusSnapshot>>` wrapper with `set`/`get`).
- `ControlEvent { source: Source, command: ControlCommand }`, `Source { Http, Osc, Midi }`.

## HTTP server

`HttpServer::start(config: HttpConfig, tx: std::sync::mpsc::Sender<ControlEvent>, status: StatusHandle) -> Result<HttpServer, ControlError>`; `HttpConfig { bind: SocketAddr, token: String }`. Runs on its own thread; `shutdown()` and `Drop` stop it and join (no leaked thread — test it). Token must be ≥16 chars; refuse to start otherwise. Every request needs header `Authorization: Bearer <token>` (constant-time compare) else 401. Routes: `GET /status` → JSON snapshot; `POST /command` JSON body ≤ 4 KiB → 202, malformed → 400 with message; anything else 404. Bind default suggestion `127.0.0.1:47800`; non-loopback binds are allowed but the doc comment must say the token is the only protection and traffic is plaintext.

## OSC listener

`OscListener::start(bind: SocketAddr, tx) -> Result<..>` UDP thread with clean shutdown. Addresses: `/lumen/next`, `/lumen/previous`, `/lumen/trigger i`, `/lumen/blackout i(0/1)`, `/lumen/clear/text`, `/lumen/clear/all`, `/lumen/macro s`, `/lumen/timer/start|pause|reset s`, `/lumen/message s`. Bundles flattened. Unknown/malformed packets ignored (counted in an `AtomicU64` exposed via `dropped()`), never panic. Packets > 64 KiB dropped.

## MIDI

`midi_ports() -> Result<Vec<String>, ControlError>` and `MidiListener::start(port_name: &str, map: Vec<MidiMapping>, tx)` where `MidiMapping { channel: u8 (0-15), note: u8, command: ControlCommand }`; Note On velocity>0 sends the mapped command. Pure function `map_midi_message(&[u8], &[MidiMapping]) -> Option<ControlCommand>` carries the logic and is unit-tested; the port listener itself just forwards (no hardware in tests).

## Acceptance

- Tests: command JSON round trip and rejection; HTTP on `127.0.0.1:0` — 401 without/wrong token, 200 status, 202 command arrives on the channel, 400 malformed, 413/400 oversize, 404, shutdown joins; OSC encode with `rosc` → send to listener → command; malformed UDP increments dropped; MIDI mapping including channel mismatch and velocity 0.
- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` pass.
- Commit on your branch. Report to `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/a3-control.md`: API, tests, limitations. Final reply: 10 lines max.
