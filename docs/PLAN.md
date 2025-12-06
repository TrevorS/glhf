# glhf: Claude Code History Search CLI

## Goal
Build a Rust CLI that provides BM25 + semantic search over all Claude Code history data in `~/.claude`.

## Design Decisions
- **Embeddings**: Local-only via `fastembed` (all-MiniLM-L6-v2, ~22MB model)
- **Indexing**: Hybrid - build ahead-of-time, auto-detect new files at search time
- **Scope**: Everything (conversations, todos, plans, history, debug logs)
- **Storage**: `~/.cache/glhf/` for indexes

## Data Sources

| Source | Format | Content |
|--------|--------|---------|
| `projects/*.jsonl` | JSONL | Full conversations with tool calls |
| `history.jsonl` | JSONL | Command log with timestamps |
| `todos/*.json` | JSON | Task arrays with status |
| `plans/*.md` | Markdown | Implementation plans |
| `debug/*.txt` | Text | Debug logs |

## CLI Interface

```
glhf search <QUERY>        # Hybrid search (default)
  -m, --mode <MODE>        # hybrid | bm25 | semantic
  -n, --limit <N>          # Results count (default: 10)
  -t, --type <TYPE>        # conversation | todo | plan | history | debug | all
  -p, --project <PATH>     # Filter by project path
  --after/--before <DATE>  # Date filters
  --json                   # JSON output

glhf index                 # Build/update indexes
  --rebuild                # Force full rebuild

glhf status                # Show index stats
```

## Architecture

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
├── lib.rs
├── cli/
│   ├── commands.rs      # Command handlers
│   └── output.rs        # Human/JSON formatting
├── index/
│   ├── bm25.rs          # Tantivy wrapper
│   ├── vector.rs        # HNSW + persistence
│   ├── embedder.rs      # fastembed wrapper
│   └── metadata.rs      # File state tracking
├── search/
│   ├── engine.rs        # Search orchestration
│   └── hybrid.rs        # RRF score fusion
├── ingest/
│   ├── conversation.rs  # projects/*.jsonl
│   ├── todo.rs          # todos/*.json
│   ├── plan.rs          # plans/*.md (split by ## headers)
│   ├── history.rs       # history.jsonl
│   └── debug.rs         # debug/*.txt (50-line windows)
└── models/
    └── document.rs      # Unified doc struct
```

## Key Implementation Details

### Document Model
```rust
pub struct Document {
    pub id: String,           // Deterministic hash
    pub doc_type: DocType,    // conversation | todo | plan | history | debug
    pub project: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub session_id: Option<String>,
    pub content: String,      // Searchable text
    pub source_path: PathBuf, // For incremental updates
    pub source_offset: u64,   // For JSONL append detection
}
```

### Hybrid Search (RRF)
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

### Incremental Updates
- Track `(modified_time, size, last_offset)` per source file
- JSONL files: only parse bytes after `last_offset`
- Other files: reindex completely if changed

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
tantivy = "0.22"
hnsw_rs = "0.3"
fastembed = "4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
bincode = "1"
rayon = "1"
chrono = { version = "0.4", features = ["serde"] }
walkdir = "2"
dirs = "5"
anyhow = "1"
thiserror = "2"
indicatif = "0.17"
pulldown-cmark = "0.10"
```

## Implementation Order

1. **Scaffold** - Cargo.toml, CLI skeleton, config paths
2. **Ingest** - Parse all data sources into Documents
3. **BM25** - Tantivy index, basic search
4. **Embeddings** - fastembed integration, batch embed
5. **Vector Index** - HNSW with persistence
6. **Hybrid** - RRF fusion, mode switching
7. **Incremental** - File state tracking, update detection
8. **Polish** - Progress bars, JSON output, error handling

## Notes

- Rust edition 2024 may have crate compat issues - fall back to 2021 if needed
- Peak memory ~2GB during initial embedding of ~100K docs
- Debug logs are noisy - consider filtering common patterns
- Project paths are encoded (`-Users-trevor-Projects-`) - need decoding
