//! Build a show from files: pure planning, folder expansion, document building,
//! running-order insert/move/remove, and drag-slot maths. Disk work runs on the file worker.
use lumen_core::{AudioCue, Background, CueAction, Document, PlaylistItem, Project, Slide};
use lumen_interchange::ImportReport;
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const EXTENSIONS: [&str; 22] = [
    "png",
    "jpg",
    "jpeg",
    "webp",
    "mp4",
    "mov",
    "m4v",
    "webm",
    "pdf",
    "pptx",
    "ppt",
    "key",
    "txt",
    "xml",
    "openlyrics",
    "pro",
    "mp3",
    "wav",
    "flac",
    "m4a",
    "aac",
    "ogg",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Image,
    Video,
    Pdf,
    Deck,
    Text,
    Audio,
}
impl Kind {
    /// Phosphor private-use codepoints (regular weight).
    pub fn icon(self) -> char {
        match self {
            Kind::Image => '\u{e2ca}',
            Kind::Video => '\u{e740}',
            Kind::Pdf => '\u{e702}',
            Kind::Deck => '\u{e654}',
            Kind::Text => '\u{e23a}',
            Kind::Audio => '\u{e33c}',
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Addition {
    Add(Kind, PathBuf),
    Skip(PathBuf),
}

#[derive(Debug, Default)]
pub struct AddBatch {
    pub documents: Vec<Document>,
    pub skipped: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn kind_of(path: &Path) -> Option<Kind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => Kind::Image,
        "mp4" | "mov" | "m4v" | "webm" => Kind::Video,
        "pdf" => Kind::Pdf,
        "pptx" | "ppt" | "key" => Kind::Deck,
        "txt" | "xml" | "openlyrics" | "pro" => Kind::Text,
        "mp3" | "wav" | "flac" | "m4a" | "aac" | "ogg" => Kind::Audio,
        _ => return None,
    })
}

pub fn plan_additions(paths: &[PathBuf]) -> Vec<Addition> {
    paths
        .iter()
        .map(|p| match kind_of(p) {
            Some(kind) => Addition::Add(kind, p.clone()),
            None => Addition::Skip(p.clone()),
        })
        .collect()
}

/// Folders become their supported files (non-recursive, hidden files ignored, natural order).
/// A `.key` Keynote package is a directory but is a single file to the user.
pub fn expand_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in paths {
        if path.is_dir() && kind_of(path).is_none() {
            let mut files: Vec<PathBuf> = std::fs::read_dir(path)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| kind_of(p).is_some() && !file_name(p).starts_with('.'))
                .collect();
            files.sort_by(|a, b| natural_cmp(&file_name(a), &file_name(b)));
            out.extend(files);
        } else {
            out.push(path.clone());
        }
    }
    out
}

pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    std::iter::from_fn(|| it.next_if(|c| c.is_ascii_digit())).collect::<String>()
                };
                let (n, m) = (take(&mut a), take(&mut b));
                let (n, m) = (n.trim_start_matches('0'), m.trim_start_matches('0'));
                match n.len().cmp(&m.len()).then_with(|| n.cmp(m)) {
                    Ordering::Equal => {}
                    other => return other,
                }
            }
            (Some(x), Some(y)) => {
                match x.to_lowercase().cmp(y.to_lowercase()) {
                    Ordering::Equal => {}
                    other => return other,
                }
                a.next();
                b.next();
            }
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// One document per file. PDF, deck and text files reuse the File > Import paths.
fn build_with_progress(
    kind: Kind,
    path: &Path,
    theme: Uuid,
    progress: impl FnMut(&str),
) -> Result<ImportReport, String> {
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let (label, background, audio) = match kind {
        Kind::Image => (
            stem.clone(),
            Background::Image {
                path: path.to_owned(),
            },
            None,
        ),
        Kind::Video => (
            stem.clone(),
            Background::Video {
                path: path.to_owned(),
                looped: false,
                muted: false,
            },
            None,
        ),
        Kind::Audio => (
            file_name(path),
            Background::Solid([0, 0, 0, 255]),
            Some(AudioCue {
                path: path.to_owned(),
                volume: 1.0,
            }),
        ),
        Kind::Pdf | Kind::Deck | Kind::Text => {
            return crate::file::import_presentation_with_progress(path, theme, progress);
        }
    };
    let mut slide = Slide::new(theme);
    slide.label = label;
    slide.background = background;
    slide.audio = audio;
    let mut document = Document::new(stem);
    document.slides.push(slide);
    Ok(ImportReport {
        document,
        warnings: Vec::new(),
    })
}

