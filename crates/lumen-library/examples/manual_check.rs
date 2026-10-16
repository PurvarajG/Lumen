use lumen_library::downloader::*;
use lumen_library::model::BookSummary;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

fn main() {
    let target_dir =
        Path::new("/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/library-test");
    std::fs::create_dir_all(target_dir).expect("create library-test dir");

    println!("Starting manual check: downloading 3 Vachanamrut units and 3 Kirtans...");

    // 1. Download 3 Vachanamrut units
    println!("\n--- Downloading Vachanamrut (3 units) ---");
    let (tx1, rx1) = channel();
    let mut handle1 = Downloader::start_with_limit(
        "vachanamrut",
        target_dir,
        tx1,
        std::sync::Arc::new(UreqFetcher::new()),
        Some(3),
    );

    while let Ok(event) = rx1.recv_timeout(Duration::from_secs(60)) {
        match event {
            DownloadEvent::Progress { done, total } => {
                println!("Vachanamrut progress: {done}/{total}");
            }
            DownloadEvent::Warning(w) => {
                println!("Vachanamrut warning: {w}");
            }
            DownloadEvent::Finished(BookSummary {
                id, units, bytes, ..
            }) => {
                println!("Vachanamrut finished: id={id}, units={units}, bytes={bytes}");
                break;
            }
            DownloadEvent::Failed(err) => {
                eprintln!("Vachanamrut download failed: {err}");
                break;
            }
        }
    }
    handle1.cancel();

    // 2. Download 3 Kirtan units
    println!("\n--- Downloading Kirtan (3 units) ---");
    let (tx2, rx2) = channel();
    let mut handle2 = Downloader::start_with_limit(
        "kirtan",
        target_dir,
        tx2,
        std::sync::Arc::new(UreqFetcher::new()),
        Some(3),
    );

    while let Ok(event) = rx2.recv_timeout(Duration::from_secs(60)) {
        match event {
            DownloadEvent::Progress { done, total } => {
                println!("Kirtan progress: {done}/{total}");
            }
            DownloadEvent::Warning(w) => {
                println!("Kirtan warning: {w}");
            }
            DownloadEvent::Finished(BookSummary {
                id, units, bytes, ..
            }) => {
                println!("Kirtan finished: id={id}, units={units}, bytes={bytes}");
                break;
            }
            DownloadEvent::Failed(err) => {
                eprintln!("Kirtan download failed: {err}");
                break;
            }
        }
    }
    handle2.cancel();

    // Load and print block counts for each unit
    println!("\n================ Block Counts ================");
    if let Ok(vach_book) = load_book(target_dir, "vachanamrut") {
        println!("Book: {} (id: {})", vach_book.title_en, vach_book.id);
        for unit in &vach_book.units {
            println!(
                "  Unit '{}' ({}): {} blocks",
                unit.key,
                unit.label,
                unit.blocks.len()
            );
        }
    } else {
        eprintln!(
            "Failed to load vachanamrut book from {}",
            target_dir.display()
        );
    }

    if let Ok(kirtan_book) = load_book(target_dir, "kirtan") {
        println!("Book: {} (id: {})", kirtan_book.title_en, kirtan_book.id);
        for unit in &kirtan_book.units {
            println!(
                "  Unit '{}' ({}): {} blocks",
                unit.key,
                unit.label,
                unit.blocks.len()
            );
        }
    } else {
        eprintln!("Failed to load kirtan book from {}", target_dir.display());
    }
}
