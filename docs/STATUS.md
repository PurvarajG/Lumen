# Lumen Presenter status

Objective: full standalone Rust alternative to ProPresenter. User authorized autonomous work on 2026-10-16.

## Current work

- Reference app verified: ProPresenter 21.4 (352583705), universal Mach-O, /Applications/ProPresenter.app.
- REA 6.5.0 installed locally; upstream shallow clone in .tools/rea; CLI used without restarting the chat.
- Ghidra 12.1.4 and OpenJDK 21 installed through Homebrew; REA provider doctor passed. Rust stable 1.99.1 installed with rustfmt/clippy.
- Architecture/spec and first milestone plan written. App is named Lumen Presenter; native Rust egui UI, portable domain core, FFmpeg media adapter.
- Static REA artifact/plist/Mach-O research completed by a smaller model; native UI observations recorded. Focused Ghidra follow-up is analyzing an arm64 copy of ssAPI with a 2GiB heap and retained snapshot.
- Core engine implemented by gpt-6-luna. Independent review requested Runtime serde and additional atomic-trigger/save-replacement/macro coverage; those changes were applied.
- Core independent re-review passed specification and quality; committed as a380657.
- Media adapter implemented by gpt-6-luna. Review reproduced and then verified fixes for an EOF cancellation deadlock and a spurious deliberate-stop error. Active/paused-drop reaping and error-path regressions added; independent re-review passed specification and quality.
- An original two-slide reference fixture was created through ProPresenter's UI and inspected separately. Observed protobuf-wire-compatible field paths recover the two known RTF texts in order; broad `.pro` compatibility remains unverified.

## Verification

- Parent ran `cargo test -p lumen-core`: 12 integration tests passed, 0 failed.
- Parent ran `cargo clippy -p lumen-core --all-targets -- -D warnings`: passed.
- Parent ran `cargo test -p lumen-media`: 11 unit plus 5 integration tests passed; no failures or skips. Parent reran media clippy with warnings denied: passed.
- Native application and media adapter are not yet built. Full application verification is pending.

## Remaining parity work

See docs/superpowers/specs/2026-10-16-lumen-design.md for the complete milestone roadmap. Full parity is not complete at the foundation milestone.
