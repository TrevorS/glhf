mod common;

use glhf::db::Database;
use glhf::ingest::parse_jsonl_file;
use glhf::{ChunkKind, Document};
use proptest::prelude::*;
use std::io::Write;
use std::path::PathBuf;

extern crate serde_json;

/// Creates a minimal valid JSONL line for a user message.
fn valid_user_jsonl(content: &str, session_id: &str) -> String {
    // Use serde_json to properly escape content and session_id
    let msg = serde_json::json!({
        "type": "user",
        "timestamp": "2025-01-15T10:00:00Z",
        "sessionId": session_id,
        "message": {
            "role": "user",
            "content": content
        }
    });
    serde_json::to_string(&msg).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn proptest_parse_jsonl_random_lines_never_panic(lines in prop::collection::vec("\\PC{0,200}", 1..10)) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            for line in &lines {
                writeln!(f, "{line}").unwrap();
            }
        }

        // Should never panic — always returns Ok (possibly with 0 docs)
        let result = parse_jsonl_file(&path);
        prop_assert!(result.is_ok(), "parse_jsonl_file panicked/errored on random input");
    }

    #[test]
    fn proptest_parse_jsonl_valid_message_produces_one_doc(
        content in "[a-zA-Z0-9]{1}[a-zA-Z0-9 ]{0,49}",
        session in "[a-f0-9]{8}",
    ) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "{}", valid_user_jsonl(&content, &session)).unwrap();
        }

        let docs = parse_jsonl_file(&path).unwrap();
        prop_assert_eq!(docs.len(), 1, "Expected exactly 1 doc, got {}", docs.len());
        prop_assert!(docs[0].content.contains(&content));
    }

    #[test]
    fn proptest_search_fts_arbitrary_query_no_error(query in ".{1,100}") {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("prop_test.db");
        let mut db = Database::open(&db_path).unwrap();
        let doc = Document::new(
            ChunkKind::Message,
            "test content for property testing of search queries".to_string(),
            PathBuf::from("/test/prop.jsonl"),
        );
        db.insert_documents(&[doc]).unwrap();

        let result = db.search_fts(&query, 10);
        prop_assert!(result.is_ok(), "search_fts errored on query: {:?}", query);
    }
}
