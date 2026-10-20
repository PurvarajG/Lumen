//! Asynchronous control of Keynote and PowerPoint slide shows.
//!
//! UI code communicates with one worker through [`DeckController`]. App lookup,
//! AppleScript execution, and health checks all run on that worker.

use std::{
    collections::hash_map::DefaultHasher,
    ffi::{OsStr, OsString},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
const CONVERT_TIMEOUT: Duration = Duration::from_secs(180);
const HEALTH_INTERVAL: Duration = Duration::from_secs(1);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
const NO_APP_ERROR: &str = "Install PowerPoint or Keynote to play this deck, or export it as PDF";

// Files are opened through Launch Services (`open -b`) first: a sandboxed app told to open a file by
// AppleScript loads it, but its slideshow then ends immediately. Starting also retries until a show stays up.
const KEYNOTE_OPEN: &str = "tell application id \"com.apple.iWork.Keynote\"
 activate
 repeat 8 times
  try
   start front document from slide 1
   delay 0.5
   if playing then return
  end try
  delay 0.5
 end repeat
 error \"Keynote did not start the slideshow\"
end tell";
const POWERPOINT_OPEN: &str = "tell application id \"com.microsoft.Powerpoint\"
 activate
 set show type of (slide show settings of active presentation) to slide show type presenter
 repeat 8 times
  try
   -- Changing starting slide/range makes PowerPoint 16 end the show at once; the default plays all from slide 1.
   run slide show slide show settings of active presentation
   delay 0.5
   if (count of slide show windows) > 0 then return
  end try
  delay 0.5
 end repeat
 error \"PowerPoint did not start the slideshow\"
end tell";
const POWERPOINT_EXPORT: &str = r#"on run argv
set sourcePath to item 1 of argv
set pdfPath to item 2 of argv
set notesPath to item 3 of argv
set sourceFile to POSIX file sourcePath
tell application id "com.microsoft.Powerpoint"
 set wasOpen to false
 set targetPresentation to missing value
 -- Index the collection: "repeat with p in presentations" yields nothing in PowerPoint 16.
 repeat with i from 1 to count of presentations
  try
   set p to presentation i
   -- PowerPoint reports /tmp/… for paths Lumen resolves to /private/tmp/….
   if full name of p is sourcePath or ("/private" & full name of p) is sourcePath then
    set targetPresentation to p
    set wasOpen to true
    exit repeat
   end if
  end try
 end repeat
 if targetPresentation is missing value then
  open sourceFile
  set targetPresentation to active presentation
 end if
 set slideCount to count of slides of targetPresentation
 save targetPresentation in (POSIX file pdfPath) as save as PDF
 set noteRows to {}
 repeat with i from 1 to slideCount
  set noteText to ""
  try
   set noteText to content of text range of text frame of place holder 2 of notes page of slide i of targetPresentation
   if noteText is missing value then set noteText to ""
  end try
  set noteText to my replaceText(noteText, return, ASCII character 29)
  set noteText to my replaceText(noteText, linefeed, ASCII character 29)
  set end of noteRows to noteText
 end repeat
 set AppleScript's text item delimiters to ASCII character 30
 set noteData to noteRows as text
 set AppleScript's text item delimiters to ""
 if not wasOpen then close targetPresentation saving no
end tell
-- File I/O stays outside the tell block: the app's own "close" command would capture "close access".
set noteFile to open for access (POSIX file notesPath) with write permission
set eof of noteFile to 0
write noteData to noteFile as «class utf8»
close access noteFile
return slideCount
end run
on replaceText(theText, findText, replaceWith)
 set AppleScript's text item delimiters to findText
 set textItems to every text item of theText
 set AppleScript's text item delimiters to replaceWith
 set theText to textItems as text
 set AppleScript's text item delimiters to ""
 return theText
end replaceText"#;
const KEYNOTE_EXPORT: &str = r#"on run argv
set sourcePath to item 1 of argv
set pdfPath to item 2 of argv
set notesPath to item 3 of argv
tell application id "com.apple.iWork.Keynote"
 set wasOpen to false
 set targetDocument to missing value
 repeat with i from 1 to count of documents
  try
   set p to document i
   if POSIX path of (file of p as alias) is sourcePath then
    set targetDocument to p
    set wasOpen to true
    exit repeat
   end if
  end try
 end repeat
 if targetDocument is missing value then
  open POSIX file sourcePath
  set targetDocument to front document
 end if
 set slideCount to count of slides of targetDocument
 export targetDocument to (POSIX file pdfPath) as PDF
 set noteRows to {}
 repeat with i from 1 to slideCount
  set noteText to ""
  try
   set noteText to presenter notes of slide i of targetDocument
   if noteText is missing value then set noteText to ""
  end try
  set noteText to my replaceText(noteText, return, ASCII character 29)
  set noteText to my replaceText(noteText, linefeed, ASCII character 29)
  set end of noteRows to noteText
 end repeat
 set AppleScript's text item delimiters to ASCII character 30
 set noteData to noteRows as text
 set AppleScript's text item delimiters to ""
 if not wasOpen then close targetDocument saving no
end tell
-- File I/O stays outside the tell block: the app's own "close" command would capture "close access".
set noteFile to open for access (POSIX file notesPath) with write permission
set eof of noteFile to 0
write noteData to noteFile as «class utf8»
close access noteFile
return slideCount
end run
on replaceText(theText, findText, replaceWith)
 set AppleScript's text item delimiters to findText
 set textItems to every text item of theText
 set AppleScript's text item delimiters to replaceWith
 set theText to textItems as text
 set AppleScript's text item delimiters to ""
 return theText
end replaceText"#;
const KEYNOTE_NEXT: &str = "tell application id \"com.apple.iWork.Keynote\" to show next";
const KEYNOTE_PREVIOUS: &str = "tell application id \"com.apple.iWork.Keynote\" to show previous";
pub(crate) const POWERPOINT_NEXT: &str = "tell application id \"com.microsoft.Powerpoint\" to go to next slide slideshow view of slide show window 1";
pub(crate) const POWERPOINT_PREVIOUS: &str = "tell application id \"com.microsoft.Powerpoint\" to go to previous slide slideshow view of slide show window 1";
const KEYNOTE_STOP: &str = "tell application id \"com.apple.iWork.Keynote\" to stop front document";
pub(crate) const POWERPOINT_STOP: &str = "tell application id \"com.microsoft.Powerpoint\" to exit slide show slideshow view of slide show window 1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeckApp {
    Keynote,
    PowerPoint,
}

