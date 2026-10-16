# Lumen Presenter status

Objective: full standalone Rust alternative to ProPresenter. User authorized autonomous engineering and efficient model delegation on 2026-10-16. Latest steering: finish the build and use gpt-taste to fix the UI.

## Delivered foundation

- Rust workspace with reviewed pure core, owned FFmpeg media adapter, and native egui/WGPU workbench. Larger models handled architecture/review; gpt-6-luna implemented modules. Parent completed final changes after worker usage exhaustion.
- Core committed as a380657; media committed as c229612. App integration and the requested UI refinement complete the foundation milestone.
- Native editor provides blank shows, editable title/documents/slides/themes, playlist order, text import, notes, macros, countdowns and stage messages. Editable state remains separate from cloned live output.
- Audience/stage windows render independently. Video cancellation/EOF/retrigger, repaint, text anchoring, close/reopen and native unsaved Quit defects found in review were fixed and exercised.
- gpt-taste guidance adapted to the Rust workbench: dark graphite, Geist, clear contrast/live states, large preview, consistent thumbnails and separate edit/stage inspector views. See UI.md.
- Local bundle: `.local/dist/Lumen Presenter.app` (0.1.0 debug build, ad-hoc signed). FFmpeg remains an external media dependency. No reference app is needed to run Lumen.

## Verification

Formatting, all 44 workspace tests, Clippy with warnings denied and native build passed. Parent exercised native editing/live isolation, generated video, outputs, countdown/message, save and Quit flows. Final independent source review reports no residual blocker. Full evidence and limitations: VERIFICATION.md.

## Research

REA 6.5.0 and upstream source are local. Ghidra 12.1.4/OpenJDK21 doctor passed; focused native analysis completed. Reference app: ProPresenter 21.4 build352583705 at /Applications/ProPresenter.app, read-only except original test documents created through its UI. Static/native/UI findings and original Unicode fixtures are documented under research. Raw artifacts stay ignored in .local/research.
- Anirdesh.com structural research completed (R2 brief): mapped URL templates, hierarchy, DOM selectors, dual-language extraction, and slide units across all 16 scripture sections in docs/research/anirdesh-structure.md. Documented Nginx User-Agent filtering behavior and exact unit counts. Sample HTML files preserved in .local/research/anirdesh/ (untracked).

## Remaining

Full ProPresenter parity is not complete. PARITY.md tracks absent features. The next interchange milestone is designed but unimplemented: OpenLyrics plus explicitly experimental, text-only extraction from the observed ProPresenter fixture field paths. Rich layers, fades, undo/recovery, production audio/output adapters and service integrations remain future work. Do not advertise those as available.


## B1 console executor (wave/b1-console, 2026-10-16)

Uncommitted implementation in the B1 worktree; **blocked**, not a verified milestone. Core tests 69 passed and latest app suite 35 passed. Workspace tests stop at lumen-control `test_midi_ports_query` because CoreMIDI initialization fails on this host. Per executor instruction, stopped without modifying that out-of-scope crate. Final fmt check, clippy, smoke and native screenshots remain pending. Report: `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/b1-console.md`. Parent owns integration and commit; no git writes attempted.
