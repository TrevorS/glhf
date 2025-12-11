//! Sophisticated benchmark comparing embedding models using synthetic relevance judgments.
//!
//! Compares potion-base-32M (512d) vs potion-multilingual-128M (256d) using:
//! - Self-retrieval: Query with doc prefix, expect that doc to rank high
//! - Session coherence: Query from session S, expect other S docs to rank high
//! - Keyword matching: Query with distinctive term, expect containing doc to rank high
//!
//! Metrics: MRR (Mean Reciprocal Rank), Hit Rate@k

// Benchmark binary - relax pedantic lints for readability
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cloned_ref_to_slice_refs)]

use std::collections::HashMap;
use std::time::Instant;

use anyhow::Result;
use model2vec_rs::model::StaticModel;
use rusqlite::Connection;

const MODEL_A_ID: &str = "minishlab/potion-base-32M";
const MODEL_B_ID: &str = "minishlab/potion-multilingual-128M";

// Test case counts
const SELF_RETRIEVAL_COUNT: usize = 100;
const SESSION_COHERENCE_COUNT: usize = 50;
const KEYWORD_MATCH_COUNT: usize = 50;

// Distinctive keywords to test
const KEYWORDS: &[&str] = &[
    "sqlite-vec",
    "model2vec",
    "rusqlite",
    "thiserror",
    "clippy",
    "cargo test",
    "git push",
    "npm install",
    "TypeError",
    "ENOENT",
];

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║          SOPHISTICATED EMBEDDING MODEL BENCHMARK                           ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");

    // Load database
    let db_path = glhf::config::database_path()?;
    if !db_path.exists() {
        anyhow::bail!("Database not found. Run 'glhf index' first.");
    }
    let conn = Connection::open(&db_path)?;

    // Load documents with session info
    println!("Loading documents from database...");
    let docs = load_documents(&conn)?;
    println!("  Loaded {} documents\n", docs.len());

    if docs.len() < 500 {
        println!(
            "  Warning: Small corpus ({} docs). Results may be noisy.\n",
            docs.len()
        );
    }

    // Load models (should be cached now)
    println!("Loading models...");
    let start = Instant::now();
    let model_a = StaticModel::from_pretrained(MODEL_A_ID, None, None, None)
        .map_err(|e| anyhow::anyhow!("Failed to load Model A: {e}"))?;
    println!("  Model A: {:.2}s", start.elapsed().as_secs_f64());

    let start = Instant::now();
    let model_b = StaticModel::from_pretrained(MODEL_B_ID, None, None, None)
        .map_err(|e| anyhow::anyhow!("Failed to load Model B: {e}"))?;
    println!("  Model B: {:.2}s\n", start.elapsed().as_secs_f64());

    // Embed all documents
    let doc_texts: Vec<String> = docs.iter().map(|d| d.content.clone()).collect();

    println!("Embedding {} documents...", docs.len());
    let start = Instant::now();
    let embeddings_a = model_a.encode(&doc_texts);
    let time_a = start.elapsed();
    println!(
        "  Model A: {:.2}s ({} dims, {:.0} docs/sec)",
        time_a.as_secs_f64(),
        embeddings_a.first().map_or(0, Vec::len),
        docs.len() as f64 / time_a.as_secs_f64()
    );

    let start = Instant::now();
    let embeddings_b = model_b.encode(&doc_texts);
    let time_b = start.elapsed();
    println!(
        "  Model B: {:.2}s ({} dims, {:.0} docs/sec)\n",
        time_b.as_secs_f64(),
        embeddings_b.first().map_or(0, Vec::len),
        docs.len() as f64 / time_b.as_secs_f64()
    );

    // Generate test cases
    println!("Generating test cases...");
    let self_retrieval_cases = generate_self_retrieval_cases(&docs, SELF_RETRIEVAL_COUNT);
    let session_cases = generate_session_coherence_cases(&docs, SESSION_COHERENCE_COUNT);
    let keyword_cases = generate_keyword_cases(&docs, KEYWORD_MATCH_COUNT);

    println!("  Self-retrieval: {} cases", self_retrieval_cases.len());
    println!("  Session coherence: {} cases", session_cases.len());
    println!("  Keyword match: {} cases\n", keyword_cases.len());

    // Run benchmarks
    println!("═══════════════════════════════════════════════════════════════════════════════");
    println!("SELF-RETRIEVAL TEST");
    println!("Query = first 50 chars of doc. Expected: that doc ranks #1.\n");

    let (results_a_self, results_b_self) = run_self_retrieval_benchmark(
        &self_retrieval_cases,
        &docs,
        &model_a,
        &model_b,
        &embeddings_a,
        &embeddings_b,
    );
    print_comparison("Self-Retrieval", &results_a_self, &results_b_self);

    println!("\n═══════════════════════════════════════════════════════════════════════════════");
    println!("SESSION COHERENCE TEST");
    println!("Query = doc from session S. Expected: other session S docs rank high.\n");

    let (results_a_session, results_b_session) = run_session_coherence_benchmark(
        &session_cases,
        &docs,
        &model_a,
        &model_b,
        &embeddings_a,
        &embeddings_b,
    );
    print_comparison("Session Coherence", &results_a_session, &results_b_session);

    println!("\n═══════════════════════════════════════════════════════════════════════════════");
    println!("KEYWORD MATCH TEST");
    println!("Query = distinctive keyword. Expected: doc containing keyword ranks high.\n");

    let (results_a_keyword, results_b_keyword) = run_keyword_benchmark(
        &keyword_cases,
        &docs,
        &model_a,
        &model_b,
        &embeddings_a,
        &embeddings_b,
    );
    print_comparison("Keyword Match", &results_a_keyword, &results_b_keyword);

    // Overall summary
    println!("\n╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                           OVERALL SUMMARY                                  ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");

    let avg_mrr_a = (results_a_self.mrr + results_a_session.mrr + results_a_keyword.mrr) / 3.0;
    let avg_mrr_b = (results_b_self.mrr + results_b_session.mrr + results_b_keyword.mrr) / 3.0;

    println!("                          Model A        Model B       Winner");
    println!("                        (base-32M)   (multilingual)");
    println!("───────────────────────────────────────────────────────────────────");
    print_summary_row("Self-retrieval", results_a_self.mrr, results_b_self.mrr);
    print_summary_row(
        "Session coherence",
        results_a_session.mrr,
        results_b_session.mrr,
    );
    print_summary_row(
        "Keyword match",
        results_a_keyword.mrr,
        results_b_keyword.mrr,
    );
    println!("───────────────────────────────────────────────────────────────────");
    print_summary_row("Average MRR", avg_mrr_a, avg_mrr_b);

    println!();
    let diff_pct = ((avg_mrr_b - avg_mrr_a) / avg_mrr_a) * 100.0;
    if diff_pct > 5.0 {
        println!(
            "📈 Recommendation: Model B shows {diff_pct:.1}% improvement. Consider switching."
        );
    } else if diff_pct < -5.0 {
        println!(
            "📉 Recommendation: Model A is {:.1}% better. Stay with current model.",
            -diff_pct
        );
    } else {
        println!("📊 Recommendation: Models are within 5%. Difference is marginal.");
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Data structures
// ─────────────────────────────────────────────────────────────────────────────

struct Document {
    #[allow(dead_code)]
    id: String,
    content: String,
    session_id: Option<String>,
}

struct BenchmarkResults {
    mrr: f64,
    hit_at_1: f64,
    hit_at_5: f64,
    hit_at_10: f64,
}

struct SelfRetrievalCase {
    query: String,
    expected_idx: usize,
}

struct SessionCoherenceCase {
    query_idx: usize,
    expected_indices: Vec<usize>,
}

struct KeywordCase {
    keyword: String,
    expected_idx: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Database loading
// ─────────────────────────────────────────────────────────────────────────────

fn load_documents(conn: &Connection) -> Result<Vec<Document>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, session_id FROM documents
         WHERE content IS NOT NULL AND LENGTH(content) > 100
         ORDER BY RANDOM()
         LIMIT 1000",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(Document {
            id: row.get(0)?,
            content: row.get(1)?,
            session_id: row.get(2)?,
        })
    })?;

    let mut docs = Vec::new();
    for row in rows {
        docs.push(row?);
    }
    Ok(docs)
}

