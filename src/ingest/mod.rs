mod conversation;

pub use conversation::parse_jsonl_file;

use crate::config;
use crate::Document;
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Discovers all conversation JSONL files in ~/.claude/projects
pub fn discover_conversation_files() -> Result<Vec<PathBuf>> {
    let projects_dir = config::projects_dir();

    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();

    for entry in WalkDir::new(&projects_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "jsonl") {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

/// Ingests all conversation files and returns Documents
pub fn ingest_all() -> Result<Vec<Document>> {
    let files = discover_conversation_files()?;
    let mut all_docs = Vec::new();

    for file_path in files {
        match parse_jsonl_file(&file_path) {
            Ok(docs) => {
                all_docs.extend(docs);
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", file_path.display(), e);
            }
        }
    }

    Ok(all_docs)
}

/// Extracts the project name from a JSONL file path
pub fn extract_project_from_path(path: &Path) -> Option<String> {
    // Path format: ~/.claude/projects/-Users-trevor-Projects-foo/session.jsonl
    let projects_dir = config::projects_dir();

    path.strip_prefix(&projects_dir)
        .ok()
        .and_then(|rel| rel.components().next())
        .map(|comp| {
            let encoded = comp.as_os_str().to_string_lossy();
            config::decode_project_path(&encoded)
        })
}
