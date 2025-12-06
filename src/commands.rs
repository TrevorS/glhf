//! CLI command implementations.

use crate::config;
use crate::index::BM25Index;
use crate::ingest;
use anyhow::{Context, Result};
use std::fs;
use std::time::Instant;

/// Builds or rebuilds the search index from all conversation files.
pub fn index(_rebuild: bool) -> Result<()> {
    let index_path = config::bm25_index_dir();

    // Always rebuild until we have proper incremental updates
    if index_path.exists() {
        fs::remove_dir_all(&index_path)?;
    }

    println!("Discovering conversation files...");
    let start = Instant::now();

    // Ingest all documents
    let documents = ingest::ingest_all().context("Failed to ingest documents")?;
    let doc_count = documents.len();

    if doc_count == 0 {
        println!("No documents found to index.");
        return Ok(());
    }

    println!("Found {} documents. Building index...", doc_count);

    // Create fresh index
    let idx = BM25Index::create(&index_path)?;

    // Add documents
    let mut writer = idx.writer()?;
    idx.add_documents(&mut writer, &documents)?;
    writer.commit().context("Failed to commit index")?;

    let elapsed = start.elapsed();

    println!(
        "Indexed {} documents in {:.2}s",
        doc_count,
        elapsed.as_secs_f64()
    );

    // Show index size
    let size = dir_size(&index_path).unwrap_or(0);
    println!("Index size: {}", format_size(size));

    Ok(())
}

/// Searches the index and prints results to stdout.
pub fn search(query: &str, limit: usize) -> Result<()> {
    let index_path = config::bm25_index_dir();

    if !index_path.exists() {
        eprintln!("No index found. Run 'glhf index' first.");
        std::process::exit(1);
    }

    let idx = BM25Index::open(&index_path).context("Failed to open index")?;
    let results = idx.search(query, limit)?;

    if results.is_empty() {
        println!("No matches found for: {}", query);
        return Ok(());
    }

    println!("Found {} results:\n", results.len());

    for (i, result) in results.iter().enumerate() {
        let project_display = result
            .project
            .as_ref()
            .map(|p| {
                // Show just the last path component
                p.rsplit('/').next().unwrap_or(p)
            })
            .unwrap_or("unknown");

        let role_display = result.role.as_deref().unwrap_or("?");

        println!(
            "[{}] {} | {} | {} | Score: {:.2}",
            i + 1,
            result.doc_type,
            project_display,
            role_display,
            result.score
        );

        // Show snippet of content
        let snippet = truncate_content(&result.content, 200);
        println!("    \"{}\"\n", snippet);
    }

    Ok(())
}

/// Prints index status information to stdout.
pub fn status() -> Result<()> {
    let index_path = config::bm25_index_dir();

    if !index_path.exists() {
        println!("No index found.");
        println!("Run 'glhf index' to build the search index.");
        return Ok(());
    }

    let idx = BM25Index::open(&index_path).context("Failed to open index")?;
    let doc_count = idx.num_docs();
    let size = dir_size(&index_path).unwrap_or(0);

    println!("Index Status");
    println!("------------");
    println!("Documents: {}", doc_count);
    println!("Size: {}", format_size(size));
    println!("Location: {}", index_path.display());

    Ok(())
}

/// Calculate directory size in bytes
fn dir_size(path: &std::path::Path) -> Result<u64> {
    let mut size = 0;
    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            size += entry.metadata()?.len();
        }
    }
    Ok(size)
}

/// Format bytes as human-readable size
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Truncate content to max length, breaking at word boundary
fn truncate_content(content: &str, max_len: usize) -> String {
    // Normalize whitespace
    let words: Vec<&str> = content.split_whitespace().collect();
    let normalized = words.join(" ");

    let char_count = normalized.chars().count();
    if char_count <= max_len {
        return normalized;
    }

    // Build up result word by word until we exceed max_len
    let mut result = String::new();
    for word in words {
        let new_len = if result.is_empty() {
            word.chars().count()
        } else {
            result.chars().count() + 1 + word.chars().count()
        };

        if new_len > max_len {
            break;
        }

        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(word);
    }

    if result.is_empty() {
        // Single word too long - just take first max_len chars
        format!(
            "{}...",
            normalized.chars().take(max_len).collect::<String>()
        )
    } else {
        format!("{}...", result)
    }
}
