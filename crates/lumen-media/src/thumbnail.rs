use crate::{Frame, MediaError, probe, validate_file};
use std::{
    collections::VecDeque,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant, SystemTime},
};

const MAX_DIMENSION: u32 = 1024;
const THUMBNAIL_TIMEOUT: Duration = Duration::from_secs(10);

pub fn thumbnail(path: &Path, max_w: u32, max_h: u32) -> Result<Frame, MediaError> {
    if max_w == 0 || max_h == 0 || max_w > MAX_DIMENSION || max_h > MAX_DIMENSION {
        return Err(MediaError::InvalidOptions(
            "thumbnail dimensions must be within 1×1 and 1024×1024".into(),
        ));
    }
    validate_file(path)?;
    let info = probe(path)?;
    let scale = (max_w as f64 / info.width as f64)
        .min(max_h as f64 / info.height as f64)
        .min(1.0);
    let width = ((info.width as f64 * scale).round() as u32).max(1);
    let height = ((info.height as f64 * scale).round() as u32).max(1);
    let frame_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| {
            MediaError::InvalidOptions("thumbnail buffer size overflows address space".into())
        })?;
    let seek = info
        .duration_secs
        .filter(|d| *d > 0.0)
        .map(|d| d.min(1.0).min(d * 0.1));
    let mut command = Command::new("ffmpeg");
    command.args(["-hide_banner", "-loglevel", "error", "-nostdin"]);
    if let Some(at) = seek {
        command.arg("-ss").arg(at.to_string());
    }
    command
        .args(["-i"])
        .arg(path)
        .args(["-frames:v", "1", "-vf"])
        .arg(format!("scale={width}:{height},format=rgba"))
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            MediaError::ToolUnavailable(format!("ffmpeg: {e}"))
        } else {
            MediaError::Process(format!("could not start ffmpeg: {e}"))
        }
    })?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| MediaError::Process("ffmpeg stdout unavailable".into()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| MediaError::Process("ffmpeg stderr unavailable".into()))?;
    let out_reader = thread::spawn(move || {
        let mut bytes = Vec::with_capacity(frame_len);
        let mut chunk = [0u8; 8192];
        loop {
            match stdout.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let retain = frame_len.saturating_sub(bytes.len()).min(n);
                    bytes.extend_from_slice(&chunk[..retain]);
                }
            }
        }
        bytes
    });
    let err_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 2048];
        loop {
            match stderr.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let retain = 8192usize.saturating_sub(bytes.len()).min(n);
                    bytes.extend_from_slice(&chunk[..retain]);
                }
            }
        }
        bytes
    });
    let deadline = Instant::now() + THUMBNAIL_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out_reader.join();
                let _ = err_reader.join();
                return Err(MediaError::Timeout(
                    "ffmpeg thumbnail decode exceeded 10 seconds".into(),
                ));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out_reader.join();
                let _ = err_reader.join();
                return Err(MediaError::Process(format!(
                    "could not inspect ffmpeg: {e}"
                )));
            }
        }
    };
    let rgba = out_reader
        .join()
        .map_err(|_| MediaError::Process("ffmpeg output reader panicked".into()))?;
    let stderr = err_reader
        .join()
        .map_err(|_| MediaError::Process("ffmpeg error reader panicked".into()))?;
    if !status.success() {
        return Err(MediaError::InvalidMedia(format!(
            "ffmpeg exited {status}: {}",
            crate::probe::bounded_text(&stderr)
        )));
    }
    if rgba.len() != frame_len {
        return Err(MediaError::InvalidMedia(format!(
            "ffmpeg returned {} bytes; expected {frame_len}",
            rgba.len()
        )));
    }
    Ok(Frame {
        width,
        height,
        rgba,
        sequence: 0,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CacheKey {
    path: PathBuf,
    modified: Option<SystemTime>,
    size: u64,
    max_w: u32,
    max_h: u32,
}

struct Entry {
    key: CacheKey,
    frame: Arc<Frame>,
}

pub struct ThumbnailCache {
    capacity: usize,
    entries: VecDeque<Entry>,
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new(256)
    }
}

impl ThumbnailCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::new(),
        }
    }

    pub fn get_or_load(
        &mut self,
        path: &Path,
        max_w: u32,
        max_h: u32,
    ) -> Result<Arc<Frame>, MediaError> {
        let canonical = path
            .canonicalize()
            .map_err(|e| MediaError::InvalidPath(format!("{}: {e}", path.display())))?;
        let metadata = std::fs::metadata(&canonical)?;
        if !metadata.is_file() {
            return Err(MediaError::InvalidPath(format!(
                "{} is not a regular file",
                path.display()
            )));
        }
        let key = CacheKey {
            path: canonical.clone(),
            modified: metadata.modified().ok(),
            size: metadata.len(),
            max_w,
            max_h,
        };
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            let entry = self.entries.remove(index).unwrap();
            let frame = Arc::clone(&entry.frame);
            self.entries.push_front(entry);
            return Ok(frame);
        }
        let frame = Arc::new(thumbnail(&canonical, max_w, max_h)?);
        if self.capacity > 0 {
            self.entries.push_front(Entry {
                key,
                frame: Arc::clone(&frame),
            });
            self.entries.truncate(self.capacity);
        }
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MediaError;
    use std::{process::Command, time::Duration};

    fn fixture(extension: &str, source: &str) -> Option<(tempfile::TempDir, std::path::PathBuf)> {
        if !crate::tool_availability()
            .into_iter()
            .find(|(name, _)| name == "ffmpeg")
            .is_some_and(|(_, available)| available)
        {
            eprintln!("SKIP: ffmpeg unavailable; cannot generate thumbnail fixture");
            return None;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("fixture.{extension}"));
        let output = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                source,
            ])
            .args(["-frames:v", "1"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Some((dir, path))
    }

    fn assert_fit(frame: &Frame) {
        assert_eq!((frame.width, frame.height), (64, 36));
        assert_eq!(frame.rgba.len(), (frame.width * frame.height * 4) as usize);
        let (pixels, _) = frame.rgba.as_chunks::<4>();
        assert!(pixels.iter().any(|p| p[0] != 0));
        assert!(pixels.iter().any(|p| p[1] != 0));
    }

    #[test]
    fn thumbnails_png_and_video_preserve_aspect_and_pixels() {
        let Some((_png_dir, png)) = fixture("png", "testsrc=size=128x72:rate=1") else {
            return;
        };
        assert_fit(&thumbnail(&png, 64, 64).unwrap());

        if !crate::tool_availability()
            .into_iter()
            .find(|(name, _)| name == "ffmpeg")
            .is_some_and(|(_, ok)| ok)
        {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("fixture.mp4");
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=128x72:rate=10",
                "-t",
                "1.2",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&video)
            .status()
            .unwrap();
        assert!(status.success());
        assert_fit(&thumbnail(&video, 64, 64).unwrap());
    }

    #[test]
    fn invalid_paths_and_oversize_dimensions_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        assert!(thumbnail(&dir.path().join("missing"), 64, 64).is_err());
        assert!(thumbnail(dir.path(), 64, 64).is_err());
        let garbage = dir.path().join("garbage.png");
        std::fs::write(&garbage, b"not an image").unwrap();
        assert!(thumbnail(&garbage, 64, 64).is_err());
        assert!(matches!(
            thumbnail(&garbage, 1025, 10),
            Err(MediaError::InvalidOptions(_))
        ));
    }

    #[test]
    fn cache_hits_then_invalidates_on_mtime_change() {
        let Some((dir, path)) = fixture("png", "testsrc=size=128x72:rate=1") else {
            return;
        };
        let mut cache = ThumbnailCache::default();
        let a = cache.get_or_load(&path, 64, 64).unwrap();
        let b = cache.get_or_load(&path, 64, 64).unwrap();
        assert!(std::sync::Arc::ptr_eq(&a, &b));
        std::thread::sleep(Duration::from_millis(20));
        let bytes = std::fs::read(&path).unwrap();
        std::fs::write(&path, bytes).unwrap();
        let c = cache.get_or_load(&path, 64, 64).unwrap();
        assert!(!std::sync::Arc::ptr_eq(&a, &c));
        assert_eq!(a.rgba, c.rgba);
        drop(dir);
    }
}
