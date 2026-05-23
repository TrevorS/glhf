mod common;

use common::*;
use glhf::db::Database;
use glhf::ingest::parse_jsonl_file;
use glhf::{ChunkKind, Document};
use std::path::PathBuf;

#[test]
fn test_parse_user_and_assistant_messages() {
    let env = TestEnv::new();
    let project_dir = env.create_project("test/project");

    let msg1 = user_message("How do I search for files?", "session-1");
    let msg2 = assistant_message("Use the grep command for searching.", "session-1");
    let lines = vec![msg1.as_str(), msg2.as_str()];
    let jsonl_path = env.write_jsonl(&project_dir, "conversation.jsonl", &lines);

    let docs = parse_jsonl_file(&jsonl_path).expect("Failed to parse JSONL");

    assert_eq!(docs.len(), 2);

    // Check user message
    assert_eq!(docs[0].role.as_deref(), Some("user"));
    assert!(docs[0].content.contains("search for files"));
    assert_eq!(docs[0].session_id.as_deref(), Some("session-1"));

    // Check assistant message
    assert_eq!(docs[1].role.as_deref(), Some("assistant"));
    assert!(docs[1].content.contains("grep command"));
}

#[test]
fn test_parse_array_content_blocks() {
    let env = TestEnv::new();
    let project_dir = env.create_project("test/blocks");

    let msg = assistant_with_blocks(&["First block", "Second block"], "session-2");
    let lines = vec![msg.as_str()];
    let jsonl_path = env.write_jsonl(&project_dir, "blocks.jsonl", &lines);

    let docs = parse_jsonl_file(&jsonl_path).expect("Failed to parse JSONL");

    assert_eq!(docs.len(), 1);
    assert!(docs[0].content.contains("First block"));
    assert!(docs[0].content.contains("Second block"));
}

#[test]
fn test_skip_non_message_types() {
    let env = TestEnv::new();
    let project_dir = env.create_project("test/skip");

    let snapshot = file_history_snapshot();
    let user_msg = user_message("Real message", "session-3");
    let lines = vec![snapshot.as_str(), user_msg.as_str()];
    let jsonl_path = env.write_jsonl(&project_dir, "mixed.jsonl", &lines);

    let docs = parse_jsonl_file(&jsonl_path).expect("Failed to parse JSONL");

    // Only the user message should be parsed
    assert_eq!(docs.len(), 1);
    assert!(docs[0].content.contains("Real message"));
}

#[test]
fn test_handle_malformed_json_gracefully() {
    let env = TestEnv::new();
    let project_dir = env.create_project("test/malformed");

    let good_msg = user_message("Good message", "session-4");
    let bad_msg = malformed_json();
    let lines = vec![good_msg.as_str(), bad_msg.as_str()];
    let jsonl_path = env.write_jsonl(&project_dir, "malformed.jsonl", &lines);

    let docs = parse_jsonl_file(&jsonl_path).expect("Failed to parse JSONL");

    // Should still get the good message
    assert_eq!(docs.len(), 1);
    assert!(docs[0].content.contains("Good message"));
}

#[test]
fn test_database_insert_and_search() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("test.db");

    // Create some documents
    let docs = vec![
        Document::new(
            ChunkKind::Message,
            "Rust programming language is great for systems".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )
        .with_role(Some("user".to_string())),
        Document::new(
            ChunkKind::Message,
            "Python is good for machine learning".to_string(),
            PathBuf::from("/test/2.jsonl"),
        )
        .with_role(Some("assistant".to_string())),
        Document::new(
            ChunkKind::Message,
            "JavaScript runs in browsers".to_string(),
            PathBuf::from("/test/3.jsonl"),
        )
        .with_role(Some("user".to_string())),
    ];

    // Create database and insert
    let mut db = Database::open(&db_path).expect("Failed to create database");
    db.insert_documents(&docs)
        .expect("Failed to insert documents");

    // Search for Rust
    let results = db.search_fts("Rust", 10).expect("Search failed");
    assert!(!results.is_empty());
    assert!(results[0].content.contains("Rust"));

    // Search for Python
    let results = db
        .search_fts("machine learning", 10)
        .expect("Search failed");
    assert!(!results.is_empty());
    assert!(results[0].content.contains("Python"));

    // Search with limit
    let results = db.search_fts("programming", 1).expect("Search failed");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_no_results() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("test_empty.db");

    let docs = vec![Document::new(
        ChunkKind::Message,
        "Hello world".to_string(),
        PathBuf::from("/test/1.jsonl"),
    )];

    let mut db = Database::open(&db_path).expect("Failed to create database");
    db.insert_documents(&docs)
        .expect("Failed to insert documents");

    let results = db.search_fts("xyznonexistent", 10).expect("Search failed");
    assert!(results.is_empty());
}

