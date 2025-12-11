# glhf

A CLI tool for searching your Claude Code conversation history. Works for both humans and AI agents.

## Features

- **Hybrid search** combining BM25 full-text + semantic search
- **Tool call indexing** - search Bash commands, file reads, edits, and more
- **Regex search** with case-insensitive option
- **Context display** - show messages before/after matches (like grep)
- **Filtering** - by tool name, project, time range, errors only, messages only, or tools only
- **Project listing** - see all indexed projects at a glance
- **Session viewer** - view full conversations or quick summaries
- **Related sessions** - find similar past work using embeddings
- **JSON output** - machine-readable format for scripting and agents
- Fast SQLite-based indexing with FTS5 and sqlite-vec

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Build the search index (embeddings auto-download on first run)
glhf index

# Search your history
glhf search "rust error handling"

# See what projects you've worked on
glhf projects

# View a session summary
glhf session abc123 --summary

# Find related past work
glhf related abc123
```

## Commands

### `glhf search` - Search conversations

```bash
# Basic search (hybrid mode: text + semantic)
glhf search "rust error handling"

# Compact output for quick scanning
glhf search "cargo" --compact

# Show session IDs to jump to full context
glhf search "bug fix" --show-session-id

# Text-only search (faster, keyword matching)
glhf search "rust error" --mode text

# Semantic search (meaning-based, finds related concepts)
glhf search "how to handle failures" --mode semantic

# Regex search
glhf search -e "cargo (build|test)" -i

# Show context around matches (like grep -C)
glhf search "error" -C 2

# Filter by tool type
glhf search "git" -t Bash
glhf search "main.rs" -t Read
glhf search "function" -t Edit

# Filter by project
glhf search "bug" -p myapp
glhf search "error" -p .    # current directory

# Filter by time
glhf search "error" --since 1d
glhf search "refactor" --since 1w
glhf search "deploy" --since 2024-12-01

# Filter by type
glhf search "failed" --errors        # only errors
glhf search "help me" --messages-only # only human/AI messages
glhf search "main.rs" --tools-only    # only tool calls

# JSON output (for scripting/agents)
glhf search "error" --json
```

#### Search Options

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
| `-p, --project <NAME>` | Filter by project (substring match, or `.` for cwd) |
| `--since <DURATION>` | Filter by time (1h, 2d, 1w, or 2024-12-01) |
| `--errors` | Only show error results |
| `--messages-only` | Only show messages (exclude tool calls) |
| `--tools-only` | Only show tool calls (exclude messages) |
| `--compact` | Single-line output for quick scanning |
| `--show-session-id` | Show session IDs for jumping to full context |
| `--json` | Output results as JSON |

### `glhf projects` - List indexed projects

```bash
glhf projects
```

Output:
```
Projects (7 total)
──────────────────────────────────────────────────
.claude                 582 docs    last: 19m ago
glhf                   4200 docs    last: 58m ago
gym                   13664 docs    last: 4h ago
myapp                  4220 docs    last: 1w ago
```

### `glhf session` - View conversation sessions

```bash
# View full session (partial ID matching supported)
glhf session abc123

# Quick summary without full content
glhf session abc123 --summary

# Show only first N messages
glhf session abc123 --limit 20

# JSON output
glhf session abc123 --json
```

Summary output:
```
Session: ab808632-48b3-46ee-a2fb-74e5c7ee722a
Project: glhf
Duration: 13h 35m (started 14h ago)
Messages: 3455 total
  - assistant: 672
  - user: 77
  - tool calls: 1375
  - tool results: 1331
Tools used: Bash (441), Read (333), Edit (228), TodoWrite (82), WebSearch (59)
```

### `glhf related` - Find similar sessions

Find past sessions that are semantically similar to a given session. Useful for "have I solved this before?" queries.

```bash
# Find sessions related to a specific session
glhf related abc123

# Limit number of results
glhf related abc123 --limit 10
```

Output:
```
Finding sessions related to: ab808632-... (glhf)

Related sessions:

[1] 89a91234 | glhf | 5d ago | Score: 0.35
    "Excellent! Now I have a complete picture of the glhf..."

