use crate::render::transition_millis;
use crate::{
    file::{self, FileAction, FileResult},
    media::MediaState,
};
use eframe::egui::{self, Context, Key, TextureHandle, ViewportCommand, ViewportId};
use lumen_core::{Background, Document, LiveSlide, OutputState, PlaylistItem, Project, Runtime};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, Sender},
    },
    time::Instant,
};
use uuid::Uuid;

const MEDIA_WAIT: std::time::Duration = std::time::Duration::from_millis(1500);

#[derive(Clone, Default)]
pub struct Published {
    pub contents: Option<Project>,
    pub clocks: Runtime,
    pub stage: crate::settings::StageOptions,
    pub transition: Option<crate::render::OutputTransition>,
    pub images: std::collections::HashMap<PathBuf, TextureHandle>,
    pub now: u32,
    pub clock: String,
    pub project: ProjectView,
    pub output: OutputState,
    pub runtime: RuntimeView,
    pub media: Option<TextureHandle>,
    /// Set while the audience window is open: it pulls video frames on its own refresh.
    pub video: Option<lumen_media::FrameSource>,
    pub media_animating: bool,
}
#[derive(Clone, Default)]
pub struct ProjectView {
    pub next: Option<LiveSlide>,
}
#[derive(Clone, Default)]
pub struct RuntimeView {
    pub timers: Vec<(String, String, bool)>,
    pub message: String,
}
#[derive(Clone, Copy)]
enum Pending {
    New,
    Open,
    Close,
}
pub struct LumenApp {
    pub project: Project,
    pub text_images: crate::text::SharedText,
    displays: Vec<crate::displays::DisplayInfo>,
    identify_until: Option<Instant>,
    last_display_scan: Instant,
    pub runtime: Runtime,
    pub selected_doc: Uuid,
    pub selected_slide: Option<Uuid>,
    pub dirty: bool,
    pub path: Option<PathBuf>,
    pub error: String,
    /// Set by Open Recent; consumed when the pending open begins.
    open_path: Option<PathBuf>,
    /// Media the open show refers to that is not on disk, as the worker last counted them.
    missing_media: Vec<PathBuf>,
    pub status: String,
    pub media: MediaState,
    pub(super) deck: crate::deck::DeckController,
    pub show_audience: bool,
    pub show_stage: bool,
    pub fullscreen_audience: bool,
    pub fullscreen_stage: bool,
    /// An output window existed last frame, so fullscreen may now apply on its own display.
    audience_placed: bool,
    stage_placed: bool,
    audience_target: Option<crate::displays::DisplayInfo>,
    stage_target: Option<crate::displays::DisplayInfo>,
    pub published: Arc<Mutex<Published>>,
    tx: Sender<FileResult>,
    rx: Receiver<FileResult>,
    pending: Option<Pending>,
    pending_id: Option<u64>,
    active_save: Option<(u64, Option<u64>)>,
    active_open: Option<u64>,
    operation_counter: u64,
    modal: bool,
    last_tick: Instant,
    pub(super) stage_draft: String,
    show_help: bool,
    pub(super) confirm_delete: Option<Uuid>,
    media_target: Option<(Uuid, bool)>,
    close_approved: bool,
    allow_close: bool,
    smoke_test: bool,
    smoke_mode: bool,
    audience_fullscreen_state: Arc<AtomicBool>,
    /// Audience window size in physical pixels (width << 32 | height); 0 until it has opened.
    audience_pixels: Arc<AtomicU64>,
    stage_fullscreen_state: Arc<AtomicBool>,
    audience_close_requested: Arc<AtomicBool>,
    stage_close_requested: Arc<AtomicBool>,
    repaint_ctx: Context,
    native_quit_pending: bool,
    pub(super) present: bool,
    /// Set when going live armed the next cue; becomes `scroll_to_armed` for exactly one frame.
    arm_scroll: bool,
    pub(super) scroll_to_armed: bool,
    pub(super) search: String,
    pub(super) focus_search: bool,
    pub(super) library_tab: usize,
    pub(super) books: crate::books::Books,
    pub(super) tool_tab: usize,
    pub(super) settings: crate::settings::Settings,
    pub(super) docks: crate::dock::Docks,
    settings_notice: Option<String>,
    show_settings: bool,
    remote: Option<crate::remote::RemoteManager>,
    remote_status: lumen_control::StatusHandle,
    remote_error: String,
    pub(super) thumbnails: crate::thumbnails::ThumbnailState,
    full_images: crate::fullres::FullImages,
    pub(super) audio: crate::audio::AudioState,
    transition: Option<crate::render::OutputTransition>,
    history: lumen_core::history::History<Project>,
    edit_focus: Option<egui::Id>,
    history_applied: bool,
    saved_project: Project,
    recovered: Option<crate::recovery::Recovered>,
    last_autosave: Instant,
    recovery_busy: bool,
    audio_seen_serial: u64,
    launched: Instant,
    active_import: Option<u64>,
    adding: Vec<u64>,
    pub(super) add_progress: Option<(usize, usize)>,
    pub(super) add_detail: Option<String>,
    pub(super) add_notice: Option<String>,
    active_recovery: u64,
    project_epoch: u64,
    saved_operation: u64,
}
impl LumenApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        project: Project,
        path: Option<PathBuf>,
        smoke: bool,
    ) -> Self {
        #[cfg(target_os = "macos")]
        if let Err(error) = crate::macos_quit::install(cc.egui_ctx.clone()) {
            panic!("Lumen could not install its native unsaved-quit guard: {error}");
        }
        crate::pdf::install(&cc.egui_ctx);
        let selected_doc = project
            .documents
            .first()
            .map(|d| d.id)
            .unwrap_or_else(Uuid::new_v4);
        let (tx, rx) = file::channel();
        let mut media = MediaState::default();
        media.set_repaint_context(cc.egui_ctx.clone());
        let loaded = crate::settings::settings_path().map(|p| crate::settings::Settings::load(&p));
        let mut settings = loaded
            .as_ref()
            .map(|v| v.settings.clone())
            .unwrap_or_default();
        let (docks, layout_notice) = crate::dock::Docks::restore(&mut settings.layouts);
        let remote_status = lumen_control::StatusHandle::default();
        let (remote, remote_error) =
            match crate::remote::RemoteManager::start(&settings, remote_status.clone()) {
                Ok(remote) => (Some(remote), String::new()),
                Err(e) => (None, e),
            };
        let saved_project = project.clone();
        let mut app = Self {
            text_images: crate::text::TextImages::install(&cc.egui_ctx),
            displays: crate::displays::enumerate_displays(),
            identify_until: None,
            last_display_scan: Instant::now(),
            project,
            runtime: Runtime::default(),
            selected_doc,
            selected_slide: None,
            dirty: false,
            path,
            error: String::new(),
            open_path: None,
            missing_media: Vec::new(),
            status: "Ready".into(),
            media,
            deck: crate::deck::DeckController::new(),
            show_audience: smoke,
            show_stage: smoke,
            fullscreen_audience: settings.audience_fullscreen,
            fullscreen_stage: settings.stage_fullscreen,
            audience_placed: false,
            stage_placed: false,
            audience_target: None,
            stage_target: None,
            published: Arc::new(Mutex::new(Published::default())),
            tx,
            rx,
            pending: None,
            pending_id: None,
            active_save: None,
            active_open: None,
            operation_counter: 0,
            modal: false,
            last_tick: Instant::now(),
            stage_draft: String::new(),
            show_help: false,
            confirm_delete: None,
            media_target: None,
            close_approved: false,
            allow_close: false,
            smoke_test: smoke,
            smoke_mode: smoke,
            audience_fullscreen_state: Arc::new(AtomicBool::new(settings.audience_fullscreen)),
            audience_pixels: Arc::new(AtomicU64::new(0)),
            stage_fullscreen_state: Arc::new(AtomicBool::new(settings.stage_fullscreen)),
            audience_close_requested: Arc::new(AtomicBool::new(false)),
            stage_close_requested: Arc::new(AtomicBool::new(false)),
            repaint_ctx: cc.egui_ctx.clone(),
            native_quit_pending: false,
            present: true,
            arm_scroll: false,
            scroll_to_armed: false,
            search: String::new(),
            focus_search: false,
            library_tab: 0,
            books: crate::books::Books::new(&cc.egui_ctx, &settings.book_downloads),
            tool_tab: 0,
            settings,
            settings_notice: loaded.and_then(|v| v.notice).or(layout_notice),
            docks,
            show_settings: false,
            remote,
            remote_status,
            remote_error,
            thumbnails: crate::thumbnails::ThumbnailState::new(cc.egui_ctx.clone()),
            full_images: crate::fullres::FullImages::new(cc.egui_ctx.clone()),
            audio: crate::audio::AudioState::default(),
            transition: None,
            history: Default::default(),
            edit_focus: None,
            history_applied: false,
            saved_project,
            recovered: None,
            last_autosave: Instant::now(),
            recovery_busy: false,
            audio_seen_serial: 0,
            launched: Instant::now(),
            active_import: None,
            adding: Vec::new(),
            add_progress: None,
            add_detail: None,
            add_notice: None,
            active_recovery: 0,
            project_epoch: 0,
            saved_operation: 0,
        };
        app.ensure_selection();
        if let (Some(path), false) = (app.path.clone(), smoke) {
            app.remember_recent(&path);
            app.check_media();
        }
        file::launch(
            FileAction::Recovery(0, app.path.clone()),
            app.tx.clone(),
            cc.egui_ctx.clone(),
        );
        if smoke {
            eprintln!("LUMEN SMOKE: audience viewport requested");
            eprintln!("LUMEN SMOKE: stage viewport requested");
            if let Some(live) = app.selected_live() {
                app.trigger(live.document_id, live.slide.id);
            }
        }
        crate::style::install(&cc.egui_ctx);
        app
    }
    fn doc(&self) -> Option<&Document> {
        self.project.document(self.selected_doc)
    }
    fn ensure_selection(&mut self) {
        if self.project.document(self.selected_doc).is_none() {
            self.selected_doc = self
                .project
                .documents
                .first()
                .map(|d| d.id)
                .unwrap_or_else(Uuid::new_v4);
        }
        if let Some(doc) = self.doc()
            && !doc.slides.iter().any(|s| Some(s.id) == self.selected_slide)
        {
            self.selected_slide = doc.slides.first().map(|s| s.id);
        }
    }
    pub(super) fn selected_live(&self) -> Option<LiveSlide> {
        let d = self.doc()?;
        let s = d
            .slides
            .iter()
            .find(|s| Some(s.id) == self.selected_slide)?;
        let t = self.project.theme(s.theme_id)?.clone();
        Some(LiveSlide {
            document_id: d.id,
            document_name: d.name.clone(),
            slide: s.clone(),
            theme: t,
        })
    }
    pub(super) fn mark(&mut self) {
        self.dirty = true;
        self.status = "Unsaved changes".into();
    }
    pub fn choose_media(&mut self, slide: Uuid, video: bool) {
        self.media_target = Some((slide, video));
        self.modal = true;
        file::launch(
            FileAction::PickMedia(video),
            self.tx.clone(),
            self.repaint_ctx.clone(),
        );
        self.status = if video {
            "Choose video..."
        } else {
            "Choose image..."
        }
        .into();
    }
    pub(super) fn trigger(&mut self, doc: Uuid, slide: Uuid) {
        self.run_trigger(|runtime, project| runtime.trigger(project, doc, slide).map(|()| true));
    }
    fn run_trigger(
        &mut self,
        f: impl FnOnce(&mut Runtime, &Project) -> Result<bool, lumen_core::CoreError>,
    ) {
        let previous = self.runtime.output.clone();
        let previous_media = self.media.image_texture.clone();
        match f(&mut self.runtime, &self.project) {
            Ok(false) => {}
            Ok(true) => {
                self.after_trigger(previous, previous_media);
                self.status = "Cue is live".into();
                if self.smoke_test {
                    eprintln!("LUMEN SMOKE: first cue triggered");
                    self.smoke_test = false;
                }
                if self.media.restart_on_trigger(
                    self.runtime
                        .output
                        .live
                        .as_ref()
                        .map(|l| &l.slide.background),
                ) {
                    self.media.stop();
                }
                self.sync_media();
                self.arm_next();
            }
            Err(e) => self.error = e.to_string(),
        }
    }
    /// Sends the armed (selected) cue, the same action for the button, Space and Return.
    pub(super) fn send_armed(&mut self) {
        let ids = |l: &LiveSlide| (l.document_id, l.slide.id);
        let armed = self.selected_live().as_ref().map(ids);
        let next = self.runtime.peek_next(&self.project).as_ref().map(ids);
        // End of the order: the live cue stays armed, and Next must not restart it.
        if next.is_none() && armed.is_some() && armed == self.runtime.output.live.as_ref().map(ids)
        {
            return;
        }
        match armed {
            // `navigate` takes the exact next occurrence; `trigger` by id could re-pick a repeated item.
            Some((doc, slide))
                if armed != next
                    && self.runtime.output.deck.is_none()
                    && !crate::books::has_preview(self) =>
            {
                self.trigger(doc, slide)
            }
            _ => self.navigate(true),
        }
    }
    /// After a cue goes live in Present, select the one after it so the next send is the next cue.
    fn arm_next(&mut self) {
        let Some((doc, slide)) = armed_after_send(&self.runtime, &self.project, self.present)
        else {
            return;
        };
        // A book preview stays open: it is independent of the selection and closes when sent.
        self.selected_doc = doc;
        self.selected_slide = Some(slide);
        self.arm_scroll = true;
        self.repaint_ctx.request_repaint();
    }
    pub(super) fn sync_media(&mut self) {
        if self.runtime.output.media_cleared {
            self.media.stop();
            return;
        }
        let background = self.runtime.output.effective_background().cloned();
        if !self.media.background_requires_reload(background.as_ref()) {
            return;
        }
        match background {
            Some(Background::Video {
                path,
                looped,
                muted,
                ..
            }) => {
                self.media.stop();
                let packed = self.audience_pixels.load(Ordering::Relaxed);
                let audience = Some(((packed >> 32) as u32, packed as u32));
                let size = crate::media::video_output_size(audience, &self.displays);
                self.media.load_video(path, looped, muted, size)
            }
            Some(Background::Image { path, .. }) => {
                self.media.stop();
                self.media.load_image(path)
            }
            Some(background @ (Background::Solid(_) | Background::Pdf { .. })) => {
                self.media.stop();
                self.media.remember_background(background);
            }
            None => self.media.stop(),
        }
    }
    fn publish(&mut self) {
        let now = self.seconds_of_day();
        let snapshot = lumen_control::StatusSnapshot {
            project: self.project.name.clone(),
            live_index: self.runtime.live_cue_index(&self.project),
            live_text: self
                .runtime
                .output
                .live
                .as_ref()
                .map(|l| l.slide.text.clone())
                .unwrap_or_default(),
            next_text: self
                .runtime
                .peek_next(&self.project)
                .map(|l| l.slide.text)
                .unwrap_or_default(),
            blackout: self.runtime.output.blackout,
            timers: self
                .project
                .timers
                .iter()
                .map(|t| (t.name.clone(), self.runtime.timer_display(t, now)))
                .collect(),
        };
        if self.remote_status.get() != snapshot {
            self.remote_status.set(snapshot);
        }
        let view = Published {
            contents: Some(self.project.clone()),
            clocks: self.runtime.clone(),
            stage: self.settings.stage.clone(),
            transition: self.transition.clone(),
            images: self.output_images(),
            now,
            clock: chrono::Local::now().format("%H:%M:%S").to_string(),
            project: ProjectView {
                next: self.runtime.peek_next(&self.project),
            },
            output: self.runtime.output.clone(),
            runtime: RuntimeView {
                timers: self
                    .project
                    .timers
                    .iter()
                    .map(|timer| {
                        let running = self
                            .runtime
                            .timer_states
                            .get(&timer.id)
                            .is_some_and(|state| state.running);
                        (
                            timer.name.clone(),
                            self.runtime.timer_display(timer, now),
                            running,
                        )
                    })
                    .collect(),
                message: self.runtime.output.stage_message.clone(),
            },
            media: self.media.image_texture.clone(),
            video: self
                .media
                .player
                .as_ref()
                .filter(|_| self.audience_open())
                .map(lumen_media::MediaPlayer::frames),
            media_animating: self
                .media
                .player
                .as_ref()
                .is_some_and(|player| !player.is_finished() && !player.is_paused()),
        };
        if let Ok(mut p) = self.published.lock() {
            *p = view;
        }
    }
    fn request(&mut self, p: Pending) {
        self.operation_counter = self.operation_counter.wrapping_add(1);
        self.pending_id = Some(self.operation_counter);
        if self.dirty {
            self.pending = Some(p);
            self.modal = true;
            if let Some((save_id, _)) = self.active_save {
                self.active_save = Some((save_id, self.pending_id));
            }
        } else {
            self.begin(p)
        }
    }
    fn begin(&mut self, p: Pending) {
        self.pending = Some(p);
        match p {
            Pending::Open => {
                let id = self.pending_id.unwrap_or_else(|| {
                    self.operation_counter = self.operation_counter.wrapping_add(1);
                    self.pending_id = Some(self.operation_counter);
                    self.operation_counter
                });
                self.active_open = Some(id);
                self.modal = true;
                file::launch(
                    FileAction::Open(id, self.open_path.take()),
                    self.tx.clone(),
                    self.repaint_ctx.clone(),
                );
                self.status = "Choose a project...".into();
            }
            Pending::New => {
                self.missing_media.clear();
                self.media.stop();
                self.active_import = None;
                self.reset_adds();
                self.project_epoch = self.project_epoch.wrapping_add(1);
                self.active_recovery = u64::MAX;
                self.recovered = None;
                self.audio.stop();
                self.transition = None;
                self.history.clear();
                self.edit_focus = None;
                self.history_applied = true;
                self.project = Project::demo();
                self.project.name = "Untitled show".into();
                self.project.documents.clear();
                self.project.playlist.clear();
                self.runtime = Runtime::default();
                self.selected_doc = Uuid::new_v4();
                self.selected_slide = None;
                self.path = None;
                self.saved_project = self.project.clone();
                self.dirty = true;
                self.pending = None;
                self.pending_id = None;
                self.modal = false;
                self.status = "New show".into();
            }
            Pending::Close => {
                self.pending = None;
                self.pending_id = None;
                self.modal = false;
                if self.native_quit_pending {
                    self.reply_native_quit(true);
                } else {
                    self.close_approved = true;
                }
            }
        }
    }
    fn reply_native_quit(&mut self, should_terminate: bool) {
        if !self.native_quit_pending {
            return;
        }
        self.native_quit_pending = false;
        #[cfg(target_os = "macos")]
        if let Err(error) = crate::macos_quit::cancel_deferred_termination() {
            self.error = error;
            self.native_quit_pending = true;
            return;
        }
        if should_terminate {
            self.close_approved = true;
        }
    }
    fn save(&mut self, save_as: bool) {
        if self.active_save.is_some() {
            return;
        }
        self.operation_counter = self.operation_counter.wrapping_add(1);
        let save_id = self.operation_counter;
        self.active_save = Some((save_id, self.pending_id));
        if let Some(path) = if save_as || self.path.is_none() {
            None
        } else {
            self.path.clone()
        } {
            file::launch(
                FileAction::Save(save_id, path, Box::new(self.project.clone())),
                self.tx.clone(),
                self.repaint_ctx.clone(),
            );
            return;
        }
        let tx = self.tx.clone();
        let snapshot = self.project.clone();
        let save_snapshot = snapshot.clone();
        let repaint_ctx = self.repaint_ctx.clone();
        self.status = "Choose where to save...".into();
        std::thread::spawn(move || {
            let result = rfd::FileDialog::new()
                .set_file_name("Show.lumen.json")
                .add_filter("Lumen project", &["json"])
                .save_file()
                .map(|p| {
                    lumen_core::save_project(&p, &snapshot)
                        .map(|_| (p, save_snapshot))
                        .map_err(|e| e.to_string())
                });
            let _ = tx.send(FileResult::Save(
                save_id,
                result.unwrap_or_else(|| Err("Save cancelled".into())),
            ));
            repaint_ctx.request_repaint();
        });
    }
    fn save_with_media(&mut self) {
        if self.active_save.is_some() {
            return;
        }
        self.operation_counter = self.operation_counter.wrapping_add(1);
        self.active_save = Some((self.operation_counter, self.pending_id));
        self.status = "Choose a name for the show folder...".into();
        file::launch(
            FileAction::SaveWithMedia(self.operation_counter, Box::new(self.project.clone())),
            self.tx.clone(),
            self.repaint_ctx.clone(),
        );
    }
    /// Counts the open show's missing files on a worker; the answer lands in `missing_media`.
    fn check_media(&mut self) {
        file::launch(
            FileAction::CheckMedia(self.project_epoch, Box::new(self.project.clone())),
            self.tx.clone(),
            self.repaint_ctx.clone(),
        );
    }
    fn remember_recent(&mut self, path: &std::path::Path) {
        // `--project show.json` is relative to a folder the next launch will not be in.
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.settings.remember_project(&path);
        self.persist_settings();
    }
    fn missing_media_banner(&mut self, ctx: &Context) {
        if self.missing_media.is_empty() {
            return;
        }
        let count = self.missing_media.len();
        let (mut locate, mut dismiss) = (false, false);
        egui::TopBottomPanel::top("missing-media-banner").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    crate::style::WARN,
                    format!(
                        "{count} media file{} {} missing —",
                        if count == 1 { "" } else { "s" },
                        if count == 1 { "is" } else { "are" }
                    ),
                )
                .on_hover_text(
                    self.missing_media
                        .iter()
                        .take(12)
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                locate = ui.button("Locate folder…").clicked();
                dismiss = ui.button("Dismiss").clicked();
            });
        });
        if locate {
            file::launch(
                FileAction::PickRelinkFolder(self.project_epoch),
                self.tx.clone(),
                self.repaint_ctx.clone(),
            );
        }
        if dismiss {
            self.missing_media.clear();
        }
    }
    fn poll_files(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            match result {
                FileResult::Open(id, result) if self.active_open == Some(id) => {
                    self.active_open = None;
                    self.modal = false;
                    self.pending = None;
                    self.pending_id = None;
                    let Some(result) = result else {
                        self.status = "Open cancelled".into();
                        continue;
                    };
                    let (p, project) = match result {
                        Ok(value) => value,
                        Err(error) => {
                            self.error = error;
                            self.status = "Open failed".into();
                            continue;
                        }
                    };
                    self.project = project;
                    self.saved_project = self.project.clone();
                    self.history.clear();
                    self.edit_focus = None;
                    self.history_applied = true;
                    self.active_import = None;
                    self.reset_adds();
                    self.project_epoch = self.project_epoch.wrapping_add(1);
                    self.audio.stop();
                    self.transition = None;
                    self.recovered = None;
                    self.remember_recent(&p);
                    self.check_media();
                    self.path = Some(p);
                    self.operation_counter = self.operation_counter.wrapping_add(1);
                    self.active_recovery = self.operation_counter;
                    file::launch(
                        FileAction::Recovery(self.active_recovery, self.path.clone()),
                        self.tx.clone(),
                        self.repaint_ctx.clone(),
                    );
                    self.runtime = Runtime::default();
                    self.selected_doc = self
                        .project
                        .documents
                        .first()
                        .map(|d| d.id)
                        .unwrap_or_else(Uuid::new_v4);
                    self.selected_slide = None;
                    self.dirty = false;
                    self.pending = None;
                    self.ensure_selection();
                    self.media.stop();
                    self.status = "Project opened".into()
                }
                FileResult::Save(id, result)
                    if self.active_save.is_some_and(|(active, _)| active == id) =>
                {
                    let Some((_, associated_pending)) = self.active_save.take() else {
                        continue;
                    };
                    match result {
                        Err(error) if error == "Save cancelled" => {
                            let decision = save_decision(
                                Some((id, associated_pending)),
                                id,
                                SaveOutcome::Cancelled,
                                false,
                                self.pending_id,
                            );
                            debug_assert_eq!(decision, SaveDecision::Stay);
                            self.status = "Save cancelled; choose Save, Discard, or Cancel".into();
                        }
                        Err(error) => {
                            let decision = save_decision(
                                Some((id, associated_pending)),
                                id,
                                SaveOutcome::Failed,
                                false,
                                self.pending_id,
                            );
                            debug_assert_eq!(decision, SaveDecision::Stay);
                            self.error = error;
                            self.status = "Save failed; the pending action is waiting".into();
                        }
                        Ok((path, snapshot)) => {
                            self.saved_operation = id;
                            let old_recovery = crate::recovery::recovery_path(self.path.as_deref());
                            self.remember_recent(&path);
                            self.path = Some(path);
                            self.saved_project = snapshot.clone();
                            if !self.recovery_busy
                                && let Some(path) = old_recovery
                            {
                                let _ = std::fs::remove_file(path);
                            }
                            self.dirty = self.project != snapshot;
                            self.remove_clean_recovery();
                            self.status = if self.dirty {
                                "Saved snapshot; newer edits remain"
                            } else {
                                "Saved"
                            }
                            .into();
                            let decision = save_decision(
                                Some((id, associated_pending)),
                                id,
                                SaveOutcome::Succeeded,
                                !self.dirty,
                                self.pending_id,
                            );
                            if decision == SaveDecision::Continue
                                && let Some(pending) = self.pending.take()
                            {
                                self.pending_id = None;
                                self.modal = false;
                                self.begin(pending);
                            } else if associated_pending.is_some() && self.pending.is_some() {
                                self.modal = true;
                                self.status =
                                    "Project changed while saving; review the pending action"
                                        .into();
                            }
                        }
                    }
                }
                FileResult::MediaProgress(id, done, total)
                    if self.active_save.is_some_and(|(active, _)| active == id) =>
                {
                    self.status = format!("Copying media {} of {total}…", done + 1);
                }
                FileResult::MediaCopied(id, original, rewritten, note)
                    if self.active_save.is_some_and(|(active, _)| active == id) =>
                {
                    // The show now points at its copies, unless it was edited while copying.
                    if self.project == *original {
                        self.project = *rewritten;
                        self.history.clear();
                        self.edit_focus = None;
                        self.history_applied = true;
                        self.missing_media.clear();
                    }
                    if !note.is_empty() {
                        self.error = note;
                    }
                }
                FileResult::MediaProgress(..) | FileResult::MediaCopied(..) => {}
                FileResult::MissingMedia(epoch, missing) => {
                    if epoch == self.project_epoch {
                        self.missing_media = missing;
                    }
                }
                FileResult::RelinkIndex(epoch, index) => {
                    if let (Some(index), true) = (index, epoch == self.project_epoch) {
                        let (found, missing) =
                            lumen_core::portable::relink(&mut self.project, &index);
                        self.status = format!(
                            "Relinked {found} media file{}; {missing} still missing",
                            if found == 1 { "" } else { "s" }
                        );
                        self.check_media();
                    }
                }
                FileResult::Open(_, _) | FileResult::Save(_, _) => {}
                FileResult::Import(id, _) if self.active_import != Some(id) => {}
                FileResult::Import(_, None) => {
                    self.active_import = None;
                    self.status = "Import cancelled".into()
                }
                FileResult::Import(_, Some(Err(e))) => {
                    self.active_import = None;
                    self.error = e
                }
                FileResult::Import(_, Some(Ok(report))) => {
                    self.active_import = None;
                    if self
                        .project
                        .themes
                        .iter()
                        .any(|t| report.document.slides.iter().all(|s| s.theme_id == t.id))
                    {
                        self.status = if report.warnings.is_empty() {
                            "Presentation imported".into()
                        } else {
                            report.warnings.join("; ")
                        };
                        let doc = report.document;
                        self.project.playlist.push(PlaylistItem {
                            id: Uuid::new_v4(),
                            document_id: doc.id,
                        });
                        self.selected_doc = doc.id;
                        self.selected_slide = doc.slides.first().map(|s| s.id);
                        self.project.documents.push(doc);
                        self.dirty = true;
                    }
                }
                FileResult::Export(result) => match result {
                    Ok(()) => self.status = "OpenLyrics exported".into(),
                    Err(e) => self.error = e,
                },
                FileResult::Recovery(id, path, result)
                    if path == self.path && id == self.active_recovery =>
                {
                    match result {
                        Ok(value) => self.recovered = value,
                        Err(e) => self.error = e,
                    }
                }
                FileResult::Recovery(_, _, _) => {}
                FileResult::Autosave(id, epoch, path, result) => {
                    self.recovery_busy = false;
                    match result {
                        Ok(()) => {
                            if recovery_is_obsolete(
                                epoch,
                                self.project_epoch,
                                id,
                                self.saved_operation,
                                !self.dirty,
                            ) {
                                let _ = std::fs::remove_file(path);
                                self.last_autosave =
                                    Instant::now() - std::time::Duration::from_secs(30);
                            }
                        }
                        Err(e) => self.error = format!("Couldn't save recovery: {e}"),
                    }
                }
                FileResult::AddProgress(id, done, total) if self.adding.contains(&id) => {
                    self.add_progress = Some((done, total));
                    self.add_detail = None;
                }
                FileResult::DeckProgress(id, message) if self.adding.contains(&id) => {
                    self.add_detail = Some(message);
                }
                FileResult::DeckProgress(id, message) if self.active_import == Some(id) => {
                    self.status = message;
                }
                FileResult::Added(id, batch) if self.adding.contains(&id) => {
                    self.adding.retain(|a| *a != id);
                    self.add_progress = None;
                    self.add_detail = None;
                    match batch {
                        Some(batch) => self.finish_add(batch),
                        None => self.status = "Add files cancelled".into(),
                    }
                }
                FileResult::AddProgress(..)
                | FileResult::DeckProgress(..)
                | FileResult::Added(..) => {}
                FileResult::PickMedia(None, _) => {
                    self.media_target = None;
                    self.modal = false;
                    self.status = "Media selection cancelled".into();
                }
                FileResult::PickMedia(Some(path), video) => {
                    self.modal = false;
                    if let Some((slide_id, _)) = self.media_target.take()
                        && let Some(slide) = self
                            .project
                            .documents
                            .iter_mut()
                            .flat_map(|d| d.slides.iter_mut())
                            .find(|s| s.id == slide_id)
                    {
                        slide.background = if video {
                            Background::Video {
                                path: path.clone(),
                                looped: true,
                                muted: true,
                                fit: lumen_core::MediaFit::default(),
                            }
                        } else {
                            Background::Image {
                                path: path.clone(),
                                fit: lumen_core::MediaFit::default(),
                            }
                        };
                        self.mark();
                    }
                }
            }
        }
    }
    fn shortcuts(&mut self, ctx: &Context) {
        crate::books::shortcuts(self, ctx, self.modal);
        let typing = ctx.wants_keyboard_input() || self.modal;
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                repeat: false,
                ..
            } = event
            else {
                continue;
            };
            let Some(shortcut) = crate::shortcuts::map(
                key,
                modifiers.command || modifiers.ctrl,
                modifiers.shift,
                typing,
            ) else {
                continue;
            };
            use crate::shortcuts::Shortcut;
            match shortcut {
                Shortcut::Next if self.present => self.send_armed(),
                Shortcut::Next => self.navigate(true),
                Shortcut::Previous => self.navigate(false),
                Shortcut::Trigger if self.present => self.send_armed(),
                Shortcut::Trigger => {
                    if let Some(l) = self.selected_live() {
                        self.trigger(l.document_id, l.slide.id)
                    }
                }
                Shortcut::Clear(n) => self.clear_layer(n),
                Shortcut::Blackout => {
                    self.transition = None;
                    self.runtime.output.blackout = !self.runtime.output.blackout;
                }
                Shortcut::Search => self.focus_search = true,
                Shortcut::Undo => self.undo(false),
                Shortcut::Redo => self.undo(true),
                Shortcut::Save(as_new) => self.save(as_new),
                Shortcut::Open => self.request(Pending::Open),
                Shortcut::New => self.request(Pending::New),
                Shortcut::MoveItem(step) if self.running_order_focused() => {
                    self.move_selected(step)
                }
                Shortcut::RemoveItem if self.running_order_focused() => self.remove_selected(),
                Shortcut::MoveItem(_) | Shortcut::RemoveItem => {}
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaveDecision {
    Ignore,
    Stay,
    Continue,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaveOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

fn save_decision(
    active: Option<(u64, Option<u64>)>,
    completion: u64,
    outcome: SaveOutcome,
    snapshot_is_current: bool,
    pending: Option<u64>,
) -> SaveDecision {
    let Some((active_id, associated_pending)) = active else {
        return SaveDecision::Ignore;
    };
    if active_id != completion {
        return SaveDecision::Ignore;
    }
    if outcome != SaveOutcome::Succeeded || !snapshot_is_current {
        return SaveDecision::Stay;
    }
    if associated_pending.is_some() && associated_pending == pending {
        SaveDecision::Continue
    } else {
        SaveDecision::Stay
    }
}

impl eframe::App for LumenApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        if let Ok(mut cache) = self.text_images.lock() {
            cache.begin_frame();
        }
        let _ = frame;
        self.scroll_to_armed = std::mem::take(&mut self.arm_scroll);
        let closing = ctx.input(|i| i.viewport().close_requested());
        if closing && self.dirty && !self.allow_close {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            if self.pending.is_none() {
                self.operation_counter = self.operation_counter.wrapping_add(1);
                let pending_id = self.operation_counter;
                self.pending_id = Some(pending_id);
                self.pending = Some(Pending::Close);
                if let Some((save_id, _)) = self.active_save {
                    self.active_save = Some((save_id, Some(pending_id)));
                }
            }
            self.modal = true;
        }
        if crate::quit::take_request() {
            // Resolve AppKit's deferred request without terminating inside a
            // redraw. Native pickers already own a pending operation; keep it.
            if self.active_open.is_some() || self.media_target.is_some() {
                #[cfg(target_os = "macos")]
                if let Err(error) = crate::macos_quit::cancel_deferred_termination() {
                    self.error = error;
                }
            } else {
                self.native_quit_pending = true;
                self.request(Pending::Close);
            }
        }
        if self.smoke_mode
            && std::env::var_os("LUMEN_SMOKE_HOLD").is_none()
            && self.launched.elapsed() > std::time::Duration::from_secs(3)
        {
            self.allow_close = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if self.smoke_mode {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let before = self.project.clone();
        self.history_applied = false;
        self.poll_files();
        self.handle_drops(ctx);
        crate::books::poll(self, ctx);
        self.thumbnails.poll(ctx);
        let events = self
            .remote
            .as_ref()
            .map(|r| r.drain_commands())
            .unwrap_or_default();
        for event in events {
            self.control(event.command);
        }
        if self
            .transition
            .as_ref()
            .is_some_and(|t| !t.waiting_for_media && !t.is_active())
        {
            self.transition = None;
        }
        self.media.poll(ctx, !self.audience_open());
        self.release_media_wait();
        self.sync_deck();
        while let Some(result) = self.deck.poll() {
            if let Err(error) = result {
                self.runtime.output.deck = None;
                self.deck.reconcile(None);
                self.error = error;
            }
        }
        if let Some(error) = ctx.data_mut(|d| d.remove_temp::<String>(egui::Id::new("pdf-error"))) {
            self.error = format!("PDF error: {error}");
        }
        let now = Instant::now();
        self.runtime
            .tick(&self.project, now.saturating_duration_since(self.last_tick));
        self.last_tick = now;
        if self.last_display_scan.elapsed().as_secs() >= 1 {
            self.displays = crate::displays::enumerate_displays();
            self.last_display_scan = Instant::now();
        }
        if self
            .settings
            .audience_display
            .as_ref()
            .is_some_and(|key| crate::displays::match_display(Some(key), &self.displays).is_none())
            || self.settings.stage_display.as_ref().is_some_and(|key| {
                crate::displays::match_display(Some(key), &self.displays).is_none()
            })
        {
            self.status =
                "Saved display is unavailable. Outputs use the primary display in a window.".into();
        }
        let audience_fullscreen = self.audience_fullscreen_state.load(Ordering::Relaxed);
        let stage_fullscreen = self.stage_fullscreen_state.load(Ordering::Relaxed);
        if (
            self.settings.audience_fullscreen,
            self.settings.stage_fullscreen,
        ) != (audience_fullscreen, stage_fullscreen)
        {
            self.settings.audience_fullscreen = audience_fullscreen;
            self.settings.stage_fullscreen = stage_fullscreen;
            self.persist_settings();
        }
        self.fullscreen_audience = audience_fullscreen;
        self.fullscreen_stage = self.stage_fullscreen_state.load(Ordering::Relaxed);
        if consume_close_request(&self.audience_close_requested, &mut self.show_audience)
            && self.smoke_mode
        {
            eprintln!("LUMEN SMOKE: audience viewport closed");
        }
        if consume_close_request(&self.stage_close_requested, &mut self.show_stage)
            && self.smoke_mode
        {
            eprintln!("LUMEN SMOKE: stage viewport closed");
        }
        self.shortcuts(ctx);
        self.recovery_banner(ctx);
        self.missing_media_banner(ctx);
        self.toolbar(ctx);
        egui::TopBottomPanel::bottom("status")
            .exact_height(26.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(if self.dirty { "UNSAVED" } else { "Saved" });
                    ui.separator();
                    let path_text = self
                        .path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "No file".into());
                    let file_name = self
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "No file".into());
                    ui.add(egui::Label::new(file_name).truncate())
                        .on_hover_text(path_text);
                    ui.separator();
                    ui.label(&self.status);
                    if !self.error.is_empty() {
                        ui.horizontal(|ui| {
                            ui.colored_label(crate::style::WARN, &self.error);
                            if self.error.contains("Keynote isn't installed")
                                && ui.small_button("Get Keynote").clicked()
                            {
                                crate::deck::open_keynote_store();
                            }
                        });
                    }
                    if !self.media.status.is_empty() {
                        ui.label(&self.media.status);
                    }
                });
            });
        self.show_docks(ctx);
        if self.modal || self.active_save.is_some() {
            self.unsaved_modal(ctx)
        }
        if let Some(id) = self.confirm_delete {
            self.delete_modal(ctx, id)
        }
        Self::drop_overlay(ctx);
        if self.project != before && !self.history_applied {
            let focus = if ctx.wants_keyboard_input() {
                ctx.memory(|m| m.focused())
            } else {
                None
            };
            if focus.is_none() || focus != self.edit_focus {
                self.history.record(before);
            }
            self.edit_focus = focus;
            self.dirty = self.project != self.saved_project;
        } else if !ctx.wants_keyboard_input() {
            self.edit_focus = None;
        }
        self.autosave();
        if self.dirty && self.recovered.is_none() && !self.recovery_busy {
            ctx.request_repaint_after(
                std::time::Duration::from_secs(30).saturating_sub(self.last_autosave.elapsed()),
            );
        }
        self.sync_deck();
        self.sync_audio_clear();
        self.sync_media();
        self.queue_object_images();
        self.publish();
        self.show_outputs(ctx);
        if self.close_approved {
            self.close_approved = false;
            self.allow_close = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        // Transitions and video repaint every frame (vsync-paced); the rest tick at 80 ms.
        let video_playing = self
            .media
            .player
            .as_ref()
            .is_some_and(|p| !p.is_finished() && !p.is_paused());
        if self.transition.is_some() || (video_playing && !self.audience_open()) {
            ctx.request_repaint();
        } else if video_playing {
            // The audience window draws every frame of the video on its own refresh. A console
            // redrawing at its own 120 Hz would make that window's passes land at uneven times.
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        if self
            .runtime
            .timer_states
            .values()
            .any(|timer| timer.running)
            || self.runtime.output.deck.is_some()
            || self.media.pending
            || self.show_stage
            || self.runtime.output.message.is_some()
            || self.remote.as_ref().is_some_and(|r| r.is_running())
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
    }
    fn on_exit(&mut self) {
        self.save_layouts_now();
        self.remove_clean_recovery();
        self.audio.stop();
        self.deck.reconcile(None);
        self.media.shutdown();
    }
}
impl LumenApp {
    fn reset_adds(&mut self) {
        self.adding.clear();
        self.add_progress = None;
        self.add_detail = None;
        self.add_notice = None;
    }
    /// `None` opens the multi-select picker; otherwise the dropped paths are added.
    pub(super) fn start_add(&mut self, paths: Option<Vec<PathBuf>>) {
        let Some(theme) = self.project.themes.first().map(|t| t.id) else {
            self.error = "Couldn't add files: this show has no theme".into();
            return;
        };
        self.operation_counter = self.operation_counter.wrapping_add(1);
        self.adding.push(self.operation_counter);
        file::launch(
            FileAction::Add(self.operation_counter, paths, theme),
            self.tx.clone(),
            self.repaint_ctx.clone(),
        );
        self.status = "Adding files…".into();
    }
    fn handle_drops(&mut self, ctx: &Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if !dropped.is_empty() && !self.modal {
            self.start_add(Some(dropped));
        }
    }
    fn drop_overlay(ctx: &Context) {
        if ctx.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        let rect = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("drop-overlay"),
        ));
        painter.rect_filled(rect, 0.0, crate::style::CANVAS.gamma_multiply(0.85));
        painter.rect_stroke(
            rect.shrink(6.0),
            6.0,
            egui::Stroke::new(3.0_f32, crate::style::SAFFRON),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Drop to add to running order",
            egui::FontId::new(
                crate::style::TITLE,
                egui::FontFamily::Name("Poppins SemiBold".into()),
            ),
            crate::style::TEXT,
        );
    }
    fn finish_add(&mut self, batch: crate::add::AddBatch) {
        let notice = crate::add::batch_notice(batch.documents.len(), &batch.skipped);
        let mut notice = notice;
        if let Some(warning) = batch.warnings.first() {
            self.error = warning.clone();
            if batch.documents.is_empty() {
                notice = warning.clone();
            }
        }
        if let Some(first) = batch.documents.first() {
            let (doc, slide) = (first.id, first.slides.first().map(|s| s.id));
            crate::add::insert_documents(&mut self.project, self.selected_doc, batch.documents);
            crate::books::close_preview(self);
            self.library_tab = 0;
            self.selected_doc = doc;
            self.selected_slide = slide;
            self.edit_focus = None;
            self.mark();
        }
        self.status = notice.clone();
        self.add_notice = Some(notice);
    }
    fn running_order_focused(&self) -> bool {
        self.library_tab == 0 && !crate::books::has_preview(self)
    }
    fn selected_item(&self) -> Option<usize> {
        self.project
            .playlist
            .iter()
            .position(|p| p.document_id == self.selected_doc)
    }
    /// Move a running-order row to final index `to` and keep it selected. One undo step.
    pub(super) fn move_running_order(&mut self, from: usize, to: usize) {
        if crate::add::move_item(&mut self.project.playlist, from, to) {
            let doc = self.project.playlist[to].document_id;
            self.select_document(doc);
            self.edit_focus = None;
            self.mark();
        }
    }
    fn move_selected(&mut self, step: isize) {
        if let Some(from) = self.selected_item() {
            let to = from.saturating_add_signed(step);
            self.move_running_order(from, to);
        }
    }
    pub(super) fn select_document(&mut self, doc: Uuid) {
        self.selected_doc = doc;
        self.selected_slide = self
            .project
            .document(doc)
            .and_then(|d| d.slides.first())
            .map(|s| s.id);
    }
    /// Removes from the running order only; the document stays in the library.
    pub(super) fn remove_item(&mut self, index: usize) {
        if crate::add::remove_playlist_item(&mut self.project, index) {
            let next = index.min(self.project.playlist.len().saturating_sub(1));
            if let Some(item) = self.project.playlist.get(next) {
                self.select_document(item.document_id);
            }
            self.edit_focus = None;
            self.mark();
        }
    }
    fn remove_selected(&mut self) {
        if let Some(index) = self.selected_item() {
            self.remove_item(index);
        }
    }
    /// The Tools pane: Settings while they are open, otherwise the mode's tools.
    pub(super) fn tools_pane(&mut self, ui: &mut egui::Ui) {
        if !self.show_settings {
            crate::panels::tools(self, ui);
            return;
        }
        if ui.button("Close settings").clicked() {
            self.show_settings = false;
        }
        if let Ok(cache) = self.text_images.lock() {
            ui.label(cache.font_notice());
        }
        let action = crate::settings::settings_panel(
            ui,
            &mut self.settings,
            &self.project,
            self.settings_notice.as_deref(),
        );
        if action.changed && self.settings.validate().is_ok() {
            self.persist_settings();
        }
        if action.restart_requested && self.settings.validate().is_ok() {
            self.restart_remote();
        }
        if !self.remote_error.is_empty() {
            ui.colored_label(crate::style::WARN, &self.remote_error);
        }
    }
    fn toolbar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("toolbar")
            .frame(crate::style::panel_frame())
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let first_row_width = ui.available_width();
                    if self.present {
                        crate::brand::lockup(ui, true, first_row_width, &self.project.name);
                    } else {
                        crate::brand::lockup(ui, false, first_row_width, &self.project.name);
                        // Keeps "Lumen" from reading as the label of the show-name field.
                        ui.separator();
                        ui.add(
                            egui::TextEdit::singleline(&mut self.project.name)
                                .desired_width(160.0)
                                .hint_text("Show name"),
                        );
                    }
                    ui.separator();
                    ui.selectable_value(&mut self.present, false, "Prepare");
                    ui.selectable_value(&mut self.present, true, "Present");
                    ui.separator();
                    for (name, open) in
                        [("Audience", self.show_audience), ("Stage", self.show_stage)]
                    {
                        if output_pill(
                            ui,
                            name,
                            if open {
                                OutputStatus::On
                            } else {
                                OutputStatus::Off
                            },
                        )
                        .clicked()
                        {
                            if name == "Audience" {
                                set_output_open(
                                    &mut self.show_audience,
                                    &self.audience_close_requested,
                                    !open,
                                );
                            } else {
                                set_output_open(
                                    &mut self.show_stage,
                                    &self.stage_close_requested,
                                    !open,
                                );
                            }
                        }
                    }
                    let remote = if !self.remote_error.is_empty() {
                        "error"
                    } else if self.remote.as_ref().is_some_and(|r| r.is_running()) {
                        "on"
                    } else {
                        "off"
                    };
                    output_pill(
                        ui,
                        "Remote",
                        match remote {
                            "error" => OutputStatus::Error,
                            "on" => OutputStatus::On,
                            _ => OutputStatus::Off,
                        },
                    );
                    if self.dirty {
                        ui.label("Unsaved changes");
                    }
                });
                ui.horizontal(|ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New").clicked() {
                            self.request(Pending::New);
                            ui.close();
                        }
                        if ui.button("Open…").clicked() {
                            self.open_path = None;
                            self.request(Pending::Open);
                            ui.close();
                        }
                        let recent: Vec<PathBuf> = self
                            .settings
                            .recent_projects
                            .iter()
                            .filter(|p| p.is_file())
                            .cloned()
                            .collect();
                        if !recent.is_empty() {
                            ui.separator();
                            ui.weak("Open recent");
                        }
                        for path in &recent {
                            let name = path.file_name().unwrap_or_default().to_string_lossy();
                            if ui
                                .button(name.as_ref())
                                .on_hover_text(path.display().to_string())
                                .clicked()
                            {
                                self.open_path = Some(path.clone());
                                self.request(Pending::Open);
                                ui.close();
                            }
                        }
                        ui.separator();
                        if ui.button("Save").clicked() {
                            self.save(false);
                            ui.close();
                        }
                        if ui.button("Save as…").clicked() {
                            self.save(true);
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                self.active_save.is_none(),
                                egui::Button::new("Save with media…"),
                            )
                            .on_hover_text(
                                "Copies every photo, video, sound and deck next to the show so it moves between Macs",
                            )
                            .clicked()
                        {
                            self.save_with_media();
                            ui.close();
                        }
                        if ui
                            .add_enabled(self.active_import.is_none(), egui::Button::new("Import…"))
                            .clicked()
                            && let Some(t) = self.project.themes.first()
                        {
                            self.operation_counter = self.operation_counter.wrapping_add(1);
                            self.active_import = Some(self.operation_counter);
                            file::launch(
                                FileAction::Import(self.operation_counter, t.id),
                                self.tx.clone(),
                                ctx.clone(),
                            );
                            self.status = "Importing presentation…".into();
                            ui.close();
                        }
                        if ui.button("Export OpenLyrics…").clicked()
                            && let Some(doc) = self.doc()
                        {
                            file::launch(
                                FileAction::Export(doc.clone()),
                                self.tx.clone(),
                                ctx.clone(),
                            );
                            ui.close();
                        }
                    });
                    if ui
                        .add_enabled(self.history.can_undo(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        self.undo(false);
                    }
                    if ui
                        .add_enabled(self.history.can_redo(), egui::Button::new("Redo"))
                        .clicked()
                    {
                        self.undo(true);
                    }
                    if ui.button("Settings").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                    egui::containers::menu::MenuButton::new("Outputs")
                        .config(egui::containers::menu::MenuConfig::new().close_behavior(
                            egui::PopupCloseBehavior::CloseOnClickOutside,
                        ))
                        .ui(ui, |ui| {
                        self.displays = crate::displays::enumerate_displays();
                        let action =
                            crate::displays::outputs_panel(ui, &mut self.settings, &self.displays);
                        if action.changed {
                            self.fullscreen_audience = self.settings.audience_fullscreen;
                            self.fullscreen_stage = self.settings.stage_fullscreen;
                            self.audience_fullscreen_state
                                .store(self.fullscreen_audience, Ordering::Relaxed);
                            self.stage_fullscreen_state
                                .store(self.fullscreen_stage, Ordering::Relaxed);
                            self.persist_settings();
                        }
                        if action.identify {
                            self.identify_until =
                                Some(Instant::now() + std::time::Duration::from_secs(3));
                        }
                        });
                    if ui.button("Shortcuts").clicked() {
                        self.show_help = !self.show_help;
                    }
                    if ui
                        .button("Reset layout")
                        .on_hover_text("Puts this mode's panels back where they started")
                        .clicked()
                    {
                        self.docks.reset(self.present);
                    }
                });
            });
        if self.show_help {
            egui::Window::new("Keyboard shortcuts").open(&mut self.show_help).show(ctx,|ui|{
            ui.label("Space / Right / Page Down / Return: send the selected cue (Next). Left / Page Up: Previous.");
            ui.label("F1 All · F2 Slide · F3 Media · F4 Overlays · F5 Audio · F6 Messages. B: Blackout.");
            ui.label("Cmd+F Search. Cmd+Z Undo. Cmd+Shift+Z Redo. Shortcuts pause while editing text.");
            ui.label("Running order: Cmd+Up / Cmd+Down move the selected item. Delete removes it from the order (not the library).");
        });
        }
    }
    fn unsaved_modal(&mut self, ctx: &Context) {
        let saving = self.active_save.is_some();
        let opening = self.active_open.is_some();
        let choosing_media = self.media_target.is_some();
        egui::Modal::new(egui::Id::new("lumen-operation-modal")).show(ctx, |ui| {
            if choosing_media {
                ui.heading("Choose background media");
                ui.label("The native file picker is open. Output keeps running while it is open.");
            } else if opening {
                ui.heading("Open project");
                ui.label("Choose a project in the native file picker.");
            } else if saving {
                ui.heading("Saving project");
                ui.label("The saved snapshot is being written atomically.");
                ui.add(egui::Spinner::new());
            } else if self.pending.is_some() {
                ui.heading("Unsaved changes");
                if !self.error.is_empty() {
                    ui.colored_label(crate::style::WARN, &self.error);
                }
                ui.label("Save changes before continuing?");
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        self.error.clear();
                        self.save(false);
                    }
                    if ui.button("Discard").clicked() {
                        let pending = self.pending.take();
                        self.modal = false;
                        if let Some(pending) = pending {
                            self.begin(pending);
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending = None;
                        self.pending_id = None;
                        self.modal = false;
                        self.reply_native_quit(false);
                    }
                });
            } else {
                ui.heading("Saving project");
                ui.add(egui::Spinner::new());
            }
            if saving && self.pending.is_some() && ui.button("Cancel pending action").clicked() {
                self.pending = None;
                self.pending_id = None;
                self.modal = false;
                self.reply_native_quit(false);
            }
        });
    }
    fn delete_modal(&mut self, ctx: &Context, id: Uuid) {
        let mut open = true;
        egui::Window::new("Delete document?")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("This removes the document and its playlist entries. Live output remains on its cloned cue.");
                ui.horizontal(|ui| {
                    if ui.button("Delete document").clicked() {
                        self.project.documents.retain(|d| d.id != id);
                        self.project.playlist.retain(|i| i.document_id != id);
                        self.selected_doc = self.project.documents.first().map(|d| d.id).unwrap_or_else(Uuid::new_v4);
                        self.selected_slide = None;
                        self.confirm_delete = None;
                        self.ensure_selection();
                        self.mark();
                    } else if ui.button("Cancel").clicked() {
                        self.confirm_delete = None;
                    }
                });
            });
        if !open {
            self.confirm_delete = None;
        }
    }
    fn show_outputs(&mut self, ctx: &Context) {
        if let Some(deadline) = self.identify_until {
            crate::displays::show_identify(ctx, &self.displays, deadline);
            if Instant::now() >= deadline {
                self.identify_until = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(80));
            }
        }
        let data = self.published.clone();
        let mut audience_open = self.audience_open();
        let target = crate::displays::target_display(
            self.settings.audience_display.as_ref(),
            &self.displays,
        )
        .cloned();
        if crate::displays::needs_replace(
            self.audience_placed,
            self.audience_target.as_ref(),
            target.as_ref(),
        ) {
            // A fullscreen window ignores a new position, so close it for a frame; it reopens
            // windowed on the new display and goes fullscreen a frame later, like the first open.
            self.audience_placed = false;
            audience_open = false;
            ctx.request_repaint();
        }
        self.audience_target = target;
        // macOS fullscreens a new window on the display it starts on, before the position
        // applies; open windowed on the chosen display, then go fullscreen a frame later.
        let audience_fullscreen = self.fullscreen_audience && self.audience_placed;
        self.audience_placed = audience_open;
        if !audience_open {
            self.audience_pixels.store(0, Ordering::Relaxed);
        }
        if audience_open {
            let viewport_id = ViewportId::from_hash_of("audience");
            ctx.request_repaint_of(viewport_id);
            let close_cb = self.audience_close_requested.clone();
            let root_ctx = ctx.clone();
            let fullscreen_state = self.audience_fullscreen_state.clone();
            let pixels = self.audience_pixels.clone();
            ctx.show_viewport_deferred(
                viewport_id,
                crate::displays::output_viewport_builder(
                    "Lumen - Audience",
                    [960.0, 540.0],
                    self.settings.audience_display.as_ref(),
                    &self.displays,
                    audience_fullscreen,
                )
                .0,
                move |ctx, _| {
                    let size = ctx.content_rect().size() * ctx.pixels_per_point();
                    pixels.store(
                        (u64::from(size.x as u32) << 32) | u64::from(size.y as u32),
                        Ordering::Relaxed,
                    );
                    let mut state = data.lock().map(|s| s.clone()).unwrap_or_default();
                    if let Some(video) = &state.video
                        && let Some(texture) = state.media.as_mut()
                        && let Some(frame) =
                            video.frame_at(Instant::now(), crate::media::refresh_period(ctx))
                    {
                        crate::media::upload(texture, frame);
                    }
                    if state.media_animating || state.transition.is_some() {
                        ctx.request_repaint();
                    } else if state.output.message.is_some() {
                        ctx.request_repaint_after(std::time::Duration::from_millis(80));
                    }
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE)
                        .show(ctx, |ui| {
                            let r = ui.max_rect();
                            if let Some(project) = state.contents.as_ref() {
                                crate::render::paint_audience(
                                    ui,
                                    r,
                                    &state.output,
                                    project,
                                    &state.clocks,
                                    state.now,
                                    state.media.as_ref(),
                                    state.transition.as_ref(),
                                    &state.images,
                                );
                            }
                        });
                    if ctx.input(|i| i.key_pressed(Key::Escape)) {
                        fullscreen_state.store(false, Ordering::Relaxed);
                        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
                    }
                    if ctx.input(|input| input.viewport().close_requested()) {
                        close_cb.store(true, Ordering::Relaxed);
                        root_ctx.request_repaint();
                        ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                    }
                },
            );
        }
        let data = self.published.clone();
        let target =
            crate::displays::target_display(self.settings.stage_display.as_ref(), &self.displays)
                .cloned();
        let mut show_stage = self.show_stage;
        if crate::displays::needs_replace(
            self.stage_placed,
            self.stage_target.as_ref(),
            target.as_ref(),
        ) {
            self.stage_placed = false;
            show_stage = false;
            ctx.request_repaint();
        }
        self.stage_target = target;
        let stage_fullscreen = self.fullscreen_stage && self.stage_placed;
        self.stage_placed = show_stage;
        if show_stage {
            let viewport_id = ViewportId::from_hash_of("stage");
            ctx.request_repaint_of(viewport_id);
            let close_cb = self.stage_close_requested.clone();
            let root_ctx = ctx.clone();
            let fullscreen_state = self.stage_fullscreen_state.clone();
            ctx.show_viewport_deferred(
                viewport_id,
                crate::displays::output_viewport_builder(
                    "Lumen - Stage",
                    [700.0, 600.0],
                    self.settings.stage_display.as_ref(),
                    &self.displays,
                    stage_fullscreen,
                )
                .0,
                move |ctx, _| {
                    let state = data.lock().map(|s| s.clone()).unwrap_or_default();
                    if state.media_animating {
                        ctx.request_repaint();
                    } else if state.runtime.timers.iter().any(|t| t.2) {
                        ctx.request_repaint_after(std::time::Duration::from_millis(80));
                    }
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading("Stage");
                        if state.stage.clock {
                            ui.label(
                                egui::RichText::new(&state.clock)
                                    .monospace()
                                    .size(crate::style::CLOCK),
                            );
                        }
                        if state.stage.current {
                            ui.colored_label(crate::style::LIVE, "Current");
                            if state.output.live.as_ref().is_some_and(|l| {
                                matches!(l.slide.background, Background::Pdf { .. })
                            }) {
                                let width = ui.available_width();
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(width, width * 9.0 / 16.0),
                                    egui::Sense::hover(),
                                );
                                crate::render::paint_preview(
                                    ui,
                                    rect,
                                    state.output.live.as_ref(),
                                    None,
                                    &state.images,
                                );
                            }
                            crate::render::stage_text(
                                ui,
                                state
                                    .output
                                    .live
                                    .as_ref()
                                    .map(|l| l.slide.text.as_str())
                                    .unwrap_or("No cue live"),
                                28.0,
                                150.0,
                            );
                        }
                        if state.stage.next {
                            ui.separator();
                            ui.label("Next");
                            crate::render::stage_text(
                                ui,
                                state
                                    .project
                                    .next
                                    .as_ref()
                                    .map(|l| l.slide.text.as_str())
                                    .unwrap_or("End of running order"),
                                24.0,
                                120.0,
                            );
                        }
                        if state.stage.message {
                            ui.separator();
                            crate::render::stage_text(ui, &state.runtime.message, 24.0, 64.0);
                        }
                        if state.stage.notes {
                            ui.separator();
                            crate::text::gu_label(
                                ui,
                                state
                                    .output
                                    .live
                                    .as_ref()
                                    .map(|l| l.slide.notes.as_str())
                                    .unwrap_or(""),
                                13.0,
                            );
                        }
                        if let Some(timer) = state
                            .stage
                            .timer
                            .and_then(|id| state.contents.as_ref()?.timer_def(id))
                        {
                            ui.label(
                                egui::RichText::new(state.clocks.timer_display(timer, state.now))
                                    .monospace()
                                    .size(crate::style::CLOCK),
                            );
                        }
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                            if ui.button("Windowed").clicked() {
                                fullscreen_state.store(false, Ordering::Relaxed);
                                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
                            }
                            if ui.button("Fullscreen").clicked() {
                                fullscreen_state.store(true, Ordering::Relaxed);
                                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
                            }
                        });
                    });
                    if ctx.input(|i| i.key_pressed(Key::Escape)) {
                        fullscreen_state.store(false, Ordering::Relaxed);
                        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
                    }
                    if ctx.input(|input| input.viewport().close_requested()) {
                        close_cb.store(true, Ordering::Relaxed);
                        root_ctx.request_repaint();
                        ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                    }
                },
            );
        }
    }
}

