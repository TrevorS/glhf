//! Embeddings generation using fastembed.
//!
//! This module provides a wrapper around fastembed for generating text embeddings
//! using the all-MiniLM-L6-v2-Q quantized model with parallel batch processing.

use crate::error::Error;
use crate::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Wrapper around fastembed for generating text embeddings.
pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    /// Creates a new embedder with the quantized model (all-MiniLM-L6-v2-Q).
    ///
    /// Uses quantized model for 2-4x faster inference.
    /// This will download the model on first use (~11MB).
    pub fn new() -> Result<Self> {
        // Use quantized model for speed
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2Q).with_show_download_progress(true),
        )
        .map_err(|e| Error::Embedding {
            message: e.to_string(),
        })?;

        Ok(Self { model })
    }

    /// Creates a new embedder without showing download progress.
    pub fn new_quiet() -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2Q).with_show_download_progress(false),
        )
        .map_err(|e| Error::Embedding {
            message: e.to_string(),
        })?;

        Ok(Self { model })
    }

    /// Embeds a single query string.
    pub fn embed_query(&mut self, query: &str) -> Result<Vec<f32>> {
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
    /// Uses internal batching with default batch size of 256.
    pub fn embed_documents(&mut self, documents: &[String]) -> Result<Vec<Vec<f32>>> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        self.model
            .embed(documents, Some(256))
            .map_err(|e| Error::Embedding {
                message: e.to_string(),
            })
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
                    static LOCAL_MODEL: std::cell::RefCell<Option<TextEmbedding>> =
                        const { std::cell::RefCell::new(None) };
                }

                LOCAL_MODEL.with(|model_cell| {
                    let mut model_ref = model_cell.borrow_mut();
                    if model_ref.is_none() {
                        *model_ref = Some(
                            TextEmbedding::try_new(
                                InitOptions::new(EmbeddingModel::AllMiniLML6V2Q)
                                    .with_show_download_progress(false),
                            )
                            .map_err(|e| Error::Embedding {
                                message: format!("Failed to create thread-local model: {e}"),
                            })?,
                        );
                    }

                    let model = model_ref.as_mut().unwrap();
                    let embeddings =
                        model
                            .embed(chunk, Some(chunk.len()))
                            .map_err(|e| Error::Embedding {
                                message: e.to_string(),
                            })?;

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
        let mut embedder = Embedder::new_quiet().unwrap();
        let embedding = embedder.embed_query("hello world").unwrap();
        assert_eq!(embedding.len(), 384);
    }

    #[test]
    #[ignore] // Requires model download
    fn test_embed_documents() {
        let mut embedder = Embedder::new_quiet().unwrap();
        let docs = vec!["hello world".to_string(), "goodbye world".to_string()];
        let embeddings = embedder.embed_documents(&docs).unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 384);
        assert_eq!(embeddings[1].len(), 384);
    }

    #[test]
    #[ignore] // Requires model download
    fn test_embed_empty() {
        let mut embedder = Embedder::new_quiet().unwrap();
        let embeddings = embedder.embed_documents(&[]).unwrap();
        assert!(embeddings.is_empty());
    }
}
