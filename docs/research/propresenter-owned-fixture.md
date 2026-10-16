# ProPresenter owned fixture format probe

This is a bounded read-only inspection of the one parent-identified fixture, `Lumen Research Fixture.pro`. The original fixture lives in ProPresenter's user library; a byte-for-byte research copy and all parser outputs are kept under ignored `.local/research/`. No other user data or app state was opened or changed.

## Observed extraction contract for this fixture

The file is 2,334 bytes and has SHA-256 `e175a65e002526c3b7fd34b7c60b33fe9d121e7bd9f1335a9f8096213900a2aa`. REA `inspect-artifact` rejected it as an unsupported binary format (`.local/research/Lumen-Research-Fixture-artifact.json`), so REA did not recognize this document container.

A bounded protobuf-wire decoder consumed the complete file without trailing bytes or invalid tags. This establishes that the bytes are valid protobuf-wire-compatible encoding; without a descriptor, it does not establish a message type or field semantics. It finds:

- Top-level field `3` is UTF-8 title `Lumen Research Fixture` (payload offset 71; 22 bytes).
- Top-level field `13` occurs twice, in file order, at tag offsets 232 and 1268. Each occurrence contains the same nested shape and an RTF payload at the field path `13[occurrence].10.23.2.1.1.1.13.5`.
- RTF payload 1 starts at byte 785 and is 352 bytes; RTF payload 2 starts at byte 1821 and is 368 bytes. Extracting those payloads and converting with macOS `textutil -convert txt -stdout` produced exact text matching the two parent-provided source strings:
  - first field-13 occurrence: `Lumen reference sample\nOriginal line two`
  - second field-13 occurrence: `Second original cue\nCreated for interoperability testing`
- Each field-13 occurrence contains a paired little-endian fixed64 value at nested path `13[occurrence].10.23.2.1.6.{1,2}`: `(1920.0, 1080.0)` in both slide records. **Inference:** this pair is likely a slide/canvas width and height, because it appears consistently in both known slide records. The field names and units remain unconfirmed; treat it as a candidate dimension pair, not a schema guarantee.
- Canonical UUID-shaped strings occur at stable nested fields. The field-13 records have `13[0].1.1 = EA645FD1-88EA-42DD-A42D-1B5AFA77389C` and `13[1].1.1 = 0B65CCAC-6909-4A79-8D3E-BD6EF71CD074`; other UUID-shaped values include the document-level `2 = 3D2CB642-0E79-49AC-8591-DE7D7433311E`, `13[0].10.1.1 = 8DF593CD-50B9-4FBD-ABDC-F8B3E9268097`, and `13[1].10.1.1 = 71825CF1-A4DB-48E7-AF19-B3023133F1E8`. The byte representation supports recognizing these as UUID strings; their specific entity roles are not known.

The exact RTF payload is preserved. A lightweight importer can use the repeated field-13 occurrence order as the order observed in this fixture, navigate to the nested field-5 RTF bytes, and parse RTF into text. It should preserve unknown protobuf fields and the original RTF until cross-fixture evidence supports a broader interpretation. The sample establishes a small extraction path only; it does not establish all document variants, text encodings, slide ordering rules, visibility, rich-text semantics, media references, transitions, or round-trip write behavior.

## Procedure and evidence

1. Copied only the explicitly authorized source to `.local/research/Lumen-Research-Fixture.pro`; source path: `/Users/purvarajsinhgohil/Library/Application Support/RenewedVision/ProPresenter/UserWorkspaces/ProPresenter/Libraries/Default/Lumen Research Fixture.pro`.
2. Ran `file`, `shasum -a 256`, `xxd -g1 -l 256`, and `strings -a -n 3` against the copied fixture. The `strings` view exposed the title, UUID-shaped strings, and embedded RTF text.
3. Ran `rea inspect-artifact .local/research/Lumen-Research-Fixture.pro --format json --full-output`; full result is `.local/research/Lumen-Research-Fixture-artifact.json`. It reports `target_unavailable`, reason `unsupported binary format`.
4. Ran `.local/research/decode_proto.py`, a local bounded wire-format parser, which preserves per-field wire type, byte offsets, lengths, scalar values, UTF-8 values, and candidate nested messages in `.local/research/Lumen-Research-Fixture-wire.json`. It reports complete consumption of 2,334 bytes and the fixture digest above.
5. Extracted the two nested RTF payloads to `.local/research/slide-1.rtf` and `slide-2.rtf`; ran `textutil -convert txt -stdout` to `.local/research/slide-1.txt` and `slide-2.txt`. The converted strings exactly match the parent-provided known texts. The field-path/offset trace is retained in `Lumen-Research-Fixture-wire.json`.

## Limits

The custom decoder uses wire framing and recursive parseability, not a vendor schema. A length-delimited payload that happens to parse as nested wire data can be a false submessage candidate, so field paths are empirical paths through this sample. Parent context establishes that this authored fixture contains two slides and their text/order; static bytes alone do not prove that field 13 is the slide collection in every ProPresenter file. UUID semantics and the dimensions' meanings remain inferred as described above. This single sample must not be used to claim compatibility with arbitrary `.pro` files or to emit modified ProPresenter documents.

## Unicode follow-up fixture

The parent subsequently edited only the second cue of the owned research presentation through the native Reflow UI and saved another local copy as `.local/research/Lumen-Unicode-Fixture.pro` (2479 bytes; SHA-256 `36c9df703718d93f731f3f2c69c0a620e8526556c372020d7134766ef3a1c012`). The original fixture copy above remains unchanged. The new file again parses completely with two top-level field-13 records, and the same nested RTF path extracts the known second-cue text:

```text
Café — naïve
Unicode: Ω שלום 😀
Escapes: {brace} \
```

The extracted `.local/research/unicode-slide-2-1.rtf` converted with macOS `textutil -convert txt -stdout` matches those authored characters, line breaks, braces and backslash. The wire tree and trace are retained in `.local/research/Lumen-Unicode-Fixture-wire.json` and `unicode-wire-trace.txt`. A Rust importer must verify these cases independently; system textutil success alone does not verify its parser. The source presentation in ProPresenter's library now contains this second-cue variant.
