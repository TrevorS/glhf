mod common;

use common::corpus::SearchCorpus;
use common::TestEnv;
use glhf::db::Database;
use glhf::db::SearchResult;
use model2vec_rs::model::StaticModel;
use std::time::Instant;

const MODELS: &[(&str, &str)] = &[
    ("base-32M", "minishlab/potion-base-32M"),
    ("retrieval-32M", "minishlab/potion-retrieval-32M"),
];

struct QueryCase {
    label: &'static str,
    query: &'static str,
    target: &'static str,
    category: &'static str,
}

fn eval_queries() -> Vec<QueryCase> {
    vec![
        // ── Claude recall queries (how glhf is actually used) ──────
        QueryCase {
            label: "recall-error-fix",
            query: "how did we fix the error handling in the Rust project",
            target: "Rust's error handling",
            category: "recall",
        },
        QueryCase {
            label: "recall-auth-session",
            query: "the session where we set up JWT authentication",
            target: "JSON Web Tokens",
            category: "recall",
        },
        QueryCase {
            label: "recall-deploy-cmd",
            query: "what docker command did we use to build the image",
            target: "docker build",
            category: "recall",
        },
        QueryCase {
            label: "recall-db-migration",
            query: "how we handled the database schema migration",
            target: "migrations track schema",
            category: "recall",
        },
        QueryCase {
            label: "recall-ci-setup",
            query: "when we configured the CI pipeline for tests",
            target: "CI/CD pipelines automate",
            category: "recall",
        },
        QueryCase {
            label: "recall-debug-crash",
            query: "the crash we debugged with the null pointer",
            target: "Segmentation fault",
            category: "recall",
        },
        // ── Synonym / paraphrase ───────────────────────────────────
        QueryCase {
            label: "synonym-errors",
            query: "exception management",
            target: "error handling",
            category: "synonym",
        },
        QueryCase {
            label: "paraphrase-cicd",
            query: "automated quality checks before integrating code",
            target: "CI/CD pipelines",
            category: "synonym",
        },
        QueryCase {
            label: "conceptual-deploy",
            query: "push code to production",
            target: "production",
            category: "synonym",
        },
        // ── Homonym disambiguation ─────────────────────────────────
        QueryCase {
            label: "thread-tech",
            query: "managing worker threads for parallel computation",
            target: "Thread pools manage",
            category: "homonym",
        },
        QueryCase {
            label: "stack-tech",
            query: "stack overflow from infinite recursion",
            target: "stack overflow occurs when recursive",
            category: "homonym",
        },
        QueryCase {
            label: "port-net",
            query: "opening port 8080 for the web server",
            target: "Network ports identify",
            category: "homonym",
        },
        QueryCase {
            label: "log-tech",
            query: "checking application logs for error messages",
            target: "Structured logging",
            category: "homonym",
        },
        QueryCase {
            label: "shell-bash",
            query: "writing shell scripts for automating deployment",
            target: "Shell scripting automates",
            category: "homonym",
        },
        // ── Distractor resistance ──────────────────────────────────
        QueryCase {
            label: "merge-git",
            query: "resolving merge conflicts in source code",
            target: "Merge conflicts occur when Git",
            category: "distractor",
        },
        QueryCase {
            label: "container-docker",
            query: "packaging applications in containers",
            target: "Docker containers",
            category: "distractor",
        },
        QueryCase {
            label: "pipeline-cicd",
            query: "CI pipeline for running tests on pull requests",
            target: "CI/CD pipelines automate",
            category: "distractor",
        },
        QueryCase {
            label: "tokens-jwt",
            query: "generating authentication tokens for user sessions",
            target: "JSON Web Tokens",
            category: "distractor",
        },
        // ── Tool/code queries ──────────────────────────────────────
        QueryCase {
            label: "tool-git-status",
            query: "git status",
            target: "git status",
            category: "tool",
        },
        QueryCase {
            label: "tool-cargo-test",
            query: "cargo test",
            target: "cargo test",
            category: "tool",
        },
        QueryCase {
            label: "tool-kubectl",
            query: "kubectl pods production",
            target: "kubectl get pods",
            category: "tool",
        },
        QueryCase {
            label: "tool-pytest",
            query: "python pytest",
            target: "pytest tests",
            category: "tool",
        },
        // ── Cross-domain / compositional ───────────────────────────
        QueryCase {
            label: "cross-domain",
            query: "the cargo equivalent for Python",
            target: "pip and virtual",
            category: "cross",
        },
        QueryCase {
            label: "compositional",
            query: "version control for database schemas",
            target: "migrations track schema",
            category: "cross",
        },
        // ── Hybrid advantage (short exact + long conceptual) ───────
        QueryCase {
            label: "short-exact",
            query: "git push",
            target: "Git push sends",
            category: "hybrid",
        },
        QueryCase {
            label: "dual-signal",
            query: "Rust error handling",
            target: "Rust's error handling",
            category: "hybrid",
        },
        QueryCase {
            label: "long-concept",
            query: "managing persistent database connections for concurrent web requests",
            target: "Connection pooling",
            category: "hybrid",
        },
    ]
}

