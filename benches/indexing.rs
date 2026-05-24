use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use glhf::db::{Database, EMBEDDING_DIM};
use glhf::{ChunkKind, Document};
use std::hint::black_box;
use std::path::PathBuf;
use tempfile::TempDir;

fn generate_mixed_docs(count: usize) -> Vec<Document> {
    (0..count)
        .map(|i| {
            let (kind, content, tool) = match i % 3 {
                0 => (
                    ChunkKind::Message,
                    format!("User message {i} about Rust programming and error handling patterns"),
                    None,
                ),
                1 => (
                    ChunkKind::ToolUse,
                    "cargo test --all --release".to_string(),
                    Some("Bash"),
                ),
                _ => (
                    ChunkKind::ToolResult,
                    "test result: 10 passed, 0 failed\nfinished in 2.3s".to_string(),
                    Some("Bash"),
                ),
            };
            let mut doc = Document::new(kind, content, PathBuf::from(format!("/test/{i}.jsonl")))
                .with_session_id(Some(format!("session-{}", i / 10)))
                .with_project(Some(format!("project-{}", i % 3)));
            if let Some(t) = tool {
                doc = doc
                    .with_tool_name(Some(t.to_string()))
                    .with_tool_id(Some(format!("tool-{i}")));
            } else {
                doc = doc.with_role(Some(
                    if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                ));
            }
            doc
        })
        .collect()
}

fn fake_embeddings(count: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|i| {
            (0..EMBEDDING_DIM)
                .map(|j| ((i * 17 + j * 31) % 1000) as f32 / 1000.0)
                .collect()
        })
        .collect()
}

fn setup_db_with_docs(count: usize) -> (TempDir, Database) {
    let tmp = TempDir::new().unwrap();
    let mut db = Database::open(&tmp.path().join("bench.db")).unwrap();
    let docs = generate_mixed_docs(count);
    db.insert_documents(&docs).unwrap();
    (tmp, db)
}

fn setup_db_with_embeddings(count: usize) -> (TempDir, Database) {
    let tmp = TempDir::new().unwrap();
    let mut db = Database::open(&tmp.path().join("bench.db")).unwrap();
    let docs = generate_mixed_docs(count);
    db.insert_documents(&docs).unwrap();

    let embs = fake_embeddings(count);
    let pairs: Vec<(&str, &[f32])> = docs
        .iter()
        .zip(embs.iter())
        .map(|(d, e)| (d.id.as_str(), e.as_slice()))
        .collect();
    db.insert_embeddings(&pairs).unwrap();
    (tmp, db)
}

// --- Indexing benchmarks ---

fn bench_insert_documents(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_documents");

    for size in [100, 1000, 5000] {
        let docs = generate_mixed_docs(size);

        group.bench_with_input(BenchmarkId::new("mixed", size), &docs, |b, docs| {
            b.iter_with_setup(
                || {
                    let tmp = TempDir::new().unwrap();
                    let db = Database::open(&tmp.path().join("b.db")).unwrap();
                    (tmp, db)
                },
                |(_tmp, mut db)| {
                    db.insert_documents(black_box(docs)).unwrap();
                },
            );
        });
    }
    group.finish();
}

fn bench_insert_embeddings(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_embeddings");

    for size in [100, 1000, 5000] {
        let docs = generate_mixed_docs(size);
        let embs = fake_embeddings(size);

        group.bench_with_input(BenchmarkId::new("vectors", size), &size, |b, _| {
            b.iter_with_setup(
                || {
                    let tmp = TempDir::new().unwrap();
                    let mut db = Database::open(&tmp.path().join("b.db")).unwrap();
                    db.insert_documents(&docs).unwrap();
                    (tmp, db)
                },
                |(_tmp, mut db)| {
                    let pairs: Vec<(&str, &[f32])> = docs
                        .iter()
                        .zip(embs.iter())
                        .map(|(d, e)| (d.id.as_str(), e.as_slice()))
                        .collect();
                    db.insert_embeddings(black_box(&pairs)).unwrap();
                },
            );
        });
    }
    group.finish();
}

fn bench_fts_rebuild(c: &mut Criterion) {
    let mut group = c.benchmark_group("fts_rebuild");

    for size in [1000, 5000] {
        group.bench_with_input(BenchmarkId::new("rebuild", size), &size, |b, &size| {
            b.iter_with_setup(
                || {
                    let tmp = TempDir::new().unwrap();
                    let mut db = Database::open(&tmp.path().join("b.db")).unwrap();
                    db.drop_fts_triggers().unwrap();
                    let docs = generate_mixed_docs(size);
                    db.insert_documents(&docs).unwrap();
                    (tmp, db)
                },
                |(_tmp, db)| {
                    db.rebuild_fts().unwrap();
                },
            );
        });
    }
    group.finish();
}