#[cfg(test)]
mod operation_tests {
    use super::{SaveDecision, SaveOutcome, save_decision};

    #[test]
    fn stale_save_completion_cannot_advance_another_operation() {
        assert_eq!(
            save_decision(
                Some((2, Some(10))),
                1,
                SaveOutcome::Succeeded,
                true,
                Some(10)
            ),
            SaveDecision::Ignore
        );
    }

    #[test]
    fn edits_during_save_keep_pending_action_blocked() {
        assert_eq!(
            save_decision(
                Some((4, Some(20))),
                4,
                SaveOutcome::Succeeded,
                false,
                Some(20)
            ),
            SaveDecision::Stay
        );
    }

    #[test]
    fn failed_save_keeps_pending_action() {
        assert_eq!(
            save_decision(Some((5, Some(30))), 5, SaveOutcome::Failed, true, Some(30)),
            SaveDecision::Stay
        );
    }

    #[test]
    fn cancelled_save_keeps_pending_action() {
        assert_eq!(
            save_decision(
                Some((5, Some(30))),
                5,
                SaveOutcome::Cancelled,
                true,
                Some(30)
            ),
            SaveDecision::Stay
        );
    }

    #[test]
    fn cancelled_pending_action_is_not_resumed_by_save() {
        assert_eq!(
            save_decision(Some((6, Some(40))), 6, SaveOutcome::Succeeded, true, None),
            SaveDecision::Stay
        );
    }
}

