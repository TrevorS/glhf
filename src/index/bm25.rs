//! BM25 full-text search index implementation.

use crate::error::{Error, Result};
use crate::models::document::Document;
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, QueryParser, TermQuery};
use tantivy::schema::{IndexRecordOption, Schema, Value, INDEXED, STORED, STRING, TEXT};
use tantivy::Term;
use tantivy::{Index, IndexReader, IndexWriter, TantivyDocument};

/// A single search result with relevance score and document metadata.
#[derive(Debug, Clone, PartialEq)]
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
        let doc_type_field = self
            .schema
            .get_field("doc_type")
            .expect("doc_type field exists");
        let project_field = self
            .schema
            .get_field("project")
            .expect("project field exists");
        let session_id_field = self
            .schema
            .get_field("session_id")
            .expect("session_id field exists");
        let role_field = self.schema.get_field("role").expect("role field exists");
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
            tantivy_doc.add_text(doc_type_field, doc.doc_type.as_str());
            tantivy_doc.add_text(project_field, doc.project.as_deref().unwrap_or(""));
            tantivy_doc.add_text(session_id_field, doc.session_id.as_deref().unwrap_or(""));
            tantivy_doc.add_text(role_field, doc.role.as_deref().unwrap_or(""));
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
        let doc_type_field = self
            .schema
            .get_field("doc_type")
            .expect("doc_type field exists");
        let project_field = self
            .schema
            .get_field("project")
            .expect("project field exists");
        let session_id_field = self
            .schema
            .get_field("session_id")
            .expect("session_id field exists");
        let role_field = self.schema.get_field("role").expect("role field exists");
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

        let doc_type = doc
            .get_first(doc_type_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
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
            doc_type,
            project,
            session_id,
            role,
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
