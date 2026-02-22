use thiserror::Error;

#[derive(Debug, Error)]
pub enum IxError {
    #[error("No index found. Run `ix` first to build an index.")]
    NoIndex,

    #[error("Slot {0} not found in index (index has {1} items)")]
    SlotOutOfRange(usize, usize),

    #[error("Index is stale (older than {0} seconds)")]
    StaleIndex(u64),

    #[error("Failed to parse selector: {0}")]
    ParseError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, IxError>;