#[test]
fn test_database_document_count() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("test_count.db");

    let docs: Vec<Document> = (0..5)
        .map(|i| {
            Document::new(
                ChunkKind::Message,
                format!("Document number {i}"),
                PathBuf::from(format!("/test/{i}.jsonl")),
            )
        })
        .collect();

    let mut db = Database::open(&db_path).expect("Failed to create database");
    db.insert_documents(&docs)
        .expect("Failed to insert documents");

    assert_eq!(db.document_count().unwrap(), 5);
}

#[test]
fn test_reopen_database() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("test_reopen.db");

    // Create and populate database
    {
        let docs = vec![Document::new(
            ChunkKind::Message,
            "Persistent data".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )];

        let mut db = Database::open(&db_path).expect("Failed to create database");
        db.insert_documents(&docs)
            .expect("Failed to insert documents");
    }

    // Reopen and verify
    let db = Database::open(&db_path).expect("Failed to open database");
    assert_eq!(db.document_count().unwrap(), 1);

    let results = db.search_fts("Persistent", 10).expect("Search failed");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_result_metadata() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("test_meta.db");

    let doc = Document::new(
        ChunkKind::Message,
        "Test content for metadata".to_string(),
        PathBuf::from("/test/meta.jsonl"),
    )
    .with_project(Some("/Users/test/project".to_string()))
    .with_session_id(Some("session-xyz".to_string()))
    .with_role(Some("assistant".to_string()));

    let mut db = Database::open(&db_path).expect("Failed to create database");
    db.insert_documents(&[doc])
        .expect("Failed to insert documents");

    let results = db.search_fts("metadata", 10).expect("Search failed");
    assert_eq!(results.len(), 1);

    let result = &results[0];
    assert_eq!(result.project.as_deref(), Some("/Users/test/project"));
    assert_eq!(result.session_id.as_deref(), Some("session-xyz"));
    assert_eq!(result.role.as_deref(), Some("assistant"));
    assert_eq!(result.chunk_kind, "message");
}

