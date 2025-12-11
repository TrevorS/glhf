# glhf: Claude Code History Search CLI

## Goal
Build a Rust CLI that provides hybrid search (FTS5 + semantic) over all Claude Code history data in `~/.claude`.

## Current Status: v0.4.0

### Implemented
- [x] CLI with `index`, `search`, `status` commands
- [x] **SQLite database** with FTS5 full-text search
- [x] **Semantic search** via sqlite-vec + model2vec-rs
- [x] **Hybrid search** with RRF (Reciprocal Rank Fusion)
- [x] Conversation JSONL parsing (user/assistant messages)
- [x] Tool call indexing (ToolUse, ToolResult chunks)
- [x] Smart content extraction per tool type
- [x] Project path decoding (`-` → `/`, `--` → `/.`)
- [x] Regex search with case-insensitive option
- [x] Context display - grep-like `-A`, `-B`, `-C` options
- [x] Filtering - by tool name, errors, messages-only, tools-only
- [x] Search mode flag (`--mode hybrid|text|semantic`)
- [x] Custom error types with thiserror
- [x] Integration tests
- [x] Unit tests
- [x] Benchmarks (Criterion)
- [x] CI/CD (GitHub Actions, Linux)

### Not Yet Implemented
- [ ] Incremental index updates
- [ ] Additional data sources (todos, plans, history, debug)
- [ ] Date/project filters
- [ ] JSON output format
- [ ] Progress bars

## Design Decisions
- **Storage**: Single SQLite database (`~/.cache/glhf/glhf.db`)
- **Full-text search**: FTS5 (built into SQLite)
- **Vector search**: sqlite-vec (brute-force, SIMD-optimized)
- **Embeddings**: model2vec-rs with Potion-multilingual-128M (256 dimensions)
- **Hybrid fusion**: Reciprocal Rank Fusion (RRF) with k=60
- **Indexing**: Full rebuild for now, incremental updates planned

## Data Sources

| Source | Format | Content | Status |
|--------|--------|---------|--------|
| `projects/*.jsonl` | JSONL | Full conversations with tool calls | Implemented |
| `history.jsonl` | JSONL | Command log with timestamps | Planned |
| `todos/*.json` | JSON | Task arrays with status | Planned |
| `plans/*.md` | Markdown | Implementation plans | Planned |
| `debug/*.txt` | Text | Debug logs | Planned |

## CLI Interface

### Current (v0.4.0)
```
glhf index [--rebuild] [--skip-embeddings]
  --rebuild              Force full rebuild
  --skip-embeddings      Skip embedding generation (text search only)

glhf search <QUERY>
  -l, --limit <N>          # Results count (default: 10)
  -m, --mode <MODE>        # hybrid | text | semantic (default: hybrid)
  -e, --regex              # Interpret query as regex
  -i, --ignore-case        # Case-insensitive search
  -A, --after-context <N>  # Show N messages after match
  -B, --before-context <N> # Show N messages before match
  -C, --context <N>        # Show N messages before and after
  -t, --tool <NAME>        # Filter by tool name
  --errors                 # Only show error results
  --messages-only          # Only show messages (exclude tools)
  --tools-only             # Only show tool calls (exclude messages)

glhf status                # Show database stats
```

### Planned
```
glhf search <QUERY>
  --type <TYPE>            # conversation | todo | plan | history | debug | all
  -p, --project <PATH>     # Filter by project path
  --after/--before <DATE>  # Date filters
  --json                   # JSON output
```

## Architecture

```
┌─────────────────────────────────────────┐
│              CLI (clap)                 │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────┴───────────────────────┐
│            commands.rs                   │
│  ┌─────────────────────────────────────┐│
│  │         Database (SQLite)           ││
│  │  ┌───────────┐  ┌────────────────┐  ││
│  │  │   FTS5    │  │  sqlite-vec    │  ││
│  │  │  (text)   │  │   (vectors)    │  ││
│  │  └─────┬─────┘  └───────┬────────┘  ││
│  │        └────────┬───────┘           ││
│  │           RRF Fusion                ││
│  └─────────────────────────────────────┘│
│  ┌─────────────────────────────────────┐│
│  │    Embedder (model2vec-rs)          ││
│  │    Potion-multilingual-128M (256d)  ││
│  └─────────────────────────────────────┘│
└─────────────────────────────────────────┘
```

## Module Structure

