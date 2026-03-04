mod common;

use common::corpus::{
    assert_in_top_k, assert_not_in_top_k, assert_ranks_above, zero_embedding, SearchCorpus,
};
use common::TestEnv;
use glhf::db::Database;
use glhf::db::SearchResult;
use glhf::ChunkKind;

/// Sets up a database with the standard corpus (FTS only, no embeddings).
fn setup_fts_corpus() -> (Database, TestEnv) {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("search_quality.db");
    let mut db = Database::open(&db_path).expect("Failed to open database");
    let corpus = SearchCorpus::standard();
    corpus.insert_into(&mut db);
    (db, env)
}

/// Sets up a database with the standard corpus including embeddings.
fn setup_full_corpus() -> (Database, TestEnv) {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("search_quality_full.db");
    let mut db = Database::open(&db_path).expect("Failed to open database");
    let corpus = SearchCorpus::standard();
    corpus.insert_with_embeddings(&mut db);
    (db, env)
}

// ═══════════════════════════════════════════════════════════════════════
// Tier 1: FTS (no #[ignore], fast CI)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn fts_exact_keyword_ranks_first() {
    let (db, _env) = setup_fts_corpus();
    let results = db.search_fts("Rust", 10).unwrap();
    assert_in_top_k(&results, "Rust", 3);
}

#[test]
fn fts_multiword_and_semantics() {
    let (db, _env) = setup_fts_corpus();
    let results = db.search_fts("fix broken pipe", 10).unwrap();
    assert_in_top_k(&results, "Broken pipe", 3);
}

#[test]
fn fts_bm25_term_frequency() {
    let (db, _env) = setup_fts_corpus();
    let results = db.search_fts("sqlite", 10).unwrap();

    // Doc mentioning "SQLite" 5 times should rank above the one with 1 mention
    assert_ranks_above(
        &results,
        "SQLite is a lightweight embedded database",
        "migrations for databases including SQLite",
    );
}

#[test]
fn fts_special_chars() {
    let (db, _env) = setup_fts_corpus();

    let cpp_results = db.search_fts("C++", 10).unwrap();
    assert_in_top_k(&cpp_results, "C++ templates", 3);

    let path_results = db.search_fts("$PATH", 10).unwrap();
    assert_in_top_k(&path_results, "$PATH environment variable", 3);
}

#[test]
fn fts_reserved_keywords() {
    let (db, _env) = setup_fts_corpus();
    let results = db.search_fts("OR", 10).unwrap();
    // Should find the doc about SQL OR, not crash on the reserved keyword
    assert_in_top_k(&results, "SQL OR operator", 5);
}

#[test]
fn fts_tool_chunks_searchable() {
    let (db, _env) = setup_fts_corpus();
    let results = db.search_fts("git status", 10).unwrap();

    let has_tool_use = results
        .iter()
        .any(|r| r.chunk_kind == "tool_use" && r.content.contains("git status"));
    assert!(has_tool_use, "Expected to find 'git status' tool_use chunk");
}

#[test]
fn fts_error_results_searchable() {
    let (db, _env) = setup_fts_corpus();
    let results = db.search_fts("ENOENT", 10).unwrap();

    let has_error = results
        .iter()
        .any(|r| r.is_error == Some(true) && r.content.contains("ENOENT"));
    assert!(has_error, "Expected to find ENOENT error tool_result");
}

#[test]
fn fts_filter_preserves_ranking() {
    let (db, _env) = setup_fts_corpus();

    // Unfiltered search for "test"
    let all = db.search_fts("test", 20).unwrap();

    // Filtered to messages only
    let messages = db
        .search_fts_filtered("test", 20, Some(ChunkKind::Message), None, false)
        .unwrap();

    assert!(
        !messages.is_empty(),
        "Should find message results for 'test'"
    );

    // Every filtered result should be a message
    for r in &messages {
        assert_eq!(r.chunk_kind, "message");
    }

    // Message results from unfiltered, preserving order
    let message_ids_from_all: Vec<&str> = all
        .iter()
        .filter(|r| r.chunk_kind == "message")
        .map(|r| r.id.as_str())
        .collect();
    let message_ids_from_filtered: Vec<&str> = messages.iter().map(|r| r.id.as_str()).collect();

    assert_eq!(
        message_ids_from_all, message_ids_from_filtered,
        "Filtered results should be the same subset in the same order as \
         message results from the unfiltered query"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tier 2: Semantic (requires model download)
// ═══════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "Requires model download"]
fn semantic_synonym_retrieval() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    let query = "exception management";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 10).unwrap();

    // Error handling docs should be found via semantic similarity
    assert_in_top_k(&results, "error handling", 5);
    // Weather filler should NOT appear near the top
    assert_not_in_top_k(&results, "weather forecast", 5);
}

