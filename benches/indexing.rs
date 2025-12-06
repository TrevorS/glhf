use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use glhf::index::BM25Index;
use glhf::models::document::{ChunkKind, Document};
use std::hint::black_box;
use std::path::PathBuf;
use tempfile::TempDir;

fn generate_message_docs(count: usize) -> Vec<Document> {
    (0..count)
        .map(|i| {
            Document::new(
                ChunkKind::Message,
                format!(
                    "This is document number {i} with some searchable content about Rust programming and systems design"
                ),
                PathBuf::from(format!("/test/{i}.jsonl")),
            )
            .with_role(Some(if i % 2 == 0 { "user" } else { "assistant" }.to_string()))
            .with_session_id(Some(format!("session-{}", i / 10)))
        })
        .collect()
}

fn generate_tool_docs(count: usize) -> Vec<Document> {
    (0..count)
        .map(|i| {
            let (chunk_kind, tool_name, content, is_error) = match i % 4 {
                0 => (
                    ChunkKind::ToolUse,
                    "Bash",
                    "git status && cargo build --release".to_string(),
                    None,
                ),
                1 => (
                    ChunkKind::ToolResult,
                    "Bash",
                    "On branch main\nnothing to commit, working tree clean".to_string(),
                    Some(false),
                ),
                2 => (
                    ChunkKind::ToolUse,
                    "Read",
                    "/home/user/project/src/main.rs".to_string(),
                    None,
                ),
                _ => (
                    ChunkKind::ToolResult,
                    "Read",
                    "fn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
                    Some(false),
                ),
            };
            Document::new(chunk_kind, content, PathBuf::from(format!("/test/{i}.jsonl")))
                .with_tool_name(Some(tool_name.to_string()))
                .with_tool_id(Some(format!("tool-{i}")))
                .with_is_error(is_error)
                .with_session_id(Some(format!("session-{}", i / 10)))
        })
        .collect()
}

fn generate_mixed_docs(count: usize) -> Vec<Document> {
    (0..count)
        .map(|i| {
            if i % 3 == 0 {
                // Message
                Document::new(
                    ChunkKind::Message,
                    format!("User message {i} about Rust programming"),
                    PathBuf::from(format!("/test/{i}.jsonl")),
                )
                .with_role(Some("user".to_string()))
                .with_session_id(Some(format!("session-{}", i / 10)))
            } else if i % 3 == 1 {
                // ToolUse
                Document::new(
                    ChunkKind::ToolUse,
                    "cargo test --all".to_string(),
                    PathBuf::from(format!("/test/{i}.jsonl")),
                )
                .with_tool_name(Some("Bash".to_string()))
                .with_tool_id(Some(format!("tool-{i}")))
                .with_session_id(Some(format!("session-{}", i / 10)))
            } else {
                // ToolResult
                Document::new(
                    ChunkKind::ToolResult,
                    "test result: 10 passed, 0 failed".to_string(),
                    PathBuf::from(format!("/test/{i}.jsonl")),
                )
                .with_tool_name(Some("Bash".to_string()))
                .with_tool_id(Some(format!("tool-{}", i - 1)))
                .with_is_error(Some(false))
                .with_session_id(Some(format!("session-{}", i / 10)))
            }
        })
        .collect()
}

fn bench_indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexing");

    for size in [100, 1000, 5000] {
        let message_docs = generate_message_docs(size);
        let tool_docs = generate_tool_docs(size);
        let mixed_docs = generate_mixed_docs(size);

        group.bench_with_input(
            BenchmarkId::new("messages", size),
            &message_docs,
            |b, docs| {
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
            },
        );

        group.bench_with_input(BenchmarkId::new("tools", size), &tool_docs, |b, docs| {
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

        group.bench_with_input(BenchmarkId::new("mixed", size), &mixed_docs, |b, docs| {
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

    // Setup: create index with mixed docs (messages + tools)
    let temp_dir = TempDir::new().unwrap();
    let docs = generate_mixed_docs(5000);
    let index = BM25Index::create(temp_dir.path()).unwrap();
    let mut writer = index.writer().unwrap();
    index.add_documents(&mut writer, &docs).unwrap();
    writer.commit().unwrap();
    index.reload().unwrap();

    let queries = ["Rust", "programming", "cargo", "test"];

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

fn bench_filtered_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("filtered_search");

    // Setup: create index with mixed docs
    let temp_dir = TempDir::new().unwrap();
    let docs = generate_mixed_docs(5000);
    let index = BM25Index::create(temp_dir.path()).unwrap();
    let mut writer = index.writer().unwrap();
    index.add_documents(&mut writer, &docs).unwrap();
    writer.commit().unwrap();
    index.reload().unwrap();

    // Filter by chunk kind
    group.bench_function("messages_only", |b| {
        b.iter(|| {
            index
                .search_filtered(black_box("test"), 10, Some(ChunkKind::Message), None, false)
                .unwrap()
        });
    });

    group.bench_function("tool_use_only", |b| {
        b.iter(|| {
            index
                .search_filtered(black_box("cargo"), 10, Some(ChunkKind::ToolUse), None, false)
                .unwrap()
        });
    });

    group.bench_function("tool_result_only", |b| {
        b.iter(|| {
            index
                .search_filtered(black_box("passed"), 10, Some(ChunkKind::ToolResult), None, false)
                .unwrap()
        });
    });

    // Filter by tool name
    group.bench_function("by_tool_name", |b| {
        b.iter(|| {
            index
                .search_filtered(black_box("test"), 10, None, Some("Bash"), false)
                .unwrap()
        });
    });

    // Regex search
    group.bench_function("regex", |b| {
        b.iter(|| index.search_regex(black_box("cargo.*test"), 10, false).unwrap());
    });

    group.bench_function("regex_ignore_case", |b| {
        b.iter(|| index.search_regex(black_box("CARGO.*TEST"), 10, true).unwrap());
    });

    group.finish();
}

criterion_group!(benches, bench_indexing, bench_search, bench_filtered_search);
criterion_main!(benches);
