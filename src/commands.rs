//! CLI command implementations.

use crate::config;
use crate::db::{Database, SearchResult};
use crate::document::{ChunkKind, DisplayLabel};
use crate::embed::Embedder;
use crate::error::Error;
use crate::ingest;
use crate::utils::truncate_text;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::io::Write;
use std::time::Instant;

/// Batch size for embedding generation (optimized for GPU efficiency).
const EMBEDDING_BATCH_SIZE: usize = 2048;

/// Maximum characters for result snippets.
const RESULT_SNIPPET_LEN: usize = 200;

/// Maximum characters for context message snippets.
const CONTEXT_SNIPPET_LEN: usize = 150;

/// Search mode for queries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchMode {
    /// Hybrid search combining FTS5 and vector search.
    #[default]
    Hybrid,
    /// Full-text search only (FTS5).
    Text,
    /// Semantic/vector search only.
    Semantic,
}

/// Options for search command.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Maximum number of results to return.
    pub limit: usize,
    /// Search mode (hybrid, text, semantic).
    pub mode: SearchMode,
    /// Whether to interpret query as a regex pattern.
    pub regex: bool,
    /// Whether to do case-insensitive matching.
    pub ignore_case: bool,
    /// Number of messages to show before each match.
    pub before: usize,
    /// Number of messages to show after each match.
    pub after: usize,
    /// Filter to a specific tool name (e.g., "Bash", "Read").
    pub tool: Option<String>,
    /// Filter to a specific project (substring match, case-insensitive).
    pub project: Option<String>,
    /// Only show error results.
    pub errors: bool,
    /// Only show message chunks (exclude tools).
    pub messages_only: bool,
    /// Only show tool chunks (exclude messages).
    pub tools_only: bool,
    /// Output results as JSON.
    pub json: bool,
    /// Only show results since this timestamp.
    pub since: Option<DateTime<Utc>>,
}

/// Builds or rebuilds the search index from all conversation files.
pub fn index(_rebuild: bool, skip_embeddings: bool) -> Result<()> {
    let db_path = config::database_path()?;

    // Always rebuild for now
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
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

    println!("Found {doc_count} documents. Building database...");

    // Create database and insert documents
    let mut db = Database::open(&db_path)?;
    db.insert_documents(&documents)?;

    let db_time = start.elapsed();
    println!(
        "Indexed {} documents in {:.2}s",
        doc_count,
        db_time.as_secs_f64()
    );

    // Generate embeddings unless skipped
    if skip_embeddings {
        println!("Skipping embeddings (text search only mode).");
    } else {
        println!("\nGenerating embeddings (this may take a while on first run)...");
        let embed_start = Instant::now();

        let embedder = Embedder::new().context("Failed to initialize embedder")?;

        // Collect document contents
        let contents: Vec<String> = documents.iter().map(|d| d.content.clone()).collect();

        // Generate embeddings with progress
        let embeddings = embedder.embed_documents_with_progress(
            &contents,
            EMBEDDING_BATCH_SIZE,
            |done, total| {
                print!("\rEmbedding: {done}/{total} documents");
                std::io::stdout().flush().ok();
            },
        )?;
        println!();

        // Insert embeddings
        let embedding_pairs: Vec<_> = documents
            .iter()
            .zip(embeddings.iter())
            .map(|(d, e)| (d.id.as_str(), e.as_slice()))
            .collect();
        db.insert_embeddings(&embedding_pairs)?;

        let embed_time = embed_start.elapsed();
        println!(
            "Generated {} embeddings in {:.2}s",
            embeddings.len(),
            embed_time.as_secs_f64()
        );
    }

    // Show database size
    let size = db.file_size().unwrap_or(0);
    println!("\nDatabase size: {}", format_size(size));
    println!("Location: {}", db_path.display());

    Ok(())
}

/// Determines the effective search mode, falling back to text if embeddings unavailable.
fn get_effective_mode(db: &Database, mode: SearchMode) -> Result<SearchMode> {
    match mode {
        SearchMode::Hybrid | SearchMode::Semantic => {
            if db.has_embeddings()? {
                Ok(mode)
            } else {
                if mode == SearchMode::Semantic {
                    println!("Warning: No embeddings found. Falling back to text search.");
                    println!(
                        "Run 'glhf index' without --skip-embeddings to enable semantic search.\n"
                    );
                }
                Ok(SearchMode::Text)
            }
        }
        SearchMode::Text => Ok(mode),
    }
}

/// Executes a search with the given mode and returns results.
fn execute_search(
    db: &Database,
    query: &str,
    options: &SearchOptions,
    mode: SearchMode,
    chunk_kind: Option<ChunkKind>,
    has_filters: bool,
    resolved_project: Option<&str>,
) -> Result<Vec<SearchResult>> {
    let results = match mode {
        SearchMode::Text => {
            if has_filters {
                db.search_fts_filtered(
                    query,
                    options.limit,
                    chunk_kind,
                    options.tool.as_deref(),
                    options.errors,
                )?
            } else {
                db.search_fts(query, options.limit)?
            }
        }
        SearchMode::Semantic | SearchMode::Hybrid => {
            let embedder = Embedder::new().context("Failed to initialize embedder")?;
            let query_embedding = embedder.embed_query(query)?;
            let mut results = if mode == SearchMode::Semantic {
                db.search_vector(&query_embedding, options.limit * 2)?
            } else {
                db.search_hybrid(query, &query_embedding, options.limit * 2)?
            };
            if has_filters {
                results.retain(|r| filter_result(r, options, resolved_project));
            }
            results.truncate(options.limit);
            results
        }
    };
    Ok(results)
}

