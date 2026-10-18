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
/// The big Preview's sharper picture; only a few are kept, one per recently selected cue.
const LARGE_WIDTH: u32 = 1280;
const LARGE_HEIGHT: u32 = 720;
const LARGE_CACHE_CAPACITY: usize = 6;
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

/// Grid tiles use the small tier; the big Preview adds the large one.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Tier {
    Small,
    Large,
}

struct Request {
    path: PathBuf,
    tier: Tier,
}

struct Failure {
    reason: String,
    missing: bool,
}

struct ResultMessage {
    path: PathBuf,
    tier: Tier,
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

/// Textures of one tier, least recently used evicted first.
struct Cache {
    entries: HashMap<PathBuf, Slot>,
    lru: VecDeque<PathBuf>,
    capacity: usize,
}

impl Cache {
    fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            lru: VecDeque::new(),
            capacity,
        }
    }

    fn touch(&mut self, path: &Path) {
        if let Some(index) = self.lru.iter().position(|entry| entry == path) {
            self.lru.remove(index);
        }
        self.lru.push_front(path.to_path_buf());
    }

    /// Evicts the least recently used entry that is not being loaded when the cache is full.
    fn insert(&mut self, path: PathBuf, slot: Slot, loading: impl Fn(&Path) -> bool) {
        if !self.entries.contains_key(&path) && self.entries.len() >= self.capacity {
            let Some(index) = self.lru.iter().rposition(|candidate| !loading(candidate)) else {
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

/// UI-side texture cache with media probing and decoding isolated on a small worker pool.
pub struct ThumbnailState {
    requests: SyncSender<Request>,
    results: Receiver<ResultMessage>,
    pending: HashSet<(PathBuf, Tier)>,
    small: Cache,
    large: Cache,
    shutdown: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl ThumbnailState {
    pub fn new(ctx: Context) -> Self {
        Self::with_loader(ctx, || {
            let mut small = ThumbnailCache::new(WORKER_CACHE_CAPACITY);
            let mut large = ThumbnailCache::new(2);
            move |path: &Path, tier| match tier {
                Tier::Small => small.get_or_load(path, THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT),
                Tier::Large => large.get_or_load(path, LARGE_WIDTH, LARGE_HEIGHT),
            }
        })
    }

    /// `make_loader` runs once per worker so each owns its own cache.
    fn with_loader<L>(ctx: Context, make_loader: impl Fn() -> L) -> Self
    where
        L: FnMut(&Path, Tier) -> Result<Arc<Frame>, MediaError> + Send + 'static,
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
            small: Cache::new(TEXTURE_CACHE_CAPACITY),
            large: Cache::new(LARGE_CACHE_CAPACITY),
            shutdown,
            workers,
        }
    }

    /// Queue a thumbnail request without probing or reading the path on the UI thread.
    pub fn request(&mut self, path: &Path) {
        self.request_tier(path, Tier::Small);
    }

    /// Queue the sharper picture for the big Preview.
    pub fn request_large(&mut self, path: &Path) {
        self.request_tier(path, Tier::Large);
    }

    fn request_tier(&mut self, path: &Path, tier: Tier) {
        let key = path.to_path_buf();
        if self.pending.contains(&(key.clone(), tier)) {
            return;
        }
        // A retry keeps showing the old error until the new result arrives.
        let retry = match self.cache(tier).entries.get(&key) {
            Some(Slot::Ready(_)) => {
                self.cache_mut(tier).touch(&key);
                return;
            }
            Some(Slot::Failed { at, missing, .. }) => {
                if !should_retry(*at, Instant::now(), *missing) {
                    self.cache_mut(tier).touch(&key);
                    return;
                }
                true
            }
            _ => false,
        };
        if self.pending.len() >= MAX_OUTSTANDING {
            if !retry {
                self.insert_entry(tier, key, Slot::QueueFull);
            }
            return;
        }

        match self.requests.try_send(Request {
            path: key.clone(),
            tier,
        }) {
            Ok(()) => {
                if !retry {
                    self.insert_entry(tier, key.clone(), Slot::Loading);
                }
                self.cache_mut(tier).touch(&key);
                self.pending.insert((key, tier));
            }
            Err(TrySendError::Full(_)) => {
                if !retry {
                    self.insert_entry(tier, key, Slot::QueueFull);
                }
            }
            Err(TrySendError::Disconnected(_)) => self.insert_entry(
                tier,
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
            self.pending.remove(&(message.path.clone(), message.tier));
            let slot = match message.result {
                Ok(frame) => {
                    let image = ColorImage::from_rgba_unmultiplied(
                        [frame.width as usize, frame.height as usize],
                        &frame.rgba,
                    );
                    Slot::Ready(ctx.load_texture(
                        texture_name(&message.path, message.tier),
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
            self.insert_entry(message.tier, message.path, slot);
        }
    }

    pub fn texture(&self, path: &Path) -> Option<&TextureHandle> {
        ready(&self.small, path)
    }

    pub fn large_texture(&self, path: &Path) -> Option<&TextureHandle> {
        ready(&self.large, path)
    }

    pub fn images(&self) -> HashMap<PathBuf, TextureHandle> {
        self.small
            .entries
            .iter()
            .filter_map(|(p, slot)| match slot {
                Slot::Ready(t) => Some((p.clone(), t.clone())),
                _ => None,
            })
            .collect()
    }

    /// What to show in place of a thumbnail; `None` when the thumbnail is ready.
    pub fn state(&self, path: &Path) -> Option<PreviewState> {
        match self.small.entries.get(path) {
            Some(Slot::Ready(_)) => None,
            Some(Slot::Failed { missing: true, .. }) => Some(PreviewState::Missing),
            Some(Slot::Failed { reason, .. }) => Some(PreviewState::Failed(reason.clone())),
            _ => Some(PreviewState::Loading),
        }
    }

    fn cache(&self, tier: Tier) -> &Cache {
        match tier {
            Tier::Small => &self.small,
            Tier::Large => &self.large,
        }
    }

    fn cache_mut(&mut self, tier: Tier) -> &mut Cache {
        match tier {
            Tier::Small => &mut self.small,
            Tier::Large => &mut self.large,
        }
    }

    fn insert_entry(&mut self, tier: Tier, path: PathBuf, slot: Slot) {
        let pending = &self.pending;
        let cache = match tier {
            Tier::Small => &mut self.small,
            Tier::Large => &mut self.large,
        };
        cache.insert(path, slot, |p| pending.contains(&(p.to_path_buf(), tier)));
    }
}

fn ready<'a>(cache: &'a Cache, path: &Path) -> Option<&'a TextureHandle> {
    match cache.entries.get(path)? {
        Slot::Ready(texture) => Some(texture),
        _ => None,
    }
}

fn worker<L>(
    requests: Arc<Mutex<Receiver<Request>>>,
    results: SyncSender<ResultMessage>,
    shutdown: Arc<AtomicBool>,
    ctx: Context,
    mut load: L,
) where
    L: FnMut(&Path, Tier) -> Result<Arc<Frame>, MediaError>,
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

        let result = load(&request.path, request.tier).map_err(|error| Failure {
            missing: !request.path.exists(),
            reason: error.to_string(),
        });
        let mut message = ResultMessage {
            path: request.path,
            tier: request.tier,
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

fn texture_name(path: &Path, tier: Tier) -> String {
    format!("lumen-thumbnail-{tier:?}:{}", path.display())
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
            move |path: &Path, _tier| {
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

    #[test]
    fn tiers_keep_their_own_textures_for_the_same_path() {
        let ctx = Context::default();
        let mut state = ThumbnailState::with_loader(ctx.clone(), || {
            |_: &Path, tier| {
                let side = if tier == Tier::Large { 4 } else { 2 };
                Ok(Arc::new(Frame {
                    width: side,
                    height: side,
                    rgba: vec![255; (side * side * 4) as usize],
                    sequence: 0,
                }))
            }
        });
        let path = Path::new("/lumen-test-tiers/a.png");
        state.request(path);
        state.request_large(path);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !state.pending.is_empty() && Instant::now() < deadline {
            state.poll(&ctx);
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(state.texture(path).unwrap().size(), [2, 2]);
        assert_eq!(state.large_texture(path).unwrap().size(), [4, 4]);
        // The large tier never feeds the grid's image map.
        assert_eq!(state.images()[path].size(), [2, 2]);
    }

    #[test]
    fn a_cache_keeps_its_capacity_and_evicts_the_oldest_idle_entry() {
        let mut cache = Cache::new(LARGE_CACHE_CAPACITY);
        let path = |n: usize| PathBuf::from(format!("/lumen-test-lru/{n}.png"));
        for n in 0..LARGE_CACHE_CAPACITY + 2 {
            cache.insert(path(n), Slot::Loading, |_| false);
        }
        assert_eq!(cache.entries.len(), LARGE_CACHE_CAPACITY);
        assert!(!cache.entries.contains_key(&path(0)));
        assert!(cache.entries.contains_key(&path(7)));
        // An entry that is still loading is not evicted, even when it is the oldest.
        let oldest = path(2);
        cache.insert(path(8), Slot::Loading, |p| p == oldest);
        assert!(cache.entries.contains_key(&oldest));
        assert!(!cache.entries.contains_key(&path(3)));
        // When everything is loading, the new entry waits rather than evicting.
        cache.insert(path(9), Slot::Loading, |_| true);
        assert!(!cache.entries.contains_key(&path(9)));
    }
}
