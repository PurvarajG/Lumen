use egui::{ColorImage, TextureHandle, TextureOptions};
use lumen_core::{Background, MediaFit};
use lumen_media::{Frame, MediaPlayer, PlaybackOptions};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// The display's refresh period from egui's frame time, kept within 24..240 Hz.
pub fn refresh_period(ctx: &egui::Context) -> Duration {
    Duration::from_secs_f32(ctx.input(|i| i.stable_dt).clamp(1.0 / 240.0, 1.0 / 24.0))
}

/// ffmpeg output is opaque, so straight and premultiplied alpha are identical.
pub fn upload(texture: &mut TextureHandle, frame: Frame) {
    let image = ColorImage::from_rgba_premultiplied(
        [frame.width as usize, frame.height as usize],
        &frame.rgba,
    );
    texture.set(image, TextureOptions::LINEAR);
}

/// Decodes a still at full size (at most 64 MiB on disk, 8192 px a side) for a worker thread.
pub fn decode_image(path: &std::path::Path) -> Result<ColorImage, String> {
    let len = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if len > 64 * 1024 * 1024 {
        return Err("Image exceeds 64 MiB".into());
    }
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader.decode().map_err(|e| e.to_string())?.to_rgba8();
    Ok(ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}

/// Video decode size: the audience window's pixel size once it has opened, else the largest
/// connected display, never above 4K. The player never upscales, so a 1080p clip stays 1080p.
pub fn video_output_size(
    audience: Option<(u32, u32)>,
    displays: &[crate::displays::DisplayInfo],
) -> (u32, u32) {
    let (w, h) = audience
        .filter(|&(w, h)| w > 0 && h > 0)
        .or_else(|| {
            displays
                .iter()
                .map(|d| (d.key.width, d.key.height))
                .max_by_key(|&(w, h)| u64::from(w) * u64::from(h))
        })
        .unwrap_or((1920, 1080));
    (w.clamp(1, 3840), h.clamp(1, 2160))
}

pub enum Loaded {
    Image(u64, Result<(PathBuf, ColorImage), String>),
    Video(u64, Result<(PathBuf, MediaPlayer), String>),
}
pub struct MediaState {
    pub tx: Sender<Loaded>,
    pub rx: Receiver<Loaded>,
    pub image_path: Option<PathBuf>,
    pub image_texture: Option<TextureHandle>,
    pub video_path: Option<PathBuf>,
    pub player: Option<MediaPlayer>,
    pub status: String,
    pub pending: bool,
    request: u64,
    repaint_ctx: Option<egui::Context>,
    requested_background: Option<Background>,
    workers: Vec<JoinHandle<()>>,
}
impl Default for MediaState {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            image_path: None,
            image_texture: None,
            video_path: None,
            player: None,
            status: String::new(),
            pending: false,
            request: 0,
            repaint_ctx: None,
            requested_background: None,
            workers: Vec::new(),
        }
    }
}
impl MediaState {
    pub fn set_repaint_context(&mut self, ctx: egui::Context) {
        self.repaint_ctx = Some(ctx);
    }

