# B3: lumen-scripture crate

Create `crates/lumen-scripture/**` only (Cargo.lock may update). No dependency on other Lumen crates. Read `AGENTS.md`, follow `~/.claude/skills/ponytail/SKILL.md`. Write tests first. Allowed deps: `roxmltree` (no DTD), `thiserror`, workspace `serde`. No network; users supply their own Bible files (Lumen ships no translation text). Test fixtures: write tiny synthetic texts yourself (e.g. "In the beginning test verse one"), never copy a real translation.

## API

- `Bible { name: String, books: Vec<Book> }`, `Book { name: String, osis_id: String, chapters: Vec<Vec<String>> }` (chapters[c-1][v-1] = verse text).
- `load_bible(path: &Path) -> Result<Bible, ScriptureError>` by extension: `.xml`/`.osis` → OSIS (`osis/osisText/div[type=book]/chapter/verse`, both milestone `sID/eID` and container verse forms; note/title/footnote elements excluded from text), `.txt` → one verse per line `Book C:V text` (book names may contain digits/spaces, e.g. `1 John 3:16 ...`). Read cap 32 MiB (bounded read), ≤ 200 books, ≤ 200 chapters/book, ≤ 200 verses/chapter. Malformed input → error naming line/element.
- Canonical 66-book table with OSIS ids and common English names/abbreviations (Gen, Ex/Exod, Ps/Psa/Psalms, Song/SoS, 1 Jn/1John, Jn, Rev…), used to resolve references; books not in the table still load by their own names.
- `parse_reference(&str, &Bible) -> Result<Passage, ScriptureError>`: `John 3:16`, `jn 3:16-18`, `John 3:16-4:2`, `Psalm 23`, `1 Cor 13:4-7`, case-insensitive, extra spaces tolerated, en-dash accepted. Out-of-range verse/chapter is an error, not a clamp.
- `Passage { book: String, start: (u16,u16), end: (u16,u16) }`, `passage_verses(&Bible, &Passage) -> Vec<(u16, u16, &str)>`.
- `passage_slides(&Bible, &Passage, opts: SlideOptions { verses_per_slide: usize (1..=10), show_verse_numbers: bool, max_chars: usize }) -> Vec<(String /*label e.g. "John 3:16-17"*/, String /*text*/)>` splitting long verses at word boundaries to respect `max_chars`.
- `search(&Bible, query: &str, limit: usize) -> Vec<(Passage, String)>` case-insensitive substring over verses, stops at limit.

## Acceptance

- Tests for every reference form and error, OSIS milestone + container forms with notes excluded, txt loader incl. numbered books, bounds/size errors, slide splitting and labels, search limit.
- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.
- Commit on your branch. Report to `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/b3-scripture.md`. Final reply: 10 lines max.