/// Fetches session messages for context display.
fn fetch_session_context(
    db: &Database,
    results: &[SearchResult],
) -> HashMap<String, Vec<SearchResult>> {
    let mut sessions = HashMap::new();
    for result in results {
        if let Some(session_id) = &result.session_id {
            sessions
                .entry(session_id.clone())
                .or_insert_with(|| db.get_session_messages(session_id).unwrap_or_default());
        }
    }
    sessions
}

/// Resolves the project filter, expanding `.` to the current working directory.
fn resolve_project_filter(project: Option<&str>) -> Option<String> {
    project.map(|p| {
        if p == "." {
            std::env::current_dir()
                .ok()
                .and_then(|cwd| cwd.to_str().map(String::from))
                .unwrap_or_else(|| ".".to_string())
        } else {
            p.to_string()
        }
    })
}

/// Searches the database and prints results to stdout.
pub fn search(query: &str, options: &SearchOptions) -> Result<()> {
    let db_path = config::database_path()?;
    if !db_path.exists() {
        return Err(Error::DatabaseNotFound { path: db_path }.into());
    }

    let db = Database::open(&db_path).context("Failed to open database")?;
    let chunk_kind = options.messages_only.then_some(ChunkKind::Message);

    // Resolve `-p .` to current working directory
    let resolved_project = resolve_project_filter(options.project.as_deref());
    let has_filters = options.tool.is_some()
        || options.project.is_some()
        || options.errors
        || options.messages_only
        || options.tools_only
        || options.since.is_some();

    let results = if options.regex {
        let mut results = db.search_regex(query, options.limit * 2, options.ignore_case)?;
        if has_filters {
            results.retain(|r| filter_result(r, options, resolved_project.as_deref()));
        }
        results.truncate(options.limit);
        results
    } else {
        let effective_mode = get_effective_mode(&db, options.mode)?;
        execute_search(
            &db,
            query,
            options,
            effective_mode,
            chunk_kind,
            has_filters,
            resolved_project.as_deref(),
        )?
    };

    // JSON output mode
    if options.json {
        if results.is_empty() {
            println!("[]");
        } else {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        return Ok(());
    }

    // Human-readable output
    if results.is_empty() {
        println!("No matches found for: {query}");
        return Ok(());
    }

    let show_context = options.before > 0 || options.after > 0;
    let session_messages = if show_context {
        fetch_session_context(&db, &results)
    } else {
        HashMap::new()
    };

    println!("Found {} results:\n", results.len());
    for (i, result) in results.iter().enumerate() {
        print_result_header(i + 1, result);
        if show_context {
            print_result_with_context(result, options, &session_messages);
        } else {
            println!(
                "    \"{}\"\n",
                truncate_text(&result.content, RESULT_SNIPPET_LEN)
            );
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
    let time_display = format_relative_time(result.timestamp.as_deref());

    println!(
        "[{}] {} | {} | {} | {} | Score: {:.4}",
        num, result.chunk_kind, project_display, label, time_display, result.score
    );
}

/// Formats a timestamp as relative time (e.g., "2h ago", "3 days ago").
fn format_relative_time(timestamp: Option<&str>) -> String {
    let Some(ts_str) = timestamp else {
        return "unknown".to_string();
    };

    let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) else {
        return "unknown".to_string();
    };

    let now = Utc::now();
    let ts_utc = ts.with_timezone(&Utc);
    let duration = now.signed_duration_since(ts_utc);

    let seconds = duration.num_seconds();
    if seconds < 0 {
        return "future".to_string();
    }

    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();
    let weeks = days / 7;

    if seconds < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{minutes}m ago")
    } else if hours < 24 {
        format!("{hours}h ago")
    } else if days < 7 {
        format!("{days}d ago")
    } else if weeks < 8 {
        format!("{weeks}w ago")
    } else {
        // Show date for older items
        ts_utc.format("%b %d").to_string()
    }
}

/// Prints a search result with context messages.
fn print_result_with_context(
    result: &SearchResult,
    options: &SearchOptions,
    session_messages: &HashMap<String, Vec<SearchResult>>,
) {
    let Some(session_id) = &result.session_id else {
        let snippet = truncate_text(&result.content, RESULT_SNIPPET_LEN);
        println!("    \"{snippet}\"\n");
        return;
    };

    let Some(session_msgs) = session_messages.get(session_id) else {
        let snippet = truncate_text(&result.content, RESULT_SNIPPET_LEN);
        println!("    \"{snippet}\"\n");
        return;
    };

    // Find the position of this result in the session
    let match_pos = session_msgs.iter().position(|m| m.id == result.id);

    let Some(pos) = match_pos else {
        let snippet = truncate_text(&result.content, RESULT_SNIPPET_LEN);
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

        let snippet = truncate_text(&msg.content, CONTEXT_SNIPPET_LEN);
        println!("{prefix} [{label}] \"{snippet}\"");
    }
    println!();
}

/// Prints database status information to stdout.
pub fn status() -> Result<()> {
    let db_path = config::database_path()?;

    if !db_path.exists() {
        println!("No database found.");
        println!("Run 'glhf index' to build the search index.");
        return Ok(());
    }

    let db = Database::open(&db_path).context("Failed to open database")?;
    let doc_count = db.document_count()?;
    let embedding_count = db.embedding_count()?;
    let size = db.file_size().unwrap_or(0);

    println!("Database Status");
    println!("---------------");
    println!("Documents:  {doc_count}");
    println!("Embeddings: {embedding_count}");
    println!("Size:       {}", format_size(size));
    println!("Location:   {}", db_path.display());

    if embedding_count == 0 && doc_count > 0 {
        println!("\nNote: No embeddings found. Run 'glhf index' to enable semantic search.");
    }

    Ok(())
}

/// Views a full conversation session by session ID.
///
/// Supports partial session ID matching. If multiple sessions match,
/// lists them for the user to choose from.
pub fn session(session_id: &str, json: bool) -> Result<()> {
    let db_path = config::database_path()?;
    if !db_path.exists() {
        return Err(Error::DatabaseNotFound { path: db_path }.into());
    }

    let db = Database::open(&db_path).context("Failed to open database")?;

    // Find matching sessions
    let matches = db.find_sessions(session_id)?;

    if matches.is_empty() {
        println!("No sessions found matching: {session_id}");
        return Ok(());
    }

    // If multiple matches, list them
    if matches.len() > 1 {
        println!("Multiple sessions match '{session_id}':\n");
        for (id, count, project) in &matches {
            let project_display = project
                .as_ref()
                .map_or("unknown", |p| p.rsplit('/').next().unwrap_or(p));
            println!("  {id} ({count} items) - {project_display}");
        }
        println!("\nSpecify a more complete session ID.");
        return Ok(());
    }

    // Single match - display the session
    let (full_session_id, _, project) = &matches[0];
    let messages = db.get_session_messages(full_session_id)?;

    if messages.is_empty() {
        println!("Session {full_session_id} has no messages.");
        return Ok(());
    }

    // JSON output
    if json {
        println!("{}", serde_json::to_string_pretty(&messages)?);
        return Ok(());
    }

    // Human-readable output
    let project_display = project
        .as_ref()
        .map_or("unknown", |p| p.rsplit('/').next().unwrap_or(p));
    println!(
        "Session: {} | {} | {} items\n",
        full_session_id,
        project_display,
        messages.len()
    );
    println!("{}", "─".repeat(60));

    for msg in &messages {
        print_session_message(msg);
    }

    Ok(())
}

/// Prints a single message in session view format.
fn print_session_message(msg: &SearchResult) {
    let time = format_relative_time(msg.timestamp.as_deref());
    let label = msg.display_label();

    // Color/style header based on type
    let header = format!("[{}] {} | {}", label, msg.chunk_kind, time);
    println!("\n{header}");
    println!("{}", "─".repeat(40));

    // Print content (truncate very long content)
    let content = if msg.content.len() > 2000 {
        format!(
            "{}...\n[truncated, {} total chars]",
            &msg.content[..2000],
            msg.content.len()
        )
    } else {
        msg.content.clone()
    };
    println!("{content}");
}

/// Filters a search result based on options.
///
/// The `resolved_project` parameter is the project filter after resolving `.`
/// to the current working directory.
fn filter_result(
    result: &SearchResult,
    options: &SearchOptions,
    resolved_project: Option<&str>,
) -> bool {
    // Filter by messages_only
    if options.messages_only && result.chunk_kind != "message" {
        return false;
    }

    // Filter by tools_only
    if options.tools_only && result.chunk_kind == "message" {
        return false;
    }

    // Filter by tool name
    if let Some(ref tool) = options.tool {
        match &result.tool_name {
            Some(name) if name.eq_ignore_ascii_case(tool) => {}
            _ => return false,
        }
    }

    // Filter by project name (case-insensitive substring match)
    if let Some(project_filter) = resolved_project {
        let filter_lower = project_filter.to_lowercase();
        match &result.project {
            Some(project) if project.to_lowercase().contains(&filter_lower) => {}
            _ => return false,
        }
    }

    // Filter by errors
    if options.errors && result.is_error != Some(true) {
        return false;
    }

    // Filter by timestamp (--since)
    if let Some(since) = options.since {
        let ts_ok = result
            .timestamp
            .as_ref()
            .and_then(|ts_str| DateTime::parse_from_rfc3339(ts_str).ok())
            .is_some_and(|ts| ts >= since);
        if !ts_ok {
            return false;
        }
    }

    true
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
