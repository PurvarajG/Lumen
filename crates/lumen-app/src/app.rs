use crate::{
    file::{self, FileAction, FileResult},
    media::MediaState,
};
use eframe::egui::{self, Color32, Context, Key, TextureHandle, ViewportCommand, ViewportId};
use lumen_core::{Background, Document, LiveSlide, OutputState, PlaylistItem, Project, Runtime};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
    },
    time::Instant,
};
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct Published {
    pub project: ProjectView,
    pub output: OutputState,
    pub runtime: RuntimeView,
    pub media: Option<TextureHandle>,
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
    pub runtime: Runtime,
    pub selected_doc: Uuid,
    pub selected_slide: Option<Uuid>,
    pub dirty: bool,
    pub path: Option<PathBuf>,
    pub error: String,
    pub status: String,
    pub media: MediaState,
    pub show_audience: bool,
    pub show_stage: bool,
    pub fullscreen_audience: bool,
    pub fullscreen_stage: bool,
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
    pub(super) stage_tools: bool,
    pub(super) timer_name_draft: String,
    pub(super) timer_duration_draft: String,
    pub(super) timer_error: String,
    show_help: bool,
    pub(super) confirm_delete: Option<Uuid>,
    media_target: Option<(Uuid, bool)>,
    close_approved: bool,
    allow_close: bool,
    smoke_test: bool,
    smoke_mode: bool,
    audience_fullscreen_state: Arc<AtomicBool>,
    stage_fullscreen_state: Arc<AtomicBool>,
    audience_close_requested: Arc<AtomicBool>,
    stage_close_requested: Arc<AtomicBool>,
    repaint_ctx: Context,
    native_quit_pending: bool,
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
        let selected_doc = project
            .documents
            .first()
            .map(|d| d.id)
            .unwrap_or_else(Uuid::new_v4);
        let (tx, rx) = file::channel();
        let mut media = MediaState::default();
        media.set_repaint_context(cc.egui_ctx.clone());
        let mut app = Self {
            project,
            runtime: Runtime::default(),
            selected_doc,
            selected_slide: None,
            dirty: false,
            path,
            error: String::new(),
            status: "Ready".into(),
            media,
            show_audience: smoke,
            show_stage: smoke,
            fullscreen_audience: false,
            fullscreen_stage: false,
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
            stage_tools: false,
            timer_name_draft: "Countdown".into(),
            timer_duration_draft: "300".into(),
            timer_error: String::new(),
            show_help: false,
            confirm_delete: None,
            media_target: None,
            close_approved: false,
            allow_close: false,
            smoke_test: smoke,
            smoke_mode: smoke,
            audience_fullscreen_state: Arc::new(AtomicBool::new(false)),
            stage_fullscreen_state: Arc::new(AtomicBool::new(false)),
            audience_close_requested: Arc::new(AtomicBool::new(false)),
            stage_close_requested: Arc::new(AtomicBool::new(false)),
            repaint_ctx: cc.egui_ctx.clone(),
            native_quit_pending: false,
        };
        app.ensure_selection();
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
        match self.runtime.trigger(&self.project, doc, slide) {
            Ok(()) => {
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
            }
            Err(e) => self.error = e.to_string(),
        }
    }
    pub(super) fn sync_media(&mut self) {
        let background = self
            .runtime
            .output
            .live
            .as_ref()
            .map(|l| l.slide.background.clone());
        if !self.media.background_requires_reload(background.as_ref()) {
            return;
        }
        match background {
            Some(Background::Video {
                path,
                looped,
                muted,
            }) => {
                self.media.stop();
                self.media.load_video(path, looped, muted)
            }
            Some(Background::Image { path }) => {
                self.media.stop();
                self.media.load_image(path)
            }
            Some(background @ Background::Solid(_)) => {
                self.media.stop();
                self.media.remember_background(background);
            }
            None => self.media.stop(),
        }
    }
    fn publish(&mut self) {
        let view = Published {
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
                            self.runtime.timer_display(timer, 0),
                            running,
                        )
                    })
                    .collect(),
                message: self.runtime.output.stage_message.clone(),
            },
            media: self.media.image_texture.clone(),
            media_animating: self
                .media
                .player
                .as_ref()
                .is_some_and(|player| !player.is_finished()),
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
                    FileAction::Open(id),
                    self.tx.clone(),
                    self.repaint_ctx.clone(),
                );
                self.status = "Choose a project...".into();
            }
            Pending::New => {
                self.media.stop();
                self.project = Project::demo();
                self.project.name = "Untitled show".into();
                self.project.documents.clear();
                self.project.playlist.clear();
                self.runtime = Runtime::default();
                self.selected_doc = Uuid::new_v4();
                self.selected_slide = None;
                self.path = None;
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
                .add_filter("Lumen project", &["lumen.json"])
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
                    self.path = Some(p);
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
                            self.path = Some(path);
                            self.dirty = self.project != snapshot;
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
                FileResult::Open(_, _) | FileResult::Save(_, _) => {}
                FileResult::Import(None) => self.status = "Import cancelled".into(),
                FileResult::Import(Some(Err(e))) => self.error = e,
                FileResult::Import(Some(Ok((name, text)))) => {
                    if let Some(theme) = self.project.themes.first().map(|t| t.id) {
                        let doc = lumen_core::document_from_text(&name, &text, theme);
                        self.project.playlist.push(PlaylistItem {
                            id: Uuid::new_v4(),
                            document_id: doc.id,
                        });
                        self.selected_doc = doc.id;
                        self.selected_slide = doc.slides.first().map(|s| s.id);
                        self.project.documents.push(doc);
                        self.mark();
                    }
                }
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
                            }
                        } else {
                            Background::Image { path: path.clone() }
                        };
                        self.mark();
                    }
                }
            }
        }
    }
    fn shortcuts(&mut self, ctx: &Context) {
        let input = ctx.input(|i| i.clone());
        let modifiers = input.modifiers;
        let command = modifiers.command || modifiers.ctrl;
        let typing = ctx.memory(|m| m.focused().is_some()) || self.modal;
        if typing {
            return;
        }
        if command && input.key_pressed(Key::S) {
            self.save(modifiers.shift);
            return;
        }
        if command && input.key_pressed(Key::O) {
            self.request(Pending::Open);
            return;
        }
        if command && input.key_pressed(Key::N) {
            self.request(Pending::New);
            return;
        }
        if input.key_pressed(Key::Enter)
            && let Some(l) = self.selected_live()
        {
            self.trigger(l.document_id, l.slide.id)
        }
        if (input.key_pressed(Key::ArrowRight) || input.key_pressed(Key::Space))
            && let Err(e) = self.runtime.next(&self.project).map(|_| self.sync_media())
        {
            self.error = e.to_string()
        }
        if input.key_pressed(Key::ArrowLeft)
            && let Err(e) = self
                .runtime
                .previous(&self.project)
                .map(|_| self.sync_media())
        {
            self.error = e.to_string()
        }
        if input.key_pressed(Key::B) {
            self.runtime.output.blackout = !self.runtime.output.blackout;
        }
        for n in 1..=9 {
            let key = match n {
                1 => Key::Num1,
                2 => Key::Num2,
                3 => Key::Num3,
                4 => Key::Num4,
                5 => Key::Num5,
                6 => Key::Num6,
                7 => Key::Num7,
                8 => Key::Num8,
                _ => Key::Num9,
            };
            if input.key_pressed(key) {
                let cue = self
                    .doc()
                    .and_then(|d| d.slides.get(n - 1).map(|s| (d.id, s.id)));
                if let Some((d, s)) = cue {
                    self.trigger(d, s)
                }
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
        let _ = frame;
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
        self.poll_files();
        self.media.poll(ctx);
        let now = Instant::now();
        self.runtime
            .tick(&self.project, now.saturating_duration_since(self.last_tick));
        self.last_tick = now;
        self.fullscreen_audience = self.audience_fullscreen_state.load(Ordering::Relaxed);
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
        self.toolbar(ctx);
        egui::SidePanel::left("library")
            .default_width(242.0)
            .width_range(220.0..=310.0)
            .resizable(true)
            .frame(crate::style::panel_frame())
            .show(ctx, |ui| crate::panels::library(self, ui));
        egui::SidePanel::right("inspector")
            .default_width(352.0)
            .width_range(330.0..=460.0)
            .resizable(true)
            .frame(crate::style::panel_frame())
            .show(ctx, |ui| crate::panels::inspector(self, ui));
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
                        ui.colored_label(Color32::from_rgb(244, 118, 117), &self.error);
                    }
                    if !self.media.status.is_empty() {
                        ui.label(&self.media.status);
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(crate::style::CANVAS)
                    .inner_margin(22),
            )
            .show(ctx, |ui| crate::panels::slides(self, ui));
        if self.modal || self.active_save.is_some() {
            self.unsaved_modal(ctx)
        }
        if let Some(id) = self.confirm_delete {
            self.delete_modal(ctx, id)
        }
        self.sync_media();
        self.publish();
        self.show_outputs(ctx);
        if self.close_approved {
            self.close_approved = false;
            self.allow_close = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if self
            .runtime
            .timer_states
            .values()
            .any(|timer| timer.running)
            || self.media.player.as_ref().is_some_and(|p| !p.is_finished())
            || self.media.pending
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
    }
    fn on_exit(&mut self) {
        self.media.shutdown();
    }
}
impl LumenApp {
    fn toolbar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("toolbar")
            .exact_height(106.0)
            .frame(
                egui::Frame::new()
                    .fill(crate::style::PANEL)
                    .inner_margin(egui::Margin::symmetric(20, 12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Lumen")
                            .size(24.0)
                            .strong()
                            .color(crate::style::ACCENT),
                    );
                    ui.separator();
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.project.name)
                                .font(egui::FontId::proportional(21.0))
                                .frame(false)
                                .hint_text("Show name")
                                .desired_width(430.0),
                        )
                        .on_hover_text("Rename this show")
                        .changed()
                    {
                        self.mark();
                    }
                    if self.dirty {
                        ui.label(
                            egui::RichText::new("Unsaved changes")
                                .size(12.0)
                                .color(crate::style::LIVE),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Shortcuts").clicked() {
                            self.show_help = !self.show_help;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for (label, act) in [
                        ("New", 0),
                        ("Open", 1),
                        ("Save", 2),
                        ("Save As", 3),
                        ("Import Text", 4),
                    ] {
                        if ui.button(label).clicked() {
                            match act {
                                0 => self.request(Pending::New),
                                1 => self.request(Pending::Open),
                                2 => self.save(false),
                                3 => self.save(true),
                                _ => file::launch(
                                    FileAction::Import,
                                    self.tx.clone(),
                                    self.repaint_ctx.clone(),
                                ),
                            }
                        }
                    }
                    ui.separator();
                    if ui
                        .selectable_label(self.show_audience, "Audience")
                        .clicked()
                    {
                        let open = !self.show_audience;
                        set_output_open(
                            &mut self.show_audience,
                            &self.audience_close_requested,
                            open,
                        );
                    }
                    if ui.selectable_label(self.show_stage, "Stage").clicked() {
                        let open = !self.show_stage;
                        set_output_open(&mut self.show_stage, &self.stage_close_requested, open);
                    }
                    ui.menu_button("Output settings", |ui| {
                        if ui
                            .checkbox(&mut self.fullscreen_audience, "Audience fullscreen")
                            .changed()
                        {
                            self.audience_fullscreen_state
                                .store(self.fullscreen_audience, Ordering::Relaxed);
                        }
                        if ui
                            .checkbox(&mut self.fullscreen_stage, "Stage fullscreen")
                            .changed()
                        {
                            self.stage_fullscreen_state
                                .store(self.fullscreen_stage, Ordering::Relaxed);
                        }
                        ui.small("Escape returns an output to its window.");
                    });
                });
            });
        if self.show_help {
            egui::Window::new("Keyboard shortcuts")
                .open(&mut self.show_help)
                .show(ctx, |ui| {
                    ui.label("Cmd/Ctrl+S Save | Cmd/Ctrl+Shift+S Save As | Cmd/Ctrl+O Open | Cmd/Ctrl+N New");
                    ui.label("Enter Go Live | Space/Right Next | Left Previous | B Blackout | 1-9 cue");
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
                    ui.colored_label(Color32::from_rgb(244, 118, 117), &self.error);
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
        let data = self.published.clone();
        if self.show_audience {
            let viewport_id = ViewportId::from_hash_of("audience");
            ctx.request_repaint_of(viewport_id);
            let close_cb = self.audience_close_requested.clone();
            let root_ctx = ctx.clone();
            let fullscreen_state = self.audience_fullscreen_state.clone();
            ctx.show_viewport_deferred(
                viewport_id,
                egui::ViewportBuilder::default()
                    .with_title("Lumen - Audience")
                    .with_inner_size([960.0, 540.0])
                    .with_fullscreen(self.fullscreen_audience),
                move |ctx, _| {
                    let state = data.lock().map(|s| s.clone()).unwrap_or_default();
                    if state.media_animating {
                        ctx.request_repaint_after(std::time::Duration::from_millis(80));
                    }
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE)
                        .show(ctx, |ui| {
                            let r = ui.max_rect();
                            crate::render::paint(
                                ui,
                                r,
                                state.output.live.as_ref(),
                                state.output.blackout,
                                state.output.clear_text,
                                state.media.as_ref(),
                            );
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
        if self.show_stage {
            let viewport_id = ViewportId::from_hash_of("stage");
            ctx.request_repaint_of(viewport_id);
            let close_cb = self.stage_close_requested.clone();
            let root_ctx = ctx.clone();
            let fullscreen_state = self.stage_fullscreen_state.clone();
            ctx.show_viewport_deferred(
                viewport_id,
                egui::ViewportBuilder::default()
                    .with_title("Lumen - Stage")
                    .with_inner_size([700.0, 600.0])
                    .with_fullscreen(self.fullscreen_stage),
                move |ctx, _| {
                    let state = data.lock().map(|s| s.clone()).unwrap_or_default();
                    if state.media_animating || state.runtime.timers.iter().any(|t| t.2) {
                        ctx.request_repaint_after(std::time::Duration::from_millis(80));
                    }
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading("STAGE");
                        ui.separator();
                        ui.columns(2, |cols| {
                            cols[0].label("LIVE");
                            if state.output.blackout {
                                cols[0].colored_label(
                                    Color32::from_rgb(244, 118, 117),
                                    "BLACKOUT | stage preview remains available",
                                );
                            }
                            crate::render::preview_frame(
                                &mut cols[0],
                                state.output.live.as_ref(),
                                false,
                                state.output.clear_text,
                                state.media.as_ref(),
                                230.0,
                            );
                            cols[1].label("NEXT");
                            crate::render::preview_frame(
                                &mut cols[1],
                                state.project.next.as_ref(),
                                false,
                                false,
                                None,
                                230.0,
                            );
                        });
                        ui.separator();
                        ui.heading("Stage message");
                        ui.label(&state.runtime.message);
                        ui.heading("Notes");
                        ui.label(
                            state
                                .output
                                .live
                                .as_ref()
                                .map(|l| l.slide.notes.as_str())
                                .unwrap_or("No cue live"),
                        );
                        ui.heading("Timers");
                        for (name, time, _) in state.runtime.timers {
                            ui.horizontal(|ui| {
                                ui.label(name);
                                ui.strong(time);
                            });
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