impl DeckApp {
    fn bundle_id(self) -> &'static str {
        match self {
            Self::Keynote => "com.apple.iWork.Keynote",
            Self::PowerPoint => "com.microsoft.Powerpoint",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Keynote => "Keynote",
            Self::PowerPoint => "PowerPoint",
        }
    }

    fn open_script(self) -> &'static str {
        match self {
            Self::Keynote => KEYNOTE_OPEN,
            Self::PowerPoint => POWERPOINT_OPEN,
        }
    }

    fn navigation_script(self, forward: bool) -> &'static str {
        match (self, forward) {
            (Self::Keynote, true) => KEYNOTE_NEXT,
            (Self::Keynote, false) => KEYNOTE_PREVIOUS,
            (Self::PowerPoint, true) => POWERPOINT_NEXT,
            (Self::PowerPoint, false) => POWERPOINT_PREVIOUS,
        }
    }

    fn stop_script(self) -> &'static str {
        match self {
            Self::Keynote => KEYNOTE_STOP,
            Self::PowerPoint => POWERPOINT_STOP,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeckKind {
    Keynote,
    PowerPoint,
    Unknown,
}

fn deck_kind(path: &Path) -> DeckKind {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("key") => DeckKind::Keynote,
        Some("ppt" | "pptx") => DeckKind::PowerPoint,
        _ => DeckKind::Unknown,
    }
}

fn select_app(kind: DeckKind, keynote: bool, powerpoint: bool) -> Option<DeckApp> {
    match kind {
        DeckKind::Keynote => keynote.then_some(DeckApp::Keynote),
        DeckKind::PowerPoint => powerpoint.then_some(DeckApp::PowerPoint),
        DeckKind::Unknown if keynote => Some(DeckApp::Keynote),
        DeckKind::Unknown if powerpoint => Some(DeckApp::PowerPoint),
        DeckKind::Unknown => None,
    }
}

