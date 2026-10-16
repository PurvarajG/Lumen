use crate::{
    app::LumenApp,
    style,
    text::{gu_label, gu_selectable},
};
use lumen_core::{
    Background, Document, LiveSlide, PlaylistItem, Project, Slide, TextAlign, Theme, VerticalAlign,
};
use lumen_library::{
    Book, DownloadEvent, DownloadHandle, Downloader, SearchIndex, SlideOpts, Unit, catalog,
    unit_slides,
};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
    thread,
};
use uuid::Uuid;

#[derive(Default)]
struct DownloadQueue {
    pending: VecDeque<String>,
    active: Option<String>,
}
impl DownloadQueue {
    fn enqueue(&mut self, id: &str) {
        if self.active.as_deref() != Some(id) && !self.pending.iter().any(|p| p == id) {
            self.pending.push_back(id.into());
        }
    }
    fn next(&mut self) -> Option<String> {
        if self.active.is_some() {
            return None;
        }
        self.active = self.pending.pop_front();
        self.active.clone()
    }
    fn finish(&mut self) {
        self.active = None;
    }
    fn cancel_pending(&mut self, id: &str) {
        self.pending.retain(|p| p != id);
    }
    fn saved(&self) -> Vec<String> {
        self.active
            .iter()
            .chain(self.pending.iter())
            .cloned()
            .collect()
    }
}

