# Lumen Presenter

Build an independent, standalone Rust presentation application, working toward ProPresenter feature parity. The user authorizes autonomous engineering decisions and tool setup without routine input. macOS is the first delivery platform; keep the domain core portable.

## Model and context policy

- The primary larger model owns architecture, interfaces, integration decisions and final review.
- Use gpt-6-luna for bounded implementation and research with exact file ownership and acceptance criteria. Escalate to gpt-6.1-sol only when repeated failures or integration complexity justify it.
- Give workers brief file paths and required interface context, not the entire conversation. Save full findings and logs to files and return compact summaries.
- Do not run conflicting edits concurrently. Keep durable progress in docs/STATUS.md.

## Engineering

- Application code is Rust. Use eframe/egui for native UI; external media libraries/tools are allowed and their runtime requirements must be disclosed.
- Separate persisted editable project state from a cloned live-output snapshot. Editing a slide must not change live output until triggered.
- Validate project inputs and references. Save atomically. Never discard unsaved edits silently.
- Use deterministic core tests for cue navigation, clocks, macro behavior, malformed documents and storage round trips. Verify the native app launches and check output windows visually.
- Run cargo fmt --all -- --check, cargo test --workspace, and cargo clippy --workspace --all-targets -- -D warnings before claiming a milestone verified.
- Keep build disk usage bounded: dev debug=0 and incremental=false. Do not delete unrelated user files.

## Research

- Use the pinned local REA CLI in .tools/rea-runtime; upstream source is in .tools/rea. CLI operation does not require changing global MCP configuration or restarting this chat.
- Inspect /Applications/ProPresenter.app read-only. Preserve observed facts, inferences, limitations, command provenance and evidence digests in docs/research; large raw evidence belongs in .local/research.
- No proprietary executable, decompiled implementation or creative assets in the application source. Implement documented and observed behavior independently.
- Use gstack browse for web browsing.
