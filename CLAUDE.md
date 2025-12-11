# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Build & Development Commands

```bash
make check          # Format, lint, and test (run before commits)
make build          # Build debug binary
make release        # Build release binary
make install        # Install to ~/.cargo/bin
cargo test <name>   # Run a single test by name
cargo bench         # Run benchmarks
```

## Using glhf (for Claude)

glhf searches your Claude Code conversation history. Use it to find past solutions, recall commands, and discover related work.

### Quick Reference

```bash
# Search with compact output (fewer tokens)
glhf search "error handling" --compact -l 10

# Find past solutions semantically
glhf search "authentication" --mode semantic --compact

# Get session ID to explore further
glhf search "bug fix" --show-session-id --compact

# Quick session overview
glhf session abc123 --summary

# Get limited context from a session
glhf session abc123 --limit 30

# Find related past sessions
glhf related abc123 --limit 5

# See all projects
glhf projects

# Filter by current project
glhf search "test" -p . --compact

# Filter by tool type
glhf search "deploy" -t Bash --compact
```

### Recommended Patterns

**Finding past solutions:**
```bash
glhf search "problem description" --mode semantic --compact
glhf search "specific keyword" --show-session-id
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

**Exploring related work:**
```bash
glhf related <session-id> --limit 5
```

### Output Modes

| Flag | Use Case |
|------|----------|
| `--compact` | Quick scanning, fewer tokens |
| `--json` | Machine-readable, structured data |
| `--show-session-id` | Get IDs to explore sessions |
| `--summary` | Session overview without full content |
| `--limit N` | Control output size |

### Tips

1. **Use `--compact` by default** - reduces output tokens significantly
2. **Use `--mode semantic`** for conceptual searches ("how to handle X")
3. **Use `--mode text`** for exact keyword matching
4. **Chain commands**: search → get session ID → view summary → get context
5. **Use `-p .`** to filter to current project
6. **Use `--since`** to focus on recent history (1h, 1d, 1w)

## Architecture

glhf is a CLI tool for searching Claude Code conversation history using hybrid search (BM25 + semantic vectors).

### Data Flow

1. **Ingest** (`ingest/`) - Walks `~/.claude/projects/` and parses JSONL conversation files
2. **Document** (`document.rs`) - Extracts chunks: messages, tool_use, tool_result
3. **Embed** (`embed.rs`) - Generates 512-dim embeddings via model2vec-rs (Potion-base-32M)
4. **Database** (`db/mod.rs`) - Stores in SQLite with FTS5 + sqlite-vec for hybrid search
5. **Commands** (`commands.rs`) - CLI handlers for index, search, status, projects, session, related

### Key Design Decisions

- **sqlite-vec FFI**: Uses `sqlite3_auto_extension` with a `Once` guard to register the extension before any connection opens. The unsafe transmute is required due to FFI signature differences.

- **Hybrid Search**: Combines FTS5 BM25 scores with vector cosine distance using Reciprocal Rank Fusion (RRF). Each search mode fetches 2x the limit, then fuses/truncates.

- **Path Encoding**: Claude Code encodes project paths in directory names: `/` becomes `-`, `/.` becomes `--`. The `config.rs` module handles decoding.

- **Chunk Types**: Three indexed types with shared `DisplayLabel` trait for consistent formatting across `Document` and `SearchResult`.

- **Session Similarity**: The `related` command averages embeddings from a session to create a "session vector", then searches for similar documents from other sessions.

### Module Responsibilities

| Module | Purpose |
|--------|---------|
| `main` | CLI argument parsing with clap |
| `commands` | CLI command handlers (index, search, status, projects, session, related) |
| `config` | Database paths, Claude directory discovery |
| `db` | SQLite with FTS5 + sqlite-vec, search methods |
| `document` | Document struct, ChunkKind enum, DisplayLabel trait |
| `embed` | Embedder wrapper around model2vec-rs |
| `error` | Custom error types with thiserror |
| `ingest` | JSONL parsing, project directory walking |
| `utils` | Shared utilities (truncate_text) |

### Adding New Commands

1. Add variant to `Commands` enum in `main.rs`
2. Add argument parsing with clap derive macros
3. Add match arm in `main()` to call command handler
4. Implement handler in `commands.rs`
5. Add any needed database methods in `db/mod.rs`

### Adding New Search Filters

1. Add field to `SearchOptions` struct in `commands.rs`
2. Add clap argument in `main.rs` Search variant
3. Wire up in `main()` match arm
4. Update `filter_result()` function in `commands.rs`
5. Optionally add SQL filter in `search_fts_filtered()` for efficiency

## Testing

Integration tests in `tests/integration.rs` cover the full pipeline. Unit tests are co-located with modules. Embedding tests are `#[ignore]` tagged since they require model download.

```bash
cargo test                    # Run all tests
cargo test test_fts_search    # Run specific test
cargo test -- --ignored       # Run ignored tests (requires model)
```
