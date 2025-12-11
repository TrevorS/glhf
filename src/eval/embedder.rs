//! Swappable embedding model backend.

use crate::error::Error;
use crate::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use model2vec_rs::model::StaticModel;

/// Available embedding models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelChoice {
    // FastEmbed ONNX models
    /// all-MiniLM-L6-v2 (full precision).
    AllMiniLML6V2,
    /// all-MiniLM-L6-v2 quantized.
    AllMiniLML6V2Q,
    /// all-MiniLM-L12-v2 (full precision).
    AllMiniLML12V2,
    /// BGE-small-en-v1.5 (full precision).
    BGESmallENV15,
    /// BGE-small-en-v1.5 quantized.
    BGESmallENV15Q,
    /// BGE-base-en-v1.5 (full precision).
    BGEBaseENV15,
    /// BGE-base-en-v1.5 quantized.
    BGEBaseENV15Q,

    // Model2Vec static embedding models
    /// Potion-base-2M (1.8M params, 256d).
    PotionBase2M,
    /// Potion-base-4M (3.7M params, 256d).
    PotionBase4M,
    /// Potion-base-8M (7.5M params, 256d).
    PotionBase8M,
    /// Potion-base-32M (32.3M params, 256d).
    PotionBase32M,
    /// Potion-retrieval-32M (retrieval-optimized, 256d).
    PotionRetrieval32M,
}

impl ModelChoice {
    /// Get the model name for display.
    pub fn name(&self) -> &'static str {
        match self {
            Self::AllMiniLML6V2 => "all-MiniLM-L6-v2",
            Self::AllMiniLML6V2Q => "all-MiniLM-L6-v2-Q",
            Self::AllMiniLML12V2 => "all-MiniLM-L12-v2",
            Self::BGESmallENV15 => "BGE-small-en-v1.5",
            Self::BGESmallENV15Q => "BGE-small-en-v1.5-Q",
            Self::BGEBaseENV15 => "BGE-base-en-v1.5",
            Self::BGEBaseENV15Q => "BGE-base-en-v1.5-Q",
            Self::PotionBase2M => "potion-base-2M",
            Self::PotionBase4M => "potion-base-4M",
            Self::PotionBase8M => "potion-base-8M",
            Self::PotionBase32M => "potion-base-32M",
            Self::PotionRetrieval32M => "potion-retrieval-32M",
        }
    }

    /// Get the embedding dimension for this model.
    pub const fn dimension(&self) -> usize {
        match self {
            Self::AllMiniLML6V2
            | Self::AllMiniLML6V2Q
            | Self::AllMiniLML12V2
            | Self::BGESmallENV15
            | Self::BGESmallENV15Q => 384,
            Self::BGEBaseENV15 | Self::BGEBaseENV15Q => 768,
            Self::PotionBase2M
            | Self::PotionBase4M
            | Self::PotionBase8M
            | Self::PotionBase32M
            | Self::PotionRetrieval32M => 256,
        }
    }

    /// Check if this is a `Model2Vec` model.
    pub const fn is_model2vec(&self) -> bool {
        matches!(
            self,
            Self::PotionBase2M
                | Self::PotionBase4M
                | Self::PotionBase8M
                | Self::PotionBase32M
                | Self::PotionRetrieval32M
        )
    }

    /// Get the `HuggingFace` model ID for `Model2Vec` models.
    pub fn hf_model_id(&self) -> Option<&'static str> {
        match self {
            Self::PotionBase2M => Some("minishlab/potion-base-2M"),
            Self::PotionBase4M => Some("minishlab/potion-base-4M"),
            Self::PotionBase8M => Some("minishlab/potion-base-8M"),
            Self::PotionBase32M => Some("minishlab/potion-base-32M"),
            Self::PotionRetrieval32M => Some("minishlab/potion-retrieval-32M"),
            _ => None,
        }
    }

    /// Get all available models.
    pub fn all() -> &'static [ModelChoice] {
        &[
            Self::AllMiniLML6V2,
            Self::AllMiniLML6V2Q,
            Self::AllMiniLML12V2,
            Self::BGESmallENV15,
            Self::BGESmallENV15Q,
            Self::BGEBaseENV15,
            Self::BGEBaseENV15Q,
            Self::PotionBase2M,
            Self::PotionBase4M,
            Self::PotionBase8M,
            Self::PotionBase32M,
            Self::PotionRetrieval32M,
        ]
    }

    /// Get recommended models for quick comparison.
    pub fn recommended() -> &'static [ModelChoice] {
        &[
            Self::AllMiniLML6V2,
            Self::AllMiniLML6V2Q,
            Self::PotionBase8M,
            Self::PotionBase32M,
        ]
    }
}

