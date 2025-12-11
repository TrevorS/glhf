//! Cross-encoder reranking using fastembed.
//!
//! Provides a wrapper around fastembed's `TextRerank` for reranking search results
//! using the `JINARerankerV1TurboEn` model (~150MB).

use crate::{Error, Result};
use fastembed::{RerankInitOptions, RerankerModel, TextRerank};

/// Wrapper around fastembed for cross-encoder reranking.
pub struct Reranker {
    model: TextRerank,
}

impl Reranker {
    /// Creates a new reranker with `JINARerankerV1TurboEn` (~150MB).
    ///
    /// This will download the model on first use.
    pub fn new() -> Result<Self> {
        let model = TextRerank::try_new(
            RerankInitOptions::new(RerankerModel::JINARerankerV1TurboEn)
                .with_show_download_progress(true),
        )
        .map_err(|e| Error::Reranking {
            message: e.to_string(),
        })?;
        Ok(Self { model })
    }

    /// Creates reranker without download progress output.
    pub fn new_quiet() -> Result<Self> {
        let model = TextRerank::try_new(
            RerankInitOptions::new(RerankerModel::JINARerankerV1TurboEn)
                .with_show_download_progress(false),
        )
        .map_err(|e| Error::Reranking {
            message: e.to_string(),
        })?;
        Ok(Self { model })
    }

    /// Reranks documents given a query.
    ///
    /// Returns `(doc_index, score)` pairs sorted by score in descending order.
    pub fn rerank(&mut self, query: &str, documents: &[&str]) -> Result<Vec<(usize, f32)>> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let results = self
            .model
            .rerank(query, documents, false, None)
            .map_err(|e| Error::Reranking {
                message: e.to_string(),
            })?;

        // Results are already sorted by score descending
        Ok(results.iter().map(|r| (r.index, r.score)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires model download
    fn test_rerank_basic() {
        let mut reranker = Reranker::new_quiet().unwrap();
        let query = "What is machine learning?";
        let docs = vec![
            "Machine learning is a subset of artificial intelligence.",
            "The weather today is sunny.",
            "Deep learning uses neural networks.",
        ];

        let results = reranker.rerank(query, &docs).unwrap();

        assert_eq!(results.len(), 3);
        // First result should be the ML-related doc
        assert!(results[0].0 == 0 || results[0].0 == 2);
    }

    #[test]
    #[ignore] // Requires model download
    fn test_rerank_empty() {
        let mut reranker = Reranker::new_quiet().unwrap();
        let results = reranker.rerank("query", &[]).unwrap();
        assert!(results.is_empty());
    }
}
