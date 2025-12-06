# glhf

A CLI tool for searching your Claude Code conversation history.

## Features

- BM25 full-text search over `~/.claude` conversation data
- Fast indexing with Tantivy
- Search by content with relevance ranking

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

# Check index status
glhf status
```

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

### Conversation JSONL Format

Each `.jsonl` file contains one JSON object per line. glhf indexes `user` and `assistant` message types:

```json
{"type":"user","timestamp":"2025-01-15T10:00:00Z","sessionId":"abc123","message":{"role":"user","content":"How do I handle errors in Rust?"}}
{"type":"assistant","timestamp":"2025-01-15T10:00:01Z","sessionId":"abc123","message":{"role":"assistant","content":"In Rust, you can handle errors using the Result type..."}}
```

Message content can also be an array of content blocks (for tool results):

```json
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Here's the file content:"},{"type":"text","text":"fn main() {}"}]}}
```

Other record types (like `file-history-snapshot`) are skipped during indexing.

### Indexed Fields

| Field | Description |
|-------|-------------|
| `content` | Full-text searchable message content |
| `role` | "user" or "assistant" |
| `project` | Decoded project path |
| `session_id` | Claude Code session identifier |
| `timestamp` | Message creation time |
| `doc_type` | Document type ("conversation") |

## Development

```bash
make help     # Show available commands
make check    # Format, lint, and test
make bench    # Run benchmarks
```

## License

MIT
