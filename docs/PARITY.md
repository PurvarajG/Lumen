# ProPresenter parity tracker

Reference baseline: installed macOS ProPresenter 21.4, build 352583705. The vendor website currently advertises newer releases; that is not the binary examined here. The objective is a full standalone alternative. This tracker prevents a working first release from being confused with full parity.

States: **Verified** = exercised through automated tests or native UI checks; **Implemented** = module and interfaces complete; **Planned** = absent or awaiting validation. Static evidence alone does not upgrade a runtime feature to Verified.

| Capability | Lumen status | Evidence / next acceptance check |
|---|---|---|
| Editable slides and reusable themes | Verified foundation | One text object; theme font/colour/alignment; native edit/live isolation exercised |
| Library and playlist document order | Verified | Core navigation tests and native library/playlist controls |
| Native operator workbench | Verified on Apple Silicon macOS 26.7 | Final dark UI inspected; blank-show creation, preview/live isolation, and cue editing exercised |
| Independent audience/stage windows | Verified windowed foundation | Current/next, notes/message/countdown and close/reopen exercised; fullscreen control implemented, external display mapping unverified |
| Image/video backgrounds | Video verified; image adapter implemented | Generated real video output, EOF/loop/stop/drop tests; thumbnail caching verified |
| Countdown clocks | Verified | Deterministic timer tests (countdown, count-up, overrun) and native stage progression |
| Macros and cue actions | Verified foundation | Atomic ordered core tests; native ClearAll media cleanup; no visual macro editor |
| Text import | Core verified; UI wired | Blank-line/CRLF tests; UTF-8 txt/md picker with 4MiB limit |
| Versioned project save/load | Verified | Roundtrip/replacement/bounds tests; native Open and Save/Quit exercised |
| ProPresenter native document import | Implemented text extraction | `lumen-interchange` protobuf field parser extracts cue order, visible text, Unicode, and RTF formatting; tested on real reference fixtures |
| OpenLyrics interchange | Verified | `lumen-interchange` OpenLyrics XML import & export with namespace, verse order, line break roundtripping, and malformed document handling tests |
| Layered slide objects / rich text | Implemented | `SlideObject` model, inline rich-text markup spans (bold, italic, underline, color), and `cosmic-text` decoration rendering |
| Transitions | Implemented | Cut, Dissolve, FadeBlack, and Push transitions modeled with duration validation and live snapshot state |
| Undo/redo and crash recovery | Implemented | `lumen-core` Command history stack with undo/redo and `lumen-app` autosave snapshot / recovery guard |
| Countdown to time / count-up / overrun | Implemented | `TimerKind` supports Countdown, CountdownToTime, and Elapsed; overrun tracking and warning thresholds validated in core tests |
| Multiple output layouts and monitor mapping | Implemented UI & display enumeration | Display discovery, display assignment, and stage layouts implemented in app; physical multi-monitor hardware validation pending |
| NDI, SDI and Syphon output | Planned | Adapter availability, SDK packaging, and hardware receiver verification |
| Video input, recording and streaming | Planned | Real capture/encode pipeline tests and audio sync measurement |
| Audio routing and strict A/V sync | Implemented audio bin & playback | Pure-Rust `rodio` audio playback with volume fading, sine wave completion test, and device enumeration; sample-accurate multi-channel matrix unverified |
| MIDI / OSC / DMX / timecode | Implemented MIDI & OSC | `lumen-control` OSC UDP listener (`/lumen/next`, etc.) and MIDI Note-On mapper; DMX and SMPTE timecode planned |
| Remote control / API | Verified | `lumen-control` HTTP REST API with constant-time bearer token auth, `/status` JSON snapshot, and `/command` dispatcher tested |
| anirdesh Books download and search | Verified | `lumen-library` downloader (with 403 backoff and cancellation tests), HTML parsers for 16 canonical books, ranked in-memory search index, and slide generation tests |
| Gujarati shaping and font fallback | Verified | `lumen-text` shaping tests for conjuncts and matras, bundled Noto Sans Gujarati OFL fallback, and runtime ChitraUni font detection |
| Scripture / licensed content services | Planned | User-supplied or licensed sources and import verification; offline anirdesh scriptures implemented independently |
| Cloud/team synchronization | Planned | Conflict/persistence/offline/recovery design and tests |
| Packaged signed releases / other OSs | Verified macOS release DMG | `scripts/package-macos.sh` builds optimized 1.0.0-rc.1 release bundle, original AppIcon.icns, licenses, and compressed UDZO DMG verified with hdiutil and codesign; notarization and Windows/Linux unverified |

Research findings and their limitations are in `docs/research`. Detailed staged architecture is in `docs/superpowers/specs/2026-10-16-lumen-design.md`.
