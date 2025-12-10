//! BM25 full-text search index implementation.

#![allow(clippy::too_many_lines)]

use crate::error::{Error, Result};
use crate::models::document::{ChunkKind, Document};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, BooleanQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::{IndexRecordOption, Schema, Value, INDEXED, STORED, STRING, TEXT};
use tantivy::Term;
use tantivy::{Index, IndexReader, IndexWriter, TantivyDocument};

/// A single search result with relevance score and document metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// Unique document identifier.
    pub id: String,
    /// The kind of chunk ("message", "`tool_use`", "`tool_result`").
    pub chunk_kind: String,
    /// The project path this document belongs to.
    pub project: Option<String>,
    /// The Claude Code session ID.
    pub session_id: Option<String>,
    /// The message role ("user" or "assistant") for Message chunks.
    pub role: Option<String>,
    /// The tool name (e.g., "Bash", "Read") for ToolUse/ToolResult chunks.
    pub tool_name: Option<String>,
    /// The tool invocation ID (links `ToolUse` to `ToolResult`).
    pub tool_id: Option<String>,
    /// The tool input parameters as JSON string.
    pub tool_input: Option<String>,
    /// Whether this tool result was an error.
    pub is_error: Option<bool>,
    /// The full document content.
    pub content: String,
    /// BM25 relevance score (higher is more relevant).
    pub score: f32,
    /// When this message was created.
    pub timestamp: Option<DateTime<Utc>>,
}

impl SearchResult {
    /// Returns true if this is a Message chunk.
    #[must_use]
    pub fn is_message(&self) -> bool {
        self.chunk_kind == "message"
    }

    /// Returns true if this is a `ToolUse` chunk.
    #[must_use]
    pub fn is_tool_use(&self) -> bool {
        self.chunk_kind == "tool_use"
    }

    /// Returns true if this is a `ToolResult` chunk.
    #[must_use]
    pub fn is_tool_result(&self) -> bool {
        self.chunk_kind == "tool_result"
    }

    /// Returns a display label for this result.
    #[must_use]
    pub fn display_label(&self) -> String {
        match self.chunk_kind.as_str() {
            "message" => self.role.clone().unwrap_or_else(|| "message".to_string()),
            "tool_use" => {
                format!("tool:{}", self.tool_name.as_deref().unwrap_or("unknown"))
            }
            "tool_result" => {
                let tool = self.tool_name.as_deref().unwrap_or("unknown");
                if self.is_error == Some(true) {
                    format!("result:{tool} (error)")
                } else {
                    format!("result:{tool}")
                }
            }
            _ => self.chunk_kind.clone(),
        }
    }
}

/// BM25 full-text search index backed by [Tantivy](https://github.com/quickwit-oss/tantivy).
///
/// This index stores conversation documents and provides fast full-text search
/// using the BM25 ranking algorithm.
pub struct BM25Index {
    index: Index,
    schema: Schema,
    reader: IndexReader,
}

impl BM25Index {
    /// Creates a new index at the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or the index
    /// cannot be initialized.
    pub fn create(path: &Path) -> Result<Self> {
        // Ensure directory exists
        fs::create_dir_all(path)?;

        let schema = build_schema();
        let index = Index::create_in_dir(path, schema.clone())
            .map_err(|e| Error::from_tantivy(e, "failed to create index"))?;

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| Error::from_tantivy(e, "failed to create index reader"))?;

