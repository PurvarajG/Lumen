use crate::error::LibraryError;
use crate::model::{Book, BookSummary, SCHEMA_VERSION, Unit, UnitRef, catalog, to_nfc};
use crate::parse::{
    parse_aksharamrutam, parse_asv, parse_bhagvat, parse_chintamani, parse_diksha, parse_gita,
    parse_harililamrut, parse_karika, parse_kavya, parse_kirtan, parse_pbp, parse_shloks,
    parse_vato, parse_yogi_gita,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadEvent {
    Progress { done: usize, total: usize },
    Warning(String),
    Finished(BookSummary),
    Failed(String),
}

pub trait HttpFetcher: Send + Sync + 'static {
    fn fetch(&self, url: &str) -> Result<String, LibraryError>;
}

pub struct UreqFetcher {
    agent: ureq::Agent,
}

impl Default for UreqFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl UreqFetcher {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(20)))
            .build();
        let agent = ureq::Agent::new_with_config(config);
        Self { agent }
    }
}

impl HttpFetcher for UreqFetcher {
    fn fetch(&self, url: &str) -> Result<String, LibraryError> {
        let request = self
            .agent
            .get(url)
            .header("User-Agent", "LumenPresenter/0.1");

        let response = match request.call() {
            Ok(resp) => resp,
            Err(ureq::Error::StatusCode(403)) => {
                return Err(LibraryError::Http(
                    "HTTP 403 Forbidden: access denied by server".into(),
                ));
            }
            Err(ureq::Error::StatusCode(404)) => {
                return Err(LibraryError::Http(
                    "HTTP 404 Not Found: resource does not exist".into(),
                ));
            }
            Err(ureq::Error::StatusCode(code)) => {
                return Err(LibraryError::Http(format!("HTTP error status: {code}")));
            }
            Err(ureq::Error::Timeout(_)) => {
                return Err(LibraryError::Http("Connection or read timed out".into()));
            }
            Err(e) => {
                return Err(LibraryError::Http(format!("Network error: {e}")));
            }
        };

        let status = response.status().as_u16();
        if status == 403 {
            return Err(LibraryError::Http("HTTP 403 Forbidden".into()));
        }
        if status == 404 {
            return Err(LibraryError::Http("HTTP 404 Not Found".into()));
        }
        if status >= 400 {
            return Err(LibraryError::Http(format!("HTTP status {status}")));
        }

        let reader = response.into_body().into_reader();
        let mut limited = reader.take(2 * 1024 * 1024 + 1); // 2 MiB bounded read
        let mut body = String::new();
        limited.read_to_string(&mut body)?;
        if body.len() > 2 * 1024 * 1024 {
            return Err(LibraryError::Http(
                "Response exceeded max 2 MiB limit".into(),
            ));
        }

        Ok(body)
    }
}