fn source_key(book: &Book, unit: &Unit) -> String {
    format!(
        "[book:{}]",
        serde_json::to_string(&(&book.id, &unit.key)).expect("string pair")
    )
}
fn scripture_theme() -> Theme {
    Theme {
        name: "Scripture".into(),
        font_size: 54.0,
        text_color: style::TEXT.to_array(),
        background_color: style::CANVAS.to_array(),
        align: TextAlign::Center,
        vertical_align: VerticalAlign::Middle,
        ..Default::default()
    }
}
fn unit_document(book: &Book, unit: &Unit, opts: SlideOpts, theme: Uuid) -> Document {
    let title = unit.title_gu.as_deref().unwrap_or_default();
    let mut doc =
        Document::new(format!("{} {} {}", unit.label, title, source_key(book, unit)).trim());
    // Track the same chunk packing as unit_slides so each footnote follows its block,
    // including split paragraphs, without exposing it in audience text.
    let limit = if opts.max_chars == 0 {
        400
    } else {
        opts.max_chars
    };
    let mut notes = Vec::new();
    let mut current_notes = Vec::new();
    let mut count = 0;
    let mut length = 0;
    for block in &unit.blocks {
        let single = Unit {
            key: String::new(),
            label: unit.label.clone(),
            title_gu: None,
            title_en: None,
            blocks: vec![block.clone()],
        };
        for (_, text) in unit_slides(
            &single,
            SlideOpts {
                blocks_per_slide: 1,
                ..opts
            },
        ) {
            let size: usize = lumen_core::markup::parse_markup(&text)
                .iter()
                .map(|s| s.text.chars().count())
                .sum();
            let separator = if count == 0 { 0 } else { 2 };
            if count > 0
                && (count >= opts.blocks_per_slide.clamp(1, 4) || length + separator + size > limit)
            {
                notes.push(current_notes.join("\n\n"));
                current_notes.clear();
                count = 0;
                length = 0;
            }
            if let Some(note) = block.notes.as_ref().filter(|n| !n.trim().is_empty())
                && !current_notes.contains(note)
            {
                current_notes.push(note.clone());
            }
            length += size + separator;
            count += 1;
        }
    }
    if count > 0 {
        notes.push(current_notes.join("\n\n"));
    }
    for ((label, text), notes) in unit_slides(unit, opts).into_iter().zip(notes) {
        let mut slide = Slide::new(theme);
        slide.label = label;
        slide.text = text;
        slide.notes = notes;
        slide.background = Background::Solid(style::CANVAS.to_array());
        doc.slides.push(slide);
    }
    doc
}
fn add_unit(project: &mut Project, book: &Book, unit: &Unit, opts: SlideOpts) -> (Uuid, bool) {
    let key = source_key(book, unit);
    if let Some(doc) = project.documents.iter().find(|d| d.name.ends_with(&key)) {
        return (doc.id, false);
    }
    let theme = if let Some(t) = project.themes.iter().find(|t| t.name == "Scripture") {
        t.id
    } else {
        let t = scripture_theme();
        let id = t.id;
        project.themes.push(t);
        id
    };
    let doc = unit_document(book, unit, opts, theme);
    let id = doc.id;
    project.documents.push(doc);
    project.playlist.push(PlaylistItem {
        id: Uuid::new_v4(),
        document_id: id,
    });
    (id, true)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    Cue(Uuid, Uuid),
    Book(String, String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    RunningOrder,
    Library,
    Books,
}
#[derive(Clone, Debug)]
struct SearchResult {
    group: Group,
    label: String,
    snippet: String,
    target: Target,
}
fn search_results(
    project: &Project,
    books: &[Book],
    index: Option<&SearchIndex>,
    query: &str,
    recent: &[Target],
) -> Vec<SearchResult> {
    let q = lumen_library::to_nfc(query.trim()).to_lowercase();
    if q.is_empty() {
        return recent
            .iter()
            .filter_map(|target| match target {
                Target::Cue(d, s) => {
                    let doc = project.document(*d)?;
                    let slide = doc.slides.iter().find(|slide| slide.id == *s)?;
                    Some(SearchResult {
                        group: Group::Library,
                        label: document_label(&doc.name).into(),
                        snippet: snippet(&slide.text),
                        target: target.clone(),
                    })
                }
                Target::Book(id, key) => {
                    let book = books.iter().find(|b| b.id == *id)?;
                    let unit = book.units.iter().find(|u| u.key == *key)?;
                    Some(SearchResult {
                        group: Group::Books,
                        label: format!("{} · {}", book.title_en, unit.label),
                        snippet: unit.title_gu.clone().unwrap_or_default(),
                        target: target.clone(),
                    })
                }
            })
            .collect();
    }
    let matches = |text: &str| lumen_library::to_nfc(text).to_lowercase().contains(&q);
    let mut results = Vec::new();
    for (group, docs) in [
        (
            Group::RunningOrder,
            project
                .playlist
                .iter()
                .filter_map(|p| project.document(p.document_id))
                .collect::<Vec<_>>(),
        ),
        (Group::Library, project.documents.iter().collect::<Vec<_>>()),
    ] {
        for doc in docs {
            for slide in &doc.slides {
                if matches(&doc.name) || matches(&slide.label) || matches(&slide.text) {
                    results.push(SearchResult {
                        group,
                        label: format!("{} · {}", document_label(&doc.name), slide.label),
                        snippet: snippet(&slide.text),
                        target: Target::Cue(doc.id, slide.id),
                    });
                }
                if results.iter().filter(|r| r.group == group).count() >= 40 {
                    break;
                }
            }
            if results.iter().filter(|r| r.group == group).count() >= 40 {
                break;
            }
        }
    }
    if let Some(index) = index {
        let mut seen = std::collections::HashSet::new();
        for hit in index.search(query, 80) {
            if !seen.insert((hit.book.clone(), hit.unit_key.clone())) {
                continue;
            }
            let label = books
                .iter()
                .find(|b| b.id == hit.book)
                .and_then(|b| {
                    b.units.iter().find(|u| u.key == hit.unit_key).map(|u| {
                        format!(
                            "{} · {} {}",
                            b.title_en,
                            u.label,
                            u.title_gu.as_deref().unwrap_or_default()
                        )
                    })
                })
                .unwrap_or_else(|| hit.unit_key.clone());
            results.push(SearchResult {
                group: Group::Books,
                label,
                snippet: hit.snippet,
                target: Target::Book(hit.book, hit.unit_key),
            });
        }
    }
    results
}
pub fn document_label(name: &str) -> &str {
    if name.ends_with(']') {
        name.rsplit_once(" [book:")
            .map(|(label, _)| label)
            .unwrap_or(name)
    } else {
        name
    }
}
fn snippet(text: &str) -> String {
    lumen_core::markup::parse_markup(text)
        .into_iter()
        .map(|s| s.text)
        .collect::<String>()
        .split_whitespace()
        .take(16)
        .collect::<Vec<_>>()
        .join(" ")
}
fn move_selection(selected: usize, count: usize, forward: bool) -> usize {
    let last = count.saturating_sub(1);
    let selected = selected.min(last);
    if forward {
        selected.saturating_add(1).min(last)
    } else {
        selected.saturating_sub(1)
    }
}

#[derive(Clone, Debug)]
enum DownloadState {
    NotDownloaded,
    Queued,
    Downloading { done: usize, total: usize },
    Cancelling,
    Downloaded { bytes: u64 },
    Failed(String),
}
struct ActiveDownload {
    id: String,
    handle: Option<DownloadHandle>,
    events: Receiver<DownloadEvent>,
    retired: Option<Receiver<()>>,
}
impl ActiveDownload {
    fn retire(&mut self, ctx: &egui::Context) {
        if let Some(handle) = self.handle.take() {
            let (tx, rx) = mpsc::channel();
            let ctx = ctx.clone();
            thread::spawn(move || {
                drop(handle);
                let _ = tx.send(());
                ctx.request_repaint();
            });
            self.retired = Some(rx);
        }
    }
}
struct Installed {
    books: Arc<Vec<Book>>,
    index: Arc<SearchIndex>,
    sizes: HashMap<String, u64>,
    errors: HashMap<String, String>,
}
pub struct Books {
    root: Option<PathBuf>,
    states: HashMap<String, DownloadState>,
    queue: DownloadQueue,
    active: Option<ActiveDownload>,
    installed: Arc<Vec<Book>>,
    index: Option<Arc<SearchIndex>>,
    loading: Option<Receiver<Installed>>,
    deleting: Option<(String, Receiver<Result<(), String>>)>,
    confirm_delete: Option<String>,
    pub opened: Option<String>,
    unit_filter: String,
    preview: Option<(String, String)>,
    preview_slide: usize,
    preview_doc: Option<(crate::settings::BookLanguage, usize, Document, Theme)>,
    pub search_open: bool,
    selection: usize,
    last_query: String,
    scroll_selection: bool,
    recent: Vec<Target>,
    focus_add: bool,
}
impl Books {
    pub fn new(ctx: &egui::Context, downloads: &[String]) -> Self {
        let root =
            crate::settings::settings_path().and_then(|p| p.parent().map(|p| p.join("Books")));
        Self::at_root(ctx, root, downloads)
    }
    fn at_root(ctx: &egui::Context, root: Option<PathBuf>, downloads: &[String]) -> Self {
        let mut state = Self {
            root,
            states: HashMap::new(),
            queue: DownloadQueue::default(),
            active: None,
            installed: Arc::new(vec![]),
            index: None,
            loading: None,
            deleting: None,
            confirm_delete: None,
            opened: None,
            unit_filter: String::new(),
            preview: None,
            preview_slide: 0,
            preview_doc: None,
            search_open: false,
            selection: 0,
            last_query: String::new(),
            scroll_selection: false,
            recent: vec![],
            focus_add: false,
        };
        for id in downloads
            .iter()
            .filter(|id| catalog().iter().any(|b| b.id == id.as_str()))
        {
            state.queue.enqueue(id);
            state.states.insert(id.clone(), DownloadState::Queued);
        }
        state.reload(ctx);
        state
    }
    fn reload(&mut self, ctx: &egui::Context) {
        let Some(root) = self.root.clone() else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        self.loading = Some(rx);
        thread::spawn(move || {
            let mut books = Vec::new();
            let mut sizes = HashMap::new();
            let mut errors = HashMap::new();
            let disk_sizes: HashMap<_, _> = lumen_library::installed_books(&root)
                .into_iter()
                .map(|(id, _, bytes)| (id, bytes))
                .collect();
            for info in catalog() {
                // units/ is a resumable partial download, never a completed installation.
                let path = root.join(info.id).join("book.json");
                if !path.is_file() {
                    continue;
                }
                match lumen_library::load_book(&root, info.id) {
                    Ok(book)
                        if book.schema == lumen_library::SCHEMA_VERSION && book.id == info.id =>
                    {
                        sizes.insert(
                            book.id.clone(),
                            disk_sizes.get(&book.id).copied().unwrap_or(0),
                        );
                        books.push(book);
                    }
                    Ok(_) => {
                        errors.insert(
                            info.id.into(),
                            "Unsupported or mismatched book cache. Delete it and retry.".into(),
                        );
                    }
                    Err(e) => {
                        errors.insert(
                            info.id.into(),
                            format!("Couldn't load book: {e}. Delete it and retry."),
                        );
                    }
                }
            }
            let index = Arc::new(SearchIndex::new(&books));
            let _ = tx.send(Installed {
                books: Arc::new(books),
                index,
                sizes,
                errors,
            });
            ctx.request_repaint();
        });
    }
    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        let before = self.queue.saved();
        if let Some(rx) = &self.loading
            && let Ok(loaded) = rx.try_recv()
        {
            for info in catalog() {
                if let Some(bytes) = loaded.sizes.get(info.id) {
                    self.queue.cancel_pending(info.id);
                    self.states
                        .insert(info.id.into(), DownloadState::Downloaded { bytes: *bytes });
                } else if self.queue.saved().iter().any(|id| id == info.id) {
                    continue;
                } else if let Some(error) = loaded.errors.get(info.id) {
                    self.states
                        .insert(info.id.into(), DownloadState::Failed(error.clone()));
                } else if matches!(
                    self.states.get(info.id),
                    Some(DownloadState::Downloaded { .. })
                ) {
                    self.states.remove(info.id);
                }
            }
            self.installed = loaded.books;
            self.index = Some(loaded.index);
            self.loading = None;
        }
        if let Some((id, rx)) = &self.deleting
            && let Ok(result) = rx.try_recv()
        {
            let id = id.clone();
            self.deleting = None;
            match result {
                Ok(()) => {
                    self.states.remove(&id);
                    if self.opened.as_deref() == Some(&id) {
                        self.opened = None;
                    }
                    if self.preview.as_ref().is_some_and(|(b, _)| *b == id) {
                        self.preview = None;
                    }
                    self.reload(ctx);
                }
                Err(error) => {
                    self.states.insert(id, DownloadState::Failed(error));
                }
            }
        }
        let mut reload = false;
        if let Some(active) = &mut self.active {
            while let Ok(event) = active.events.try_recv() {
                if active.retired.is_some() {
                    continue;
                }
                match event {
                    DownloadEvent::Progress { done, total } => {
                        self.states.insert(
                            active.id.clone(),
                            DownloadState::Downloading { done, total },
                        );
                    }
                    DownloadEvent::Warning(message) => {
                        eprintln!("Book download warning: {message}");
                    }
                    DownloadEvent::Finished(summary) => {
                        self.states.insert(
                            active.id.clone(),
                            DownloadState::Downloaded {
                                bytes: summary.bytes,
                            },
                        );
                        active.retire(ctx);
                        reload = true;
                    }
                    DownloadEvent::Failed(error) => {
                        self.states
                            .insert(active.id.clone(), DownloadState::Failed(error));
                        active.retire(ctx);
                    }
                }
            }
        }
        if self
            .active
            .as_ref()
            .and_then(|a| a.retired.as_ref())
            .is_some_and(|rx| rx.try_recv().is_ok())
        {
            let active = self.active.take().expect("active download");
            if matches!(self.states.get(&active.id), Some(DownloadState::Cancelling)) {
                self.states.insert(active.id, DownloadState::NotDownloaded);
            }
            self.queue.finish();
        }
        if reload {
            self.reload(ctx);
        }
        if self.loading.is_none()
            && self.deleting.is_none()
            && let Some(id) = self.queue.next()
        {
            if self.installed.iter().any(|b| b.id == id) {
                self.queue.finish();
            } else if let Some(root) = &self.root {
                let (tx, rx) = mpsc::channel();
                let handle = Downloader::start(&id, root, tx);
                self.states
                    .insert(id.clone(), DownloadState::Downloading { done: 0, total: 1 });
                self.active = Some(ActiveDownload {
                    id,
                    handle: Some(handle),
                    events: rx,
                    retired: None,
                });
            } else {
                self.states.insert(
                    id,
                    DownloadState::Failed(
                        "Home folder unavailable. Set LUMEN_DATA_DIR and restart.".into(),
                    ),
                );
                self.queue.finish();
            }
        }
        if self.active.is_some() || self.loading.is_some() || self.deleting.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        before != self.queue.saved()
    }
    fn enqueue(&mut self, id: &str) {
        if self.active.as_ref().is_some_and(|a| a.id == id) {
            return;
        }
        self.queue.enqueue(id);
        self.states.insert(id.into(), DownloadState::Queued);
    }
    fn cancel(&mut self, id: &str, ctx: &egui::Context) {
        if let Some(active) = &mut self.active
            && active.id == id
        {
            active.retire(ctx);
            self.states.insert(id.into(), DownloadState::Cancelling);
        } else {
            self.queue.cancel_pending(id);
            self.states.insert(id.into(), DownloadState::NotDownloaded);
        }
    }
    fn remember(&mut self, target: Target) {
        self.recent.retain(|t| *t != target);
        self.recent.insert(0, target);
        self.recent.truncate(12);
    }
    fn open_unit(&mut self, id: &str, key: &str) {
        self.opened = Some(id.into());
        self.preview = Some((id.into(), key.into()));
        self.preview_slide = 0;
        self.preview_doc = None;
        self.remember(Target::Book(id.into(), key.into()));
    }
    fn selected_unit(&self) -> Option<(&Book, &Unit)> {
        let (id, key) = self.preview.as_ref()?;
        let book = self.installed.iter().find(|b| b.id == *id)?;
        Some((book, book.units.iter().find(|u| u.key == *key)?))
    }
}
impl Drop for Books {
    fn drop(&mut self) {
        if let Some(mut active) = self.active.take()
            && let Some(handle) = active.handle.take()
        {
            // Downloader's Drop joins network I/O; never do that on the UI thread.
            thread::spawn(move || drop(handle));
        }
    }
}

