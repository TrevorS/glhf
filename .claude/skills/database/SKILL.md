---
name: database
description: Work with the SQLite database layer including FTS5, sqlite-vec, and schema operations. Use for database queries, indexing issues, or storage debugging.
---

# SQLite Database Layer

## Schema

```sql
-- Main documents table
CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    chunk_kind TEXT NOT NULL,  -- 'message', 'tool_use', 'tool_result'
    content TEXT NOT NULL,
    project TEXT,
    session_id TEXT,
    role TEXT,                 -- 'user', 'assistant'
    tool_name TEXT,
    tool_id TEXT,
    tool_input TEXT,
    is_error INTEGER,
    timestamp TEXT,
    source_path TEXT NOT NULL
);

-- FTS5 full-text search
CREATE VIRTUAL TABLE documents_fts USING fts5(
    content,
    content='documents',
    content_rowid='rowid'
);

-- Triggers for FTS sync
CREATE TRIGGER documents_ai AFTER INSERT ON documents BEGIN
    INSERT INTO documents_fts(rowid, content)
    VALUES (NEW.rowid, NEW.content);
END;

-- Vector embeddings (512 dims for Potion-base-32M)
CREATE VIRTUAL TABLE documents_vec USING vec0(
    id TEXT PRIMARY KEY,
    embedding FLOAT[512]
);
```

## sqlite-vec Initialization

**CRITICAL**: Must use `sqlite3_auto_extension` before any connections:

```rust
use std::sync::Once;

static SQLITE_VEC_INIT: Once = Once::new();

fn init_sqlite_vec() {
    SQLITE_VEC_INIT.call_once(|| {
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
    });
}

pub fn open(path: &Path) -> Result<Database> {
    init_sqlite_vec();  // Must be before Connection::open
    let conn = Connection::open(path)?;
    // ...
}
```

## Common Queries

### FTS5 Search
```sql
SELECT d.*, fts.rank
FROM documents_fts fts
JOIN documents d ON d.rowid = fts.rowid
WHERE documents_fts MATCH ?1
ORDER BY fts.rank
LIMIT ?2;
```

### Vector Search
```sql
SELECT d.*, v.distance
FROM documents_vec v
JOIN documents d ON d.id = v.id
WHERE v.embedding MATCH ?1
  AND k = ?2
ORDER BY v.distance;
```

### Filtered Search
```sql
SELECT d.*, fts.rank
FROM documents_fts fts
JOIN documents d ON d.rowid = fts.rowid
WHERE documents_fts MATCH ?1
  AND (?2 IS NULL OR d.tool_name = ?2)
  AND (?3 = 0 OR d.is_error = 1)
ORDER BY fts.rank
LIMIT ?4;
```

## Database Location

```rust
// ~/.cache/glhf/glhf.db
pub fn database_path() -> Result<PathBuf> {
    let cache = dirs::cache_dir()
        .ok_or(Error::MissingDirectory { dir_type: "cache" })?;
    Ok(cache.join("glhf").join("glhf.db"))
}
```

## Debugging

### Check database stats
```bash
glhf status
```

### Inspect with sqlite3
```bash
sqlite3 ~/.cache/glhf/glhf.db
.tables
SELECT COUNT(*) FROM documents;
SELECT chunk_kind, COUNT(*) FROM documents GROUP BY chunk_kind;
```

### Verify FTS5
```sql
SELECT * FROM documents_fts WHERE documents_fts MATCH 'search term';
```

### Verify sqlite-vec
```sql
SELECT vec_version();
SELECT COUNT(*) FROM documents_vec;
```
