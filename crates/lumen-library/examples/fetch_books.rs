//! Downloads every catalog book into `<dir>`, one after another, skipping books that already
//! have a `book.json`. Rate-limited by the downloader (1.1 s per request); interrupted runs resume
//! from the cached units. Usage: `fetch_books <dir>`.
use lumen_library::downloader::{DownloadEvent, Downloader};
use lumen_library::model::catalog;
use std::path::PathBuf;
use std::sync::mpsc::channel;

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("usage: fetch_books <dir>"));
    let mut failed = Vec::new();
    for book in catalog() {
        if dir.join(book.id).join("book.json").exists() {
            println!("{}: already present", book.id);
            continue;
        }
        println!("{}: downloading ({} units)", book.id, book.units);
        let (tx, rx) = channel();
        let _handle = Downloader::start(book.id, &dir, tx);
        let mut last_percent = 101;
        for event in rx {
            match event {
                DownloadEvent::Progress { done, total } => {
                    let percent = done * 100 / total.max(1);
                    if percent != last_percent && percent % 5 == 0 {
                        last_percent = percent;
                        println!("{}: {done}/{total}", book.id);
                    }
                }
                DownloadEvent::Warning(w) => println!("{}: warning: {w}", book.id),
                DownloadEvent::Finished(s) => {
                    println!(
                        "{}: finished, {} units, {} bytes",
                        book.id, s.units, s.bytes
                    );
                    break;
                }
                DownloadEvent::Failed(err) => {
                    println!("{}: FAILED: {err}", book.id);
                    failed.push((book.id, err));
                    break;
                }
            }
        }
    }
    if failed.is_empty() {
        println!("all books present");
    } else {
        println!("{} book(s) failed:", failed.len());
        for (id, err) in &failed {
            println!("  {id}: {err}");
        }
        std::process::exit(1);
    }
}
