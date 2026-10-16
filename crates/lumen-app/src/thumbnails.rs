use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use lumen_media::{Frame, ThumbnailCache};

const THUMBNAIL_WIDTH: u32 = 320;
const THUMBNAIL_HEIGHT: u32 = 180;
const REQUEST_QUEUE_CAPACITY: usize = 32;
const RESULT_QUEUE_CAPACITY: usize = 64;
const MAX_OUTSTANDING: usize = RESULT_QUEUE_CAPACITY;
const TEXTURE_CACHE_CAPACITY: usize = 256;
const QUEUE_FULL: &str = "Thumbnail queue is full";

struct Request {
    path: PathBuf,
}

struct ResultMessage {
    path: PathBuf,
    result: Result<Arc<Frame>, String>,
}

#[derive(Default)]
struct ThumbnailEntry {
    texture: Option<TextureHandle>,
    status: Option<String>,
}

/// UI-side texture cache with media probing and decoding isolated on one worker.
pub struct ThumbnailState {
    requests: SyncSender<Request>,
    results: Receiver<ResultMessage>,
    pending: HashSet<PathBuf>,
    entries: HashMap<PathBuf, ThumbnailEntry>,
    lru: VecDeque<PathBuf>,
    shutdown: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ThumbnailState {
    pub fn new(ctx: Context) -> Self {
        let (request_tx, request_rx) = mpsc::sync_channel::<Request>(REQUEST_QUEUE_CAPACITY);
        let (result_tx, result_rx) = mpsc::sync_channel::<ResultMessage>(RESULT_QUEUE_CAPACITY);
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = Arc::clone(&shutdown);
        let worker_ctx = ctx.clone();
        let worker = thread::Builder::new()
            .name("lumen-thumbnails".into())
            .spawn(move || {
                let mut cache = ThumbnailCache::new(TEXTURE_CACHE_CAPACITY);
                while !worker_shutdown.load(Ordering::Acquire) {
                    let request = match request_rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(request) => request,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    if worker_shutdown.load(Ordering::Acquire) {
                        break;
                    }

                    let result = cache
                        .get_or_load(&request.path, THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT)
                        .map_err(|error| error.to_string());
                    let message = ResultMessage {
                        path: request.path,
                        result,
                    };
                    let mut message = message;
                    loop {
                        match result_tx.try_send(message) {
                            Ok(()) => {
                                worker_ctx.request_repaint();
                                break;
                            }
                            Err(TrySendError::Full(returned)) => {
                                if worker_shutdown.load(Ordering::Acquire) {
                                    break;
                                }
                                message = returned;
                                thread::sleep(Duration::from_millis(10));
                            }
                            Err(TrySendError::Disconnected(_)) => return,
                        }
                    }
                }
            })
            .ok();

        Self {
            requests: request_tx,
            results: result_rx,
            pending: HashSet::new(),
            entries: HashMap::new(),
            lru: VecDeque::new(),
            shutdown,
            worker,
        }
    }

    /// Queue a thumbnail request without probing or reading the path on the UI thread.
    pub fn request(&mut self, path: &Path) {
        let key = path.to_path_buf();
        if self.pending.contains(&key) {
            return;
        }
        if let Some(entry) = self.entries.get(&key)
            && (entry.texture.is_some() || entry.status.as_deref() != Some(QUEUE_FULL))
        {
            self.touch(&key);
            return;
        }
        if self.pending.len() >= MAX_OUTSTANDING {
            self.insert_entry(
                key,
                ThumbnailEntry {
                    texture: None,
                    status: Some(QUEUE_FULL.into()),
                },
            );
            return;
        }

        match self.requests.try_send(Request { path: key.clone() }) {
            Ok(()) => {
                self.insert_entry(
                    key.clone(),
                    ThumbnailEntry {
                        texture: None,
                        status: Some("Loading thumbnail…".into()),
                    },
                );
                self.touch(&key);
                self.pending.insert(key);
            }
            Err(TrySendError::Full(_)) => self.insert_entry(
                key,
                ThumbnailEntry {
                    texture: None,
                    status: Some(QUEUE_FULL.into()),
                },
            ),
            Err(TrySendError::Disconnected(_)) => self.insert_entry(
                key,
                ThumbnailEntry {
                    texture: None,
                    status: Some("Thumbnail worker stopped".into()),
                },
            ),
        }
    }

    /// Drain completed results and create GPU textures on the UI thread.
    pub fn poll(&mut self, ctx: &Context) {
        while let Ok(message) = self.results.try_recv() {
            self.pending.remove(&message.path);
            let entry = match message.result {
                Ok(frame) => {
                    let image = ColorImage::from_rgba_unmultiplied(
                        [frame.width as usize, frame.height as usize],
                        &frame.rgba,
                    );
                    ThumbnailEntry {
                        texture: Some(ctx.load_texture(
                            texture_name(&message.path),
                            image,
                            TextureOptions::LINEAR,
                        )),
                        status: None,
                    }
                }
                Err(error) => ThumbnailEntry {
                    texture: None,
                    status: Some(error),
                },
            };
            self.insert_entry(message.path, entry);
        }
    }

    pub fn texture(&self, path: &Path) -> Option<&TextureHandle> {
        self.entries.get(path)?.texture.as_ref()
    }

    pub fn images(&self) -> HashMap<PathBuf, TextureHandle> {
        self.entries
            .iter()
            .filter_map(|(p, e)| e.texture.as_ref().map(|t| (p.clone(), t.clone())))
            .collect()
    }
    pub fn status(&self, path: &Path) -> Option<&str> {
        self.entries.get(path)?.status.as_deref()
    }

    fn touch(&mut self, path: &Path) {
        if let Some(index) = self.lru.iter().position(|entry| entry == path) {
            self.lru.remove(index);
        }
        self.lru.push_front(path.to_path_buf());
    }

    fn insert_entry(&mut self, path: PathBuf, entry: ThumbnailEntry) {
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
        self.entries.insert(path.clone(), entry);
        self.touch(&path);
    }
}

impl Drop for ThumbnailState {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        // Dropping JoinHandle detaches the worker; a currently running media decode is
        // bounded by lumen-media's timeout and does not block closing the UI.
        self.worker.take();
    }
}

fn texture_name(path: &Path) -> String {
    format!("lumen-thumbnail:{}", path.display())
}
