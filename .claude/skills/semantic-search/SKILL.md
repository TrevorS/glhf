---
name: semantic-search
description: Implement and debug semantic/vector search with fastembed and sqlite-vec. Use for embedding generation, vector queries, hybrid search, or RRF fusion.
---

# Semantic Search Implementation

## Architecture

```
┌─────────────────────────────────────┐
│         Search Query                │
└─────────────┬───────────────────────┘
              │
    ┌─────────┴─────────┐
    │                   │
┌───▼───┐         ┌─────▼─────┐
│ FTS5  │         │ fastembed │
│(text) │         │(embedding)│
└───┬───┘         └─────┬─────┘
    │                   │
    │              ┌────▼────┐
    │              │sqlite-vec│
    │              │ (vector) │
    │              └────┬────┘
    │                   │
    └─────────┬─────────┘
              │
        ┌─────▼─────┐
        │ RRF Fusion│
        │  (k=60)   │
        └───────────┘
```

## Key Components

### Embedder (src/embed.rs)

```rust
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true)
        )?;
        Ok(Self { model })
    }

    pub fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.model.embed(vec![text], None)?;
        Ok(embeddings.into_iter().next().unwrap())
    }
}
```

### Vector Search (src/db/mod.rs)

```rust
// sqlite-vec query for similarity search
let sql = r#"
    SELECT d.*, v.distance
    FROM documents_vec v
    JOIN documents d ON d.id = v.id
    WHERE v.embedding MATCH ?1
    ORDER BY v.distance
    LIMIT ?2
"#;

// Convert embedding to bytes for sqlite-vec
let embedding_bytes: Vec<u8> = embedding
    .iter()
    .flat_map(|f| f.to_le_bytes())
    .collect();
```

### RRF Fusion

```rust
fn rrf_fusion(fts: &[Result], vec: &[Result], k: f32, limit: usize) -> Vec<Result> {
    let mut scores: HashMap<String, f32> = HashMap::new();

    for (rank, result) in fts.iter().enumerate() {
        *scores.entry(result.id.clone()).or_default() += 1.0 / (k + rank as f32 + 1.0);
    }
    for (rank, result) in vec.iter().enumerate() {
        *scores.entry(result.id.clone()).or_default() += 1.0 / (k + rank as f32 + 1.0);
    }

    // Sort by combined score descending
    let mut results: Vec<_> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results.truncate(limit);
    results
}
```

## Search Modes

| Mode | Flag | Description |
|------|------|-------------|
| Hybrid | `--mode hybrid` | FTS5 + vector with RRF (default) |
| Text | `--mode text` | FTS5 only (fast, keyword match) |
| Semantic | `--mode semantic` | Vector only (meaning-based) |

## Debugging

### Check embedding dimensions
```rust
assert_eq!(embedding.len(), 384); // all-MiniLM-L6-v2
```

### Verify sqlite-vec loaded
```sql
SELECT vec_version();
```

### Test vector distance
```sql
SELECT vec_distance_L2(embedding, ?1) FROM documents_vec LIMIT 1;
```
