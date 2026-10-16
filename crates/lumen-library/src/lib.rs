pub mod downloader;
pub mod error;
pub mod model;
pub mod parse;
pub mod search;
pub mod slides;

pub use downloader::{
    DownloadEvent, DownloadHandle, Downloader, HttpFetcher, UreqFetcher, delete_book,
    installed_books, load_book,
};
pub use error::LibraryError;
pub use model::{
    Block, BlockKind, Book, BookInfo, BookSummary, CATALOG, SCHEMA_VERSION, Unit, UnitRef, catalog,
    to_nfc,
};
pub use parse::{
    merge_vachanamrut_units, parse_aksharamrutam, parse_asv, parse_bhagvat, parse_chintamani,
    parse_diksha, parse_gita, parse_harililamrut, parse_karika, parse_kavya, parse_kirtan,
    parse_pbp, parse_shloks, parse_vachanamrut, parse_vachanamrut_page, parse_vato,
    parse_yogi_gita,
};
pub use search::{Hit, SearchIndex, search};
pub use slides::{Lang, SlideOpts, unit_slides};
