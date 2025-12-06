//! CLI command implementations.

use crate::config;
use crate::error::Error;
use crate::index::{BM25Index, SearchResult};
use crate::ingest;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Options for search command.
#[derive(Debug, Clone, Copy, Default)]
pub struct SearchOptions {
    /// Maximum number of results to return.
    pub limit: usize,
    /// Whether to interpret query as a regex pattern.
    pub regex: bool,
    /// Whether to do case-insensitive matching.
    pub ignore_case: bool,
    /// Number of messages to show before each match.
    pub before: usize,
    /// Number of messages to show after each match.
    pub after: usize,
}

/// Builds or rebuilds the search index from all conversation files.
pub fn index(_rebuild: bool) -> Result<()> {
    let index_path = config::bm25_index_dir()?;

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

    println!("Found {doc_count} documents. Building index...");

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
pub fn search(query: &str, options: SearchOptions) -> Result<()> {
    let index_path = config::bm25_index_dir()?;

    if !index_path.exists() {
        return Err(Error::IndexNotFound { path: index_path }.into());
    }

    let idx = BM25Index::open(&index_path).context("Failed to open index")?;

    let results = if options.regex {
        idx.search_regex(query, options.limit, options.ignore_case)?
    } else {
        idx.search(query, options.limit)?
    };

    if results.is_empty() {
        println!("No matches found for: {query}");
        return Ok(());
    }

    let show_context = options.before > 0 || options.after > 0;

    // If we need context, fetch session messages
    let session_messages: HashMap<String, Vec<SearchResult>> = if show_context {
        let mut sessions = HashMap::new();
        for result in &results {
            if let Some(session_id) = &result.session_id {
                if !sessions.contains_key(session_id) {
                    if let Ok(msgs) = idx.get_session_messages(session_id) {
                        sessions.insert(session_id.clone(), msgs);
                    }
                }
            }
        }
        sessions
    } else {
        HashMap::new()
    };

    println!("Found {} results:\n", results.len());

    for (i, result) in results.iter().enumerate() {
        print_result_header(i + 1, result);

        if show_context {
            print_result_with_context(result, &options, &session_messages);
        } else {
            // Show snippet of content
            let snippet = truncate_content(&result.content, 200);
            println!("    \"{snippet}\"\n");
        }
    }

    Ok(())
}

/// Prints the header for a search result.
fn print_result_header(num: usize, result: &SearchResult) {
    let project_display = result.project.as_ref().map_or("unknown", |p| {
        // Show just the last path component
        p.rsplit('/').next().unwrap_or(p)
    });

    let label = result.display_label();

    println!(
        "[{}] {} | {} | {} | Score: {:.2}",
        num, result.chunk_kind, project_display, label, result.score
    );
}

/// Prints a search result with context messages.
fn print_result_with_context(
    result: &SearchResult,
    options: &SearchOptions,
    session_messages: &HashMap<String, Vec<SearchResult>>,
) {
    let Some(session_id) = &result.session_id else {
        // No session, just show the match
        let snippet = truncate_content(&result.content, 200);
        println!("    \"{snippet}\"\n");
        return;
    };

    let Some(session_msgs) = session_messages.get(session_id) else {
        // Couldn't find session messages, just show the match
        let snippet = truncate_content(&result.content, 200);
        println!("    \"{snippet}\"\n");
        return;
    };

    // Find the position of this result in the session
    let match_pos = session_msgs.iter().position(|m| m.id == result.id);

    let Some(pos) = match_pos else {
        // Couldn't find match in session, just show the match
        let snippet = truncate_content(&result.content, 200);
        println!("    \"{snippet}\"\n");
        return;
    };

    // Calculate context range
    let start = pos.saturating_sub(options.before);
    let end = (pos + 1 + options.after).min(session_msgs.len());

    // Print context messages
    for (idx, msg) in session_msgs[start..end].iter().enumerate() {
        let absolute_idx = start + idx;
        let is_match = absolute_idx == pos;
        let prefix = if is_match { ">>>" } else { "   " };
        let label = msg.display_label();

        let snippet = truncate_content(&msg.content, 150);
        println!("{prefix} [{label}] \"{snippet}\"");
    }
    println!();
}

/// Prints index status information to stdout.
pub fn status() -> Result<()> {
    let index_path = config::bm25_index_dir()?;

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
    println!("Documents: {doc_count}");
    println!("Size: {}", format_size(size));
    println!("Location: {}", index_path.display());

    Ok(())
}

/// Calculate directory size in bytes.
fn dir_size(path: &Path) -> Result<u64> {
    let mut size = 0;
    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if entry.file_type().is_file() {
            size += entry.metadata()?.len();
        }
    }
    Ok(size)
}

/// Format bytes as human-readable size.
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
        format!("{bytes} B")
    }
}

/// Truncate content to max length, breaking at word boundary.
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
        format!("{result}...")
    }
}