fn sync_downloads(app: &mut LumenApp) {
    let downloads: Vec<_> = app
        .books
        .queue
        .saved()
        .into_iter()
        .filter(|id| !matches!(app.books.states.get(id), Some(DownloadState::Cancelling)))
        .collect();
    if app.settings.book_downloads != downloads {
        app.settings.book_downloads = downloads;
        app.persist_settings();
    }
}
pub fn poll(app: &mut LumenApp, ctx: &egui::Context) {
    if app.books.poll(ctx) {
        sync_downloads(app);
    }
}
fn options(app: &LumenApp) -> SlideOpts {
    SlideOpts {
        lang: app.settings.book_language.lang(),
        blocks_per_slide: app.settings.book_blocks_per_slide.clamp(1, 4),
        ..Default::default()
    }
}
fn select_target(app: &mut LumenApp, target: Target, live: bool) {
    app.books.search_open = false;
    match target {
        Target::Cue(doc, slide) => {
            app.selected_doc = doc;
            app.selected_slide = Some(slide);
            app.books.preview = None;
            app.books.remember(Target::Cue(doc, slide));
            if live {
                app.trigger(doc, slide);
            }
        }
        Target::Book(id, key) => {
            app.books.open_unit(&id, &key);
            app.books.focus_add = true;
            app.library_tab = 2;
            if live {
                add_selected(app, true);
            }
        }
    }
}
fn add_selected(app: &mut LumenApp, live: bool) {
    let opts = options(app);
    let Some((book, unit)) = app.books.selected_unit() else {
        return;
    };
    if unit_slides(unit, opts).is_empty() {
        return;
    }
    let (id, added) = add_unit(&mut app.project, book, unit, opts);
    app.selected_doc = id;
    app.selected_slide = app
        .project
        .document(id)
        .and_then(|d| d.slides.first().map(|s| s.id));
    app.books.preview = None;
    app.books.focus_add = false;
    app.library_tab = 0;
    if added {
        app.mark();
    }
    if let Some(slide) = app.selected_slide {
        app.books.remember(Target::Cue(id, slide));
        if live {
            app.trigger(id, slide);
        }
    }
}