/// Runs on the file worker. `progress(done, total)` is called before each item.
pub fn build_batch(
    plan: &[Addition],
    theme: Uuid,
    mut progress: impl FnMut(usize, usize),
    mut deck_progress: impl FnMut(&str),
) -> AddBatch {
    let total = plan
        .iter()
        .filter(|a| matches!(a, Addition::Add(..)))
        .count();
    let mut batch = AddBatch::default();
    for addition in plan {
        match addition {
            Addition::Skip(path) => batch.skipped.push(file_name(path)),
            Addition::Add(kind, path) => {
                progress(batch.documents.len() + 1, total);
                match build_with_progress(*kind, path, theme, &mut deck_progress) {
                    Ok(report) => {
                        batch.warnings.extend(report.warnings);
                        batch.documents.push(report.document);
                    }
                    Err(error) => {
                        batch.skipped.push(file_name(path));
                        batch.warnings.push(format!("{}: {error}", file_name(path)));
                    }
                }
            }
        }
    }
    batch
}

pub fn batch_notice(added: usize, skipped: &[String]) -> String {
    let mut text = format!("Added {added} item{}", if added == 1 { "" } else { "s" });
    if !skipped.is_empty() {
        let shown = skipped
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let more = skipped.len().saturating_sub(3);
        text += &format!(
            " · skipped {} ({shown}{})",
            skipped.len(),
            if more > 0 {
                format!(" +{more} more")
            } else {
                String::new()
            }
        );
    }
    text
}

/// Insert after the running-order item showing `after`, else at the end. One mutation = one undo step.
pub fn insert_documents(project: &mut Project, after: Uuid, documents: Vec<Document>) {
    let at = project
        .playlist
        .iter()
        .position(|p| p.document_id == after)
        .map_or(project.playlist.len(), |i| i + 1);
    let items: Vec<_> = documents
        .iter()
        .map(|d| PlaylistItem {
            id: Uuid::new_v4(),
            document_id: d.id,
        })
        .collect();
    project.playlist.splice(at..at, items);
    project.documents.extend(documents);
}

/// Move the item at `from` so it ends at index `to`. False when out of bounds or a no-op.
pub fn move_item<T>(list: &mut Vec<T>, from: usize, to: usize) -> bool {
    if from >= list.len() || to >= list.len() || from == to {
        return false;
    }
    let item = list.remove(from);
    list.insert(to, item);
    true
}

/// Turn an insertion slot (0..=len, between rows) into the final index for `move_item`.
pub fn slot_to_index(from: usize, slot: usize) -> usize {
    if slot > from { slot - 1 } else { slot }
}

/// Removes from the running order only; the document stays in the library.
pub fn remove_playlist_item(project: &mut Project, index: usize) -> bool {
    if index >= project.playlist.len() {
        return false;
    }
    project.playlist.remove(index);
    true
}

/// Insertion slot for a vertical list: before the first row whose middle is below the pointer.
pub fn list_slot(rows: &[egui::Rect], pointer: egui::Pos2) -> usize {
    rows.iter()
        .position(|r| pointer.y < r.center().y)
        .unwrap_or(rows.len())
}

/// Drop target in a wrapped grid: the nearest tile and whether the pointer is on its right half.
/// The insertion slot is `nearest + after as usize`.
pub fn grid_target(tiles: &[egui::Rect], pointer: egui::Pos2) -> Option<(usize, bool)> {
    let d = |r: &egui::Rect| {
        let c = r.center();
        (c.x - pointer.x).powi(2) + (c.y - pointer.y).powi(2)
    };
    let (i, tile) = tiles
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| d(a).total_cmp(&d(b)))?;
    Some((i, pointer.x >= tile.center().x))
}