    pub fn load_image(&mut self, path: PathBuf) {
        self.request = self.request.wrapping_add(1);
        let req = self.request;
        let tx = self.tx.clone();
        let repaint_ctx = self.repaint_ctx.clone();
        self.pending = true;
        self.requested_background = Some(Background::Image {
            path: path.clone(),
            fit: MediaFit::default(),
        });
        self.status = "Loading still image...".into();
        self.track_worker(thread::spawn(move || {
            let res = decode_image(&path).map(|img| (path, img));
            let _ = tx.send(Loaded::Image(req, res));
            if let Some(ctx) = repaint_ctx {
                ctx.request_repaint();
            }
        }));
    }
    pub fn load_video(&mut self, path: PathBuf, looped: bool, muted: bool, size: (u32, u32)) {
        self.request = self.request.wrapping_add(1);
        let req = self.request;
        let tx = self.tx.clone();
        let repaint_ctx = self.repaint_ctx.clone();
        self.pending = true;
        self.requested_background = Some(Background::Video {
            path: path.clone(),
            looped,
            muted,
            fit: MediaFit::default(),
        });
        self.status = "Loading video...".into();
        self.track_worker(thread::spawn(move || {
            let res = MediaPlayer::open(
                &path,
                PlaybackOptions {
                    width: size.0,
                    height: size.1,
                    fps: 60,
                    looped,
                    muted,
                },
            )
            .map(|p| (path, p))
            .map_err(|e| e.to_string());
            let _ = tx.send(Loaded::Video(req, res));
            if let Some(ctx) = repaint_ctx {
                ctx.request_repaint();
            }
        }));
    }
    /// A fit-only change repaints with the new fit but must not restart a playing video.
    pub fn matches_background(&self, background: Option<&Background>) -> bool {
        self.requested_background.as_ref().map(ignoring_fit) == background.map(ignoring_fit)
    }
    pub fn background_requires_reload(&self, background: Option<&Background>) -> bool {
        !self.matches_background(background)
    }
    pub fn restart_on_trigger(&self, background: Option<&Background>) -> bool {
        matches!(background, Some(Background::Video { .. }))
            && !self.pending
            && (self.player.is_none() || self.is_finished())
    }
    pub fn is_finished(&self) -> bool {
        self.player.as_ref().is_some_and(MediaPlayer::is_finished)
    }
    pub fn remember_background(&mut self, background: Background) {
        self.requested_background = Some(background);
    }
    /// `pull_frames` is false while the audience window pulls them on its own refresh.
    pub fn poll(&mut self, ctx: &egui::Context, pull_frames: bool) {
        self.reap_workers();
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Loaded::Image(req, Ok((p, img))) if req == self.request => {
                    self.pending = false;
                    self.image_texture =
                        Some(ctx.load_texture("lumen-background", img, TextureOptions::LINEAR));
                    self.image_path = Some(p);
                    self.status.clear();
                }
                Loaded::Image(req, Err(e)) if req == self.request => {
                    self.pending = false;
                    self.status = format!("Image error: {e}")
                }
                Loaded::Video(req, Ok((p, player))) if req == self.request => {
                    self.pending = false;
                    if let Some(old) = self.player.replace(player) {
                        self.retire_player(old);
                    }
                    // A handle that exists before the first frame, so the audience can share it.
                    self.image_texture = Some(ctx.load_texture(
                        "lumen-video",
                        ColorImage::filled([2, 2], egui::Color32::BLACK),
                        TextureOptions::LINEAR,
                    ));
                    self.image_path = None;
                    self.video_path = Some(p);
                    self.status = "Video playing".into();
                }
                Loaded::Video(req, Err(e)) if req == self.request => {
                    self.pending = false;
                    self.status = format!("Video error: {e}")
                }
                Loaded::Video(_, Ok((_, stale))) => {
                    self.retire_player(stale);
                }
                _ => {}
            }
        }
        if let Some(player) = &self.player {
            if pull_frames
                && let Some(tex) = self.image_texture.as_mut()
                && let Some(frame) = player
                    .frames()
                    .frame_at(Instant::now(), refresh_period(ctx))
            {
                upload(tex, frame);
            }
            if let Some(e) = player.error() {
                self.status = format!("Video error: {e}");
            } else if player.is_finished() {
                self.status = "Video finished".into();
            }
        }
    }
    pub fn stop(&mut self) {
        self.request = self.request.wrapping_add(1);
        self.pending = false;
        self.requested_background = None;
        if let Some(p) = self.player.take() {
            self.retire_player(p);
        }
        self.video_path = None;
        self.image_path = None;
        self.image_texture = None;
        self.status.clear();
    }

    pub fn shutdown(&mut self) {
        self.stop();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
        // A load may have completed just as shutdown began. Dropping a queued
        // result drops its unclaimed MediaPlayer and reaps its owned children.
        while let Ok(event) = self.rx.try_recv() {
            drop(event);
        }
    }

    fn retire_player(&mut self, mut player: MediaPlayer) {
        self.track_worker(thread::spawn(move || player.stop()));
    }

    fn track_worker(&mut self, worker: JoinHandle<()>) {
        self.workers.push(worker);
    }

    fn reap_workers(&mut self) {
        let mut active = Vec::new();
        for worker in self.workers.drain(..) {
            if worker.is_finished() {
                let _ = worker.join();
            } else {
                active.push(worker);
            }
        }
        self.workers = active;
    }
}

fn ignoring_fit(background: &Background) -> Background {
    let mut background = background.clone();
    background.set_fit(MediaFit::default());
    background
}

