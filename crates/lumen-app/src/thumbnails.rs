use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use lumen_media::{Frame, MediaError, ThumbnailCache};

const THUMBNAIL_WIDTH: u32 = 320;
const THUMBNAIL_HEIGHT: u32 = 180;
const REQUEST_QUEUE_CAPACITY: usize = 32;
const RESULT_QUEUE_CAPACITY: usize = 64;
const MAX_OUTSTANDING: usize = RESULT_QUEUE_CAPACITY;
const TEXTURE_CACHE_CAPACITY: usize = 256;
/// Decoded-frame cache per worker; the UI-side texture cache is the one that matters.
const WORKER_CACHE_CAPACITY: usize = 64;
/// Dropping a pile of clips queues ffprobe + ffmpeg for each; three keeps the UI fed without
/// starving the live video decoder.
const THUMBNAIL_WORKERS: usize = 3;
const RETRY_AFTER: Duration = Duration::from_secs(10);

/// What a media preview surface shows while there is no thumbnail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewState {
    Loading,
    Failed(String),
    Missing,
}

struct Request {
    path: PathBuf,
}

struct Failure {
    reason: String,
    missing: bool,
}

struct ResultMessage {
    path: PathBuf,
    result: Result<Arc<Frame>, Failure>,
}

enum Slot {
    Loading,
    QueueFull,
    Ready(TextureHandle),
    Failed {
        reason: String,
        at: Instant,
        missing: bool,
    },
}

/// A failed thumbnail is tried again after `RETRY_AFTER` unless the file does not exist.
pub fn should_retry(failed_at: Instant, now: Instant, missing: bool) -> bool {
    !missing && now.saturating_duration_since(failed_at) >= RETRY_AFTER
}

/// UI-side texture cache with media probing and decoding isolated on a small worker pool.
pub struct ThumbnailState {
    requests: SyncSender<Request>,
    results: Receiver<ResultMessage>,
    pending: HashSet<PathBuf>,
    entries: HashMap<PathBuf, Slot>,
    lru: VecDeque<PathBuf>,
    shutdown: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl ThumbnailState {
    pub fn new(ctx: Context) -> Self {
        Self::with_loader(ctx, || {
            let mut cache = ThumbnailCache::new(WORKER_CACHE_CAPACITY);
            move |path: &Path| cache.get_or_load(path, THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT)
        })
    }

    /// `make_loader` runs once per worker so each owns its own cache.
    fn with_loader<L>(ctx: Context, make_loader: impl Fn() -> L) -> Self
    where
        L: FnMut(&Path) -> Result<Arc<Frame>, MediaError> + Send + 'static,
    {
        let (request_tx, request_rx) = mpsc::sync_channel::<Request>(REQUEST_QUEUE_CAPACITY);
        let (result_tx, result_rx) = mpsc::sync_channel::<ResultMessage>(RESULT_QUEUE_CAPACITY);
        let shutdown = Arc::new(AtomicBool::new(false));
        let request_rx = Arc::new(Mutex::new(request_rx));
        let workers = (0..THUMBNAIL_WORKERS)
            .filter_map(|n| {
                let (rx, tx) = (Arc::clone(&request_rx), result_tx.clone());
                let (stop, ctx, load) = (Arc::clone(&shutdown), ctx.clone(), make_loader());
                thread::Builder::new()
                    .name(format!("lumen-thumbnails-{n}"))
                    .spawn(move || worker(rx, tx, stop, ctx, load))
                    .ok()
            })
            .collect();

        Self {
            requests: request_tx,
            results: result_rx,
            pending: HashSet::new(),
            entries: HashMap::new(),
            lru: VecDeque::new(),
            shutdown,
            workers,
        }
    }

    /// Queue a thumbnail request without probing or reading the path on the UI thread.
    pub fn request(&mut self, path: &Path) {
        let key = path.to_path_buf();
        if self.pending.contains(&key) {
            return;
        }
        // A retry keeps showing the old error until the new result arrives.
        let retry = match self.entries.get(&key) {
            Some(Slot::Ready(_)) => {
                self.touch(&key);
                return;
            }
            Some(Slot::Failed { at, missing, .. }) => {
                if !should_retry(*at, Instant::now(), *missing) {
                    self.touch(&key);
                    return;
                }
                true
            }
            _ => false,
        };
        if self.pending.len() >= MAX_OUTSTANDING {
            if !retry {
                self.insert_entry(key, Slot::QueueFull);
            }
            return;
        }

        match self.requests.try_send(Request { path: key.clone() }) {
            Ok(()) => {
                if !retry {
                    self.insert_entry(key.clone(), Slot::Loading);
                }
                self.touch(&key);
                self.pending.insert(key);
            }
            Err(TrySendError::Full(_)) => {
                if !retry {
                    self.insert_entry(key, Slot::QueueFull);
                }
            }
            Err(TrySendError::Disconnected(_)) => self.insert_entry(
                key,
                Slot::Failed {
                    reason: "Thumbnail worker stopped".into(),
                    at: Instant::now(),
                    missing: false,
                },
            ),
        }
    }

    /// Drain completed results and create GPU textures on the UI thread.
    pub fn poll(&mut self, ctx: &Context) {
        while let Ok(message) = self.results.try_recv() {
            self.pending.remove(&message.path);
            let slot = match message.result {
                Ok(frame) => {
                    let image = ColorImage::from_rgba_unmultiplied(
                        [frame.width as usize, frame.height as usize],
                        &frame.rgba,
                    );
                    Slot::Ready(ctx.load_texture(
                        texture_name(&message.path),
                        image,
                        TextureOptions::LINEAR,
                    ))
                }
                Err(Failure { reason, missing }) => Slot::Failed {
                    reason,
                    at: Instant::now(),
                    missing,
                },
            };
            self.insert_entry(message.path, slot);
        }
    }

    pub fn texture(&self, path: &Path) -> Option<&TextureHandle> {
        match self.entries.get(path)? {
            Slot::Ready(texture) => Some(texture),
            _ => None,
        }
    }

    pub fn images(&self) -> HashMap<PathBuf, TextureHandle> {
        self.entries
            .iter()
            .filter_map(|(p, slot)| match slot {
                Slot::Ready(t) => Some((p.clone(), t.clone())),
                _ => None,
            })
            .collect()
    }

    /// What to show in place of a thumbnail; `None` when the thumbnail is ready.
    pub fn state(&self, path: &Path) -> Option<PreviewState> {
        match self.entries.get(path) {
            Some(Slot::Ready(_)) => None,
            Some(Slot::Failed { missing: true, .. }) => Some(PreviewState::Missing),
            Some(Slot::Failed { reason, .. }) => Some(PreviewState::Failed(reason.clone())),
            _ => Some(PreviewState::Loading),
        }
    }

    fn touch(&mut self, path: &Path) {
        if let Some(index) = self.lru.iter().position(|entry| entry == path) {
            self.lru.remove(index);
        }
        self.lru.push_front(path.to_path_buf());
    }

    fn insert_entry(&mut self, path: PathBuf, slot: Slot) {
        if !self.entries.contains_key(&path) && self.entries.len() >= TEXTURE_CACHE_CAPACITY {
            let Some(index) = self
                .lru
                .iter()
                .rposition(|candidate| !self.pending.contains(candidate))
            else {
                return;
            };
            if let Some(evicted) = self.lru.remove(index) {
                self.entries.remove(&evicted);
            }
        }
        self.entries.insert(path.clone(), slot);
        self.touch(&path);
    }
}

fn worker<L>(
    requests: Arc<Mutex<Receiver<Request>>>,
    results: SyncSender<ResultMessage>,
    shutdown: Arc<AtomicBool>,
    ctx: Context,
    mut load: L,
) where
    L: FnMut(&Path) -> Result<Arc<Frame>, MediaError>,
{
    while !shutdown.load(Ordering::Acquire) {
        // The lock is held only while waiting, so an idle worker never blocks a busy one.
        let received = match requests.lock() {
            Ok(rx) => rx.recv_timeout(Duration::from_millis(100)),
            Err(_) => return,
        };
        let request = match received {
            Ok(request) => request,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };
        if shutdown.load(Ordering::Acquire) {
            return;
        }

        let result = load(&request.path).map_err(|error| Failure {
            missing: !request.path.exists(),
            reason: error.to_string(),
        });
        let mut message = ResultMessage {
            path: request.path,
            result,
        };
        loop {
            match results.try_send(message) {
                Ok(()) => {
                    ctx.request_repaint();
                    break;
                }
                Err(TrySendError::Full(returned)) => {
                    if shutdown.load(Ordering::Acquire) {
                        return;
                    }
                    message = returned;
                    thread::sleep(Duration::from_millis(10));
                }
                Err(TrySendError::Disconnected(_)) => return,
            }
        }
    }
}

impl Drop for ThumbnailState {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        // Dropping the JoinHandles detaches the workers; a currently running media decode is
        // bounded by lumen-media's timeout and does not block closing the UI.
        self.workers.clear();
    }
}