#[test]
fn test_tool_use_indexing() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("test_tools.db");

    let docs = vec![
        Document::new(
            ChunkKind::ToolUse,
            "git status".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )
        .with_tool_name(Some("Bash".to_string()))
        .with_tool_id(Some("tool-123".to_string()))
        .with_tool_input(Some(r#"{"command": "git status"}"#.to_string())),
        Document::new(
            ChunkKind::ToolResult,
            "On branch main".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )
        .with_tool_id(Some("tool-123".to_string()))
        .with_is_error(Some(false)),
    ];

    let mut db = Database::open(&db_path).expect("Failed to create database");
    db.insert_documents(&docs)
        .expect("Failed to insert documents");

    // Search for git
    let results = db.search_fts("git", 10).expect("Search failed");
    assert!(!results.is_empty());
    assert_eq!(results[0].chunk_kind, "tool_use");
    assert_eq!(results[0].tool_name.as_deref(), Some("Bash"));

    // Search for branch
    let results = db.search_fts("branch main", 10).expect("Search failed");
    assert!(!results.is_empty());
    assert_eq!(results[0].chunk_kind, "tool_result");
}

#[test]
fn test_filtered_search() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("test_filter.db");

    let docs = vec![
        Document::new(
            ChunkKind::Message,
            "User asking about git".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )
        .with_role(Some("user".to_string())),
        Document::new(
            ChunkKind::ToolUse,
            "git status".to_string(),
            PathBuf::from("/test/2.jsonl"),
        )
        .with_tool_name(Some("Bash".to_string())),
        Document::new(
            ChunkKind::ToolResult,
            "git output".to_string(),
            PathBuf::from("/test/3.jsonl"),
        )
        .with_tool_name(Some("Bash".to_string()))
        .with_is_error(Some(true)),
    ];

    let mut db = Database::open(&db_path).expect("Failed to create database");
    db.insert_documents(&docs)
        .expect("Failed to insert documents");

    // Filter by chunk kind (messages only)
    let results = db
        .search_fts_filtered("git", 10, Some(ChunkKind::Message), None, false)
        .expect("Search failed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].chunk_kind, "message");

    // Filter by tool name
    let results = db
        .search_fts_filtered("git", 10, None, Some("Bash"), false)
        .expect("Search failed");
    assert_eq!(results.len(), 2);

    // Filter by errors
    let results = db
        .search_fts_filtered("git", 10, None, None, true)
        .expect("Search failed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].is_error, Some(true));
}

#[test]
fn test_fts_special_characters() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("test_special.db");

    let docs = vec![
        Document::new(
            ChunkKind::Message,
            "Learning C++ programming".to_string(),
            PathBuf::from("/test/1.jsonl"),
        ),
        Document::new(
            ChunkKind::Message,
            "Install node.js and npm".to_string(),
            PathBuf::from("/test/2.jsonl"),
        ),
        Document::new(
            ChunkKind::Message,
            "Set $HOME variable".to_string(),
            PathBuf::from("/test/3.jsonl"),
        ),
        Document::new(
            ChunkKind::Message,
            "Email user@example.com for help".to_string(),
            PathBuf::from("/test/4.jsonl"),
        ),
        Document::new(
            ChunkKind::Message,
            "Use foo{bar} syntax".to_string(),
            PathBuf::from("/test/5.jsonl"),
        ),
    ];

    let mut db = Database::open(&db_path).expect("Failed to create database");
    db.insert_documents(&docs).expect("Failed to insert");

    // These should not crash - they all broke before the fix
    let results = db.search_fts("C++", 10).expect("C++ search failed");
    assert_eq!(results.len(), 1);
    assert!(results[0].content.contains("C++"));

    let results = db.search_fts("node.js", 10).expect("node.js search failed");
    assert_eq!(results.len(), 1);

    let results = db.search_fts("$HOME", 10).expect("$HOME search failed");
    assert_eq!(results.len(), 1);

    let results = db
        .search_fts("user@example.com", 10)
        .expect("@ search failed");
    assert_eq!(results.len(), 1);

    let results = db.search_fts("foo{bar}", 10).expect("{} search failed");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_fts_reserved_keywords() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("test_keywords.db");

    let docs = vec![
        Document::new(
            ChunkKind::Message,
            "Use OR for alternatives".to_string(),
            PathBuf::from("/test/1.jsonl"),
        ),
        Document::new(
            ChunkKind::Message,
            "Use AND for both conditions".to_string(),
            PathBuf::from("/test/2.jsonl"),
        ),
        Document::new(
            ChunkKind::Message,
            "NOT is a negation operator".to_string(),
            PathBuf::from("/test/3.jsonl"),
        ),
    ];

    let mut db = Database::open(&db_path).expect("Failed to create database");
    db.insert_documents(&docs).expect("Failed to insert");

    // Searching for literal keywords should not crash
    let results = db.search_fts("OR", 10).expect("OR search failed");
    assert!(!results.is_empty());

    let results = db.search_fts("AND", 10).expect("AND search failed");
    assert!(!results.is_empty());

    let results = db.search_fts("NOT", 10).expect("NOT search failed");
    assert!(!results.is_empty());
}

#[test]
fn test_fts_empty_query() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("test_empty_query.db");

    let docs = vec![Document::new(
        ChunkKind::Message,
        "Some content here".to_string(),
        PathBuf::from("/test/1.jsonl"),
    )];

    let mut db = Database::open(&db_path).expect("Failed to create database");
    db.insert_documents(&docs).expect("Failed to insert");

    // Empty queries should return empty results, not crash
    let results = db.search_fts("", 10).expect("Empty search failed");
    assert!(results.is_empty());

    let results = db.search_fts("   ", 10).expect("Whitespace search failed");
    assert!(results.is_empty());

    // Filtered search too
    let results = db
        .search_fts_filtered("", 10, None, None, false)
        .expect("Empty filtered search failed");
    assert!(results.is_empty());
}

// --- Incremental indexing integration tests ---

