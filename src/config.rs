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
    use proptest::prelude::*;

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

    proptest! {
        #[test]
        fn proptest_decode_never_panics(input in ".*") {
            let _ = decode_project_path(&input);
        }

        #[test]
        fn proptest_decode_leading_dash_becomes_slash(s in "[a-zA-Z0-9]{1,20}") {
            let encoded = format!("-{s}");
            let decoded = decode_project_path(&encoded);
            prop_assert!(decoded.starts_with('/'));
        }

        #[test]
        fn proptest_decode_no_dashes_unchanged(s in "[a-zA-Z0-9_.]{0,50}") {
            // Input without dashes should pass through unchanged
            let decoded = decode_project_path(&s);
            prop_assert_eq!(decoded, s);
        }

        #[test]
        fn proptest_decode_output_never_contains_single_dash(s in "-[a-zA-Z]{1,5}(-[a-zA-Z]{1,5}){0,5}") {
            // All single dashes become '/', so output should have no dashes
            // (only if input has no double-dashes and no literal dashes in segments)
            let decoded = decode_project_path(&s);
            // Output should have slashes where dashes were
            prop_assert!(!decoded.contains('-'));
        }
    }
}
