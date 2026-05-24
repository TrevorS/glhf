//! Configuration and path utilities.

use crate::error::{Error, Result};
use std::path::PathBuf;

fn claude_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|p| p.join(".claude"))
        .ok_or(Error::MissingDirectory { dir_type: "home" })
}

fn index_dir() -> Result<PathBuf> {
    dirs::cache_dir()
        .map(|p| p.join("glhf"))
        .ok_or(Error::MissingDirectory { dir_type: "cache" })
}

pub fn database_path() -> Result<PathBuf> {
    index_dir().map(|p| p.join("glhf.db"))
}

pub fn projects_dir() -> Result<PathBuf> {
    claude_dir().map(|p| p.join("projects"))
}
