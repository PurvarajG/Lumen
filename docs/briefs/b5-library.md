# B5: lumen-library — anirdesh.com book downloader, parser and search

User decision: Lumen downloads books from https://www.anirdesh.com on the user's Mac for personal use, caches them locally, keeps Gujarati + English, and NEVER bundles or commits book text (© Anirdesh / BAPS). Read `docs/research/anirdesh-structure.md` (binding blueprint: URL templates, selectors, unit counts), `AGENTS.md`, and follow `~/.claude/skills/ponytail/SKILL.md`. Write tests first.

Own `crates/lumen-library/**` only (Cargo.lock may update). Depend on `lumen-core` (for `markup::escape_markup` only), `serde`/`serde_json`/`thiserror` (workspace), `scraper` (HTML), `ureq` (blocking HTTP, rustls; no tokio), `unicode-normalization`. No async runtime.

## Model (serde, versioned cache format)

```rust
pub struct BookInfo { pub id: &'static str, pub title_gu: &'static str, pub title_en: &'static str, pub units: usize, pub pages: usize }
pub fn catalog() -> &'static [BookInfo];        // every book in the blueprint table EXCEPT `ebooks` (binary EPUB) and the special charitra/adhiveshan sections
pub struct Book { pub id: String, pub title_gu: String, pub title_en: String, pub units: Vec<Unit>, pub schema: u32 }
pub struct Unit { pub key: String /* e.g. "gadhada-1-1" */, pub label: String /* "Gadhada I-1" */, pub title_gu: Option<String>, pub title_en: Option<String>, pub blocks: Vec<Block> }
pub struct Block { pub gu: Option<String>, pub en: Option<String>, pub translit: Option<String>, pub notes: Option<String> /* footnotes */ , pub kind: BlockKind }
pub enum BlockKind { Paragraph, Verse, Stanza, Refrain, Shlok, Heading }
```

Kirtan stanzas keep line breaks (`\n`) and refrain marks; footnotes (`sup`, `span.target`) go to `notes`, not text. Exclude audio/player/toolbar/sidebar markup per blueprint §4. NFC-normalize all text.

## Parsing

One parser function per book: `fn parse_<book>(html: &str, unit: &UnitRef) -> Result<Vec<Block>/Unit, LibraryError>`, sharing helpers. Where the blueprint says both languages are on one page, fetch once. Vachanamrut: gu and en pages separately, merge paragraph-by-paragraph by position; if counts differ, keep both sequences without guessing alignment (gu-only blocks then en-only blocks) and record a warning. Missing selectors → `LibraryError::Layout { book, unit, detail }` (site changed), never a silent empty unit. Pure functions, no network.

Tests: synthetic minimal HTML fixtures you write that mirror each book's selectors (no real scripture text — use placeholder words like "ગુજરાતી પાઠ એક" / "sample text one"), including footnotes, audio blocks to exclude, refrains, `&emsp;`, missing English, and layout-change errors. Plus ONE `#[ignore]` test that parses every saved real sample under `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/research/anirdesh/` and asserts non-empty blocks (never copy those files into git).

## Downloader

`Downloader::start(book_id, cache_dir: &Path, events: Sender<DownloadEvent>) -> DownloadHandle` on its own thread. `User-Agent: LumenPresenter/0.1`, ≥ 1.1 s between requests, 20 s timeout, max 2 MiB per response (bounded read), 3 retries with backoff on 5xx/timeouts, stop immediately on 403/404 for the book with a clear error. Resumable: each fetched unit is written to `cache_dir/<book>/units/<key>.json` atomically; restarting skips units already cached. Events: `Progress { done, total }`, `Warning(String)`, `Finished(Book summary)`, `Failed(String)`. `DownloadHandle::cancel()` stops within one request and joins; Drop cancels. When complete, write `cache_dir/<book>/book.json` (atomic). `load_book(cache_dir, id) -> Result<Book>` and `installed_books(cache_dir) -> Vec<(id, units, bytes)>`; `delete_book`. Inject the HTTP fetch behind a small trait so tests run against a fake server/in-memory map — no network in tests.

## Search and slides

- `search(books: &[Book], query: &str, limit: usize) -> Vec<Hit { book, unit_key, block_index, snippet }>`: case-insensitive for English, NFC for Gujarati, matches unit labels ("gadhada i-1", "ga 1-1"), titles and text; label matches rank first, then title, then text. Must handle the full Kirtan book (2,491 units) in < 50 ms for a typical query in `--release` (build a lowercase index once in a `SearchIndex::new(&[Book])`).
- `unit_slides(unit: &Unit, opts: SlideOpts { lang: Lang /* Gu, En, Both */, blocks_per_slide: 1..=4, max_chars: usize }) -> Vec<(String /*label*/, String /*text, markup-escaped*/)>`: Both = Gujarati then English separated by a blank line; long blocks split at sentence/line boundaries to fit `max_chars`; stanzas never split mid-line.

## Acceptance

- `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.
- Manual check (do it once, ≤ 15 requests, 1.1 s apart): download 3 Vachanamrut units and 3 kirtans into `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/library-test/` with a tiny example binary, print block counts. Delete nothing else.
- Commit on your branch. Report `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/b5-library.md`: API, per-book parser status, search timing, network behaviour, limitations. Final reply: 10 lines max.
