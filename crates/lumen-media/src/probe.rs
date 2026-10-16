use crate::{MediaError, validate_file};
use serde_json::Value;
use std::{ffi::OsString, path::Path, process::Command};

#[derive(Clone, Debug, PartialEq)]
pub struct MediaInfo {
    pub width: u32,
    pub height: u32,
    pub duration_secs: Option<f64>,
    pub has_audio: bool,
}

pub fn probe(path: &Path) -> Result<MediaInfo, MediaError> {
    probe_with_program(path, "ffprobe")
}

pub(crate) fn probe_with_program(path: &Path, program: &str) -> Result<MediaInfo, MediaError> {
    validate_file(path)?;
    let mut args: Vec<OsString> = [
        "-v",
        "error",
        "-show_entries",
        "format=duration:stream=codec_type,width,height",
        "-of",
        "json",
        "--",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    args.push(path.as_os_str().to_os_string());
    let output = run_bounded(program, &args, 10_000)?;
    if !output.status.success() {
        return Err(MediaError::Process(format!(
            "{} exited {}: {}",
            program,
            output.status,
            bounded_text(&output.stderr)
        )));
    }
    let parsed: Value = serde_json::from_slice(&output.stdout).map_err(|e| {
        MediaError::InvalidMedia(format!("{}: invalid {} JSON: {e}", path.display(), program))
    })?;
    let streams = parsed
        .get("streams")
        .and_then(Value::as_array)
        .ok_or_else(|| MediaError::InvalidMedia(format!("{program} returned no streams array")))?;
    let video = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(Value::as_str) == Some("video"))
        .ok_or_else(|| MediaError::InvalidMedia("media has no video stream".into()))?;
    let width = video
        .get("width")
        .and_then(Value::as_u64)
        .filter(|v| (1..=16384).contains(v))
        .ok_or_else(|| MediaError::InvalidMedia("video width is missing or out of range".into()))?
        as u32;
    let height = video
        .get("height")
        .and_then(Value::as_u64)
        .filter(|v| (1..=16384).contains(v))
        .ok_or_else(|| MediaError::InvalidMedia("video height is missing or out of range".into()))?
        as u32;
    let duration_secs = match parsed.get("format").and_then(|f| f.get("duration")) {
        None | Some(Value::Null) => None,
        Some(v) => Some(
            number_or_string(v)
                .filter(|n| n.is_finite() && *n >= 0.0 && *n <= 31_536_000.0)
                .ok_or_else(|| {
                    MediaError::InvalidMedia(
                        "duration is malformed or outside the supported range".into(),
                    )
                })?,
        ),
    };
    Ok(MediaInfo {
        width,
        height,
        duration_secs,
        has_audio: streams
            .iter()
            .any(|s| s.get("codec_type").and_then(Value::as_str) == Some("audio")),
    })
}

pub fn tool_availability() -> Vec<(String, bool)> {
    ["ffmpeg", "ffprobe", "ffplay"]
        .iter()
        .map(|tool| {
            let available = Command::new(tool)
                .arg("-version")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|s| s.success());
            ((*tool).to_string(), available)
        })
        .collect()
}

fn number_or_string(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

pub(crate) struct Captured {
    pub status: std::process::ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Captures a bounded prefix while draining pipes, and terminates the process on timeout.
pub(crate) fn run_bounded(
    program: &str,
    args: &[OsString],
    timeout_ms: u64,
) -> Result<Captured, MediaError> {
    use std::{
        io::Read,
        process::Stdio,
        thread,
        time::{Duration, Instant},
    };
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                MediaError::ToolUnavailable(format!("{program}: {e}"))
            } else {
                MediaError::Process(format!("could not start {program}: {e}"))
            }
        })?;
    let mut out = child
        .stdout
        .take()
        .ok_or_else(|| MediaError::Process(format!("{program} stdout unavailable")))?;
    let mut err = child
        .stderr
        .take()
        .ok_or_else(|| MediaError::Process(format!("{program} stderr unavailable")))?;
    let out_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            match out.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let keep = 1_048_576usize.saturating_sub(buf.len()).min(n);
                    buf.extend_from_slice(&chunk[..keep]);
                }
            }
        }
        buf
    });
    let err_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match err.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let keep = 4096usize.saturating_sub(buf.len()).min(n);
                    buf.extend_from_slice(&chunk[..keep]);
                }
            }
        }
        buf
    });
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out_reader.join();
            let _ = err_reader.join();
            return Err(MediaError::Timeout(format!(
                "{program} exceeded {timeout_ms} ms"
            )));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = out_reader
        .join()
        .map_err(|_| MediaError::Process(format!("{program} stdout reader panicked")))?;
    let stderr = err_reader
        .join()
        .map_err(|_| MediaError::Process(format!("{program} stderr reader panicked")))?;
    Ok(Captured {
        status,
        stdout,
        stderr,
    })
}

pub(crate) fn bounded_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut s: String = text.chars().take(2048).collect();
    if text.chars().count() > 2048 {
        s.push('…');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn probe_process_timeout_kills_and_reaps_command() {
        let args = ["-c", "exec sleep 30"]
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
        let started = std::time::Instant::now();
        let result = run_bounded("/bin/sh", &args, 100);
        assert!(matches!(result, Err(MediaError::Timeout(_))));
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }

    #[cfg(unix)]
    #[test]
    fn probe_process_caps_retained_stdout_while_draining_pipe() {
        let args = ["-c", "head -c 2000000 /dev/zero"]
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
        let output = run_bounded("/bin/sh", &args, 3_000).unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 1_048_576);
    }
}