pub fn shortcuts(app: &mut LumenApp, ctx: &egui::Context, modal: bool) {
    if modal {
        return;
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
        app.focus_search = true;
        app.books.search_open = true;
    }
    let search_focused = ctx.memory(|m| m.has_focus(egui::Id::new("search")));
    if app.books.search_open && (search_focused || !ctx.wants_keyboard_input()) {
        let results = search_results(
            &app.project,
            &app.books.installed,
            app.books.index.as_deref(),
            &app.search,
            &app.books.recent,
        );
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
            app.books.selection = move_selection(app.books.selection, results.len(), true);
            app.books.scroll_selection = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
            app.books.selection = move_selection(app.books.selection, results.len(), false);
            app.books.scroll_selection = true;
        }
        let live = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
        if (live || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)))
            && let Some(result) =
                results.get(app.books.selection.min(results.len().saturating_sub(1)))
        {
            ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("search")));
            select_target(app, result.target.clone(), live);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            app.books.search_open = false;
            app.search.clear();
            ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("search")));
        }
    } else if app.books.preview.is_some() && !ctx.wants_keyboard_input() {
        let live = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
        if live || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            add_selected(app, live);
        }
    }
}

pub fn search_field(app: &mut LumenApp, ui: &mut egui::Ui) -> bool {
    ui.label("Search");
    let response = ui.add(
        egui::TextEdit::singleline(&mut app.search)
            .id(egui::Id::new("search"))
            .hint_text("Running order, library and books")
            .desired_width(f32::INFINITY),
    );
    if app.focus_search {
        response.request_focus();
        app.focus_search = false;
        app.books.search_open = true;
    }
    if response.changed() || response.gained_focus() {
        app.books.search_open = true;
    }
    if response.changed() || app.books.last_query != app.search {
        app.books.selection = 0;
        app.books.scroll_selection = true;
        app.books.last_query = app.search.clone();
    }
    if !app.books.search_open {
        return false;
    }
    ui.horizontal(|ui| {
        ui.label(if app.search.trim().is_empty() {
            "Recent items"
        } else {
            "Search results"
        });
        if ui.small_button("Close").clicked() {
            app.books.search_open = false;
            app.search.clear();
            response.surrender_focus();
        }
    });
    let results = search_results(
        &app.project,
        &app.books.installed,
        app.books.index.as_deref(),
        &app.search,
        &app.books.recent,
    );
    if results.is_empty() {
        ui.label(if app.search.trim().is_empty() {
            "Open a cue or book unit to find it here again."
        } else {
            "No matches. Try a title, passage label or first line."
        });
        if app.books.installed.is_empty() {
            ui.label("Download books to search their passages offline.");
            if ui.button("Download books").clicked() {
                app.books.search_open = false;
                app.search.clear();
                app.library_tab = 2;
                response.surrender_focus();
            }
        }
    }
    if app.books.loading.is_some() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Indexing books…");
        });
    }
    let mut chosen = None;
    egui::ScrollArea::vertical()
        .id_salt("unified-search")
        .show(ui, |ui| {
            let mut group = None;
            for (i, result) in results.iter().enumerate() {
                if group != Some(result.group) {
                    style::section(
                        ui,
                        match result.group {
                            Group::RunningOrder => "Running order",
                            Group::Library => "Library",
                            Group::Books => "Books",
                        },
                    );
                    group = Some(result.group);
                }
                ui.push_id(i, |ui| {
                    let r = gu_selectable(ui, app.books.selection == i, &result.label, style::BODY);
                    if r.clicked() {
                        chosen = Some(result.target.clone());
                    }
                    if app.books.scroll_selection && app.books.selection == i {
                        r.scroll_to_me(Some(egui::Align::Center));
                    }
                    gu_label(ui, &result.snippet, style::CAPTION);
                });
            }
        });
    app.books.scroll_selection = false;
    if let Some(target) = chosen {
        response.surrender_focus();
        select_target(app, target, false);
    }
    true
}

