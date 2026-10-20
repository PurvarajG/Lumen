use egui::Context;
use lumen_core::{Project, load_project, save_project};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

pub enum FileResult {
    Open(u64, Option<Result<(PathBuf, Project), String>>),
    /// Sent before the matching `Save` of a "Save with media": the project as it was copied and
    /// with its paths pointing at the copies, plus what to tell the operator.
    MediaCopied(u64, Box<Project>, Box<Project>, String),
    MediaProgress(u64, usize, usize),
    MissingMedia(u64, Vec<PathBuf>),
    /// The files under the folder the operator chose to relink from; `None` if cancelled.
    RelinkIndex(u64, Option<lumen_core::portable::FileIndex>),
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
    /// Opens `Some(path)`, or asks for one.
    Open(u64, Option<PathBuf>),
    SaveWithMedia(u64, Box<Project>),
    CheckMedia(u64, Box<Project>),
    PickRelinkFolder(u64),
    Save(u64, PathBuf, Box<Project>),
    Import(u64, uuid::Uuid),
    Export(lumen_core::Document),
    Recovery(u64, Option<PathBuf>),
    Autosave(u64, u64, PathBuf, Box<Project>),
    PickMedia(bool),
    /// Add files to the running order; `None` paths opens the multi-select picker.
    Add(u64, Option<Vec<PathBuf>>, uuid::Uuid, lumen_core::MediaFit),
}