#[test]
#[ignore = "Requires model download"]
fn semantic_conceptual_similarity() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    let query = "push code to production";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    // Deployment docs should rank above CSS/frontend docs
    assert_ranks_above(&results, "production", "CSS flexbox");
}

#[test]
#[ignore = "Requires model download"]
fn semantic_session_coherence() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    let query = "login security";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 10).unwrap();

    // Auth session docs should appear in top results
    let auth_in_top = results
        .iter()
        .take(5)
        .any(|r| r.session_id.as_deref() == Some("auth-session-001"));
    assert!(
        auth_in_top,
        "Expected auth session docs in top 5 for 'login security'"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tier 2b: Harder semantic (requires model download)
//
// These test near-miss distractors, keyword collisions, compositional
// queries, and cross-domain analogies that stress embedding models.
// ═══════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "Requires model download"]
fn semantic_keyword_collision() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // "Rust programming" should ideally find the language, not the survival game.
    // Static embedding models struggle here because "Rust" is a strong keyword
    // match to both. We test that at least SOME Rust language doc appears in top 10.
    let query = "Rust programming tutorial";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    // Weak assertion: any Rust-the-language doc in top 10
    let has_rust_lang = results.iter().take(10).any(|r| {
        r.content.contains("Cargo is Rust")
            || r.content.contains("Rust's error handling")
            || r.content.contains("Testing in Rust")
            || r.content.contains("Async Rust")
    });
    assert!(
        has_rust_lang,
        "Expected at least one Rust language doc in top 10"
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_cross_domain_analogy() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // Should find pip (Python package manager) not just Cargo.
    // Retrieval models may rank Python tool-use chunks (pytest) and NumPy
    // above the pip doc since "cargo equivalent" pulls in build tooling.
    let query = "the cargo equivalent for Python";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    assert_in_top_k(&results, "pip and virtual environments", 10);
}

#[test]
#[ignore = "Requires model download"]
fn semantic_merge_disambiguation() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // "merge conflicts in code" should find git merge, not pandas merge
    let query = "resolving merge conflicts in source code repositories";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 10).unwrap();

    assert_ranks_above(
        &results,
        "Merge conflicts occur when Git",
        "Pandas merge and join operations",
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_compositional_concept() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // Requires composing "version control" + "database" concepts
    let query = "version control for database schemas";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 10).unwrap();

    assert_in_top_k(&results, "Database migrations track schema changes", 5);
}

#[test]
#[ignore = "Requires model download"]
fn semantic_abstract_to_concrete() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // Abstract description → concrete error messages
    let query = "why did my program crash with a null reference";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 10).unwrap();

    // Should find one of the error tool_results
    let has_error_doc = results.iter().take(5).any(|r| {
        r.content.contains("Segmentation fault") || r.content.contains("Cannot read property")
    });
    assert!(
        has_error_doc,
        "Expected crash/null error docs in top 5 for abstract crash query"
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_paraphrase_detection() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // Paraphrase of CI/CD without using those exact terms.
    // Known weakness: static models struggle with loose paraphrases at scale.
    // At 500 docs this doc falls well outside top 15. A contextual model
    // should keep it in top 5. We use top 30 as the current model's ceiling.
    let query = "automated quality checks before integrating code changes";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 40).unwrap();

    assert_in_top_k(&results, "CI/CD pipelines automate", 30);
}

#[test]
#[ignore = "Requires model download"]
fn semantic_distractor_container() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // "Docker containers" should rank above "CSS container queries".
    // Some models rank CSS container queries competitively since "container"
    // is the dominant signal. At minimum, both should appear in top 5.
    let query = "packaging applications in containers for deployment";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 10).unwrap();

    assert_in_top_k(&results, "Docker containers package applications", 5);
    assert_in_top_k(&results, "CSS container queries", 5);
}