// ─────────────────────────────────────────────────────────────────────────────
// Test case generation
// ─────────────────────────────────────────────────────────────────────────────

fn generate_self_retrieval_cases(docs: &[Document], count: usize) -> Vec<SelfRetrievalCase> {
    docs.iter()
        .enumerate()
        .filter(|(_, d)| d.content.len() > 100)
        .take(count)
        .map(|(idx, d)| {
            // Use first 50 chars as query (natural prefix)
            let query = d.content.chars().take(50).collect::<String>();
            SelfRetrievalCase {
                query,
                expected_idx: idx,
            }
        })
        .collect()
}

fn generate_session_coherence_cases(docs: &[Document], count: usize) -> Vec<SessionCoherenceCase> {
    // Group docs by session
    let mut sessions: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, doc) in docs.iter().enumerate() {
        if let Some(ref session_id) = doc.session_id {
            sessions.entry(session_id.clone()).or_default().push(idx);
        }
    }

    // Find sessions with 5+ docs
    let mut cases = Vec::new();
    for indices in sessions.values() {
        if indices.len() >= 5 && cases.len() < count {
            // Use first doc as query, rest as expected
            let query_idx = indices[0];
            let expected_indices: Vec<usize> = indices[1..].to_vec();
            cases.push(SessionCoherenceCase {
                query_idx,
                expected_indices,
            });
        }
    }
    cases
}