// --- Search benchmarks ---

fn bench_fts_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("fts_search");
    let (_tmp, db) = setup_db_with_docs(5000);

    for query in [
        "Rust",
        "cargo test",
        "error handling",
        "programming patterns",
    ] {
        group.bench_with_input(BenchmarkId::new("query", query), &query, |b, q| {
            b.iter(|| db.search_fts(black_box(q), 10).unwrap());
        });
    }

    for limit in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::new("limit", limit), &limit, |b, &limit| {
            b.iter(|| db.search_fts("Rust", black_box(limit)).unwrap());
        });
    }
    group.finish();
}

fn bench_vector_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_search");
    let (_tmp, db) = setup_db_with_embeddings(5000);

    let query_emb: Vec<f32> = (0..EMBEDDING_DIM)
        .map(|i| (i % 100) as f32 / 100.0)
        .collect();

    for limit in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::new("limit", limit), &limit, |b, &limit| {
            b.iter(|| {
                db.search_vector(black_box(&query_emb), black_box(limit))
                    .unwrap()
            });
        });
    }
    group.finish();
}

fn bench_hybrid_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("hybrid_search");
    let (_tmp, db) = setup_db_with_embeddings(5000);

    let query_emb: Vec<f32> = (0..EMBEDDING_DIM)
        .map(|i| (i % 100) as f32 / 100.0)
        .collect();

    for query in ["Rust", "cargo test", "error handling patterns"] {
        group.bench_with_input(BenchmarkId::new("query", query), &query, |b, q| {
            b.iter(|| {
                db.search_hybrid(black_box(q), black_box(&query_emb), 10)
                    .unwrap()
            });
        });
    }
    group.finish();
}

fn bench_filtered_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("filtered_search");
    let (_tmp, db) = setup_db_with_docs(5000);

    group.bench_function("by_chunk_kind", |b| {
        b.iter(|| {
            db.search_fts_filtered(black_box("test"), 10, Some(ChunkKind::Message), None, false)
                .unwrap()
        });
    });

    group.bench_function("by_tool_name", |b| {
        b.iter(|| {
            db.search_fts_filtered(black_box("test"), 10, None, Some("Bash"), false)
                .unwrap()
        });
    });

    group.bench_function("errors_only", |b| {
        b.iter(|| {
            db.search_fts_filtered(black_box("test"), 10, None, None, true)
                .unwrap()
        });
    });

    group.finish();
}

// --- Incremental + session benchmarks ---

fn bench_incremental(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental");
    let (_tmp, mut db) = setup_db_with_docs(5000);

    for i in 0..5000 {
        db.upsert_file_meta(&format!("/test/{i}.jsonl"), 1000, 1)
            .unwrap();
    }

    group.bench_function("get_indexed_files", |b| {
        b.iter(|| black_box(db.get_indexed_files().unwrap()));
    });

    group.bench_function("documents_without_embeddings", |b| {
        b.iter(|| black_box(db.documents_without_embeddings().unwrap()));
    });

    let replacement = generate_mixed_docs(20);
    group.bench_function("delete_reinsert_one_file", |b| {
        b.iter(|| {
            db.delete_documents_by_source(black_box("/test/0.jsonl"))
                .unwrap();
            db.insert_documents(black_box(&replacement)).unwrap();
        });
    });

    group.finish();
}

fn bench_session_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_queries");
    let (_tmp, db) = setup_db_with_docs(5000);

    group.bench_function("recent_sessions", |b| {
        b.iter(|| black_box(db.get_recent_sessions(10, None).unwrap()));
    });

    group.bench_function("recent_by_project", |b| {
        b.iter(|| black_box(db.get_recent_sessions(10, Some("alpha")).unwrap()));
    });

    group.bench_function("find_sessions", |b| {
        b.iter(|| black_box(db.find_sessions(black_box("session-5")).unwrap()));
    });

    group.bench_function("get_session_messages", |b| {
        b.iter(|| black_box(db.get_session_messages(black_box("session-10")).unwrap()));
    });

    group.bench_function("status_stats", |b| {
        b.iter(|| black_box(db.status_stats().unwrap()));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_insert_documents,
    bench_insert_embeddings,
    bench_fts_rebuild,
    bench_fts_search,
    bench_vector_search,
    bench_hybrid_search,
    bench_filtered_search,
    bench_incremental,
    bench_session_queries
);
criterion_main!(benches);