[2] e0bc965b | sandbox | 3w ago | Score: 0.34
    "The file..."
```

### `glhf status` - Check index status

```bash
glhf status
```

Output:
```
Database Status
---------------
Documents:  33371
Embeddings: 33371
Size:       162.35 MB
Location:   /Users/you/Library/Caches/glhf/glhf.db
```

### `glhf index` - Build/rebuild search index

```bash
# Build index (incremental - currently rebuilds fully)
glhf index

# Force full rebuild
glhf index --rebuild

# Skip embeddings (text search only, faster)
glhf index --skip-embeddings
```

## Common Workflows

### "How did I solve this before?"

```bash
# Search for similar past work
glhf search "authentication" --mode semantic

# Find the session and get context
glhf search "JWT token" --show-session-id
glhf session abc123 --summary
glhf session abc123 --limit 50
```

### "What commands did I run?"

```bash
# Find Bash commands
glhf search "deploy" -t Bash --compact

# Find recent errors
glhf search "error" --errors --since 1d
```

### "What have I been working on?"

```bash
# See all projects
glhf projects

# Recent activity in current project
glhf search "" -p . --since 1w --compact -l 20
```

### "Find related past sessions"

```bash
# Get session ID from search
glhf search "database migration" --show-session-id

# Find related sessions
glhf related abc123
```

## Search Modes

| Mode | Description | Best For |
|------|-------------|----------|
| `hybrid` | Combines FTS5 + vector search with RRF fusion (default) | General use |
| `text` | BM25 full-text search only | Exact keyword matching, speed |
| `semantic` | Vector similarity search | Finding conceptually related content |

### Query Syntax (Text Mode)

| Syntax | Example | Description |
|--------|---------|-------------|
| Single word | `error` | Match documents containing "error" |
| Multiple words | `rust error` | Implicit AND - both words required |
| OR | `rust OR python` | Match either word |
| Phrase | `"git status"` | Exact phrase match |
| Prefix | `err*` | Match words starting with "err" |
| NOT | `rust NOT python` | Exclude documents with "python" |

## For Claude Code / AI Agents

glhf is designed to work well with AI agents. Key features:

### JSON Output

All commands support `--json` for machine-readable output:

```bash
glhf search "error" --json
glhf session abc123 --json
```

### Compact Output

Use `--compact` for dense, scannable results that use fewer tokens:

```bash
glhf search "cargo" --compact -l 20
```

### Session Discovery

Use `--show-session-id` to get session IDs, then explore with `glhf session`:

```bash
# Find relevant results with session IDs
glhf search "bug fix" --show-session-id

# Get summary of interesting session
glhf session abc123 --summary

# Get full context if needed
glhf session abc123 --limit 50
```

### Find Past Solutions

Use `glhf related` to find sessions where you solved similar problems:

```bash
glhf related <current-session-id> --limit 5
```

## Data Format

glhf indexes conversation data from Claude Code stored in `~/.claude/projects/`.

### Directory Structure

```
~/.claude/
├── projects/
│   ├── -Users-you-Projects-myapp/     # Encoded project path
│   │   ├── abc123.jsonl               # Conversation session
│   │   └── def456.jsonl
│   └── -Users-you--dotfiles/          # Double dash = hidden dir (.dotfiles)
│       └── 789xyz.jsonl
```

Project directories are encoded versions of the original path:
- Single dash `-` represents `/`
- Double dash `--` represents `/.` (hidden directories)

### Indexed Content

| Chunk Type | Description | Example Content |
|------------|-------------|-----------------|
| `message` | User prompts and assistant responses | "How do I handle errors in Rust?" |
| `tool_use` | Tool invocations by the assistant | Command: `git status` |
| `tool_result` | Output from tool execution | "On branch main, nothing to commit" |

## Development

```bash
make help     # Show available commands
make check    # Format, lint, and test
make bench    # Run benchmarks
```

### Dependencies

- **Embeddings**: model2vec-rs with Potion-multilingual-128M (256 dimensions, auto-downloads)
- **Database**: SQLite with FTS5 + sqlite-vec
- **No external setup required** - just `cargo build`

## License

MIT