```
src/
├── main.rs              # CLI entry (clap)
├── lib.rs               # Crate docs, re-exports
├── commands.rs          # Command handlers
├── config.rs            # Path configuration
├── error.rs             # Custom error types
├── document.rs          # ChunkKind enum, Document struct
├── db/
│   └── mod.rs           # SQLite + FTS5 + sqlite-vec
├── embed.rs             # model2vec-rs wrapper
└── ingest/
    ├── mod.rs           # File discovery
    └── conversation.rs  # JSONL parsing + tool extraction
```

### Planned Additions
```
src/
├── ingest/
│   ├── todo.rs          # todos/*.json
│   ├── plan.rs          # plans/*.md
│   ├── history.rs       # history.jsonl
│   └── debug.rs         # debug/*.txt
└── cli/
    └── output.rs        # Human/JSON formatting
```

## Key Implementation Details

### Database Schema
```sql
-- Main documents table
CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    chunk_kind TEXT NOT NULL,
    content TEXT NOT NULL,
    project TEXT,
    session_id TEXT,
    role TEXT,
    tool_name TEXT,
    tool_id TEXT,
    tool_input TEXT,
    is_error INTEGER,
    timestamp TEXT,
    source_path TEXT NOT NULL
);

-- FTS5 virtual table
CREATE VIRTUAL TABLE documents_fts USING fts5(
    content,
    content='documents',
    content_rowid='rowid'
);

-- Vector table (256 dimensions for Potion-multilingual-128M)
CREATE VIRTUAL TABLE documents_vec USING vec0(
    id TEXT PRIMARY KEY,
    embedding FLOAT[256]
);
```

### Hybrid Search (RRF)
```rust
fn rrf_fusion(fts: &[Result], vec: &[Result]) -> Vec<Result> {
    let k = 60.0;
    for (rank, result) in fts.iter().enumerate() {
        scores[id] += 1.0 / (k + rank + 1);
    }
    for (rank, result) in vec.iter().enumerate() {
        scores[id] += 1.0 / (k + rank + 1);
    }
    // Sort by combined score
}
```

### Smart Tool Content Extraction
```rust
match tool_name {
    "Bash" => input["command"],
    "Read" | "Write" => input["file_path"],
    "Edit" => file_path + old + new,
    "Grep" | "Glob" => pattern in path,
    "Task" => prompt (truncated),
    "WebFetch" => url,
    "WebSearch" => query,
    _ => generic extraction
}
```

## Dependencies

```toml
[dependencies]
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
dirs = "6"
model2vec-rs = "0.1"
hex = "0.4"
regex = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
sqlite-vec = "0.1"
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
walkdir = "2"
zerocopy = { version = "0.8", features = ["derive"] }

[dev-dependencies]
criterion = { version = "0.8", features = ["html_reports"] }
tempfile = "3"
```

## Environment Setup

Semantic search uses model2vec-rs with Potion-multilingual-128M. The model downloads automatically on first use.

No manual setup required - just run:
```bash
cargo build --release
./target/release/glhf index
```

## Implementation Order

1. ~~**Scaffold** - Cargo.toml, CLI skeleton, config paths~~ ✅
2. ~~**Ingest** - Parse conversation JSONL into Documents~~ ✅
3. ~~**BM25** - Tantivy index, basic search~~ ✅ (replaced with FTS5)
4. ~~**Tool Calls** - Index ToolUse/ToolResult chunks~~ ✅
5. ~~**Search Options** - Regex, context, filtering~~ ✅
6. ~~**SQLite Migration** - Replace Tantivy with SQLite + FTS5~~ ✅
7. ~~**Embeddings** - fastembed integration~~ ✅ (replaced with model2vec-rs)
8. ~~**Vector Index** - sqlite-vec~~ ✅
9. ~~**Hybrid** - RRF fusion, mode switching~~ ✅
10. **More Sources** - todos, plans, history, debug
11. **Incremental** - File state tracking, update detection
12. **Polish** - Progress bars, JSON output, date filters

## Notes

- Using Rust edition 2021
- SQLite database is ~2-5MB for ~14K documents + embeddings
- Embedding generation: very fast with model2vec-rs (~2K docs/sec)
- FTS5 search: <1ms for typical queries
- Vector search: ~10-50ms for 10K+ documents (brute-force)
- Hybrid search combines both for best relevance
- Model download (~130MB) happens on first run
