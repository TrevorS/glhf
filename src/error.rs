//! Error types for the glhf library.
//!
//! This module provides structured error types for all operations in the library,
//! using the [`thiserror`] crate for ergonomic error definitions.

use std::path::PathBuf;
use thiserror::Error;

/// The main error type for the glhf library.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Failed to find a required system directory (e.g., home or cache directory).
    #[error("could not find {dir_type} directory")]
    MissingDirectory {
        /// The type of directory that was missing (e.g., "home", "cache").
        dir_type: &'static str,
    },

    /// An I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse JSON data.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// An index operation failed.
    #[error("index error: {message}")]
    Index {
        /// A description of what went wrong.
        message: String,
        /// The underlying Tantivy error, if available.
        #[source]
        source: Option<tantivy::TantivyError>,
    },

    /// Failed to parse a search query.
    #[error("invalid search query: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),

    /// Failed to parse a file.
    #[error("failed to parse {path}: {message}")]
    Parse {
        /// The path to the file that failed to parse.
        path: PathBuf,
        /// A description of the parse error.
        message: String,
    },

    /// The search index was not found.
    #[error("no index found at {path}; run 'glhf index' first")]
    IndexNotFound {
        /// The path where the index was expected.
        path: PathBuf,
    },

    /// Invalid regular expression pattern.
    #[error("invalid regex pattern: {0}")]
    Regex(#[from] regex::Error),
}

impl Error {
    /// Creates a new index error with the given message.
    pub fn index(message: impl Into<String>) -> Self {
        Self::Index {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new index error from a Tantivy error.
    pub fn from_tantivy(err: tantivy::TantivyError, message: impl Into<String>) -> Self {
        Self::Index {
            message: message.into(),
            source: Some(err),
        }
    }

    /// Creates a new parse error for a specific file.
    pub fn parse(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Parse {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// A specialized Result type for glhf operations.
pub type Result<T> = std::result::Result<T, Error>;
