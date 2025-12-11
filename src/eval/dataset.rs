//! Dataset types for evaluation.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// A document in the evaluation corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique document ID.
    pub id: String,
    /// Document content (the text to embed).
    pub content: String,
    /// Type of chunk (message, `tool_use`, `tool_result`).
    pub chunk_kind: String,
    /// Role if message (user/assistant).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Tool name if tool chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

/// The evaluation corpus - a collection of documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Corpus {
    pub documents: Vec<Document>,
}

impl Corpus {
    /// Load corpus from a JSON file.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save corpus to a JSON file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }

    /// Get all document IDs.
    pub fn doc_ids(&self) -> HashSet<String> {
        self.documents.iter().map(|d| d.id.clone()).collect()
    }

    /// Get document by ID.
    pub fn get(&self, id: &str) -> Option<&Document> {
        self.documents.iter().find(|d| d.id == id)
    }
}

/// Type of query for categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    /// Conceptual/semantic queries.
    Semantic,
    /// Exact keyword matches.
    Keyword,
    /// Looking for code patterns.
    Code,
    /// Finding tool invocations.
    Tool,
    /// Finding error messages.
    Error,
}

impl std::fmt::Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Semantic => write!(f, "semantic"),
            Self::Keyword => write!(f, "keyword"),
            Self::Code => write!(f, "code"),
            Self::Tool => write!(f, "tool"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// A test query with ground truth relevance labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    /// Unique query ID.
    pub id: String,
    /// The query text.
    pub query: String,
    /// Type of query.
    pub query_type: QueryType,
    /// IDs of relevant documents (ground truth).
    pub relevant_doc_ids: Vec<String>,
    /// Optional tags for filtering.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Collection of test queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySet {
    pub queries: Vec<Query>,
}

impl QuerySet {
    /// Load queries from a JSON file.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save queries to a JSON file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }
}