pub struct DownloadHandle {
    cancel_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl DownloadHandle {
    pub fn cancel(&mut self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
        if let Some(h) = self.thread_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for DownloadHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub struct Downloader;

impl Downloader {
    pub fn start(book_id: &str, cache_dir: &Path, events: Sender<DownloadEvent>) -> DownloadHandle {
        Self::start_internal(
            book_id,
            cache_dir,
            events,
            Arc::new(UreqFetcher::new()),
            None,
        )
    }

    pub fn start_with_fetcher(
        book_id: &str,
        cache_dir: &Path,
        events: Sender<DownloadEvent>,
        fetcher: Arc<dyn HttpFetcher>,
    ) -> DownloadHandle {
        Self::start_internal(book_id, cache_dir, events, fetcher, None)
    }

    pub fn start_with_limit(
        book_id: &str,
        cache_dir: &Path,
        events: Sender<DownloadEvent>,
        fetcher: Arc<dyn HttpFetcher>,
        max_units: Option<usize>,
    ) -> DownloadHandle {
        Self::start_internal(book_id, cache_dir, events, fetcher, max_units)
    }

    fn start_internal(
        book_id: &str,
        cache_dir: &Path,
        events: Sender<DownloadEvent>,
        fetcher: Arc<dyn HttpFetcher>,
        max_units: Option<usize>,
    ) -> DownloadHandle {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_for_thread = cancel_flag.clone();
        let book_id_owned = book_id.to_string();
        let cache_dir_buf = cache_dir.to_path_buf();

        let thread_handle = thread::spawn(move || {
            let res = run_download(
                &book_id_owned,
                &cache_dir_buf,
                &events,
                fetcher,
                cancel_for_thread,
                max_units,
            );
            if let Err(err) = res {
                let _ = events.send(DownloadEvent::Failed(err.to_string()));
            }
        });

        DownloadHandle {
            cancel_flag,
            thread_handle: Some(thread_handle),
        }
    }
}

// Bounded sleep checking cancel flag every 50ms
fn cancellable_sleep(duration: Duration, cancel: &Arc<AtomicBool>) -> bool {
    let start = Instant::now();
    while start.elapsed() < duration {
        if cancel.load(Ordering::SeqCst) {
            return false;
        }
        let remaining = duration.saturating_sub(start.elapsed());
        let slice = remaining.min(Duration::from_millis(50));
        thread::sleep(slice);
    }
    !cancel.load(Ordering::SeqCst)
}

fn fetch_with_retry(
    fetcher: &dyn HttpFetcher,
    url: &str,
    last_req: &mut Option<Instant>,
    cancel: &Arc<AtomicBool>,
) -> Result<String, LibraryError> {
    const MIN_INTERVAL: Duration = Duration::from_millis(1100);
    const BACKOFFS: [Duration; 3] = [
        Duration::from_millis(500),
        Duration::from_millis(1000),
        Duration::from_millis(2000),
    ];

    let mut attempt = 0;
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(LibraryError::Cancelled);
        }

        // Rate limiting: ≥ 1.1s between requests
        if let Some(last) = *last_req {
            let elapsed = last.elapsed();
            if elapsed < MIN_INTERVAL && !cancellable_sleep(MIN_INTERVAL - elapsed, cancel) {
                return Err(LibraryError::Cancelled);
            }
        }
        *last_req = Some(Instant::now());

        match fetcher.fetch(url) {
            Ok(body) => return Ok(body),
            Err(err) => {
                let is_403_or_404 = match &err {
                    LibraryError::Http(msg) => msg.contains("403") || msg.contains("404"),
                    _ => false,
                };
                if is_403_or_404 {
                    // Stop immediately on 403/404 for the book with clear error
                    return Err(err);
                }

                if attempt < BACKOFFS.len() {
                    let backoff = BACKOFFS[attempt];
                    attempt += 1;
                    if !cancellable_sleep(backoff, cancel) {
                        return Err(LibraryError::Cancelled);
                    }
                } else {
                    return Err(err);
                }
            }
        }
    }
}

fn fetch_page(
    fetcher: &dyn HttpFetcher,
    url: &str,
    last_req: &mut Option<Instant>,
    last_page: &mut Option<(String, String)>,
    cancel: &Arc<AtomicBool>,
) -> Result<String, LibraryError> {
    if let Some((_, html)) = last_page.as_ref().filter(|(u, _)| u == url) {
        return Ok(html.clone());
    }
    let html = fetch_with_retry(fetcher, url, last_req, cancel)?;
    *last_page = Some((url.to_string(), html.clone()));
    Ok(html)
}

fn run_download(
    book_id: &str,
    cache_dir: &Path,
    events: &Sender<DownloadEvent>,
    fetcher: Arc<dyn HttpFetcher>,
    cancel: Arc<AtomicBool>,
    max_units: Option<usize>,
) -> Result<(), LibraryError> {
    let book_info = catalog()
        .iter()
        .find(|b| b.id == book_id)
        .ok_or_else(|| LibraryError::NotFound(format!("Unknown book id '{book_id}'")))?;

    let book_dir = cache_dir.join(book_id);
    let units_dir = book_dir.join("units");
    fs::create_dir_all(&units_dir)?;

    let plan = build_download_plan(book_id)?;
    let total_plan_units = if let Some(m) = max_units {
        plan.len().min(m)
    } else {
        plan.len()
    };

    let mut last_request: Option<Instant> = None;
    // Units are ordered, so consecutive units usually share one page (diksha 5, vato 10).
    let mut last_page: Option<(String, String)> = None;
    let mut completed_units = 0;
    let mut collected_units = Vec::new();

    let _ = events.send(DownloadEvent::Progress {
        done: 0,
        total: total_plan_units,
    });

    for item in plan.into_iter().take(total_plan_units) {
        if cancel.load(Ordering::SeqCst) {
            return Err(LibraryError::Cancelled);
        }

        let unit_file = units_dir.join(format!("{}.json", item.key));
        if unit_file.exists() {
            // Already cached, load and skip network
            let data = fs::read_to_string(&unit_file)?;
            if let Ok(u) = serde_json::from_str::<Unit>(&data) {
                collected_units.push(u);
                completed_units += 1;
                let _ = events.send(DownloadEvent::Progress {
                    done: completed_units,
                    total: total_plan_units,
                });
                continue;
            }
        }

        // Fetch and parse unit
        let unit_ref = UnitRef::new(book_id, &item.key);
        let parsed_unit = match item.strategy {
            FetchStrategy::Vachanamrut { gu_url, en_url } => {
                let gu_html = fetch_page(
                    fetcher.as_ref(),
                    &gu_url,
                    &mut last_request,
                    &mut last_page,
                    &cancel,
                )?;
                let en_html = fetch_page(
                    fetcher.as_ref(),
                    &en_url,
                    &mut last_request,
                    &mut last_page,
                    &cancel,
                )?;
                let gu_u = crate::parse::parse_vachanamrut_page(&gu_html, "gu", &unit_ref)?;
                let en_u = crate::parse::parse_vachanamrut_page(&en_html, "en", &unit_ref)?;
                let (merged, warn) = crate::parse::merge_vachanamrut_units(gu_u, en_u);
                if let Some(w) = warn {
                    let _ = events.send(DownloadEvent::Warning(w));
                }
                merged
            }
            FetchStrategy::SinglePage { url, parser } => {
                let html = fetch_page(
                    fetcher.as_ref(),
                    &url,
                    &mut last_request,
                    &mut last_page,
                    &cancel,
                )?;
                parser(&html, &unit_ref)?
            }
        };

        // Write unit atomically
        write_unit_atomic(&unit_file, &parsed_unit)?;

        collected_units.push(parsed_unit);
        completed_units += 1;

        let _ = events.send(DownloadEvent::Progress {
            done: completed_units,
            total: total_plan_units,
        });
    }

    // Write cache_dir/<book>/book.json atomically
    let full_book = Book {
        id: book_id.to_string(),
        title_gu: to_nfc(book_info.title_gu),
        title_en: to_nfc(book_info.title_en),
        units: collected_units,
        schema: SCHEMA_VERSION,
    };

    let book_file = book_dir.join("book.json");
    write_book_atomic(&book_file, &full_book)?;

    let total_bytes = calculate_dir_bytes(&book_dir)?;
    let summary = BookSummary {
        id: book_id.to_string(),
        title_gu: to_nfc(book_info.title_gu),
        title_en: to_nfc(book_info.title_en),
        units: full_book.units.len(),
        bytes: total_bytes,
    };

    let _ = events.send(DownloadEvent::Finished(summary));
    Ok(())
}

fn write_unit_atomic(target_path: &Path, unit: &Unit) -> Result<(), LibraryError> {
    let tmp_path = target_path.with_extension("json.tmp");
    let json_bytes = serde_json::to_vec_pretty(unit)?;
    fs::write(&tmp_path, json_bytes)?;
    fs::rename(tmp_path, target_path)?;
    Ok(())
}

fn write_book_atomic(target_path: &Path, book: &Book) -> Result<(), LibraryError> {
    let tmp_path = target_path.with_extension("json.tmp");
    let json_bytes = serde_json::to_vec_pretty(book)?;
    fs::write(&tmp_path, json_bytes)?;
    fs::rename(tmp_path, target_path)?;
    Ok(())
}

pub fn load_book(cache_dir: &Path, id: &str) -> Result<Book, LibraryError> {
    let book_dir = cache_dir.join(id);
    let book_file = book_dir.join("book.json");

    if book_file.exists() {
        let content = fs::read_to_string(&book_file)?;
        let book = serde_json::from_str::<Book>(&content)?;
        return Ok(book);
    }

    let units_dir = book_dir.join("units");
    if units_dir.exists() {
        let mut units = Vec::new();
        for entry in fs::read_dir(&units_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let data = fs::read_to_string(&path)?;
                if let Ok(u) = serde_json::from_str::<Unit>(&data) {
                    units.push(u);
                }
            }
        }
        if !units.is_empty() {
            let book_info = catalog().iter().find(|b| b.id == id);
            return Ok(Book {
                id: id.to_string(),
                title_gu: book_info
                    .map(|b| to_nfc(b.title_gu))
                    .unwrap_or_else(|| id.to_string()),
                title_en: book_info
                    .map(|b| to_nfc(b.title_en))
                    .unwrap_or_else(|| id.to_string()),
                units,
                schema: SCHEMA_VERSION,
            });
        }
    }

