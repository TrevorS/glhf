//! `SQLite` database layer with FTS5 and vector search support.
//!
//! This module provides unified storage for documents, full-text search via FTS5,
//! and vector similarity search via sqlite-vec.

use crate::models::document::{ChunkKind, Document};
use crate::Result;
use rusqlite::{params, Connection};
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Once;

/// Embedding dimension for all-MiniLM-L6-v2 model.
pub const EMBEDDING_DIM: usize = 384;

/// Ensures sqlite-vec is registered only once per process.
static SQLITE_VEC_INIT: Once = Once::new();

/// Registers the sqlite-vec extension globally using `sqlite3_auto_extension`.
/// This must be called before opening any connections that need vector support.
fn init_sqlite_vec() {
    SQLITE_VEC_INIT.call_once(|| {
        // SAFETY: sqlite3_auto_extension is thread-safe and sqlite3_vec_init
        // is a valid extension entry point. We use transmute because the function
        // signatures differ slightly but are compatible.
        #[allow(unsafe_code, clippy::missing_transmute_annotations)]
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
    });
}

/// A search result from the database.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub chunk_kind: String,
    pub content: String,
    pub project: Option<String>,
    pub session_id: Option<String>,
    pub role: Option<String>,
    pub tool_name: Option<String>,
    pub tool_id: Option<String>,
    pub is_error: Option<bool>,
    pub timestamp: Option<String>,
    pub score: f64,
}

impl SearchResult {
    /// Returns a display label for this result.
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

/// `SQLite` database with FTS5 and vector search capabilities.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens or creates a database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        // Initialize sqlite-vec extension (once per process)
        init_sqlite_vec();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Creates a new in-memory database (for testing).
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        // Initialize sqlite-vec extension (once per process)
        init_sqlite_vec();

        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Initializes the database schema.
    fn init_schema(&self) -> Result<()> {
        // Main documents table
        self.conn.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                chunk_kind TEXT NOT NULL,
                content TEXT NOT NULL,
                project TEXT,
                session_id TEXT,
                role TEXT,
                tool_name TEXT,
                tool_id TEXT,
                tool_input TEXT,
                is_error INTEGER,
                timestamp TEXT,
                source_path TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_documents_session ON documents(session_id);
            CREATE INDEX IF NOT EXISTS idx_documents_project ON documents(project);
            CREATE INDEX IF NOT EXISTS idx_documents_chunk_kind ON documents(chunk_kind);
            CREATE INDEX IF NOT EXISTS idx_documents_tool_name ON documents(tool_name);
            ",
        )?;

        // FTS5 virtual table for full-text search
        self.conn.execute_batch(
            r"
            CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
                content,
                content='documents',
                content_rowid='rowid'
            );

            -- Triggers to keep FTS in sync
            CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
                INSERT INTO documents_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
            END;

            CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
                INSERT INTO documents_fts(documents_fts, rowid, content) VALUES('delete', OLD.rowid, OLD.content);
            END;

            CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents BEGIN
                INSERT INTO documents_fts(documents_fts, rowid, content) VALUES('delete', OLD.rowid, OLD.content);
                INSERT INTO documents_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
            END;
            ",
        )?;

        // Vector table for embeddings
        self.conn.execute(
            &format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS documents_vec USING vec0(
                    id TEXT PRIMARY KEY,
                    embedding FLOAT[{EMBEDDING_DIM}]
                )"
            ),
            [],
        )?;

        Ok(())
    }

    /// Clears all data from the database.
    pub fn clear(&self) -> Result<()> {
        self.conn.execute_batch(
            r"
            DELETE FROM documents_vec;
            DELETE FROM documents;
            DELETE FROM documents_fts;
            ",
        )?;
        Ok(())
    }

    /// Inserts a document into the database.
    pub fn insert_document(&self, doc: &Document) -> Result<()> {
        self.conn.execute(
            r"
            INSERT OR REPLACE INTO documents
            (id, chunk_kind, content, project, session_id, role, tool_name, tool_id, tool_input, is_error, timestamp, source_path)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ",
            params![
                doc.id,
                doc.chunk_kind.as_str(),
                doc.content,
                doc.project,
                doc.session_id,
                doc.role,
                doc.tool_name,
                doc.tool_id,
                doc.tool_input,
                doc.is_error,
                doc.timestamp.map(|t| t.to_rfc3339()),
                doc.source_path.to_string_lossy(),
            ],
        )?;
        Ok(())
    }

    /// Inserts multiple documents in a transaction.
    pub fn insert_documents(&mut self, docs: &[Document]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                r"
                INSERT OR REPLACE INTO documents
                (id, chunk_kind, content, project, session_id, role, tool_name, tool_id, tool_input, is_error, timestamp, source_path)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ",
            )?;

            for doc in docs {
                stmt.execute(params![
                    doc.id,
                    doc.chunk_kind.as_str(),
                    doc.content,
                    doc.project,
                    doc.session_id,
                    doc.role,
                    doc.tool_name,
                    doc.tool_id,
                    doc.tool_input,
                    doc.is_error,
                    doc.timestamp.map(|t| t.to_rfc3339()),
                    doc.source_path.to_string_lossy(),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Inserts an embedding for a document.
    pub fn insert_embedding(&self, doc_id: &str, embedding: &[f32]) -> Result<()> {
        let embedding_bytes = embedding_to_bytes(embedding);
        self.conn.execute(
            "INSERT OR REPLACE INTO documents_vec (id, embedding) VALUES (?1, ?2)",
            params![doc_id, embedding_bytes],
        )?;
        Ok(())
    }

    /// Inserts multiple embeddings in a transaction.
    pub fn insert_embeddings(&mut self, embeddings: &[(&str, &[f32])]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt =
                tx.prepare("INSERT OR REPLACE INTO documents_vec (id, embedding) VALUES (?1, ?2)")?;

            for (doc_id, embedding) in embeddings {
                let embedding_bytes = embedding_to_bytes(embedding);
                stmt.execute(params![doc_id, embedding_bytes])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Returns the number of documents in the database.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn document_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))?;
        Ok(count.max(0) as usize)
    }

    /// Returns the number of embeddings in the database.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn embedding_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM documents_vec", [], |row| row.get(0))?;
        Ok(count.max(0) as usize)
    }

    /// Full-text search using FTS5.
    #[allow(clippy::cast_possible_wrap)]
    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT d.id, d.chunk_kind, d.content, d.project, d.session_id,
                   d.role, d.tool_name, d.tool_id, d.is_error, d.timestamp,
                   bm25(documents_fts) as score
            FROM documents_fts f
            JOIN documents d ON d.rowid = f.rowid
            WHERE documents_fts MATCH ?1
            ORDER BY score
            LIMIT ?2
            ",
        )?;

        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok(SearchResult {
                    id: row.get(0)?,
                    chunk_kind: row.get(1)?,
                    content: row.get(2)?,
                    project: row.get(3)?,
                    session_id: row.get(4)?,
                    role: row.get(5)?,
                    tool_name: row.get(6)?,
                    tool_id: row.get(7)?,
                    is_error: row.get(8)?,
                    timestamp: row.get(9)?,
                    score: row.get::<_, f64>(10)?.abs(), // BM25 returns negative scores
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Full-text search with filters.
    #[allow(clippy::cast_possible_wrap)]
    pub fn search_fts_filtered(
        &self,
        query: &str,
        limit: usize,
        chunk_kind: Option<ChunkKind>,
        tool_name: Option<&str>,
        errors_only: bool,
    ) -> Result<Vec<SearchResult>> {
        let mut sql = String::from(
            r"
            SELECT d.id, d.chunk_kind, d.content, d.project, d.session_id,
                   d.role, d.tool_name, d.tool_id, d.is_error, d.timestamp,
                   bm25(documents_fts) as score
            FROM documents_fts f
            JOIN documents d ON d.rowid = f.rowid
            WHERE documents_fts MATCH ?1
            ",
        );

        let mut param_idx = 2;
        if chunk_kind.is_some() {
            let _ = write!(sql, " AND d.chunk_kind = ?{param_idx}");
            param_idx += 1;
        }
        if tool_name.is_some() {
            let _ = write!(sql, " AND LOWER(d.tool_name) = LOWER(?{param_idx})");
            param_idx += 1;
        }
        if errors_only {
            let _ = write!(sql, " AND d.is_error = ?{param_idx}");
        }

        sql.push_str(" ORDER BY score LIMIT ?");

        let mut stmt = self.conn.prepare(&sql)?;

        // Build dynamic parameters
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(query.to_string())];

        if let Some(ck) = chunk_kind {
            params_vec.push(Box::new(ck.as_str().to_string()));
        }
        if let Some(tn) = tool_name {
            params_vec.push(Box::new(tn.to_string()));
        }
        if errors_only {
            params_vec.push(Box::new(1_i32));
        }
        params_vec.push(Box::new(limit as i64));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(AsRef::as_ref).collect();

        let results = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(SearchResult {
                    id: row.get(0)?,
                    chunk_kind: row.get(1)?,
                    content: row.get(2)?,
                    project: row.get(3)?,
                    session_id: row.get(4)?,
                    role: row.get(5)?,
                    tool_name: row.get(6)?,
                    tool_id: row.get(7)?,
                    is_error: row.get(8)?,
                    timestamp: row.get(9)?,
                    score: row.get::<_, f64>(10)?.abs(),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Vector similarity search using sqlite-vec.
    #[allow(clippy::cast_possible_wrap)]
    pub fn search_vector(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let embedding_bytes = embedding_to_bytes(query_embedding);

        let mut stmt = self.conn.prepare(
            r"
            SELECT v.id, v.distance, d.chunk_kind, d.content, d.project, d.session_id,
                   d.role, d.tool_name, d.tool_id, d.is_error, d.timestamp
            FROM documents_vec v
            JOIN documents d ON d.id = v.id
            WHERE embedding MATCH ?1
            ORDER BY distance
            LIMIT ?2
            ",
        )?;

        let results = stmt
            .query_map(params![embedding_bytes, limit as i64], |row| {
                Ok(SearchResult {
                    id: row.get(0)?,
                    score: 1.0 - row.get::<_, f64>(1)?, // Convert distance to similarity
                    chunk_kind: row.get(2)?,
                    content: row.get(3)?,
                    project: row.get(4)?,
                    session_id: row.get(5)?,
                    role: row.get(6)?,
                    tool_name: row.get(7)?,
                    tool_id: row.get(8)?,
                    is_error: row.get(9)?,
                    timestamp: row.get(10)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Hybrid search combining FTS5 and vector search with RRF fusion.
    pub fn search_hybrid(
        &self,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // Get more results from each method for better fusion
        let fetch_limit = limit * 3;

        let fts_results = self.search_fts(query, fetch_limit)?;
        let vec_results = self.search_vector(query_embedding, fetch_limit)?;

        // RRF fusion
        let fused = rrf_fusion(&fts_results, &vec_results, limit);
        Ok(fused)
    }

    /// Adaptive hybrid search - routes based on BM25/vector result agreement.
    ///
    /// When BM25 and vector search agree on top results, uses hybrid RRF fusion.
    /// When they disagree and BM25 has high confidence, trusts BM25 alone.
    /// This prevents low-quality vector results from diluting good BM25 matches.
    pub fn search_hybrid_adaptive(
        &self,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let fetch_limit = limit * 3;

        // Run both searches
        let bm25_results = self.search_fts(query, fetch_limit)?;
        let vector_results = self.search_vector(query_embedding, fetch_limit)?;

        // Check if top results agree
        let top1_match = !bm25_results.is_empty()
            && !vector_results.is_empty()
            && bm25_results[0].id == vector_results[0].id;

        // Check BM25 confidence: high score with clear gap to #2
        let bm25_confident = bm25_results.len() >= 2
            && bm25_results[0].score > 10.0
            && (bm25_results[0].score - bm25_results[1].score) > 3.0;

        // Route decision
        if top1_match {
            // Agreement - hybrid fusion is safe
            let fused = rrf_fusion(&bm25_results, &vector_results, limit);
            Ok(fused)
        } else if bm25_confident {
            // Disagreement + BM25 confident - trust BM25 alone
            Ok(bm25_results.into_iter().take(limit).collect())
        } else {
            // Uncertain - hedge with hybrid fusion
            let fused = rrf_fusion(&bm25_results, &vector_results, limit);
            Ok(fused)
        }
    }

    /// Gets all messages for a session (for context display).
    pub fn get_session_messages(&self, session_id: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, chunk_kind, content, project, session_id,
                   role, tool_name, tool_id, is_error, timestamp
            FROM documents
            WHERE session_id = ?1
            ORDER BY timestamp ASC, rowid ASC
            ",
        )?;

        let results = stmt
            .query_map(params![session_id], |row| {
                Ok(SearchResult {
                    id: row.get(0)?,
                    chunk_kind: row.get(1)?,
                    content: row.get(2)?,
                    project: row.get(3)?,
                    session_id: row.get(4)?,
                    role: row.get(5)?,
                    tool_name: row.get(6)?,
                    tool_id: row.get(7)?,
                    is_error: row.get(8)?,
                    timestamp: row.get(9)?,
                    score: 0.0,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Regex search (full table scan).
    pub fn search_regex(
        &self,
        pattern: &str,
        limit: usize,
        ignore_case: bool,
    ) -> Result<Vec<SearchResult>> {
        let regex = if ignore_case {
            regex::Regex::new(&format!("(?i){pattern}"))?
        } else {
            regex::Regex::new(pattern)?
        };

        let mut stmt = self.conn.prepare(
            r"
            SELECT id, chunk_kind, content, project, session_id,
                   role, tool_name, tool_id, is_error, timestamp
            FROM documents
            ",
        )?;

        let mut results = Vec::new();
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let content: String = row.get(2)?;
            if regex.is_match(&content) {
                results.push(SearchResult {
                    id: row.get(0)?,
                    chunk_kind: row.get(1)?,
                    content,
                    project: row.get(3)?,
                    session_id: row.get(4)?,
                    role: row.get(5)?,
                    tool_name: row.get(6)?,
                    tool_id: row.get(7)?,
                    is_error: row.get(8)?,
                    timestamp: row.get(9)?,
                    score: 1.0,
                });

                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    /// Returns the file size of the database in bytes.
    pub fn file_size(&self) -> Result<u64> {
        let path: String = self
            .conn
            .query_row("PRAGMA database_list", [], |row| row.get(2))?;

        if path.is_empty() || path == ":memory:" {
            return Ok(0);
        }

        Ok(std::fs::metadata(path)?.len())
    }

    /// Checks if embeddings exist in the database.
    pub fn has_embeddings(&self) -> Result<bool> {
        Ok(self.embedding_count()? > 0)
    }
}

/// Converts an f32 slice to bytes for sqlite-vec.
fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Reciprocal Rank Fusion for combining search results.
fn rrf_fusion(
    fts_results: &[SearchResult],
    vec_results: &[SearchResult],
    limit: usize,
) -> Vec<SearchResult> {
    use std::collections::HashMap;

    const K: f64 = 60.0;

    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut results_map: HashMap<String, SearchResult> = HashMap::new();

    // Score FTS results
    for (rank, result) in fts_results.iter().enumerate() {
        let rrf_score = 1.0 / (K + rank as f64 + 1.0);
        *scores.entry(result.id.clone()).or_insert(0.0) += rrf_score;
        results_map
            .entry(result.id.clone())
            .or_insert_with(|| result.clone());
    }

    // Score vector results
    for (rank, result) in vec_results.iter().enumerate() {
        let rrf_score = 1.0 / (K + rank as f64 + 1.0);
        *scores.entry(result.id.clone()).or_insert(0.0) += rrf_score;
        results_map
            .entry(result.id.clone())
            .or_insert_with(|| result.clone());
    }

    // Sort by combined score
    let mut scored_ids: Vec<_> = scores.into_iter().collect();
    scored_ids.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take top results and update scores
    scored_ids
        .into_iter()
        .take(limit)
        .filter_map(|(id, score)| {
            results_map.remove(&id).map(|mut r| {
                r.score = score;
                r
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_doc(_id: &str, content: &str) -> Document {
        Document::new(
            ChunkKind::Message,
            content.to_string(),
            PathBuf::from("/test"),
        )
    }

    #[test]
    fn test_database_create() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.document_count().unwrap(), 0);
    }

    #[test]
    fn test_insert_and_count() {
        let mut db = Database::open_in_memory().unwrap();
        let docs = vec![make_doc("1", "hello world"), make_doc("2", "goodbye world")];
        db.insert_documents(&docs).unwrap();
        assert_eq!(db.document_count().unwrap(), 2);
    }

    #[test]
    fn test_fts_search() {
        let mut db = Database::open_in_memory().unwrap();
        let docs = vec![
            make_doc("1", "rust programming language"),
            make_doc("2", "python programming language"),
            make_doc("3", "hello world"),
        ];
        db.insert_documents(&docs).unwrap();

        let results = db.search_fts("rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("rust"));
    }

    #[test]
    fn test_rrf_fusion() {
        let fts = vec![
            SearchResult {
                id: "a".to_string(),
                score: 10.0,
                chunk_kind: "message".to_string(),
                content: "test a".to_string(),
                project: None,
                session_id: None,
                role: None,
                tool_name: None,
                tool_id: None,
                is_error: None,
                timestamp: None,
            },
            SearchResult {
                id: "b".to_string(),
                score: 5.0,
                chunk_kind: "message".to_string(),
                content: "test b".to_string(),
                project: None,
                session_id: None,
                role: None,
                tool_name: None,
                tool_id: None,
                is_error: None,
                timestamp: None,
            },
        ];

        let vec = vec![
            SearchResult {
                id: "b".to_string(),
                score: 0.9,
                chunk_kind: "message".to_string(),
                content: "test b".to_string(),
                project: None,
                session_id: None,
                role: None,
                tool_name: None,
                tool_id: None,
                is_error: None,
                timestamp: None,
            },
            SearchResult {
                id: "c".to_string(),
                score: 0.8,
                chunk_kind: "message".to_string(),
                content: "test c".to_string(),
                project: None,
                session_id: None,
                role: None,
                tool_name: None,
                tool_id: None,
                is_error: None,
                timestamp: None,
            },
        ];

        let fused = rrf_fusion(&fts, &vec, 10);

        // "b" appears in both, should have highest score
        assert_eq!(fused[0].id, "b");
        assert!(fused[0].score > fused[1].score);
    }
}
