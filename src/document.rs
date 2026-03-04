//! Document types for indexed content.

use chrono::{DateTime, Utc};
use std::fmt;
use std::path::{Path, PathBuf};

/// Trait for types that can generate a display label for search results.
///
/// This provides a consistent way to display chunk types across different
/// result types (Document, `SearchResult`, etc.).
pub trait DisplayLabel {
    /// Returns the chunk kind as a string slice.
    fn chunk_kind_str(&self) -> &str;

    /// Returns the role if this is a message chunk.
    fn role_ref(&self) -> Option<&str>;

    /// Returns the tool name if this is a tool chunk.
    fn tool_name_ref(&self) -> Option<&str>;

    /// Returns whether this is an error result.
    fn is_error_flag(&self) -> Option<bool>;

    /// Returns a display label for this chunk.
    fn display_label(&self) -> String {
        match self.chunk_kind_str() {
            "message" => self
                .role_ref()
                .map_or_else(|| "message".to_string(), String::from),
            "tool_use" => {
                format!("tool:{}", self.tool_name_ref().unwrap_or("unknown"))
            }
            "tool_result" => {
                let tool = self.tool_name_ref().unwrap_or("unknown");
                if self.is_error_flag() == Some(true) {
                    format!("result:{tool} (error)")
                } else {
                    format!("result:{tool}")
                }
            }
            other => other.to_string(),
        }
    }
}

/// The kind of chunk being indexed.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new chunk types
/// in the future without breaking semver compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ChunkKind {
    /// A text message (user prompt or assistant response).
    #[default]
    Message,
    /// A tool invocation by the assistant.
    ToolUse,
    /// The result/output from a tool execution.
    ToolResult,
}

impl fmt::Display for ChunkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ChunkKind {
    /// Returns the string representation of the chunk kind.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            ChunkKind::Message => "message",
            ChunkKind::ToolUse => "tool_use",
            ChunkKind::ToolResult => "tool_result",
        }
    }

    /// Parses a string into a `ChunkKind`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "message" => Some(ChunkKind::Message),
            "tool_use" => Some(ChunkKind::ToolUse),
            "tool_result" => Some(ChunkKind::ToolResult),
            _ => None,
        }
    }
}

/// A document/chunk to be indexed and searched.
///
/// Documents are created from parsed conversation files and contain
/// both the searchable content and metadata for filtering/display.
///
/// # Example
///
/// ```
/// use glhf::{ChunkKind, Document};
/// use std::path::PathBuf;
///
/// // A user message
/// let msg = Document::new(
///     ChunkKind::Message,
///     "Hello, how do I use Rust?".to_string(),
///     PathBuf::from("/path/to/conversation.jsonl"),
/// )
/// .with_role(Some("user".to_string()))
/// .with_project(Some("/Users/me/project".to_string()));
///
/// // A tool use
/// let tool = Document::new(
///     ChunkKind::ToolUse,
///     "git status".to_string(),
///     PathBuf::from("/path/to/conversation.jsonl"),
/// )
/// .with_tool_name(Some("Bash".to_string()))
/// .with_tool_input(Some(r#"{"command": "git status"}"#.to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    // === Identity ===
    /// Unique identifier derived from source path and content hash.
    pub id: String,
    /// The kind of chunk (Message, `ToolUse`, `ToolResult`).
    pub chunk_kind: ChunkKind,

    // === Context ===
    /// The project path this document belongs to.
    pub project: Option<String>,
    /// When this chunk was created.
    pub timestamp: Option<DateTime<Utc>>,
    /// The Claude Code session ID.
    pub session_id: Option<String>,
    /// Path to the source file this document was extracted from.
    pub source_path: PathBuf,

    // === Message-specific ===
    /// The message role ("user" or "assistant") for Message chunks.
    pub role: Option<String>,

    // === Tool-specific ===
    /// The tool name (e.g., "Bash", "Read", "Edit", "Grep").
    pub tool_name: Option<String>,
    /// The tool invocation ID (links `ToolUse` to its `ToolResult`).
    pub tool_id: Option<String>,
    /// The tool input parameters as JSON string.
    pub tool_input: Option<String>,
    /// Whether this tool result was an error.
    pub is_error: Option<bool>,

    // === Searchable Content ===
    /// The primary searchable text content.
    pub content: String,
}

