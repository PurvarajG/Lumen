# ProPresenter 21.4 static reconnaissance

Scope: read-only static inspection of `/Applications/ProPresenter.app`; the application was not launched and its user-data directories were not read. This is evidence about one installed macOS build, not an implementation specification.

## Findings

1. **Native macOS application, dual architecture.** The bundle identifies `com.renewedvision.propresenter`, version 21.4 / build 352583705, minimum macOS 13.5, build commit `fb0718cad346a9d1221cebe5759c81e925fd3fb3`. REA identified the main executable as a universal Mach-O with x86_64 and arm64 slices (selected arm64); `otool -L` shows AppKit, SwiftUI, AVFoundation, VideoToolbox, Metal/MetalKit, CoreMIDI, WebKit, SQLite, ProCore, ssAPI, and Syphon dependencies. The bundle contains five top-level private frameworks: LetsMove, ProCore, Sparkle, Syphon, and ssAPI. **Inference:** rendering and output paths likely rely on Apple media/graphics APIs plus private ProCore/ssAPI and Syphon integration; static linkage does not prove which path a user action takes.

2. **Several generations of presentation and playlist formats are registered.** `Info.plist` exports current ProPresenter presentation, presentation bundle, playlist bundle, theme bundle, playlist, and theme UTIs. It imports legacy ProPresenter 4/5/6 document, playlist, template, and bundle UTIs; the legacy `.pro6`, `.pro6pl`, and `.pro6Template` declarations conform to XML (`public.xml` / `text/xml`). Registered URL schemes are `pro` and `propresenter`. This establishes OS-level file associations and declared conformance only; it does not establish the current on-disk schema or whether every registered format is writable.

3. **Presentation, playlist, slide, timer, and macro concepts are visible in packaged UI resources.** The resource inventory includes `PlaylistOutlineViewController.nib`, `PlaylistDetailViewController.nib`, `PresentationOutlineViewController.nib`, `ProPresentationHeaderViewController.nib`, slide-note/tab nibs, `RVTimerController.nib`, `MessageFormTimerViewController.nib`, `CustomIcon_Macro.svg`, and `CustomIcon_Timer.svg`. Packaged `Themes/*/Theme` entries and an ssAPI GLSL resource are present. These names support feature-area leads for later static code analysis, but do not establish runtime behavior or data models.

4. **The app is a substantial native package, not an Electron app.** REA's full artifact inventory reports 1,560 content nodes, 1,681 occurrences, and 1,662 containment edges; formats include 42 universal Mach-O files, 69 plists, and 7 JavaScript bundles. No single JavaScript application graph was attempted. The bundle includes native UI resources and private frameworks, alongside embedded native libraries. The executable links SQLite and protobuf-related bundled libraries; this alone does not establish that playlists or presentations are stored in SQLite or protobuf.

## Evidence and provenance

Full REA JSON is retained locally (not source):

- `.local/research/propresenter-artifact.json` — `inspect-artifact /Applications/ProPresenter.app`; root SHA-256 `f1095e6c26416016a3f29228ccc5dcd530c04337684959115ebc617859528a36`; artifact inventory Evidence `ev_7f6ec25b368ed8f8968aee60528def4245e439e4b6e71a6fe04e6fd6e410aae3` (nested `inventory_artifact` Evidence `ev_8be32215ce3e212ccf143628963578e9020eda435f278655fc9a0442a14343c5`).
- `.local/research/propresenter-macho.json` — REA `inspect-macho` on the main executable; Evidence `ev_708cd41c06844780705447d49294f9930df60e93077ad3e84e948ce9e99e7c41`; executable SHA-256 `3d9829bc522bcd1e807174c8a803029bfa09cb222a7e64a692c266113a11f0e8`.
- `.local/research/propresenter-plist.json` — REA `inspect-plist` on the bundle; Evidence `ev_d57eb007f2d5e62d796f51e1d81eb4560b9ceaf91a949a1f7c97c7e1bd71361f`.
- Supplemental read-only commands: `/usr/libexec/PlistBuddy -c Print .../Contents/Info.plist`, `file`, `otool -L`, and `find` over `Contents/Resources` and `Contents/Frameworks`.

## Limits / next useful questions

This pass did not decompile code, inspect app user data, infer undocumented file schemas, or execute UI flows. Current save formats, playlist nesting/order/state semantics, slide object schema, rendering lifecycle, output routing, timer state machine, and macro action model remain unknown. Next static pass should inspect relevant native symbols/strings and targeted functions once Ghidra is available; format claims should be corroborated using user-owned sample documents or explicit documentation, not guessed from registered UTIs.
