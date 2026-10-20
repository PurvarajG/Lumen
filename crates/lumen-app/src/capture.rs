//! A PowerPoint slideshow played in a window that Lumen captures (macOS ScreenCaptureKit) and
//! shows as the audience background, so Lumen keeps the audience screen and its transitions.
//!
//! One worker thread starts and stops the show, finds its window and owns the stream, like
//! `deck::DeckController`. The newest frame goes to the UI through [`Latest`]: older frames are
//! dropped, never queued. Evidence and limits: `docs/research/powerpoint-window-capture.md`.

use crate::deck::{POWERPOINT_NEXT, POWERPOINT_PREVIOUS, POWERPOINT_STOP};
use lumen_media::Frame;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

const POWERPOINT_BUNDLE: &str = "com.microsoft.Powerpoint";
/// Height of the slideshow window's title bar in points.
const TITLE_BAR: f64 = 32.0;
/// The window's rounded bottom corners leak a few dark pixels into the capture.
const BOTTOM_INSET: f64 = 8.0;
const FIRST_FRAME_WITHIN: Duration = Duration::from_secs(3);
const POLL: Duration = Duration::from_millis(50);
const HEALTH: Duration = Duration::from_secs(2);

/// Window-mode slideshow without `activate`, so Lumen keeps focus. Arguments: left, top, deck file
/// name. The window is made as tall as macOS allows and 16:9 inside its title bar; the reply is
/// `left,top,width,height` in points.
const WINDOW_OPEN: &str = r#"on run argv
set theName to item 3 of argv
tell application id "com.microsoft.Powerpoint"
 set p to missing value
 repeat 40 times
  try
   set p to presentation theName
   exit repeat
  end try
  delay 0.25
 end repeat
 if p is missing value then error "PowerPoint did not open " & theName
 set show type of (slide show settings of p) to slide show type window
 repeat 8 times
  try
   run slide show slide show settings of p
   delay 0.5
   if (count of slide show windows) > 0 then exit repeat
  end try
  delay 0.5
 end repeat
 if (count of slide show windows) is 0 then error "PowerPoint did not start the slideshow"
 tell slide show window 1
  set left position to (item 1 of argv as integer)
  set top to (item 2 of argv as integer)
  set height to 4000
  set h to height
  set width to (h - 32) * 16 / 9
  return (left position as text) & "," & (top as text) & "," & (width as text) & "," & (height as text)
 end tell
end tell
end run"#;
const PROGRESS: &str = "tell application id \"com.microsoft.Powerpoint\"\n return ((current show position of slide show view of slide show window 1) as text) & \",\" & ((count of slides of presentation of slide show window 1) as text)\nend tell";
const SHOW_WINDOWS: &str =
    "tell application id \"com.microsoft.Powerpoint\" to count of slide show windows";

/// A window as ScreenCaptureKit lists it, in points.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowInfo {
    pub id: u32,
    pub bundle: String,
    pub title: String,
    pub width: f64,
    pub height: f64,
}

/// The slideshow window of `deck_stem`: PowerPoint's, titled "PowerPoint Slide Show – [name]", the
/// largest if several match.
pub fn pick_slideshow_window<'a>(
    windows: &'a [WindowInfo],
    deck_stem: &str,
) -> Option<&'a WindowInfo> {
    windows
        .iter()
        .filter(|w| w.bundle == POWERPOINT_BUNDLE && w.title.starts_with("PowerPoint Slide Show"))
        .filter(|w| deck_stem.is_empty() || w.title.contains(deck_stem))
        .max_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
}

/// `sourceRect` (x, y, w, h in window points): the largest centred 16:9 box below the title bar,
/// inset at the bottom.
pub fn crop_rect(width: f64, height: f64) -> (f64, f64, f64, f64) {
    let avail_h = (height - TITLE_BAR - BOTTOM_INSET).max(9.0);
    let w = width.min(avail_h * 16.0 / 9.0).floor();
    let h = (w * 9.0 / 16.0).floor();
    (((width - w) / 2.0).floor(), TITLE_BAR, w, h)
}

