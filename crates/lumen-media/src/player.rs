use crate::{MediaError, probe::bounded_text, probe::probe_with_program, validate_file};
use std::{
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// A frame this late means the clock stalled (pause, loop restart, slow decode).
const REBASE_AFTER: Duration = Duration::from_millis(250);
/// Frames the reader may decode ahead of the pacer.
const READ_AHEAD: usize = 3;

#[derive(Clone, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub sequence: u64,
}

#[derive(Clone, Debug)]
pub struct PlaybackOptions {
    /// Largest decoded size; smaller sources are never upscaled.
    pub width: u32,
    pub height: u32,
    /// Maximum rate; the source rate is used when it is lower.
    pub fps: u32,
    pub looped: bool,
    pub muted: bool,
}

impl Default for PlaybackOptions {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            fps: 30,
            looped: true,
            muted: true,
        }
    }
}

#[derive(Default)]
struct Children {
    video: Option<Child>,
    audio: Option<Child>,
}

pub struct MediaPlayer {
    children: Arc<Mutex<Children>>,
    latest: Arc<Mutex<Option<Frame>>>,
    error: Arc<Mutex<Option<String>>>,
    finished: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
    stderr_readers: Vec<JoinHandle<()>>,
    audio_monitor: Option<JoinHandle<()>>,
    cancel_audio_monitor: Arc<AtomicBool>,
    stopped: bool,
}

impl MediaPlayer {
    pub fn open(path: &Path, options: PlaybackOptions) -> Result<Self, MediaError> {
        Self::open_with_programs(
            path,
            options,
            "ffprobe",
            std::ffi::OsStr::new("ffmpeg"),
            std::ffi::OsStr::new("ffplay"),
        )
    }

