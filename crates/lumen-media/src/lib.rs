//! Native FFmpeg-backed media probing and bounded playback for Lumen Presenter.

use std::path::Path;

mod player;
mod probe;

pub use player::{Frame, MediaPlayer, PlaybackOptions};
pub use probe::{MediaInfo, probe, tool_availability};

/// Errors produced while validating, probing, or decoding media.
#[derive(Debug)]
pub enum MediaError {
    InvalidPath(String),
    InvalidOptions(String),
    InvalidMedia(String),
    ToolUnavailable(String),
    Process(String),
    Timeout(String),
    Unsupported(String),
    Io(std::io::Error),
}

impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath(s) => write!(f, "invalid media path: {s}"),
            Self::InvalidOptions(s) => write!(f, "invalid playback options: {s}"),
            Self::InvalidMedia(s) => write!(f, "invalid media: {s}"),
            Self::ToolUnavailable(s) => write!(f, "media tool unavailable: {s}"),
            Self::Process(s) => write!(f, "media process failed: {s}"),
            Self::Timeout(s) => write!(f, "media operation timed out: {s}"),
            Self::Unsupported(s) => write!(f, "unsupported operation: {s}"),
            Self::Io(e) => write!(f, "media I/O error: {e}"),
        }
    }
}

impl std::error::Error for MediaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for MediaError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Validates that a path names a regular file, returning its OS-native form unchanged.
pub(crate) fn validate_file(path: &Path) -> Result<(), MediaError> {
    let meta = std::fs::metadata(path)
        .map_err(|e| MediaError::InvalidPath(format!("{}: {e}", path.display())))?;
    if !meta.is_file() {
        return Err(MediaError::InvalidPath(format!(
            "{} is not a regular file",
            path.display()
        )));
    }
    Ok(())
}
