# glhf Visual Design Specification

Inspired by [Charm Bracelet](https://charm.sh) (Lip Gloss, Glamour, Bubble Tea) and [Rich/Textual](https://github.com/Textualize/rich).

## Design Philosophy

### Core Principles

1. **Dual-mode output**: Beautiful for humans, efficient for Claude
2. **Progressive enhancement**: Graceful degradation based on terminal capabilities
3. **Semantic colors**: Colors convey meaning, not just decoration
4. **Comfortable density**: Information-rich without feeling cluttered
5. **Distinctive identity**: A consistent visual language that's recognizable

### Output Modes

| Mode | Target | Styling |
|------|--------|---------|
| `--json` | Machines | None (structured data) |
| `--compact` | Claude/scripts | Minimal (icons only) |
| Default | Humans (TTY) | Full styling |
| Piped output | Scripts | Auto-disable styling |

---

## Color Palette

Inspired by the synthwave/vaporwave aesthetic of Charm, but with a more subdued, professional feel.

### Primary Colors (Semantic)

```
┌─────────────────────────────────────────────────────────────┐
│  CHUNK TYPES                                                │
├─────────────────────────────────────────────────────────────┤
│  message:user      │  cyan/teal     │  Human input          │
│  message:assistant │  magenta/pink  │  AI responses         │
│  tool_use          │  yellow/gold   │  Tool invocations     │
│  tool_result       │  green         │  Successful results   │
│  tool_result:error │  red           │  Error results        │
└─────────────────────────────────────────────────────────────┘
```

### Secondary Colors (UI Elements)

```
┌─────────────────────────────────────────────────────────────┐
│  UI ELEMENTS                                                │
├─────────────────────────────────────────────────────────────┤
│  borders/dividers  │  dim gray      │  Structure            │
│  timestamps        │  dim           │  Metadata             │
│  scores            │  blue gradient │  Relevance indicator  │
│  session IDs       │  dim cyan      │  Reference codes      │
│  project names     │  bold white    │  Context              │
│  matched text      │  inverse/bold  │  Search highlights    │
└─────────────────────────────────────────────────────────────┘
```

---

## Component Designs

### Search Results (Default Mode)

```
╭───────────────────────────────────────────────────────────────────╮
│  🔍 Found 5 results for "error handling"                          │
╰───────────────────────────────────────────────────────────────────╯

┌─ tool_use ─────────────────────────────────────── 2h ago ─┐
│  📁 glhf  ·  🔧 Bash  ·  sess:a4f2c1d8                    │
├───────────────────────────────────────────────────────────┤
│  cargo test -- --nocapture 2>&1 | grep -i error           │
└───────────────────────────────────────────────────────────┘

┌─ message ──────────────────────────────────────── 2h ago ─┐
│  📁 glhf  ·  🤖 assistant  ·  sess:a4f2c1d8               │
├───────────────────────────────────────────────────────────┤
│  I'll add proper error handling for the edge case where   │
│  the file doesn't exist. Let me update the function...    │
└───────────────────────────────────────────────────────────┘
```

### Search Results (Compact Mode - for Claude)

Optimized for token efficiency while remaining scannable:

```
Found 5 results:
⚡ [1] glhf │ Bash │ 2h │ a4f2c1d8 │ "cargo test -- --nocapture..."
💬 [2] glhf │ asst │ 2h │ a4f2c1d8 │ "I'll add proper error hand..."
🔧 [3] myapp│ Read │ 1d │ b3e9f2a1 │ "src/lib.rs:142-156"
💬 [4] myapp│ user │ 1d │ b3e9f2a1 │ "can you fix the null check"
✅ [5] myapp│ Bash │ 1d │ b3e9f2a1 │ "Tests passed: 42/42"
```

### Status Command

```
╭──────────────────────────────────────────────────╮
│               📊 Database Status                 │
├──────────────────────────────────────────────────┤
│                                                  │
│   Documents    ████████████░░░░  12,847          │
│   Embeddings   ████████████░░░░  12,847  100%    │
│   Size         ████░░░░░░░░░░░░  156.2 MB        │
│                                                  │
│   📍 ~/.local/share/glhf/glhf.db                 │
│                                                  │
╰──────────────────────────────────────────────────╯
```

### Projects List

```
╭─────────────────────────────────────────────────────────────╮
│  📂 Projects (8 total)                                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  glhf                 ████████████  2,847 docs   2h ago     │
│  webapp               ████████░░░░  1,923 docs   1d ago     │
│  api-server           ██████░░░░░░  1,456 docs   3d ago     │
│  scripts              ████░░░░░░░░    892 docs   1w ago     │
│  dotfiles             ██░░░░░░░░░░    234 docs   2w ago     │
│                                                             │
╰─────────────────────────────────────────────────────────────╯
```

### Session Summary

```
╭─────────────────────────────────────────────────────────────╮
│  📋 Session a4f2c1d8                                        │
│  📁 glhf  ·  ⏱️  2h 34m  ·  started 3h ago                  │
╰─────────────────────────────────────────────────────────────╯

   Message Breakdown               Tool Usage
  ┌────────────────────┐          ┌────────────────────┐
  │ 👤 user       24   │          │ 📖 Read       47   │
  │ 🤖 assistant  28   │          │ ✏️  Edit       23   │
  │ 🔧 tool_use   89   │          │ 💻 Bash       12   │
  │ ✅ tool_result 89  │          │ 🔍 Grep        7   │
  └────────────────────┘          └────────────────────┘

   Total: 230 messages
```

### Indexing Progress

```
╭─────────────────────────────────────────────────────────────╮
│  🔄 Building search index...                                │
╰─────────────────────────────────────────────────────────────╯

  Scanning projects   ████████████████████  45/45
  Parsing files       ████████████░░░░░░░░  8,234 chunks
  Generating vectors  ██████░░░░░░░░░░░░░░  32%  [2,634/8,234]
                      ◐  ETA: 45s

```

### Context View (with highlighting)

```
┌─ Context ───────────────────────────────────────────────────┐
│                                                             │
│     [user] "how do I handle the connection timeout?"        │
│                                                             │
│ >>> [assistant] "You can wrap the connection in a          │
│ >>>  tokio::time::timeout to handle connection timeouts.   │
│ >>>  Here's an example..."                                  │
│                                                             │
│     [tool:Bash] "cargo run --example timeout_demo"          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Typography & Symbols

### Icons (Unicode)

| Concept | Icon | Fallback |
|---------|------|----------|
| User message | 👤 | [user] |
| Assistant | 🤖 | [asst] |
| Tool use | 🔧 | [tool] |
| Tool result | ✅ | [done] |
| Error | ❌ | [err] |
| Project | 📁 | [proj] |
| Search | 🔍 | [find] |
| Time | ⏱️ | [time] |
| Session | 📋 | [sess] |
| Database | 📊 | [db] |

### Box Drawing

Use rounded corners for a softer, more modern feel:

```
╭─────────────────────────╮   ┌─────────────────────────┐
│  Rounded (preferred)    │   │  Sharp (fallback)       │
╰─────────────────────────╯   └─────────────────────────┘
```

---

## Implementation Plan

### Rust Crates to Add

```toml
[dependencies]
# Terminal styling
console = "0.15"           # TTY detection, colors, styling
owo-colors = "4"           # Zero-cost color abstractions

# Tables and layouts
comfy-table = "7"          # Beautiful tables with auto-sizing

# Progress indicators
indicatif = "0.17"         # Progress bars and spinners
```

### Architecture

```
src/
├── style/
│   ├── mod.rs            # Style module exports
│   ├── theme.rs          # Color themes (dark/light)
│   ├── components.rs     # Reusable styled components
│   ├── icons.rs          # Unicode icons with fallbacks
│   └── detect.rs         # Terminal capability detection
├── output/
│   ├── mod.rs            # Output mode handling
│   ├── human.rs          # Styled human output
│   ├── compact.rs        # Claude-optimized output
│   └── json.rs           # JSON output
```

### Terminal Detection

```rust
fn should_style() -> bool {
    // Auto-detect based on environment
    !std::env::var("NO_COLOR").is_ok()
        && console::Term::stdout().is_term()
        && !is_compact_mode()
        && !is_json_mode()
}
```

### Color Adaptation

```rust
// Adapt to terminal capabilities
match console::colors_enabled() {
    true if console::Term::stdout().features().colors_supported() => {
        // Full 24-bit color
    }
    true => {
        // 256 or 16 colors
    }
    false => {
        // No colors, use bold/dim/underline only
    }
}
```

---

## Accessibility

1. **NO_COLOR support**: Respect the `NO_COLOR` environment variable
2. **High contrast**: Ensure sufficient contrast ratios
3. **Screen readers**: Meaningful alt text in output structure
4. **Monochrome mode**: All information conveyed without color (icons, structure)
5. **ASCII fallback**: For terminals without Unicode support

---

## CLI Flags

```
--color=<when>     Color output: always, auto, never [default: auto]
--icons=<when>     Use icons: always, auto, never, ascii [default: auto]
--theme=<name>     Color theme: dark, light, mono [default: auto]
```

---

## Examples

### Before (Current)

```
Found 5 results:

[1] tool_use | glhf | tool:Bash | 2h ago
    "cargo test -- --nocapture"

[2] message | glhf | assistant | 2h ago
    "I'll add proper error handling..."
```

### After (Styled)

```
╭───────────────────────────────────────────────────────────────╮
│  🔍 Found 5 results                                           │
╰───────────────────────────────────────────────────────────────╯

  🔧 glhf · Bash · 2h ago                            sess:a4f2c1d8
  ╭─────────────────────────────────────────────────────────────╮
  │ cargo test -- --nocapture                                   │
  ╰─────────────────────────────────────────────────────────────╯

  🤖 glhf · assistant · 2h ago                       sess:a4f2c1d8
  ╭─────────────────────────────────────────────────────────────╮
  │ I'll add proper error handling for the edge case where the  │
  │ file doesn't exist. Let me update the function...           │
  ╰─────────────────────────────────────────────────────────────╯
```

---

## Related Sessions Output

```
╭─────────────────────────────────────────────────────────────╮
│  🔗 Related Sessions to a4f2c1d8                            │
╰─────────────────────────────────────────────────────────────╯

  Score   Session     Project     Messages   When
  ─────────────────────────────────────────────────────────────
  0.94    b3e9f2a1    glhf        156        yesterday
  0.87    c7d1e3f4    glhf        89         3d ago
  0.72    d8e2f5a6    rust-proj   234        1w ago
```

---

## Recent Sessions Output

```
╭─────────────────────────────────────────────────────────────╮
│  📅 Recent Sessions                                         │
╰─────────────────────────────────────────────────────────────╯

  Session     Project              Messages   Duration   When
  ─────────────────────────────────────────────────────────────
  a4f2c1d8    glhf                 230        2h 34m     3h ago
  b3e9f2a1    webapp               156        1h 12m     yesterday
  c7d1e3f4    api-server           89         45m        3d ago
```

---

## Score Visualization

For search relevance scores:

```
[0.95] ████████████████████ Excellent match
[0.75] ███████████████░░░░░ Good match
[0.50] ██████████░░░░░░░░░░ Moderate match
[0.25] █████░░░░░░░░░░░░░░░ Weak match
```

---

## Summary

This design brings glhf's visual output on par with modern CLI tools like those from Charm Bracelet, while maintaining the efficiency needed for AI-assisted workflows. The key is progressive enhancement - beautiful by default for humans, lean for machines.

Sources:
- [Charm Bracelet Lip Gloss](https://github.com/charmbracelet/lipgloss)
- [Rich Python Library](https://github.com/Textualize/rich)
- [Comfy Table](https://github.com/Nukesor/comfy-table)
- [Ratatui](https://github.com/ratatui/ratatui)
