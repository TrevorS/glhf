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

/// Returns the BM25 index directory (`~/.cache/glhf/bm25`).
///
/// # Errors
///
/// Returns an error if the cache directory cannot be determined.
pub fn bm25_index_dir() -> Result<PathBuf> {
    index_dir().map(|p| p.join("bm25"))
}

/// Returns the projects directory containing conversation JSONL files (`~/.claude/projects`).
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined.
pub fn projects_dir() -> Result<PathBuf> {
    claude_dir().map(|p| p.join("projects"))
}

/// Decodes an encoded project path from Claude's directory structure.
///
/// Claude encodes paths by replacing directory separators:
/// - Single dash `-` represents `/`
/// - Double dash `--` represents `/.` (hidden directory prefix)
///
/// # Examples
///
/// ```
/// use glhf::config::decode_project_path;
///
/// assert_eq!(
///     decode_project_path("-Users-trevor-Projects-foo"),
///     "/Users/trevor/Projects/foo"
/// );
/// assert_eq!(
///     decode_project_path("-Users-trevor--claude"),
///     "/Users/trevor/.claude"
/// );
/// ```
pub fn decode_project_path(encoded: &str) -> String {
    let mut result = String::with_capacity(encoded.len());
    let mut chars = encoded.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '-' {
            if chars.peek() == Some(&'-') {
                // Double dash -> /. (hidden dir)
                chars.next();
                result.push_str("/.");
            } else {
                // Single dash -> slash
                result.push('/');
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_project_path() {
        assert_eq!(
            decode_project_path("-Users-trevor-Projects-foo"),
            "/Users/trevor/Projects/foo"
        );
        assert_eq!(
            decode_project_path("-Users-trevor--claude"),
            "/Users/trevor/.claude"
        );
    }

    #[test]
    fn test_decode_project_path_empty() {
        assert_eq!(decode_project_path(""), "");
    }

    #[test]
    fn test_decode_project_path_no_dashes() {
        assert_eq!(decode_project_path("simple"), "simple");
    }
}
