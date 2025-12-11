//! Embeddings generation using model2vec.
//!
//! This module provides a wrapper around model2vec for generating text embeddings
//! using the Potion-base-32M model.

use crate::error::Error;
use crate::Result;
use model2vec_rs::model::StaticModel;

const MODEL_ID: &str = "minishlab/potion-base-32M";

/// Wrapper around model2vec for generating text embeddings.
pub struct Embedder {
    model: StaticModel,
}

impl Embedder {
    /// Creates a new embedder with Potion-base-32M.
    ///
    /// This will download the model on first use (~130MB).
    pub fn new() -> Result<Self> {
        let model = StaticModel::from_pretrained(MODEL_ID, None, None, None).map_err(|e| {
            Error::Embedding {
                message: format!("Failed to load {MODEL_ID}: {e}"),
            }
        })?;

        Ok(Self { model })
    }

    /// Creates a new embedder (same as new, model2vec doesn't show progress).
    pub fn new_quiet() -> Result<Self> {
        Self::new()
    }

    /// Embeds a single query string.
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        let embeddings = self.model.encode(&[query.to_string()]);
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| Error::Embedding {
                message: "No embedding returned".to_string(),
            })
    }

    /// Embeds multiple documents in a batch.
    ///
    /// Returns embeddings in the same order as the input documents.
    pub fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f32>>> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        Ok(self.model.encode(documents))
    }

    /// Embeds documents in batches with progress reporting.
    ///
    /// The callback is called after each batch with (completed, total).
    pub fn embed_documents_with_progress<F>(
        &self,
        documents: &[String],
        batch_size: usize,
        on_progress: F,
    ) -> Result<Vec<Vec<f32>>>
    where
        F: Fn(usize, usize),
    {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let total = documents.len();
        let mut all_embeddings = Vec::with_capacity(total);
        let mut completed = 0;

        for chunk in documents.chunks(batch_size) {
            let chunk_vec: Vec<String> = chunk.to_vec();
            let embeddings = self.model.encode(&chunk_vec);
            all_embeddings.extend(embeddings);

            completed += chunk.len();
            on_progress(completed, total);
        }

        Ok(all_embeddings)
    }

    /// Returns the embedding dimension (512 for Potion-base-32M).
    #[must_use]
    pub const fn dimension(&self) -> usize {
        512
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires model download"]
    fn test_embed_query() {
        let embedder = Embedder::new_quiet().unwrap();
        let embedding = embedder.embed_query("hello world").unwrap();
        assert_eq!(embedding.len(), 512);
    }

    #[test]
    #[ignore = "Requires model download"]
    fn test_embed_documents() {
        let embedder = Embedder::new_quiet().unwrap();
        let docs = vec!["hello world".to_string(), "goodbye world".to_string()];
        let embeddings = embedder.embed_documents(&docs).unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 512);
        assert_eq!(embeddings[1].len(), 512);
    }

    #[test]
    #[ignore = "Requires model download"]
    fn test_embed_empty() {
        let embedder = Embedder::new_quiet().unwrap();
        let embeddings = embedder.embed_documents(&[]).unwrap();
        assert!(embeddings.is_empty());
    }
}
