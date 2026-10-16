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