fn conversion_app(
    kind: DeckKind,
    keynote: bool,
    powerpoint: bool,
) -> Result<DeckApp, &'static str> {
    match kind {
        DeckKind::Keynote if keynote => Ok(DeckApp::Keynote),
        DeckKind::Keynote => Err(
            "Keynote isn't installed. Get it free from the App Store, or export the deck as PDF.",
        ),
        DeckKind::PowerPoint if powerpoint => Ok(DeckApp::PowerPoint),
        DeckKind::PowerPoint if keynote => Ok(DeckApp::Keynote),
        DeckKind::PowerPoint => Err("Install PowerPoint or Keynote, or export the deck as PDF"),
        DeckKind::Unknown => Err("Unsupported presentation file"),
    }
}

fn export_script(app: DeckApp) -> &'static str {
    match app {
        DeckApp::PowerPoint => POWERPOINT_EXPORT,
        DeckApp::Keynote => KEYNOTE_EXPORT,
    }
}

fn cached_pdf_path(
    source: &Path,
    len: u64,
    modified: Option<std::time::SystemTime>,
    cache: &Path,
) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    len.hash(&mut hasher);
    modified.hash(&mut hasher);
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    cache.join(format!("{stem}-{:016x}.pdf", hasher.finish()))
}

fn parse_notes(data: &str, count: usize) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }
    let notes: Vec<_> = data
        .split('\u{1e}')
        .map(|line| {
            if line == "missing value" {
                String::new()
            } else {
                line.replace('\u{1d}', "\n")
            }
        })
        .collect();
    if notes.len() == count {
        notes
    } else {
        vec![String::new(); count]
    }
}

pub fn convert_to_pdf(
    source: &Path,
    mut progress: impl FnMut(&str),
) -> Result<(PathBuf, Vec<String>), String> {
    let kind = deck_kind(source);
    let keynote = application_installed(DeckApp::Keynote.bundle_id());
    let powerpoint = application_installed(DeckApp::PowerPoint.bundle_id());
    let app = conversion_app(kind, keynote, powerpoint).map_err(str::to_owned)?;
    let source = source
        .canonicalize()
        .map_err(|e| format!("Couldn't open {}: {e}", source.display()))?;
    let metadata = std::fs::metadata(&source).map_err(|e| e.to_string())?;
    let cache = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Library/Application Support/Lumen Presenter/decks");
    std::fs::create_dir_all(&cache).map_err(|e| format!("Couldn't create deck cache: {e}"))?;
    let cached = cached_pdf_path(&source, metadata.len(), metadata.modified().ok(), &cache);
    let notes_cache = cached.with_extension("notes");
    if cached.is_file() {
        let page_count = lumen_media::pdf_page_count(&cached).map_err(|e| e.to_string())? as usize;
        let notes = std::fs::read_to_string(notes_cache)
            .map(|data| parse_notes(&data, page_count))
            .unwrap_or_else(|_| vec![String::new(); page_count]);
        return Ok((cached, notes));
    }
    progress(&format!(
        "Converting {} in {}…",
        source.file_name().unwrap_or_default().to_string_lossy(),
        app.display_name()
    ));
    let container = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(match app {
            DeckApp::PowerPoint => "Library/Containers/com.microsoft.Powerpoint/Data/tmp",
            DeckApp::Keynote => "Library/Containers/com.apple.iWork.Keynote/Data/tmp",
        });
    let id = uuid::Uuid::new_v4();
    let temp_pdf = container.join(format!("lumen-{id}.pdf"));
    let temp_notes = container.join(format!("lumen-{id}.notes"));
    let _ = std::fs::remove_file(&temp_pdf);
    let _ = std::fs::remove_file(&temp_notes);
    // The script opens the deck itself so it can tell whether the user already had it open.
    let argv = [
        source.as_os_str(),
        temp_pdf.as_os_str(),
        temp_notes.as_os_str(),
    ];
    let output = match run_script_timeout(export_script(app), &argv, CONVERT_TIMEOUT) {
        Ok(output) => output,
        Err(error) => {
            let _ = std::fs::remove_file(&temp_pdf);
            let _ = std::fs::remove_file(&temp_notes);
            return Err(error);
        }
    };
    let slide_count: usize = match output.trim().parse() {
        Ok(count) => count,
        Err(_) => {
            let _ = std::fs::remove_file(&temp_pdf);
            let _ = std::fs::remove_file(&temp_notes);
            return Err(format!(
                "{} did not report a slide count",
                app.display_name()
            ));
        }
    };
    let notes_data = std::fs::read_to_string(&temp_notes).unwrap_or_default();
    let notes = parse_notes(&notes_data, slide_count);
    let page_count = match lumen_media::pdf_page_count(&temp_pdf) {
        Ok(count) => count as usize,
        Err(error) => {
            let _ = std::fs::remove_file(&temp_pdf);
            let _ = std::fs::remove_file(&temp_notes);
            return Err(error.to_string());
        }
    };
    if page_count != slide_count {
        let _ = std::fs::remove_file(&temp_pdf);
        let _ = std::fs::remove_file(&temp_notes);
        return Err(format!(
            "Exported PDF has {page_count} pages, but the deck has {slide_count} slides"
        ));
    }
    std::fs::write(&notes_cache, notes_data)
        .map_err(|e| format!("Couldn't cache deck notes: {e}"))?;
    std::fs::rename(&temp_pdf, &cached)
        .map_err(|e| format!("Couldn't cache converted deck: {e}"))?;
    let _ = std::fs::remove_file(temp_notes);
    Ok((cached, notes))
}

