---
name: rust-tests
description: Run and debug Rust tests for glhf. Use when running tests, fixing test failures, or adding new test cases.
---

# Rust Testing Guide

## Running Tests

### All Tests
```bash
cargo test
```

### With Environment (for embedding tests)
```bash
export ORT_LIB_LOCATION="$HOME/.cache/glhf/onnxruntime-linux-x64-1.20.0/lib"
export ORT_STRATEGY=system
export LD_LIBRARY_PATH="$ORT_LIB_LOCATION:$LD_LIBRARY_PATH"
export HF_HOME="$HOME/.cache/huggingface/hub"

cargo test
cargo test embed -- --ignored  # Run embedding tests
```

### Specific Tests
```bash
cargo test test_name           # Run tests matching name
cargo test --lib               # Only library tests
cargo test --test integration  # Only integration tests
cargo test -- --nocapture      # Show println output
```

## Test Structure

```
tests/
├── integration.rs      # End-to-end tests
└── common/
    └── mod.rs          # Test utilities (TestEnv, fixtures)

src/
├── db/mod.rs          # Unit tests in #[cfg(test)] mod tests
├── embed.rs           # Embedding tests (#[ignore] for CI)
└── models/document.rs # Document parsing tests
```

## Test Utilities

### TestEnv (tests/common/mod.rs)
```rust
let env = TestEnv::new();                    // Creates temp directory
let project = env.create_project("path");    // Creates project dir
let jsonl = env.write_jsonl(&project, "file.jsonl", &lines);
```

### Fixtures
```rust
fn user_message(content: &str, session: &str) -> String
fn assistant_message(content: &str, session: &str) -> String
fn tool_use_message(tool: &str, input: Value, session: &str) -> String
fn tool_result_message(content: &str, tool_id: &str, session: &str) -> String
```

## Test Categories

| Category | Location | Run Command |
|----------|----------|-------------|
| Unit tests | `src/**/*.rs` | `cargo test --lib` |
| Integration | `tests/integration.rs` | `cargo test --test integration` |
| Doc tests | `src/**/*.rs` | `cargo test --doc` |
| Embedding | `src/embed.rs` | `cargo test embed -- --ignored` |
| Benchmarks | `benches/indexing.rs` | `cargo bench` |

## Writing Tests

### Database Tests
```rust
#[test]
fn test_database_operation() {
    let db = Database::open_in_memory().unwrap();

    let doc = Document::new(
        ChunkKind::Message,
        "test content".to_string(),
        PathBuf::from("/test"),
    );

    db.insert_document(&doc).unwrap();

    let results = db.search_fts("test", 10).unwrap();
    assert_eq!(results.len(), 1);
}
```

### Embedding Tests (ignored by default)
```rust
#[test]
#[ignore]  // Requires model files
fn test_embedding() {
    let embedder = Embedder::new().unwrap();
    let embedding = embedder.embed_query("test").unwrap();
    assert_eq!(embedding.len(), 384);
}
```

## Common Failures

| Error | Cause | Fix |
|-------|-------|-----|
| `ONNX not found` | Missing runtime | Run setup-models.sh |
| `model.onnx not found` | Missing model | Download from HuggingFace |
| `sqlite-vec not loaded` | Init order | Call init_sqlite_vec() before open |
| `FTS5 match failed` | Bad query syntax | Escape special chars |