#[test]
fn test_incremental_index_new_file() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("incr.db");
    let project_dir = env.create_project("test/incr");

    // First file
    let msg1 = user_message("first session message", "session-1");
    let path1 = env.write_jsonl(&project_dir, "session-1.jsonl", &[msg1.as_str()]);
    let docs1 = parse_jsonl_file(&path1).unwrap();

    let mut db = Database::open(&db_path).unwrap();
    db.insert_documents(&docs1).unwrap();
    db.upsert_file_meta(&path1.to_string_lossy(), 1000, docs1.len())
        .unwrap();

    assert_eq!(db.document_count().unwrap(), 1);

    // Second file (new)
    let msg2 = user_message("second session message", "session-2");
    let path2 = env.write_jsonl(&project_dir, "session-2.jsonl", &[msg2.as_str()]);
    let docs2 = parse_jsonl_file(&path2).unwrap();

    // Simulate incremental: check meta, find new file, ingest it
    let indexed = db.get_indexed_files().unwrap();
    assert!(!indexed.contains_key(&path2.to_string_lossy().to_string()));

    db.insert_documents(&docs2).unwrap();
    db.upsert_file_meta(&path2.to_string_lossy(), 2000, docs2.len())
        .unwrap();

    assert_eq!(db.document_count().unwrap(), 2);

    // Both searchable
    let results = db.search_fts("session message", 10).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_incremental_index_modified_file() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("incr_mod.db");
    let project_dir = env.create_project("test/incr-mod");

    // Initial ingest
    let msg1 = user_message("original content", "session-1");
    let path = env.write_jsonl(&project_dir, "session-1.jsonl", &[msg1.as_str()]);
    let docs = parse_jsonl_file(&path).unwrap();

    let mut db = Database::open(&db_path).unwrap();
    db.insert_documents(&docs).unwrap();
    db.upsert_file_meta(&path.to_string_lossy(), 1000, docs.len())
        .unwrap();

    // Verify original content
    let results = db.search_fts("original", 10).unwrap();
    assert_eq!(results.len(), 1);

    // Simulate file modification: delete old docs, re-ingest with new content
    // (In production, we'd overwrite the file; here we just test the DB workflow)
    let path_str = path.to_string_lossy().to_string();
    db.delete_documents_by_source(&path_str).unwrap();

    let msg2 = user_message("updated content", "session-1");
    let msg3 = assistant_message("new reply", "session-1");
    let new_path = env.write_jsonl(
        &project_dir,
        "session-1.jsonl",
        &[msg2.as_str(), msg3.as_str()],
    );
    let new_docs = parse_jsonl_file(&new_path).unwrap();
    db.insert_documents(&new_docs).unwrap();
    db.upsert_file_meta(&path_str, 2000, new_docs.len())
        .unwrap();

    // Old content gone, new content present
    let results = db.search_fts("original", 10).unwrap();
    assert!(results.is_empty());

    let results = db.search_fts("updated", 10).unwrap();
    assert_eq!(results.len(), 1);

    assert_eq!(db.document_count().unwrap(), 2);
}

#[test]
fn test_incremental_index_deleted_file() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("incr_del.db");
    let project_dir = env.create_project("test/incr-del");

    let msg1 = user_message("alpha unique content", "session-1");
    let msg2 = user_message("beta unique content", "session-2");
    let path_a = env.write_jsonl(&project_dir, "session-1.jsonl", &[msg1.as_str()]);
    let path_b = env.write_jsonl(&project_dir, "session-2.jsonl", &[msg2.as_str()]);

    let mut db = Database::open(&db_path).unwrap();
    for (path, docs) in [
        (&path_a, parse_jsonl_file(&path_a).unwrap()),
        (&path_b, parse_jsonl_file(&path_b).unwrap()),
    ] {
        let count = docs.len();
        db.insert_documents(&docs).unwrap();
        db.upsert_file_meta(&path.to_string_lossy(), 1000, count)
            .unwrap();
    }

    assert_eq!(db.document_count().unwrap(), 2);

    // Simulate file a being deleted from disk
    std::fs::remove_file(&path_a).unwrap();
    db.delete_documents_by_source(&path_a.to_string_lossy())
        .unwrap();

    assert_eq!(db.document_count().unwrap(), 1);
    let results = db.search_fts("alpha", 10).unwrap();
    assert!(results.is_empty());
    let results = db.search_fts("beta", 10).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_tool_use_parsing() {
    let env = TestEnv::new();
    let project_dir = env.create_project("test/tools");

    let tool = tool_use_message("Bash", "git status", "session-1");
    let result = tool_result_message("On branch main", false, "session-1");
    let path = env.write_jsonl(
        &project_dir,
        "tools.jsonl",
        &[tool.as_str(), result.as_str()],
    );

    let docs = parse_jsonl_file(&path).unwrap();
    // Should parse at least the tool_use (tool_result parsing depends on format)
    assert!(!docs.is_empty());

    // Check tool_use was parsed
    let tool_docs: Vec<_> = docs
        .iter()
        .filter(|d| d.chunk_kind == ChunkKind::ToolUse)
        .collect();
    assert!(!tool_docs.is_empty());
    assert_eq!(tool_docs[0].tool_name.as_deref(), Some("Bash"));
}