/// What to do with a "play inside Lumen" request before anything starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    Capture,
    /// Play fullscreen in PowerPoint instead, and say why.
    Fallback(FailReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailReason {
    NoPermission,
    NoPowerPoint,
    /// Not a PowerPoint deck, or the stream gave no frame in time.
    Other,
}

pub fn plan(permission: bool, powerpoint_installed: bool) -> Plan {
    if !powerpoint_installed {
        Plan::Fallback(FailReason::NoPowerPoint)
    } else if !permission {
        Plan::Fallback(FailReason::NoPermission)
    } else {
        Plan::Capture
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// PowerPoint steps its builds and slides.
    Deck,
    /// The deck is still starting; the press is dropped rather than skipping the cue.
    Hold,
    /// The running order moves on.
    Cues,
}

/// Next/Previous go to PowerPoint while its captured show is up, and to the running order when
/// no captured deck is playing or it has ended.
pub fn route(wanted: bool, live: bool, starting: bool) -> Route {
    match (wanted, live, starting) {
        (true, true, _) => Route::Deck,
        (true, false, true) => Route::Hold,
        _ => Route::Cues,
    }
}

/// The newest captured frame. Replacing drops the older one.
#[derive(Clone, Default)]
pub struct Latest(Arc<Mutex<Slot>>);

#[derive(Default)]
struct Slot {
    frame: Option<Frame>,
    /// While held, new frames wait in `parked` instead of reaching the screen.
    held: bool,
    parked: Option<Frame>,
}

impl Latest {
    pub fn put(&self, frame: Frame) {
        if let Ok(mut slot) = self.0.lock() {
            if slot.held {
                slot.parked = Some(frame);
            } else {
                slot.frame = Some(frame);
            }
        }
    }
    pub fn take(&self) -> Option<Frame> {
        self.0.lock().ok().and_then(|mut slot| slot.frame.take())
    }
    /// Keeps what follows off the screen until [`release`](Self::release) says whether to show it.
    fn hold(&self) {
        if let Ok(mut slot) = self.0.lock() {
            slot.held = true;
        }
    }
    fn release(&self, show: bool) {
        if let Ok(mut slot) = self.0.lock() {
            slot.held = false;
            let parked = slot.parked.take();
            if show && parked.is_some() {
                slot.frame = parked;
            }
        }
    }
    fn clear(&self) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Slot::default();
        }
    }
    fn has_frame(&self) -> bool {
        self.0.lock().is_ok_and(|slot| slot.frame.is_some())
    }
}

/// PowerPoint answers "position,slides" where position runs past the last slide on its black
/// "End of slide show" screen, which the audience must never see.
fn parse_progress(reply: &str) -> Option<(u32, u32)> {
    let (position, slides) = reply.split_once(',')?;
    Some((position.trim().parse().ok()?, slides.trim().parse().ok()?))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    /// Capture size in pixels.
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, PartialEq)]
pub enum Event {
    /// The first frame arrived.
    Started(PathBuf),
    /// Could not start; the deck should play fullscreen in PowerPoint.
    Failed(PathBuf, FailReason, String),
    /// The show ended, PowerPoint quit or the stream stopped; the last frame stays up.
    Ended(PathBuf, String),
}

enum Command {
    Start(PathBuf, Placement),
    Stop,
    Navigate(bool),
    /// Answers once every earlier command is done.
    Flush(mpsc::Sender<()>),
}

pub struct CaptureController {
    commands: mpsc::Sender<Command>,
    events: mpsc::Receiver<Event>,
    latest: Latest,
    /// The deck whose capture is wanted or running, and whether its first frame has arrived.
    wanted: Option<PathBuf>,
    live: bool,
    ended: bool,
    /// A frame has been shown since the capture started; it stays true after the show ends.
    shown: bool,
}

impl Default for CaptureController {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureController {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let latest = Latest::default();
        let worker_latest = latest.clone();
        thread::Builder::new()
            .name("lumen-capture".into())
            .spawn(move || worker(command_rx, event_tx, worker_latest))
            .expect("failed to start capture worker");
        Self {
            commands: command_tx,
            events: event_rx,
            latest,
            wanted: None,
            live: false,
            ended: false,
            shown: false,
        }
    }

    pub fn latest(&self) -> Latest {
        self.latest.clone()
    }

