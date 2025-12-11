//! Embedding model evaluation tests.
//!
//! Run with: cargo test --release eval_models -- --nocapture --ignored

use glhf::eval::{
    create_backend,
    dataset::{Corpus, QuerySet},
    embedder::ModelChoice,
    metrics::{cosine_similarity, rank_by_similarity, EvalMetrics, QueryResult},
};
use std::path::Path;
use std::time::{Duration, Instant};

const CORPUS_PATH: &str = "tests/eval_data/corpus.json";
const QUERIES_PATH: &str = "tests/eval_data/queries.json";

// Expanded eval set (~1000 docs, ~100 queries)
const CORPUS_EXPANDED_PATH: &str = "tests/eval_data/corpus_expanded.json";
const QUERIES_EXPANDED_PATH: &str = "tests/eval_data/queries_expanded.json";

/// Load the evaluation corpus.
fn load_corpus() -> Corpus {
    Corpus::load(Path::new(CORPUS_PATH)).expect("Failed to load corpus")
}

/// Load the test queries.
fn load_queries() -> QuerySet {
    QuerySet::load(Path::new(QUERIES_PATH)).expect("Failed to load queries")
}

/// Run evaluation for a single model.
fn evaluate_model(model: ModelChoice) -> EvalMetrics {
    let corpus = load_corpus();
    let queries = load_queries();

    // Create embedder (auto-selects fastembed or model2vec)
    let mut embedder = create_backend(model).expect("Failed to create embedder");

    // Embed corpus
    let corpus_start = Instant::now();
    let doc_texts: Vec<String> = corpus.documents.iter().map(|d| d.content.clone()).collect();
    let doc_embeddings = embedder
        .embed_batch(&doc_texts)
        .expect("Failed to embed corpus");
    let corpus_embed_time = corpus_start.elapsed();

    // Create (id, embedding) pairs
    let doc_embedding_pairs: Vec<_> = corpus
        .documents
        .iter()
        .zip(doc_embeddings.into_iter())
        .map(|(d, e)| (d.id.clone(), e))
        .collect();

    // Evaluate each query
    let mut query_results = Vec::new();
    for query in &queries.queries {
        let query_start = Instant::now();
        let query_embedding = embedder
            .embed_one(&query.query)
            .expect("Failed to embed query");
        let embed_time = query_start.elapsed();

        // Rank documents by similarity
        let ranked = rank_by_similarity(&query_embedding, &doc_embedding_pairs, 20);
        let retrieved_ids: Vec<String> = ranked.into_iter().map(|(id, _)| id).collect();

        query_results.push(QueryResult {
            query_id: query.id.clone(),
            retrieved_ids,
            relevant_ids: query.relevant_doc_ids.iter().cloned().collect(),
            embed_time,
        });
    }

    EvalMetrics::from_results(
        embedder.name(),
        &query_results,
        corpus_embed_time,
        corpus.documents.len(),
    )
}

#[test]
#[ignore] // Requires model downloads, run explicitly
fn eval_all_models() {
    println!("\n=== Embedding Model Evaluation ===\n");

    let corpus = load_corpus();
    let queries = load_queries();
    println!("Corpus: {} documents", corpus.documents.len());
    println!("Queries: {} test queries\n", queries.queries.len());

    // Models to evaluate - MiniLM + Potion
    let models = &[
        ModelChoice::AllMiniLML6V2,
        ModelChoice::AllMiniLML6V2Q,
        ModelChoice::AllMiniLML12V2,
        ModelChoice::PotionBase2M,
        ModelChoice::PotionBase4M,
        ModelChoice::PotionBase8M,
        ModelChoice::PotionBase32M,
        ModelChoice::PotionRetrieval32M,
    ];

    let mut results = Vec::new();
    for &model in models {
        print!("Evaluating {}...", model.name());
        match create_backend(model) {
            Ok(_) => {
                let metrics = evaluate_model(model);
                println!(" done ({:.1} docs/s)", metrics.throughput);
                results.push(metrics);
            }
            Err(e) => println!(" skipped: {}", e),
        }
    }

    // Print comparison table
    println!("\n=== Results ===\n");
    EvalMetrics::print_header();
    for metrics in &results {
        metrics.print_row();
    }
    println!();

    // Print per-query breakdown for best model
    if let Some(best) = results.iter().max_by(|a, b| {
        a.mrr
            .partial_cmp(&b.mrr)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        println!(
            "Best model by MRR: {} (MRR={:.3})",
            best.model_name, best.mrr
        );
    }

    if let Some(fastest) = results.iter().max_by(|a, b| {
        a.throughput
            .partial_cmp(&b.throughput)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        println!(
            "Fastest model: {} ({:.1} docs/s)",
            fastest.model_name, fastest.throughput
        );
    }
}

#[test]
#[ignore] // Requires model downloads
fn eval_single_model() {
    println!("\n=== Single Model Evaluation ===\n");

    let model = ModelChoice::AllMiniLML6V2Q; // Quick quantized model
    println!("Evaluating {}...\n", model.name());

    let metrics = evaluate_model(model);

    EvalMetrics::print_header();
    metrics.print_row();
    println!();

    println!("Corpus embed time: {:?}", metrics.corpus_embed_time);
    println!("Mean query time: {:?}", metrics.mean_query_time);
    println!("Throughput: {:.1} docs/sec", metrics.throughput);
}

#[test]
#[ignore] // Requires model downloads
fn eval_all_available_models() {
    println!("\n=== Full Model Comparison ===\n");

    let corpus = load_corpus();
    let queries = load_queries();
    println!("Corpus: {} documents", corpus.documents.len());
    println!("Queries: {} test queries\n", queries.queries.len());

    let models = ModelChoice::all();

    let mut results = Vec::new();
    for &model in models {
        println!("Evaluating {}...", model.name());
        match create_backend(model) {
            Ok(_) => {
                let metrics = evaluate_model(model);
                results.push(metrics);
            }
            Err(e) => {
                println!("  Skipped: {}", e);
            }
        }
    }

    println!("\n=== Results ===\n");
    EvalMetrics::print_header();
    for metrics in &results {
        metrics.print_row();
    }
    println!();
}

#[test]
fn test_metrics_calculation() {
    // Unit test for metrics without model loading
    let result = QueryResult {
        query_id: "test".to_string(),
        retrieved_ids: vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ],
        relevant_ids: ["b", "d"].iter().map(|s| s.to_string()).collect(),
        embed_time: Duration::from_millis(10),
    };

    // Recall@1: 0/2 = 0.0 (first result 'a' is not relevant)
    assert!((result.recall_at_k(1) - 0.0).abs() < 0.001);

    // Recall@2: 1/2 = 0.5 (b is in top 2)
    assert!((result.recall_at_k(2) - 0.5).abs() < 0.001);

    // Recall@4: 2/2 = 1.0 (both b and d are in top 4)
    assert!((result.recall_at_k(4) - 1.0).abs() < 0.001);

    // MRR: 1/2 = 0.5 (first relevant doc 'b' is at position 2)
    assert!((result.reciprocal_rank() - 0.5).abs() < 0.001);
}

#[test]
fn test_cosine_similarity() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

    let c = vec![0.0, 1.0, 0.0];
    assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.001);

    let d = vec![-1.0, 0.0, 0.0];
    assert!((cosine_similarity(&a, &d) - (-1.0)).abs() < 0.001);
}

