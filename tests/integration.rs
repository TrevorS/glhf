mod common;

use common::*;
use glhf::index::BM25Index;
use glhf::ingest::parse_jsonl_file;
use glhf::models::document::{ChunkKind, Document};
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
fn test_index_and_search() {
    let env = TestEnv::new();

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

    // Create index
    let index = BM25Index::create(&env.index_dir).expect("Failed to create index");
    let mut writer = index.writer().expect("Failed to create writer");
    index
        .add_documents(&mut writer, &docs)
        .expect("Failed to add documents");
    writer.commit().expect("Failed to commit");
    index.reload().expect("Failed to reload reader");

    // Search for Rust
    let results = index.search("Rust programming", 10).expect("Search failed");
    assert!(!results.is_empty());
    assert!(results[0].content.contains("Rust"));

    // Search for Python
    let results = index.search("machine learning", 10).expect("Search failed");
    assert!(!results.is_empty());
    assert!(results[0].content.contains("Python"));

    // Search with limit
    let results = index.search("programming", 1).expect("Search failed");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_no_results() {
    let env = TestEnv::new();

    let docs = vec![Document::new(
        ChunkKind::Message,
        "Hello world".to_string(),
        PathBuf::from("/test/1.jsonl"),
    )];

    let index = BM25Index::create(&env.index_dir).expect("Failed to create index");
    let mut writer = index.writer().expect("Failed to create writer");
    index
        .add_documents(&mut writer, &docs)
        .expect("Failed to add documents");
    writer.commit().expect("Failed to commit");
    index.reload().expect("Failed to reload reader");

    let results = index.search("xyznonexistent", 10).expect("Search failed");
    assert!(results.is_empty());
}

#[test]
fn test_index_document_count() {
    let env = TestEnv::new();

    let docs: Vec<Document> = (0..5)
        .map(|i| {
            Document::new(
                ChunkKind::Message,
                format!("Document number {}", i),
                PathBuf::from(format!("/test/{}.jsonl", i)),
            )
        })
        .collect();

    let index = BM25Index::create(&env.index_dir).expect("Failed to create index");
    let mut writer = index.writer().expect("Failed to create writer");
    index
        .add_documents(&mut writer, &docs)
        .expect("Failed to add documents");
    writer.commit().expect("Failed to commit");
    index.reload().expect("Failed to reload reader");

    assert_eq!(index.num_docs(), 5);
}

#[test]
fn test_reopen_index() {
    let env = TestEnv::new();

    // Create and populate index
    {
        let docs = vec![Document::new(
            ChunkKind::Message,
            "Persistent data".to_string(),
            PathBuf::from("/test/1.jsonl"),
        )];

        let index = BM25Index::create(&env.index_dir).expect("Failed to create index");
        let mut writer = index.writer().expect("Failed to create writer");
        index
            .add_documents(&mut writer, &docs)
            .expect("Failed to add documents");
        writer.commit().expect("Failed to commit");
    }

    // Reopen and verify
    let index = BM25Index::open(&env.index_dir).expect("Failed to open index");
    assert_eq!(index.num_docs(), 1);

    let results = index.search("Persistent", 10).expect("Search failed");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_result_metadata() {
    let env = TestEnv::new();

    let doc = Document::new(
        ChunkKind::Message,
        "Test content for metadata".to_string(),
        PathBuf::from("/test/meta.jsonl"),
    )
    .with_project(Some("/Users/test/project".to_string()))
    .with_session_id(Some("session-xyz".to_string()))
    .with_role(Some("assistant".to_string()));

    let index = BM25Index::create(&env.index_dir).expect("Failed to create index");
    let mut writer = index.writer().expect("Failed to create writer");
    index
        .add_documents(&mut writer, &[doc])
        .expect("Failed to add documents");
    writer.commit().expect("Failed to commit");
    index.reload().expect("Failed to reload reader");

    let results = index.search("metadata", 10).expect("Search failed");
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

    let index = BM25Index::create(&env.index_dir).expect("Failed to create index");
    let mut writer = index.writer().expect("Failed to create writer");
    index
        .add_documents(&mut writer, &docs)
        .expect("Failed to add documents");
    writer.commit().expect("Failed to commit");
    index.reload().expect("Failed to reload reader");

    // Search for git
    let results = index.search("git", 10).expect("Search failed");
    assert!(!results.is_empty());
    assert_eq!(results[0].chunk_kind, "tool_use");
    assert_eq!(results[0].tool_name.as_deref(), Some("Bash"));

    // Search for branch
    let results = index.search("branch main", 10).expect("Search failed");
    assert!(!results.is_empty());
    assert_eq!(results[0].chunk_kind, "tool_result");
}
