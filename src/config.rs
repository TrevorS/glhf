use std::path::PathBuf;

/// Returns the Claude Code data directory (~/.claude)
pub fn claude_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".claude")
}

/// Returns the glhf cache/index directory (~/.cache/glhf)
pub fn index_dir() -> PathBuf {
    dirs::cache_dir()
        .expect("Could not find cache directory")
        .join("glhf")
}

/// Returns the BM25 index directory
pub fn bm25_index_dir() -> PathBuf {
    index_dir().join("bm25")
}

/// Returns the projects directory containing conversation JSONL files
pub fn projects_dir() -> PathBuf {
    claude_dir().join("projects")
}

/// Decodes an encoded project path from Claude's directory structure
/// e.g., "-Users-trevor-Projects-foo" -> "/Users/trevor/Projects/foo"
pub fn decode_project_path(encoded: &str) -> String {
    if encoded.starts_with('-') {
        encoded.replacen('-', "/", 1).replace('-', "/")
    } else {
        encoded.replace('-', "/")
    }
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