impl Document {
    /// Creates a new Document with an auto-generated ID.
    ///
    /// The ID is a deterministic hash based on the source path and content,
    /// ensuring the same document always gets the same ID.
    #[must_use]
    pub fn new(chunk_kind: ChunkKind, content: String, source_path: PathBuf) -> Self {
        let id = generate_id(&source_path, &content);
        Self {
            id,
            chunk_kind,
            project: None,
            timestamp: None,
            session_id: None,
            source_path,
            role: None,
            tool_name: None,
            tool_id: None,
            tool_input: None,
            is_error: None,
            content,
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

    /// Sets the role for this document (for Message chunks).
    #[must_use]
    pub fn with_role(mut self, role: Option<String>) -> Self {
        self.role = role;
        self
    }

    /// Sets the tool name for this document (for ToolUse/ToolResult chunks).
    #[must_use]
    pub fn with_tool_name(mut self, tool_name: Option<String>) -> Self {
        self.tool_name = tool_name;
        self
    }

    /// Sets the tool ID for this document.
    #[must_use]
    pub fn with_tool_id(mut self, tool_id: Option<String>) -> Self {
        self.tool_id = tool_id;
        self
    }

    /// Sets the tool input for this document.
    #[must_use]
    pub fn with_tool_input(mut self, tool_input: Option<String>) -> Self {
        self.tool_input = tool_input;
        self
    }

    /// Sets whether this tool result was an error.
    #[must_use]
    pub fn with_is_error(mut self, is_error: Option<bool>) -> Self {
        self.is_error = is_error;
        self
    }

    /// Returns true if this is a Message chunk.
    #[must_use]
    pub fn is_message(&self) -> bool {
        self.chunk_kind == ChunkKind::Message
    }

    /// Returns true if this is a `ToolUse` chunk.
    #[must_use]
    pub fn is_tool_use(&self) -> bool {
        self.chunk_kind == ChunkKind::ToolUse
    }

    /// Returns true if this is a `ToolResult` chunk.
    #[must_use]
    pub fn is_tool_result(&self) -> bool {
        self.chunk_kind == ChunkKind::ToolResult
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

impl DisplayLabel for Document {
    fn chunk_kind_str(&self) -> &str {
        self.chunk_kind.as_str()
    }

    fn role_ref(&self) -> Option<&str> {
        self.role.as_deref()
    }

    fn tool_name_ref(&self) -> Option<&str> {
        self.tool_name.as_deref()
    }

    fn is_error_flag(&self) -> Option<bool> {
        self.is_error
    }
}

/// Generates a deterministic ID based on source path and content.
///
/// Uses SHA-256 hash of the source path and content to ensure:
/// - Same document always gets the same ID (idempotent indexing)
/// - Different documents get different IDs (collision-resistant)
/// - Re-indexing won't create duplicates
fn generate_id(source_path: &Path, content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(source_path.to_string_lossy().as_bytes());
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())[..32].to_string()
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
        // Same input should produce same ID (deterministic)
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 32); // First 32 chars of hex-encoded SHA-256
    }

    #[test]
    fn test_generate_id_different_for_different_content() {
        let path = PathBuf::from("/test/path.jsonl");
        let id1 = generate_id(&path, "content1");
        let id2 = generate_id(&path, "content2");
        // Different content should produce different IDs
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_id_different_for_different_paths() {
        let content = "same content";
        let id1 = generate_id(&PathBuf::from("/path1.jsonl"), content);
        let id2 = generate_id(&PathBuf::from("/path2.jsonl"), content);
        // Different paths should produce different IDs
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_snippet_short() {
        let doc = Document::new(
            ChunkKind::Message,
            "short".to_string(),
            PathBuf::from("/test"),
        );
        assert_eq!(doc.snippet(100), "short");
    }

    #[test]
    fn test_snippet_truncates() {
        let doc = Document::new(
            ChunkKind::Message,
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
            ChunkKind::Message,
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
            ChunkKind::Message,
            "Hello 🦀 world! This is a test.".to_string(),
            PathBuf::from("/test"),
        );
        let snippet = doc.snippet(10);
        // Should include the emoji and break at word boundary
        assert_eq!(snippet, "Hello 🦀");
    }

    #[test]
    fn test_chunk_kind_default() {
        assert_eq!(ChunkKind::default(), ChunkKind::Message);
    }

    #[test]
    fn test_chunk_kind_display() {
        assert_eq!(ChunkKind::Message.to_string(), "message");
        assert_eq!(ChunkKind::ToolUse.to_string(), "tool_use");
        assert_eq!(ChunkKind::ToolResult.to_string(), "tool_result");
    }

    #[test]
    fn test_chunk_kind_parse() {
        assert_eq!(ChunkKind::parse("message"), Some(ChunkKind::Message));
        assert_eq!(ChunkKind::parse("tool_use"), Some(ChunkKind::ToolUse));
        assert_eq!(ChunkKind::parse("tool_result"), Some(ChunkKind::ToolResult));
        assert_eq!(ChunkKind::parse("invalid"), None);
    }

    #[test]
    fn test_display_label_message() {
        let doc = Document::new(
            ChunkKind::Message,
            "test".to_string(),
            PathBuf::from("/test"),
        )
        .with_role(Some("user".to_string()));
        assert_eq!(doc.display_label(), "user");
    }

    #[test]
    fn test_display_label_tool_use() {
        let doc = Document::new(
            ChunkKind::ToolUse,
            "git status".to_string(),
            PathBuf::from("/test"),
        )
        .with_tool_name(Some("Bash".to_string()));
        assert_eq!(doc.display_label(), "tool:Bash");
    }

    #[test]
    fn test_display_label_tool_result_error() {
        let doc = Document::new(
            ChunkKind::ToolResult,
            "error output".to_string(),
            PathBuf::from("/test"),
        )
        .with_tool_name(Some("Bash".to_string()))
        .with_is_error(Some(true));
        assert_eq!(doc.display_label(), "result:Bash (error)");
    }

    #[test]
    fn test_is_helpers() {
        let msg = Document::new(ChunkKind::Message, String::new(), PathBuf::from("/test"));
        assert!(msg.is_message());
        assert!(!msg.is_tool_use());
        assert!(!msg.is_tool_result());

        let tool = Document::new(ChunkKind::ToolUse, String::new(), PathBuf::from("/test"));
        assert!(!tool.is_message());
        assert!(tool.is_tool_use());
        assert!(!tool.is_tool_result());
    }

    // --- Property tests ---

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_generate_id_deterministic(
            path in "[a-z/]{1,50}",
            content in ".*"
        ) {
            let path = PathBuf::from(&path);
            let id1 = generate_id(&path, &content);
            let id2 = generate_id(&path, &content);
            prop_assert_eq!(&id1, &id2);
        }

        #[test]
        fn proptest_generate_id_length_always_32(
            path in ".*",
            content in ".*"
        ) {
            let id = generate_id(&PathBuf::from(&path), &content);
            prop_assert_eq!(id.len(), 32);
        }

        #[test]
        fn proptest_generate_id_all_hex(
            path in ".*",
            content in ".*"
        ) {
            let id = generate_id(&PathBuf::from(&path), &content);
            prop_assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        }

        #[test]
        fn proptest_snippet_length_bounded(
            content in ".{1,500}",
            max_chars in 1..500usize
        ) {
            let doc = Document::new(
                ChunkKind::Message,
                content,
                PathBuf::from("/test"),
            );
            let snippet = doc.snippet(max_chars);
            prop_assert!(snippet.chars().count() <= max_chars);
        }

        #[test]
        fn proptest_snippet_max_returns_full_content(content in ".{0,200}") {
            let doc = Document::new(
                ChunkKind::Message,
                content.clone(),
                PathBuf::from("/test"),
            );
            let snippet = doc.snippet(usize::MAX);
            prop_assert_eq!(snippet, content.as_str());
        }

        #[test]
        fn proptest_snippet_valid_utf8(content in "\\PC{0,200}", max_chars in 0..100usize) {
            let doc = Document::new(
                ChunkKind::Message,
                content,
                PathBuf::from("/test"),
            );
            let snippet = doc.snippet(max_chars);
            // snippet is &str, so it's always valid UTF-8
            prop_assert!(std::str::from_utf8(snippet.as_bytes()).is_ok());
        }
    }

    #[test]
    fn test_chunk_kind_roundtrip_exhaustive() {
        let kinds = [
            ChunkKind::Message,
            ChunkKind::ToolUse,
            ChunkKind::ToolResult,
        ];
        for kind in &kinds {
            let s = kind.as_str();
            let parsed = ChunkKind::parse(s);
            assert_eq!(parsed, Some(*kind), "Roundtrip failed for {kind:?}");
        }
    }
}
