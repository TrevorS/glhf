# glhf

A CLI tool for searching your Claude Code conversation history.

## Features

- **BM25 full-text search** over `~/.claude` conversation data
- **Tool call indexing** - search Bash commands, file reads, edits, and more
- **Regex search** with case-insensitive option
- **Context display** - show messages before/after matches (like grep)
- **Filtering** - by tool name, errors only, messages only, or tools only
- Fast indexing with Tantivy

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Build the search index
glhf index

# Search conversations
glhf search "rust error handling"

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

# Check index status
glhf status
```

### Search Options

| Option | Description |
|--------|-------------|
| `-l, --limit <N>` | Maximum results (default: 10) |
| `-e, --regex` | Interpret query as regex |
| `-i, --ignore-case` | Case-insensitive search |
| `-A <N>` | Show N messages after each match |
| `-B <N>` | Show N messages before each match |
| `-C <N>` | Show N messages before and after |
| `-t, --tool <NAME>` | Filter by tool (Bash, Read, Edit, Grep, etc.) |
| `--errors` | Only show error results |
| `--messages-only` | Only show messages (exclude tool calls) |
| `--tools-only` | Only show tool calls (exclude messages) |

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

### Test Coverage

- 20 unit tests
- 10 integration tests
- 3 doc tests
- Criterion benchmarks for indexing and search

## License

MIT
