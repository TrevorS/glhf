//! Configuration and path utilities.
//!
//! This module provides paths to Claude Code data directories and the glhf
//! index location.

use crate::error::{Error, Result};
use std::path::PathBuf;

/// Returns the Claude Code data directory (`~/.claude`).
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined.
pub fn claude_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|p| p.join(".claude"))
        .ok_or(Error::MissingDirectory { dir_type: "home" })
}

/// Returns the glhf cache/index directory (`~/.cache/glhf`).
///
/// # Errors
///
/// Returns an error if the cache directory cannot be determined.
pub fn index_dir() -> Result<PathBuf> {
    dirs::cache_dir()
        .map(|p| p.join("glhf"))
        .ok_or(Error::MissingDirectory { dir_type: "cache" })
}

/// Returns the database file path (`~/.cache/glhf/glhf.db`).
///
/// # Errors
///
/// Returns an error if the cache directory cannot be determined.
pub fn database_path() -> Result<PathBuf> {
    index_dir().map(|p| p.join("glhf.db"))
}

/// Returns the projects directory containing conversation JSONL files (`~/.claude/projects`).
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined.
pub fn projects_dir() -> Result<PathBuf> {
    claude_dir().map(|p| p.join("projects"))
}