        Ok(Self {
            index,
            schema,
            reader,
        })
    }

    /// Opens an existing index at the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the index cannot be opened.
    pub fn open(path: &Path) -> Result<Self> {
        let index =
            Index::open_in_dir(path).map_err(|e| Error::from_tantivy(e, "failed to open index"))?;

        let schema = index.schema();
        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| Error::from_tantivy(e, "failed to create index reader"))?;

        Ok(Self {
            index,
            schema,
            reader,
        })
    }

    /// Creates a writer for adding documents.
    ///
    /// # Errors
    ///
    /// Returns an error if the writer cannot be created.
    pub fn writer(&self) -> Result<IndexWriter> {
        // 50MB heap for the writer
        self.index
            .writer(50_000_000)
            .map_err(|e| Error::from_tantivy(e, "failed to create index writer"))
    }

    /// Adds documents to the index.
    ///
    /// # Errors
    ///
    /// Returns an error if a document cannot be added.
    pub fn add_documents(&self, writer: &mut IndexWriter, docs: &[Document]) -> Result<()> {
        let id_field = self.schema.get_field("id").expect("id field exists");
        let chunk_kind_field = self
            .schema
            .get_field("chunk_kind")
            .expect("chunk_kind field exists");
        let project_field = self
            .schema
            .get_field("project")
            .expect("project field exists");
        let session_id_field = self
            .schema
            .get_field("session_id")
            .expect("session_id field exists");
        let role_field = self.schema.get_field("role").expect("role field exists");
        let tool_name_field = self
            .schema
            .get_field("tool_name")
            .expect("tool_name field exists");
        let tool_id_field = self
            .schema
            .get_field("tool_id")
            .expect("tool_id field exists");
        let tool_input_field = self
            .schema
            .get_field("tool_input")
            .expect("tool_input field exists");
        let is_error_field = self
            .schema
            .get_field("is_error")
            .expect("is_error field exists");
        let content_field = self
            .schema
            .get_field("content")
            .expect("content field exists");
        let timestamp_field = self
            .schema
            .get_field("timestamp")
            .expect("timestamp field exists");

        for doc in docs {
            let mut tantivy_doc = TantivyDocument::new();
            tantivy_doc.add_text(id_field, &doc.id);
            tantivy_doc.add_text(chunk_kind_field, doc.chunk_kind.as_str());
            tantivy_doc.add_text(project_field, doc.project.as_deref().unwrap_or(""));
            tantivy_doc.add_text(session_id_field, doc.session_id.as_deref().unwrap_or(""));
            tantivy_doc.add_text(role_field, doc.role.as_deref().unwrap_or(""));
            tantivy_doc.add_text(tool_name_field, doc.tool_name.as_deref().unwrap_or(""));
            tantivy_doc.add_text(tool_id_field, doc.tool_id.as_deref().unwrap_or(""));
            tantivy_doc.add_text(tool_input_field, doc.tool_input.as_deref().unwrap_or(""));
            tantivy_doc.add_text(
                is_error_field,
                doc.is_error
                    .map_or("", |e| if e { "true" } else { "false" }),
            );
            tantivy_doc.add_text(content_field, &doc.content);

            if let Some(ts) = doc.timestamp {
                let dt = tantivy::DateTime::from_timestamp_micros(ts.timestamp_micros());
                tantivy_doc.add_date(timestamp_field, dt);
            }

            writer
                .add_document(tantivy_doc)
                .map_err(|e| Error::from_tantivy(e, "failed to add document"))?;
        }

        Ok(())
    }

    /// Searches the index for documents matching the query.
    ///
    /// # Errors
    ///
    /// Returns an error if the query cannot be parsed or the search fails.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        let content_field = self
            .schema
            .get_field("content")
            .expect("content field exists");
        let query_parser = QueryParser::for_index(&self.index, vec![content_field]);
        let query = query_parser.parse_query(query_str)?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| Error::from_tantivy(e, "search failed"))?;

        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| Error::from_tantivy(e, "failed to retrieve document"))?;

            let mut result = self.extract_search_result(&retrieved_doc);
            result.score = score;
            results.push(result);
        }

        Ok(results)
    }

    /// Searches the index with optional filters for chunk kind, tool name, and errors.
    ///
    /// # Errors
    ///
    /// Returns an error if the query cannot be parsed or the search fails.
    pub fn search_filtered(
        &self,
        query_str: &str,
        limit: usize,
        chunk_kind: Option<ChunkKind>,
        tool_name: Option<&str>,
        errors_only: bool,
    ) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        // Build the content query
        let content_field = self
            .schema
            .get_field("content")
            .expect("content field exists");
        let query_parser = QueryParser::for_index(&self.index, vec![content_field]);
        let content_query = query_parser.parse_query(query_str)?;

        // Build filter clauses
        let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = vec![];
        clauses.push((Occur::Must, Box::new(content_query)));

        // Filter by chunk_kind
        if let Some(kind) = chunk_kind {
            let chunk_kind_field = self
                .schema
                .get_field("chunk_kind")
                .expect("chunk_kind field exists");
            let term = Term::from_field_text(chunk_kind_field, kind.as_str());
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        // Filter by tool_name
        if let Some(name) = tool_name {
            let tool_name_field = self
                .schema
                .get_field("tool_name")
                .expect("tool_name field exists");
            let term = Term::from_field_text(tool_name_field, name);
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        // Filter by is_error
        if errors_only {
            let is_error_field = self
                .schema
                .get_field("is_error")
                .expect("is_error field exists");
            let term = Term::from_field_text(is_error_field, "true");
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        let query = BooleanQuery::new(clauses);

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| Error::from_tantivy(e, "filtered search failed"))?;

        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| Error::from_tantivy(e, "failed to retrieve document"))?;

            let mut result = self.extract_search_result(&retrieved_doc);
            result.score = score;
            results.push(result);
        }

        Ok(results)
    }

    /// Searches the index using a regular expression pattern.
    ///
    /// This performs a full scan of all documents and filters using the regex.
    /// For case-insensitive matching, set `ignore_case` to true.
    ///
    /// # Errors
    ///
    /// Returns an error if the regex pattern is invalid or the search fails.
    pub fn search_regex(
        &self,
        pattern: &str,
        limit: usize,
        ignore_case: bool,
    ) -> Result<Vec<SearchResult>> {
        // Build regex with case sensitivity option
        let regex = regex::RegexBuilder::new(pattern)
            .case_insensitive(ignore_case)
            .build()?;

        let searcher = self.reader.searcher();

        // We need to scan all documents and filter by regex
        let top_docs = searcher
            .search(&AllQuery, &TopDocs::with_limit(100_000))
            .map_err(|e| Error::from_tantivy(e, "regex search failed"))?;

        let mut results: Vec<SearchResult> = top_docs
            .into_iter()
            .filter_map(|(_, doc_address)| {
                searcher.doc(doc_address).ok().and_then(|doc| {
                    let result = self.extract_search_result(&doc);
                    regex.is_match(&result.content).then_some(result)
                })
            })
            .take(limit)
            .collect();

        // Sort by timestamp (most recent first for regex matches)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(results)
    }

    /// Retrieves all messages for a given session, sorted by timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if the search fails.
    pub fn get_session_messages(&self, session_id: &str) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        let session_id_field = self
            .schema
            .get_field("session_id")
            .expect("session_id field exists");

        let term = Term::from_field_text(session_id_field, session_id);
        let query = TermQuery::new(term, IndexRecordOption::Basic);

        // Get all docs for this session (up to a reasonable limit)
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(1000))
            .map_err(|e| Error::from_tantivy(e, "session query failed"))?;

        let mut results: Vec<SearchResult> = top_docs
            .into_iter()
            .filter_map(|(_, doc_address)| {
                searcher.doc(doc_address).ok().map(|doc| {
                    let mut result = self.extract_search_result(&doc);
                    result.score = 0.0;
                    result
                })
            })
            .collect();

        // Sort by timestamp
        results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(results)
    }

    /// Retrieves all messages (for getting context when `session_id` is unknown).
    ///
    /// This is expensive and should only be used when necessary.
    ///
    /// # Errors
    ///
    /// Returns an error if the search fails.
    pub fn get_all_messages(&self) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        let top_docs = searcher
            .search(&AllQuery, &TopDocs::with_limit(100_000))
            .map_err(|e| Error::from_tantivy(e, "all query failed"))?;

        let mut results: Vec<SearchResult> = top_docs
            .into_iter()
            .filter_map(|(_, doc_address)| {
                searcher.doc(doc_address).ok().map(|doc| {
                    let mut result = self.extract_search_result(&doc);
                    result.score = 0.0;
                    result
                })
            })
            .collect();

        // Sort by timestamp
        results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(results)
    }

    /// Extracts a [`SearchResult`] from a [`TantivyDocument`].
    fn extract_search_result(&self, doc: &TantivyDocument) -> SearchResult {
        let id_field = self.schema.get_field("id").expect("id field exists");
        let chunk_kind_field = self
            .schema
            .get_field("chunk_kind")
            .expect("chunk_kind field exists");
        let project_field = self
            .schema
            .get_field("project")
            .expect("project field exists");
        let session_id_field = self
            .schema
            .get_field("session_id")
            .expect("session_id field exists");
        let role_field = self.schema.get_field("role").expect("role field exists");
        let tool_name_field = self
            .schema
            .get_field("tool_name")
            .expect("tool_name field exists");
        let tool_id_field = self
            .schema
            .get_field("tool_id")
            .expect("tool_id field exists");
        let tool_input_field = self
            .schema
            .get_field("tool_input")
            .expect("tool_input field exists");
        let is_error_field = self
            .schema
            .get_field("is_error")
            .expect("is_error field exists");
        let content_field = self
            .schema
            .get_field("content")
            .expect("content field exists");
        let timestamp_field = self
            .schema
            .get_field("timestamp")
            .expect("timestamp field exists");

        let id = doc
            .get_first(id_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let chunk_kind = doc
            .get_first(chunk_kind_field)
            .and_then(|v| v.as_str())
            .unwrap_or("message")
            .to_string();

        let project = doc
            .get_first(project_field)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        let session_id = doc
            .get_first(session_id_field)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        let role = doc
            .get_first(role_field)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        let tool_name = doc
            .get_first(tool_name_field)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        let tool_id = doc
            .get_first(tool_id_field)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        let tool_input = doc
            .get_first(tool_input_field)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        let is_error = doc
            .get_first(is_error_field)
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            });

        let content = doc
            .get_first(content_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let timestamp = doc
            .get_first(timestamp_field)
            .and_then(|v| v.as_datetime())
            .and_then(|dt| DateTime::<Utc>::from_timestamp_micros(dt.into_timestamp_micros()));

        SearchResult {
            id,
            chunk_kind,
            project,
            session_id,
            role,
            tool_name,
            tool_id,
            tool_input,
            is_error,
            content,
            score: 0.0,
            timestamp,
        }
    }

    /// Returns the number of documents in the index.
    #[must_use]
    pub fn num_docs(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    /// Manually reload the reader to see recent commits.
    ///
    /// # Errors
    ///
    /// Returns an error if the reader cannot be reloaded.
    pub fn reload(&self) -> Result<()> {
        self.reader
            .reload()
            .map_err(|e| Error::from_tantivy(e, "failed to reload reader"))
    }
}

fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // ID field - stored but not indexed for search
    schema_builder.add_text_field("id", STRING | STORED);

    // Chunk kind - stored and indexed as keyword
    schema_builder.add_text_field("chunk_kind", STRING | STORED);

    // Metadata fields - stored and indexed as keywords
    schema_builder.add_text_field("project", STRING | STORED);
    schema_builder.add_text_field("session_id", STRING | STORED);
    schema_builder.add_text_field("role", STRING | STORED);

    // Tool-specific fields
    schema_builder.add_text_field("tool_name", STRING | STORED);
    schema_builder.add_text_field("tool_id", STRING | STORED);
    schema_builder.add_text_field("tool_input", STORED); // Stored but not indexed (can be large)
    schema_builder.add_text_field("is_error", STRING | STORED);

    // Main content - full-text indexed and stored
    schema_builder.add_text_field("content", TEXT | STORED);

    // Timestamp - indexed for range queries
    schema_builder.add_date_field("timestamp", INDEXED | STORED);

    schema_builder.build()
}
