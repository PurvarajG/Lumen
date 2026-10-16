use egui::Context;
use lumen_core::{Project, load_project, save_project};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

pub enum FileResult {
    Open(u64, Option<Result<(PathBuf, Project), String>>),
    Save(u64, Result<(PathBuf, Project), String>),
    Import(u64, Option<Result<lumen_interchange::ImportReport, String>>),
    Export(Result<(), String>),
    Recovery(
        u64,
        Option<PathBuf>,
        Result<Option<crate::recovery::Recovered>, String>,
    ),
    Autosave(u64, u64, PathBuf, Result<(), String>),
    PickMedia(Option<PathBuf>, bool),
}
pub enum FileAction {
    Open(u64),
    Save(u64, PathBuf, Box<Project>),
    Import(u64, uuid::Uuid),
    Export(lumen_core::Document),
    Recovery(u64, Option<PathBuf>),
    Autosave(u64, u64, PathBuf, Box<Project>),
    PickMedia(bool),
}

pub fn launch(action: FileAction, tx: Sender<FileResult>, repaint_ctx: Context) {
    thread::spawn(move || {
        match action {
            FileAction::Open(id) => {
                let result = rfd::FileDialog::new()
                    .add_filter("Lumen project", &["lumen.json", "json"])
                    .pick_file()
                    .map(|p| {
                        load_project(&p)
                            .map(|project| (p, project))
                            .map_err(|e| e.to_string())
                    });
                let _ = tx.send(FileResult::Open(id, result));
            }
            FileAction::Save(id, path, project) => {
                let result = save_project(&path, &project)
                    .map(|_| (path, *project))
                    .map_err(|e| e.to_string());
                let _ = tx.send(FileResult::Save(id, result));
            }
            FileAction::Import(id, theme) => {
                let result = rfd::FileDialog::new()
                    .add_filter("Presentation", &["txt", "xml", "openlyrics", "pro"])
                    .pick_file()
                    .map(|path| {
                        lumen_interchange::import_file(&path, theme).map_err(|e| e.to_string())
                    });
                let _ = tx.send(FileResult::Import(id, result));
            }
            FileAction::Export(document) => {
                let result = rfd::FileDialog::new()
                    .add_filter("OpenLyrics", &["xml"])
                    .set_file_name(format!("{}.xml", document.name))
                    .save_file()
                    .map(|path| {
                        let xml = lumen_interchange::export_openlyrics(&document)
                            .map_err(|e| e.to_string())?;
                        std::fs::write(path, xml).map_err(|e| e.to_string())
                    })
                    .unwrap_or_else(|| Err("Export cancelled".into()));
                let _ = tx.send(FileResult::Export(result));
            }
            FileAction::Recovery(id, path) => {
                let result = crate::recovery::check(path.as_deref());
                let _ = tx.send(FileResult::Recovery(id, path, result));
            }
            FileAction::Autosave(id, epoch, path, project) => {
                let result = save_project(&path, &project).map_err(|e| e.to_string());
                let _ = tx.send(FileResult::Autosave(id, epoch, path, result));
            }
            FileAction::PickMedia(video) => {
                let result = if video {
                    rfd::FileDialog::new()
                        .add_filter("Video", &["mp4", "mov", "m4v", "webm"])
                        .pick_file()
                } else {
                    rfd::FileDialog::new()
                        .add_filter("Image", &["png", "jpg", "jpeg", "webp"])
                        .pick_file()
                };
                let _ = tx.send(FileResult::PickMedia(result, video));
            }
        }
        repaint_ctx.request_repaint();
    });
}

pub fn channel() -> (Sender<FileResult>, Receiver<FileResult>) {
    mpsc::channel()
}
