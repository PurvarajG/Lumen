# Lumen Presenter

Lumen Presenter is an independent, native Rust live presentation application for macOS, designed specifically for sabha and mandir AV operators. It combines a fast, dark-mode operator console with dual-window presentation feeds (Audience and Stage), offline scripture search, and first-class Gujarati and English typographic shaping.

Full ProPresenter parity is a long-term goal; see [the parity tracker](docs/PARITY.md) for verified features, implemented modules, and future capabilities.

---

## Download

**[Download Lumen Presenter 1.0.0-rc.5 for Mac (DMG)](https://github.com/PurvarajG/Lumen/releases/download/v1.0.0-rc.5/Lumen-Presenter-1.0.0-rc.5-macos-arm64.dmg)** · [all versions and release notes](https://github.com/PurvarajG/Lumen/releases)

- Needs a Mac with Apple Silicon (M1 or newer) running macOS 12 or later.
- Video needs FFmpeg: `brew install ffmpeg`.
- Every release page lists the SHA-256 of its DMG so you can check the download.
- No account, no cloud. 12 of the 15 scripture and kirtan books are inside the app, so search works offline from the first launch and nothing is downloaded. The DMG is about 21 MB because of them. (Yogi Gita, Swaminarayan Siddhant Karika and Aksharamrutam are not included yet; see Scripture & Books.)

### Install
1. Open the downloaded `.dmg` and drag **Lumen Presenter** into **Applications**.
2. **First launch:** the app is signed for local use but not yet notarized by Apple, so macOS blocks a plain double-click. Right-click (or Control-click) **Lumen Presenter** in Applications, choose **Open**, then **Open** again. You only do this once. If macOS still refuses, run this in Terminal, then open the app normally:
   ```sh
   xattr -cr "/Applications/Lumen Presenter.app"
   ```
3. Connect the projector or TV, then click **Audience** in the header and pick the display in **Outputs**.

---

## Using Lumen

### Layout
The window is a set of six panels you can move: **Running order**, **Macros**, **Preview**, **Cues**, **Live** and **Tools** (called **Inspector** in Prepare). Drag a panel's tab to another panel's edge to split it or onto its tab bar to stack it, and drag the dividers to resize. Prepare and Present each remember their own arrangement, and **Reset layout** in the header puts the current mode back to the default.

- **Present** (default): Running order on the left; a large **Preview** of the next cue above the **Cues** grid in the middle; the **Live** monitor with the transport buttons above **Tools** (Timers, Messages, Overlays, Audio, Stage) on the right.
- **Prepare:** a big running order over Macros, a small Preview above a large Cues grid, and the slide Inspector (text, style, background, transition, actions) on the right with Live as a second tab.
- **Preview and live are separate.** Selecting or editing a cue only changes the Preview. `Return`, `Space` or **Go live** sends a copy to the audience screen.

### Photos and videos
Photos and videos that are not 16:9 show with bars by default. Right-click a cue (or a running-order row) and pick **Scale**: **Fit (show bars)**, **Fill screen (crop)** or **Stretch**. The Prepare inspector has the same choice. A change takes effect the next time the cue goes live. Video frames are timed to the audience display's own refresh; a 25 fps clip is smoothest on a 75 Hz projector, and 30 fps content is smoothest at 60 Hz. Video is decoded at the audience display's own pixel size (up to 4K) and logos and overlay images are drawn at full size, so nothing is shrunk on the way to the projector or LED wall.

### Moving a show to another Mac
**File → Save with media…** copies every photo, video, sound and deck the show uses into a new folder (`<Name>/<Name>.lumen.json` plus `Media/`), and the show then refers to them by relative paths, so the whole folder can be copied, zipped or reused for a later service. If a show opens with files missing, a banner offers **Locate folder…**, which relinks them by file name. **File → Open recent** lists earlier shows, and Lumen reopens the last one at launch.

### Output windows
- **Audience:** clean fullscreen or windowed output for projectors and TVs. If the chosen display comes back at a different resolution (an LED processor changing its input mode) or after a cable drop, the window returns to it by itself. For an LED wall fed through a processor, set the processor to take a 16:9 input at its native resolution (1920×1080 or 3840×2160) at 60 Hz (50 Hz for 25 fps content), full-range RGB.
- **Stage:** a confidence monitor with the current slide, the next slide, notes, clocks and urgent messages.

---

## Scripture & Books (Offline Operation)

Lumen ships with 12 scripture and kirtan collections (Vachanamrut, Swamini Vato, Satsang Diksha, Kirtan Muktavali, Nishkulanand Kavya, Bhaktachintamani, Harililamrut, Shrimad Bhagavad Gita, Shrimad Bhagavata Purana and others) from anirdesh.com, already indexed. Nothing is downloaded at first launch. Yogi Gita, Swaminarayan Siddhant Karika and Aksharamrutam are in the catalog but their download does not work yet (the site's pages differ from what the importer expects), so they are not included.

> [!NOTE]
> **Copyright:** the book texts are © Anirdesh.com / BAPS and are included with their permission; they are not covered by Lumen's MIT licence (see `Contents/Resources/Licenses/Anirdesh-books.txt` in the app). They are never committed to this repository. The commercial ChitraUni font is not bundled either.

To use the books:
1. Open the **Books** tab in the left sidebar. Included books say **Included**; click **Open book** to browse one.
2. Use `Cmd+F` or the search field to find passages by title, index (e.g. *Gadhada I-1*), or text lyrics. Matching passages can be previewed and converted to presentation slides with a single click.

A development build that is not packaged has no included books; the Books tab then offers **Download** (and **Resume** for a half-finished download) from anirdesh.com, and adds a line to `Books/download.log` in the data folder if a download fails. Maintainers fetch the books for a release with the `fetch_books` step in [docs/RELEASE.md](docs/RELEASE.md).

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
- **Network:** Not required. The books are included; playback, searching and control all work offline.

---

## Developer Guide

### Workspace Structure
- `crates/lumen-core`: Project schema, atomic file persistence, slide models, undo/redo history, clocks, macros, and live output runtime.
- `crates/lumen-text`: High-performance shaped text rasterization (cosmic-text), conjunct and matra shaping, font fallback, and caching.
- `crates/lumen-media`: FFmpeg subprocess video pipeline, pure-Rust audio decoding (rodio), and thumbnail generation.
- `crates/lumen-control`: Remote control listeners (HTTP JSON API, OSC over UDP, MIDI input).
- `crates/lumen-interchange`: OpenLyrics XML import/export and ProPresenter document text extraction.
- `crates/lumen-library`: anirdesh.com book downloader, HTML parser, in-memory search index, and slide generator.
- `crates/lumen-app`: Native macOS operator workbench and presentation windows built with `eframe`/`egui` and `wgpu`; the movable panels use `egui_dock` (MIT, pinned to 0.18.0 for egui 0.33).

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