/// Simple BM25-like scoring for FTS simulation.
/// Uses substring matching and IDF-like weighting.
fn bm25_rank(query: &str, documents: &[(String, String)], top_k: usize) -> Vec<(String, f64)> {
    let query_lower = query.to_lowercase();
    let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

    // Calculate document frequency for each term (for IDF weighting)
    let doc_count = documents.len() as f64;
    let term_doc_freq: std::collections::HashMap<&str, usize> = query_terms
        .iter()
        .map(|term| {
            let df = documents
                .iter()
                .filter(|(_, content)| content.to_lowercase().contains(term))
                .count();
            (*term, df)
        })
        .collect();

    let mut scores: Vec<(String, f64)> = documents
        .iter()
        .map(|(id, content)| {
            let content_lower = content.to_lowercase();
            let doc_len = content.len() as f64;
            let avg_len = 500.0; // approximate average doc length

            let mut score = 0.0;
            for term in &query_terms {
                // Term frequency with saturation
                let tf = content_lower.matches(term).count() as f64;
                if tf > 0.0 {
                    // BM25 formula components
                    let k1 = 1.2;
                    let b = 0.75;
                    let tf_component =
                        (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * doc_len / avg_len));

                    // IDF component
                    let df = *term_doc_freq.get(term).unwrap_or(&1) as f64;
                    let idf = ((doc_count - df + 0.5) / (df + 0.5) + 1.0).ln();

                    score += tf_component * idf;
                }
            }
            (id.clone(), score)
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(top_k);
    scores
}

/// RRF fusion of two ranked lists with optional weighting.
fn rrf_fusion(list_a: &[(String, f64)], list_b: &[(String, f64)], k: f64) -> Vec<(String, f64)> {
    rrf_fusion_weighted(list_a, list_b, k, 1.0, 1.0)
}

/// Weighted RRF fusion - weight_a and weight_b control contribution of each list.
fn rrf_fusion_weighted(
    list_a: &[(String, f64)],
    list_b: &[(String, f64)],
    k: f64,
    weight_a: f64,
    weight_b: f64,
) -> Vec<(String, f64)> {
    use std::collections::HashMap;

    let mut scores: HashMap<String, f64> = HashMap::new();

    for (rank, (id, _)) in list_a.iter().enumerate() {
        *scores.entry(id.clone()).or_insert(0.0) += weight_a / (k + rank as f64 + 1.0);
    }

    for (rank, (id, _)) in list_b.iter().enumerate() {
        *scores.entry(id.clone()).or_insert(0.0) += weight_b / (k + rank as f64 + 1.0);
    }

    let mut result: Vec<_> = scores.into_iter().collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result
}

/// Per-query diagnostic comparing MiniLM vs Potion.
#[test]
#[ignore]
fn eval_diagnostic() {
    use std::collections::HashMap;

    println!("\n=== Diagnostic: MiniLM vs Potion Per-Query ===\n");

    let corpus = load_corpus();
    let queries = load_queries();

    // Build doc content lookup for display
    let doc_content: HashMap<_, _> = corpus
        .documents
        .iter()
        .map(|d| (d.id.clone(), d.content.chars().take(60).collect::<String>()))
        .collect();

    // Embed corpus with both models
    let doc_texts: Vec<String> = corpus.documents.iter().map(|d| d.content.clone()).collect();

    println!("Loading MiniLM-L6-v2-Q...");
    let mut minilm = create_backend(ModelChoice::AllMiniLML6V2Q).unwrap();
    let minilm_embeddings = minilm.embed_batch(&doc_texts).unwrap();
    let minilm_pairs: Vec<_> = corpus
        .documents
        .iter()
        .zip(minilm_embeddings.into_iter())
        .map(|(d, e)| (d.id.clone(), e))
        .collect();

    println!("Loading Potion-base-32M...");
    let mut potion = create_backend(ModelChoice::PotionBase32M).unwrap();
    let potion_embeddings = potion.embed_batch(&doc_texts).unwrap();
    let potion_pairs: Vec<_> = corpus
        .documents
        .iter()
        .zip(potion_embeddings.into_iter())
        .map(|(d, e)| (d.id.clone(), e))
        .collect();

    println!("\n");

    // Track per-type performance
    let mut type_scores: HashMap<String, (Vec<f64>, Vec<f64>)> = HashMap::new();

    for query in &queries.queries {
        // Embed query with both
        let minilm_qemb = minilm.embed_one(&query.query).unwrap();
        let potion_qemb = potion.embed_one(&query.query).unwrap();

        // Rank with both
        let minilm_ranked = rank_by_similarity(&minilm_qemb, &minilm_pairs, 10);
        let potion_ranked = rank_by_similarity(&potion_qemb, &potion_pairs, 10);

        let minilm_ids: Vec<_> = minilm_ranked.iter().map(|(id, _)| id.clone()).collect();
        let potion_ids: Vec<_> = potion_ranked.iter().map(|(id, _)| id.clone()).collect();

        // Calculate RR for each
        let relevant: std::collections::HashSet<_> = query.relevant_doc_ids.iter().collect();

        let minilm_rr = minilm_ids
            .iter()
            .position(|id| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);

        let potion_rr = potion_ids
            .iter()
            .position(|id| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);

        let delta = potion_rr - minilm_rr;
        let _status = if delta >= 0.0 { "=" } else { "WORSE" };

        // Print query result
        println!(
            "Q{}: \"{}\" ({})",
            &query.id[1..],
            &query.query,
            query.query_type
        );
        println!(
            "  MiniLM: RR={:.3}  Top3: [{}, {}, {}]",
            minilm_rr,
            &minilm_ids[0][..8],
            &minilm_ids[1][..8],
            &minilm_ids[2][..8]
        );
        println!(
            "  Potion: RR={:.3}  Top3: [{}, {}, {}]  {}",
            potion_rr,
            &potion_ids[0][..8],
            &potion_ids[1][..8],
            &potion_ids[2][..8],
            if delta < -0.001 {
                format!("({:.3})", delta)
            } else {
                String::new()
            }
        );

        // Show what Potion missed if it failed
        if delta < -0.001 {
            let relevant_id = query.relevant_doc_ids.first().unwrap();
            if let Some(content) = doc_content.get(relevant_id) {
                println!("  Expected: {}... ({})", content, &relevant_id[..8]);
            }
        }
        println!();

        // Track by type
        let qtype = format!("{}", query.query_type);
        type_scores
            .entry(qtype)
            .or_insert_with(|| (Vec::new(), Vec::new()))
            .0
            .push(minilm_rr);
        type_scores
            .entry(format!("{}", query.query_type))
            .or_insert_with(|| (Vec::new(), Vec::new()))
            .1
            .push(potion_rr);
    }

    // Print summary by query type
    println!("=== By Query Type ===\n");
    let mut types: Vec<_> = type_scores.keys().collect();
    types.sort();
    for qtype in types {
        let (minilm_scores, potion_scores) = type_scores.get(qtype).unwrap();
        let minilm_avg: f64 = minilm_scores.iter().sum::<f64>() / minilm_scores.len() as f64;
        let potion_avg: f64 = potion_scores.iter().sum::<f64>() / potion_scores.len() as f64;
        let delta = potion_avg - minilm_avg;
        println!(
            "  {:10} MiniLM={:.3}  Potion={:.3}  delta={:+.3}",
            qtype, minilm_avg, potion_avg, delta
        );
    }
}

/// Evaluate hybrid search: BM25 + Potion with RRF fusion.
#[test]
#[ignore]
fn eval_hybrid() {
    println!("\n=== Hybrid Search Evaluation: BM25 + Potion ===\n");

    let corpus = load_corpus();
    let queries = load_queries();

    // Build document lookup for BM25
    let doc_texts: Vec<(String, String)> = corpus
        .documents
        .iter()
        .map(|d| (d.id.clone(), d.content.clone()))
        .collect();

    // Load models
    println!("Loading MiniLM-L6-v2-Q...");
    let mut minilm = create_backend(ModelChoice::AllMiniLML6V2Q).unwrap();
    let minilm_embeddings = minilm
        .embed_batch(&doc_texts.iter().map(|(_, c)| c.clone()).collect::<Vec<_>>())
        .unwrap();
    let minilm_pairs: Vec<_> = corpus
        .documents
        .iter()
        .zip(minilm_embeddings.into_iter())
        .map(|(d, e)| (d.id.clone(), e))
        .collect();

    println!("Loading Potion-base-32M...");
    let mut potion = create_backend(ModelChoice::PotionBase32M).unwrap();
    let potion_embeddings = potion
        .embed_batch(&doc_texts.iter().map(|(_, c)| c.clone()).collect::<Vec<_>>())
        .unwrap();
    let potion_pairs: Vec<_> = corpus
        .documents
        .iter()
        .zip(potion_embeddings.into_iter())
        .map(|(d, e)| (d.id.clone(), e))
        .collect();

    println!("\nComparing: MiniLM-only vs Potion-only vs BM25-only vs Hybrid (BM25+Potion)\n");

    let mut minilm_rrs = Vec::new();
    let mut potion_rrs = Vec::new();
    let mut bm25_rrs = Vec::new();
    let mut hybrid_rrs = Vec::new();

    for query in &queries.queries {
        let relevant: std::collections::HashSet<_> = query.relevant_doc_ids.iter().collect();

        // MiniLM vector search
        let minilm_qemb = minilm.embed_one(&query.query).unwrap();
        let minilm_ranked = rank_by_similarity(&minilm_qemb, &minilm_pairs, 20);
        let minilm_rr = minilm_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        minilm_rrs.push(minilm_rr);

        // Potion vector search
        let potion_qemb = potion.embed_one(&query.query).unwrap();
        let potion_ranked = rank_by_similarity(&potion_qemb, &potion_pairs, 20);
        let potion_rr = potion_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        potion_rrs.push(potion_rr);

        // BM25 text search
        let bm25_ranked = bm25_rank(&query.query, &doc_texts, 20);
        let bm25_rr = bm25_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        bm25_rrs.push(bm25_rr);

        // Hybrid: BM25 + Potion with RRF
        let potion_for_rrf: Vec<(String, f64)> = potion_ranked
            .iter()
            .map(|(id, score)| (id.clone(), *score as f64))
            .collect();
        let hybrid_ranked = rrf_fusion(&bm25_ranked, &potion_for_rrf, 60.0);
        let hybrid_rr = hybrid_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        hybrid_rrs.push(hybrid_rr);

        // Print per-query comparison
        let best = if hybrid_rr >= minilm_rr && hybrid_rr >= potion_rr && hybrid_rr >= bm25_rr {
            "HYBRID"
        } else if minilm_rr >= potion_rr && minilm_rr >= bm25_rr {
            "minilm"
        } else if potion_rr >= bm25_rr {
            "potion"
        } else {
            "bm25"
        };

        println!(
            "Q{:02}: MiniLM={:.3} Potion={:.3} BM25={:.3} Hybrid={:.3}  [{}]",
            &query.id[1..],
            minilm_rr,
            potion_rr,
            bm25_rr,
            hybrid_rr,
            best
        );
    }

    // Calculate MRR for each approach
    let minilm_mrr: f64 = minilm_rrs.iter().sum::<f64>() / minilm_rrs.len() as f64;
    let potion_mrr: f64 = potion_rrs.iter().sum::<f64>() / potion_rrs.len() as f64;
    let bm25_mrr: f64 = bm25_rrs.iter().sum::<f64>() / bm25_rrs.len() as f64;
    let hybrid_mrr: f64 = hybrid_rrs.iter().sum::<f64>() / hybrid_rrs.len() as f64;

    println!("\n=== Summary (MRR) ===\n");
    println!("  MiniLM-only:     {:.3}", minilm_mrr);
    println!("  Potion-only:     {:.3}", potion_mrr);
    println!("  BM25-only:       {:.3}", bm25_mrr);
    println!("  Hybrid (BM25+P): {:.3}", hybrid_mrr);

    // Also compute hybrid with MiniLM
    let mut hybrid_minilm_rrs = Vec::new();
    for query in &queries.queries {
        let relevant: std::collections::HashSet<_> = query.relevant_doc_ids.iter().collect();

        let minilm_qemb = minilm.embed_one(&query.query).unwrap();
        let minilm_ranked = rank_by_similarity(&minilm_qemb, &minilm_pairs, 20);
        let minilm_for_rrf: Vec<(String, f64)> = minilm_ranked
            .iter()
            .map(|(id, score)| (id.clone(), *score as f64))
            .collect();

        let bm25_ranked = bm25_rank(&query.query, &doc_texts, 20);
        let hybrid_minilm_ranked = rrf_fusion(&bm25_ranked, &minilm_for_rrf, 60.0);
        let hybrid_minilm_rr = hybrid_minilm_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        hybrid_minilm_rrs.push(hybrid_minilm_rr);
    }
    let hybrid_minilm_mrr: f64 =
        hybrid_minilm_rrs.iter().sum::<f64>() / hybrid_minilm_rrs.len() as f64;

    println!("\n=== Summary (MRR) ===\n");
    println!("  MiniLM-only:       {:.3}", minilm_mrr);
    println!("  Hybrid (BM25+M):   {:.3}", hybrid_minilm_mrr);
    println!("  BM25-only:         {:.3}", bm25_mrr);
    println!("  Potion-only:       {:.3}", potion_mrr);
    println!("  Hybrid (BM25+P):   {:.3}", hybrid_mrr);

    println!(
        "\n  MiniLM hybrid vs MiniLM: {:+.3}",
        hybrid_minilm_mrr - minilm_mrr
    );
    println!("  Potion hybrid vs Potion: {:+.3}", hybrid_mrr - potion_mrr);

    // Try different BM25 weights
    println!("\n=== Weight Tuning: BM25 weight vs Potion weight ===\n");
    for bm25_weight in [1.0, 1.5, 2.0, 2.5, 3.0] {
        let mut weighted_rrs = Vec::new();
        for query in &queries.queries {
            let relevant: std::collections::HashSet<_> = query.relevant_doc_ids.iter().collect();

            let potion_qemb = potion.embed_one(&query.query).unwrap();
            let potion_ranked = rank_by_similarity(&potion_qemb, &potion_pairs, 20);
            let potion_for_rrf: Vec<(String, f64)> = potion_ranked
                .iter()
                .map(|(id, score)| (id.clone(), *score as f64))
                .collect();

            let bm25_ranked = bm25_rank(&query.query, &doc_texts, 20);
            let weighted_ranked =
                rrf_fusion_weighted(&bm25_ranked, &potion_for_rrf, 60.0, bm25_weight, 1.0);

            let rr = weighted_ranked
                .iter()
                .position(|(id, _)| relevant.contains(id))
                .map(|p| 1.0 / (p + 1) as f64)
                .unwrap_or(0.0);
            weighted_rrs.push(rr);
        }
        let mrr: f64 = weighted_rrs.iter().sum::<f64>() / weighted_rrs.len() as f64;
        println!(
            "  BM25 weight {:.1} : Potion 1.0  =>  MRR = {:.3}",
            bm25_weight, mrr
        );
    }

    // Analyze failures: where does hybrid BM25+Potion still fail?
    println!("\n=== Hybrid (BM25+Potion) Failure Analysis ===\n");

    let doc_content: std::collections::HashMap<_, _> = corpus
        .documents
        .iter()
        .map(|d| (d.id.clone(), d.content.chars().take(80).collect::<String>()))
        .collect();

    for (i, query) in queries.queries.iter().enumerate() {
        let hybrid_rr = hybrid_rrs[i];
        let minilm_rr = minilm_rrs[i];

        // Only show where hybrid fails but MiniLM succeeds
        if hybrid_rr < 0.5 && minilm_rr >= 0.5 {
            let relevant: std::collections::HashSet<_> = query.relevant_doc_ids.iter().collect();

            // Get the rankings again for display
            let potion_qemb = potion.embed_one(&query.query).unwrap();
            let potion_ranked = rank_by_similarity(&potion_qemb, &potion_pairs, 5);
            let bm25_ranked = bm25_rank(&query.query, &doc_texts, 5);

            println!(
                "Q{:02}: \"{}\" ({})",
                &query.id[1..],
                &query.query,
                query.query_type
            );
            println!("  Hybrid RR={:.3}, MiniLM RR={:.3}", hybrid_rr, minilm_rr);
            println!(
                "  Expected: {}",
                query
                    .relevant_doc_ids
                    .first()
                    .map(|id| &id[..8])
                    .unwrap_or("?")
            );

            println!("  BM25 top-3:");
            for (j, (id, score)) in bm25_ranked.iter().take(3).enumerate() {
                let marker = if relevant.contains(id) { " <--" } else { "" };
                let preview = doc_content.get(id).map(|s| s.as_str()).unwrap_or("");
                println!(
                    "    {}. [{:.2}] {}... {}",
                    j + 1,
                    score,
                    &preview[..preview.len().min(50)],
                    marker
                );
            }

            println!("  Potion top-3:");
            for (j, (id, score)) in potion_ranked.iter().take(3).enumerate() {
                let marker = if relevant.contains(id) { " <--" } else { "" };
                let preview = doc_content.get(id).map(|s| s.as_str()).unwrap_or("");
                println!(
                    "    {}. [{:.3}] {}... {}",
                    j + 1,
                    score,
                    &preview[..preview.len().min(50)],
                    marker
                );
            }
            println!();
        }
    }
}

/// Adaptive routing evaluation - simulates search_hybrid_adaptive logic.
#[test]
#[ignore]
fn eval_adaptive_routing() {
    println!("\n=== Adaptive Routing Evaluation ===\n");

    let corpus = load_corpus();
    let queries = load_queries();

    // Build document lookup for BM25
    let doc_texts: Vec<(String, String)> = corpus
        .documents
        .iter()
        .map(|d| (d.id.clone(), d.content.clone()))
        .collect();

    // Load Potion embeddings
    println!("Loading Potion-base-32M...");
    let mut potion = create_backend(ModelChoice::PotionBase32M).unwrap();
    let potion_embeddings = potion
        .embed_batch(&doc_texts.iter().map(|(_, c)| c.clone()).collect::<Vec<_>>())
        .unwrap();
    let potion_pairs: Vec<_> = corpus
        .documents
        .iter()
        .zip(potion_embeddings.into_iter())
        .map(|(d, e)| (d.id.clone(), e))
        .collect();

    println!("\nComparing: Static Hybrid vs Adaptive Routing\n");
    println!(
        "{:<4} {:<35} {:>6} {:>6} {:>7} {:<8} {}",
        "Q", "Query", "Hybrid", "Adapt", "Delta", "Route", "BM25 Score"
    );
    println!("{}", "-".repeat(100));

    let mut hybrid_rrs = Vec::new();
    let mut adaptive_rrs = Vec::new();
    let mut routes = Vec::new();

    for query in &queries.queries {
        let relevant: std::collections::HashSet<_> = query.relevant_doc_ids.iter().collect();

        // Run BM25
        let bm25_ranked = bm25_rank(&query.query, &doc_texts, 20);

        // Run Potion vector search
        let potion_qemb = potion.embed_one(&query.query).unwrap();
        let potion_ranked = rank_by_similarity(&potion_qemb, &potion_pairs, 20);
        let potion_for_rrf: Vec<(String, f64)> = potion_ranked
            .iter()
            .map(|(id, score)| (id.clone(), *score as f64))
            .collect();

        // Static hybrid (RRF fusion)
        let hybrid_ranked = rrf_fusion(&bm25_ranked, &potion_for_rrf, 60.0);
        let hybrid_rr = hybrid_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        hybrid_rrs.push(hybrid_rr);

        // Adaptive routing logic (mirrors search_hybrid_adaptive)
        let top1_match = !bm25_ranked.is_empty()
            && !potion_for_rrf.is_empty()
            && bm25_ranked[0].0 == potion_for_rrf[0].0;

        // BM25 confidence: high score with clear gap to #2
        let bm25_confident = bm25_ranked.len() >= 2
            && bm25_ranked[0].1 > 10.0
            && (bm25_ranked[0].1 - bm25_ranked[1].1) > 3.0;

        let (adaptive_ranked, route) = if top1_match {
            // Agreement - use hybrid
            (hybrid_ranked.clone(), "hybrid")
        } else if bm25_confident {
            // Disagreement + BM25 confident - use BM25 only
            (bm25_ranked.clone(), "bm25")
        } else {
            // Uncertain - use hybrid
            (hybrid_ranked.clone(), "hybrid")
        };

        let adaptive_rr = adaptive_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        adaptive_rrs.push(adaptive_rr);
        routes.push(route);

        let delta = adaptive_rr - hybrid_rr;
        let delta_str = if delta.abs() < 0.001 {
            "=".to_string()
        } else if delta > 0.0 {
            format!("+{:.3}", delta)
        } else {
            format!("{:.3}", delta)
        };

        let query_display = if query.query.len() > 40 {
            format!("{}...", &query.query[..37])
        } else {
            query.query.clone()
        };

        // Show BM25 scores for debugging
        let bm25_top = bm25_ranked[0].1;
        let bm25_gap = if bm25_ranked.len() >= 2 {
            bm25_ranked[0].1 - bm25_ranked[1].1
        } else {
            0.0
        };
        let agree = if top1_match { "agree" } else { "DIFF" };

        println!(
            "{:<4} {:<35} {:>6.3} {:>6.3} {:>7} {:<8} BM25:{:.1}/gap:{:.1} {}",
            &query.id[1..],
            query_display,
            hybrid_rr,
            adaptive_rr,
            delta_str,
            route,
            bm25_top,
            bm25_gap,
            agree
        );
    }

    // Summary
    let hybrid_mrr: f64 = hybrid_rrs.iter().sum::<f64>() / hybrid_rrs.len() as f64;
    let adaptive_mrr: f64 = adaptive_rrs.iter().sum::<f64>() / adaptive_rrs.len() as f64;

    let bm25_count = routes.iter().filter(|&&r| r == "bm25").count();
    let hybrid_count = routes.iter().filter(|&&r| r == "hybrid").count();

    println!("\n{}", "=".repeat(90));
    println!("\n=== Summary ===\n");
    println!("Static Hybrid MRR:   {:.3}", hybrid_mrr);
    println!("Adaptive Routing MRR: {:.3}", adaptive_mrr);
    println!("Improvement:         {:+.3}", adaptive_mrr - hybrid_mrr);
    println!(
        "\nRouting breakdown: {} BM25-only, {} hybrid",
        bm25_count, hybrid_count
    );

    // Show which queries improved/regressed
    let improved: Vec<_> = queries
        .queries
        .iter()
        .zip(hybrid_rrs.iter().zip(adaptive_rrs.iter()))
        .filter(|(_, (h, a))| **a > **h + 0.001)
        .map(|(q, _)| q.id.as_str())
        .collect();

    let regressed: Vec<_> = queries
        .queries
        .iter()
        .zip(hybrid_rrs.iter().zip(adaptive_rrs.iter()))
        .filter(|(_, (h, a))| **a < **h - 0.001)
        .map(|(q, _)| q.id.as_str())
        .collect();

    if !improved.is_empty() {
        println!("\nImproved: {}", improved.join(", "));
    }
    if !regressed.is_empty() {
        println!("Regressed: {}", regressed.join(", "));
    }
}

/// Evaluate cross-encoder reranking on top of hybrid search.
#[test]
#[ignore]
fn eval_reranked() {
    use glhf::rerank::Reranker;

    println!("\n=== Reranked Search Evaluation ===\n");

    let corpus = load_corpus();
    let queries = load_queries();

    // Build document lookup for BM25
    let doc_texts: Vec<(String, String)> = corpus
        .documents
        .iter()
        .map(|d| (d.id.clone(), d.content.clone()))
        .collect();

    // Load Potion embeddings
    println!("Loading Potion-base-32M...");
    let mut potion = create_backend(ModelChoice::PotionBase32M).unwrap();
    let potion_embeddings = potion
        .embed_batch(&doc_texts.iter().map(|(_, c)| c.clone()).collect::<Vec<_>>())
        .unwrap();
    let potion_pairs: Vec<_> = corpus
        .documents
        .iter()
        .zip(potion_embeddings.into_iter())
        .map(|(d, e)| (d.id.clone(), e))
        .collect();

    // Load reranker
    println!("Loading JINA Reranker v1 Turbo...");
    let mut reranker = Reranker::new().expect("Failed to load reranker");

    println!("\nComparing: Hybrid vs Adaptive vs Reranked\n");
    println!(
        "{:<4} {:<35} {:>6} {:>6} {:>6} {:>7}",
        "Q", "Query", "Hybrid", "Adapt", "Rerank", "Delta"
    );
    println!("{}", "-".repeat(80));

    let mut hybrid_rrs = Vec::new();
    let mut adaptive_rrs = Vec::new();
    let mut reranked_rrs = Vec::new();

    for query in &queries.queries {
        let relevant: std::collections::HashSet<_> = query.relevant_doc_ids.iter().collect();

        // Run BM25
        let bm25_ranked = bm25_rank(&query.query, &doc_texts, 20);

        // Run Potion vector search
        let potion_qemb = potion.embed_one(&query.query).unwrap();
        let potion_ranked = rank_by_similarity(&potion_qemb, &potion_pairs, 20);
        let potion_for_rrf: Vec<(String, f64)> = potion_ranked
            .iter()
            .map(|(id, score)| (id.clone(), *score as f64))
            .collect();

        // Static hybrid (RRF fusion)
        let hybrid_ranked = rrf_fusion(&bm25_ranked, &potion_for_rrf, 60.0);
        let hybrid_rr = hybrid_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        hybrid_rrs.push(hybrid_rr);

        // Adaptive routing
        let top1_match = !bm25_ranked.is_empty()
            && !potion_for_rrf.is_empty()
            && bm25_ranked[0].0 == potion_for_rrf[0].0;

        let bm25_confident = bm25_ranked.len() >= 2
            && bm25_ranked[0].1 > 10.0
            && (bm25_ranked[0].1 - bm25_ranked[1].1) > 3.0;

        let adaptive_ranked = if top1_match {
            hybrid_ranked.clone()
        } else if bm25_confident {
            bm25_ranked.clone()
        } else {
            hybrid_ranked.clone()
        };

        let adaptive_rr = adaptive_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        adaptive_rrs.push(adaptive_rr);

        // Reranked: take top-20 from hybrid, rerank with cross-encoder
        let top_candidates: Vec<&str> = hybrid_ranked
            .iter()
            .take(20)
            .map(|(id, _)| {
                doc_texts
                    .iter()
                    .find(|(doc_id, _)| doc_id == id)
                    .map(|(_, content)| content.as_str())
                    .unwrap_or("")
            })
            .collect();

        let reranked_indices = reranker.rerank(&query.query, &top_candidates).unwrap();

        // Map back to doc IDs
        let reranked_ids: Vec<String> = reranked_indices
            .iter()
            .map(|(idx, _)| hybrid_ranked[*idx].0.clone())
            .collect();

        let reranked_rr = reranked_ids
            .iter()
            .position(|id| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        reranked_rrs.push(reranked_rr);

        // Print per-query comparison
        let delta = reranked_rr - hybrid_rr;
        let delta_str = if delta.abs() < 0.001 {
            "=".to_string()
        } else if delta > 0.0 {
            format!("+{:.3}", delta)
        } else {
            format!("{:.3}", delta)
        };

        let query_display = if query.query.len() > 35 {
            format!("{}...", &query.query[..32])
        } else {
            query.query.clone()
        };

        println!(
            "{:<4} {:<35} {:>6.3} {:>6.3} {:>6.3} {:>7}",
            &query.id[1..],
            query_display,
            hybrid_rr,
            adaptive_rr,
            reranked_rr,
            delta_str
        );
    }

    // Summary
    let hybrid_mrr: f64 = hybrid_rrs.iter().sum::<f64>() / hybrid_rrs.len() as f64;
    let adaptive_mrr: f64 = adaptive_rrs.iter().sum::<f64>() / adaptive_rrs.len() as f64;
    let reranked_mrr: f64 = reranked_rrs.iter().sum::<f64>() / reranked_rrs.len() as f64;

    println!("\n{}", "=".repeat(80));
    println!("\n=== Summary ===\n");
    println!("Hybrid MRR:   {:.3}", hybrid_mrr);
    println!(
        "Adaptive MRR: {:.3} ({:+.3})",
        adaptive_mrr,
        adaptive_mrr - hybrid_mrr
    );
    println!(
        "Reranked MRR: {:.3} ({:+.3})",
        reranked_mrr,
        reranked_mrr - hybrid_mrr
    );

    // Show which queries improved/regressed with reranking
    let improved: Vec<_> = queries
        .queries
        .iter()
        .zip(hybrid_rrs.iter().zip(reranked_rrs.iter()))
        .filter(|(_, (h, r))| **r > **h + 0.001)
        .map(|(q, _)| q.id.as_str())
        .collect();

    let regressed: Vec<_> = queries
        .queries
        .iter()
        .zip(hybrid_rrs.iter().zip(reranked_rrs.iter()))
        .filter(|(_, (h, r))| **r < **h - 0.001)
        .map(|(q, _)| q.id.as_str())
        .collect();

    if !improved.is_empty() {
        println!("\nReranking improved: {}", improved.join(", "));
    }
    if !regressed.is_empty() {
        println!("Reranking regressed: {}", regressed.join(", "));
    }
}

/// Evaluate on expanded dataset (~1000 docs, ~100 queries).
/// Run with: cargo test --release eval_expanded -- --nocapture --ignored
#[test]
#[ignore]
fn eval_expanded() {
    use std::collections::{HashMap, HashSet};

    println!("\n=== Expanded Dataset Evaluation ===\n");

    // Load expanded corpus and queries
    let corpus_path = Path::new(CORPUS_EXPANDED_PATH);
    let queries_path = Path::new(QUERIES_EXPANDED_PATH);

    if !corpus_path.exists() {
        println!("Expanded corpus not found. Run: python scripts/sample_corpus.py");
        return;
    }
    if !queries_path.exists() {
        println!("Expanded queries not found. Run: python scripts/label_queries.py");
        return;
    }

    let corpus = Corpus::load(corpus_path).expect("Failed to load expanded corpus");
    let queries = QuerySet::load(queries_path).expect("Failed to load expanded queries");

    println!("Corpus: {} documents", corpus.documents.len());
    println!("Queries: {} test queries\n", queries.queries.len());

    // Build document lookup for BM25
    let doc_texts: Vec<(String, String)> = corpus
        .documents
        .iter()
        .map(|d| (d.id.clone(), d.content.clone()))
        .collect();

    // Load Potion embeddings
    println!("Loading Potion-base-32M...");
    let mut potion = create_backend(ModelChoice::PotionBase32M).unwrap();
    let embedding_start = Instant::now();
    let potion_embeddings = potion
        .embed_batch(&doc_texts.iter().map(|(_, c)| c.clone()).collect::<Vec<_>>())
        .unwrap();
    let embedding_time = embedding_start.elapsed();
    println!(
        "Embedded {} docs in {:.2}s ({:.0} docs/s)\n",
        corpus.documents.len(),
        embedding_time.as_secs_f64(),
        corpus.documents.len() as f64 / embedding_time.as_secs_f64()
    );

    let potion_pairs: Vec<_> = corpus
        .documents
        .iter()
        .zip(potion_embeddings.into_iter())
        .map(|(d, e)| (d.id.clone(), e))
        .collect();

    // Track metrics by query type
    let mut type_metrics: HashMap<String, (Vec<f64>, Vec<f64>, Vec<f64>)> = HashMap::new();

    let mut bm25_rrs = Vec::new();
    let mut hybrid_rrs = Vec::new();
    let mut adaptive_rrs = Vec::new();

    println!(
        "{:<5} {:<12} {:<35} {:>6} {:>6} {:>6}",
        "Q", "Type", "Query", "BM25", "Hybrid", "Adapt"
    );
    println!("{}", "-".repeat(90));

    for query in &queries.queries {
        let relevant: HashSet<_> = query.relevant_doc_ids.iter().collect();

        // Run BM25
        let bm25_ranked = bm25_rank(&query.query, &doc_texts, 20);
        let bm25_rr = bm25_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        bm25_rrs.push(bm25_rr);

        // Run Potion vector search
        let potion_qemb = potion.embed_one(&query.query).unwrap();
        let potion_ranked = rank_by_similarity(&potion_qemb, &potion_pairs, 20);
        let potion_for_rrf: Vec<(String, f64)> = potion_ranked
            .iter()
            .map(|(id, score)| (id.clone(), *score as f64))
            .collect();

        // Static hybrid (RRF fusion)
        let hybrid_ranked = rrf_fusion(&bm25_ranked, &potion_for_rrf, 60.0);
        let hybrid_rr = hybrid_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        hybrid_rrs.push(hybrid_rr);

        // Adaptive routing
        let top1_match = !bm25_ranked.is_empty()
            && !potion_for_rrf.is_empty()
            && bm25_ranked[0].0 == potion_for_rrf[0].0;

        let bm25_confident = bm25_ranked.len() >= 2
            && bm25_ranked[0].1 > 10.0
            && (bm25_ranked[0].1 - bm25_ranked[1].1) > 3.0;

        let adaptive_ranked = if top1_match {
            hybrid_ranked.clone()
        } else if bm25_confident {
            bm25_ranked.clone()
        } else {
            hybrid_ranked.clone()
        };

        let adaptive_rr = adaptive_ranked
            .iter()
            .position(|(id, _)| relevant.contains(id))
            .map(|p| 1.0 / (p + 1) as f64)
            .unwrap_or(0.0);
        adaptive_rrs.push(adaptive_rr);

        // Track by query type
        let qtype = format!("{}", query.query_type);
        type_metrics
            .entry(qtype.clone())
            .or_insert_with(|| (Vec::new(), Vec::new(), Vec::new()));
        let entry = type_metrics.get_mut(&qtype).unwrap();
        entry.0.push(bm25_rr);
        entry.1.push(hybrid_rr);
        entry.2.push(adaptive_rr);

        // Print per-query
        let query_display = if query.query.len() > 32 {
            format!("{}...", &query.query[..29])
        } else {
            query.query.clone()
        };

        println!(
            "{:<5} {:<12} {:<35} {:>6.3} {:>6.3} {:>6.3}",
            &query.id[1..],
            query.query_type,
            query_display,
            bm25_rr,
            hybrid_rr,
            adaptive_rr
        );
    }

    // Summary
    let bm25_mrr: f64 = bm25_rrs.iter().sum::<f64>() / bm25_rrs.len() as f64;
    let hybrid_mrr: f64 = hybrid_rrs.iter().sum::<f64>() / hybrid_rrs.len() as f64;
    let adaptive_mrr: f64 = adaptive_rrs.iter().sum::<f64>() / adaptive_rrs.len() as f64;

    println!("\n{}", "=".repeat(90));
    println!(
        "\n=== Overall MRR ({} queries) ===\n",
        queries.queries.len()
    );
    println!("  BM25-only:   {:.3}", bm25_mrr);
    println!(
        "  Hybrid:      {:.3} ({:+.3})",
        hybrid_mrr,
        hybrid_mrr - bm25_mrr
    );
    println!(
        "  Adaptive:    {:.3} ({:+.3})",
        adaptive_mrr,
        adaptive_mrr - bm25_mrr
    );

    // By query type
    println!("\n=== MRR by Query Type ===\n");
    println!(
        "{:<12} {:>6} {:>6} {:>6} {:>5}",
        "Type", "BM25", "Hybrid", "Adapt", "n"
    );
    println!("{}", "-".repeat(50));

    let mut types: Vec<_> = type_metrics.keys().collect();
    types.sort();
    for qtype in types {
        let (bm25s, hybrids, adaptives) = type_metrics.get(qtype).unwrap();
        let bm25_avg: f64 = bm25s.iter().sum::<f64>() / bm25s.len() as f64;
        let hybrid_avg: f64 = hybrids.iter().sum::<f64>() / hybrids.len() as f64;
        let adaptive_avg: f64 = adaptives.iter().sum::<f64>() / adaptives.len() as f64;
        println!(
            "{:<12} {:>6.3} {:>6.3} {:>6.3} {:>5}",
            qtype,
            bm25_avg,
            hybrid_avg,
            adaptive_avg,
            bm25s.len()
        );
    }

    // Recall@k
    println!("\n=== Recall@k (Adaptive) ===\n");
    for k in [1, 3, 5, 10] {
        let recall: f64 = queries
            .queries
            .iter()
            .map(|q| {
                let relevant: HashSet<_> = q.relevant_doc_ids.iter().collect();

                // Recompute adaptive ranking
                let bm25_ranked = bm25_rank(&q.query, &doc_texts, 20);
                let potion_qemb = potion.embed_one(&q.query).unwrap();
                let potion_ranked = rank_by_similarity(&potion_qemb, &potion_pairs, 20);
                let potion_for_rrf: Vec<(String, f64)> = potion_ranked
                    .iter()
                    .map(|(id, score)| (id.clone(), *score as f64))
                    .collect();
                let hybrid_ranked = rrf_fusion(&bm25_ranked, &potion_for_rrf, 60.0);

                let top1_match = !bm25_ranked.is_empty()
                    && !potion_for_rrf.is_empty()
                    && bm25_ranked[0].0 == potion_for_rrf[0].0;
                let bm25_confident = bm25_ranked.len() >= 2
                    && bm25_ranked[0].1 > 10.0
                    && (bm25_ranked[0].1 - bm25_ranked[1].1) > 3.0;

                let adaptive_ranked = if top1_match {
                    hybrid_ranked
                } else if bm25_confident {
                    bm25_ranked
                } else {
                    hybrid_ranked
                };

                // Check recall@k
                let top_k_ids: HashSet<_> =
                    adaptive_ranked.iter().take(k).map(|(id, _)| id).collect();
                let found = relevant.iter().filter(|r| top_k_ids.contains(*r)).count();
                found as f64 / relevant.len() as f64
            })
            .sum::<f64>()
            / queries.queries.len() as f64;

        println!("  Recall@{:<2}: {:.3}", k, recall);
    }
}
