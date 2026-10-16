# Lumen Foundation Implementation Plan

> **For agentic workers:** Use subagent-driven-development with compact file briefs, independent review and autonomous decisions under the user's instruction.

**Goal:** Deliver a runnable macOS Rust presentation workbench with tested cue semantics and separate live outputs.

**Architecture:** A Cargo workspace separates lumen-core (pure domain/storage), lumen-media (owned FFmpeg processes) and lumen-app (native operator/output UI). Research evidence stays outside application source.

**Tech Stack:** Rust stable, serde/serde_json, uuid, thiserror, eframe/egui 0.33.3, image, rfd, FFmpeg/ffplay.

## Global constraints

- All application code is Rust; macOS first, portable core.
- Live output is a snapshot and does not change during editing.
- Persisted project schema version is 1; maximum project JSON size is 16 MiB.
- UI minimum size is 1100×700; default size is 1500×950; output canvas is 1920×1080.
- Keep target build small with dev/test debug=0 and incremental=false.
- Workers change only owned files and do not commit; the orchestrator reviews and commits.
- No copied proprietary source or assets, no advertising an unverified capability.

## Task 1: Core model, runtime and storage

**Files:** crates/lumen-core/Cargo.toml, crates/lumen-core/src/{lib,model,runtime,storage,import}.rs, crates/lumen-core/tests/*.rs.

**Interfaces:** Read docs/briefs/core.md for the exact public contracts. Parent supplies workspace manifest. No UI dependencies.

- [x] Write tests proving snapshot isolation and ordered playlist navigation across documents, including empty and boundary cases.
- [x] Run `cargo test -p lumen-core` and capture the initial failure.
- [x] Implement the public models and runtime to meet those cases.
- [x] Add deterministic timer/macro tests and malformed project/storage/import tests, then implement those features.
- [x] Run `cargo test -p lumen-core` and `cargo clippy -p lumen-core --all-targets -- -D warnings`.
- [x] Save the final report to .local/reports/core.md; parent reviews source and verification output.

## Task 2: Media adapter

**Files:** crates/lumen-media/Cargo.toml, crates/lumen-media/src/*.rs, crates/lumen-media/tests/*.rs.

**Interfaces:** Read docs/briefs/media.md. Media adapter exports RGBA frames and owns child processes; no UI/core dependencies.

- [x] Test media probing/error behavior and frame sizing with a generated local FFmpeg test pattern.
- [x] Implement an off-thread decoder with a single latest frame, bounded stderr and explicit stop/drop cleanup.
- [x] Add pause/restart/loop/mute controls supported by the selected adapter and expose actual capabilities.
- [x] Verify lifecycle with a generated short video and run media crate tests/clippy.
- [x] Save report to .local/reports/media.md; parent reviews and runs integration verification.

## Task 3: Native application and outputs

**Files:** crates/lumen-app/Cargo.toml, crates/lumen-app/src/*.rs, examples/demo.lumen.json, scripts/run.sh, scripts/package-macos.sh.

**Interfaces:** Consume the reviewed core/media public APIs. Read docs/briefs/app.md for UI requirements and exact behavior.

- [x] Implement the native workbench and document/slide editing, with cloned live render state.
- [x] Wire project load/save/text import dialogs and unsaved-change handling.
- [x] Implement original demo content, audience/stage viewports, keyboard controls, timer/macro controls and media rendering.
- [x] Build the app and run full workspace tests/clippy/format.
- [x] Launch the native app; inspect the workbench and both output viewports, correct defects found.
- [x] Package a local macOS .app and record runtime dependencies and unimplemented parity features.

## Task 4: Integration and independent review

**Files:** README.md, docs/STATUS.md, docs/research/*, relevant source fixes.

- [x] Have a fresh reviewer inspect source/diff, contracts and test coverage for correctness and reliability.
- [x] Fix actionable findings and rerun affected checks.
- [x] Run `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build -p lumen-app`.
- [x] Record actual output-window smoke checks, app artifact path, evidence limitations and the parity roadmap.
- [x] Commit only source, lockfile, original demo and docs. Keep tools, build products and raw research local.
