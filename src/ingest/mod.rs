//! Document ingestion from Claude Code data files.
//!
//! This module handles discovering and parsing conversation files from
//! the `~/.claude/projects` directory structure.

mod conversation;

pub use conversation::parse_jsonl_file;

use crate::config;
use crate::error::Result;
use crate::Document;
use std::path::{Path, PathBuf};
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

/// Ingests all conversation files and returns Documents.
///
/// Parse errors for individual files are logged to stderr but do not
/// cause the entire operation to fail.
///
/// # Errors
///
/// Returns an error if the projects directory cannot be determined.
pub fn ingest_all() -> Result<Vec<Document>> {
    let files = discover_conversation_files()?;
    let mut all_docs = Vec::with_capacity(files.len() * 10); // Estimate ~10 docs per file

    for file_path in files {
        match parse_jsonl_file(&file_path) {
            Ok(docs) => {
                all_docs.extend(docs);
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", file_path.display(), e);
            }
        }
    }

    Ok(all_docs)
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
