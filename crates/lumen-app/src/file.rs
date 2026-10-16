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
    AddProgress(u64, usize, usize),
    DeckProgress(u64, String),
    Added(u64, Option<crate::add::AddBatch>),
}
pub enum FileAction {
    Open(u64),
    Save(u64, PathBuf, Box<Project>),
    Import(u64, uuid::Uuid),
    Export(lumen_core::Document),
    Recovery(u64, Option<PathBuf>),
    Autosave(u64, u64, PathBuf, Box<Project>),
    PickMedia(bool),
    /// Add files to the running order; `None` paths opens the multi-select picker.
    Add(u64, Option<Vec<PathBuf>>, uuid::Uuid),
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
                    .add_filter(
                        "Presentation",
                        &[
                            "txt",
                            "xml",
                            "openlyrics",
                            "pro",
                            "pdf",
                            "ppt",
                            "pptx",
                            "key",
                        ],
                    )
                    .pick_file()
                    .map(|path| {
                        import_presentation_with_progress(&path, theme, |message| {
                            let _ = tx.send(FileResult::DeckProgress(id, message.to_owned()));
                            repaint_ctx.request_repaint();
                        })
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
            FileAction::Add(id, paths, theme) => {
                let picked = paths.or_else(|| {
                    rfd::FileDialog::new()
                        .add_filter("Media, slides, songs and audio", &crate::add::EXTENSIONS)
                        .pick_files()
                });
                let batch = picked.map(|paths| {
                    let plan = crate::add::plan_additions(&crate::add::expand_paths(&paths));
                    crate::add::build_batch(
                        &plan,
                        theme,
                        |done, total| {
                            let _ = tx.send(FileResult::AddProgress(id, done, total));
                            repaint_ctx.request_repaint();
                        },
                        |message| {
                            let _ = tx.send(FileResult::DeckProgress(id, message.to_owned()));
                            repaint_ctx.request_repaint();
                        },
                    )
                });
                let _ = tx.send(FileResult::Added(id, batch));
            }
        }
        repaint_ctx.request_repaint();
    });
}

pub fn channel() -> (Sender<FileResult>, Receiver<FileResult>) {
    mpsc::channel()
}

pub(crate) fn import_presentation_with_progress(
    path: &std::path::Path,
    theme: uuid::Uuid,
    mut progress: impl FnMut(&str),
) -> Result<lumen_interchange::ImportReport, String> {
    use lumen_core::{Background, Document, Slide};
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "pdf" | "ppt" | "pptx" | "key") {
        return lumen_interchange::import_file(path, theme).map_err(|e| e.to_string());
    }
    let mut document = Document::new(path.file_stem().unwrap_or_default().to_string_lossy());
    if extension == "pdf" {
        for page in 1..=lumen_media::pdf_page_count(path).map_err(|e| e.to_string())? {
            let mut slide = Slide::new(theme);
            slide.label = format!("Page {page}");
            slide.background = Background::Pdf {
                path: path.to_owned(),
                page,
            };
            document.slides.push(slide);
        }
    } else {
        let (pdf, notes) = crate::deck::convert_to_pdf(path, &mut progress)?;
        let pages = lumen_media::pdf_page_count(&pdf).map_err(|e| e.to_string())? as usize;
        if notes.len() != pages {
            return Err(format!(
                "Deck notes count {} does not match PDF page count {pages}",
                notes.len()
            ));
        }
        document.source = Some(path.to_owned());
        for (index, note) in notes.into_iter().enumerate() {
            let mut slide = Slide::new(theme);
            slide.label = format!("Slide {}", index + 1);
            slide.background = Background::Pdf {
                path: pdf.clone(),
                page: index as u32 + 1,
            };
            slide.notes = note;
            document.slides.push(slide);
        }
    }
    Ok(lumen_interchange::ImportReport {
        document,
        warnings: Vec::new(),
    })
}
