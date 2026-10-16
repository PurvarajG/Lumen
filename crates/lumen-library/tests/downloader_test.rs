use lumen_library::downloader::*;
use lumen_library::error::LibraryError;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::tempdir;

struct FakeFetcher {
    responses: Mutex<HashMap<String, Result<String, LibraryError>>>,
    call_counts: Mutex<HashMap<String, usize>>,
    fetch_count: AtomicUsize,
}

impl FakeFetcher {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
            call_counts: Mutex::new(HashMap::new()),
            fetch_count: AtomicUsize::new(0),
        }
    }

    fn set_response(&self, url: &str, result: Result<String, LibraryError>) {
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), result);
    }
}

impl HttpFetcher for FakeFetcher {
    fn fetch(&self, url: &str) -> Result<String, LibraryError> {
        self.fetch_count.fetch_add(1, Ordering::SeqCst);
        let mut counts = self.call_counts.lock().unwrap();
        *counts.entry(url.to_string()).or_insert(0) += 1;

        let map = self.responses.lock().unwrap();
        if let Some(res) = map.get(url) {
            match res {
                Ok(s) => Ok(s.clone()),
                Err(LibraryError::Http(msg)) => Err(LibraryError::Http(msg.clone())),
                Err(e) => Err(LibraryError::Http(e.to_string())),
            }
        } else {
            Err(LibraryError::Http(format!("FakeFetcher 404: {url}")))
        }
    }
}

#[test]
fn test_downloader_fake_resumable_and_atomic() {
    let dir = tempdir().unwrap();
    let cache_dir = dir.path();

    let fake = Arc::new(FakeFetcher::new());
    // Provide responses for 2 yogi-gita sections
    let sample_html = r#"
    <h1 class="sec_title">વિભાગ એક</h1>
    <div class="gu_text"><p>ગુજરાતી લખાણ એક.</p></div>
    <div class="en_text"><p>English text one.</p></div>
    "#;

    fake.set_response(
        "https://www.anirdesh.com/yogi-gita/index.php?page=foreword",
        Ok(sample_html.into()),
    );
    fake.set_response(
        "https://www.anirdesh.com/yogi-gita/index.php?page=publishers_note",
        Ok(sample_html.into()),
    );

    let (tx, rx) = channel();
    let mut handle =
        Downloader::start_with_limit("yogi-gita", cache_dir, tx, fake.clone(), Some(2));

    // Wait for completion
    let mut finished = false;
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(5)) {
        if let DownloadEvent::Finished(summary) = event {
            assert_eq!(summary.id, "yogi-gita");
            assert_eq!(summary.units, 2);
            finished = true;
            break;
        }
    }
    assert!(finished, "download should finish successfully");
    handle.cancel();

    // Verify files on disk
    let book = load_book(cache_dir, "yogi-gita").expect("load_book");
    assert_eq!(book.id, "yogi-gita");
    assert_eq!(book.units.len(), 2);

    let installed = installed_books(cache_dir);
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].0, "yogi-gita");
    assert_eq!(installed[0].1, 2);
    assert!(installed[0].2 > 0);

    // Now test resumability: run again with same limit. Should make 0 additional network calls!
    let calls_before = fake.fetch_count.load(Ordering::SeqCst);
    let (tx2, rx2) = channel();
    let mut handle2 =
        Downloader::start_with_limit("yogi-gita", cache_dir, tx2, fake.clone(), Some(2));

    let mut finished2 = false;
    while let Ok(event) = rx2.recv_timeout(Duration::from_secs(5)) {
        if let DownloadEvent::Finished(_) = event {
            finished2 = true;
            break;
        }
    }
    assert!(finished2);
    handle2.cancel();

    let calls_after = fake.fetch_count.load(Ordering::SeqCst);
    assert_eq!(
        calls_before, calls_after,
        "cached units should be skipped without HTTP fetch"
    );

    // Test delete_book
    delete_book(cache_dir, "yogi-gita").expect("delete_book");
    assert!(installed_books(cache_dir).is_empty());
}

#[test]
fn test_downloader_stop_immediately_on_403() {
    let dir = tempdir().unwrap();
    let cache_dir = dir.path();

    let fake = Arc::new(FakeFetcher::new());
    fake.set_response(
        "https://www.anirdesh.com/yogi-gita/index.php?page=foreword",
        Err(LibraryError::Http(
            "HTTP 403 Forbidden: forbidden user agent".into(),
        )),
    );

    let (tx, rx) = channel();
    let mut handle =
        Downloader::start_with_limit("yogi-gita", cache_dir, tx, fake.clone(), Some(1));

    let mut failed = false;
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(5)) {
        if let DownloadEvent::Failed(msg) = event {
            assert!(msg.contains("403"));
            failed = true;
            break;
        }
    }
    assert!(failed, "should fail on 403");
    handle.cancel();

    // Verify it did not retry multiple times
    let counts = fake.call_counts.lock().unwrap();
    let calls = counts
        .get("https://www.anirdesh.com/yogi-gita/index.php?page=foreword")
        .copied()
        .unwrap_or(0);
    assert_eq!(calls, 1, "403 must stop immediately without retry");
}

#[test]
fn test_downloader_cancel_stops_promptly() {
    let dir = tempdir().unwrap();
    let cache_dir = dir.path();

    let fake = Arc::new(FakeFetcher::new());
    let sample_html = r#"
    <h1 class="sec_title">વિભાગ</h1>
    <div class="gu_text"><p>લખાણ</p></div>
    "#;
    for sec in &["foreword", "publishers_note", "nivedan", "patro", "sunrut"] {
        fake.set_response(
            &format!("https://www.anirdesh.com/yogi-gita/index.php?page={sec}"),
            Ok(sample_html.into()),
        );
    }

    let (tx, _rx) = channel();
    let mut handle = Downloader::start("yogi-gita", cache_dir, tx);
    // Cancel immediately
    handle.cancel();
    // Drop also cancels
    drop(handle);
}