fn consume_close_request(flag: &AtomicBool, is_open: &mut bool) -> bool {
    if flag.swap(false, Ordering::Relaxed) {
        *is_open = false;
        true
    } else {
        false
    }
}

fn set_output_open(is_open: &mut bool, close_requested: &AtomicBool, open: bool) {
    if open {
        close_requested.store(false, Ordering::Relaxed);
    }
    *is_open = open;
}

#[cfg(test)]
mod output_close_tests {
    use super::{consume_close_request, set_output_open};
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn close_event_between_root_updates_closes_then_reopen_resets_flag() {
        let close_requested = AtomicBool::new(false);
        let mut is_open = true;

        // Deferred viewport callback reports after the parent update has returned.
        close_requested.store(true, Ordering::Relaxed);
        assert!(consume_close_request(&close_requested, &mut is_open));
        assert!(!is_open);
        assert!(!close_requested.load(Ordering::Relaxed));

        // Reopening starts clean and is not immediately closed by an old event.
        set_output_open(&mut is_open, &close_requested, true);
        assert!(!consume_close_request(&close_requested, &mut is_open));
        assert!(is_open);
    }
}

impl LumenApp {
    pub(super) fn seconds_of_day(&self) -> u32 {
        use chrono::Timelike;
        chrono::Local::now().num_seconds_from_midnight()
    }
    fn audience_open(&self) -> bool {
        self.show_audience && self.runtime.output.deck.is_none()
    }
    fn sync_deck(&mut self) {
        self.deck.reconcile(self.runtime.output.deck.clone());
    }
    /// Starts the fade once the new media has a frame, or after 1.5 s so a bad file can't freeze the picture.
    fn release_media_wait(&mut self) {
        let ready = !self.media.pending
            && self
                .media
                .player
                .as_ref()
                .is_none_or(|p| p.frames().has_frame());
        if let Some(t) = self.transition.as_mut().filter(|t| t.waiting_for_media)
            && (ready || t.started.elapsed() >= MEDIA_WAIT)
        {
            t.started = Instant::now();
            t.waiting_for_media = false;
        }
    }
    /// True when the live cue needs a background that is not on screen yet.
    fn live_media_will_load(&self) -> bool {
        let output = &self.runtime.output;
        let background = output.effective_background();
        !output.media_cleared
            && matches!(
                background,
                Some(Background::Video { .. } | Background::Image { .. })
            )
            && (self.media.pending
                || self.media.image_texture.is_none()
                || self.media.background_requires_reload(background)
                || self.media.restart_on_trigger(background))
    }
    fn after_trigger(&mut self, previous: OutputState, previous_media: Option<TextureHandle>) {
        if self.runtime.output.live.as_ref().is_some_and(|l| {
            l.slide
                .actions
                .iter()
                .any(|a| matches!(a, lumen_core::CueAction::PlayDeck { .. }))
        }) {
            self.deck.reconcile(None);
        }
        self.sync_deck();
        if let Some(live) = &self.runtime.output.live {
            let kind = effective_transition(&self.project.transition, &live.slide.transition);
            let audio = live.slide.audio.clone();
            // Even a Cut holds the old picture until the new media is ready (no black flash).
            let waiting_for_media = self.live_media_will_load();
            self.transition = (transition_millis(&kind) > 0 || waiting_for_media).then(|| {
                crate::render::OutputTransition {
                    previous,
                    previous_media,
                    started: Instant::now(),
                    kind,
                    waiting_for_media,
                }
            });
            if let Some(cue) = &audio {
                self.audio.play_cue(cue);
            }
        }
    }
    pub(super) fn navigate(&mut self, forward: bool) {
        if self.runtime.output.deck.is_some() {
            self.deck.navigate(forward);
            return;
        }
        let previous = self.runtime.output.clone();
        let media = self.media.image_texture.clone();
        let result = if forward {
            self.runtime.next(&self.project)
        } else {
            self.runtime.previous(&self.project)
        };
        match result {
            Ok(true) => {
                self.after_trigger(previous, media);
                self.sync_media();
                self.arm_next();
            }
            Ok(false) => {}
            Err(e) => self.error = e.to_string(),
        }
    }
    pub(super) fn apply_actions(&mut self, actions: &[lumen_core::CueAction]) {
        // Blackout is a safety action: it never eases.
        if actions
            .iter()
            .any(|a| matches!(a, lumen_core::CueAction::Blackout(_)))
        {
            self.transition = None;
        }
        if let Err(e) = self.runtime.apply_actions(&self.project, actions) {
            self.error = e.to_string();
        }
        self.sync_deck();
        self.sync_audio_clear();
        self.sync_media();
    }
    pub(super) fn run_macro(&mut self, id: Uuid) {
        if let Err(e) = self.runtime.run_macro(&self.project, id) {
            self.error = e.to_string();
        }
        self.sync_deck();
        self.sync_audio_clear();
        self.sync_media();
    }
    pub(super) fn clear_layer(&mut self, n: u8) {
        use lumen_core::CueAction;
        let action = match n {
            1 => CueAction::ClearAll,
            2 => CueAction::ClearSlide,
            3 => CueAction::ClearMedia,
            4 => CueAction::ClearProps,
            5 => CueAction::ClearAudio,
            _ => CueAction::ClearMessages,
        };
        let previous = self.runtime.output.clone();
        let previous_media = self.media.image_texture.clone();
        self.apply_actions(&[action]);
        let kind = self.project.transition.clone();
        self.transition = (transition_millis(&kind) > 0).then(|| crate::render::OutputTransition {
            previous,
            previous_media,
            started: Instant::now(),
            kind,
            waiting_for_media: false,
        });
    }
    fn sync_audio_clear(&mut self) {
        if self.audio_seen_serial != self.runtime.output.audio_clear_serial {
            self.audio.clear();
            self.audio_seen_serial = self.runtime.output.audio_clear_serial;
        }
    }
    pub(super) fn thumbnail(&mut self, background: &Background) -> Option<TextureHandle> {
        let path = match background {
            Background::Image { path, .. } | Background::Video { path, .. } => path,
            Background::Solid(_) | Background::Pdf { .. } => return None,
        };
        self.thumbnails.request(path);
        self.thumbnails.texture(path).cloned()
    }
    /// The big Preview's sharper still; `None` until it has loaded, so callers fall back to `thumbnail`.
    pub(super) fn thumbnail_large(&mut self, background: &Background) -> Option<TextureHandle> {
        let (Background::Image { path, .. } | Background::Video { path, .. }) = background else {
            return None;
        };
        self.thumbnails.request_large(path);
        self.thumbnails.large_texture(path).cloned()
    }
    fn queue_object_images(&mut self) {
        let live = self
            .runtime
            .output
            .live
            .iter()
            .flat_map(|l| &l.slide.objects);
        let paths: std::collections::HashSet<PathBuf> = self
            .project
            .documents
            .iter()
            .flat_map(|d| &d.slides)
            .flat_map(|s| &s.objects)
            .chain(self.project.props.iter().flat_map(|p| &p.objects))
            .chain(
                self.runtime
                    .output
                    .prop_snapshots
                    .iter()
                    .flat_map(|p| &p.objects),
            )
            .chain(live)
            .filter_map(|object| match object {
                lumen_core::SlideObject::Image { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect();
        for path in &paths {
            self.thumbnails.request(path);
        }
        self.full_images.sync(&paths);
    }
    /// Thumbnails for the grids; the outputs get full-size textures where they are ready.
    fn output_images(&self) -> std::collections::HashMap<PathBuf, TextureHandle> {
        self.full_images.over(self.thumbnails.images())
    }
    pub(super) fn paint_live(&self, ui: &egui::Ui, rect: egui::Rect) {
        crate::render::paint_audience(
            ui,
            rect,
            &self.runtime.output,
            &self.project,
            &self.runtime,
            self.seconds_of_day(),
            self.media.image_texture.as_ref(),
            self.transition.as_ref(),
            &self.output_images(),
        );
    }
    fn undo(&mut self, redo: bool) {
        let restored = if redo {
            self.history.redo(self.project.clone())
        } else {
            self.history.undo(self.project.clone())
        };
        if let Some(project) = restored {
            self.project = project;
            self.dirty = self.project != self.saved_project;
            self.history_applied = true;
            self.edit_focus = None;
            self.ensure_selection();
            self.runtime.sync_timers(&self.project);
        }
    }
    pub(super) fn persist_settings(&mut self) {
        if let Some(path) = crate::settings::settings_path() {
            if let Err(e) = self.settings.save_atomic(&path) {
                self.settings_notice = Some(format!("Couldn't save settings: {e}"));
            }
        } else {
            self.settings_notice =
                Some("Settings couldn't be saved: home folder unavailable".into());
        }
    }
    fn restart_remote(&mut self) {
        let result = if let Some(remote) = self.remote.as_mut() {
            remote.restart(&self.settings, self.remote_status.clone())
        } else {
            crate::remote::RemoteManager::start(&self.settings, self.remote_status.clone()).map(
                |r| {
                    self.remote = Some(r);
                },
            )
        };
        self.remote_error = result.err().unwrap_or_default();
    }
    fn control(&mut self, command: lumen_control::ControlCommand) {
        use lumen_control::ControlCommand as C;
        use lumen_core::CueAction as A;
        let timer_action = match &command {
            C::TimerStart { .. } => 0,
            C::TimerPause { .. } => 1,
            _ => 2,
        };
        match command {
            C::Next => self.navigate(true),
            C::Previous => self.navigate(false),
            C::Trigger { index } => self.run_trigger(|r, p| r.trigger_index(p, index)),
            C::Blackout { on } => self.apply_actions(&[A::Blackout(on)]),
            C::ClearText => self.clear_layer(2),
            C::ClearAll => self.clear_layer(1),
            C::RunMacro { name } => {
                if let Some(id) = self
                    .project
                    .macros
                    .iter()
                    .find(|m| m.name == name)
                    .map(|m| m.id)
                {
                    self.run_macro(id)
                }
            }
            C::TimerStart { name } | C::TimerPause { name } | C::TimerReset { name } => {
                if let Some(id) = self
                    .project
                    .timers
                    .iter()
                    .find(|t| t.name == name)
                    .map(|t| t.id)
                {
                    // Variant is classified before moving the command in the outer match.
                    let action = match timer_action {
                        0 => A::StartTimer { id },
                        1 => A::PauseTimer { id },
                        _ => A::ResetTimer { id },
                    };
                    self.apply_actions(&[action]);
                }
            }
            C::StageMessage { text } => self.apply_actions(&[A::StageMessage(text)]),
        }
    }
    fn autosave(&mut self) {
        if !self.dirty || self.recovery_busy || self.recovered.is_some() {
            return;
        }
        if self.last_autosave.elapsed() < std::time::Duration::from_secs(30) {
            return;
        }
        self.last_autosave = Instant::now();
        if let Some(path) = crate::recovery::recovery_path(self.path.as_deref()) {
            self.recovery_busy = true;
            self.operation_counter = self.operation_counter.wrapping_add(1);
            file::launch(
                FileAction::Autosave(
                    self.operation_counter,
                    self.project_epoch,
                    path,
                    Box::new(self.project.clone()),
                ),
                self.tx.clone(),
                self.repaint_ctx.clone(),
            );
        }
    }
    fn remove_clean_recovery(&self) {
        if !self.dirty
            && !self.recovery_busy
            && let Some(path) = crate::recovery::recovery_path(self.path.as_deref())
        {
            let _ = std::fs::remove_file(path);
        }
    }
    fn recovery_banner(&mut self, ctx: &Context) {
        let Some(recovered) = self.recovered.as_ref() else {
            return;
        };
        let time = recovered.time.clone();
        let mut restore = false;
        let mut discard = false;
        egui::TopBottomPanel::top("recovery-banner").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    crate::style::INFO,
                    format!("Recovered unsaved changes from {time} —"),
                );
                restore = ui.button("Restore").clicked();
                discard = ui.button("Discard").clicked();
            });
        });
        if restore && let Some(recovered) = self.recovered.take() {
            self.history.record(self.project.clone());
            self.project = recovered.project;
            self.mark();
            self.ensure_selection();
            self.history_applied = true;
        } else if discard
            && let Some(recovered) = self.recovered.take()
            && let Err(e) = std::fs::remove_file(recovered.path)
        {
            self.error = format!("Couldn't discard recovery: {e}");
        }
    }
}