fn generate_keyword_cases(docs: &[Document], count: usize) -> Vec<KeywordCase> {
    let mut cases = Vec::new();

    for keyword in KEYWORDS {
        if cases.len() >= count {
            break;
        }

        // Find a doc containing this keyword
        for (idx, doc) in docs.iter().enumerate() {
            if doc.content.to_lowercase().contains(&keyword.to_lowercase()) {
                cases.push(KeywordCase {
                    keyword: (*keyword).to_string(),
                    expected_idx: idx,
                });
                break;
            }
        }
    }
    cases
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark runners
// ─────────────────────────────────────────────────────────────────────────────

fn run_self_retrieval_benchmark(
    cases: &[SelfRetrievalCase],
    docs: &[Document],
    model_a: &StaticModel,
    model_b: &StaticModel,
    embeddings_a: &[Vec<f32>],
    embeddings_b: &[Vec<f32>],
) -> (BenchmarkResults, BenchmarkResults) {
    let mut ranks_a = Vec::new();
    let mut ranks_b = Vec::new();

    for case in cases {
        // Embed query
        let query_emb_a = model_a.encode(&[case.query.clone()]).remove(0);
        let query_emb_b = model_b.encode(&[case.query.clone()]).remove(0);

        // Find rank of expected doc
        let rank_a = find_rank(embeddings_a, &query_emb_a, case.expected_idx);
        let rank_b = find_rank(embeddings_b, &query_emb_b, case.expected_idx);

        ranks_a.push(rank_a);
        ranks_b.push(rank_b);
    }

    // Show a few examples
    println!("Sample results (first 5):");
    for (i, case) in cases.iter().take(5).enumerate() {
        let query_preview = truncate(&case.query, 40);
        let doc_preview = truncate(&docs[case.expected_idx].content, 30);
        println!("  Q: \"{query_preview}\" → Doc: \"{doc_preview}\"");
        println!("     Rank: A={}, B={}\n", ranks_a[i], ranks_b[i]);
    }

    (calculate_metrics(&ranks_a), calculate_metrics(&ranks_b))
}

fn run_session_coherence_benchmark(
    cases: &[SessionCoherenceCase],
    docs: &[Document],
    model_a: &StaticModel,
    model_b: &StaticModel,
    embeddings_a: &[Vec<f32>],
    embeddings_b: &[Vec<f32>],
) -> (BenchmarkResults, BenchmarkResults) {
    let mut ranks_a = Vec::new();
    let mut ranks_b = Vec::new();

    for case in cases {
        // Use the query doc's content
        let query = docs[case.query_idx].content.clone();
        let query_emb_a = model_a.encode(&[query.clone()]).remove(0);
        let query_emb_b = model_b.encode(&[query]).remove(0);

        // Find best rank among expected docs
        let best_rank_a = case
            .expected_indices
            .iter()
            .map(|&idx| find_rank(embeddings_a, &query_emb_a, idx))
            .min()
            .unwrap_or(1000);
        let best_rank_b = case
            .expected_indices
            .iter()
            .map(|&idx| find_rank(embeddings_b, &query_emb_b, idx))
            .min()
            .unwrap_or(1000);

        ranks_a.push(best_rank_a);
        ranks_b.push(best_rank_b);
    }

    // Show sample
    if !cases.is_empty() {
        println!("Sample results (first 3):");
        for (i, case) in cases.iter().take(3).enumerate() {
            let query_preview = truncate(&docs[case.query_idx].content, 50);
            println!(
                "  Q: \"{}\" (session has {} other docs)",
                query_preview,
                case.expected_indices.len()
            );
            println!("     Best rank: A={}, B={}\n", ranks_a[i], ranks_b[i]);
        }
    }

    (calculate_metrics(&ranks_a), calculate_metrics(&ranks_b))
}

fn run_keyword_benchmark(
    cases: &[KeywordCase],
    docs: &[Document],
    model_a: &StaticModel,
    model_b: &StaticModel,
    embeddings_a: &[Vec<f32>],
    embeddings_b: &[Vec<f32>],
) -> (BenchmarkResults, BenchmarkResults) {
    let mut ranks_a = Vec::new();
    let mut ranks_b = Vec::new();

    for case in cases {
        let query_emb_a = model_a.encode(&[case.keyword.clone()]).remove(0);
        let query_emb_b = model_b.encode(&[case.keyword.clone()]).remove(0);

        let rank_a = find_rank(embeddings_a, &query_emb_a, case.expected_idx);
        let rank_b = find_rank(embeddings_b, &query_emb_b, case.expected_idx);

        ranks_a.push(rank_a);
        ranks_b.push(rank_b);
    }

    // Show sample
    println!("Sample results:");
    for (i, case) in cases.iter().enumerate() {
        let doc_preview = truncate(&docs[case.expected_idx].content, 40);
        println!("  Keyword: \"{}\" → Doc: \"{}\"", case.keyword, doc_preview);
        println!("     Rank: A={}, B={}", ranks_a[i], ranks_b[i]);
    }
    println!();

    (calculate_metrics(&ranks_a), calculate_metrics(&ranks_b))
}

// ─────────────────────────────────────────────────────────────────────────────
// Metrics
// ─────────────────────────────────────────────────────────────────────────────

fn find_rank(embeddings: &[Vec<f32>], query: &[f32], target_idx: usize) -> usize {
    let mut similarities: Vec<(usize, f32)> = embeddings
        .iter()
        .enumerate()
        .map(|(i, emb)| (i, cosine_similarity(emb, query)))
        .collect();

    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    similarities
        .iter()
        .position(|(idx, _)| *idx == target_idx)
        .map_or(embeddings.len(), |p| p + 1)
}

fn calculate_metrics(ranks: &[usize]) -> BenchmarkResults {
    if ranks.is_empty() {
        return BenchmarkResults {
            mrr: 0.0,
            hit_at_1: 0.0,
            hit_at_5: 0.0,
            hit_at_10: 0.0,
        };
    }

    let n = ranks.len() as f64;

    let mrr: f64 = ranks.iter().map(|&r| 1.0 / r as f64).sum::<f64>() / n;
    let hit_at_1 = ranks.iter().filter(|&&r| r <= 1).count() as f64 / n;
    let hit_at_5 = ranks.iter().filter(|&&r| r <= 5).count() as f64 / n;
    let hit_at_10 = ranks.iter().filter(|&&r| r <= 10).count() as f64 / n;

    BenchmarkResults {
        mrr,
        hit_at_1,
        hit_at_5,
        hit_at_10,
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Display
// ─────────────────────────────────────────────────────────────────────────────

fn print_comparison(test_name: &str, a: &BenchmarkResults, b: &BenchmarkResults) {
    println!("Results for {test_name}:");
    println!();
    println!("                    Model A      Model B      Δ");
    println!(
        "  MRR:              {:.3}        {:.3}       {:+.1}%",
        a.mrr,
        b.mrr,
        ((b.mrr - a.mrr) / a.mrr) * 100.0
    );
    println!(
        "  Hit@1:            {:.1}%       {:.1}%      {:+.1}%",
        a.hit_at_1 * 100.0,
        b.hit_at_1 * 100.0,
        (b.hit_at_1 - a.hit_at_1) * 100.0
    );
    println!(
        "  Hit@5:            {:.1}%       {:.1}%      {:+.1}%",
        a.hit_at_5 * 100.0,
        b.hit_at_5 * 100.0,
        (b.hit_at_5 - a.hit_at_5) * 100.0
    );
    println!(
        "  Hit@10:           {:.1}%       {:.1}%      {:+.1}%",
        a.hit_at_10 * 100.0,
        b.hit_at_10 * 100.0,
        (b.hit_at_10 - a.hit_at_10) * 100.0
    );
}

fn print_summary_row(label: &str, a: f64, b: f64) {
    let diff = ((b - a) / a) * 100.0;
    let winner = if diff > 2.0 {
        "B ⬆"
    } else if diff < -2.0 {
        "A ⬆"
    } else {
        "≈"
    };
    println!("{label:20}  {a:.3}        {b:.3}         {winner} ({diff:+.1}%)");
}

fn truncate(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ").replace('\r', "");
    if s.len() <= max_len {
        s
    } else {
        format!("{}...", &s[..max_len.min(s.len())])
    }
}
