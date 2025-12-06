//! Configuration and path utilities.
//!
//! This module provides paths to Claude Code data directories and the glhf
//! index location.

use std::path::PathBuf;

/// Returns the Claude Code data directory (`~/.claude`).
pub fn claude_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".claude")
}

/// Returns the glhf cache/index directory (`~/.cache/glhf`).
pub fn index_dir() -> PathBuf {
    dirs::cache_dir()
        .expect("Could not find cache directory")
        .join("glhf")
}

/// Returns the BM25 index directory (`~/.cache/glhf/bm25`).
pub fn bm25_index_dir() -> PathBuf {
    index_dir().join("bm25")
}

/// Returns the projects directory containing conversation JSONL files (`~/.claude/projects`).
pub fn projects_dir() -> PathBuf {
    claude_dir().join("projects")
}

/// Decodes an encoded project path from Claude's directory structure
/// e.g., "-Users-trevor-Projects-foo" -> "/Users/trevor/Projects/foo"
/// Double dashes represent /. : "-Users-trevor--claude" -> "/Users/trevor/.claude"
pub fn decode_project_path(encoded: &str) -> String {
    let mut result = String::new();
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
}
