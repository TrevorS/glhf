//! Embeddings generation using model2vec.
//!
//! This module provides a wrapper around model2vec for generating text embeddings
//! using the Potion-base-32M model with parallel batch processing.

use crate::error::Error;
use crate::Result;
use model2vec_rs::model::StaticModel;
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

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

    /// Embeds documents in batches with parallel processing.
    ///
    /// Uses thread-local embedders for parallel batch processing.
    /// The callback is called periodically with progress updates.
    pub fn embed_documents_with_progress<F>(
        &self,
        documents: &[String],
        batch_size: usize,
        on_progress: F,
    ) -> Result<Vec<Vec<f32>>>
    where
        F: Fn(usize, usize) + Sync, // (completed, total)
    {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let total = documents.len();
        let completed = AtomicUsize::new(0);

        // Process batches in parallel using thread-local embedders
        let chunks: Vec<_> = documents.chunks(batch_size).collect();

        let results: Vec<std::result::Result<Vec<Vec<f32>>, Error>> = chunks
            .par_iter()
            .map(|chunk| {
                // Each thread creates its own embedder instance
                thread_local! {
                    static LOCAL_MODEL: std::cell::RefCell<Option<StaticModel>> =
                        const { std::cell::RefCell::new(None) };
                }

                LOCAL_MODEL.with(|model_cell| {
                    let mut model_ref = model_cell.borrow_mut();
                    if model_ref.is_none() {
                        *model_ref = Some(
                            StaticModel::from_pretrained(MODEL_ID, None, None, None).map_err(
                                |e| Error::Embedding {
                                    message: format!("Failed to create thread-local model: {e}"),
                                },
                            )?,
                        );
                    }

                    let model = model_ref.as_ref().unwrap();
                    let chunk_vec: Vec<String> = chunk.to_vec();
                    let embeddings = model.encode(&chunk_vec);

                    // Update progress
                    let done = completed.fetch_add(chunk.len(), Ordering::Relaxed) + chunk.len();
                    on_progress(done, total);

                    Ok(embeddings)
                })
            })
            .collect();

        // Flatten results preserving order
        let mut all_embeddings = Vec::with_capacity(total);
        for result in results {
            all_embeddings.extend(result?);
        }

        Ok(all_embeddings)
    }

    /// Returns the embedding dimension (256 for Potion-base-32M).
    #[must_use]
    pub const fn dimension(&self) -> usize {
        256
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
        assert_eq!(embedding.len(), 256);
    }

    #[test]
    #[ignore] // Requires model download
    fn test_embed_documents() {
        let embedder = Embedder::new_quiet().unwrap();
        let docs = vec!["hello world".to_string(), "goodbye world".to_string()];
        let embeddings = embedder.embed_documents(&docs).unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 256);
        assert_eq!(embeddings[1].len(), 256);
    }

    #[test]
    #[ignore] // Requires model download
    fn test_embed_empty() {
        let embedder = Embedder::new_quiet().unwrap();
        let embeddings = embedder.embed_documents(&[]).unwrap();
        assert!(embeddings.is_empty());
    }
}