pub fn open_keynote_store() {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("/usr/bin/open")
            .arg("macappstore://apps.apple.com/app/id409183694")
            .spawn();
    }
}

fn osascript_args(script: &str, argv: &[&OsStr]) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("-e"),
        OsString::from(script),
        OsString::from("--"),
    ];
    args.extend(argv.iter().map(|arg| arg.to_os_string()));
    args
}

/// Owns a single deck-control worker. All methods are non-blocking from the UI.
pub struct DeckController {
    commands: Sender<WorkerCommand>,
    events: Receiver<Result<(), String>>,
    desired_path: Option<PathBuf>,
    app_name: String,
}

impl Default for DeckController {
    fn default() -> Self {
        Self::new()
    }
}

impl DeckController {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        thread::Builder::new()
            .name("lumen-deck-control".into())
            .spawn(move || {
                worker_loop(command_rx, event_tx);
            })
            .expect("failed to start deck-control worker");
        Self {
            commands: command_tx,
            events: event_rx,
            desired_path: None,
            app_name: "PowerPoint or Keynote".into(),
        }
    }

    /// Start a different deck, or stop the active deck when passed `None`.
    /// Repeating the current path does nothing.
    pub fn reconcile(&mut self, path: Option<PathBuf>) {
        if path == self.desired_path {
            return;
        }
        self.desired_path = path.clone();
        if let Some(path) = path.as_ref() {
            self.app_name = match deck_kind(path) {
                DeckKind::Keynote => "Keynote",
                DeckKind::PowerPoint => "PowerPoint",
                DeckKind::Unknown => "Keynote",
            }
            .into();
        }
        let _ = self.commands.send(WorkerCommand::Reconcile(path));
    }

    /// Route one slide advance to the active deck.
    pub fn navigate(&self, forward: bool) {
        let _ = self.commands.send(WorkerCommand::Navigate(forward));
    }

    /// Read the next worker result, if one is ready.
    pub fn poll(&mut self) -> Option<Result<(), String>> {
        match self.events.try_recv() {
            Ok(result) => {
                if result.is_err() {
                    self.desired_path = None;
                }
                Some(result)
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    /// Name shown by the live monitor while the deck is active.
    pub fn app_name(&self) -> &str {
        &self.app_name
    }
}

enum WorkerCommand {
    Reconcile(Option<PathBuf>),
    Navigate(bool),
}

struct ActiveDeck {
    app: DeckApp,
    next_health_check: Instant,
}

fn worker_loop(commands: Receiver<WorkerCommand>, events: Sender<Result<(), String>>) {
    let mut active: Option<ActiveDeck> = None;
    loop {
        match commands.recv_timeout(POLL_INTERVAL) {
            Ok(WorkerCommand::Reconcile(path)) => {
                if let Some(deck) = active.take()
                    && let Err(error) = stop_deck(deck.app)
                {
                    let _ = events.send(Err(error));
                }
                if let Some(path) = path {
                    match start_deck(&path) {
                        Ok(app) => {
                            active = Some(ActiveDeck {
                                app,
                                next_health_check: Instant::now() + HEALTH_INTERVAL,
                            })
                        }
                        Err(error) => {
                            let _ = events.send(Err(error));
                        }
                    }
                }
            }
            Ok(WorkerCommand::Navigate(forward)) => {
                if let Some(deck) = &active
                    && let Err(error) = run_script(deck.app.navigation_script(forward), &[])
                {
                    let stop_error = stop_deck(deck.app).err();
                    active = None;
                    let message = stop_error.map_or(error.clone(), |stop| {
                        format!("{error}; additionally, stopping the slideshow failed: {stop}")
                    });
                    let _ = events.send(Err(message));
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if let Some(deck) = &mut active
            && Instant::now() >= deck.next_health_check
        {
            let app = deck.app;
            deck.next_health_check = Instant::now() + HEALTH_INTERVAL;
            if !application_running(app.bundle_id()) {
                active = None;
                let _ = events.send(Err(format!(
                    "{} quit; deck playback stopped",
                    app.display_name()
                )));
            }
        }
    }
    if let Some(deck) = active {
        let _ = stop_deck(deck.app);
    }
}

fn start_deck(path: &Path) -> Result<DeckApp, String> {
    if path.as_os_str().is_empty() {
        return Err("Deck path cannot be empty".into());
    }
    let kind = deck_kind(path);
    let keynote = application_installed(DeckApp::Keynote.bundle_id());
    let powerpoint = application_installed(DeckApp::PowerPoint.bundle_id());
    let app = select_app(kind, keynote, powerpoint).ok_or_else(|| NO_APP_ERROR.to_owned())?;
    // An absolute path can never be read by `open` as an option.
    let path = path
        .canonicalize()
        .map_err(|error| format!("Cannot open deck {}: {error}", path.display()))?;
    open_with(app.bundle_id(), &path)?;
    run_script(app.open_script(), &[])?;
    Ok(app)
}

#[cfg(target_os = "macos")]
fn application_installed(bundle_id: &str) -> bool {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;
    let bundle_id = NSString::from_str(bundle_id);
    // NSWorkspace is intentionally confined to the worker thread.
    unsafe {
        NSWorkspace::sharedWorkspace()
            .URLForApplicationWithBundleIdentifier(&bundle_id)
            .is_some()
    }
}

#[cfg(target_os = "macos")]
fn application_running(bundle_id: &str) -> bool {
    use objc2_app_kit::NSRunningApplication;
    use objc2_foundation::NSString;
    let bundle_id = NSString::from_str(bundle_id);
    // Querying NSRunningApplication cannot launch the target app.
    unsafe { NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id).count() > 0 }
}

#[cfg(not(target_os = "macos"))]
fn application_installed(_bundle_id: &str) -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
fn application_running(_bundle_id: &str) -> bool {
    false
}

fn stop_deck(app: DeckApp) -> Result<(), String> {
    if application_running(app.bundle_id()) {
        run_script(app.stop_script(), &[]).map(|_| ())
    } else {
        Ok(())
    }
}

fn open_with(bundle_id: &str, path: &Path) -> Result<(), String> {
    let mut child = Command::new("/usr/bin/open")
        .arg("-b")
        .arg(bundle_id)
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not open deck: {error}"))?;
    match wait_with_timeout(&mut child, PROCESS_TIMEOUT) {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(format!("Could not open deck {}", path.display())),
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(error)
        }
    }
}

fn run_script(script: &str, argv: &[&OsStr]) -> Result<String, String> {
    run_script_timeout(script, argv, PROCESS_TIMEOUT)
}

fn run_script_timeout(script: &str, argv: &[&OsStr], timeout: Duration) -> Result<String, String> {
    let args = osascript_args(script, argv);
    let mut child = Command::new("osascript")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not run AppleScript: {error}"))?;
    match wait_with_timeout(&mut child, timeout) {
        Ok(status) if status.success() => {
            let output = child
                .wait_with_output()
                .map_err(|error| format!("Could not read AppleScript result: {error}"))?;
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        }
        Ok(_) => {
            let output = child.wait_with_output().map_err(|error| {
                format!("AppleScript failed and its output could not be read: {error}")
            })?;
            Err(format!(
                "AppleScript failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(error)
        }
    }
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<ExitStatus, String> {
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("Could not inspect AppleScript process: {error}"))?
        {
            return Ok(status);
        }
        if started.elapsed() >= timeout {
            return Err(format!(
                "AppleScript command timed out after {} seconds",
                timeout.as_secs()
            ));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_application_by_extension_and_fallback() {
        assert_eq!(deck_kind(Path::new("talk.key")), DeckKind::Keynote);
        assert_eq!(deck_kind(Path::new("talk.PPTX")), DeckKind::PowerPoint);
        assert_eq!(
            select_app(deck_kind(Path::new("talk.ppt")), true, true),
            Some(DeckApp::PowerPoint)
        );
        assert_eq!(
            select_app(deck_kind(Path::new("talk.key")), false, true),
            None
        );
        assert_eq!(
            select_app(DeckKind::Unknown, true, true),
            Some(DeckApp::Keynote)
        );
        assert_eq!(
            select_app(DeckKind::Unknown, false, true),
            Some(DeckApp::PowerPoint)
        );
        assert_eq!(select_app(DeckKind::Unknown, false, false), None);
    }

    #[test]
    fn argv_keeps_paths_separate_from_script_text() {
        let path = "/tmp/a deck; do-not-run.pptx";
        let built = osascript_args(POWERPOINT_EXPORT, &[OsStr::new(path)]);
        assert_eq!(built[0].to_string_lossy(), "-e");
        assert_eq!(built[1].to_string_lossy(), POWERPOINT_EXPORT);
        assert_eq!(built[2].to_string_lossy(), "--");
        assert_eq!(built[3].to_string_lossy(), path);
        assert!(!built[1].to_string_lossy().contains(path));
    }

    #[test]
    fn deck_cache_key_changes_when_source_metadata_changes() {
        let path = Path::new("/tmp/talk.pptx");
        let cache = Path::new("/tmp/cache");
        let base = cached_pdf_path(path, 12, Some(std::time::UNIX_EPOCH), cache);
        assert_ne!(
            base,
            cached_pdf_path(path, 13, Some(std::time::UNIX_EPOCH), cache)
        );
        assert_ne!(
            base,
            cached_pdf_path(
                path,
                12,
                Some(std::time::UNIX_EPOCH + Duration::from_secs(1)),
                cache
            )
        );
    }

    #[test]
    fn notes_parser_handles_empty_values_and_count_mismatches() {
        assert_eq!(parse_notes("", 1), vec![String::new()]);
        assert_eq!(parse_notes("missing value", 1), vec![String::new()]);
        assert_eq!(
            parse_notes("first\u{1d}line\u{1e}\u{1e}third", 3),
            vec!["first\nline", "", "third"]
        );
        assert_eq!(
            parse_notes("only one", 2),
            vec![String::new(), String::new()]
        );
    }

    #[test]
    fn conversion_selection_reports_actionable_keynote_absence() {
        assert_eq!(
            conversion_app(DeckKind::Keynote, false, true).unwrap_err(),
            "Keynote isn't installed. Get it free from the App Store, or export the deck as PDF."
        );
        assert_eq!(
            conversion_app(DeckKind::PowerPoint, false, false).unwrap_err(),
            "Install PowerPoint or Keynote, or export the deck as PDF"
        );
        assert_eq!(
            conversion_app(DeckKind::PowerPoint, true, false).unwrap(),
            DeckApp::Keynote
        );
    }

    #[test]
    fn powerpoint_open_script_enables_presenter_view_before_playback() {
        assert!(
            POWERPOINT_OPEN.find("slide show type presenter").unwrap()
                < POWERPOINT_OPEN.find("run slide show").unwrap()
        );
    }

    #[test]
    fn extensions_are_case_insensitive_and_empty_path_is_invalid() {
        assert_eq!(deck_kind(Path::new("x.KEY")), DeckKind::Keynote);
        assert!(Path::new("").as_os_str().is_empty());
    }
}
