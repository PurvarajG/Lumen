# Foundation verification — 2026-10-16

Scope: Lumen Presenter 0.1.0 native Rust foundation and the requested gpt-taste UI refinement. This is not a full-parity or production-hardware certification.

## Automated checks

- `cargo fmt --all -- --check`: pass.
- `cargo test --workspace`: 44 passed, zero failed or ignored (app 16; core 12; media 16).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo build -p lumen-app`: pass.
- CLI help/demo export: successful. Missing projects, unknown arguments and invalid export combinations fail without the unintended export side effect.
- Real FFmpeg tests generate original clips and verify frame bounds, probing, non-looping EOF, pause/drop, path argument handling, timeout/error behaviour, mid-frame cancellation and owned-child reaping.
- Save coordination regressions cover failed/cancelled writes, edits during saves, stale completions and pending-action cancellation.
- Media reconciliation tests cover loop/mute identity, Clear All, stopped-video retriggering and joining cleanup workers.

The upstream `block` 0.1.6 dependency emits a Cargo future-incompatibility advisory with this Rust version. Current compilation, tests and Clippy pass. No dependency-wide upgrade was made during final acceptance.

## Parent native checks

Performed against the packaged `.local/dist/Lumen Presenter.app` with native UI automation on Apple Silicon macOS 26.7:

- Operator launches with actual Metal output; unique scroll IDs and readable glyphs. Final dark palette, embedded Geist, header/file controls, preview, cue grid and inspector tabs inspected visually.
- Triggered the original Welcome cue, then changed its editable text. Large preview and thumbnail changed while the live monitor retained the original text.
- Opened a local original generated-video show through the native picker. Background loader reached Video playing without additional interaction.
- Non-looping 3-second clip reached Video finished and did not restart. No FFmpeg decoder remained. Stop then explicit Go Live started the same clip again.
- Changing the same file to loop mode and triggering it started a `-stream_loop -1` decoder. Audience screenshots showed changing test-pattern frames and correctly centred two-line text without left clipping.
- Stage screenshots showed distinct live/next content, notes, a delivered stage message and a countdown advancing from 04:53 to 04:28 without operator interaction.
- Audience and stage close returned to the operator; output lifecycle close/reopen was exercised during this integration pass and the earlier regression pass.
- The Clear All macro cleared the live cue and removed the generated-video FFmpeg process.
- Native Cmd+Q with confirmed dirty state offered Save/Discard/Cancel. Cancel preserved the editable text. Save wrote the edited cue to an original QA project and exited. Clean Quit exited. Discard while looping video was active exited with no Lumen or matching FFmpeg child left behind.
- Final package New creates an empty Untitled show; Add selected document is disabled until a document exists. Stage tools remain available with an empty library. Create document makes the first cue. The window Close button's Cancel branch preserves the show.

The macOS termination adapter adds only the optional applicationShouldTerminate: method to the existing winit delegate class. It resolves AppKit's deferred request with NO and performs approved root Close through the event loop, avoiding synchronous termination reentry during redraw. On exit the app explicitly joins media cleanup.

## Review and evidence

Independent core, media and app reviews were performed. Findings fixed included atomic action handling, cancellation deadlock after decoder EOF, stale-save data loss, deferred-window close lifecycle, text double-alignment, output invalidation, async media wakeups, loop/mute identity, macro media cleanup, empty-library references, shutdown joins, negative timer components, unintended EOF looping, stopped-video retrigger and native Quit coordination. The final integration reviewer reported no residual source blocker.

Detailed raw logs and reports remain in ignored `.local/reports`, including native-qa.md and the integration review. Original controlled reference evidence is described under docs/research.

## Limits

No audible hardware loopback/sync measurement, external monitor/device acceptance, notarization, minimum-OS validation or Windows/Linux launch was performed. Only cut transitions and one text/background composition are rendered. ProPresenter native import and OpenLyrics are designs for the next milestone, not shipped features. See PARITY.md.

---

# Release packaging and profile verification (D1) — 2026-10-16

Scope: Lumen Presenter 1.0.0-rc.1 release packaging, profile optimization, bundled licenses, authored vector app icon, and compressed DMG distribution.

## Automated checks
- `cargo fmt --all -- --check`: pass.
- `cargo test --workspace`: all workspace tests pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- Binary size measurement:
  - Before profile optimization (default release profile): 22,408,320 bytes (21 MiB).
  - After profile optimization (`lto = "thin"`, `codegen-units = 1`, `strip = true`, `panic = "unwind"`): 16,867,920 bytes (16 MiB).
  - Net reduction: 5,540,400 bytes (-24.7%).

## Bundle and packaging verification
- `scripts/package-macos.sh --release`: succeeded cleanly.
- Workspace version `1.0.0-rc.1` dynamically extracted from root `Cargo.toml` into `Contents/Info.plist`.
- Bundled licenses verified in `Contents/Resources/Licenses/`:
  - `Poppins-OFL.txt`
  - `NotoSansGujarati-OFL.txt`
  - `Phosphor-MIT.txt`
  - `Lumen-MIT.txt`
- Authored vector app icon (`crates/lumen-app/assets/icon.svg`) compiled into multi-resolution `AppIcon.icns` via `sips` and `iconutil`, and verified in `Contents/Resources/AppIcon.icns`.
- Prohibited file assertions: verified that neither ChitraUni font files nor book text files are bundled in the application bundle.
- Code signing check: `codesign --verify --deep --strict` on `.local/dist/Lumen Presenter.app` passed.
- DMG creation and verification:
  - Formatted with `hdiutil create -format UDZO`.
  - Artifact: `.local/dist/Lumen-Presenter-1.0.0-rc.1-macos-arm64.dmg`.
  - Size: 8,332,297 bytes (7.9 MiB).
  - SHA-256: `b55ac8d382819928ed6bf56dfd38c38691c7a531124cc9dd1387861781d207b1`.
  - Checksum verification: `hdiutil verify` reported VALID.
  - Mount test verified containing both `Lumen Presenter.app` and an `/Applications` symlink.
