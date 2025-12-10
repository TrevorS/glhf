//! Embeddings generation using fastembed.
//!
//! This module provides a wrapper around fastembed for generating text embeddings
//! using the all-MiniLM-L6-v2 model.

use crate::error::Error;
use crate::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// Wrapper around fastembed for generating text embeddings.
pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    /// Creates a new embedder with the default model (all-MiniLM-L6-v2).
    ///
    /// This will download the model on first use (~22MB).
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )
        .map_err(|e| Error::Embedding {
            message: e.to_string(),
        })?;

        Ok(Self { model })
    }

    /// Creates a new embedder without showing download progress.
    pub fn new_quiet() -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
        )
        .map_err(|e| Error::Embedding {
            message: e.to_string(),
        })?;

        Ok(Self { model })
    }

    /// Embeds a single query string.
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        let embeddings = self
            .model
            .embed(vec![query], None)
            .map_err(|e| Error::Embedding {
                message: e.to_string(),
            })?;

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

        self.model
            .embed(documents.to_vec(), None)
            .map_err(|e| Error::Embedding {
                message: e.to_string(),
            })
    }

    /// Embeds documents in batches, calling a callback after each batch.
    ///
    /// This is useful for showing progress during embedding generation.
    pub fn embed_documents_with_progress<F>(
        &self,
        documents: &[String],
        batch_size: usize,
        mut on_progress: F,
    ) -> Result<Vec<Vec<f32>>>
    where
        F: FnMut(usize, usize), // (completed, total)
    {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let total = documents.len();
        let mut all_embeddings = Vec::with_capacity(total);

        for (batch_idx, chunk) in documents.chunks(batch_size).enumerate() {
            let batch_embeddings =
                self.model
                    .embed(chunk.to_vec(), None)
                    .map_err(|e| Error::Embedding {
                        message: e.to_string(),
                    })?;

            all_embeddings.extend(batch_embeddings);

            let completed = ((batch_idx + 1) * batch_size).min(total);
            on_progress(completed, total);
        }

        Ok(all_embeddings)
    }

    /// Returns the embedding dimension (384 for all-MiniLM-L6-v2).
    #[must_use]
    pub const fn dimension(&self) -> usize {
        384
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires model download
    fn test_embed_query() {
        let embedder = Embedder::new_quiet().unwrap();
        let embedding = embedder.embed_query("hello world").unwrap();
        assert_eq!(embedding.len(), 384);
    }

    #[test]
    #[ignore] // Requires model download
    fn test_embed_documents() {
        let embedder = Embedder::new_quiet().unwrap();
        let docs = vec!["hello world".to_string(), "goodbye world".to_string()];
        let embeddings = embedder.embed_documents(&docs).unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 384);
        assert_eq!(embeddings[1].len(), 384);
    }

    #[test]
    #[ignore] // Requires model download
    fn test_embed_empty() {
        let embedder = Embedder::new_quiet().unwrap();
        let embeddings = embedder.embed_documents(&[]).unwrap();
        assert!(embeddings.is_empty());
    }
}
