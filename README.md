# Lumen Presenter

A working native Rust presentation workbench for macOS, developed toward a standalone ProPresenter alternative. Version 0.1.0 delivers the foundation and a polished operator UI. **Full ProPresenter parity is not complete.** See [the parity tracker](docs/PARITY.md) for verified features and remaining work.

## Run the app

The verified local development bundle is `.local/dist/Lumen Presenter.app`. Open it in Finder, or from this checkout:

```sh
open ".local/dist/Lumen Presenter.app"
```

For development and rebuilding:

```sh
scripts/run.sh
scripts/verify.sh
scripts/package-macos.sh
```

`package-macos.sh --release` builds an optimized bundle; the currently verified artifact is the debug build. The bundle is ad-hoc signed for local use, not notarized for distribution. This build was exercised on Apple Silicon macOS 26.7. Other platforms and the minimum deployment OS have not been validated.

Rust stable and Xcode command-line tools are build prerequisites. Video requires external `ffmpeg` and `ffprobe`; audible video additionally requires `ffplay`. These are installed on this machine and discovered in Homebrew's standard paths when launching from Finder. The bundle does not embed FFmpeg. Text and still images work independently of ProPresenter and FFmpeg. External media paths are stored as references; moving a show does not copy its media.

## Operate a show

The initial demonstration is original sample content. **New** starts an empty show with reusable default themes. Edit the show name in the header, create a document, add slides, and add the document to the playlist. Blank paragraphs in an imported UTF-8 text file become separate slides.

Select a thumbnail to edit its label, text, notes, theme, alignment, colour or background. The large preview shows the selected cue. **Go Live** or double-clicking a cue triggers a cloned snapshot: subsequent edits stay off air until triggered again. The right-hand monitor shows actual live state. **Previous/Next** follow playlist order; **Blackout**, **Clear Text**, **Clear All**, video pause/stop and predefined macros affect output.

**Audience** opens a separate clean presentation window. **Stage** opens current/next previews, notes, countdown clocks and the stage message. In the inspector, **Stage tools** exposes timer and message controls. **Output settings** selects fullscreen for each output; Escape returns it to a window. Output windows can close and reopen independently.

Use Save/Save As for versioned `.lumen.json` projects. Writes validate the model and replace the destination atomically. New, Open, the window Close button and native macOS Quit guard unsaved changes with Save/Discard/Cancel. Runtime clocks and live state are not saved as show content.

Keyboard controls are listed in **Shortcuts**: Cmd/Ctrl+S saves, Cmd/Ctrl+O opens, Enter triggers, Space/Right advances, Left goes back and B toggles blackout. Cue shortcuts are suppressed while typing in an editor.

## Verified and remaining

The final foundation check passes formatting, **44 tests**, Clippy with warnings denied, and a native build. Native UI checks cover edit/live isolation, audience/stage output, generated video playback and end behaviour, timer progression, stage messages, independent output close/reopen, atomic save through the native picker, and macOS Quit cancellation/save/discard. Review findings and verification are recorded in [docs/VERIFICATION.md](docs/VERIFICATION.md). Design decisions are in [docs/UI.md](docs/UI.md).

This version uses one text object per slide, cut transitions and a single media background. It does not implement rich text/object layers, undo/redo, arbitrary ProPresenter import, OpenLyrics, NDI/SDI/Syphon, capture/recording/streaming, remote protocols, Bible services, cloud collaboration or production audio routing. Audio playback uses an independent ffplay process with approximate sync; audible output and external display/device hardware have not been acceptance-tested. Different, untriggered external-media previews currently have no separate thumbnail decoder. These limitations are explicit in the parity tracker.

## Structure and research

- `crates/lumen-core`: validated project model, atomic storage, plain-text import, detached live snapshots, navigation, clocks and bounded atomic macro actions.
- `crates/lumen-media`: bounded FFmpeg frame delivery, probing, pause/stop and owned-child cleanup.
- `crates/lumen-app`: Rust egui/WGPU operator UI and independent output viewports. No webview or JavaScript runtime.
- `docs/research`: observations, evidence and inference limits from the installed ProPresenter 21.4 reference.

The requested [morluto/rea](https://github.com/morluto/rea) source is cloned into ignored `.tools/rea`. Pinned REA 6.5.0 runs from `.tools/rea-runtime`; `scripts/rea.sh` configures the local Ghidra/JDK paths. It was used for artifact, plist, Mach-O and focused native analysis without changing global MCP configuration. Original two-slide and Unicode reference fixtures informed a future experimental interchange design; they do not establish arbitrary `.pro` compatibility. Raw evidence stays in `.local/research`.

Larger models handled architecture/integration/review; smaller workers implemented bounded modules. The final UI pass and completion checks were performed by the orchestrator after an implementation worker reached its usage limit. Source contains no proprietary executable, decompiled implementation or copied reference-app creative assets. Geist is embedded under its included SIL Open Font License; application source is MIT licensed.