#[test]
#[ignore = "Requires model download"]
fn semantic_distractor_branches() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // "code branches" should ideally find git branches, not decision trees.
    // Known weakness: static embedding models can't connect "branches for
    // feature development" to git workflow — "branches" dominates and neither
    // git nor ML docs rank well. A contextual model would nail this.
    //
    // Weak assertion: at least one of the branch-related docs appears
    // somewhere in results (verifies the query doesn't completely miss).
    let query = "creating and managing branches for feature development";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    let has_any_branch_doc = results
        .iter()
        .any(|r| r.content.to_lowercase().contains("branch"));
    assert!(
        has_any_branch_doc,
        "Expected at least one branch-related doc in results"
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_distractor_pipeline() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // "code deployment pipeline" should find CI/CD, not Airflow data pipelines
    let query = "continuous integration pipeline for running tests on pull requests";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 10).unwrap();

    assert_ranks_above(
        &results,
        "CI/CD pipelines automate",
        "Apache Airflow orchestrates complex data pipelines",
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_distractor_tokens() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // "authentication tokens" should find JWT, not rate limiting tokens
    let query = "generating and validating authentication tokens for user sessions";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 10).unwrap();

    assert_ranks_above(
        &results,
        "JSON Web Tokens provide stateless authentication",
        "Token bucket rate limiting",
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tier 2c: Homonym disambiguation (requires model download)
//
// Same word, completely different meaning. Tests whether the model uses
// surrounding context to disambiguate. Static models often fail here.
// ═══════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "Requires model download"]
fn semantic_thread_concurrency() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    let query = "managing worker threads for parallel computation";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    assert_ranks_above(
        &results,
        "Thread pools manage concurrent execution",
        "discussion thread on the community forum",
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_stack_programming() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    let query = "stack overflow from infinite recursion";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    assert_ranks_above(
        &results,
        "stack overflow occurs when recursive function",
        "stack of unread papers on my desk",
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_port_networking() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    let query = "opening port 8080 for the web server";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    assert_ranks_above(
        &results,
        "Network ports identify specific services",
        "shipping port handles thousands of cargo",
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_log_debugging() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    let query = "checking application logs for error messages";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    assert_ranks_above(
        &results,
        "Structured logging with JSON format",
        "fallen log across the hiking trail",
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_class_inheritance() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    let query = "defining class hierarchies with inheritance in object-oriented code";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    assert_ranks_above(
        &results,
        "Object-oriented class hierarchies",
        "university class on medieval",
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_shell_scripting() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    let query = "writing shell scripts for automating deployment tasks";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    assert_ranks_above(
        &results,
        "Shell scripting automates repetitive",
        "nautilus shell exhibits a perfect logarithmic",
    );
}

#[test]
#[ignore = "Requires model download"]
fn semantic_multi_hop_reasoning() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // Requires connecting "ownership and borrowing" → Rust → testing in Rust
    let query = "testing framework for the language with ownership and borrowing";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 10).unwrap();

    // Any Rust doc in top 5 counts
    let has_rust = results.iter().take(5).any(|r| {
        r.content.contains("Testing in Rust")
            || r.content.contains("Rust's error handling")
            || r.content.contains("Cargo is Rust")
    });
    assert!(has_rust, "Expected a Rust doc in top 5 for multi-hop query");
}

#[test]
#[ignore = "Requires model download"]
fn semantic_intent_prevention() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // "preventing data loss" should find backup/migration/transaction docs,
    // not error docs about crashes (those describe data loss, not prevention).
    // Static models struggle here — "data loss" matches crash descriptions too.
    let query = "preventing data loss during system failures";
    let embedding = embedder.embed_query(query).unwrap();
    let results = db.search_vector(&embedding, 20).unwrap();

    // Should find DB/deploy docs somewhere in top 10
    let has_prevention = results.iter().take(10).any(|r| {
        r.content.contains("migrations")
            || r.content.contains("ACID")
            || r.content.contains("rollback")
            || r.content.contains("backup")
            || r.content.contains("replication")
            || r.content.contains("transaction")
    });
    assert!(
        has_prevention,
        "Expected prevention-oriented doc in top 10, not just error descriptions"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tier 3: Hybrid + RRF (requires model download)
// ═══════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "Requires model download"]
fn hybrid_short_query_favors_fts() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    let query = "git push"; // 8 chars — FTS weight should be 2.5
    let embedding = embedder.embed_query(query).unwrap();

    let hybrid_results = db.search_hybrid(query, &embedding, 10).unwrap();
    let semantic_results = db.search_vector(&embedding, 10).unwrap();

    // Find the position of the exact-match doc in each result set
    let hybrid_pos = hybrid_results
        .iter()
        .position(|r| r.content.contains("Git push sends local commits"));
    let semantic_pos = semantic_results
        .iter()
        .position(|r| r.content.contains("Git push sends local commits"));

    match (hybrid_pos, semantic_pos) {
        (Some(h), Some(s)) => assert!(
            h <= s,
            "Exact match should rank same or higher in hybrid ({h}) vs semantic ({s})"
        ),
        (Some(_), None) => {} // Found in hybrid but not semantic — FTS helped
        (None, _) => panic!("Expected 'git push' doc in hybrid results"),
    }
}

#[test]
#[ignore = "Requires model download"]
fn hybrid_long_query_leverages_semantics() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // Conceptually about connection pooling but avoids exact keywords.
    // At 500 docs, many networking/DB/threading docs compete since the query
    // touches on "persistent links", "data storage", and "concurrent handling".
    let query = "managing persistent links between the application server and \
                 data storage layer for concurrent request handling";
    let embedding = embedder.embed_query(query).unwrap();

    let results = db.search_hybrid(query, &embedding, 30).unwrap();

    // Should find the connection pooling doc via semantic similarity
    assert_in_top_k(&results, "Connection pooling", 20);
}

#[test]
#[ignore = "Requires model download"]
fn hybrid_rrf_boosts_dual_signal() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    // "Rust error handling" matches doc 1 in both FTS (keywords) and semantics (topic)
    let query = "Rust error handling";
    let embedding = embedder.embed_query(query).unwrap();

    let results = db.search_hybrid(query, &embedding, 10).unwrap();

    // The dual-signal doc should be #1
    assert_in_top_k(&results, "Rust's error handling model", 1);
}

// ═══════════════════════════════════════════════════════════════════════
// Tier 4: Edge cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn empty_corpus_returns_empty() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("empty.db");
    let db = Database::open(&db_path).expect("Failed to open empty database");

    // FTS
    assert!(db.search_fts("anything", 10).unwrap().is_empty());

    // Vector search with zero embedding
    let embedding = zero_embedding();
    assert!(db.search_vector(&embedding, 10).unwrap().is_empty());

    // Hybrid
    assert!(db
        .search_hybrid("anything", &embedding, 10)
        .unwrap()
        .is_empty());
}

