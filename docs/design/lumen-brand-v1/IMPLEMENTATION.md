# Integration specification

## Change boundary

Integrate the selected identity into the existing native app and macOS bundle. Preserve the current console's structure, dimensions and workflow. No new dashboard, menus, panels, fonts, decorative backgrounds, animations, or audience slide themes are part of this change.

## Header component

All measurements below are logical egui points, not screenshot pixels.

| Property | Value |
| --- | --- |
| Existing toolbar outer padding | Keep `style::PADDING = 12` |
| Mark allocation | 28 × 28 |
| Mark-to-name gap | 8 |
| Name | `Lumen`, bundled Poppins SemiBold, 18 pt, `style::TEXT` |
| Descriptor | `PRESENTER`, Poppins Medium, 11 pt, `style::TEXT_DIM` |
| Name-to-descriptor gap | 8 |
| Full lockup allocation | 188 × 28 |
| Compact lockup | Mark + Lumen, approximately 106 × 28 |
| Minimum | Mark only, 28 × 28, accessible name Lumen Presenter |
| Brand colour | `style::SAFFRON` #F2A33A |
| Interaction | Static identity; no new navigation or focus stop |
| Hover | Optional tooltip: Lumen Presenter plus current show name |

A dark `PRESENTER` descriptor in native text can use TEXT_DIM. The supplied all-light header export uses TEXT for maximum raster contrast; native text is the preferred integration. Align mark and text optically to the current first row. Keep the existing 8 pt item spacing and separators. Retain the two-row toolbar and its current top-left flow. Do not enlarge the header to imitate the generated image.

At available first-row widths below approximately 1200 pt, omit the descriptor if needed. Below approximately 1000 pt, omit the wordmark if needed. These are fallback guidelines, not instructions to shrink/restructure the whole window: calculate available width with the current controls and use egui wrapping where it already exists. Functional controls, dirty-state text and names in Prepare mode take priority. Do not shrink text below 11 pt.

Keep `style::LEFT_WIDTH = 280` and `style::RIGHT_WIDTH = 360` and their resize behaviour. Do not replace these with the proportions in the generated mockup. Central preview remains 16:9 with current fit logic. Keep the current thumbnail sizing, document groups and scrolling.

## Project-name behaviour

`app.rs::toolbar` currently displays `project.name` through Gujarati-aware rendering in Present mode and exposes an editable text field in Prepare mode. “Sunday Gathering” comes from demo project data in `lumen-core/src/model.rs`; it is not a fixed app title.

- Present: put the brand lockup where the prominent project-name label currently appears. A hover tooltip may show the current project name; use existing shaping if rendering Gujarati.
- Prepare: retain the existing editable name field after the compact brand and before the Prepare/Present controls. Limit its initial width to about 160 pt and retain normal edit/undo/dirty/save behaviour. If the current implementation has a related defect, fix only what is necessary to preserve that behaviour.
- Do not rename existing shows, migrate persisted data, remove name fields from the schema, or replace all occurrences of the demo title.
- Keep window title, file status and unsaved warnings consistent with current application behaviour.

## Exact feature preservation

| Region | Must remain accessible and functional |
| --- | --- |
| Header | Prepare/Present; Audience, Stage, Remote state; unsaved text |
| Toolbar row 2 | File and all its actions; Undo/Redo; Settings; Outputs/Identify; Shortcuts |
| Left rail | Search; Running order, Library, Books; Add files; document rows; New document; drag/reorder/context menus; Macros including Blackout, Restore output, Quiet stage |
| Centre | Preview; show/document context; edit/live isolation notice; selected-document slides; on-deck/LIVE affordances; all current Prepare inspector/editor capabilities |
| Right | Live monitor; Previous, Go live/Next, Clear, Blackout; Timers, Messages, Props, Audio, Stage tabs; overlay chips and On/Off actions |
| Timers | Add timer; name/type/duration; Start/Pause/Reset and overflow actions; overrun and warning settings |
| Footer | Saved/Unsaved; file/path context; current status/errors |
| Output | Existing separate live snapshot; audience/stage rendering; deck, video, captions, messages, props and overlays |

