//! BM25 full-text search index implementation.

use crate::models::document::Document;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, TantivyDocument};

/// A single search result with relevance score and document metadata.
#[derive(Debug)]
pub struct SearchResult {
    /// Unique document identifier.
    pub id: String,
    /// The type of document ("conversation", etc.).
    pub doc_type: String,
    /// The project path this document belongs to.
    pub project: Option<String>,
    /// The Claude Code session ID.
    pub session_id: Option<String>,
    /// The message role ("user" or "assistant").
    pub role: Option<String>,
    /// The full document content.
    pub content: String,
    /// BM25 relevance score (higher is more relevant).
    pub score: f32,
    /// When this message was created.
    pub timestamp: Option<DateTime<Utc>>,
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
    /// Creates a new index at the specified path
    pub fn create(path: &Path) -> Result<Self> {
        // Ensure directory exists
        fs::create_dir_all(path)?;

        let schema = build_schema();
        let index =
            Index::create_in_dir(path, schema.clone()).context("Failed to create tantivy index")?;

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("Failed to create index reader")?;

        Ok(Self {
            index,
            schema,
            reader,
        })
    }

    /// Opens an existing index at the specified path
    pub fn open(path: &Path) -> Result<Self> {
        let index = Index::open_in_dir(path).context("Failed to open tantivy index")?;

        let schema = index.schema();
        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("Failed to create index reader")?;

        Ok(Self {
            index,
            schema,
            reader,
        })
    }

    /// Creates a writer for adding documents
    pub fn writer(&self) -> Result<IndexWriter> {
        // 50MB heap for the writer
        self.index
            .writer(50_000_000)
            .context("Failed to create index writer")
    }

    /// Adds documents to the index
    pub fn add_documents(&self, writer: &mut IndexWriter, docs: &[Document]) -> Result<()> {
        let id_field = self.schema.get_field("id").unwrap();
        let doc_type_field = self.schema.get_field("doc_type").unwrap();
        let project_field = self.schema.get_field("project").unwrap();
        let session_id_field = self.schema.get_field("session_id").unwrap();
        let role_field = self.schema.get_field("role").unwrap();
        let content_field = self.schema.get_field("content").unwrap();
        let timestamp_field = self.schema.get_field("timestamp").unwrap();

        for doc in docs {
            let mut tantivy_doc = TantivyDocument::new();
            tantivy_doc.add_text(id_field, &doc.id);
            tantivy_doc.add_text(doc_type_field, doc.doc_type.as_str());
            tantivy_doc.add_text(project_field, doc.project.as_deref().unwrap_or(""));
            tantivy_doc.add_text(session_id_field, doc.session_id.as_deref().unwrap_or(""));
            tantivy_doc.add_text(role_field, doc.role.as_deref().unwrap_or(""));
            tantivy_doc.add_text(content_field, &doc.content);

            if let Some(ts) = doc.timestamp {
                let dt = tantivy::DateTime::from_timestamp_micros(ts.timestamp_micros());
                tantivy_doc.add_date(timestamp_field, dt);
            }

            writer.add_document(tantivy_doc)?;
        }

        Ok(())
    }

    /// Searches the index
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        let content_field = self.schema.get_field("content").unwrap();
        let query_parser = QueryParser::for_index(&self.index, vec![content_field]);
        let query = query_parser
            .parse_query(query_str)
            .context("Failed to parse query")?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .context("Search failed")?;

        let id_field = self.schema.get_field("id").unwrap();
        let doc_type_field = self.schema.get_field("doc_type").unwrap();
        let project_field = self.schema.get_field("project").unwrap();
        let session_id_field = self.schema.get_field("session_id").unwrap();
        let role_field = self.schema.get_field("role").unwrap();
        let timestamp_field = self.schema.get_field("timestamp").unwrap();

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let id = retrieved_doc
                .get_first(id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let doc_type = retrieved_doc
                .get_first(doc_type_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let project = retrieved_doc
                .get_first(project_field)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from);

            let session_id = retrieved_doc
                .get_first(session_id_field)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from);

            let role = retrieved_doc
                .get_first(role_field)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from);

            let content = retrieved_doc
                .get_first(content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let timestamp = retrieved_doc
                .get_first(timestamp_field)
                .and_then(|v| v.as_datetime())
                .map(|dt| {
                    DateTime::<Utc>::from_timestamp_micros(dt.into_timestamp_micros())
                        .unwrap_or_default()
                });

            results.push(SearchResult {
                id,
                doc_type,
                project,
                session_id,
                role,
                content,
                score,
                timestamp,
            });
        }

        Ok(results)
    }

    /// Returns the number of documents in the index
    pub fn num_docs(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    /// Manually reload the reader to see recent commits
    pub fn reload(&self) -> Result<()> {
        self.reader.reload().context("Failed to reload reader")
    }
}

fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // ID field - stored but not indexed for search
    schema_builder.add_text_field("id", STRING | STORED);

    // Metadata fields - stored and indexed as keywords
    schema_builder.add_text_field("doc_type", STRING | STORED);
    schema_builder.add_text_field("project", STRING | STORED);
    schema_builder.add_text_field("session_id", STRING | STORED);
    schema_builder.add_text_field("role", STRING | STORED);

    // Main content - full-text indexed and stored
    schema_builder.add_text_field("content", TEXT | STORED);

    // Timestamp - indexed for range queries
    schema_builder.add_date_field("timestamp", INDEXED | STORED);

    schema_builder.build()
}
