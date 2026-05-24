//! Error types for the glhf library.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not find {dir_type} directory")]
    MissingDirectory { dir_type: &'static str },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("embedding error: {message}")]
    Embedding { message: String },

    #[error("no database found at {path}; run 'glhf index' first")]
    DatabaseNotFound { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, Error>;
