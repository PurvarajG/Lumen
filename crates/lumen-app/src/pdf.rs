//! Shared PDF worker: each surface requests its own physical pixel dimensions.
use egui::{ColorImage, TextureHandle, TextureOptions};
use lumen_media::{Frame, ThumbnailCache};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Key {
    path: PathBuf,
    page: u32,
    width: u32,
    height: u32,
}
struct Entry {
    texture: Option<TextureHandle>,
    frame: Option<Arc<Frame>>,
    checked: Instant,
    pending: bool,
    complete: bool,
}
struct PdfState {
    tx: mpsc::SyncSender<Key>,
    rx: mpsc::Receiver<(Key, Result<Arc<Frame>, String>)>,
    entries: HashMap<Key, Entry>,
}
impl PdfState {
    fn new(ctx: egui::Context) -> Self {
        let (tx, requests) = mpsc::sync_channel::<Key>(32);
        let (results, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut cache = ThumbnailCache::new(4);
            while let Ok(key) = requests.recv() {
                let result = cache
                    .get_or_load_pdf(&key.path, key.page, key.width, key.height)
                    .map_err(|e| e.to_string());
                if results.send((key, result)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self {
            tx,
            rx,
            entries: HashMap::new(),
        }
    }
    fn texture(&mut self, ui: &egui::Ui, key: Key) -> Option<TextureHandle> {
        while let Ok((key, result)) = self.rx.try_recv() {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.pending = false;
                entry.complete = true;
                entry.checked = Instant::now();
                match result {
                    Ok(frame) => {
                        if !entry
                            .frame
                            .as_ref()
                            .is_some_and(|old| Arc::ptr_eq(old, &frame))
                        {
                            entry.texture = Some(ui.ctx().load_texture(
                                format!("pdf:{key:?}"),
                                ColorImage::from_rgba_unmultiplied(
                                    [frame.width as usize, frame.height as usize],
                                    &frame.rgba,
                                ),
                                TextureOptions::LINEAR,
                            ));
                            entry.frame = Some(frame);
                        }
                    }
                    Err(error) => {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new("pdf-error"), error));
                    }
                }
            }
        }
        let fallback = self
            .entries
            .iter()
            .filter(|(k, _)| k.path == key.path && k.page == key.page)
            .filter_map(|(_, e)| e.texture.clone())
            .next();
        if !self.entries.contains_key(&key) {
            // Keep GPU and retained RGBA memory bounded, including large output resizes.
            let bytes: usize = self
                .entries
                .values()
                .filter_map(|e| e.frame.as_ref())
                .map(|f| f.rgba.len())
                .sum();
            if (self.entries.len() >= 16 || bytes >= 256 * 1024 * 1024)
                && let Some(old) = self
                    .entries
                    .iter()
                    .filter(|(_, e)| !e.pending)
                    .min_by_key(|(_, e)| e.checked)
                    .map(|(k, _)| k.clone())
            {
                self.entries.remove(&old);
            }
            self.entries.insert(
                key.clone(),
                Entry {
                    texture: None,
                    frame: None,
                    checked: Instant::now(),
                    pending: false,
                    complete: false,
                },
            );
        }
        let entry = self.entries.get_mut(&key)?;
        if !entry.pending && !entry.complete && self.tx.try_send(key).is_ok() {
            entry.pending = true;
        }
        if entry.pending || !entry.complete {
            ui.ctx().request_repaint_after(Duration::from_millis(80));
        }
        entry.texture.clone().or(fallback)
    }
}

pub fn install(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        d.insert_temp(
            egui::Id::new("pdf-worker"),
            Arc::new(Mutex::new(PdfState::new(ctx.clone()))),
        )
    });
}

pub fn texture(ui: &egui::Ui, path: &Path, page: u32, rect: egui::Rect) -> Option<TextureHandle> {
    let state = ui
        .ctx()
        .data(|d| d.get_temp::<Arc<Mutex<PdfState>>>(egui::Id::new("pdf-worker")))?;
    let key = Key {
        path: path.to_owned(),
        page,
        width: physical_pixels(rect.width(), ui.ctx().pixels_per_point()),
        height: physical_pixels(rect.height(), ui.ctx().pixels_per_point()),
    };
    state.lock().ok()?.texture(ui, key)
}
fn physical_pixels(points: f32, scale: f32) -> u32 {
    (points * scale).round().clamp(1.0, 8192.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_surface_uses_physical_pixels_with_bounded_sizes() {
        assert_eq!(physical_pixels(960.0, 2.0), 1920);
        assert_eq!(physical_pixels(320.0, 1.5), 480);
        assert_eq!(physical_pixels(10000.0, 2.0), 8192);
        assert_eq!(physical_pixels(0.0, 2.0), 1);
    }
}
