# Lumen Presenter

Lumen Presenter is an independent, native Rust live presentation application for macOS, designed specifically for sabha and mandir AV operators. It combines a fast, dark-mode operator console with dual-window presentation feeds (Audience and Stage), offline scripture search, and first-class Gujarati and English typographic shaping.

Full ProPresenter parity is a long-term goal; see [the parity tracker](docs/PARITY.md) for verified features, implemented modules, and future capabilities.

---

## Installation & First Run

### Installing from DMG
1. Mount the release image: `Lumen-Presenter-1.0.0-rc.1-macos-arm64.dmg` (located in `.local/dist/` or project releases).
2. Drag **Lumen Presenter.app** into `/Applications`.
3. **macOS Gatekeeper note:** The release build is ad-hoc signed for local operation (commercial Apple Developer ID notarization is pending). On first run, right-click (or Control-click) **Lumen Presenter.app** in Finder and choose **Open**, or run:
   ```sh
   xattr -cr "/Applications/Lumen Presenter.app"
   ```

### First Run & Operator Workflow
- **Present Mode (Default):** Streamlined operator layout for live services.
  - **Left Sidebar:** Search across books, library, and playlists; tabs for running order, library cues, and downloaded scripture books.
  - **Center:** Large preview of the selected ("on deck") cue with its saffron highlight, and the 16:9 cue grid with live status indicators.
  - **Right Column:** Audience output monitor (what the congregation sees with a red LIVE indicator), live transport controls, and tabbed utilities (Timers, Messages, Props, Audio, Stage tools).
- **Prepare Mode:** Swaps inspector tools for slide editing, object styling, typography, background media assignment, and transition configuration.
- **Preview vs Live Isolation:** Selecting or editing a cue immediately updates the preview area without affecting live output. Pressing `Return` or clicking **Go live** triggers a cloned snapshot to the audience screen.
- **Output Windows:**
  - **Audience:** Clean fullscreen or windowed output for projectors and TVs.
  - **Stage:** Dedicated confidence monitor display showing current slide, next slide, presenter notes, live clocks, and urgent stage messages.

---

## Scripture & Books (Offline Operation)

Lumen includes built-in download and indexing for scripture and kirtan collections (Vachanamrut, Swamini Vato, Satsang Diksha, Kirtan Muktavali, and 12 other canonical texts) from anirdesh.com.

> [!NOTE]
> **Copyright & Privacy:** Book texts and commercial fonts are never bundled in Lumen binaries. They are downloaded on demand directly to the operator's machine and stored in local application data. Once downloaded, all book search, indexing, and slide generation operate completely offline without internet access.

To download books:
1. Open the **Books** tab in the left sidebar.
2. Click **Download** next to the desired text.
3. Once downloaded, use `Cmd+F` or the search field to find passages by title, index (e.g. *Gadhada I-1*), or text lyrics. Matching passages can be previewed and converted to presentation slides with a single click.

---

## PowerPoint, Keynote and PDF

- **PDF (still pages):** File → Import… a `.pdf`. Each page becomes a slide, rendered from the PDF's vectors at every output's real pixel size, so text and shapes stay sharp on the projector. Export a PowerPoint or Keynote deck as PDF when it has no animations.
- **PowerPoint / Keynote (animated decks):** File → Import… a `.pptx`, `.ppt` or `.key`. Lumen creates a "Play" cue; going live on it opens the deck in PowerPoint or Keynote and starts the real slideshow, so builds, transitions and embedded video play exactly as authored. While it plays, Lumen's Next/Previous (buttons, arrow keys, remote, OSC, MIDI) drive the deck; F1/F2 or Stop deck ends it. macOS asks once for permission to let Lumen control PowerPoint/Keynote. Requires PowerPoint or Keynote installed.

## Keyboard Shortcuts (Present Mode)

Shortcuts are active during live operation and are automatically suppressed while editing text fields.

| Shortcut | Action |
|---|---|
| `Space` / `→` / `Page Down` | Advance to Next cue |
| `←` / `Page Up` | Return to Previous cue |
| `Return` | Go live with selected / on-deck cue |
| `F1` | **Clear All** (blackout + clear slide, media, props, audio, messages) |
| `F2` | **Clear Slide** (hide text only, keep background media) |
| `F3` | **Clear Media** (stop background video/image) |
| `F4` | **Clear Props** |
| `F5` | **Clear Audio** |
| `F6` | **Clear Messages** (dismiss active stage message) |
| `B` | Toggle **Blackout** |
| `Cmd+F` | Focus Search field |
| `Cmd+Z` / `Cmd+Shift+Z` | Undo / Redo |
| `Cmd+S` / `Cmd+O` | Save Project (`.lumen.json`) / Open Project |

---

## Remote Control API

Lumen provides a built-in remote control engine (`lumen-control`) supporting HTTP REST, OSC over UDP, and MIDI Note-On mapping.

### HTTP REST API
- **Default address:** `http://127.0.0.1:47800`
- **Authentication:** `Authorization: Bearer <token>` (token must be at least 16 characters; verified using constant-time comparison).

