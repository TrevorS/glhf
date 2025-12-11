use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test environment with temporary directories for claude data and index
#[allow(dead_code, clippy::struct_field_names)]
pub struct TestEnv {
    pub temp_dir: TempDir, // Must keep alive for cleanup
    pub claude_dir: PathBuf,
    pub projects_dir: PathBuf,
    pub index_dir: PathBuf,
}

impl TestEnv {
    /// Creates a new test environment with temporary directories
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let claude_dir = temp_dir.path().join(".claude");
        let projects_dir = claude_dir.join("projects");
        let index_dir = temp_dir.path().join("index");

        fs::create_dir_all(&projects_dir).expect("Failed to create projects dir");
        fs::create_dir_all(&index_dir).expect("Failed to create index dir");

        Self {
            temp_dir,
            claude_dir,
            projects_dir,
            index_dir,
        }
    }

    /// Creates a project directory with an encoded name
    pub fn create_project(&self, name: &str) -> PathBuf {
        // Encode path like Claude does: /Users/foo/project -> -Users-foo-project
        let encoded = format!("-{}", name.replace('/', "-"));
        let project_dir = self.projects_dir.join(&encoded);
        fs::create_dir_all(&project_dir).expect("Failed to create project dir");
        project_dir
    }

    /// Writes a JSONL file with the given content lines
    #[allow(clippy::unused_self)]
    pub fn write_jsonl(&self, project_dir: &Path, filename: &str, lines: &[&str]) -> PathBuf {
        let file_path = project_dir.join(filename);
        let mut file = File::create(&file_path).expect("Failed to create JSONL file");
        for line in lines {
            writeln!(file, "{line}").expect("Failed to write line");
        }
        file_path
    }
}

/// Generates a user message JSON line
pub fn user_message(content: &str, session_id: &str) -> String {
    format!(
        r#"{{"type":"user","timestamp":"2025-01-15T10:00:00Z","sessionId":"{session_id}","message":{{"role":"user","content":"{content}"}}}}"#
    )
}

/// Generates an assistant message JSON line
pub fn assistant_message(content: &str, session_id: &str) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"2025-01-15T10:00:01Z","sessionId":"{session_id}","message":{{"role":"assistant","content":"{content}"}}}}"#
    )
}

/// Generates an assistant message with array content (tool results)
pub fn assistant_with_blocks(texts: &[&str], session_id: &str) -> String {
    let blocks: Vec<String> = texts
        .iter()
        .map(|t| format!(r#"{{"type":"text","text":"{t}"}}"#))
        .collect();
    format!(
        r#"{{"type":"assistant","timestamp":"2025-01-15T10:00:02Z","sessionId":"{session_id}","message":{{"role":"assistant","content":[{}]}}}}"#,
        blocks.join(",")
    )
}

/// Generates a non-message type that should be skipped
pub fn file_history_snapshot() -> String {
    r#"{"type":"file-history-snapshot","messageId":"abc123","snapshot":{}}"#.to_string()
}

/// Generates malformed JSON
pub fn malformed_json() -> String {
    r#"{"type":"user", this is not valid json"#.to_string()
}
