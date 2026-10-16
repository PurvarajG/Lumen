#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("Layout error in book '{book}', unit '{unit}': {detail}")]
    Layout {
        book: String,
        unit: String,
        detail: String,
    },
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Operation cancelled")]
    Cancelled,
    #[error("Warning: {0}")]
    Warning(String),
}
