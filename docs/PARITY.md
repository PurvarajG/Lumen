# ProPresenter parity tracker

Reference baseline: installed macOS ProPresenter 21.4, build 352583705. The vendor website currently advertises newer releases; that is not the binary examined here. The objective is a full standalone alternative. This tracker prevents a working first release from being confused with full parity.

States: **Verified** = exercised through tests or native UI; **Building** = implementation work underway; **Planned** = absent or not verified. Static evidence alone does not upgrade a runtime feature to Verified.

| Capability | Lumen status | Evidence / next acceptance check |
|---|---|---|
| Editable slides and reusable themes | Core verified; UI building | Core models validated; edit/live isolation test passes |
| Library and playlist document order | Core verified; UI building | Cross-document cue navigation and boundary tests pass |
| Native operator workbench | Planned | Launch and exercise editing/cueing on macOS |
| Independent audience/stage windows | Planned | Visually compare output with preview; close/reopen independently |
| Image/video backgrounds | Building | Generated media decoding and owned-child lifecycle tests |
| Countdown clocks | Core verified; UI building | Pause/resume/reset/expiry tests pass |
| Macros and cue actions | Core verified; UI building | Bounded ordered atomic action tests pass |
| Text import | Core verified; UI building | Blank-line/CRLF import tests pass |
| Versioned project save/load | Core verified; UI building | Roundtrip, replacement, malformed/oversized file tests pass |
| ProPresenter native document import | Planned | Registered types observed; actual schemas not established |
| OpenLyrics interchange | Planned | Parse/export real fixture; preserve ordering/line breaks |
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
| Packaged signed releases / other OSs | Planned | Packaged app launch, notarization and platform CI |

Research findings and their limitations are in docs/research. Detailed staged architecture is in docs/superpowers/specs/2026-10-16-lumen-design.md.