    fn open_with_programs(
        path: &Path,
        options: PlaybackOptions,
        probe_program: &str,
        ffmpeg_program: &std::ffi::OsStr,
        ffplay_program: &std::ffi::OsStr,
    ) -> Result<Self, MediaError> {
        validate_file(path)?;
        validate_options(&options)?;
        let path_arg = path.as_os_str();
        let info = probe_with_program(path, probe_program)?;
        let (width, height) = fit_size((info.width, info.height), (options.width, options.height));
        let frame_len = width as usize * height as usize * 4;
        let fps = info
            .fps
            .map_or(30.0, f64::round)
            .clamp(1.0, f64::from(options.fps)) as u32;
        let mut video = Command::new(ffmpeg_program);
        video.args(["-hide_banner", "-loglevel", "error", "-nostdin"]);
        if options.looped {
            video.args(["-stream_loop", "-1"]);
        }
        let filter = format!("scale={width}:{height},fps={fps}:round=near,format=rgba");
        video
            .args([
                std::ffi::OsStr::new("-i"),
                path_arg,
                std::ffi::OsStr::new("-vf"),
                std::ffi::OsStr::new(&filter),
                std::ffi::OsStr::new("-an"),
                std::ffi::OsStr::new("-sn"),
                std::ffi::OsStr::new("-dn"),
                std::ffi::OsStr::new("-f"),
                std::ffi::OsStr::new("rawvideo"),
                std::ffi::OsStr::new("-pix_fmt"),
                std::ffi::OsStr::new("rgba"),
                std::ffi::OsStr::new("pipe:1"),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let ffmpeg_label = ffmpeg_program.to_string_lossy();
        let mut video_child = spawn(&mut video, &ffmpeg_label)?;
        let stdout = video_child
            .stdout
            .take()
            .ok_or_else(|| MediaError::Process("ffmpeg stdout pipe unavailable".into()))?;
        let video_err = video_child
            .stderr
            .take()
            .ok_or_else(|| MediaError::Process("ffmpeg stderr pipe unavailable".into()))?;
        let video_stderr = Arc::new(Mutex::new(Vec::new()));
        let mut stderr_readers = match drain_stderr(video_err, video_stderr.clone(), "ffmpeg") {
            Ok(reader) => vec![reader],
            Err(e) => {
                terminate_reap(&mut video_child);
                return Err(e);
            }
        };
        let audio_stderr = Arc::new(Mutex::new(Vec::new()));

        let mut audio_child = if !options.muted && info.has_audio {
            match start_audio_child(
                ffplay_program,
                path_arg,
                options.looped,
                &mut video_child,
                &mut stderr_readers,
                audio_stderr.clone(),
            ) {
                Ok(child) => Some(child),
                Err(e) => {
                    for h in stderr_readers {
                        let _ = h.join();
                    }
                    return Err(e);
                }
            }
        } else {
            None
        };

        let children = Arc::new(Mutex::new(Children {
            video: Some(video_child),
            audio: audio_child.take(),
        }));
        let latest = Arc::new(Mutex::new(None));
        let error = Arc::new(Mutex::new(None));
        let finished = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let cancel_audio_monitor = Arc::new(AtomicBool::new(false));
        let audio_monitor = if lock(&children).audio.is_some() {
            let children = children.clone();
            let error = error.clone();
            let children_for_failure = children.clone();
            let stderr = audio_stderr.clone();
            let cancel = cancel_audio_monitor.clone();
            Some(
                thread::Builder::new()
                    .name("lumen-media-audio-watch".into())
                    .spawn(move || {
                        while !cancel.load(Ordering::Acquire) {
                            let outcome = {
                                let mut g = lock(&children);
                                match g.audio.as_mut() {
                                    Some(child) => match child.try_wait() {
                                        Ok(Some(status)) => {
                                            g.audio = None;
                                            Ok(Some(status))
                                        }
                                        Ok(None) => Ok(None),
                                        Err(e) => Err(e),
                                    },
                                    None => Ok(None),
                                }
                            };
                            match outcome {
                                Ok(Some(status)) => {
                                    if !status.success() {
                                        set_error(
                                            &error,
                                            format!(
                                                "ffplay exited {status}: {}",
                                                bounded_text(&lock(&stderr))
                                            ),
                                        );
                                    }
                                    break;
                                }
                                Err(e) => {
                                    set_error(
                                        &error,
                                        format!("could not inspect ffplay process: {e}"),
                                    );
                                    break;
                                }
                                Ok(None) => {}
                            }
                            thread::sleep(std::time::Duration::from_millis(40));
                        }
                    })
                    .map_err(|e| {
                        terminate_children(&children_for_failure);
                        for h in stderr_readers.drain(..) {
                            let _ = h.join();
                        }
                        MediaError::Process(format!("could not start audio monitor: {e}"))
                    })?,
            )
        } else {
            None
        };
        let worker_result = {
            let context = DecoderContext {
                frame_len,
                width,
                height,
                fps,
                children: children.clone(),
                latest: latest.clone(),
                error: error.clone(),
                finished: finished.clone(),
                paused: paused.clone(),
                stderr: video_stderr,
            };
            start_decoder(stdout, context)
        };
        let workers = match worker_result {
            Ok(workers) => workers,
            Err(e) => {
                cancel_audio_monitor.store(true, Ordering::Release);
                terminate_children(&children);
                if let Some(monitor) = audio_monitor {
                    let _ = monitor.join();
                }
                for h in stderr_readers {
                    let _ = h.join();
                }
                return Err(MediaError::Process(format!(
                    "could not start decoder worker: {e}"
                )));
            }
        };
        Ok(Self {
            children,
            latest,
            error,
            finished,
            paused,
            workers,
            stderr_readers,
            audio_monitor,
            cancel_audio_monitor,
            stopped: false,
        })
    }

    /// Takes the newest frame, if one has arrived since the last call.
    pub fn take_frame(&self) -> Option<Frame> {
        lock(&self.latest).take()
    }
    pub fn error(&self) -> Option<String> {
        lock(&self.error).clone()
    }
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    pub fn set_paused(&mut self, paused: bool) -> Result<(), MediaError> {
        if self.stopped || self.is_finished() {
            return Err(MediaError::Process(
                "cannot change pause state after playback has finished".into(),
            ));
        }
        if self.is_paused() == paused {
            return Ok(());
        }
        #[cfg(unix)]
        {
            let mut children = lock(&self.children);
            let mut signalled: Vec<libc::pid_t> = Vec::new();
            pause_one(&mut children.video, paused, &mut signalled)?;
            pause_one(&mut children.audio, paused, &mut signalled)?;
            self.paused.store(paused, Ordering::Release);
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = paused;
            Err(MediaError::Unsupported(
                "pause/resume requires Unix process signals".into(),
            ))
        }
    }

    pub fn stop(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        self.finished.store(true, Ordering::Release);
        self.paused.store(false, Ordering::Release);
        self.cancel_audio_monitor.store(true, Ordering::Release);
        terminate_children(&self.children);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
        if let Some(monitor) = self.audio_monitor.take() {
            let _ = monitor.join();
        }
        for reader in self.stderr_readers.drain(..) {
            let _ = reader.join();
        }
        self.finished.store(true, Ordering::Release);
    }
}

impl Drop for MediaPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Largest size that fits `max` with the source aspect ratio, never above the source.
fn fit_size(src: (u32, u32), max: (u32, u32)) -> (u32, u32) {
    let scale = (f64::from(max.0) / f64::from(src.0))
        .min(f64::from(max.1) / f64::from(src.1))
        .min(1.0);
    let fit = |v: u32| ((f64::from(v) * scale).round() as u32).max(1);
    (fit(src.0), fit(src.1))
}

fn validate_options(o: &PlaybackOptions) -> Result<(), MediaError> {
    if !(1..=3840).contains(&o.width) || !(1..=2160).contains(&o.height) {
        return Err(MediaError::InvalidOptions(
            "dimensions must be within 1×1 and 3840×2160".into(),
        ));
    }
    if !(1..=60).contains(&o.fps) {
        return Err(MediaError::InvalidOptions(
            "fps must be between 1 and 60".into(),
        ));
    }
    Ok(())
}

fn start_audio_child(
    audio_program: &std::ffi::OsStr,
    media_path: &std::ffi::OsStr,
    looped: bool,
    video_child: &mut Child,
    stderr_readers: &mut Vec<JoinHandle<()>>,
    audio_stderr: Arc<Mutex<Vec<u8>>>,
) -> Result<Child, MediaError> {
    let mut cmd = Command::new(audio_program);
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostats",
        "-nodisp",
        "-autoexit",
    ]);
    if looped {
        cmd.args(["-loop", "0"]);
    }
    cmd.arg(media_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    match cmd.spawn() {
        Ok(mut child) => {
            if let Some(stderr) = child.stderr.take() {
                match drain_stderr(stderr, audio_stderr, "ffplay") {
                    Ok(reader) => stderr_readers.push(reader),
                    Err(e) => {
                        terminate_reap(&mut child);
                        terminate_reap(video_child);
                        return Err(e);
                    }
                }
            }
            Ok(child)
        }
        Err(e) => {
            terminate_reap(video_child);
            Err(if e.kind() == std::io::ErrorKind::NotFound {
                MediaError::ToolUnavailable(format!("{}: {e}", audio_program.to_string_lossy()))
            } else {
                MediaError::Process(format!(
                    "could not start {}: {e}",
                    audio_program.to_string_lossy()
                ))
            })
        }
    }
}

#[derive(Clone)]
struct DecoderContext {
    frame_len: usize,
    width: u32,
    height: u32,
    fps: u32,
    children: Arc<Mutex<Children>>,
    latest: Arc<Mutex<Option<Frame>>>,
    error: Arc<Mutex<Option<String>>>,
    finished: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    stderr: Arc<Mutex<Vec<u8>>>,
}

/// Starts the reader (ffmpeg pipe to frames) and the pacer (frames to `latest` on a steady clock).
fn start_decoder(
    stdout: impl Read + Send + 'static,
    context: DecoderContext,
) -> std::io::Result<Vec<JoinHandle<()>>> {
    let (tx, rx) = mpsc::sync_channel(READ_AHEAD);
    let pacer_context = context.clone();
    let pacer = thread::Builder::new()
        .name("lumen-media-pacer".into())
        .spawn(move || pace(rx, pacer_context))?;
    // If this spawn fails the closure and `tx` drop, which ends the pacer.
    match thread::Builder::new()
        .name("lumen-media-video".into())
        .spawn(move || read_frames(stdout, tx, context))
    {
        Ok(reader) => Ok(vec![reader, pacer]),
        Err(e) => {
            let _ = pacer.join();
            Err(e)
        }
    }
}

fn read_frames(mut stdout: impl Read, tx: mpsc::SyncSender<Vec<u8>>, context: DecoderContext) {
    let DecoderContext {
        frame_len,
        children,
        error,
        finished,
        paused,
        stderr,
        ..
    } = context;
    let mut failed = None;
    loop {
        let mut bytes = vec![0; frame_len];
        let mut filled = 0;
        while filled < bytes.len() {
            match stdout.read(&mut bytes[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    failed = Some(format!("reading ffmpeg frame: {e}"));
                    break;
                }
            }
        }
        if let Some(e) = failed.take() {
            if !finished.load(Ordering::Acquire) {
                set_error(&error, e);
            }
            break;
        }
        if filled == 0 {
            break;
        }
        if filled != frame_len {
            if !finished.load(Ordering::Acquire) {
                set_error(
                    &error,
                    format!("ffmpeg ended mid-frame ({filled} of {frame_len} bytes)"),
                );
            }
            break;
        }
        if tx.send(bytes).is_err() {
            break; // pacer is gone (stop)
        }
    }
    let status = wait_video_exit(&children, &finished);
    if let Err(e) = &status {
        set_error(
            &error,
            format!("could not inspect ffmpeg process after EOF: {e}"),
        );
    }
    if let Ok(Some(status)) = status
        && !status.success()
        && !finished.load(Ordering::Acquire)
    {
        let reason = bounded_text(&lock(&stderr));
        set_error(&error, format!("ffmpeg exited {status}: {reason}"));
    }
    if paused.swap(false, Ordering::AcqRel) {
        // The video child is reaped; resume the audio child so stop can terminate/reap it cleanly.
        let mut g = lock(&children);
        if let Some(child) = g.audio.as_mut() {
            let _ = signal_child(child, false);
        }
    }
    // Dropping `tx` lets the pacer show the queued frames, then mark playback finished.
}

fn frame_due(start: Instant, n: u64, fps: u32) -> Instant {
    start + Duration::from_secs_f64(n as f64 / f64::from(fps))
}

/// The clock start to use for frame `n` at `now`.
// shortcut: rebase the clock when >250 ms late (pause, stall); upgrade to PTS-driven timing if A/V drift is reported.
fn paced_start(start: Instant, n: u64, fps: u32, now: Instant) -> Instant {
    if now > frame_due(start, n, fps) + REBASE_AFTER {
        let elapsed = Duration::from_secs_f64(n as f64 / f64::from(fps));
        now.checked_sub(elapsed).unwrap_or(now)
    } else {
        start
    }
}

fn pace(rx: mpsc::Receiver<Vec<u8>>, context: DecoderContext) {
    let DecoderContext {
        width,
        height,
        fps,
        latest,
        finished,
        paused,
        ..
    } = context;
    let mut sequence = 0u64;
    let mut clock: Option<(Instant, u64)> = None; // (start, frames shown since start)
    'frames: while let Ok(rgba) = rx.recv() {
        loop {
            if finished.load(Ordering::Acquire) {
                break 'frames;
            }
            if paused.load(Ordering::Acquire) {
                clock = None; // resume re-anchors the clock on the next frame
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            let now = Instant::now();
            let (start, n) = clock.get_or_insert((now, 0));
            *start = paced_start(*start, *n, fps, now);
            let due = frame_due(*start, *n, fps);
            if now >= due {
                *n += 1;
                break;
            }
            thread::sleep((due - now).min(Duration::from_millis(20)));
        }
        sequence = sequence.wrapping_add(1);
        *lock(&latest) = Some(Frame {
            width,
            height,
            rgba,
            sequence,
        });
    }
    finished.store(true, Ordering::Release);
}

/// Reap a naturally exited decoder without holding lifecycle ownership across a wait.
/// While it is still running, the handle stays in `children`, so stop can kill it.
fn wait_video_exit(
    children: &Mutex<Children>,
    stopping: &AtomicBool,
) -> Result<Option<std::process::ExitStatus>, std::io::Error> {
    loop {
        if stopping.load(Ordering::Acquire) {
            return Ok(None);
        }
        let exited = {
            let mut g = lock(children);
            let Some(child) = g.video.as_mut() else {
                return Ok(None);
            };
            match child.try_wait() {
                Ok(Some(status)) => {
                    g.video = None;
                    Some(Ok(status))
                }
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            }
        };
        if let Some(result) = exited {
            return result.map(Some);
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn set_error(slot: &Mutex<Option<String>>, message: String) {
    *lock(slot) = Some(message);
}

#[cfg(unix)]
fn pause_one(
    child: &mut Option<Child>,
    paused: bool,
    signalled: &mut Vec<libc::pid_t>,
) -> Result<(), MediaError> {
    let Some(child) = child else {
        return Ok(());
    };
    match child.try_wait() {
        Ok(Some(_)) => return Ok(()),
        Err(e) => {
            return Err(MediaError::Process(format!(
                "could not inspect child before pause: {e}"
            )));
        }
        Ok(None) => {}
    }
    let pid = child.id() as libc::pid_t;
    let signal = if paused { libc::SIGSTOP } else { libc::SIGCONT };
    // An unreaped child's PID cannot be reused; holding the children lock prevents reaping here.
    if unsafe { libc::kill(pid, signal) } != 0 {
        let e = std::io::Error::last_os_error();
        for prior in signalled.iter().copied() {
            unsafe {
                libc::kill(prior, if paused { libc::SIGCONT } else { libc::SIGSTOP });
            }
        }
        return Err(MediaError::Process(format!(
            "could not {} child {pid}: {e}",
            if paused { "pause" } else { "resume" }
        )));
    }
    signalled.push(pid);
    Ok(())
}

fn drain_stderr(
    mut stderr: impl Read + Send + 'static,
    retained: Arc<Mutex<Vec<u8>>>,
    tool: &'static str,
) -> Result<JoinHandle<()>, MediaError> {
    thread::Builder::new()
        .name(format!("lumen-media-{tool}-stderr"))
        .spawn(move || {
            let mut chunk = [0u8; 2048];
            loop {
                match stderr.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut g = lock(&retained);
                        let keep = 8192usize.saturating_sub(g.len()).min(n);
                        g.extend_from_slice(&chunk[..keep]);
                    }
                }
            }
        })
        .map_err(|e| MediaError::Process(format!("could not start {tool} stderr drainer: {e}")))
}

fn spawn(cmd: &mut Command, tool: &str) -> Result<Child, MediaError> {
    cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            MediaError::ToolUnavailable(format!("{tool}: {e}"))
        } else {
            MediaError::Process(format!("could not start {tool}: {e}"))
        }
    })
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn terminate_children(children: &Mutex<Children>) {
    let mut owned = {
        let mut g = lock(children);
        let mut owned = Vec::with_capacity(2);
        take_and_kill(&mut g.video, &mut owned);
        take_and_kill(&mut g.audio, &mut owned);
        owned
    };
    // Cancellation retains exclusive ownership of the handles, but releases the
    // lifecycle mutex before reaping so the decoder and audio monitor can exit.
    for child in &mut owned {
        let _ = child.wait();
    }
}

