# glhf: Claude Code History Search CLI

## Goal
Build a Rust CLI that provides BM25 + semantic search over all Claude Code history data in `~/.claude`.

## Current Status: v0.1.0 (MVP)

### Implemented
- [x] CLI with `index`, `search`, `status` commands
- [x] BM25 full-text search via Tantivy
- [x] Conversation JSONL parsing (user/assistant messages)
- [x] Array content block extraction (tool results)
- [x] Project path decoding (`-` → `/`, `--` → `/.`)
- [x] Integration tests (9 tests)
- [x] Benchmarks (Criterion)
- [x] CI/CD (GitHub Actions, Linux)
- [x] Rustdocs

### Not Yet Implemented
- [ ] Semantic search (embeddings)
- [ ] Hybrid search (RRF fusion)
- [ ] Incremental index updates
- [ ] Additional data sources (todos, plans, history, debug)
- [ ] Date/project filters
- [ ] JSON output format
- [ ] Progress bars

## Design Decisions
- **Embeddings**: Local-only via `fastembed` (all-MiniLM-L6-v2, ~22MB model) - *planned*
- **Indexing**: Full rebuild for now, incremental updates planned
- **Scope**: Conversations only (MVP), expand to all sources later
- **Storage**: `~/.cache/glhf/` for indexes

## Data Sources

| Source | Format | Content | Status |
|--------|--------|---------|--------|
| `projects/*.jsonl` | JSONL | Full conversations with tool calls | ✅ Implemented |
| `history.jsonl` | JSONL | Command log with timestamps | Planned |
| `todos/*.json` | JSON | Task arrays with status | Planned |
| `plans/*.md` | Markdown | Implementation plans | Planned |
| `debug/*.txt` | Text | Debug logs | Planned |

## CLI Interface

### Current (v0.1.0)
```
glhf index                 # Build index (full rebuild)
glhf search <QUERY>        # BM25 search
  -n, --limit <N>          # Results count (default: 10)
glhf status                # Show index stats
```

### Planned
```
glhf search <QUERY>
  -m, --mode <MODE>        # hybrid | bm25 | semantic
  -t, --type <TYPE>        # conversation | todo | plan | history | debug | all
  -p, --project <PATH>     # Filter by project path
  --after/--before <DATE>  # Date filters
  --json                   # JSON output

glhf index
  --rebuild                # Force full rebuild (vs incremental)
```

## Architecture

### Current
```
┌─────────────────────────────────────────┐
│              CLI (clap)                 │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────┴───────────────────────┐
│            commands.rs                   │
│  ┌─────────────┐                        │
│  │ BM25 Index  │                        │
│  │  (tantivy)  │                        │
│  └─────────────┘                        │
└─────────────────────────────────────────┘
```

### Planned
```
┌─────────────────────────────────────────┐
│              CLI (clap)                 │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────┴───────────────────────┐
│            SearchEngine                  │
│  ┌─────────────┐    ┌─────────────────┐ │
│  │ BM25 Index  │    │  Vector Index   │ │
│  │  (tantivy)  │    │ (hnsw_rs)       │ │
│  └──────┬──────┘    └────────┬────────┘ │
│         └──────────┬─────────┘          │
│              RRF Fusion                  │
└─────────────────────────────────────────┘
```

## Module Structure

```
src/
├── main.rs              # CLI entry (clap)
├── lib.rs               # Crate docs, re-exports
├── commands.rs          # Command handlers
├── config.rs            # Path configuration
├── index/
│   ├── mod.rs
│   └── bm25.rs          # Tantivy wrapper ✅
├── ingest/
│   ├── mod.rs           # File discovery ✅
│   └── conversation.rs  # projects/*.jsonl ✅
└── models/
    └── document.rs      # Document struct ✅
```

### Planned Additions
```
src/
├── index/
│   ├── vector.rs        # HNSW + persistence
│   ├── embedder.rs      # fastembed wrapper
│   └── metadata.rs      # File state tracking
├── search/
│   ├── engine.rs        # Search orchestration
│   └── hybrid.rs        # RRF score fusion
├── ingest/
│   ├── todo.rs          # todos/*.json
│   ├── plan.rs          # plans/*.md
│   ├── history.rs       # history.jsonl
│   └── debug.rs         # debug/*.txt
└── cli/
    └── output.rs        # Human/JSON formatting
```

## Key Implementation Details

### Document Model (Implemented)
```rust
pub struct Document {
    pub id: String,               // SHA256 hash of path + content prefix
    pub chunk_kind: ChunkKind,    // Message, ToolUse, or ToolResult
    pub project: Option<String>,  // Decoded project path
    pub timestamp: Option<DateTime<Utc>>,
    pub session_id: Option<String>,
    pub role: Option<String>,     // "user" or "assistant" (for Message)
    pub tool_name: Option<String>,// e.g., "Bash", "Read", "Edit" (for tools)
    pub tool_id: Option<String>,  // Links ToolUse to ToolResult
    pub is_error: Option<bool>,   // Whether tool result was an error
    pub content: String,          // Searchable text
    pub source_path: PathBuf,
}
```

### Hybrid Search (Planned - RRF)
```rust
// Reciprocal Rank Fusion - no score normalization needed
fn hybrid_search(bm25: Vec<(DocId, f32)>, semantic: Vec<(DocId, f32)>) -> Vec<DocId> {
    let k = 60.0;
    for (rank, (id, _)) in bm25.iter().enumerate() {
        scores[id] += 1.0 / (k + rank + 1);
    }
    for (rank, (id, _)) in semantic.iter().enumerate() {
        scores[id] += 1.0 / (k + rank + 1);
    }
    // Sort by combined score
}
```

### Incremental Updates (Planned)
- Track `(modified_time, size, last_offset)` per source file
- JSONL files: only parse bytes after `last_offset`
- Other files: reindex completely if changed

## Dependencies

### Current
```toml
[dependencies]
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
dirs = "6"
hex = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
tantivy = "0.25"
thiserror = "2"
walkdir = "2"

[dev-dependencies]
criterion = { version = "0.8", features = ["html_reports"] }
tempfile = "3"
```

### Planned Additions
```toml
hnsw_rs = "0.3"        # Vector index
fastembed = "4"        # Local embeddings
bincode = "1"          # Vector serialization
rayon = "1"            # Parallel embedding
indicatif = "0.17"     # Progress bars
pulldown-cmark = "0.10" # Markdown parsing
```

## Benchmarks (v0.1.0)

| Operation | Time |
|-----------|------|
| Index 100 docs | ~215ms |
| Index 1000 docs | ~224ms |
| Index 5000 docs | ~217ms |
| Search (single term) | ~30µs |
| Search (two terms) | ~33µs |

## Implementation Order

1. ~~**Scaffold** - Cargo.toml, CLI skeleton, config paths~~ ✅
2. ~~**Ingest** - Parse conversation JSONL into Documents~~ ✅
3. ~~**BM25** - Tantivy index, basic search~~ ✅
4. **Embeddings** - fastembed integration, batch embed
5. **Vector Index** - HNSW with persistence
6. **Hybrid** - RRF fusion, mode switching
7. **More Sources** - todos, plans, history, debug
8. **Incremental** - File state tracking, update detection
9. **Polish** - Progress bars, JSON output, filters

## Notes

- Using Rust edition 2021
- Index is ~2MB for ~14K documents
- Indexing ~14K docs takes ~1 second
- Project paths are encoded (`-Users-trevor-Projects-`) - decoding implemented
- Debug logs are noisy - consider filtering common patterns when implemented