pub fn catalog_panel(app: &mut LumenApp, ui: &mut egui::Ui) {
    if !app.settings.books_note_seen {
        ui.label("Books are downloaded from anirdesh.com to this Mac for personal use.");
        if ui.button("Got it").clicked() {
            app.settings.books_note_seen = true;
            app.persist_settings();
        }
        ui.separator();
    }
    if app.books.root.is_none() {
        ui.colored_label(
            style::WARN,
            "Home folder unavailable. Set LUMEN_DATA_DIR and restart to download books.",
        );
        return;
    }
    if app.books.loading.is_some() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Loading books…");
        });
    }
    if let Some(id) = app.books.opened.clone() {
        if ui.button("All books").clicked() {
            app.books.opened = None;
            app.books.unit_filter.clear();
        }
        let installed = app.books.installed.clone();
        if let Some(book) = installed.iter().find(|b| b.id == id) {
            gu_label(ui, &book.title_gu, style::HEADING);
            ui.label(&book.title_en);
            ui.label("Filter units");
            ui.add(
                egui::TextEdit::singleline(&mut app.books.unit_filter)
                    .desired_width(f32::INFINITY)
                    .hint_text("Label or title"),
            );
            let q = lumen_library::to_nfc(app.books.unit_filter.trim()).to_lowercase();
            let mut chosen = None;
            egui::ScrollArea::vertical()
                .id_salt("book-units")
                .show(ui, |ui| {
                    let mut shown = 0;
                    for unit in &book.units {
                        let label = format!(
                            "{} {} {}",
                            unit.label,
                            unit.title_gu.as_deref().unwrap_or_default(),
                            unit.title_en.as_deref().unwrap_or_default()
                        );
                        if !lumen_library::to_nfc(&label).to_lowercase().contains(&q) {
                            continue;
                        }
                        shown += 1;
                        ui.push_id(&unit.key, |ui| {
                            if gu_selectable(
                                ui,
                                app.books
                                    .preview
                                    .as_ref()
                                    .is_some_and(|(b, k)| *b == id && *k == unit.key),
                                &label,
                                style::BODY,
                            )
                            .clicked()
                            {
                                chosen = Some(unit.key.clone());
                            }
                        });
                    }
                    if shown == 0 {
                        ui.label("No matching units. Clear the filter to see all passages.");
                    }
                });
            if let Some(key) = chosen {
                app.books.open_unit(&id, &key);
            }
            return;
        }
    }
    if app.books.installed.is_empty() {
        ui.label("No books downloaded yet. Download a book to read and search offline.");
    }
    egui::ScrollArea::vertical()
        .id_salt("books-catalog")
        .show(ui, |ui| {
            for info in catalog() {
                ui.push_id(info.id, |ui| {
                    gu_label(ui, info.title_gu, style::HEADING);
                    ui.label(info.title_en);
                    let minutes = (info.pages as f32 * 1.1 / 60.0).ceil().max(1.0) as usize;
                    ui.label(
                        egui::RichText::new(format!(
                            "{} units · ≈ {:.1} MB · ≈ {minutes} min",
                            info.units,
                            info.units as f32 * 4.0 / 1024.0
                        ))
                        .color(style::TEXT_DIM)
                        .size(style::CAPTION),
                    );
                    let state = app
                        .books
                        .states
                        .get(info.id)
                        .cloned()
                        .unwrap_or(DownloadState::NotDownloaded);
                    let busy = app
                        .books
                        .deleting
                        .as_ref()
                        .is_some_and(|(id, _)| id == info.id);
                    if busy {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Deleting book…");
                        });
                    } else {
                        match state {
                            DownloadState::NotDownloaded => {
                                ui.label("Not downloaded");
                                if ui.button(format!("Download {}", info.title_en)).clicked() {
                                    app.books.enqueue(info.id);
                                    sync_downloads(app);
                                }
                            }
                            DownloadState::Queued => {
                                ui.horizontal(|ui| {
                                    ui.label("Queued");
                                    if ui.button("Cancel").clicked() {
                                        app.books.cancel(info.id, ui.ctx());
                                        sync_downloads(app);
                                    }
                                });
                            }
                            DownloadState::Downloading { done, total } => {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(format!(
                                        "Downloading {} %",
                                        done.saturating_mul(100)
                                            .checked_div(total)
                                            .unwrap_or(0)
                                            .min(100)
                                    ));
                                    if ui.button("Cancel").clicked() {
                                        app.books.cancel(info.id, ui.ctx());
                                        sync_downloads(app);
                                    }
                                });
                            }
                            DownloadState::Cancelling => {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Cancelling…");
                                });
                            }
                            DownloadState::Downloaded { bytes } => {
                                ui.label(format!(
                                    "Downloaded · {:.1} MB",
                                    bytes as f64 / 1_048_576.0
                                ));
                                ui.horizontal(|ui| {
                                    if ui.button("Open book").clicked() {
                                        app.books.opened = Some(info.id.into());
                                        app.books.unit_filter.clear();
                                    }
                                    if ui.button("Delete").clicked() {
                                        app.books.confirm_delete = Some(info.id.into());
                                    }
                                });
                            }
                            DownloadState::Failed(message) => {
                                ui.colored_label(style::WARN, format!("Failed: {message}"));
                                ui.horizontal(|ui| {
                                    if ui
                                        .add_enabled(
                                            app.books
                                                .active
                                                .as_ref()
                                                .is_none_or(|a| a.id != info.id),
                                            egui::Button::new("Retry"),
                                        )
                                        .clicked()
                                    {
                                        app.books.enqueue(info.id);
                                        sync_downloads(app);
                                    }
                                    if ui.button("Delete cache").clicked() {
                                        app.books.confirm_delete = Some(info.id.into());
                                    }
                                });
                            }
                        }
                    }
                    if app.books.confirm_delete.as_deref() == Some(info.id) {
                        ui.label(
                            "Delete this book from this Mac? Added slides stay in your project.",
                        );
                        ui.horizontal(|ui| {
                            if ui.button("Keep book").clicked() {
                                app.books.confirm_delete = None;
                            }
                            if ui
                                .add_enabled(
                                    app.books.deleting.is_none(),
                                    egui::Button::new("Delete book"),
                                )
                                .clicked()
                                && let Some(root) = app.books.root.clone()
                            {
                                let id = info.id.to_string();
                                let (tx, rx) = mpsc::channel();
                                let ctx = ui.ctx().clone();
                                app.books.deleting = Some((id.clone(), rx));
                                app.books.confirm_delete = None;
                                thread::spawn(move || {
                                    let _ =
                                        tx.send(lumen_library::delete_book(&root, &id).map_err(
                                            |e| format!("Couldn't delete book: {e}. Retry."),
                                        ));
                                    ctx.request_repaint();
                                });
                            }
                        });
                    }
                    ui.add_space(style::GAP);
                    ui.separator();
                });
            }
        });
}

