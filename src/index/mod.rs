//! Full-text search index implementations.
//!
//! This module provides the [`BM25Index`] for searching conversation content
//! using the BM25 ranking algorithm, backed by Tantivy.

mod bm25;

pub use bm25::{BM25Index, SearchResult};
