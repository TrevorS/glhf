# glhf

A CLI tool for searching your Claude Code conversation history.

## Features

- **Hybrid search** combining BM25 full-text + semantic search
- **Tool call indexing** - search Bash commands, file reads, edits, and more
- **Regex search** with case-insensitive option
- **Context display** - show messages before/after matches (like grep)
- **Filtering** - by tool name, project, time range, errors only, messages only, or tools only
- **JSON output** - machine-readable format for scripting and agents
- **Session viewer** - view full conversation sessions with `glhf session`
- Fast SQLite-based indexing with FTS5 and sqlite-vec

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Build the search index (embeddings auto-download on first run)
glhf index

# Search conversations (hybrid mode by default)
glhf search "rust error handling"

# Text-only search (faster, no embeddings)
glhf search "rust error" --mode text

# Semantic search (meaning-based)
glhf search "how to handle failures" --mode semantic

# Search with regex
glhf search -e "cargo (build|test)" -i

# Show context around matches
glhf search "error" -C 2

# Filter by tool
glhf search "git" -t Bash

# Only show errors
glhf search "failed" --errors

# Only show messages (no tool calls)
glhf search "help me" --messages-only

# Only show tool calls
glhf search "main.rs" --tools-only

# Filter by project
glhf search "bug" -p myapp

# Filter to current project
glhf search "error" -p .

# Search recent history
glhf search "error" --since 1d
glhf search "refactor" --since 1w

# JSON output (for scripting/agents)
glhf search "error" --json

# Check index status
glhf status

# View a full conversation session
glhf session <SESSION_ID>
glhf session abc123    # partial ID matching
```

### Search Options

| Option | Description |
|--------|-------------|
| `-l, --limit <N>` | Maximum results (default: 10) |
| `-m, --mode <MODE>` | hybrid, text, or semantic (default: hybrid) |
| `-e, --regex` | Interpret query as regex |
| `-i, --ignore-case` | Case-insensitive search |
| `-A <N>` | Show N messages after each match |
| `-B <N>` | Show N messages before each match |
| `-C <N>` | Show N messages before and after |
| `-t, --tool <NAME>` | Filter by tool (Bash, Read, Edit, Grep, etc.) |
| `-p, --project <NAME>` | Filter by project (substring match, or `.` for current dir) |
| `--since <DURATION>` | Filter by time (1h, 2d, 1w, or 2024-12-01) |
| `--errors` | Only show error results |
| `--messages-only` | Only show messages (exclude tool calls) |
| `--tools-only` | Only show tool calls (exclude messages) |
| `--json` | Output results as JSON |

## Search Modes

| Mode | Description |
|------|-------------|
| `hybrid` | Combines FTS5 + vector search with RRF fusion (default) |
| `text` | BM25 full-text search only (fast, keyword matching) |
| `semantic` | Vector similarity search (meaning-based) |

## Query Syntax

Text mode (`--mode text`) uses SQLite FTS5 for full-text search. Query syntax:

| Syntax | Example | Description |
|--------|---------|-------------|
| Single word | `error` | Match documents containing "error" |
| Multiple words | `rust error` | Implicit AND - both words required |
| OR | `rust OR python` | Match either word |
| Phrase | `"git status"` | Exact phrase match |
| Prefix | `err*` | Match words starting with "err" |
| NOT | `rust NOT python` | Exclude documents with "python" |

**Hybrid mode** (default) combines FTS5 results with semantic search using Reciprocal Rank Fusion. This finds both exact keyword matches and semantically similar content.

**Regex mode** (`-e`) bypasses FTS5 entirely and does a full table scan with regex matching.

## Data Format

glhf indexes conversation data from Claude Code stored in `~/.claude/projects/`.

### Directory Structure

```
~/.claude/
├── projects/
│   ├── -Users-trevor-Projects-myapp/     # Encoded project path
│   │   ├── abc123.jsonl                  # Conversation session
│   │   └── def456.jsonl
│   └── -Users-trevor--dotfiles/          # Double dash = hidden dir (.dotfiles)
│       └── 789xyz.jsonl
```

Project directories are encoded versions of the original path:
- Single dash `-` represents `/`
- Double dash `--` represents `/.` (hidden directories)

Example: `-Users-trevor-Projects-myapp` → `/Users/trevor/Projects/myapp`

### What Gets Indexed

glhf indexes three types of chunks from conversation files:

| Chunk Type | Description | Example Content |
|------------|-------------|-----------------|
| `message` | User prompts and assistant responses | "How do I handle errors in Rust?" |
| `tool_use` | Tool invocations by the assistant | Command: `git status` |
| `tool_result` | Output from tool execution | "On branch main, nothing to commit" |

### Indexed Fields

| Field | Description |
|-------|-------------|
| `content` | Full-text searchable content |
| `chunk_kind` | "message", "tool_use", or "tool_result" |
| `role` | "user" or "assistant" (for messages) |
| `tool_name` | Tool name: Bash, Read, Edit, Grep, etc. |
| `tool_id` | Links tool_use to its tool_result |
| `tool_input` | Raw JSON input for tool calls |
| `is_error` | Whether tool result was an error |
| `project` | Decoded project path |
| `session_id` | Claude Code session identifier |
| `timestamp` | Message creation time |

## Development

```bash
make help     # Show available commands
make check    # Format, lint, and test
make bench    # Run benchmarks
```

### Dependencies

- **Embeddings**: model2vec-rs with Potion-base-32M (~130MB, auto-downloads)
- **Database**: SQLite with FTS5 + sqlite-vec
- **No external setup required** - just `cargo build`

## License

MIT