The reference screenshot only shows one tab and demo state. It is not a list of all available features. Newer code and features take precedence over historical screenshots.

## Asset rules

The mark is the same two-path silhouette in every variant. Internal gaps are transparent; never fill them with black to fake transparency. Keep its aspect ratio. Use an 8 pt clear space around an isolated 28 pt UI mark where existing row spacing permits. Do not add a plate, bevel or glow in the toolbar.

Use `lumen-mark-saffron-64.png` or a larger export as an embedded cached texture at 28 pt on Retina. `lumen-header-on-dark@2x.png` is a ready-to-use complete lockup if native text is impractical. Give image-only controls/labels an accessible name where the current egui version supports it. Do not decode images or create textures each frame. A painter implementation must preserve both paths and transparent gap; use SVG as the geometry source.

The 1024 px macOS icon uses the same transparent mark, centred in a charcoal 824 × 824 plate at x=100,y=100 with radius=185. This is separate from in-app branding. The supplied ICNS includes all standard sizes. Small mark exports have the same geometry, so check the diagonal gap at 16–28 px. If a small-size optical correction is needed, document it explicitly rather than inventing another logo.

## Repository integration map

Line positions are from 16 October 2026 and may move. Locate symbols before editing.

| Location | Purpose / action |
| --- | --- |
| `crates/lumen-app/src/app.rs`, `toolbar`, around line 1277 | Replace prominent name display; retain naming in Prepare and all existing controls |
| `crates/lumen-app/src/style.rs` | Existing palette, fonts and widths; add only scoped brand sizing constants if useful |
| `crates/lumen-app/src/main.rs`, NativeOptions around line 108 | Inspect viewport icon support; integrate only if needed by current native setup |
| `crates/lumen-app/assets/` | Destination for implementation-ready runtime assets |
| `crates/lumen-app/assets/icon.svg` | Existing flame/diya icon master; replace during implementation |
| `scripts/generate-icon.sh` | Currently draws flame/diya in Swift independently of SVG; must be updated with new geometry |
| `scripts/package-macos.sh` | Inspect existing AppIcon.icns copy/Info.plist handling and verify actual bundle |
| `crates/lumen-core/src/model.rs`, demo around line 777 | Context only: source of demo show title; no global rename |
| `docs/STATUS.md` | Record integration and actual verification |

Prefer existing dependencies and embedded PNG decoding. Do not add a webview, CSS system, runtime font download, SVG parser or image-generation dependency. This package's Python/fontTools/rsvg tooling is build-time handoff tooling only.

## Acceptance and verification

1. At the app's current 1500 × 982 baseline, only branding/title treatment and app icon differ materially. The two-row toolbar, panel divisions, control order, preview and thumbnails retain their existing layout.
2. “Sunday Gathering” is absent from the prominent Present header. Any show name, including Gujarati, remains editable in Prepare and survives save/reopen. No user document is silently renamed.
3. Every feature in the preservation table works, including features absent from the reference screenshot. Unsaved indicators and quit/recovery prompts remain intact.
4. No application branding is composited into audience or stage output. Changing header appearance never triggers or mutates live output.
5. Check approximately 1024, 1280 and 1500 pt window widths where supported, plus Prepare and Present. Narrow layout retains all functional controls and uses the brand fallbacks. Inspect Retina and non-Retina rendering where available.
6. Inspect the supplied mark at 16, 24, 28 and 32 px for the transparent diagonal gap and consistent silhouette. App icon stays recognizable in Dock/Finder sizes.
7. Rebuild the icon with the updated production generator; verify it cannot resurrect the old flame icon. Verify the packaged app uses the new ICNS. Preserve current signing and packaging behaviour.
8. Run the project-required checks: `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`. Add meaningful tests only for changed behaviour; no tests that merely duplicate static asset values.
9. Capture actual native window-only before/after images and inspect them. Report unverified hardware/scales honestly. Generated images are not application validation.

No source implementation or Rust verification was performed as part of preparing this design package.