    Err(LibraryError::NotFound(format!(
        "Book '{id}' not found in cache at {}",
        book_dir.display()
    )))
}

pub fn installed_books(cache_dir: &Path) -> Vec<(String, usize, u64)> {
    let mut installed = Vec::new();
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return installed;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = match path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if let Ok(book) = load_book(cache_dir, &id) {
            let bytes = calculate_dir_bytes(&path).unwrap_or(0);
            installed.push((id, book.units.len(), bytes));
        }
    }

    installed
}

pub fn delete_book(cache_dir: &Path, id: &str) -> Result<(), LibraryError> {
    let book_dir = cache_dir.join(id);
    if book_dir.exists() {
        fs::remove_dir_all(book_dir)?;
    }
    Ok(())
}

fn calculate_dir_bytes(dir: &Path) -> Result<u64, std::io::Error> {
    let mut total = 0;
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                total += calculate_dir_bytes(&path)?;
            } else {
                total += entry.metadata()?.len();
            }
        }
    }
    Ok(total)
}

// Download planning structures
enum FetchStrategy {
    Vachanamrut {
        gu_url: String,
        en_url: String,
    },
    SinglePage {
        url: String,
        parser: fn(&str, &UnitRef) -> Result<Unit, LibraryError>,
    },
}

struct PlanItem {
    key: String,
    strategy: FetchStrategy,
}

