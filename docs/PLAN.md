# glhf: Claude Code History Search CLI

## Goal
Build a Rust CLI that provides BM25 + semantic search over all Claude Code history data in `~/.claude`.

## Current Status: v0.2.0

### Implemented
- [x] CLI with `index`, `search`, `status` commands
- [x] BM25 full-text search via Tantivy
- [x] Conversation JSONL parsing (user/assistant messages)
- [x] **Tool call indexing** (ToolUse, ToolResult chunks)
- [x] Smart content extraction per tool type (Bash→command, Read→path, etc.)
- [x] Array content block extraction
- [x] Project path decoding (`-` → `/`, `--` → `/.`)
- [x] **Regex search** with case-insensitive option (`-e`, `-i`)
- [x] **Context display** - grep-like `-A`, `-B`, `-C` options
- [x] **Filtering** - by tool name, errors, messages-only, tools-only
- [x] Custom error types with thiserror
- [x] Integration tests (10 tests)
- [x] Unit tests (20 tests)
- [x] Doc tests (3 tests)
- [x] Benchmarks (Criterion) - indexing and search
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
- **Scope**: Conversations + tool calls (v0.2.0), expand to all sources later
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

### Current (v0.2.0)
```
glhf index [--rebuild]     # Build index (full rebuild)

glhf search <QUERY>        # Full-text search
  -l, --limit <N>          # Results count (default: 10)
  -e, --regex              # Interpret query as regex
  -i, --ignore-case        # Case-insensitive search
  -A, --after-context <N>  # Show N messages after match
  -B, --before-context <N> # Show N messages before match
  -C, --context <N>        # Show N messages before and after
  -t, --tool <NAME>        # Filter by tool name (Bash, Read, Edit, etc.)
  --errors                 # Only show error results
  --messages-only          # Only show messages (exclude tools)
  --tools-only             # Only show tool calls (exclude messages)

glhf status                # Show index stats
```

### Planned
```
glhf search <QUERY>
  -m, --mode <MODE>        # hybrid | bm25 | semantic
  --type <TYPE>            # conversation | todo | plan | history | debug | all
  -p, --project <PATH>     # Filter by project path
  --after/--before <DATE>  # Date filters
  --json                   # JSON output
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
├── error.rs             # Custom error types ✅
├── index/
│   ├── mod.rs
│   └── bm25.rs          # Tantivy wrapper ✅
├── ingest/
│   ├── mod.rs           # File discovery ✅
│   └── conversation.rs  # projects/*.jsonl + tool extraction ✅
└── models/
    └── document.rs      # ChunkKind enum, Document struct ✅
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
pub enum ChunkKind {
    Message,     // User/assistant text
    ToolUse,     // Tool invocation
    ToolResult,  // Tool output
}

pub struct Document {
    pub id: String,               // SHA256 hash of path + content prefix
    pub chunk_kind: ChunkKind,    // Message, ToolUse, or ToolResult
    pub project: Option<String>,  // Decoded project path
    pub timestamp: Option<DateTime<Utc>>,
    pub session_id: Option<String>,
    pub role: Option<String>,     // "user" or "assistant" (for Message)
    pub tool_name: Option<String>,// e.g., "Bash", "Read", "Edit" (for tools)
    pub tool_id: Option<String>,  // Links ToolUse to ToolResult
    pub tool_input: Option<String>, // Tool input as JSON
    pub is_error: Option<bool>,   // Whether tool result was an error
    pub content: String,          // Searchable text
    pub source_path: PathBuf,
}
```

### Smart Tool Content Extraction (Implemented)
```rust
// Tool-specific content extraction for better searchability
match tool_name {
    "Bash" => input["command"],           // git status
    "Read" | "Write" => input["file_path"], // /path/to/file.rs
    "Edit" => file_path + old + new,      // path: old → new
    "Grep" | "Glob" => pattern in path,   // "error" in src/
    "Task" => prompt (truncated),         // Launch agent...
    "WebFetch" => url,                    // https://...
    "WebSearch" => query,                 // search terms
    _ => generic extraction
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
regex = "1"
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

## Benchmarks (v0.2.0)

### Indexing
| Operation | 100 docs | 1000 docs | 5000 docs |
|-----------|----------|-----------|-----------|
| Messages | ~215ms | ~224ms | ~250ms |
| Tools | ~210ms | ~220ms | ~245ms |
| Mixed | ~212ms | ~222ms | ~248ms |

### Search
| Operation | Time |
|-----------|------|
| Single term | ~30µs |
| Multi-term | ~35µs |
| Filtered (by tool) | ~40µs |
| Regex | ~50µs |

## Implementation Order

1. ~~**Scaffold** - Cargo.toml, CLI skeleton, config paths~~ ✅
2. ~~**Ingest** - Parse conversation JSONL into Documents~~ ✅
3. ~~**BM25** - Tantivy index, basic search~~ ✅
4. ~~**Tool Calls** - Index ToolUse/ToolResult chunks~~ ✅
5. ~~**Search Options** - Regex, context, filtering~~ ✅
6. **Embeddings** - fastembed integration, batch embed
7. **Vector Index** - HNSW with persistence
8. **Hybrid** - RRF fusion, mode switching
9. **More Sources** - todos, plans, history, debug
10. **Incremental** - File state tracking, update detection
11. **Polish** - Progress bars, JSON output, date filters

## Notes

- Using Rust edition 2021
- Index is ~2MB for ~14K documents
- Indexing ~14K docs takes ~1 second
- Project paths are encoded (`-Users-trevor-Projects-`) - decoding implemented
- Tool calls add ~30% more indexed content vs messages only
- Debug logs are noisy - consider filtering common patterns when implemented