#[test]
fn test_status_stats() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("stats.db");

    let docs = vec![
        Document::new(
            ChunkKind::Message,
            "user question".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )
        .with_role(Some("user".to_string()))
        .with_session_id(Some("s1".to_string()))
        .with_project(Some("project-a".to_string())),
        Document::new(
            ChunkKind::Message,
            "assistant reply".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )
        .with_role(Some("assistant".to_string()))
        .with_session_id(Some("s1".to_string()))
        .with_project(Some("project-a".to_string())),
        Document::new(
            ChunkKind::ToolUse,
            "git status".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )
        .with_tool_name(Some("Bash".to_string()))
        .with_session_id(Some("s1".to_string()))
        .with_project(Some("project-a".to_string())),
        Document::new(
            ChunkKind::ToolResult,
            "error output".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )
        .with_is_error(Some(true))
        .with_session_id(Some("s1".to_string()))
        .with_project(Some("project-a".to_string())),
    ];

    let mut db = Database::open(&db_path).unwrap();
    db.insert_documents(&docs).unwrap();

    let stats = db.status_stats().unwrap();
    assert_eq!(stats.session_count, 1);
    assert_eq!(stats.project_count, 1);
    assert_eq!(stats.error_count, 1);
    assert_eq!(stats.tool_result_count, 1);
    assert_eq!(*stats.chunk_counts.get("message").unwrap_or(&0), 2);
    assert_eq!(*stats.chunk_counts.get("tool_use").unwrap_or(&0), 1);
    assert_eq!(*stats.role_counts.get("user").unwrap_or(&0), 1);
    assert_eq!(*stats.role_counts.get("assistant").unwrap_or(&0), 1);
}

#[test]
fn test_recent_sessions() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("recent.db");

    let docs = vec![
        Document::new(
            ChunkKind::Message,
            "old message".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )
        .with_session_id(Some("old-session".to_string()))
        .with_project(Some("project-a".to_string()))
        .with_timestamp(Some(
            chrono::DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        )),
        Document::new(
            ChunkKind::Message,
            "new message".to_string(),
            PathBuf::from("/test/2.jsonl"),
        )
        .with_session_id(Some("new-session".to_string()))
        .with_project(Some("project-a".to_string()))
        .with_timestamp(Some(
            chrono::DateTime::parse_from_rfc3339("2025-06-01T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        )),
    ];

    let mut db = Database::open(&db_path).unwrap();
    db.insert_documents(&docs).unwrap();

    // Get recent sessions
    let sessions = db.get_recent_sessions(10, None).unwrap();
    assert_eq!(sessions.len(), 2);
    // Most recent first
    assert_eq!(sessions[0].session_id, "new-session");
    assert_eq!(sessions[1].session_id, "old-session");

    // With limit
    let sessions = db.get_recent_sessions(1, None).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "new-session");
}

#[test]
fn test_find_sessions_by_partial_id() {
    let env = TestEnv::new();
    let db_path = env.index_dir.join("find_sess.db");

    let docs = vec![
        Document::new(
            ChunkKind::Message,
            "hello".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )
        .with_session_id(Some("abc123-def456".to_string())),
        Document::new(
            ChunkKind::Message,
            "world".to_string(),
            PathBuf::from("/test/2.jsonl"),
        )
        .with_session_id(Some("xyz789-def456".to_string())),
    ];

    let mut db = Database::open(&db_path).unwrap();
    db.insert_documents(&docs).unwrap();

    // Partial match
    let found = db.find_sessions("abc123").unwrap();
    assert_eq!(found.len(), 1);

    // Matches both (shared substring)
    let found = db.find_sessions("def456").unwrap();
    assert_eq!(found.len(), 2);

    // No match
    let found = db.find_sessions("zzz").unwrap();
    assert!(found.is_empty());
}
