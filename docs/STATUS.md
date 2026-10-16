# Lumen Presenter status

Objective: full standalone Rust alternative to ProPresenter. User authorized autonomous work on 2026-10-16.

## Current work

- Reference app verified: ProPresenter 21.4 (352583705), universal Mach-O, /Applications/ProPresenter.app.
- REA 6.5.0 installed locally; upstream shallow clone in .tools/rea; CLI used without restarting the chat.
- Ghidra 12.1.4 and OpenJDK 21 installed through Homebrew; REA provider doctor passed. Rust stable 1.99.1 installed with rustfmt/clippy.
- Architecture/spec and first milestone plan written. App is named Lumen Presenter; native Rust egui UI, portable domain core, FFmpeg media adapter.
- Static REA artifact/plist/Mach-O research completed by a smaller model; native UI observations recorded. Focused Ghidra follow-up is analyzing an arm64 copy of ssAPI with a 2GiB heap and retained snapshot.
- Core engine implemented by gpt-6-luna. Independent review requested Runtime serde and additional atomic-trigger/save-replacement/macro coverage; those changes were applied.

## Verification

- Parent ran `cargo test -p lumen-core`: 12 integration tests passed, 0 failed.
- Parent ran `cargo clippy -p lumen-core --all-targets -- -D warnings`: passed.
- Native application and media adapter are not yet built. Full application verification is pending.

## Remaining parity work

See docs/superpowers/specs/2026-10-16-lumen-design.md for the complete milestone roadmap. Full parity is not complete at the foundation milestone.
