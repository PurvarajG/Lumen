# ProPresenter 21.4 native follow-up

Scope: read-only static inspection of the installed ProPresenter 21.4 bundle and its bundled `ssAPI` framework. No app launch, UI interaction, user-data access, or target modification occurred. This pass does not recover proprietary implementation source.

## Native rendering and output evidence

REA/Ghidra 12.1.4 analyzed the arm64 slice of `ssAPI.framework/Versions/A/ssAPI` (the original is universal; Ghidra rejects a universal Mach-O when asked to enforce an architecture, so the analysis used a local arm64-thinned copy). Its overview reports 42 segments, 19,090 procedures, and 61,591 strings. A bounded static string-name search returned 4,147 matches for `timer|macro|playlist|slide|render|output|screen|countdown|syphon`; this is a broad symbol/string search, not proof of call paths or runtime use.

The retained search evidence includes symbols and method-like names for:

- Renderer coordination: `ssRVRender`, `ssTexObjRenderer`, `ssBaseOutput`, `ssBasicRenderer`, `ssInputRenderer`, `ssThumbnailRenderer`, `ssRenderClock`, `ssRenderClockNetworkSync`.
- Output backends: `ssWindowedOutput`, `ssDecklinkOutput`, `ssDVIOutputML`, `ssNDIOutput`, `ssAJAOutput`, `ssRecordOutput`, and `ssSyphonOutput`.
- Screen/output control methods: `ssTexObjRenderer::SetOutput`, `SetOutputsEnd`, `SetScreens`, `SetScreenBackgroundColor`, `SetTargetPreview`; base/window output names include `SetupWindow`, `CreateWindow`, `SetWindowFrame`, `ShowSystemWindow`, and `HideSystemWindow`.
- Rendering resource names include OpenGL/Metal variants and planar render textures; a retained name references `renderDrawable:descriptor:size:format:isUsingHDR:`.

**Inference:** `ssAPI` appears to own or provide a separate rendering/output engine with multiple output backends, screen/preview setup, and both GL/Metal render paths. These are symbol/string-level clues only; the search did not inspect function bodies or establish which backends are wired into a particular ProPresenter workflow. The earlier bundle-level link and resource observations are consistent with this split, but they do not prove runtime behavior.

The parent agent reports that its separate UI observation showed a Screens menu with independent Audience and Stage enablement, Audience Looks, Stage Layouts, Configure Screens, and Capture Settings / Start Capture. It also reports a Macro panel with searchable collection, grid/table modes, and Add Macro, plus timer configuration for countdown or countdown-to-time and optional Allows Overrun. This paragraph is parent-reported UI evidence, not something observed or verified in this static pass.

## Document interchange clues

- The app's `Info.plist` registers legacy `.pro6`, `.pro6pl`, and `.pro6Template` formats as XML-conforming imported types. This supports treating at least those legacy interchange formats as XML-compatible; it does not describe their schema or establish that current `.pro` documents use the same representation.
- The bundle inventory contains a Spotlight importer `schema.xml` for `com.renewedvision.pro.presentation`; its only custom field is a `CFDate` named `com_renewedvision_pro_lastPlayed`, alongside generic Spotlight metadata such as title and text content. This is an indexing schema, not a presentation serialization schema.
- The full bundle inventory had no standalone `.proto`, `.xsd`, `.schema`, or `.sql` files. It did include protobuf-related libraries/resources (`libprotobuf.31.1.0.dylib`, SwiftProtobuf-related framework resources), but their presence is not evidence that documents, playlists, or app state are stored as protobuf. No serialized current documents or app databases were examined.

## Evidence and cost

- `.local/research/ssapi-overview.json` — REA `analyze` result, Evidence `ev_6837b6060eece2a8fc9a450ce78c36d9a2e72910698e95aaccad4c7f6dda0869`; target digest `083790e5d10a10be6e93a3c0b7e0a7833ee344e6fa9965e1f9321e52b7709fd2`.
- `.local/research/ssapi-search-strings.json` — REA `search` result, Evidence `ev_023166c0dea46e93dee86c767c2783565b6f9fa70743b813f24746b7a2074d04`, with all 4,147 matching records retained.
- `.local/research/ssapi-snapshot.json` — REA snapshot retaining the overview and focused search results. The second, previously uncached search still launched a fresh Ghidra import; observed elapsed time was about 3m20s. A repeat of an exact query can use the snapshot cache, but a new query should be assumed to need provider startup/import again. The JVM was launched with `-Xmx2G`; this limits Java heap, not total process-family memory.
- `.local/research/ssAPI-arm64` — temporary locally thinned analysis input, derived from the installed framework. Keep local; do not commit binary artifacts.
- Spotlight schema source: `/Applications/ProPresenter.app/Contents/Library/Spotlight/ProSpotlightImporter.mdimporter/Contents/Resources/schema.xml`, SHA-256 `cb8b6c7ce6ba8e5581242a9d280f9fda62a9edf0d4714dbe2baeb9798d1608ec`.

## Limits and next steps

This pass analyzed `ssAPI`, not ProPresenter's main executable or ProCore, and the new search resolved strings/symbol names without decompiling selected bodies. It did not statically establish the main app's control calls into the renderer, current file serialization, timer/macro data models, or behavior under different screen configurations. The most efficient next native step is a single analysis of the main executable or ProCore followed by carefully selected exact procedure inspection from its retained snapshot; avoid exploratory new searches unless they answer a concrete implementation question. For interchange, use authorized sample documents or published format descriptions and inspect their actual structure separately.
