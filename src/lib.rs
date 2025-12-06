//! # glhf
//!
//! A library for indexing and searching Claude Code conversation history.
//!
//! This crate provides BM25 full-text search over the conversation data stored
//! in `~/.claude` by Claude Code. It parses JSONL conversation files, extracts
//! messages, and indexes them for fast retrieval.
//!
//! ## Quick Start
//!
//! ```no_run
//! use glhf::index::BM25Index;
//! use glhf::ingest;
//!
//! // Ingest all conversation files
//! let documents = ingest::ingest_all().unwrap();
//!
//! // Create and populate the index
//! let index = BM25Index::create(std::path::Path::new("/tmp/index")).unwrap();
//! let mut writer = index.writer().unwrap();
//! index.add_documents(&mut writer, &documents).unwrap();
//! writer.commit().unwrap();
//!
//! // Search
//! let results = index.search("rust error handling", 10).unwrap();
//! for result in results {
//!     println!("{}: {}", result.score, result.content);
//! }
//! ```

pub mod commands;
pub mod config;
pub mod error;
pub mod index;
pub mod ingest;
pub mod models;

pub use error::{Error, Result};
pub use models::document::{ChunkKind, Document};
