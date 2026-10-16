# ProPresenter parity tracker

Reference baseline: installed macOS ProPresenter 21.4, build 352583705. The vendor website currently advertises newer releases; that is not the binary examined here. The objective is a full standalone alternative. This tracker prevents a working first release from being confused with full parity.

States: **Verified** = exercised through tests or native UI; **Building** = implementation work underway; **Planned** = absent or not verified. Static evidence alone does not upgrade a runtime feature to Verified.

| Capability | Lumen status | Evidence / next acceptance check |
|---|---|---|
| Editable slides and reusable themes | Verified foundation | One text object; theme font/colour/alignment; native edit/live isolation exercised |
| Library and playlist document order | Verified | Core navigation tests and native library/playlist controls |
| Native operator workbench | Verified on Apple Silicon macOS 26.7 | Final dark UI inspected; blank-show creation and cue editing exercised |
| Independent audience/stage windows | Verified windowed foundation | Current/next, notes/message/countdown and close/reopen exercised; fullscreen control implemented, external display mapping unverified |
| Image/video backgrounds | Video verified; image adapter implemented | Generated real video output, EOF/loop/stop/drop tests; separate untriggered-media thumbnails absent |
| Countdown clocks | Verified | Deterministic timer tests and native stage progression |
| Macros and cue actions | Verified foundation | Atomic ordered core tests; native ClearAll media cleanup; no macro editor |
| Text import | Core verified; UI wired | Blank-line/CRLF tests; UTF-8 txt/md picker with 4MiB limit |
| Versioned project save/load | Verified | Roundtrip/replacement/bounds tests; native Open and Save/Quit exercised |
| ProPresenter native document import | Designed, not implemented | Original two-cue/Unicode fixtures reveal limited text paths; arbitrary schemas/fidelity unverified |
| OpenLyrics interchange | Designed, not implemented | Bounded parser/export contract written; no shipped importer |
| Layered slide objects / rich text | Planned | Independent object positioning, font/style spans, bounds tests |
| Transitions | Planned | Measure and inspect actual output fades |
| Undo/redo and crash recovery | Planned | Exercise edit/save/reopen and recovery scenarios |
| Countdown to time / count-up / overrun | Planned | Reference controls observed; implement deterministic clock semantics |
| Multiple output layouts and monitor mapping | Planned | Actual multi-monitor hardware validation |
| NDI, SDI and Syphon output | Planned | Adapter availability, SDK packaging and hardware/receiver verification |
| Video input, recording and streaming | Planned | Real capture/encode pipeline tests and audio sync measurement |
| Audio routing and strict A/V sync | Planned | Loopback/output device tests and drift measurements |
| MIDI / OSC / DMX / timecode | Planned | Protocol fixtures plus device/server validation |
| Remote control / API | Planned | Local authenticated control and cue update tests |
| Scripture / licensed content services | Planned | User-supplied or licensed sources and import verification |
| Cloud/team synchronization | Planned | Conflict/persistence/offline/recovery design and tests |
| Packaged signed releases / other OSs | Local macOS bundle verified | Ad-hoc signed development bundle; optimized release, notarization and other OSs unverified |

Research findings and their limitations are in docs/research. Detailed staged architecture is in docs/superpowers/specs/2026-10-16-lumen-design.md.
