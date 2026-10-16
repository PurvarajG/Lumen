# Interchange implementation contract

This task follows the native app foundation. Own crates/lumen-interchange/** only; parent coordinates GUI integration and workspace/git. Read AGENTS.md, docs/superpowers/specs/2026-10-16-interchange-design.md and docs/research/propresenter-owned-fixture.md. Use lumen-core, uuid, thiserror, roxmltree and rtf-parser=0.4.3 with default-features=false (avoid JS bindings). Cargo is in /opt/homebrew/opt/rustup/bin. This is independent Rust parsing of user-selected files, with no subprocess or network access.

## API

Re-export from lib.rs:

- `ImportReport { pub document: lumen_core::Document, pub warnings: Vec<String> }` Clone/Debug.
- `InterchangeError` std::error::Error/Display with actionable IO, unsupported, malformed and limit reasons.
- `import_file(path:&Path,theme_id:Uuid)->Result<ImportReport,InterchangeError>` dispatch by extension (txt, xml/openlyrics, pro). Bound actual read to16MiB even if metadata changes. Empty/unrecognized extensions error. Text routes core document_from_text; filename stem is default name.
- `import_openlyrics(xml:&str,theme_id:Uuid)->Result<ImportReport,InterchangeError>` namespace-aware OpenLyrics0.8/0.9 song subset. Require song/title/lyrics; preserve actual text and br linebreaks. One verse becomes one Slide. Honor verseOrder including repeated names by cloning to new slide IDs per occurrence. Without order use declaration order. Unknown referenced verse or duplicate ambiguous names is an error. Exclude chord/comment markup from lyric text, retain ordinary inline text. Use first title. Unsupported rich metadata/language details add warnings if actually present. DTD/external entity processing disabled; no fetching. Reject inputs over cap.
- `export_openlyrics(document:&Document)->Result<String,InterchangeError>` original/user text, title from document.name, unique v1/v2 names, verseOrder declaration sequence, XML-escaped title/text, newline as br, version0.9 and standard namespace. Empty document/name yields clear error. This is text export only, not layout/notes/media serialization.
- `import_propresenter(bytes:&[u8],fallback_name:&str,theme_id:Uuid)->Result<ImportReport,InterchangeError>` experimental observed21.4 text path as below. Always return a warning about text-only compatibility and loss of media/layout/actions. Regenerate our UUIDs; do not preserve claimed vendor ID roles. Preserve every observed top-level field13 cue position; unsupported text structure can be a blank cue plus warning when at least one recognized text cue exists. If no cue has recognized RTF structure, return unsupported error, not a successful empty show.

## ProPresenter observed subset

Known authored fixture: 2334bytes, SHA256 e175a65e002526c3b7fd34b7c60b33fe9d121e7bd9f1335a9f8096213900a2aa. In ignored .local/research/Lumen-Research-Fixture.pro if present. Top-level field3=title; repeated field13 records in observed authored order. Within each record navigate field10/23/2/1/1/1/13/5 to RTF bytes. Fields on that route may repeat; collect recognized payloads in file order and join text objects with newline. Do not search arbitrary nested data for an RTF prefix or pretend inferred fields are an official schema. Header path/title/cue framing must be structurally valid. A cue's unsupported fields are ignored for text import with the explicit fidelity warning; no claim of arbitrary .pro compatibility.

Implement bounded protobuf wire framing: wire0 varint,1 fixed64,2 length-delimited,5 fixed32; reject unsupported groups3/4, field0, invalid varint10thbyte, lengths/offset overflow, truncation. <=100000 total examined fields, nesting<=32, input<=16MiB, each RTF<=1MiB. Parse only required structural paths, preserve unknown framing safely. No unsafe code. RTF parser uses `RtfDocument::try_from` (Result) rather than parse_rtf convenience (unwrap). Bound braces nesting before parse. Guard against malformed-parser panics if needed; never let user data unwind through UI. Preserve Unicode and escaped literal backslash/braces; remove only RTF's terminal paragraph newline, not internal breaks. No textutil dependency.

## Tests / actual evidence

Write tests first. OpenLyrics: namespaced import, br/inline spans, chord/comment exclusion, repeated/reordered verses, missing verse/error, XML entities escaping roundtrip, malformed/oversized/DTD inputs. Pro: build synthetic wire fixtures from tiny tag/varint helpers with the established nested paths and original RTF text; prove order/blank cue preservation, multiple text objects, Unicode and escaped controls, malformed varints/lengths/groups/depth and unsupported unrelated bytes. Do not commit app binaries or copy arbitrary user data. If actual local owned fixture exists, verify it via a small ignored/manual test or temporary example and record two exact recovered texts; public CI tests must not require .local.

Also verify `.local/research/Lumen-Unicode-Fixture.pro` if present: second cue exactly `Café — naïve\nUnicode: Ω שלום 😀\nEscapes: {brace} \\` (last character is one literal backslash; this prose uses escapes). Same observed nested RTF path. Parent authored and independently recovered it with textutil; research appendix records hash/provenance. Preserve Unicode and literals without relying on textutil in the Rust implementation.

Run fmt/test/clippy for the crate; no source commits/workspace edits. Save report to .local/reports/interchange.md with red/green record, exact tests and local fixture result. Return compact status and limitations. GUI integration happens in a separate parent-coordinated task.