    /// Start `path`, or stop the capture when `None`. Repeating the current path does nothing.
    pub fn reconcile(&mut self, path: Option<PathBuf>, placement: Placement) {
        if path == self.wanted {
            return;
        }
        self.live = false;
        self.ended = false;
        self.shown = false;
        self.latest.clear();
        let _ = self.commands.send(Command::Stop);
        self.wanted = path.clone();
        if let Some(path) = path {
            let _ = self.commands.send(Command::Start(path, placement));
        }
    }

    pub fn navigate(&self, forward: bool) {
        let _ = self.commands.send(Command::Navigate(forward));
    }

    /// True from the first frame until the show ends.
    pub fn is_live(&self) -> bool {
        self.live
    }

    /// True once the deck's picture has been up, also after its show ended (the last frame holds).
    pub fn has_shown(&self) -> bool {
        self.shown
    }

    pub fn is_wanted(&self) -> bool {
        self.wanted.is_some()
    }

    /// True while a capture is wanted but has neither delivered a frame nor failed.
    pub fn is_starting(&self) -> bool {
        self.wanted.is_some() && !self.live && !self.ended
    }

    /// The next worker result for the deck still wanted; results for an older deck are dropped.
    pub fn poll(&mut self) -> Option<Event> {
        while let Ok(event) = self.events.try_recv() {
            let path = match &event {
                Event::Started(p) | Event::Failed(p, ..) | Event::Ended(p, _) => p,
            };
            if self.wanted.as_ref() != Some(path) {
                continue;
            }
            match &event {
                Event::Started(_) => {
                    self.live = true;
                    self.shown = true;
                }
                Event::Failed(..) => {
                    self.live = false;
                    self.wanted = None;
                }
                Event::Ended(..) => {
                    self.live = false;
                    self.ended = true;
                }
            }
            return Some(event);
        }
        None
    }
}

impl CaptureController {
    /// Stops the stream and PowerPoint's show, waiting briefly so quitting does not leave it up.
    pub fn shutdown(&mut self) {
        let (done, wait) = mpsc::channel();
        let _ = self.commands.send(Command::Stop);
        let _ = self.commands.send(Command::Flush(done));
        let _ = wait.recv_timeout(Duration::from_secs(4));
    }
}

