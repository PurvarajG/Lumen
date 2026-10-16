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
    Import(Option<Result<(String, String), String>>),
    PickMedia(Option<PathBuf>, bool),
}
pub enum FileAction {
    Open(u64),
    Save(u64, PathBuf, Project),
    Import,
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
                    .map(|_| (path, project))
                    .map_err(|e| e.to_string());
                let _ = tx.send(FileResult::Save(id, result));
            }
            FileAction::Import => {
                let result = rfd::FileDialog::new()
                    .add_filter("Text", &["txt", "md"])
                    .pick_file()
                    .map(|p| {
                        let name = p
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Imported text")
                            .to_owned();
                        use std::io::Read;
                        let mut bytes = Vec::new();
                        std::fs::File::open(p)
                            .map_err(|e| e.to_string())?
                            .take(4 * 1024 * 1024 + 1)
                            .read_to_end(&mut bytes)
                            .map_err(|e| e.to_string())?;
                        if bytes.len() > 4 * 1024 * 1024 {
                            return Err("Text import is limited to 4 MiB".into());
                        }
                        String::from_utf8(bytes)
                            .map(|text| (name, text))
                            .map_err(|e| format!("text file is not UTF-8: {e}"))
                    });
                let _ = tx.send(FileResult::Import(result));
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
