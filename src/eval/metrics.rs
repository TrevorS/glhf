//! Evaluation metrics for retrieval.

use std::collections::HashSet;
use std::time::Duration;

/// Results for a single query.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Query ID.
    pub query_id: String,
    /// Retrieved document IDs in ranked order.
    pub retrieved_ids: Vec<String>,
    /// Ground truth relevant document IDs.
    pub relevant_ids: HashSet<String>,
    /// Time to embed the query.
    pub embed_time: Duration,
}

impl QueryResult {
    /// Calculate Recall@K - fraction of relevant docs in top K.
    pub fn recall_at_k(&self, k: usize) -> f64 {
        if self.relevant_ids.is_empty() {
            return 0.0;
        }

        let top_k: HashSet<_> = self.retrieved_ids.iter().take(k).collect();
        let hits = self
            .relevant_ids
            .iter()
            .filter(|id| top_k.contains(id))
            .count();

        hits as f64 / self.relevant_ids.len() as f64
    }

    /// Calculate Precision@K - fraction of top K that are relevant.
    pub fn precision_at_k(&self, k: usize) -> f64 {
        let top_k: Vec<_> = self.retrieved_ids.iter().take(k).collect();
        if top_k.is_empty() {
            return 0.0;
        }

        let hits = top_k
            .iter()
            .filter(|id| self.relevant_ids.contains(**id))
            .count();
        hits as f64 / top_k.len() as f64
    }

    /// Calculate MRR (Mean Reciprocal Rank) - 1/rank of first relevant doc.
    pub fn reciprocal_rank(&self) -> f64 {
        for (i, id) in self.retrieved_ids.iter().enumerate() {
            if self.relevant_ids.contains(id) {
                return 1.0 / (i + 1) as f64;
            }
        }
        0.0
    }

    /// Calculate NDCG@K (Normalized Discounted Cumulative Gain).
    pub fn ndcg_at_k(&self, k: usize) -> f64 {
        let dcg = self.dcg_at_k(k);
        let idcg = self.ideal_dcg_at_k(k);

        if idcg == 0.0 {
            0.0
        } else {
            dcg / idcg
        }
    }

    fn dcg_at_k(&self, k: usize) -> f64 {
        self.retrieved_ids
            .iter()
            .take(k)
            .enumerate()
            .filter(|(_, id)| self.relevant_ids.contains(*id))
            .map(|(i, _)| 1.0 / (i + 2) as f64) // log2(i+2) approximated as i+2 for simplicity
            .sum()
    }

    fn ideal_dcg_at_k(&self, k: usize) -> f64 {
        let num_relevant = self.relevant_ids.len().min(k);
        (0..num_relevant).map(|i| 1.0 / (i + 2) as f64).sum()
    }
}

/// Aggregated evaluation metrics across all queries.
#[derive(Debug, Clone)]
pub struct EvalMetrics {
    /// Model name.
    pub model_name: String,
    /// Mean Recall@1.
    pub mean_recall_1: f64,
    /// Mean Recall@5.
    pub mean_recall_5: f64,
    /// Mean Recall@10.
    pub mean_recall_10: f64,
    /// Mean Reciprocal Rank.
    pub mrr: f64,
    /// Mean NDCG@10.
    pub ndcg_10: f64,
    /// Total corpus embedding time.
    pub corpus_embed_time: Duration,
    /// Mean query embedding time.
    pub mean_query_time: Duration,
    /// Documents per second throughput.
    pub throughput: f64,
}

impl EvalMetrics {
    /// Calculate metrics from query results.
    pub fn from_results(
        model_name: &str,
        results: &[QueryResult],
        corpus_embed_time: Duration,
        corpus_size: usize,
    ) -> Self {
        let n = results.len() as f64;

        let mean_recall_1 = results.iter().map(|r| r.recall_at_k(1)).sum::<f64>() / n;
        let mean_recall_5 = results.iter().map(|r| r.recall_at_k(5)).sum::<f64>() / n;
        let mean_recall_10 = results.iter().map(|r| r.recall_at_k(10)).sum::<f64>() / n;
        let mrr = results
            .iter()
            .map(QueryResult::reciprocal_rank)
            .sum::<f64>()
            / n;
        let ndcg_10 = results.iter().map(|r| r.ndcg_at_k(10)).sum::<f64>() / n;

        let total_query_time: Duration = results.iter().map(|r| r.embed_time).sum();
        #[allow(clippy::cast_possible_truncation)]
        let mean_query_time = total_query_time / (results.len() as u32);

        let throughput = corpus_size as f64 / corpus_embed_time.as_secs_f64();

        Self {
            model_name: model_name.to_string(),
            mean_recall_1,
            mean_recall_5,
            mean_recall_10,
            mrr,
            ndcg_10,
            corpus_embed_time,
            mean_query_time,
            throughput,
        }
    }

    /// Print metrics as a formatted table row.
    pub fn print_row(&self) {
        println!(
            "| {:<22} | {:>6.3} | {:>6.3} | {:>7.3} | {:>5.3} | {:>7.3} | {:>8.1} |",
            self.model_name,
            self.mean_recall_1,
            self.mean_recall_5,
            self.mean_recall_10,
            self.mrr,
            self.ndcg_10,
            self.throughput,
        );
    }

    /// Print table header.
    pub fn print_header() {
        println!(
            "| {:<22} | {:>6} | {:>6} | {:>7} | {:>5} | {:>7} | {:>8} |",
            "Model", "R@1", "R@5", "R@10", "MRR", "NDCG@10", "docs/s"
        );
        println!(
            "|{:-<24}|{:-<8}|{:-<8}|{:-<9}|{:-<7}|{:-<9}|{:-<10}|",
            "", "", "", "", "", "", ""
        );
    }
}

/// Full evaluation result including per-query details.
#[derive(Debug)]
pub struct EvalResult {
    /// Aggregated metrics.
    pub metrics: EvalMetrics,
    /// Per-query results.
    pub query_results: Vec<QueryResult>,
}

/// Calculate cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Rank documents by similarity to query embedding.
pub fn rank_by_similarity(
    query_embedding: &[f32],
    doc_embeddings: &[(String, Vec<f32>)],
    top_k: usize,
) -> Vec<(String, f32)> {
    let mut scored: Vec<_> = doc_embeddings
        .iter()
        .map(|(id, emb)| (id.clone(), cosine_similarity(query_embedding, emb)))
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);
    scored
}