fn texture_name(path: &Path) -> String {
    format!("lumen-thumbnail:{}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn failures_retry_after_ten_seconds_except_missing_files() {
        let failed = Instant::now();
        let at = |secs| failed + Duration::from_secs(secs);
        assert!(!should_retry(failed, at(9), false));
        assert!(should_retry(failed, at(10), false));
        assert!(!should_retry(failed, at(3600), true));
        // A clock that appears to run backwards never retries early.
        assert!(!should_retry(at(5), failed, false));
    }

    fn fake_frame() -> Arc<Frame> {
        Arc::new(Frame {
            width: 2,
            height: 2,
            rgba: vec![255; 16],
            sequence: 0,
        })
    }

    #[test]
    fn three_workers_drain_six_requests_and_missing_files_are_marked() {
        let ctx = Context::default();
        let (active, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let mut state = ThumbnailState::with_loader(ctx.clone(), || {
            let (active, peak) = (Arc::clone(&active), Arc::clone(&peak));
            move |path: &Path| {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(150));
                active.fetch_sub(1, Ordering::SeqCst);
                if path.starts_with("/lumen-test-absent") {
                    Err(MediaError::InvalidPath("gone".into()))
                } else {
                    Ok(fake_frame())
                }
            }
        });
        let paths: Vec<PathBuf> = (0..5)
            .map(|n| PathBuf::from(format!("/lumen-test-present-{n}.png")))
            .chain([PathBuf::from("/lumen-test-absent/x.mov")])
            .collect();
        for path in &paths {
            state.request(path);
            assert_eq!(state.state(path), Some(PreviewState::Loading));
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while !state.pending.is_empty() && Instant::now() < deadline {
            state.poll(&ctx);
            thread::sleep(Duration::from_millis(10));
        }
        assert!(state.pending.is_empty(), "queue did not drain");
        assert_eq!(peak.load(Ordering::SeqCst), THUMBNAIL_WORKERS);
        for path in &paths[..5] {
            assert!(state.texture(path).is_some());
            assert_eq!(state.state(path), None);
        }
        assert_eq!(state.state(&paths[5]), Some(PreviewState::Missing));
        // Missing files are never queued again.
        state.request(&paths[5]);
        assert!(state.pending.is_empty());
    }
}
