//! Document types for indexed content.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::{Path, PathBuf};

/// The type of document being indexed.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new document types
/// in the future without breaking semver compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum DocType {
    /// A conversation message (user or assistant).
    #[default]
    Conversation,
    // Future: Todo, Plan, History, Debug
}

impl fmt::Display for DocType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl DocType {
    /// Returns the string representation of the document type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            DocType::Conversation => "conversation",
        }
    }
}

/// A document to be indexed and searched.
///
/// Documents are created from parsed conversation files and contain
/// both the searchable content and metadata for filtering/display.
///
/// # Example
///
/// ```
/// use glhf::models::document::{DocType, Document};
/// use std::path::PathBuf;
///
/// let doc = Document::new(
///     DocType::Conversation,
///     "Hello, how do I use Rust?".to_string(),
///     PathBuf::from("/path/to/conversation.jsonl"),
/// )
/// .with_role(Some("user".to_string()))
/// .with_project(Some("/Users/me/project".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Creates a new Document with an auto-generated ID.
    ///
    /// The ID is a deterministic hash based on the source path and content,
    /// ensuring the same document always gets the same ID.
    #[must_use]
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

    /// Sets the project path for this document.
    #[must_use]
    pub fn with_project(mut self, project: Option<String>) -> Self {
        self.project = project;
        self
    }

    /// Sets the timestamp for this document.
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: Option<DateTime<Utc>>) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Sets the session ID for this document.
    #[must_use]
    pub fn with_session_id(mut self, session_id: Option<String>) -> Self {
        self.session_id = session_id;
        self
    }

    /// Sets the role for this document.
    #[must_use]
    pub fn with_role(mut self, role: Option<String>) -> Self {
        self.role = role;
        self
    }

    /// Returns a snippet of the content, truncated to approximately `max_chars` characters.
    ///
    /// The snippet will be truncated at a word boundary (space or newline) if possible.
    /// This method is UTF-8 safe and will never split a multi-byte character.
    #[must_use]
    pub fn snippet(&self, max_chars: usize) -> &str {
        // Count characters, not bytes
        let char_count = self.content.chars().count();
        if char_count <= max_chars {
            return &self.content;
        }

        // Find the byte index corresponding to max_chars characters
        let byte_index = self
            .content
            .char_indices()
            .nth(max_chars)
            .map_or(self.content.len(), |(i, _)| i);

        let truncated = &self.content[..byte_index];

        // Try to break at a word boundary
        if let Some(last_space) = truncated.rfind([' ', '\n']) {
            &self.content[..last_space]
        } else {
            truncated
        }
    }
}

/// Generates a deterministic ID from the source path and content.
fn generate_id(source_path: &Path, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_path.to_string_lossy().as_bytes());

    // Use first ~100 chars of content to avoid hashing huge strings
    let content_bytes: String = content.chars().take(100).collect();
    hasher.update(content_bytes.as_bytes());

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

    #[test]
    fn test_snippet_utf8_safe() {
        // Test with multi-byte UTF-8 characters
        let doc = Document::new(
            DocType::Conversation,
            "日本語テスト hello world".to_string(),
            PathBuf::from("/test"),
        );
        // Should not panic and should truncate at character boundary
        let snippet = doc.snippet(5);
        assert_eq!(snippet, "日本語テス");
    }

    #[test]
    fn test_snippet_emoji() {
        let doc = Document::new(
            DocType::Conversation,
            "Hello 🦀 world! This is a test.".to_string(),
            PathBuf::from("/test"),
        );
        let snippet = doc.snippet(10);
        // Should include the emoji and break at word boundary
        assert_eq!(snippet, "Hello 🦀");
    }

    #[test]
    fn test_doctype_default() {
        assert_eq!(DocType::default(), DocType::Conversation);
    }

    #[test]
    fn test_doctype_display() {
        assert_eq!(DocType::Conversation.to_string(), "conversation");
    }
}
