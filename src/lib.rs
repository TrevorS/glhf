//! # glhf
//!
//! A library for indexing and searching Claude Code conversation history.
//!
//! This crate provides hybrid search (FTS5 + semantic) over the conversation data
//! stored in `~/.claude` by Claude Code. It parses JSONL conversation files,
//! extracts messages, generates embeddings, and indexes them in SQLite for fast retrieval.
//!
//! ## Quick Start
//!
//! ```no_run
//! use glhf::db::Database;
//! use glhf::embed::Embedder;
//! use glhf::ingest;
//!
//! // Ingest all conversation files
//! let documents = ingest::ingest_all().unwrap();
//!
//! // Create database and embedder
//! let mut db = Database::open(std::path::Path::new("/tmp/glhf.db")).unwrap();
//! let embedder = Embedder::new().unwrap();
//!
//! // Insert documents
//! db.insert_documents(&documents).unwrap();
//!
//! // Generate and insert embeddings
//! let contents: Vec<String> = documents.iter().map(|d| d.content.clone()).collect();
//! let embeddings = embedder.embed_documents(&contents).unwrap();
//! let embedding_pairs: Vec<_> = documents.iter()
//!     .zip(embeddings.iter())
//!     .map(|(d, e)| (d.id.as_str(), e.as_slice()))
//!     .collect();
//! db.insert_embeddings(&embedding_pairs).unwrap();
//!
//! // Hybrid search
//! let query = "rust error handling";
//! let query_embedding = embedder.embed_query(query).unwrap();
//! let results = db.search_hybrid(query, &query_embedding, 10).unwrap();
//! for result in results {
//!     println!("{}: {}", result.score, result.content);
//! }
//! ```

pub mod commands;
pub mod config;
pub mod db;
pub mod embed;
pub mod error;
pub mod ingest;
pub mod models;

pub use error::{Error, Result};
pub use models::document::{ChunkKind, Document};
