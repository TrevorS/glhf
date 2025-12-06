use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use glhf::index::BM25Index;
use glhf::models::document::{DocType, Document};
use std::hint::black_box;
use std::path::PathBuf;
use tempfile::TempDir;

fn generate_docs(count: usize) -> Vec<Document> {
    (0..count)
        .map(|i| {
            Document::new(
                DocType::Conversation,
                format!(
                    "This is document number {} with some searchable content about Rust programming and systems design",
                    i
                ),
                PathBuf::from(format!("/test/{}.jsonl", i)),
            )
            .with_role(Some(if i % 2 == 0 { "user" } else { "assistant" }.to_string()))
            .with_session_id(Some(format!("session-{}", i / 10)))
        })
        .collect()
}

fn bench_indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexing");

    for size in [100, 1000, 5000] {
        let docs = generate_docs(size);

        group.bench_with_input(BenchmarkId::new("add_documents", size), &docs, |b, docs| {
            b.iter_with_setup(
                || {
                    let temp_dir = TempDir::new().unwrap();
                    let index = BM25Index::create(temp_dir.path()).unwrap();
                    (temp_dir, index)
                },
                |(_temp_dir, index)| {
                    let mut writer = index.writer().unwrap();
                    index.add_documents(&mut writer, black_box(docs)).unwrap();
                    writer.commit().unwrap();
                },
            );
        });
    }

    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    // Setup: create index with 5000 docs
    let temp_dir = TempDir::new().unwrap();
    let docs = generate_docs(5000);
    let index = BM25Index::create(temp_dir.path()).unwrap();
    let mut writer = index.writer().unwrap();
    index.add_documents(&mut writer, &docs).unwrap();
    writer.commit().unwrap();
    index.reload().unwrap();

    let queries = ["Rust", "programming", "systems design", "document number"];

    for query in queries {
        group.bench_with_input(BenchmarkId::new("query", query), &query, |b, query| {
            b.iter(|| index.search(black_box(query), 10).unwrap());
        });
    }

    // Bench different result limits
    for limit in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::new("limit", limit), &limit, |b, limit| {
            b.iter(|| index.search("programming", black_box(*limit)).unwrap());
        });
    }

    group.finish();
}

criterion_group!(benches, bench_indexing, bench_search);
criterion_main!(benches);
