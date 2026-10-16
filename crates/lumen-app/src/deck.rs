//! Asynchronous control of Keynote and PowerPoint slide shows.
//!
//! UI code communicates with one worker through [`DeckController`]. App lookup,
//! AppleScript execution, and health checks all run on that worker.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
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
const KEYNOTE_NEXT: &str = "tell application id \"com.apple.iWork.Keynote\" to show next";
const KEYNOTE_PREVIOUS: &str = "tell application id \"com.apple.iWork.Keynote\" to show previous";
const POWERPOINT_NEXT: &str = "tell application id \"com.microsoft.Powerpoint\" to go to next slide slideshow view of slide show window 1";
const POWERPOINT_PREVIOUS: &str = "tell application id \"com.microsoft.Powerpoint\" to go to previous slide slideshow view of slide show window 1";
const KEYNOTE_STOP: &str = "tell application id \"com.apple.iWork.Keynote\" to stop front document";
const POWERPOINT_STOP: &str = "tell application id \"com.microsoft.Powerpoint\" to exit slide show slideshow view of slide show window 1";

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
    let args = osascript_args(script, argv);
    let mut child = Command::new("osascript")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not run AppleScript: {error}"))?;
    match wait_with_timeout(&mut child, PROCESS_TIMEOUT) {
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
            return Err("AppleScript command timed out after 10 seconds".into());
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
        let built = osascript_args(KEYNOTE_OPEN, &[OsStr::new(path)]);
        assert_eq!(built[0].to_string_lossy(), "-e");
        assert_eq!(built[1].to_string_lossy(), KEYNOTE_OPEN);
        assert_eq!(built[2].to_string_lossy(), "--");
        assert_eq!(built[3].to_string_lossy(), path);
        assert!(!built[1].to_string_lossy().contains(path));
    }

    #[test]
    fn extensions_are_case_insensitive_and_empty_path_is_invalid() {
        assert_eq!(deck_kind(Path::new("x.KEY")), DeckKind::Keynote);
        assert!(Path::new("").as_os_str().is_empty());
    }
}