impl From<ModelChoice> for Option<EmbeddingModel> {
    fn from(choice: ModelChoice) -> Self {
        match choice {
            ModelChoice::AllMiniLML6V2 => Some(EmbeddingModel::AllMiniLML6V2),
            ModelChoice::AllMiniLML6V2Q => Some(EmbeddingModel::AllMiniLML6V2Q),
            ModelChoice::AllMiniLML12V2 => Some(EmbeddingModel::AllMiniLML12V2),
            ModelChoice::BGESmallENV15 => Some(EmbeddingModel::BGESmallENV15),
            ModelChoice::BGESmallENV15Q => Some(EmbeddingModel::BGESmallENV15Q),
            ModelChoice::BGEBaseENV15 => Some(EmbeddingModel::BGEBaseENV15),
            ModelChoice::BGEBaseENV15Q => Some(EmbeddingModel::BGEBaseENV15Q),
            // Model2Vec models don't use fastembed
            _ => None,
        }
    }
}

/// Trait for embedding backends.
pub trait EmbedderBackend: Send + Sync {
    /// Get the model name.
    fn name(&self) -> &str;

    /// Get the embedding dimension.
    fn dimension(&self) -> usize;

    /// Embed a batch of texts.
    fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Embed a single text.
    fn embed_one(&mut self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()])?;
        results.into_iter().next().ok_or_else(|| Error::Embedding {
            message: "No embedding returned".to_string(),
        })
    }
}

/// ONNX-based embedder using fastembed.
pub struct FastEmbedBackend {
    model: TextEmbedding,
    choice: ModelChoice,
}

impl FastEmbedBackend {
    /// Create a new embedder with the specified model.
    pub fn new(choice: ModelChoice) -> Result<Self> {
        let fastembed_model: Option<EmbeddingModel> = choice.into();
        let fastembed_model = fastembed_model.ok_or_else(|| Error::Embedding {
            message: format!("{} is not a fastembed model", choice.name()),
        })?;

        let model = TextEmbedding::try_new(
            InitOptions::new(fastembed_model).with_show_download_progress(false),
        )
        .map_err(|e| Error::Embedding {
            message: format!("Failed to load model {}: {}", choice.name(), e),
        })?;

        Ok(Self { model, choice })
    }
}

impl EmbedderBackend for FastEmbedBackend {
    fn name(&self) -> &str {
        self.choice.name()
    }

    fn dimension(&self) -> usize {
        self.choice.dimension()
    }

    fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        self.model.embed(texts, None).map_err(|e| Error::Embedding {
            message: e.to_string(),
        })
    }
}

/// `Model2Vec` static embedding backend.
pub struct Model2VecBackend {
    model: StaticModel,
    choice: ModelChoice,
}

impl Model2VecBackend {
    /// Create a new `Model2Vec` embedder.
    pub fn new(choice: ModelChoice) -> Result<Self> {
        let model_id = choice.hf_model_id().ok_or_else(|| Error::Embedding {
            message: format!("{} is not a Model2Vec model", choice.name()),
        })?;

        let model = StaticModel::from_pretrained(model_id, None, None, None).map_err(|e| {
            Error::Embedding {
                message: format!("Failed to load model {}: {}", choice.name(), e),
            }
        })?;

        Ok(Self { model, choice })
    }
}

impl EmbedderBackend for Model2VecBackend {
    fn name(&self) -> &str {
        self.choice.name()
    }

    fn dimension(&self) -> usize {
        self.choice.dimension()
    }

    fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // model2vec-rs encode() takes &[String] and returns Vec<Vec<f32>> directly
        Ok(self.model.encode(texts))
    }
}

/// Create the appropriate backend for a model choice.
pub fn create_backend(choice: ModelChoice) -> Result<Box<dyn EmbedderBackend>> {
    if choice.is_model2vec() {
        Ok(Box::new(Model2VecBackend::new(choice)?))
    } else {
        Ok(Box::new(FastEmbedBackend::new(choice)?))
    }
}