// shortcut: per-slide Cut = show default, so one slide can't force a hard cut while the show fades; add Option<Transition> if operators ask.
fn effective_transition(
    show: &lumen_core::Transition,
    slide: &lumen_core::Transition,
) -> lumen_core::Transition {
    match slide {
        lumen_core::Transition::Cut => show.clone(),
        own => own.clone(),
    }
}

/// The cue to select after a send: the one after live, or live itself at the end of the order.
/// `None` outside Present, where the selection belongs to the editor.
fn armed_after_send(runtime: &Runtime, project: &Project, present: bool) -> Option<(Uuid, Uuid)> {
    if !present {
        return None;
    }
    let ids = |l: &LiveSlide| (l.document_id, l.slide.id);
    runtime
        .peek_next(project)
        .as_ref()
        .map(ids)
        .or_else(|| runtime.output.live.as_ref().map(ids))
}

#[cfg(test)]
mod console_safety_tests {
    use super::*;
    fn three_documents() -> (Project, Vec<(Uuid, Uuid)>) {
        let mut project = Project::demo();
        project.documents.clear();
        project.playlist.clear();
        let theme = project.themes[0].id;
        let mut cues = Vec::new();
        for name in ["A", "B", "C"] {
            let mut doc = Document::new(name);
            let slide = lumen_core::Slide::new(theme);
            cues.push((doc.id, slide.id));
            doc.slides.push(slide);
            project.playlist.push(PlaylistItem {
                id: Uuid::new_v4(),
                document_id: doc.id,
            });
            project.documents.push(doc);
        }
        (project, cues)
    }
    #[test]
    fn sending_arms_the_cue_after_it_across_documents() {
        let (project, cues) = three_documents();
        let mut runtime = Runtime::default();
        // Nothing live yet: the first cue is the one Next would send.
        assert_eq!(armed_after_send(&runtime, &project, true), Some(cues[0]));
        runtime.trigger(&project, cues[0].0, cues[0].1).unwrap();
        assert_eq!(armed_after_send(&runtime, &project, true), Some(cues[1]));
        runtime.next(&project).unwrap();
        assert_eq!(armed_after_send(&runtime, &project, true), Some(cues[2]));
        runtime.next(&project).unwrap();
        // End of the order: the live cue stays selected.
        assert_eq!(armed_after_send(&runtime, &project, true), Some(cues[2]));
        runtime.previous(&project).unwrap();
        assert_eq!(armed_after_send(&runtime, &project, true), Some(cues[2]));
    }
    #[test]
    fn prepare_never_moves_the_selection() {
        let (project, cues) = three_documents();
        let mut runtime = Runtime::default();
        runtime.trigger(&project, cues[0].0, cues[0].1).unwrap();
        assert_eq!(armed_after_send(&runtime, &project, false), None);
    }
    #[test]
    fn recovery_older_than_successful_save_or_replaced_project_is_removed() {
        assert!(recovery_is_obsolete(1, 2, 20, 0, false));
        assert!(recovery_is_obsolete(1, 1, 10, 11, false));
        assert!(recovery_is_obsolete(1, 1, 12, 11, true));
        assert!(!recovery_is_obsolete(1, 1, 12, 11, false));
    }
    #[test]
    fn slide_cut_uses_show_transition_and_other_kinds_override() {
        use lumen_core::Transition::{Cut, Fade, Push};
        let show = Fade { millis: 400 };
        assert_eq!(effective_transition(&show, &Cut), show);
        let push = Push {
            millis: 300,
            direction: lumen_core::PushDirection::Left,
        };
        assert_eq!(effective_transition(&show, &push), push);
    }
    #[test]
    fn transition_duration_matches_every_kind() {
        assert_eq!(transition_millis(&lumen_core::Transition::Cut), 0);
        assert_eq!(
            transition_millis(&lumen_core::Transition::FadeThroughBlack { millis: 123 }),
            123
        );
        assert_eq!(
            transition_millis(&lumen_core::Transition::Push {
                millis: 456,
                direction: lumen_core::PushDirection::Up
            }),
            456
        );
    }
}