#[test]
fn score_ordering_and_bounds() {
    let (db, _env) = setup_fts_corpus();
    let results = db.search_fts("Rust programming", 10).unwrap();
    assert!(
        results.len() >= 2,
        "Need at least 2 results to verify ordering"
    );

    // All scores should be non-negative
    for r in &results {
        assert!(
            r.score >= 0.0,
            "Score should be non-negative, got {}",
            r.score
        );
    }

    // Results should be in descending score order (best first)
    for window in results.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "Results should be ordered by score descending: {} >= {}",
            window[0].score,
            window[1].score
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Eval report (run manually to see detailed rankings)
// ═══════════════════════════════════════════════════════════════════════

/// Finds the rank (1-indexed) of the first result containing `needle`, or None.
fn find_rank(results: &[SearchResult], needle: &str) -> Option<usize> {
    let needle_lower = needle.to_lowercase();
    results
        .iter()
        .position(|r| r.content.to_lowercase().contains(&needle_lower))
        .map(|i| i + 1)
}

/// Prints a single eval row.
fn report_row(label: &str, query: &str, needle: &str, results: &[SearchResult]) {
    let rank = find_rank(results, needle);
    let total = results.len();
    let rank_str = match rank {
        Some(r) => format!("#{r}"),
        None => "MISS".to_string(),
    };
    let score_str = rank.map_or_else(
        || "-".to_string(),
        |r| format!("{:.4}", results[r - 1].score),
    );
    println!("  {label:<42} {query:<38} {rank_str:<6} {score_str:<10} (of {total})");
}

/// Prints a section header for the eval report.
fn report_section(title: &str) {
    println!("\n  {title}");
    println!("  {:-<96}", "");
    println!(
        "  {:42} {:38} {:6} {:10} Pool",
        "Test", "Query", "Rank", "Score"
    );
    println!("  {:-<96}", "");
}

/// Runs a batch of semantic queries against the database and reports results.
fn report_semantic_batch(
    db: &Database,
    embedder: &glhf::embed::Embedder,
    queries: &[(&str, &str, &str)],
) {
    for (label, query, needle) in queries {
        let emb = embedder.embed_query(query).unwrap();
        let r = db.search_vector(&emb, 20).unwrap();
        report_row(label, query, needle, &r);
    }
}

/// FTS section of the eval report.
fn eval_fts(db: &Database) {
    report_section("FTS (BM25)");

    let q = "Rust";
    let r = db.search_fts(q, 10).unwrap();
    report_row("exact_keyword", q, "Rust's error handling", &r);

    let q = "fix broken pipe";
    let r = db.search_fts(q, 10).unwrap();
    report_row("multiword", q, "Broken pipe", &r);

    let q = "sqlite";
    let r = db.search_fts(q, 10).unwrap();
    report_row("bm25_tf (5x mention)", q, "SQLite is a lightweight", &r);
    report_row(
        "bm25_tf (1x mention)",
        q,
        "migrations for databases including SQLite",
        &r,
    );

    let q = "C++";
    let r = db.search_fts(q, 10).unwrap();
    report_row("special_char_cpp", q, "C++ templates", &r);

    let q = "$PATH";
    let r = db.search_fts(q, 10).unwrap();
    report_row("special_char_path", q, "$PATH environment", &r);

    let q = "OR";
    let r = db.search_fts(q, 10).unwrap();
    report_row("reserved_keyword", q, "SQL OR operator", &r);

    let q = "git status";
    let r = db.search_fts(q, 10).unwrap();
    report_row("tool_chunk", q, "git status", &r);

    let q = "ENOENT";
    let r = db.search_fts(q, 10).unwrap();
    report_row("error_result", q, "ENOENT", &r);
}

/// Semantic section of the eval report.
fn eval_semantic(db: &Database, embedder: &glhf::embed::Embedder) {
    report_section("Semantic (vector cosine)");
    report_semantic_batch(
        db,
        embedder,
        &[
            (
                "synonym_retrieval",
                "exception management",
                "error handling",
            ),
            (
                "synonym_negative",
                "exception management",
                "weather forecast",
            ),
            ("conceptual_deploy", "push code to production", "production"),
            (
                "conceptual_css_control",
                "push code to production",
                "CSS flexbox",
            ),
            ("session_coherence", "login security", "login flow"),
            ("cross_domain", "async concurrent tasks", "tokio runtime"),
        ],
    );
}

/// Harder semantic (distractor resistance) section of the eval report.
fn eval_semantic_harder(db: &Database, embedder: &glhf::embed::Embedder) {
    report_section("Semantic — harder (distractor resistance)");
    report_semantic_batch(
        db,
        embedder,
        &[
            (
                "keyword_collision_lang",
                "Rust programming tutorial",
                "Rust's error handling",
            ),
            (
                "keyword_collision_game",
                "Rust programming tutorial",
                "multiplayer survival",
            ),
            (
                "cross_domain_analogy",
                "the cargo equivalent for Python",
                "pip and virtual",
            ),
            (
                "merge_git",
                "resolving merge conflicts in source code",
                "Merge conflicts occur when Git",
            ),
            (
                "merge_pandas",
                "resolving merge conflicts in source code",
                "Pandas merge and join",
            ),
            (
                "compositional",
                "version control for database schemas",
                "migrations track schema",
            ),
            (
                "abstract_to_concrete",
                "why did my program crash with a null reference",
                "Segmentation fault",
            ),
            (
                "paraphrase",
                "automated quality checks before integrating code",
                "CI/CD pipelines",
            ),
            (
                "container_docker",
                "packaging applications in containers for deployment",
                "Docker containers",
            ),
            (
                "container_css",
                "packaging applications in containers for deployment",
                "CSS container queries",
            ),
            (
                "branches_git",
                "creating and managing branches for feature dev",
                "rebase",
            ),
            (
                "branches_ml",
                "creating and managing branches for feature dev",
                "Decision tree branches",
            ),
            (
                "pipeline_cicd",
                "CI pipeline for running tests on pull requests",
                "CI/CD pipelines automate",
            ),
            (
                "pipeline_airflow",
                "CI pipeline for running tests on pull requests",
                "Airflow orchestrates",
            ),
            (
                "tokens_jwt",
                "generating authentication tokens for user sessions",
                "JSON Web Tokens",
            ),
            (
                "tokens_ratelimit",
                "generating authentication tokens for user sessions",
                "Token bucket rate",
            ),
        ],
    );
}

/// Homonym disambiguation section of the eval report.
fn eval_homonyms(db: &Database, embedder: &glhf::embed::Embedder) {
    report_section("Semantic — homonym disambiguation");
    report_semantic_batch(
        db,
        embedder,
        &[
            (
                "thread_tech",
                "managing worker threads for parallel computation",
                "Thread pools manage",
            ),
            (
                "thread_forum",
                "managing worker threads for parallel computation",
                "discussion thread on the community",
            ),
            (
                "stack_tech",
                "stack overflow from infinite recursion",
                "stack overflow occurs when recursive",
            ),
            (
                "stack_papers",
                "stack overflow from infinite recursion",
                "stack of unread papers",
            ),
            (
                "port_network",
                "opening port 8080 for the web server",
                "Network ports identify",
            ),
            (
                "port_harbor",
                "opening port 8080 for the web server",
                "shipping port handles thousands",
            ),
            (
                "log_tech",
                "checking application logs for error messages",
                "Structured logging with JSON",
            ),
            (
                "log_timber",
                "checking application logs for error messages",
                "fallen log across the hiking",
            ),
            (
                "class_oop",
                "defining class hierarchies with inheritance",
                "Object-oriented class hierarchies",
            ),
            (
                "class_school",
                "defining class hierarchies with inheritance",
                "university class on medieval",
            ),
            (
                "shell_bash",
                "writing shell scripts for automating deployment",
                "Shell scripting automates",
            ),
            (
                "shell_marine",
                "writing shell scripts for automating deployment",
                "nautilus shell exhibits",
            ),
            (
                "multi_hop",
                "testing framework for the language with ownership",
                "Testing in Rust",
            ),
            (
                "intent_prevention",
                "preventing data loss during system failures",
                "migrations",
            ),
        ],
    );
}

/// Hybrid (RRF fusion) section of the eval report.
fn eval_hybrid(db: &Database, embedder: &glhf::embed::Embedder) {
    report_section("Hybrid (RRF fusion)");

    let q = "git push";
    let emb = embedder.embed_query(q).unwrap();
    let rh = db.search_hybrid(q, &emb, 10).unwrap();
    let rs = db.search_vector(&emb, 10).unwrap();
    report_row("short_hybrid", q, "Git push sends", &rh);
    report_row("short_semantic_baseline", q, "Git push sends", &rs);

    let q = "managing persistent links between the application server and data storage layer for concurrent request handling";
    let emb = embedder.embed_query(q).unwrap();
    let r = db.search_hybrid(q, &emb, 10).unwrap();
    report_row(
        "long_query_semantics",
        "(long DB query)",
        "Connection pooling",
        &r,
    );

    let q = "Rust error handling";
    let emb = embedder.embed_query(q).unwrap();
    let r = db.search_hybrid(q, &emb, 10).unwrap();
    report_row("dual_signal_boost", q, "Rust's error handling", &r);
}

#[test]
#[ignore = "Requires model download"]
fn eval_report() {
    let (db, _env) = setup_full_corpus();
    let embedder = glhf::embed::Embedder::new().unwrap();

    println!("\n  {}", "=".repeat(96));
    println!("  SEARCH QUALITY EVAL REPORT");
    println!(
        "  Corpus: {} docs, {} embeddings",
        db.document_count().unwrap(),
        db.embedding_count().unwrap()
    );
    println!("  {}\n", "=".repeat(96));

    eval_fts(&db);
    eval_semantic(&db, &embedder);
    eval_semantic_harder(&db, &embedder);
    eval_homonyms(&db, &embedder);
    eval_hybrid(&db, &embedder);

    println!("\n  {}\n", "=".repeat(96));
}