pub fn launch(action: FileAction, tx: Sender<FileResult>, repaint_ctx: Context) {
    thread::spawn(move || {
        match action {
            FileAction::Open(id, path) => {
                let result = path
                    .or_else(|| {
                        rfd::FileDialog::new()
                            .add_filter("Lumen project", &["json"])
                            .pick_file()
                    })
                    .map(|p| {
                        load_project(&p)
                            .map(|project| (p, project))
                            .map_err(|e| e.to_string())
                    });
                let _ = tx.send(FileResult::Open(id, result));
            }
            FileAction::SaveWithMedia(id, project) => {
                let chosen = rfd::FileDialog::new()
                    .set_file_name("Show")
                    .set_title("Save with media: name the show folder")
                    .save_file();
                let result = chosen.map(|picked| {
                    let folder = show_folder_for(&picked);
                    let saved = save_with_media(&project, &folder, |done, total| {
                        let _ = tx.send(FileResult::MediaProgress(id, done, total));
                        repaint_ctx.request_repaint();
                    })?;
                    let note = missing_note(&saved.missing);
                    let _ = tx.send(FileResult::MediaCopied(
                        id,
                        project,
                        Box::new(saved.project.clone()),
                        note,
                    ));
                    Ok((saved.show, saved.project))
                });
                let _ = tx.send(FileResult::Save(
                    id,
                    result.unwrap_or_else(|| Err("Save cancelled".into())),
                ));
            }
            FileAction::CheckMedia(epoch, mut project) => {
                let _ = tx.send(FileResult::MissingMedia(
                    epoch,
                    lumen_core::portable::missing_paths(&mut project),
                ));
            }
            FileAction::PickRelinkFolder(epoch) => {
                let index = rfd::FileDialog::new()
                    .set_title("Locate the folder with the missing media")
                    .pick_folder()
                    .map(|root| lumen_core::portable::index_files(&root));
                let _ = tx.send(FileResult::RelinkIndex(epoch, index));
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
                // The Recovery folder is not created anywhere else; without it every autosave fails.
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
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
            FileAction::Add(id, paths, theme, fit) => {
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
                        fit,
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

/// The folder a "Save with media" creates for the name typed in the dialog: `Show`,
/// `Show.lumen.json` and `Show.json` all mean the folder `Show` next to it.
fn show_folder_for(picked: &std::path::Path) -> PathBuf {
    let name = picked.file_name().unwrap_or_default().to_string_lossy();
    let stem = name
        .strip_suffix(".lumen.json")
        .or_else(|| name.strip_suffix(".json"))
        .unwrap_or(&name);
    picked.with_file_name(stem)
}

pub struct MediaSaved {
    pub show: PathBuf,
    pub project: Project,
    pub missing: Vec<PathBuf>,
}

/// Creates `folder` (which must not exist) holding `<name>.lumen.json` and a `Media` folder with
/// a copy of every file the show uses, and saves the show pointing at those copies.
pub fn save_with_media(
    original: &Project,
    folder: &std::path::Path,
    mut progress: impl FnMut(usize, usize),
) -> Result<MediaSaved, String> {
    if folder.exists() {
        return Err(format!(
            "\u{201c}{}\u{201d} already exists; choose a new name",
            folder.display()
        ));
    }
    let name = folder.file_name().unwrap_or_default().to_string_lossy();
    let media = folder.join("Media");
    let mut attempt = || {
        std::fs::create_dir_all(&media)
            .map_err(|e| format!("creating {}: {e}", media.display()))?;
        let mut project = original.clone();
        let plan = lumen_core::portable::plan_media_copies(&mut project, &media, |p| p.is_file());
        let total = plan.copies.len();
        for (done, (from, to)) in plan.copies.iter().enumerate() {
            progress(done, total);
            std::fs::copy(from, to).map_err(|e| format!("copying {}: {e}", from.display()))?;
        }
        let show = folder.join(format!("{name}.lumen.json"));
        save_project(&show, &project).map_err(|e| e.to_string())?;
        Ok(MediaSaved {
            show,
            project,
            missing: plan.missing,
        })
    };
    attempt().inspect_err(|_| {
        // The folder did not exist before this call, so nothing of the operator's is lost.
        let _ = std::fs::remove_dir_all(folder);
    })
}

fn missing_note(missing: &[PathBuf]) -> String {
    if missing.is_empty() {
        return String::new();
    }
    let names: Vec<_> = missing
        .iter()
        .take(5)
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy())
        .collect();
    let more = missing.len().saturating_sub(5);
    format!(
        "{} file{} missing and not copied: {}{}",
        missing.len(),
        if missing.len() == 1 { " was" } else { "s were" },
        names.join(", "),
        if more > 0 {
            format!(" and {more} more")
        } else {
            String::new()
        }
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use lumen_core::{Background, MediaFit};
    use std::path::Path;

    #[test]
    fn save_with_media_copies_files_and_writes_a_relocatable_show() {
        let root = std::env::temp_dir().join(format!("lumen-file-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let dir = &root;
        let photo = dir.join("photo.png");
        std::fs::write(&photo, b"png").unwrap();
        let mut project = Project::demo();
        project.documents[0].slides[0].background = Background::Image {
            path: photo.clone(),
            fit: MediaFit::default(),
        };
        project.documents[0].slides[1].background = Background::Video {
            path: dir.join("gone.mp4"),
            looped: false,
            muted: false,
            fit: MediaFit::default(),
        };
        let folder = dir.join("Sabha");
        let saved = save_with_media(&project, &folder, |_, _| {}).unwrap();
        assert_eq!(saved.show, folder.join("Sabha.lumen.json"));
        assert_eq!(saved.missing.len(), 1);
        assert_eq!(
            std::fs::read(folder.join("Media/photo.png")).unwrap(),
            b"png"
        );

        // The folder moves; the show still finds its media.
        let moved = dir.join("Moved");
        std::fs::rename(&folder, &moved).unwrap();
        let reopened = load_project(&moved.join("Sabha.lumen.json")).unwrap();
        let Background::Image { path, .. } = &reopened.documents[0].slides[0].background else {
            panic!("photo background")
        };
        assert!(path.is_file(), "{path:?}");

        // An existing folder is never touched.
        assert!(save_with_media(&project, &moved, |_, _| {}).is_err());
        assert!(moved.join("Media/photo.png").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_folder_name_drops_the_show_extension() {
        for typed in ["/x/Show", "/x/Show.lumen.json", "/x/Show.json"] {
            assert_eq!(show_folder_for(Path::new(typed)), Path::new("/x/Show"));
        }
        assert_eq!(
            missing_note(&[PathBuf::from("/a/b.mp4")]),
            "1 file was missing and not copied: b.mp4"
        );
    }
}