pub fn preview_panel(app: &mut LumenApp, ui: &mut egui::Ui) -> bool {
    if app.books.selected_unit().is_none() {
        return false;
    }
    ui.heading("Book preview");
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Language");
        egui::ComboBox::from_id_salt("book-language")
            .selected_text(app.settings.book_language.label())
            .show_ui(ui, |ui| {
                for lang in crate::settings::BookLanguage::ALL {
                    changed |= ui
                        .selectable_value(&mut app.settings.book_language, lang, lang.label())
                        .changed();
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Blocks per slide");
        changed |= ui
            .add(egui::DragValue::new(&mut app.settings.book_blocks_per_slide).range(1..=4))
            .changed();
    });
    if changed {
        app.books.preview_slide = 0;
        app.books.preview_doc = None;
        app.persist_settings();
    }
    if app
        .books
        .preview_doc
        .as_ref()
        .is_none_or(|(lang, count, _, _)| {
            *lang != app.settings.book_language || *count != app.settings.book_blocks_per_slide
        })
    {
        let theme = scripture_theme();
        let (book, unit) = app.books.selected_unit().expect("selected unit");
        let doc = unit_document(book, unit, options(app), theme.id);
        app.books.preview_doc = Some((
            app.settings.book_language,
            app.settings.book_blocks_per_slide,
            doc,
            theme,
        ));
    }
    let (book, unit) = app.books.selected_unit().expect("selected unit");
    gu_label(
        ui,
        &format!(
            "{} · {} {}",
            book.title_en,
            unit.label,
            unit.title_gu.as_deref().unwrap_or_default()
        ),
        style::BODY,
    );
    let (_, _, doc, theme) = app.books.preview_doc.as_ref().expect("preview document");
    let add = ui.add_enabled(
        !doc.slides.is_empty(),
        egui::Button::new("Add to running order"),
    );
    if app.books.focus_add {
        add.request_focus();
        app.books.focus_add = false;
    }
    let live = !ui.ctx().wants_keyboard_input()
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
    if add.clicked() || live {
        add_selected(app, live);
        return true;
    }
    ui.small("Return adds · Cmd+Return adds and goes live");
    if ui.button("Back to cue preview").clicked() {
        app.books.preview = None;
        return true;
    }
    if doc.slides.is_empty() {
        ui.label("This unit has no displayable text. Try another language or unit.");
        return true;
    }
    app.books.preview_slide = app
        .books
        .preview_slide
        .min(doc.slides.len().saturating_sub(1));
    let cue = LiveSlide {
        document_id: doc.id,
        document_name: doc.name.clone(),
        slide: doc.slides[app.books.preview_slide].clone(),
        theme: theme.clone(),
    };
    let height = crate::panels::preview_height(ui.available_width(), ui.available_height());
    crate::render::preview_frame_with_images(
        ui,
        Some(&cue),
        false,
        false,
        None,
        &HashMap::new(),
        height,
    );
    egui::ScrollArea::vertical()
        .id_salt("book-slides")
        .show(ui, |ui| {
            for (i, slide) in doc.slides.iter().enumerate() {
                if gu_selectable(
                    ui,
                    app.books.preview_slide == i,
                    &format!("{} · {}", i + 1, snippet(&slide.text)),
                    style::BODY,
                )
                .clicked()
                {
                    app.books.preview_slide = i;
                }
            }
        });
    true
}

pub fn has_preview(app: &LumenApp) -> bool {
    app.books.selected_unit().is_some()
}
pub fn add_and_go_live(app: &mut LumenApp) {
    add_selected(app, true);
}
pub fn remember_cue(app: &mut LumenApp, doc: Uuid, slide: Uuid) {
    app.books.preview = None;
    app.books.remember(Target::Cue(doc, slide));
}
pub fn close_preview(app: &mut LumenApp) {
    app.books.preview = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumen_library::{Block, BlockKind};
    fn project() -> Project {
        Project {
            schema_version: lumen_core::SCHEMA_VERSION,
            name: "Test".into(),
            documents: vec![],
            playlist: vec![],
            themes: vec![Default::default()],
            macros: vec![],
            timers: vec![],
            props: vec![],
            messages: vec![],
        }
    }
    fn fixture() -> Book {
        Book {
            id: "synthetic".into(),
            title_gu: "નમૂનો".into(),
            title_en: "Synthetic".into(),
            schema: lumen_library::SCHEMA_VERSION,
            units: vec![Unit {
                key: "one".into(),
                label: "Synthetic 1".into(),
                title_gu: Some("નમૂનો".into()),
                title_en: None,
                blocks: vec![Block {
                    gu: Some("કસોટી".into()),
                    en: Some("Synthetic test content".into()),
                    translit: None,
                    notes: Some("Synthetic footnote".into()),
                    kind: BlockKind::Paragraph,
                }],
            }],
        }
    }
    #[test]
    fn saved_download_queue_restarts_active_before_pending() {
        let mut queue = DownloadQueue::default();
        queue.enqueue("vato");
        queue.enqueue("diksha");
        queue.next();
        let mut resumed = DownloadQueue::default();
        for id in queue.saved() {
            resumed.enqueue(&id);
        }
        assert_eq!(resumed.next().as_deref(), Some("vato"));
        resumed.finish();
        assert_eq!(resumed.next().as_deref(), Some("diksha"));
    }
    #[test]
    fn notes_follow_split_blocks_and_combined_slides_exactly() {
        let mut book = fixture();
        let mut first = book.units[0].blocks[0].clone();
        first.en = Some("First synthetic sentence. Another synthetic sentence.".into());
        let mut second = first.clone();
        second.en = Some("Second [b] synthetic block.".into());
        second.notes = Some("Second note".into());
        book.units[0].blocks = vec![first, second];
        for count in 1..=4 {
            for max_chars in [15, 35, 80, 400] {
                let opts = SlideOpts {
                    lang: lumen_library::Lang::En,
                    blocks_per_slide: count,
                    max_chars,
                };
                let expected = unit_slides(&book.units[0], opts);
                let doc = unit_document(&book, &book.units[0], opts, Uuid::new_v4());
                assert_eq!(
                    doc.slides
                        .iter()
                        .map(|s| (s.label.clone(), s.text.clone()))
                        .collect::<Vec<_>>(),
                    expected
                );
                assert!(doc.slides.iter().all(|s| !s.notes.is_empty()));
                assert!(doc.slides.last().unwrap().notes.contains("Second note"));
            }
        }
    }
    #[test]
    fn worker_indexes_only_completed_books_and_reports_corrupt_caches() {
        let root = std::env::temp_dir().join(format!("lumen-books-{}", Uuid::new_v4()));
        let completed = root.join("vachanamrut");
        let partial = root.join("diksha/units");
        std::fs::create_dir_all(&completed).unwrap();
        std::fs::create_dir_all(&partial).unwrap();
        let mut book = fixture();
        book.id = "vachanamrut".into();
        std::fs::write(
            completed.join("book.json"),
            serde_json::to_vec(&book).unwrap(),
        )
        .unwrap();
        std::fs::write(
            partial.join("one.json"),
            serde_json::to_vec(&book.units[0]).unwrap(),
        )
        .unwrap();
        std::fs::create_dir_all(root.join("vato")).unwrap();
        std::fs::write(root.join("vato/book.json"), b"malformed synthetic cache").unwrap();
        let ctx = egui::Context::default();
        let mut books = Books::at_root(&ctx, Some(root.clone()), &[]);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while books.loading.is_some() && std::time::Instant::now() < deadline {
            books.poll(&ctx);
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(books.loading.is_none());
        assert_eq!(books.installed.len(), 1);
        assert!(
            matches!(books.states.get("vachanamrut"), Some(DownloadState::Downloaded { bytes }) if *bytes > 0)
        );
        assert!(matches!(
            books.states.get("vato"),
            Some(DownloadState::Failed(_))
        ));
        assert!(!books.states.contains_key("diksha"));
        assert_eq!(
            books.index.as_ref().unwrap().search("Synthetic", 5)[0].book,
            "vachanamrut"
        );
        drop(books);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn download_queue_serializes_deduplicates_and_cancels() {
        let mut q = DownloadQueue::default();
        q.enqueue("a");
        q.enqueue("a");
        q.enqueue("b");
        q.enqueue("c");
        assert_eq!(q.next().as_deref(), Some("a"));
        assert_eq!(q.next(), None);
        q.enqueue("a");
        q.cancel_pending("b");
        q.finish();
        assert_eq!(q.next().as_deref(), Some("c"));
        q.finish();
        assert_eq!(q.next(), None);
    }
    #[test]
    fn unit_conversion_keeps_notes_off_audience_and_uses_options() {
        let book = fixture();
        let theme = Uuid::new_v4();
        let doc = unit_document(
            &book,
            &book.units[0],
            SlideOpts {
                lang: lumen_library::Lang::En,
                ..Default::default()
            },
            theme,
        );
        assert!(doc.name.contains("Synthetic 1"));
        assert!(doc.name.contains("નમૂનો"));
        assert_eq!(doc.slides.len(), 1);
        assert_eq!(doc.slides[0].theme_id, theme);
        assert_eq!(doc.slides[0].text, "Synthetic test content");
        assert_eq!(doc.slides[0].notes, "Synthetic footnote");
    }
    #[test]
    fn adding_unit_deduplicates_after_project_round_trip() {
        let mut project = project();
        let book = fixture();
        let (first, added) = add_unit(&mut project, &book, &book.units[0], SlideOpts::default());
        assert!(added);
        let mut project: Project =
            serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
        let count = project.playlist.len();
        let (second, added) = add_unit(&mut project, &book, &book.units[0], SlideOpts::default());
        assert_eq!(first, second);
        assert!(!added);
        assert_eq!(project.playlist.len(), count);
        assert_eq!(
            project
                .themes
                .iter()
                .filter(|t| t.name == "Scripture")
                .count(),
            1
        );
        project.validate().unwrap();
    }
    #[test]
    fn search_groups_cues_and_books_and_recent_items() {
        let mut project = project();
        let book = fixture();
        let (id, _) = add_unit(&mut project, &book, &book.units[0], SlideOpts::default());
        let index = lumen_library::SearchIndex::new(std::slice::from_ref(&book));
        let results = search_results(
            &project,
            std::slice::from_ref(&book),
            Some(&index),
            "Synthetic",
            &[],
        );
        assert_eq!(
            results.iter().map(|r| r.group).collect::<Vec<_>>(),
            vec![Group::RunningOrder, Group::Library, Group::Books]
        );
        assert!(
            results
                .iter()
                .all(|r| !r.label.is_empty() && !r.snippet.is_empty())
        );
        let target = Target::Cue(id, project.document(id).unwrap().slides[0].id);
        let recent = search_results(
            &project,
            &[book],
            Some(&index),
            "",
            std::slice::from_ref(&target),
        );
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].target, target);
    }
    #[test]
    fn keyboard_selection_is_bounded_even_when_results_disappear() {
        assert_eq!(move_selection(0, 3, true), 1);
        assert_eq!(move_selection(2, 3, true), 2);
        assert_eq!(move_selection(0, 3, false), 0);
        assert_eq!(move_selection(7, 2, false), 0);
        assert_eq!(move_selection(4, 0, true), 0);
    }
}
