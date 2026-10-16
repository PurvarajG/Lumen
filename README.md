# Lumen Presenter

A standalone native Rust presentation application, being developed toward ProPresenter feature parity. The first milestone is a working show editor with audience and stage outputs; full production parity is tracked explicitly in [docs/PARITY.md](docs/PARITY.md).

## Development

The initial target is Apple Silicon macOS. Rust stable, Xcode command-line tools and FFmpeg (`ffmpeg`, `ffprobe`, `ffplay`) are the development prerequisites. FFmpeg remains an external runtime dependency for video/audio in this milestone. Basic text and still-image presentation does not depend on the reference application.

```sh
. scripts/env.sh
cargo test --workspace
```

Native launch/package instructions will be verified and recorded when the application milestone builds. Current actual verification results are in [docs/STATUS.md](docs/STATUS.md).

## Structure

- `crates/lumen-core`: versioned project model, validated atomic storage, text import, detached live snapshots, cue navigation, timers and bounded macro actions.
- `crates/lumen-media`: owned FFmpeg processes, bounded frame delivery and media probing.
- `crates/lumen-app`: native egui operator interface, editing and output viewports (application work follows adapter verification).
- `docs/research`: evidence, observed behavior and limits from the installed ProPresenter 21.4 reference.

The editable project and live output are separate owned values. Editing a slide or theme does not update an audience screen until it is triggered again. Blackout and text clear are output masks; they do not discard the selected live cue.

## Research tooling

The requested [morluto/rea](https://github.com/morluto/rea) repository is cloned locally under ignored `.tools/rea`. The pinned REA 6.5.0 CLI is installed under `.tools/rea-runtime`; `scripts/rea.sh` configures the installed Ghidra/JDK paths. No global MCP configuration was changed. Large raw evidence, temporary native slices and the original research fixture stay in ignored `.local/research`.

Native inspection and controlled original test documents guide independent behavior and interchange work. The application contains no proprietary executable, decompiled implementation or copied reference-app creative assets. Findings distinguish direct observations from inferences; the two-slide `.pro` fixture does not establish arbitrary document compatibility.

## Autonomous work policy

The user authorized autonomous execution. The larger model handles architecture, interface contracts, integration and review. Smaller model workers implement bounded tasks and write compact reports to local files. Source, decisions and progress remain in this repository so the project can resume without repeating completed investigations.