#### Query Live Status (`GET /status`)
Returns a JSON snapshot containing current project name, live cue index, live text, next slide text, blackout state, and active timer readings:
```sh
curl -H "Authorization: Bearer my-secret-token-12345" \
  http://127.0.0.1:47800/status
```

#### Dispatch Remote Commands (`POST /command`)
Accepts JSON commands up to 4 KiB:
```sh
# Advance to next cue
curl -X POST -H "Authorization: Bearer my-secret-token-12345" \
  -H "Content-Type: application/json" \
  -d '{"cmd":"Next"}' \
  http://127.0.0.1:47800/command

# Go back to previous cue
curl -X POST -H "Authorization: Bearer my-secret-token-12345" \
  -H "Content-Type: application/json" \
  -d '{"cmd":"Previous"}' \
  http://127.0.0.1:47800/command

# Trigger specific cue index
curl -X POST -H "Authorization: Bearer my-secret-token-12345" \
  -H "Content-Type: application/json" \
  -d '{"cmd":"Trigger","index":2}' \
  http://127.0.0.1:47800/command

# Toggle blackout
curl -X POST -H "Authorization: Bearer my-secret-token-12345" \
  -H "Content-Type: application/json" \
  -d '{"cmd":"Blackout","on":true}' \
  http://127.0.0.1:47800/command

# Clear All
curl -X POST -H "Authorization: Bearer my-secret-token-12345" \
  -H "Content-Type: application/json" \
  -d '{"cmd":"ClearAll"}' \
  http://127.0.0.1:47800/command

# Send a stage confidence message
curl -X POST -H "Authorization: Bearer my-secret-token-12345" \
  -H "Content-Type: application/json" \
  -d '{"cmd":"StageMessage","text":"5 minutes until sabha concludes"}' \
  http://127.0.0.1:47800/command

# Start, pause, or reset a countdown clock
curl -X POST -H "Authorization: Bearer my-secret-token-12345" \
  -H "Content-Type: application/json" \
  -d '{"cmd":"TimerStart","name":"Main Timer"}' \
  http://127.0.0.1:47800/command
```

### OSC (Open Sound Control)
Lumen listens for UDP OSC packets (single messages and bundles) on its configured port:
- `/lumen/next` — Advance to next cue
- `/lumen/previous` — Return to previous cue
- `/lumen/trigger <int>` — Trigger cue at index
- `/lumen/blackout <bool|int>` — Set blackout on (1) or off (0)
- `/lumen/clear/text` — Clear slide text
- `/lumen/clear/all` — Clear all layers
- `/lumen/macro <string>` — Run macro by name
- `/lumen/timer/start <string>` — Start named timer
- `/lumen/timer/pause <string>` — Pause named timer
- `/lumen/timer/reset <string>` — Reset named timer
- `/lumen/message <string>` — Display stage message (up to 500 characters)

### MIDI Mapping
Lumen accepts MIDI Note-On events (`0x90..=0x9F`, `velocity > 0`) on any configured MIDI input port. Mappings pair MIDI channels (0–15) and note numbers (0–127) to commands (e.g. Note 60 on Channel 0 → `Next`, Note 62 → `ClearAll`).

---

## Runtime Requirements

- **Operating System:** macOS 12.0 or newer on Apple Silicon (arm64).
- **FFmpeg & ffprobe:** Required for background video decoding and media thumbnail generation. Install via Homebrew:
  ```sh
  brew install ffmpeg
  ```
- **Gujarati Typography:**
  - Bundled fallback: **Noto Sans Gujarati** (SIL Open Font License) is embedded and always active.
  - High-fidelity optional font: **ChitraUni** (commercial font, © Summit Information Technologies). If installed in Font Book (`~/Library/Fonts` or `/Library/Fonts`), Lumen automatically detects and shapes text with ChitraUni.
- **Network:** Only required during on-demand book downloads from anirdesh.com. All subsequent playback, searching, and control work offline.

---

## Developer Guide

### Workspace Structure
- `crates/lumen-core`: Project schema, atomic file persistence, slide models, undo/redo history, clocks, macros, and live output runtime.
- `crates/lumen-text`: High-performance shaped text rasterization (cosmic-text), conjunct and matra shaping, font fallback, and caching.
- `crates/lumen-media`: FFmpeg subprocess video pipeline, pure-Rust audio decoding (rodio), and thumbnail generation.
- `crates/lumen-control`: Remote control listeners (HTTP JSON API, OSC over UDP, MIDI input).
- `crates/lumen-interchange`: OpenLyrics XML import/export and ProPresenter document text extraction.
- `crates/lumen-library`: anirdesh.com book downloader, HTML parser, in-memory search index, and slide generator.
- `crates/lumen-app`: Native macOS operator workbench and presentation windows built with `eframe`/`egui` and `wgpu`.

### Building and Verification
```sh
# Run the local application
scripts/run.sh

# Run formatting, workspace tests, and Clippy
scripts/verify.sh

# Build optimized release bundle (.app) and disk image (.dmg)
scripts/package-macos.sh --release
```

All release artifacts are generated into `.local/dist/`.
