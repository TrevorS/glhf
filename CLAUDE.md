# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

direct-commits-allowed: true

## Build & Development Commands

```bash
make check          # Format, lint, and test (run before commits)
make build          # Build debug binary
make release        # Build release binary
make install        # Install to ~/.cargo/bin
make prop           # Run property tests only
make fuzz           # Run primary fuzz target for 60s (requires nightly)
cargo test <name>   # Run a single test by name
cargo bench         # Run benchmarks
```

## Using glhf (for Claude)

glhf searches your Claude Code conversation history. Use it to find past solutions and recall commands.

**Index management:** Search prints a staleness note when files have changed since the last index. Run `glhf index` for a fast incremental update or `glhf index --full` to rebuild.

### Quick Reference

```bash
# Search with compact output (fewer tokens)
glhf search "error handling" --compact -l 10

# Filter by tool type
glhf search "deploy" -t Bash --compact

# Filter by current project
glhf search "test" -p . --compact

# Filter by time
glhf search "error" --errors --since 1d --compact

# Quick session overview
glhf session abc123 --summary

# Get limited context from a session
glhf session abc123 --limit 30

# Recent sessions
glhf recent -p myproject
```

### Recommended Patterns

**Finding past solutions:**
```bash
glhf search "problem description" --compact
glhf session <id> --summary
```

**Recalling commands:**
```bash
glhf search "cargo clippy" -t Bash --compact
glhf search "git" -t Bash --since 1w --compact
```

**Finding errors:**
```bash
glhf search "error" --errors --since 1d --compact
```

### All Search Flags

These are the ONLY valid flags for `glhf search`. Do not invent others.

| Flag | Short | Description |
|------|-------|-------------|
| `--limit N` | `-l N` | Max results (default: 10) |
| `--tool NAME` | `-t NAME` | Filter by tool (Bash, Read, Edit, etc.) |
| `--project NAME` | `-p NAME` | Filter by project (use `.` for current) |
| `--since DURATION` | | Time filter (1h, 2d, 1w, or 2024-12-01) |
| `--errors` | | Only show error results |
| `--json` | | Machine-readable JSON output |
| `--compact` | | One line per result |

### Tips

1. **Use `--compact` by default** - reduces output tokens significantly
2. **Chain commands**: search → view session summary → get context
3. **Use `-p .`** to filter to current project
4. **Use `--since`** to focus on recent history (1h, 1d, 1w)

## Architecture

glhf is a CLI tool for searching Claude Code conversation history using hybrid search (BM25 + semantic vectors).

### Data Flow

1. **Ingest** (`ingest/`) - Walks `~/.claude/projects/` and parses JSONL conversation files
2. **Document** (`document.rs`) - Extracts chunks: messages, tool_use, tool_result
3. **Embed** (`embed.rs`) - Generates 512-dim embeddings via model2vec-rs (Potion-base-32M)
4. **Database** (`db/mod.rs`) - Stores in SQLite with FTS5 + sqlite-vec for hybrid search
5. **Format** (`format.rs`) - Display formatting, time/size helpers, project name extraction
6. **Commands** (`commands.rs`) - CLI handlers for index, search, status, session, recent

### Key Design Decisions

- **sqlite-vec FFI**: Uses `sqlite3_auto_extension` with a `Once` guard to register the extension before any connection opens. The unsafe transmute is required due to FFI signature differences.

- **Hybrid Search**: Combines FTS5 BM25 scores with vector cosine similarity using convex combination (CC) with alpha=0.75 (75% FTS, 25% vector). Min-max normalization maps both score types to [0,1] before combining. Evaluated on 1K queries across StackOverflow-QA and CodeFeedback-MT — outperforms RRF by +8.9% MRR and FTS-only by +1.2% MRR.

- **Embedding Model**: potion-base-32M (512 dimensions) via model2vec-rs. Static embeddings (token lookup + mean pooling) — no transformer inference, ~4600 docs/sec on CPU. Evaluated against potion-retrieval-32M, potion-code-16M, static-retrieval-mrl-en-v1, and ONNX models (arctic-embed-xs, MiniLM, bge-small). base-32M wins on our eval (27/27 hybrid hit@5).

- **Path Encoding**: Claude Code encodes project paths in directory names: `/` becomes `-`, `/.` becomes `--`. The encoding is lossy, so we store raw encoded paths and extract display names via pattern matching.

- **Incremental Indexing**: The `index_meta` table tracks `(source_path, mtime_secs, doc_count)` per file. Only files with changed mtimes are re-parsed. `--full` deletes the DB and rebuilds.

- **SQLite Performance**: WAL mode, `synchronous=NORMAL`, 64MB cache. Bulk indexing drops FTS triggers and rebuilds the FTS index in one pass (5x faster than per-row triggers). Freshness check uses file-count comparison instead of per-file mtime scanning.

### Module Responsibilities

| Module | Purpose |
|--------|---------|
| `main` | CLI argument parsing with clap |
| `commands` | CLI command handlers (index, search, status, session, recent) |
| `config` | Database paths, Claude directory discovery |
| `db` | SQLite with FTS5 + sqlite-vec, hybrid search with CC fusion |
| `document` | Document struct, ChunkKind enum, DisplayLabel trait |
| `embed` | Embedder wrapper around model2vec-rs |
| `error` | Custom error types with thiserror |
| `format` | Display formatting, time/size/number helpers, result printing |
| `ingest` | JSONL parsing, project directory walking |
| `utils` | Shared utilities (truncate_text) |

## Testing

Unit tests are co-located with modules. Integration tests cover the full ingest/search pipeline. Search quality eval tests validate retrieval relevance.

```bash
cargo test                                       # Unit + integration + FTS quality tests
cargo test test_fts_search                       # Run specific test by name
cargo test --test search_quality                 # FTS search quality only (fast, no model)
cargo test --test search_quality -- --ignored    # Full eval: semantic + hybrid (requires model)
cargo test -- --ignored                          # All ignored tests (requires model)
```

### Search Quality Tests

`tests/search_quality.rs` has a 500-doc synthetic corpus with FTS, semantic, and hybrid tiers. Semantic/hybrid tests require model download and are `#[ignore]` tagged.

### Property Tests

Property tests use `proptest` and verify invariants like "never panics on arbitrary input", score normalization bounds, and CC fusion ordering.

### Fuzz Testing

Fuzz targets live in `fuzz/` (separate workspace, requires `cargo-fuzz` + nightly):

```bash
cargo +nightly fuzz run fuzz_fts_escape -- -max_total_time=60
cargo +nightly fuzz run fuzz_parse_jsonl -- -max_total_time=60
cargo +nightly fuzz run fuzz_truncate -- -max_total_time=60
cargo +nightly fuzz run fuzz_decode_path -- -max_total_time=60
```
