//! Document types for indexed content.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::{Path, PathBuf};

/// The type of document being indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocType {
    /// A conversation message (user or assistant).
    Conversation,
    // Future: Todo, Plan, History, Debug
}

impl fmt::Display for DocType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocType::Conversation => write!(f, "conversation"),
        }
    }
}

impl DocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocType::Conversation => "conversation",
        }
    }
}

/// A document to be indexed and searched.
///
/// Documents are created from parsed conversation files and contain
/// both the searchable content and metadata for filtering/display.
#[derive(Debug, Clone)]
pub struct Document {
    /// Unique identifier derived from source path and content hash.
    pub id: String,
    /// The type of document (conversation, todo, etc.).
    pub doc_type: DocType,
    /// The project path this document belongs to.
    pub project: Option<String>,
    /// When this message was created.
    pub timestamp: Option<DateTime<Utc>>,
    /// The Claude Code session ID.
    pub session_id: Option<String>,
    /// The message role ("user" or "assistant").
    pub role: Option<String>,
    /// The searchable text content.
    pub content: String,
    /// Path to the source file this document was extracted from.
    pub source_path: PathBuf,
}

impl Document {
    /// Creates a new Document with an auto-generated ID
    pub fn new(doc_type: DocType, content: String, source_path: PathBuf) -> Self {
        let id = generate_id(&source_path, &content);
        Self {
            id,
            doc_type,
            project: None,
            timestamp: None,
            session_id: None,
            role: None,
            content,
            source_path,
        }
    }

    pub fn with_project(mut self, project: Option<String>) -> Self {
        self.project = project;
        self
    }

    pub fn with_timestamp(mut self, timestamp: Option<DateTime<Utc>>) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub fn with_session_id(mut self, session_id: Option<String>) -> Self {
        self.session_id = session_id;
        self
    }

    pub fn with_role(mut self, role: Option<String>) -> Self {
        self.role = role;
        self
    }

    /// Returns a snippet of the content, truncated to max_len characters
    pub fn snippet(&self, max_len: usize) -> &str {
        if self.content.len() <= max_len {
            &self.content
        } else {
            // Find a good break point (space or newline)
            let truncated = &self.content[..max_len];
            if let Some(last_space) = truncated.rfind([' ', '\n']) {
                &self.content[..last_space]
            } else {
                truncated
            }
        }
    }
}

/// Generates a deterministic ID from the source path and content
fn generate_id(source_path: &Path, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_path.to_string_lossy().as_bytes());
    // Use first ~100 chars of content to avoid hashing huge strings
    // Find a safe UTF-8 boundary near 100 chars
    let content_prefix = if content.chars().count() > 100 {
        content.chars().take(100).collect::<String>()
    } else {
        content.to_string()
    };
    hasher.update(content_prefix.as_bytes());
    let result = hasher.finalize();
    // Take first 16 hex chars (8 bytes)
    hex::encode(&result[..8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id_deterministic() {
        let path = PathBuf::from("/test/path.jsonl");
        let content = "test content";
        let id1 = generate_id(&path, content);
        let id2 = generate_id(&path, content);
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 16);
    }

    #[test]
    fn test_snippet_short() {
        let doc = Document::new(
            DocType::Conversation,
            "short".to_string(),
            PathBuf::from("/test"),
        );
        assert_eq!(doc.snippet(100), "short");
    }

    #[test]
    fn test_snippet_truncates() {
        let doc = Document::new(
            DocType::Conversation,
            "hello world this is a test".to_string(),
            PathBuf::from("/test"),
        );
        // Truncates at word boundary before max_len
        assert_eq!(doc.snippet(15), "hello world");
    }
}