fn build_download_plan(book_id: &str) -> Result<Vec<PlanItem>, LibraryError> {
    let mut plan = Vec::new();

    match book_id {
        "vachanamrut" => {
            for vachno in 1..=274 {
                let key = vachanamrut_key(vachno);
                let gu_url = format!(
                    "https://www.anirdesh.com/vachanamrut/index.php?format=gu&vachno={vachno}"
                );
                let en_url = format!(
                    "https://www.anirdesh.com/vachanamrut/index.php?format=en&vachno={vachno}"
                );
                plan.push(PlanItem {
                    key,
                    strategy: FetchStrategy::Vachanamrut { gu_url, en_url },
                });
            }
        }
        "vato" => {
            let prakarans = [
                (1, 342),
                (2, 192),
                (3, 74),
                (4, 140),
                (5, 407),
                (6, 292),
                (7, 37),
            ];
            for &(p, count) in &prakarans {
                for no in 1..=count {
                    let key = format!("{p}-{no}");
                    let beg = ((no - 1) / 10) * 10 + 1;
                    let url = format!(
                        "https://www.anirdesh.com/vato/index.php?prakaran={p}&beg={beg}&increment=10"
                    );
                    plan.push(PlanItem {
                        key,
                        strategy: FetchStrategy::SinglePage {
                            url,
                            parser: parse_vato,
                        },
                    });
                }
            }
        }
        "kirtan" => {
            for no in 1..=1300 {
                let key = format!("1-{no}");
                let url = format!("https://www.anirdesh.com/kirtan/index.php?part=1&no={no}");
                plan.push(PlanItem {
                    key,
                    strategy: FetchStrategy::SinglePage {
                        url,
                        parser: parse_kirtan,
                    },
                });
            }
            for no in 1..=1191 {
                let key = format!("2-{no}");
                let url = format!("https://www.anirdesh.com/kirtan/index.php?part=2&no={no}");
                plan.push(PlanItem {
                    key,
                    strategy: FetchStrategy::SinglePage {
                        url,
                        parser: parse_kirtan,
                    },
                });
            }
        }
        "diksha" => {
            for no in 1..=315 {
                let key = format!("{no}");
                let shlok_base = ((no - 1) / 5) * 5 + 1;
                let url = format!(
                    "https://www.anirdesh.com/diksha/index.php?shlok={shlok_base}&increment=5"
                );
                plan.push(PlanItem {
                    key,
                    strategy: FetchStrategy::SinglePage {
                        url,
                        parser: parse_diksha,
                    },
                });
            }
        }
        "yogi-gita" => {
            let sections = [
                "foreword",
                "publishers_note",
                "nivedan",
                "patro",
                "sunrut",
                "prarthana",
                "jivan_bhavana",
                "hrudayni_vato",
            ];
            for sec in sections {
                let url = format!("https://www.anirdesh.com/yogi-gita/index.php?page={sec}");
                plan.push(PlanItem {
                    key: sec.to_string(),
                    strategy: FetchStrategy::SinglePage {
                        url,
                        parser: parse_yogi_gita,
                    },
                });
            }
        }
        "karika" => {
            let sections = [
                "mangal-dhara",
                "aksharbrahma-dhara",
                "parabrahma-dhara",
                "praman-dhara",
                "maya-dhara",
                "jiva-dhara",
                "ishwar-dhara",
                "sadhan-dhara",
                "fal-dhara",
                "upsanhar",
                "namavali",
            ];
            for sec in sections {
                let url = format!("https://www.anirdesh.com/karika/index.php?section={sec}");
                plan.push(PlanItem {
                    key: sec.to_string(),
                    strategy: FetchStrategy::SinglePage {
                        url,
                        parser: parse_karika,
                    },
                });
            }
        }
        "chintamani" => {
            for p in 1..=164 {
                let key = format!("{p}");
                let url = format!("https://www.anirdesh.com/chintamani/index.php?prakaran={p}");
                plan.push(PlanItem {
                    key,
                    strategy: FetchStrategy::SinglePage {
                        url,
                        parser: parse_chintamani,
                    },
                });
            }
        }
        "harililamrut" => {
            let kalashes = [
                (1, 20),
                (2, 18),
                (3, 27),
                (4, 31),
                (5, 28),
                (6, 29),
                (7, 83),
                (8, 63),
                (9, 13),
                (10, 20),
            ];
            for &(k, count) in &kalashes {
                for v in 1..=count {
                    let key = format!("{k}-{v}");
                    let url = format!(
                        "https://www.anirdesh.com/harililamrut/index.php?kalash={k}&vishram={v}"
                    );
                    plan.push(PlanItem {
                        key,
                        strategy: FetchStrategy::SinglePage {
                            url,
                            parser: parse_harililamrut,
                        },
                    });
                }
            }
        }
        "aksharamrutam" => {
            let prakarans = [
                (8, 131),
                (9, 199),
                (10, 146),
                (11, 237),
                (12, 107),
                (13, 108),
                (14, 84),
                (15, 32),
            ];
            for &(p, count) in &prakarans {
                for no in 1..=count {
                    let key = format!("{p}-{no}");
                    let start = ((no - 1) / 10) * 10 + 1;
                    let url = format!(
                        "https://www.anirdesh.com/aksharamrutam/index.php?prakaran={p}&start={start}&increment=10"
                    );
                    plan.push(PlanItem {
                        key,
                        strategy: FetchStrategy::SinglePage {
                            url,
                            parser: parse_aksharamrutam,
                        },
                    });
                }
            }
        }
        "pbp" => {
            for chp in 1..=31 {
                let key = format!("{chp}");
                let url = format!("https://www.anirdesh.com/pbp/index.php?page=chp_{chp}");
                plan.push(PlanItem {
                    key,
                    strategy: FetchStrategy::SinglePage {
                        url,
                        parser: parse_pbp,
                    },
                });
            }
        }
        "asv" => {
            for chp in 1..=39 {
                let key = format!("{chp}");
                let url = format!("https://www.anirdesh.com/asv/index.php?page=chp_{chp}");
                plan.push(PlanItem {
                    key,
                    strategy: FetchStrategy::SinglePage {
                        url,
                        parser: parse_asv,
                    },
                });
            }
        }
        "gita" => {
            for chp in 1..=18 {
                let key = format!("{chp}");
                let url = format!("https://www.anirdesh.com/gita/index.php?page=chapter_{chp}");
                plan.push(PlanItem {
                    key,
                    strategy: FetchStrategy::SinglePage {
                        url,
                        parser: parse_gita,
                    },
                });
            }
        }
        "bhagvat" => {
            let skandhas = [
                (1, 19),
                (2, 10),
                (3, 33),
                (4, 31),
                (5, 26),
                (6, 19),
                (7, 15),
                (8, 24),
                (9, 24),
                (10, 90),
                (11, 31),
                (12, 13),
            ];
            for &(s, count) in &skandhas {
                for a in 1..=count {
                    let key = format!("{s}-{a}");
                    let url =
                        format!("https://www.anirdesh.com/bhagvat/index.php?skandh={s}&adhyay={a}");
                    plan.push(PlanItem {
                        key,
                        strategy: FetchStrategy::SinglePage {
                            url,
                            parser: parse_bhagvat,
                        },
                    });
                }
            }
        }
        "shloks" => {
            plan.push(PlanItem {
                key: "collection".into(),
                strategy: FetchStrategy::SinglePage {
                    url: "https://www.anirdesh.com/kirtan/shloks.php".into(),
                    parser: parse_shloks,
                },
            });
        }
        "kavya" => {
            for p in 1..=55 {
                let key = format!("prakash-{p}");
                let url = format!("https://www.anirdesh.com/kavya/prakash/index.php?prakarah={p}");
                plan.push(PlanItem {
                    key,
                    strategy: FetchStrategy::SinglePage {
                        url,
                        parser: parse_kavya,
                    },
                });
            }
        }
        _ => {
            return Err(LibraryError::NotFound(format!(
                "No download plan for book '{book_id}'"
            )));
        }
    }

    Ok(plan)
}

fn vachanamrut_key(vachno: usize) -> String {
    match vachno {
        1..=78 => format!("gadhada-1-{}", vachno),
        79..=96 => format!("sarangpur-{}", vachno - 78),
        97..=108 => format!("kariyani-{}", vachno - 96),
        109..=126 => format!("loya-{}", vachno - 108),
        127..=133 => format!("panchala-{}", vachno - 126),
        134..=200 => format!("gadhada-2-{}", vachno - 133),
        201..=220 => format!("vartal-{}", vachno - 200),
        221..=223 => format!("ahmedabad-{}", vachno - 220),
        224..=262 => format!("gadhada-3-{}", vachno - 223),
        263 => "bhugol-khagol".into(),
        264 => "amdavad-4".into(),
        265 => "amdavad-5".into(),
        266 => "amdavad-7".into(),
        267 => "amdavad-8".into(),
        268 => "ashlali".into(),
        269..=273 => format!("jetalpur-{}", vachno - 268),
        274 => "additional-11".into(),
        _ => format!("{vachno}"),
    }
}