fn osascript(script: &str, argv: &[&OsStr]) -> Result<String, String> {
    use std::process::{Command, Stdio};
    let mut command = Command::new("osascript");
    command.arg("-e").arg(script).arg("--").args(argv);
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run AppleScript: {e}"))?;
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Could not read AppleScript result: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(format!(
            "AppleScript failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn powerpoint_running() -> bool {
    native::running(POWERPOINT_BUNDLE)
}

fn powerpoint_installed() -> bool {
    native::installed(POWERPOINT_BUNDLE)
}

struct Running {
    path: PathBuf,
    stream: native::Stream,
    next_health: Instant,
}

fn stop_show() {
    if powerpoint_running() {
        let _ = osascript(POWERPOINT_STOP, &[]);
    }
}

fn worker(commands: mpsc::Receiver<Command>, events: mpsc::Sender<Event>, latest: Latest) {
    let mut running: Option<Running> = None;
    loop {
        match commands.recv_timeout(POLL) {
            Ok(Command::Stop) => {
                if let Some(run) = running.take() {
                    run.stream.stop();
                    stop_show();
                }
            }
            Ok(Command::Start(path, placement)) => {
                if let Some(run) = running.take() {
                    run.stream.stop();
                    stop_show();
                }
                match start(&path, placement, &latest) {
                    Ok(stream) => {
                        let _ = events.send(Event::Started(path.clone()));
                        running = Some(Running {
                            path,
                            stream,
                            next_health: Instant::now() + HEALTH,
                        });
                    }
                    Err((reason, message)) => {
                        stop_show();
                        let _ = events.send(Event::Failed(path, reason, message));
                    }
                }
            }
            Ok(Command::Flush(done)) => {
                let _ = done.send(());
            }
            Ok(Command::Navigate(forward)) => navigate(forward, &mut running, &events, &latest),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if let Some(run) = &mut running
            && Instant::now() >= run.next_health
        {
            run.next_health = Instant::now() + HEALTH;
            if !powerpoint_running() {
                end(&mut running, &events, "PowerPoint quit");
            } else if run.stream.stopped() {
                end(&mut running, &events, "The capture stopped");
            } else if show_windows() == 0 {
                end(&mut running, &events, "The slideshow ended");
            }
        }
    }
    if let Some(run) = running {
        run.stream.stop();
        stop_show();
    }
}

fn show_windows() -> i64 {
    osascript(SHOW_WINDOWS, &[])
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Stops the stream and the show, keeps the last picture up (the UI keeps its texture), reports it.
fn end(running: &mut Option<Running>, events: &mpsc::Sender<Event>, why: &str) {
    if let Some(run) = running.take() {
        run.stream.stop();
        stop_show();
        let _ = events.send(Event::Ended(run.path, why.into()));
    }
}

/// One Next or Previous. A Next that would show PowerPoint's black end screen is held back:
/// the last real picture stays up and the show ends instead.
fn navigate(
    forward: bool,
    running: &mut Option<Running>,
    events: &mpsc::Sender<Event>,
    latest: &Latest,
) {
    if running.is_none() {
        return;
    }
    let on_last = forward
        && osascript(PROGRESS, &[])
            .ok()
            .and_then(|reply| parse_progress(&reply))
            .is_none_or(|(position, slides)| position >= slides);
    if on_last {
        latest.hold();
    }
    let sent = osascript(
        if forward {
            POWERPOINT_NEXT
        } else {
            POWERPOINT_PREVIOUS
        },
        &[],
    );
    let past_end = sent.is_err()
        || osascript(PROGRESS, &[])
            .ok()
            .and_then(|reply| parse_progress(&reply))
            .is_none_or(|(position, slides)| position > slides);
    if on_last {
        latest.release(!past_end);
    }
    if past_end {
        end(running, events, "The slideshow ended");
    }
}

fn start(
    path: &Path,
    placement: Placement,
    latest: &Latest,
) -> Result<native::Stream, (FailReason, String)> {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(ext.as_str(), "ppt" | "pptx") {
        return Err((
            FailReason::Other,
            "Only PowerPoint decks play inside Lumen".into(),
        ));
    }
    if !native::supported() {
        return Err((
            FailReason::Other,
            "Playing inside Lumen needs macOS 13 or newer".into(),
        ));
    }
    let permission = native::screen_access();
    match plan(permission, powerpoint_installed()) {
        Plan::Capture => {}
        Plan::Fallback(reason) => {
            if reason == FailReason::NoPermission {
                native::request_screen_access();
            }
            return Err((reason, String::new()));
        }
    }
    let fail = |message: String| (FailReason::Other, message);
    // `-g` loads the deck without bringing PowerPoint to the front.
    let opened = std::process::Command::new("/usr/bin/open")
        .arg("-g")
        .arg("-b")
        .arg(POWERPOINT_BUNDLE)
        .arg(path)
        .status()
        .map_err(|e| fail(format!("Could not open deck: {e}")))?;
    if !opened.success() {
        return Err(fail(format!("Could not open deck {}", path.display())));
    }
    let name = path.file_name().unwrap_or_default();
    let (left, top) = (placement.x.to_string(), placement.y.to_string());
    let frame = osascript(WINDOW_OPEN, &[left.as_ref(), top.as_ref(), name]).map_err(fail)?;
    let numbers: Vec<f64> = frame
        .split(',')
        .filter_map(|n| n.trim().parse().ok())
        .collect();
    let [_, _, width, height] = numbers[..] else {
        return Err(fail(format!(
            "PowerPoint reported an odd window size: {frame}"
        )));
    };
    latest.clear();
    let deadline = Instant::now() + Duration::from_secs(4);
    let window = loop {
        let windows = native::windows().map_err(fail)?;
        if let Some(found) = pick_slideshow_window(&windows.info, &stem) {
            break windows.start(
                found.id,
                crop_rect(width, height),
                placement,
                latest.clone(),
            );
        }
        if Instant::now() >= deadline {
            return Err(fail("The slideshow window did not appear".into()));
        }
        thread::sleep(Duration::from_millis(200));
    };
    let stream = window.map_err(fail)?;
    let first = Instant::now() + FIRST_FRAME_WITHIN;
    while !latest.has_frame() {
        if Instant::now() >= first {
            stream.stop();
            return Err(fail("No picture from PowerPoint within 3 seconds".into()));
        }
        thread::sleep(POLL);
    }
    Ok(stream)
}

#[cfg(target_os = "macos")]
mod native {
    use super::{Latest, Placement, WindowInfo};
    use block2::RcBlock;
    use dispatch2::{DispatchQueue, DispatchRetained};
    use lumen_media::Frame;
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_core_media::{CMSampleBuffer, CMTime, CMTimeFlags};
    use objc2_foundation_v6::{NSError, NSObject, NSObjectProtocol};
    use objc2_screen_capture_kit::{
        SCContentFilter, SCShareableContent, SCStream, SCStreamConfiguration, SCStreamDelegate,
        SCStreamOutput, SCStreamOutputType, SCWindow,
    };
    use objc2_v6::{
        AnyThread, DefinedClass, define_class, msg_send, rc::Retained, runtime::ProtocolObject,
    };
    use std::{
        ffi::c_void,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU64, Ordering},
            mpsc,
        },
        time::Duration,
    };

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }
    #[link(name = "CoreMedia", kind = "framework")]
    unsafe extern "C" {
        fn CMSampleBufferGetImageBuffer(sample: *const c_void) -> *mut c_void;
    }
    #[link(name = "CoreVideo", kind = "framework")]
    unsafe extern "C" {
        fn CVPixelBufferLockBaseAddress(buffer: *mut c_void, flags: u64) -> i32;
        fn CVPixelBufferUnlockBaseAddress(buffer: *mut c_void, flags: u64) -> i32;
        fn CVPixelBufferGetBaseAddress(buffer: *mut c_void) -> *const u8;
        fn CVPixelBufferGetBytesPerRow(buffer: *mut c_void) -> usize;
        fn CVPixelBufferGetWidth(buffer: *mut c_void) -> usize;
        fn CVPixelBufferGetHeight(buffer: *mut c_void) -> usize;
    }
    const READ_ONLY: u64 = 1;
    /// `kCVPixelFormatType_32BGRA`
    const BGRA: u32 = u32::from_be_bytes(*b"BGRA");

    /// ScreenCaptureKit window capture needs macOS 13; the class is simply absent before that.
    pub fn supported() -> bool {
        objc2_v6::runtime::AnyClass::get(c"SCStream").is_some()
    }
    pub fn screen_access() -> bool {
        unsafe { CGPreflightScreenCaptureAccess() }
    }
    pub fn request_screen_access() {
        unsafe { CGRequestScreenCaptureAccess() };
    }

    pub fn installed(bundle: &str) -> bool {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::NSString;
        let bundle = NSString::from_str(bundle);
        unsafe {
            NSWorkspace::sharedWorkspace()
                .URLForApplicationWithBundleIdentifier(&bundle)
                .is_some()
        }
    }

    pub fn running(bundle: &str) -> bool {
        use objc2_app_kit::NSRunningApplication;
        use objc2_foundation::NSString;
        let bundle = NSString::from_str(bundle);
        unsafe {
            NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle).count() > 0
        }
    }

    struct Ivars {
        latest: Latest,
        stopped: Arc<AtomicBool>,
        sequence: AtomicU64,
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "LumenCaptureOutput"]
        #[ivars = Ivars]
        struct Output;

        unsafe impl NSObjectProtocol for Output {}

        unsafe impl SCStreamOutput for Output {
            #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
            fn did_output(
                &self,
                _stream: &SCStream,
                sample: &CMSampleBuffer,
                kind: SCStreamOutputType,
            ) {
                if kind == SCStreamOutputType::Screen
                    && let Some(mut frame) = read_frame(sample)
                {
                    frame.sequence = self.ivars().sequence.fetch_add(1, Ordering::Relaxed);
                    self.ivars().latest.put(frame);
                }
            }
        }

        unsafe impl SCStreamDelegate for Output {
            #[unsafe(method(stream:didStopWithError:))]
            fn did_stop(&self, _stream: &SCStream, _error: &NSError) {
                self.ivars().stopped.store(true, Ordering::Release);
            }
        }
    );

    impl Output {
        fn new(ivars: Ivars) -> Retained<Self> {
            let this = Self::alloc().set_ivars(ivars);
            unsafe { msg_send![super(this), init] }
        }
    }

    /// BGRA pixels of one complete frame as premultiplied RGBA; idle frames carry no picture.
    fn read_frame(sample: &CMSampleBuffer) -> Option<Frame> {
        unsafe {
            let buffer = CMSampleBufferGetImageBuffer(std::ptr::from_ref(sample).cast());
            if buffer.is_null() || CVPixelBufferLockBaseAddress(buffer, READ_ONLY) != 0 {
                return None;
            }
            let (width, height) = (
                CVPixelBufferGetWidth(buffer),
                CVPixelBufferGetHeight(buffer),
            );
            let stride = CVPixelBufferGetBytesPerRow(buffer);
            let base = CVPixelBufferGetBaseAddress(buffer);
            let mut rgba = vec![0u8; width * height * 4];
            if !base.is_null() {
                for y in 0..height {
                    let row = std::slice::from_raw_parts(base.add(y * stride), width * 4);
                    let out = &mut rgba[y * width * 4..(y + 1) * width * 4];
                    let (src, dst) = (row.as_chunks::<4>().0, out.as_chunks_mut::<4>().0);
                    for (src, dst) in src.iter().zip(dst) {
                        *dst = [src[2], src[1], src[0], src[3]];
                    }
                }
            }
            CVPixelBufferUnlockBaseAddress(buffer, READ_ONLY);
            (!base.is_null()).then_some(Frame {
                width: width as u32,
                height: height as u32,
                rgba,
                sequence: 0,
            })
        }
    }

    struct Send<T>(T);
    // SAFETY: ScreenCaptureKit objects may be used from any thread.
    unsafe impl<T> std::marker::Send for Send<T> {}

    pub struct Windows {
        pub info: Vec<WindowInfo>,
        content: Send<Retained<SCShareableContent>>,
    }

    /// Lists the windows ScreenCaptureKit can capture, on screen or not.
    pub fn windows() -> Result<Windows, String> {
        let (tx, rx) = mpsc::channel();
        let block = RcBlock::new(
            move |content: *mut SCShareableContent, error: *mut NSError| {
                let result = unsafe { Retained::retain(content) }
                    .map(Send)
                    .ok_or_else(|| {
                        unsafe { error.as_ref() }.map_or("No windows listed".into(), describe)
                    });
                let _ = tx.send(result);
            },
        );
        unsafe {
            SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(
                false, false, &block,
            );
        }
        let content = rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "ScreenCaptureKit did not answer".to_owned())?
            .map_err(|e| format!("Screen Recording is not available: {e}"))?;
        let info = unsafe { content.0.windows() }
            .iter()
            .map(|w| unsafe {
                let frame = w.frame();
                WindowInfo {
                    id: w.windowID(),
                    bundle: w
                        .owningApplication()
                        .map(|a| a.bundleIdentifier().to_string())
                        .unwrap_or_default(),
                    title: w.title().map(|t| t.to_string()).unwrap_or_default(),
                    width: frame.size.width,
                    height: frame.size.height,
                }
            })
            .collect();
        Ok(Windows { info, content })
    }

    fn describe(error: &NSError) -> String {
        error.localizedDescription().to_string()
    }

    impl Windows {
        pub fn start(
            &self,
            id: u32,
            crop: (f64, f64, f64, f64),
            placement: Placement,
            latest: Latest,
        ) -> Result<Stream, String> {
            let window: Retained<SCWindow> = unsafe { self.content.0.windows() }
                .iter()
                .find(|w| unsafe { w.windowID() } == id)
                .ok_or("The slideshow window disappeared")?;
            let stopped = Arc::new(AtomicBool::new(false));
            let output = Output::new(Ivars {
                latest,
                stopped: stopped.clone(),
                sequence: AtomicU64::new(0),
            });
            unsafe {
                let config = SCStreamConfiguration::new();
                config.setWidth(placement.width.clamp(2, 3840) as usize);
                config.setHeight(placement.height.clamp(2, 2160) as usize);
                config.setPixelFormat(BGRA);
                config.setShowsCursor(false);
                // The cropped window fills the output; without this it keeps 1:1 scale in a corner.
                config.setScalesToFit(true);
                // Added in macOS 14; older systems would crash on the unknown selector.
                if config.respondsToSelector(objc2_v6::sel!(setIgnoreShadowsSingleWindow:)) {
                    config.setIgnoreShadowsSingleWindow(true);
                }
                config.setQueueDepth(3);
                // 30 fps keeps an even two-refresh cadence on a 60 Hz output and halves the copy and
                // upload cost; 60 fps measured 36-38% of a core on top of PowerPoint's own 33%.
                config.setMinimumFrameInterval(CMTime {
                    value: 1,
                    timescale: 30,
                    flags: CMTimeFlags::Valid,
                    epoch: 0,
                });
                config.setSourceRect(CGRect {
                    origin: CGPoint {
                        x: crop.0,
                        y: crop.1,
                    },
                    size: CGSize {
                        width: crop.2,
                        height: crop.3,
                    },
                });
                let filter = SCContentFilter::initWithDesktopIndependentWindow(
                    SCContentFilter::alloc(),
                    &window,
                );
                let delegate = ProtocolObject::from_ref(&*output);
                let stream = SCStream::initWithFilter_configuration_delegate(
                    SCStream::alloc(),
                    &filter,
                    &config,
                    Some(delegate),
                );
                let queue = DispatchQueue::new("lumen.capture", None);
                stream
                    .addStreamOutput_type_sampleHandlerQueue_error(
                        ProtocolObject::from_ref(&*output),
                        SCStreamOutputType::Screen,
                        Some(&queue),
                    )
                    .map_err(|e| describe(&e))?;
                let (tx, rx) = mpsc::channel();
                let block = RcBlock::new(move |error: *mut NSError| {
                    let _ = tx.send(error.as_ref().map(describe));
                });
                stream.startCaptureWithCompletionHandler(Some(&block));
                match rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(None) => {}
                    Ok(Some(error)) => return Err(format!("Capture did not start: {error}")),
                    Err(_) => return Err("Capture did not start in time".into()),
                }
                Ok(Stream {
                    stream: Send(stream),
                    _output: Send(output),
                    _queue: Send(queue),
                    stopped,
                })
            }
        }
    }

    pub struct Stream {
        stream: Send<Retained<SCStream>>,
        _output: Send<Retained<Output>>,
        _queue: Send<DispatchRetained<DispatchQueue>>,
        stopped: Arc<AtomicBool>,
    }

    impl Stream {
        /// True once macOS ended the stream by itself.
        pub fn stopped(&self) -> bool {
            self.stopped.load(Ordering::Acquire)
        }
        pub fn stop(self) {
            let (tx, rx) = mpsc::channel();
            let block = RcBlock::new(move |_error: *mut NSError| {
                let _ = tx.send(());
            });
            unsafe { self.stream.0.stopCaptureWithCompletionHandler(Some(&block)) };
            let _ = rx.recv_timeout(Duration::from_secs(3));
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod native {
    use super::{Latest, Placement, WindowInfo};
    pub fn supported() -> bool {
        false
    }
    pub fn screen_access() -> bool {
        false
    }
    pub fn request_screen_access() {}
    pub fn installed(_: &str) -> bool {
        false
    }
    pub fn running(_: &str) -> bool {
        false
    }
    pub struct Windows {
        pub info: Vec<WindowInfo>,
    }
    pub fn windows() -> Result<Windows, String> {
        Err("Window capture needs macOS".into())
    }
    impl Windows {
        pub fn start(
            &self,
            _: u32,
            _: (f64, f64, f64, f64),
            _: Placement,
            _: Latest,
        ) -> Result<Stream, String> {
            Err("Window capture needs macOS".into())
        }
    }
    pub struct Stream;
    impl Stream {
        pub fn stopped(&self) -> bool {
            true
        }
        pub fn stop(self) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(id: u32, bundle: &str, title: &str, w: f64, h: f64) -> WindowInfo {
        WindowInfo {
            id,
            bundle: bundle.into(),
            title: title.into(),
            width: w,
            height: h,
        }
    }

    #[test]
    fn picks_the_powerpoint_slideshow_window_of_the_deck() {
        let windows = [
            window(1, POWERPOINT_BUNDLE, "talk", 1512.0, 900.0),
            window(2, POWERPOINT_BUNDLE, "", 1920.0, 30.0),
            window(
                3,
                "com.apple.Safari",
                "PowerPoint Slide Show – [talk]",
                1814.0,
                1050.0,
            ),
            window(
                4,
                POWERPOINT_BUNDLE,
                "PowerPoint Slide Show – [other]",
                1814.0,
                1050.0,
            ),
            window(
                5,
                POWERPOINT_BUNDLE,
                "PowerPoint Slide Show – [talk]",
                1000.0,
                600.0,
            ),
            window(
                6,
                POWERPOINT_BUNDLE,
                "PowerPoint Slide Show – [talk]",
                1814.0,
                1050.0,
            ),
        ];
        assert_eq!(
            pick_slideshow_window(&windows, "talk").map(|w| w.id),
            Some(6)
        );
        assert_eq!(pick_slideshow_window(&windows, "missing"), None);
        assert_eq!(pick_slideshow_window(&windows[..2], "talk"), None);
    }

    #[test]
    fn crop_is_the_largest_centred_16_9_below_the_title_bar() {
        // The window from the spike: 1814 × 1050 pt.
        let (x, y, w, h) = crop_rect(1814.0, 1050.0);
        assert_eq!(y, 32.0);
        assert!((h - w * 9.0 / 16.0).abs() < 1.0);
        assert!(w <= 1814.0 && x + w <= 1814.0 && y + h <= 1050.0 - 8.0);
        assert!((w / h - 16.0 / 9.0).abs() < 0.01);
        // A wide, short window is limited by its height; a narrow one by its width.
        assert_eq!(
            crop_rect(2400.0, 568.0).2,
            ((568.0 - 40.0) * 16.0 / 9.0f64).floor()
        );
        let (x, _, w, _) = crop_rect(800.0, 1000.0);
        assert_eq!((x, w), (0.0, 800.0));
    }

    #[test]
    fn falls_back_to_fullscreen_without_permission_or_powerpoint() {
        assert_eq!(plan(true, true), Plan::Capture);
        assert_eq!(plan(false, true), Plan::Fallback(FailReason::NoPermission));
        assert_eq!(plan(true, false), Plan::Fallback(FailReason::NoPowerPoint));
        assert_eq!(plan(false, false), Plan::Fallback(FailReason::NoPowerPoint));
    }

    #[test]
    fn next_goes_to_powerpoint_only_while_its_show_is_up() {
        assert_eq!(route(true, true, false), Route::Deck);
        assert_eq!(route(true, false, true), Route::Hold, "still starting");
        assert_eq!(
            route(true, false, false),
            Route::Cues,
            "show ended or failed"
        );
        assert_eq!(route(false, false, false), Route::Cues);
    }

    #[test]
    fn a_held_frame_reaches_the_screen_only_when_released_for_showing() {
        let latest = Latest::default();
        let frame = |sequence| Frame {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
            sequence,
        };
        latest.put(frame(1));
        latest.hold();
        latest.put(frame(2));
        assert_eq!(
            latest.take().map(|f| f.sequence),
            Some(1),
            "old picture stays"
        );
        latest.release(false);
        assert!(latest.take().is_none(), "the end screen is dropped");
        latest.hold();
        latest.put(frame(3));
        latest.release(true);
        assert_eq!(latest.take().map(|f| f.sequence), Some(3));
    }

    #[test]
    fn progress_past_the_last_slide_is_the_end_screen() {
        assert_eq!(parse_progress("2,3"), Some((2, 3)));
        assert_eq!(parse_progress("4,3"), Some((4, 3)));
        assert_eq!(parse_progress("missing value"), None);
    }

    #[test]
    fn the_latest_frame_replaces_the_older_one() {
        let latest = Latest::default();
        let frame = |sequence| Frame {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
            sequence,
        };
        assert!(latest.take().is_none());
        latest.put(frame(1));
        latest.put(frame(2));
        assert_eq!(latest.take().map(|f| f.sequence), Some(2));
        assert!(latest.take().is_none());
    }
}