fn take_and_kill(slot: &mut Option<Child>, owned: &mut Vec<Child>) {
    if let Some(mut child) = slot.take() {
        let _ = signal_child(&mut child, false);
        if !matches!(child.try_wait(), Ok(Some(_))) {
            let _ = child.kill();
        }
        owned.push(child);
    }
}

fn terminate_reap(child: &mut Child) {
    let _ = signal_child(child, false);
    match child.try_wait() {
        Ok(Some(_)) => {}
        _ => {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn signal_child(child: &mut Child, pause: bool) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        let sig = if pause { libc::SIGSTOP } else { libc::SIGCONT };
        if unsafe { libc::kill(child.id() as libc::pid_t, sig) } == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (child, pause);
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::process::Command;

    fn make_fixture_player(looped: bool) -> Option<(MediaPlayer, tempfile::TempDir)> {
        if !crate::tool_availability()
            .into_iter()
            .all(|(_, available)| available)
        {
            eprintln!("SKIP: required FFmpeg tools unavailable");
            return None;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("long clip.mp4");
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=96x54:rate=12",
                "-t",
                "8",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        let player = MediaPlayer::open(
            &path,
            PlaybackOptions {
                width: 64,
                height: 64,
                fps: 12,
                looped,
                muted: true,
            },
        )
        .unwrap();
        Some((player, dir))
    }

    fn fake_player(child: Child, stdout: impl Read + Send + 'static) -> MediaPlayer {
        let children = Arc::new(Mutex::new(Children {
            video: Some(child),
            audio: None,
        }));
        let latest = Arc::new(Mutex::new(None));
        let error = Arc::new(Mutex::new(None));
        let finished = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let context = DecoderContext {
            frame_len: 4,
            width: 1,
            height: 1,
            fps: 60,
            children: children.clone(),
            latest: latest.clone(),
            error: error.clone(),
            finished: finished.clone(),
            paused: paused.clone(),
            stderr: Arc::new(Mutex::new(Vec::new())),
        };
        let workers = start_decoder(stdout, context).unwrap();
        MediaPlayer {
            children,
            latest,
            error,
            finished,
            paused,
            workers,
            stderr_readers: Vec::new(),
            audio_monitor: None,
            cancel_audio_monitor: Arc::new(AtomicBool::new(false)),
            stopped: false,
        }
    }

    fn executable_script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
        path
    }

    fn shell_quote(path: &std::path::Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
    }

    fn fake_probe(dir: &std::path::Path) -> std::path::PathBuf {
        executable_script(
            dir,
            "fake-ffprobe",
            "echo '{\"streams\":[{\"codec_type\":\"video\",\"width\":1,\"height\":1},{\"codec_type\":\"audio\"}],\"format\":{\"duration\":\"60\"}}'",
        )
    }

    fn write_fake_input(dir: &std::path::Path) -> std::path::PathBuf {
        let input = dir.join("fake input.mov");
        std::fs::write(&input, b"fixture placeholder").unwrap();
        input
    }

    fn fake_options() -> PlaybackOptions {
        PlaybackOptions {
            width: 1,
            height: 1,
            fps: 1,
            looped: false,
            muted: false,
        }
    }

    #[test]
    fn fit_size_keeps_aspect_and_never_upscales() {
        assert_eq!(fit_size((1280, 720), (1920, 1080)), (1280, 720));
        assert_eq!(fit_size((3840, 2160), (1920, 1080)), (1920, 1080));
        assert_eq!(fit_size((96, 54), (64, 64)), (64, 36));
        assert_eq!(fit_size((1080, 1920), (1920, 1080)), (608, 1080));
    }

    #[test]
    fn pacer_clock_does_not_burst_after_a_stall() {
        let t0 = Instant::now();
        let fps = 30;
        // On time (and up to 250 ms late): the clock is untouched.
        assert_eq!(paced_start(t0, 10, fps, frame_due(t0, 10, fps)), t0);
        let late = frame_due(t0, 10, fps) + Duration::from_millis(250);
        assert_eq!(paced_start(t0, 10, fps, late), t0);
        // A 2 s stall rebases so frame 10 is due now and frame 11 one period later.
        let now = t0 + Duration::from_secs(2);
        let start = paced_start(t0, 10, fps, now);
        let skew = |a: Instant, b: Instant| a.max(b) - a.min(b);
        assert!(skew(frame_due(start, 10, fps), now) < Duration::from_millis(1));
        assert!(frame_due(start, 11, fps) > now + Duration::from_millis(30));
    }

    #[test]
    fn stop_reaps_the_live_ffmpeg_child() {
        let Some((mut player, _dir)) = make_fixture_player(true) else {
            return;
        };
        let pid = lock(&player.children).video.as_ref().unwrap().id() as libc::pid_t;
        player.stop();
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            -1,
            "ffmpeg PID {pid} is still alive after stop"
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }

    #[test]
    fn stop_can_kill_child_after_decoder_stdout_eof() {
        let mut child = Command::new("/bin/sh")
            .args(["-c", "exec 1>&-; exec sleep 30"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let player = fake_player(child, stdout);
        let pid = lock(&player.children).video.as_ref().unwrap().id() as libc::pid_t;
        thread::sleep(std::time::Duration::from_millis(100));
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let stop_thread = thread::spawn(move || {
            let mut player = player;
            player.stop();
            let _ = done_tx.send(());
        });
        let stopped = done_rx.recv_timeout(std::time::Duration::from_secs(1));
        if stopped.is_err() {
            // Keep the regression bounded even against the pre-fix deadlock.
            unsafe { libc::kill(pid, libc::SIGKILL) };
            let _ = done_rx.recv_timeout(std::time::Duration::from_secs(1));
        }
        stop_thread.join().unwrap();
        assert!(
            stopped.is_ok(),
            "stop blocked after decoder stdout reached EOF"
        );
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }

    #[test]
    fn stop_can_resume_and_kill_child_after_decoder_stdout_eof() {
        let mut child = Command::new("/bin/sh")
            .args(["-c", "exec 1>&-; exec sleep 30"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut player = fake_player(child, stdout);
        let pid = lock(&player.children).video.as_ref().unwrap().id() as libc::pid_t;
        thread::sleep(std::time::Duration::from_millis(100));
        player.set_paused(true).unwrap();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let stop_thread = thread::spawn(move || {
            player.stop();
            let _ = done_tx.send(());
        });
        let stopped = done_rx.recv_timeout(std::time::Duration::from_secs(1));
        if stopped.is_err() {
            unsafe { libc::kill(pid, libc::SIGKILL) };
            let _ = done_rx.recv_timeout(std::time::Duration::from_secs(1));
        }
        stop_thread.join().unwrap();
        assert!(
            stopped.is_ok(),
            "stop blocked for a paused child after decoder EOF"
        );
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }

    #[test]
    fn drop_reaps_active_decoder_child() {
        let Some((player, _dir)) = make_fixture_player(true) else {
            return;
        };
        let pid = lock(&player.children).video.as_ref().unwrap().id() as libc::pid_t;
        drop(player);
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }

    #[test]
    fn drop_reaps_paused_decoder_child() {
        let Some((mut player, _dir)) = make_fixture_player(true) else {
            return;
        };
        let pid = lock(&player.children).video.as_ref().unwrap().id() as libc::pid_t;
        player.set_paused(true).unwrap();
        drop(player);
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }

    #[test]
    fn stop_mid_frame_does_not_report_decode_error() {
        let mut child = Command::new("/bin/sh")
            .args(["-c", "printf '\\001\\002\\003\\004\\005'; exec sleep 30"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut player = fake_player(child, stdout);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while player.take_frame().is_none() {
            assert!(
                std::time::Instant::now() < deadline,
                "first fake frame timed out"
            );
            thread::sleep(std::time::Duration::from_millis(5));
        }
        player.stop();
        assert_eq!(player.error(), None);
    }

    #[test]
    fn unexpected_partial_frame_eof_reports_decoder_error() {
        let mut child = Command::new("/bin/sh")
            .args(["-c", "printf '\\001'"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut player = fake_player(child, stdout);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while !player.is_finished() {
            assert!(
                std::time::Instant::now() < deadline,
                "fake decoder did not finish"
            );
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(player.error().unwrap().contains("ended mid-frame"));
        player.stop();
    }

    #[test]
    fn failed_audio_launch_reaps_already_started_video_child() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_fake_input(dir.path());
        let video = executable_script(dir.path(), "fake-ffmpeg", "exec sleep 30");
        let mut video_child = Command::new(&video)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = video_child.id() as libc::pid_t;
        let missing_audio = dir.path().join("missing-ffplay");
        let mut readers = Vec::new();
        let result = start_audio_child(
            missing_audio.as_os_str(),
            input.as_os_str(),
            false,
            &mut video_child,
            &mut readers,
            Arc::new(Mutex::new(Vec::new())),
        );
        assert!(matches!(result, Err(MediaError::ToolUnavailable(_))));
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }

    #[test]
    fn audio_runtime_failure_is_reported_without_audio_hardware() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_fake_input(dir.path());
        let pid_file = dir.path().join("video.pid");
        let video = executable_script(
            dir.path(),
            "fake-ffmpeg",
            &format!(
                "printf '%s' \"$$\" > {}; printf '\\001\\002\\003\\004'; exec sleep 30",
                shell_quote(&pid_file)
            ),
        );
        let audio = executable_script(
            dir.path(),
            "fake-ffplay",
            "echo simulated-audio-backend-failure >&2; exit 9",
        );
        let probe = fake_probe(dir.path());
        let mut player = MediaPlayer::open_with_programs(
            &input,
            fake_options(),
            probe.to_str().unwrap(),
            video.as_os_str(),
            audio.as_os_str(),
        )
        .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some(error) = player.error() {
                assert!(
                    error.contains("ffplay exited"),
                    "unexpected runtime error: {error}"
                );
                assert!(error.contains("simulated-audio-backend-failure"));
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "audio failure was not reported"
            );
            thread::sleep(std::time::Duration::from_millis(10));
        }
        let pid: libc::pid_t = std::fs::read_to_string(pid_file).unwrap().parse().unwrap();
        player.stop();
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
}
