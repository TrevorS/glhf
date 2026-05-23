//! Document ingestion from Claude Code data files.
//!
//! This module handles discovering and parsing conversation files from
//! the `~/.claude/projects` directory structure.

mod conversation;

pub use conversation::parse_jsonl_file;

use crate::config;
use crate::error::Result;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Discovers all conversation JSONL files in `~/.claude/projects`.
///
/// # Errors
///
/// Returns an error if the projects directory cannot be determined.
pub fn discover_conversation_files() -> Result<Vec<PathBuf>> {
    let projects_dir = config::projects_dir()?;

    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let files = WalkDir::new(&projects_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| {
            entry.path().is_file() && entry.path().extension().is_some_and(|ext| ext == "jsonl")
        })
        .map(walkdir::DirEntry::into_path)
        .collect();

    Ok(files)
}

/// Discovers conversation files with their modification times.
///
/// Returns `(path, mtime_secs)` pairs where `mtime_secs` is seconds since Unix epoch.
///
/// # Errors
///
/// Returns an error if the projects directory cannot be determined.
pub fn discover_with_mtimes() -> Result<Vec<(PathBuf, i64)>> {
    let files = discover_conversation_files()?;
    let mut result = Vec::with_capacity(files.len());
    for path in files {
        #[allow(clippy::cast_possible_wrap)]
        let mtime = path
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        result.push((path, mtime));
    }
    Ok(result)
}

/// Extracts the project name from a JSONL file path.
///
/// Returns the raw encoded directory name (e.g., `-Users-trevor-Projects-foo`).
/// We intentionally do NOT decode the path because Claude's encoding is lossy:
/// hyphens in original path names become indistinguishable from path separators.
///
/// Returns `None` if the path is not under the projects directory or
/// if the projects directory cannot be determined.
pub fn extract_project_from_path(path: &Path) -> Option<String> {
    // Path format: ~/.claude/projects/-Users-trevor-Projects-foo/session.jsonl
    let projects_dir = config::projects_dir().ok()?;

    path.strip_prefix(&projects_dir)
        .ok()
        .and_then(|rel| rel.components().next())
        .map(|comp| comp.as_os_str().to_string_lossy().to_string())
}
