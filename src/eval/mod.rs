//! Embedding model evaluation framework.
//!
//! This module provides tools for evaluating different embedding models
//! against a test corpus with ground-truth relevance labels.

pub mod dataset;
pub mod embedder;
pub mod metrics;

pub use dataset::{Corpus, Document, Query, QuerySet, QueryType};
pub use embedder::{
    create_backend, EmbedderBackend, FastEmbedBackend, Model2VecBackend, ModelChoice,
};
pub use metrics::{cosine_similarity, rank_by_similarity, EvalMetrics, EvalResult, QueryResult};
