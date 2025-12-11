# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
make check          # Format, lint, and test (run before commits)
make build          # Build debug binary
make release        # Build release binary
make install        # Install to ~/.cargo/bin
cargo test <name>   # Run a single test by name
cargo bench         # Run benchmarks
```

## Architecture

glhf is a CLI tool for searching Claude Code conversation history using hybrid search (BM25 + semantic vectors).

### Data Flow

1. **Ingest** (`ingest/`) - Walks `~/.claude/projects/` and parses JSONL conversation files
2. **Document** (`document.rs`) - Extracts chunks: messages, tool_use, tool_result
3. **Embed** (`embed.rs`) - Generates 512-dim embeddings via model2vec-rs (Potion-base-32M)
4. **Database** (`db/mod.rs`) - Stores in SQLite with FTS5 + sqlite-vec for hybrid search
5. **Commands** (`commands.rs`) - CLI handlers for index, search, status

### Key Design Decisions

- **sqlite-vec FFI**: Uses `sqlite3_auto_extension` with a `Once` guard to register the extension before any connection opens. The unsafe transmute is required due to FFI signature differences.

- **Hybrid Search**: Combines FTS5 BM25 scores with vector cosine distance using Reciprocal Rank Fusion (RRF). Each search mode fetches 2x the limit, then fuses/truncates.

- **Path Encoding**: Claude Code encodes project paths in directory names: `/` becomes `-`, `/.` becomes `--`. The `config.rs` module handles decoding.

- **Chunk Types**: Three indexed types with shared `DisplayLabel` trait for consistent formatting across `Document` and `SearchResult`.

### Module Responsibilities

| Module | Purpose |
|--------|---------|
| `commands` | CLI command handlers (index, search, status) |
| `config` | Database paths, Claude directory discovery |
| `db` | SQLite with FTS5 + sqlite-vec, search methods |
| `document` | Document struct, ChunkKind enum, DisplayLabel trait |
| `embed` | Embedder wrapper around model2vec-rs |
| `error` | Custom error types with thiserror |
| `ingest` | JSONL parsing, project directory walking |
| `utils` | Shared utilities (truncate_text) |

## Testing

Integration tests in `tests/integration.rs` cover the full pipeline. Unit tests are co-located with modules. Embedding tests are `#[ignore]` tagged since they require model download.