/// Icon/type of a document, from its first slide.
pub fn doc_kind(doc: &Document) -> Kind {
    let Some(slide) = doc.slides.first() else {
        return Kind::Text;
    };
    if slide
        .actions
        .iter()
        .any(|a| matches!(a, CueAction::PlayDeck { .. }))
    {
        return Kind::Deck;
    }
    match &slide.background {
        Background::Pdf { .. } => Kind::Pdf,
        Background::Image { .. } => Kind::Image,
        Background::Video { .. } => Kind::Video,
        Background::Solid(_) if slide.audio.is_some() && slide.text.is_empty() => Kind::Audio,
        Background::Solid(_) => Kind::Text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumen_core::history::History;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn plan_maps_extensions_case_insensitively_and_skips_the_rest() {
        let plan = plan_additions(&[
            p("/a/Photo.PNG"),
            p("/a/b.JpEg"),
            p("/a/c.webp"),
            p("/a/Clip.MOV"),
            p("/a/d.m4v"),
            p("/a/e.PDF"),
            p("/a/f.PPTX"),
            p("/a/g.Key"),
            p("/a/h.TXT"),
            p("/a/i.pro"),
            p("/a/j.FLAC"),
            p("/a/k.ogg"),
            p("/a/notes.docx"),
            p("/a/clip.avi"),
            p("/a/README"),
        ]);
        let kinds: Vec<_> = plan
            .iter()
            .map(|a| match a {
                Addition::Add(k, _) => Some(*k),
                Addition::Skip(_) => None,
            })
            .collect();
        use Kind::*;
        assert_eq!(
            kinds,
            vec![
                Some(Image),
                Some(Image),
                Some(Image),
                Some(Video),
                Some(Video),
                Some(Pdf),
                Some(Deck),
                Some(Deck),
                Some(Text),
                Some(Text),
                Some(Audio),
                Some(Audio),
                None,
                None,
                None
            ]
        );
        assert_eq!(plan[12], Addition::Skip(p("/a/notes.docx")));
    }

    #[test]
    fn natural_order_puts_2_before_10() {
        let mut names = vec!["10.png", "2.png", "1.png", "02b.png", "A.png", "b.png"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            names,
            vec!["1.png", "2.png", "02b.png", "10.png", "A.png", "b.png"]
        );
    }

    #[test]
    fn folder_expands_to_supported_files_in_natural_order() {
        let dir = std::env::temp_dir().join(format!("lumen-add-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["10.png", "2.png", "1.PNG", ".DS_Store", "notes.docx"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        let loose = p("/elsewhere/a.mp4");
        let names: Vec<_> = expand_paths(&[loose.clone(), dir.clone()])
            .iter()
            .map(|p| file_name(p))
            .collect();
        assert_eq!(names, vec!["a.mp4", "1.PNG", "2.png", "10.png"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn keynote_package_directory_is_one_file() {
        let dir = std::env::temp_dir().join(format!("lumen-{}.key", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Index.zip"), b"x").unwrap();
        assert_eq!(expand_paths(std::slice::from_ref(&dir)), vec![dir.clone()]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn media_files_become_single_slide_documents() {
        let theme = Uuid::new_v4();
        let image =
            build_with_progress(Kind::Image, Path::new("/m/Welcome.png"), theme, |_| {}).unwrap();
        let slide = &image.document.slides[0];
        assert_eq!(image.document.name, "Welcome");
        assert_eq!(image.document.slides.len(), 1);
        assert_eq!(slide.theme_id, theme);
        assert!(slide.text.is_empty());
        assert_eq!(
            slide.background,
            Background::Image {
                path: p("/m/Welcome.png")
            }
        );
        let video =
            build_with_progress(Kind::Video, Path::new("/m/Intro.mp4"), theme, |_| {}).unwrap();
        assert_eq!(
            video.document.slides[0].background,
            Background::Video {
                path: p("/m/Intro.mp4"),
                looped: false,
                muted: false
            }
        );
        let audio =
            build_with_progress(Kind::Audio, Path::new("/m/Song.mp3"), theme, |_| {}).unwrap();
        let slide = &audio.document.slides[0];
        assert_eq!(slide.label, "Song.mp3");
        assert_eq!(slide.background, Background::Solid([0, 0, 0, 255]));
        assert_eq!(
            slide.audio,
            Some(AudioCue {
                path: p("/m/Song.mp3"),
                volume: 1.0
            })
        );
        assert_eq!(doc_kind(&audio.document), Kind::Audio);
        assert_eq!(doc_kind(&video.document), Kind::Video);
        assert_eq!(doc_kind(&image.document), Kind::Image);
    }

    #[test]
    fn batch_reports_progress_and_names_skipped_files() {
        let theme = Uuid::new_v4();
        let plan = plan_additions(&[p("/m/a.png"), p("/m/notes.docx"), p("/m/b.mp3")]);
        let mut seen = Vec::new();
        let batch = build_batch(&plan, theme, |done, total| seen.push((done, total)), |_| {});
        assert_eq!(seen, vec![(1, 2), (2, 2)]);
        assert_eq!(batch.documents.len(), 2);
        assert_eq!(batch.skipped, vec!["notes.docx"]);
        assert_eq!(
            batch_notice(7, &["notes.docx".into(), "clip.avi".into()]),
            "Added 7 items · skipped 2 (notes.docx, clip.avi)"
        );
        assert_eq!(batch_notice(1, &[]), "Added 1 item");
        assert!(
            batch_notice(0, &["a".into(), "b".into(), "c".into(), "d".into()])
                .ends_with("(a, b, c +1 more)")
        );
    }

    fn docs(n: usize, theme: Uuid) -> Vec<Document> {
        (0..n)
            .map(|i| {
                let mut d = Document::new(format!("new {i}"));
                d.slides.push(Slide::new(theme));
                d
            })
            .collect()
    }

    #[test]
    fn insert_goes_after_selection_or_at_end_and_stays_valid() {
        let mut project = Project::demo();
        let order: Vec<Uuid> = project.playlist.iter().map(|i| i.document_id).collect();
        assert!(order.len() >= 2);
        let theme = project.themes[0].id;
        let new = docs(2, theme);
        let new_ids: Vec<Uuid> = new.iter().map(|d| d.id).collect();
        insert_documents(&mut project, order[0], new);
        let after: Vec<Uuid> = project.playlist.iter().map(|i| i.document_id).collect();
        assert_eq!(after[0], order[0]);
        assert_eq!(&after[1..3], &new_ids[..]);
        assert_eq!(&after[3..], &order[1..]);
        project.validate().unwrap();
        let tail = docs(1, theme);
        let tail_id = tail[0].id;
        insert_documents(&mut project, Uuid::nil(), tail);
        assert_eq!(project.playlist.last().unwrap().document_id, tail_id);
        project.validate().unwrap();
    }

    #[test]
    fn a_batch_is_one_undo_step() {
        let mut project = Project::demo();
        let before = project.clone();
        let mut history = History::<Project>::default();
        history.record(project.clone());
        let theme = project.themes[0].id;
        insert_documents(&mut project, Uuid::nil(), docs(5, theme));
        assert_eq!(history.undo(project.clone()), Some(before));
        assert!(!history.can_undo());
    }

    #[test]
    fn move_item_respects_bounds_and_identity() {
        let mut v = vec!['a', 'b', 'c', 'd'];
        assert!(!move_item(&mut v, 1, 1));
        assert!(!move_item(&mut v, 4, 0));
        assert!(!move_item(&mut v, 0, 4));
        assert_eq!(v, vec!['a', 'b', 'c', 'd']);
        assert!(move_item(&mut v, 0, 2));
        assert_eq!(v, vec!['b', 'c', 'a', 'd']);
        assert!(move_item(&mut v, 3, 0));
        assert_eq!(v, vec!['d', 'b', 'c', 'a']);
        assert_eq!(slot_to_index(0, 3), 2);
        assert_eq!(slot_to_index(3, 1), 1);
        assert_eq!(slot_to_index(2, 3), 2);
    }

    #[test]
    fn remove_drops_the_running_order_item_but_keeps_the_document() {
        let mut project = Project::demo();
        let documents = project.documents.len();
        let id = project.playlist[0].document_id;
        let items = project.playlist.len();
        assert!(!remove_playlist_item(&mut project, items));
        assert!(remove_playlist_item(&mut project, 0));
        assert_eq!(project.playlist.len(), items - 1);
        assert_eq!(project.documents.len(), documents);
        assert!(project.document(id).is_some());
        project.validate().unwrap();
    }

    #[test]
    fn drag_slots_follow_the_pointer() {
        let row = |y: f32| egui::Rect::from_min_size(egui::pos2(0.0, y), egui::vec2(200.0, 56.0));
        let rows = [row(0.0), row(60.0), row(120.0)];
        assert_eq!(list_slot(&rows, egui::pos2(10.0, 5.0)), 0);
        assert_eq!(list_slot(&rows, egui::pos2(10.0, 40.0)), 1);
        assert_eq!(list_slot(&rows, egui::pos2(10.0, 400.0)), 3);
        let tile =
            |x: f32, y: f32| egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(100.0, 56.0));
        let tiles = [tile(0.0, 0.0), tile(108.0, 0.0), tile(0.0, 70.0)];
        assert_eq!(
            grid_target(&tiles, egui::pos2(10.0, 10.0)),
            Some((0, false))
        );
        assert_eq!(grid_target(&tiles, egui::pos2(90.0, 10.0)), Some((0, true)));
        assert_eq!(
            grid_target(&tiles, egui::pos2(20.0, 100.0)),
            Some((2, false))
        );
        assert_eq!(
            grid_target(&tiles, egui::pos2(300.0, 10.0)),
            Some((1, true))
        );
        assert_eq!(grid_target(&[], egui::pos2(0.0, 0.0)), None);
    }
}