fn recovery_is_obsolete(
    epoch: u64,
    current_epoch: u64,
    autosave: u64,
    saved: u64,
    clean: bool,
) -> bool {
    epoch != current_epoch || autosave < saved || clean
}

#[derive(Clone, Copy)]
enum OutputStatus {
    Off,
    On,
    Error,
}
fn output_pill(ui: &mut egui::Ui, name: &str, status: OutputStatus) -> egui::Response {
    let tint = match status {
        OutputStatus::Off => crate::style::TEXT_DIM,
        OutputStatus::On => crate::style::OK,
        OutputStatus::Error => crate::style::LIVE,
    };
    let state = match status {
        OutputStatus::Off => "off",
        OutputStatus::On => "on",
        OutputStatus::Error => "error",
    };
    let label = format!("{name} {state}");
    let galley = ui
        .painter()
        .layout_no_wrap(label.clone(), egui::FontId::proportional(11.0), tint);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(galley.size().x + 30.0, 28.0),
        egui::Sense::click(),
    );
    ui.painter().rect_stroke(
        rect,
        14.0,
        egui::Stroke::new(1.0_f32, tint),
        egui::StrokeKind::Inside,
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            14.0,
            egui::Stroke::new(2.0_f32, crate::style::SAFFRON),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter()
        .circle_filled(egui::pos2(rect.left() + 10.0, rect.center().y), 3.0, tint);
    ui.painter().galley(
        egui::pos2(rect.left() + 19.0, rect.center().y - galley.size().y / 2.0),
        galley,
        tint,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label.clone())
    });
    response
}