fn find_rank(results: &[SearchResult], target: &str) -> Option<usize> {
    let lower = target.to_lowercase();
    results
        .iter()
        .position(|r| r.content.to_lowercase().contains(&lower))
        .map(|i| i + 1)
}

fn find_rank_in_local(results: &[(usize, f32, String)], target: &str) -> Option<usize> {
    let lower = target.to_lowercase();
    results
        .iter()
        .position(|(_i, _s, content)| content.to_lowercase().contains(&lower))
        .map(|i| i + 1)
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

fn search_vector_local(
    query_emb: &[f32],
    doc_embeddings: &[Vec<f32>],
    doc_contents: &[String],
    limit: usize,
) -> Vec<(usize, f32, String)> {
    let mut scores: Vec<(usize, f32)> = doc_embeddings
        .iter()
        .enumerate()
        .map(|(i, emb)| (i, cosine_similarity(query_emb, emb)))
        .collect();
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scores
        .into_iter()
        .take(limit)
        .map(|(i, s)| (i, s, doc_contents[i].clone()))
        .collect()
}

/// Simulates RRF hybrid by combining FTS and vector ranks.
fn hybrid_rank(fts_rank: Option<usize>, vec_rank: Option<usize>) -> Option<usize> {
    const K: f64 = 60.0;
    match (fts_rank, vec_rank) {
        (Some(f), Some(v)) => {
            let fts_rrf = 1.0 / (K + f as f64);
            let vec_rrf = 1.0 / (K + v as f64);
            let combined = fts_rrf + vec_rrf;
            Some(if combined > 0.0 { 1 } else { 999 })
        }
        (Some(r), None) | (None, Some(r)) => Some(r),
        (None, None) => None,
    }
}

fn rank_str(rank: Option<usize>) -> String {
    match rank {
        Some(r) => format!("#{r:<3}"),
        None => "MISS".to_string(),
    }
}

fn get_corpus_contents(db: &mut Database) -> Vec<String> {
    let broad_query =
        "a OR the OR is OR to OR of OR in OR for OR with OR and OR on OR error OR test";
    let results = db.search_fts(broad_query, 600).unwrap_or_default();
    results.into_iter().map(|r| r.content).collect()
}

#[test]
#[ignore = "Requires model downloads (~300MB total)"]
fn model_comparison_eval() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("model_cmp.db");
    let mut db = Database::open(&db_path).unwrap();
    let corpus = SearchCorpus::standard();
    corpus.insert_into(&mut db);

    let doc_count = db.document_count().unwrap();
    let queries = eval_queries();

    println!("\n  {}", "=".repeat(130));
    println!("  MODEL COMPARISON EVAL — FTS vs Semantic vs Hybrid");
    println!("  Corpus: {doc_count} docs, {} queries", queries.len());
    println!("  {}", "=".repeat(130));

    // FTS baseline
    println!("\n  FTS BASELINE");
    println!("  {:40} {:6}", "Query", "FTS");
    println!("  {}", "-".repeat(50));

    let mut fts_ranks: Vec<Option<usize>> = Vec::new();
    for q in &queries {
        let results = db.search_fts(q.query, 20).unwrap();
        let rank = find_rank(&results, q.target);
        fts_ranks.push(rank);
        println!("  {:40} {}", q.label, rank_str(rank));
    }

    let fts_hit5: usize = fts_ranks
        .iter()
        .filter(|r| r.is_some_and(|r| r <= 5))
        .count();
    println!("\n  FTS Hit@5: {fts_hit5}/{}\n", queries.len());

    // Per-model comparison
    for (model_name, model_id) in MODELS {
        println!("  {}", "=".repeat(130));
        println!("  MODEL: {model_name} ({model_id})");

        let start = Instant::now();
        let model = match StaticModel::from_pretrained(model_id, None, None, None) {
            Ok(m) => m,
            Err(e) => {
                println!("  SKIP: Failed to load: {e}");
                continue;
            }
        };
        let load_time = start.elapsed();
        let test_emb = model.encode(&["test".to_string()]);
        let dims = test_emb.first().map_or(0, Vec::len);
        println!(
            "  Loaded in {:.2}s, {dims} dimensions",
            load_time.as_secs_f64()
        );

        let corpus2 = SearchCorpus::standard();
        let doc_texts: Vec<String> = {
            let env2 = TestEnv::new();
            let db2_path = env2.index_dir.join(format!("cmp_{model_name}.db"));
            let mut db2 = Database::open(&db2_path).unwrap();
            corpus2.insert_into(&mut db2);
            get_corpus_contents(&mut db2)
        };

        let start = Instant::now();
        let doc_embeddings = model.encode(&doc_texts);
        let embed_time = start.elapsed();
        println!(
            "  Embedded {} docs in {:.2}s ({:.0} docs/sec)\n",
            doc_texts.len(),
            embed_time.as_secs_f64(),
            doc_texts.len() as f64 / embed_time.as_secs_f64()
        );

        // Header with all 3 modes
        println!(
            "  {:40} {:>8} {:>6} {:>6} {:>6} {:>8} {:>8}",
            "Query", "Cat", "FTS", "Vec", "Hybr", "FvV", "FvH"
        );
        println!("  {}", "-".repeat(90));

        let mut vec_hit5 = 0_usize;
        let mut hybr_hit5 = 0_usize;
        let mut vec_better = 0_usize;
        let mut fts_better_count = 0_usize;
        let mut hybr_better_than_fts = 0_usize;
        let mut hybr_worse_than_fts = 0_usize;

        for (i, q) in queries.iter().enumerate() {
            let query_emb = model.encode(&[q.query.to_string()]);
            let query_vec = &query_emb[0];

            let vec_results = search_vector_local(query_vec, &doc_embeddings, &doc_texts, 20);
            let vec_rank = find_rank_in_local(&vec_results, q.target);
            let fts_rank = fts_ranks[i];
            let hybr = hybrid_rank(fts_rank, vec_rank);

            if vec_rank.is_some_and(|r| r <= 5) {
                vec_hit5 += 1;
            }
            if hybr.is_some_and(|r| r <= 5) {
                hybr_hit5 += 1;
            }

            // FTS vs Vec delta
            let fvv = match (fts_rank, vec_rank) {
                (Some(f), Some(v)) => {
                    let d = f as i32 - v as i32;
                    if d > 0 {
                        vec_better += 1;
                    } else if d < 0 {
                        fts_better_count += 1;
                    }
                    format!("{d:+}")
                }
                (None, Some(_)) => {
                    vec_better += 1;
                    "+MISS".to_string()
                }
                (Some(_), None) => {
                    fts_better_count += 1;
                    "-MISS".to_string()
                }
                (None, None) => "BOTH".to_string(),
            };

            // FTS vs Hybrid delta
            let fvh = match (fts_rank, hybr) {
                (Some(f), Some(h)) => {
                    let d = f as i32 - h as i32;
                    if d > 0 {
                        hybr_better_than_fts += 1;
                    } else if d < 0 {
                        hybr_worse_than_fts += 1;
                    }
                    format!("{d:+}")
                }
                (None, Some(_)) => {
                    hybr_better_than_fts += 1;
                    "+MISS".to_string()
                }
                (Some(_), None) => {
                    hybr_worse_than_fts += 1;
                    "-MISS".to_string()
                }
                (None, None) => "BOTH".to_string(),
            };

            println!(
                "  {:40} {:>8} {:>6} {:>6} {:>6} {:>8} {:>8}",
                q.label,
                q.category,
                rank_str(fts_rank),
                rank_str(vec_rank),
                rank_str(hybr),
                fvv,
                fvh
            );
        }

        let total = queries.len();
        let ties = total - vec_better - fts_better_count;
        println!("\n  Summary for {model_name}:");
        println!("  Hit@5:    FTS={fts_hit5}/{total}  Vec={vec_hit5}/{total}  Hybrid={hybr_hit5}/{total}");
        println!("  FTS vs Vec:    FTS wins={fts_better_count}  Vec wins={vec_better}  Tie={ties}");
        println!(
            "  FTS vs Hybrid: Hybrid helps={hybr_better_than_fts}  Hybrid hurts={hybr_worse_than_fts}\n"
        );
    }

    println!("  {}\n", "=".repeat(130));
}