impl Drop for MediaState {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::{MediaState, video_output_size};
    use crate::displays::{DisplayInfo, DisplayKey};
    use lumen_core::{Background, MediaFit};
    use std::path::PathBuf;
    use std::{sync::mpsc, thread, time::Duration};

    fn display(width: u32, height: u32) -> DisplayInfo {
        DisplayInfo {
            key: DisplayKey {
                name: "d".into(),
                width,
                height,
                uuid: None,
            },
            x: 0,
            y: 0,
            primary: true,
        }
    }

    #[test]
    fn video_size_follows_audience_then_largest_display_and_caps_at_4k() {
        let displays = [display(1920, 1080), display(2560, 1440)];
        assert_eq!(
            video_output_size(Some((3840, 2160)), &displays),
            (3840, 2160)
        );
        assert_eq!(video_output_size(Some((1280, 720)), &displays), (1280, 720));
        assert_eq!(video_output_size(None, &displays), (2560, 1440));
        assert_eq!(video_output_size(Some((0, 0)), &displays), (2560, 1440));
        assert_eq!(
            video_output_size(Some((7680, 4320)), &displays),
            (3840, 2160)
        );
        assert_eq!(video_output_size(None, &[]), (1920, 1080));
    }

    #[test]
    fn video_identity_includes_mute_and_loop_options() {
        let mut media = MediaState::default();
        let first = Background::Video {
            path: PathBuf::from("shared.mp4"),
            looped: true,
            muted: false,
            fit: MediaFit::Fit,
        };
        media.requested_background = Some(first.clone());
        assert!(media.matches_background(Some(&first)));
        assert!(!media.background_requires_reload(Some(&first)));
        assert!(!media.matches_background(Some(&Background::Video {
            path: PathBuf::from("shared.mp4"),
            looped: false,
            muted: false,
            fit: MediaFit::Fit,
        })));
        assert!(!media.matches_background(Some(&Background::Video {
            path: PathBuf::from("shared.mp4"),
            looped: true,
            muted: true,
            fit: MediaFit::Fit,
        })));
        assert!(media.background_requires_reload(Some(&Background::Video {
            path: PathBuf::from("shared.mp4"),
            looped: false,
            muted: false,
            fit: MediaFit::Fit,
        })));
    }

    #[test]
    fn a_fit_change_does_not_reload_playing_media() {
        let mut media = MediaState::default();
        let video = |fit| Background::Video {
            path: PathBuf::from("cue.mp4"),
            looped: true,
            muted: true,
            fit,
        };
        media.remember_background(video(MediaFit::Fit));
        assert!(!media.background_requires_reload(Some(&video(MediaFit::Fill))));
        assert!(media.background_requires_reload(Some(&Background::Video {
            path: PathBuf::from("other.mp4"),
            looped: true,
            muted: true,
            fit: MediaFit::Fit,
        })));
    }

    #[test]
    fn explicit_cue_restarts_stopped_video_but_preserves_pending_load() {
        let mut media = MediaState::default();
        let video = Background::Video {
            path: PathBuf::from("cue.mp4"),
            looped: false,
            muted: true,
            fit: MediaFit::Fit,
        };
        media.remember_background(video.clone());
        assert!(
            !media.background_requires_reload(Some(&video)),
            "routine reconciliation must preserve stopped/end state"
        );
        assert!(
            media.restart_on_trigger(Some(&video)),
            "explicit trigger must restart a stopped clip"
        );
        media.pending = true;
        assert!(
            !media.restart_on_trigger(Some(&video)),
            "do not replace an in-flight decoder on every trigger"
        );
        assert!(!media.restart_on_trigger(None));
    }

    #[test]
    fn clear_all_requires_stopping_current_playback() {
        let mut media = MediaState::default();
        media.remember_background(Background::Video {
            path: PathBuf::from("live.mp4"),
            looped: true,
            muted: false,
            fit: MediaFit::Fit,
        });
        assert!(media.background_requires_reload(None));
    }

    #[test]
    fn shutdown_joins_tracked_media_workers() {
        let (tx, rx) = mpsc::channel();
        {
            let mut media = MediaState::default();
            media.track_worker(thread::spawn(move || {
                thread::sleep(Duration::from_millis(10));
                let _ = tx.send(());
            }));
        }
        assert!(rx.try_recv().is_ok());
    }
}
