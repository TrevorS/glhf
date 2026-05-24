# glhf

A CLI tool for searching your Claude Code conversation history.

## Features

- **Hybrid search** combining BM25 full-text + semantic vector search
- **Tool call indexing** - search Bash commands, file reads, edits, and more
- **Filtering** - by tool name, project, time range, errors
- **Session viewer** - view full conversations or quick summaries
- **JSON output** - machine-readable format for agents
- **Incremental indexing** - only re-processes changed files
- Fast SQLite-based storage with FTS5 and sqlite-vec

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Build the search index (model auto-downloads on first run)
glhf index

# Search your history
glhf search "rust error handling"

# Compact output for scanning
glhf search "cargo test" --compact

# View a session
glhf session abc123 --summary
```

## Commands

### `glhf search` - Search conversations

```bash
# Basic search (hybrid: text + semantic)
glhf search "rust error handling"

# Compact single-line output
glhf search "cargo" --compact

# Filter by tool type
glhf search "git" -t Bash
glhf search "main.rs" -t Read

# Filter by project
glhf search "bug" -p myapp
glhf search "error" -p .          # current directory

# Filter by time
glhf search "error" --since 1d
glhf search "refactor" --since 1w

# Only errors
glhf search "failed" --errors

# JSON output
glhf search "error" --json
```

#### Search Flags

| Flag | Description |
|------|-------------|
| `-l, --limit <N>` | Maximum results (default: 10) |
| `-t, --tool <NAME>` | Filter by tool (Bash, Read, Edit, Grep) |
| `-p, --project <NAME>` | Filter by project (substring match, `.` for cwd) |
| `--since <DURATION>` | Time filter (1h, 2d, 1w, or 2024-12-01) |
| `--errors` | Only show error results |
| `--compact` | One line per result |
| `--json` | JSON output |

### `glhf session` - View a conversation

```bash
glhf session abc123              # Full session
glhf session abc123 --summary    # Quick overview
glhf session abc123 --limit 30   # First 30 messages
glhf session abc123 --json       # JSON output
```

### `glhf recent` - Recent sessions

```bash
glhf recent                      # Last 10 sessions
glhf recent -l 20                # More sessions
glhf recent -p myproject         # Filter by project
```

### `glhf status` - Index stats

Shows database size, document counts, top projects, content breakdown, and timeline.

### `glhf index` - Build/update index

```bash
glhf index                       # Incremental update (fast)
glhf index --full                # Full rebuild
glhf index --skip-embeddings     # Text search only
```

## How It Works

glhf combines FTS5 keyword search with static vector embeddings ([potion-base-32M](https://huggingface.co/minishlab/potion-base-32M)) using convex combination fusion (α=0.75). The two search systems find largely disjoint results — FTS catches exact keywords while vectors catch paraphrases — so combining them improves recall without hurting precision.

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed design decisions, model evaluation results, and what we tried and rejected.

### References

- Bruch, Gai, Ingber. ["An Analysis of Fusion Functions for Hybrid Retrieval."](https://arxiv.org/abs/2210.11934) ACM TOIS 42(1), 2023.
- Tulkens & van Dongen. [Model2Vec.](https://github.com/MinishLab/model2vec) 2024.

## Development

```bash
make check    # Format, lint, and test
make build    # Build debug binary
make release  # Build release binary
make install  # Install to ~/.cargo/bin
```

## License

MIT
