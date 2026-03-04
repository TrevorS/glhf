#![no_main]

use glhf::db::Database;
use glhf::{ChunkKind, Document};
use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

fuzz_target!(|data: &str| {
    // Create an in-memory DB with one doc so FTS table is populated
    let db = Database::open_in_memory().unwrap();
    let doc = Document::new(
        ChunkKind::Message,
        "fuzz seed content for testing".to_string(),
        PathBuf::from("/fuzz/seed.jsonl"),
    );
    db.insert_document(&doc).unwrap();

    // The main invariant: search_fts should never error or panic on any input
    let _ = db.search_fts(data, 10);
});
