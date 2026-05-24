# Architecture

## How hybrid search works

glhf runs every query through two independent search systems and combines their results:

1. **FTS5 (BM25)** — SQLite full-text search. Finds documents containing the query keywords, ranked by term frequency and document length. Fast, precise for keyword matches, but misses paraphrases and conceptual similarity.

2. **Vector search (sqlite-vec)** — Cosine similarity between the query embedding and pre-computed document embeddings. Finds semantically related content even without shared keywords, but can return noise when the embedding model over-generalizes.

3. **Convex combination fusion** — Scores from both systems are min-max normalized to [0, 1], then combined as `score = 0.75 * fts_score + 0.25 * vec_score`. Documents appearing in either result set are included in the final ranking.

## Why this architecture

FTS and vector search find **largely disjoint documents** — on real queries against 77K conversation docs, the two systems share only 0-6 results out of 20+20 candidates. They're genuinely complementary:

- FTS finds exact keyword matches (tool commands, error messages, file paths)
- Vector search finds paraphrases and conceptual matches ("authentication setup" → "how to configure JWT tokens")

Hybrid search via score fusion captures both, improving recall without sacrificing precision.

## Embedding model: potion-base-32M

The embedding model is [potion-base-32M](https://huggingface.co/minishlab/potion-base-32M) from MinishLab's [model2vec](https://github.com/MinishLab/model2vec) family, loaded via the [model2vec-rs](https://github.com/MinishLab/model2vec-rs) Rust crate.

It's a **static embedding model** — no transformer inference at runtime. It tokenizes text, looks up pre-computed token embeddings in a table, and averages them. This makes it extremely fast (~4600 docs/sec on CPU, 0.01ms per query) at the cost of some retrieval quality compared to transformer-based models.

Model2Vec distills sentence transformers into static token-lookup tables via PCA and Zipf weighting. See: Tulkens & van Dongen, [Model2Vec: Fast State-of-the-Art Static Embeddings](https://github.com/MinishLab/model2vec), 2024.

### What we evaluated

| Model | Type | Dims | Speed | Hybrid hit@5 |
|-------|------|------|-------|-------------|
| **potion-base-32M** | static | 512 | 4600 docs/s | **27/27** |
| potion-retrieval-32M | static | 512 | 4600 docs/s | 26/27 |
| potion-code-16M | static | 256 | 3100 docs/s | 23/27 |
| static-retrieval-mrl-en-v1 | static | 1024 | 2400 docs/s | 25/27 |
| snowflake-arctic-embed-xs | ONNX int8 | 384 | 19 docs/s | — |
| bge-small-en-v1.5 | ONNX int8 | 384 | 19 docs/s | — |

potion-base-32M achieves perfect hybrid hit@5 on our 27-query synthetic eval while being 240x faster than the best ONNX alternative. Ensembling multiple models provided no benefit — they miss the same queries.

## Fusion: convex combination vs RRF

We switched from Reciprocal Rank Fusion (RRF) to convex combination (CC) based on evaluation against real retrieval datasets.

**RRF** (`score = Σ 1/(k + rank)`) uses rank positions and ignores raw scores. We found it **performs worse than FTS alone** on our data — 0.6165 MRR vs 0.6631 for FTS-only. RRF over-weights noisy vector results because it treats rank #1 with 99% confidence the same as rank #1 with 51% confidence.

**CC** (`score = α * norm_fts + (1-α) * norm_vec`) uses min-max normalized scores and preserves score magnitude. With α=0.75, it beats both FTS-only (+1.2% MRR) and RRF (+8.9% MRR).

### Evaluation results

Tested on 1000 queries across [StackOverflow-QA](https://huggingface.co/datasets/CoIR-Retrieval/stackoverflow-qa-queries-corpus) and [CodeFeedback-MT](https://huggingface.co/datasets/CoIR-Retrieval/codefeedback-mt-queries-corpus):

| Strategy | MRR | Hit@1 | Hit@5 | Hit@10 |
|----------|-----|-------|-------|--------|
| FTS only | 0.6631 | 0.6090 | 0.7280 | 0.7630 |
| Vector only | 0.5391 | 0.4810 | 0.6090 | 0.6620 |
| RRF k=60 | 0.6165 | 0.5580 | 0.6930 | 0.7350 |
| **CC α=0.75** | **0.6714** | **0.6170** | **0.7380** | **0.7740** |

Alpha was tuned by sweeping 0.0 to 1.0 in 0.05 steps. The peak region is a broad plateau from 0.65 to 0.80 with <0.3% variance.

## What we tried and rejected

| Approach | Why rejected |
|----------|-------------|
| **ONNX transformer models** (arctic-embed-xs, MiniLM-L6-v2, bge-small) | 240x slower than static embeddings (19 docs/s vs 4600). Full reindex: 96 min vs 24 sec. |
| **Model ensembles** (combining 2-3 embedding models) | No recall improvement — models miss the same queries. Doubles embedding time for zero benefit. |
| **int4 ONNX quantization** | Actually *slower* than int8 on CPU due to ORT's MatMulNBits kernel overhead ([ORT #23004](https://github.com/microsoft/onnxruntime/issues/23004)). |
| **Percentile filtering of vector results** | CC α=0.5 with bottom-30% filtering performed the same as without. Noise isn't concentrated at the score tail. |
| **Query-length FTS weighting** | Replaced by the fixed α=0.75 in CC. Alpha sweep showed per-query adaptation provides <0.3% improvement over a fixed weight. |
| **RRF fusion** | Worse than FTS-only (0.6165 vs 0.6631 MRR). RRF discards score magnitude, so noisy vector results drag down strong FTS matches. |

## References

- Bruch, Gai, Ingber. ["An Analysis of Fusion Functions for Hybrid Retrieval."](https://arxiv.org/abs/2210.11934) ACM TOIS 42(1), 2023.
- Tulkens & van Dongen. [Model2Vec: Fast State-of-the-Art Static Embeddings.](https://github.com/MinishLab/model2vec) 2024.
- Chen et al. ["Sticking to the Mean: Detecting Sticky Tokens in Text Embedding Models."](https://arxiv.org/abs/2507.18171) ACL 2025.
