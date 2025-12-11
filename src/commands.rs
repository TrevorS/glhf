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

/// Normalizes scores to 0-1 range within the result set.
///
/// Uses min-max normalization so the best result has score 1.0 and
/// the worst has score 0.0. This makes scores comparable within a
/// single search, though not across different searches.
fn normalize_scores(results: &mut [SearchResult]) {
    if results.is_empty() {
        return;
    }

    let min = results
        .iter()
        .map(|r| r.score)
        .fold(f64::INFINITY, f64::min);
    let max = results
        .iter()
        .map(|r| r.score)
        .fold(f64::NEG_INFINITY, f64::max);

    if (max - min).abs() < f64::EPSILON {
        // All same score → all 1.0
        for r in results.iter_mut() {
            r.score = 1.0;
        }
    } else {
        for r in results.iter_mut() {
            r.score = (r.score - min) / (max - min);
        }
    }
}

/// Normalizes scores for ranked sessions (used by `related` command).
fn normalize_ranked_sessions(sessions: &mut [RankedSession]) {
    if sessions.is_empty() {
        return;
    }

    let min = sessions.iter().map(|s| s.1).fold(f64::INFINITY, f64::min);
    let max = sessions
        .iter()
        .map(|s| s.1)
        .fold(f64::NEG_INFINITY, f64::max);

    if (max - min).abs() < f64::EPSILON {
        for s in sessions.iter_mut() {
            s.1 = 1.0;
        }
    } else {
        for s in sessions.iter_mut() {
            s.1 = (s.1 - min) / (max - min);
        }
    }
}

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
    /// Compact output format (one line per result).
    pub compact: bool,
    /// Show session IDs in results.
    pub show_session_id: bool,
    /// Only show results since this timestamp.
    pub since: Option<DateTime<Utc>>,
    /// Show relevance scores in output.
    pub show_scores: bool,
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
    // Determine if we have filters that can be pushed to SQL
    let has_sql_filters = chunk_kind.is_some() || options.tool.is_some() || options.errors;

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

            // Use filtered methods when SQL-pushable filters are active
            let mut results = if has_sql_filters {
                if mode == SearchMode::Semantic {
                    db.search_vector_filtered(
                        &query_embedding,
                        options.limit,
                        chunk_kind,
                        options.tool.as_deref(),
                        options.errors,
                    )?
                } else {
                    db.search_hybrid_filtered(
                        query,
                        &query_embedding,
                        options.limit,
                        chunk_kind,
                        options.tool.as_deref(),
                        options.errors,
                    )?
                }
            } else if mode == SearchMode::Semantic {
                db.search_vector(&query_embedding, options.limit)?
            } else {
                db.search_hybrid(query, &query_embedding, options.limit)?
            };

            // Still need to filter project and since (not in DB methods)
            if options.project.is_some() || options.since.is_some() {
                results.retain(|r| filter_result(r, options, resolved_project));
                results.truncate(options.limit);
            }
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

    let mut results = if options.regex {
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

    // Normalize scores to 0-1 range for consistent display
    normalize_scores(&mut results);

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
        if options.compact {
            print_result_compact(i + 1, result, options.show_scores);
        } else {
            print_result_header(i + 1, result, options.show_session_id, options.show_scores);
            if show_context {
                print_result_with_context(result, options, &session_messages);
            } else {
                println!(
                    "    \"{}\"\n",
                    truncate_text(&result.content, RESULT_SNIPPET_LEN)
                );
            }
        }
    }
    Ok(())
}

/// Prints the header for a search result.
fn print_result_header(
    num: usize,
    result: &SearchResult,
    show_session_id: bool,
    show_scores: bool,
) {
    let project_display = result.project.as_ref().map_or("unknown", |p| {
        // Show just the last path component
        p.rsplit('/').next().unwrap_or(p)
    });

    let label = result.display_label();
    let time_display = format_relative_time(result.timestamp.as_deref());
    let score_display = if show_scores {
        format!(" | Score: {:.2}", result.score)
    } else {
        String::new()
    };

    if show_session_id {
        let session_display = result
            .session_id
            .as_ref()
            .map_or("unknown", |s| &s[..s.len().min(8)]);
        println!(
            "[{}] {} | {} | {} | {}{} | sess:{}",
            num,
            result.chunk_kind,
            project_display,
            label,
            time_display,
            score_display,
            session_display
        );
    } else {
        println!(
            "[{}] {} | {} | {} | {}{}",
            num, result.chunk_kind, project_display, label, time_display, score_display
        );
    }
}

/// Prints a compact single-line search result.
fn print_result_compact(num: usize, result: &SearchResult, show_scores: bool) {
    let project_display = result
        .project
        .as_ref()
        .map_or("unknown", |p| p.rsplit('/').next().unwrap_or(p));

    let label = result.display_label();
    let time_display = format_relative_time(result.timestamp.as_deref());
    let session_display = result
        .session_id
        .as_ref()
        .map_or("--------", |s| &s[..s.len().min(8)]);
    let snippet = truncate_text(&result.content, 60);

    if show_scores {
        println!(
            "[{num}] {:.2} | {project_display} | {label} | {time_display} | {session_display} | \"{snippet}\"",
            result.score
        );
    } else {
        println!(
            "[{num}] {project_display} | {label} | {time_display} | {session_display} | \"{snippet}\""
        );
    }
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

/// Lists all indexed projects with stats.
pub fn projects() -> Result<()> {
    let db_path = config::database_path()?;
    if !db_path.exists() {
        return Err(Error::DatabaseNotFound { path: db_path }.into());
    }

    let db = Database::open(&db_path).context("Failed to open database")?;
    let projects = db.list_projects()?;

    if projects.is_empty() {
        println!("No projects found.");
        return Ok(());
    }

    println!("Projects ({} total)", projects.len());
    println!("{}", "─".repeat(50));

    for (project, doc_count, last_activity) in &projects {
        // Show just the last path component as the display name
        let display_name = project.rsplit('/').next().unwrap_or(project);
        let time_display = format_relative_time(last_activity.as_deref());

        // Pad the name for alignment
        println!(
            "{:<20} {:>6} docs    last: {}",
            truncate_text(display_name, 20),
            doc_count,
            time_display
        );
    }

    Ok(())
}

/// Views a full conversation session by session ID.
///
/// Supports partial session ID matching. If multiple sessions match,
/// lists them for the user to choose from.
pub fn session(session_id: &str, json: bool, limit: Option<usize>, summary: bool) -> Result<()> {
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

    // Summary mode
    if summary {
        print_session_summary(full_session_id, project.as_deref(), &messages);
        return Ok(());
    }

    // JSON output
    if json {
        let output_messages: Vec<_> = if let Some(n) = limit {
            messages.into_iter().take(n).collect()
        } else {
            messages
        };
        println!("{}", serde_json::to_string_pretty(&output_messages)?);
        return Ok(());
    }

    // Human-readable output
    let project_display = project
        .as_ref()
        .map_or("unknown", |p| p.rsplit('/').next().unwrap_or(p));

    let display_count = limit.unwrap_or(messages.len()).min(messages.len());
    let truncated = limit.is_some() && limit.unwrap() < messages.len();

    println!(
        "Session: {} | {} | {} items{}\n",
        full_session_id,
        project_display,
        messages.len(),
        if truncated {
            format!(" (showing first {display_count})")
        } else {
            String::new()
        }
    );
    println!("{}", "─".repeat(60));

    for msg in messages.iter().take(display_count) {
        print_session_message(msg);
    }

    if truncated {
        println!(
            "\n... {} more messages not shown",
            messages.len() - display_count
        );
    }

    Ok(())
}

/// Prints a summary of a session without full content.
fn print_session_summary(session_id: &str, project: Option<&str>, messages: &[SearchResult]) {
    use std::collections::HashMap;

    let project_display = project.map_or("unknown", |p| p.rsplit('/').next().unwrap_or(p));

    // Count by chunk kind
    let mut kind_counts: HashMap<&str, usize> = HashMap::new();
    for msg in messages {
        *kind_counts.entry(msg.chunk_kind.as_str()).or_insert(0) += 1;
    }

    // Count by role (for messages)
    let mut role_counts: HashMap<&str, usize> = HashMap::new();
    for msg in messages.iter().filter(|m| m.chunk_kind == "message") {
        if let Some(role) = &msg.role {
            *role_counts.entry(role.as_str()).or_insert(0) += 1;
        }
    }

    // Count tools used
    let mut tool_counts: HashMap<&str, usize> = HashMap::new();
    for msg in messages.iter().filter(|m| m.chunk_kind == "tool_use") {
        if let Some(tool) = &msg.tool_name {
            *tool_counts.entry(tool.as_str()).or_insert(0) += 1;
        }
    }

    // Calculate duration
    let first_ts = messages.first().and_then(|m| m.timestamp.as_ref());
    let last_ts = messages.last().and_then(|m| m.timestamp.as_ref());
    let duration = match (first_ts, last_ts) {
        (Some(first), Some(last)) => {
            if let (Ok(f), Ok(l)) = (
                chrono::DateTime::parse_from_rfc3339(first),
                chrono::DateTime::parse_from_rfc3339(last),
            ) {
                let dur = l.signed_duration_since(f);
                format_duration(dur)
            } else {
                "unknown".to_string()
            }
        }
        _ => "unknown".to_string(),
    };

    let started = format_relative_time(first_ts.map(std::string::String::as_str));

    println!("Session: {session_id}");
    println!("Project: {project_display}");
    println!("Duration: {duration} (started {started})");
    println!("Messages: {} total", messages.len());

    // Role breakdown
    if !role_counts.is_empty() {
        let mut roles: Vec<_> = role_counts.into_iter().collect();
        roles.sort_by(|a, b| b.1.cmp(&a.1));
        for (role, count) in roles {
            println!("  - {role}: {count}");
        }
    }

    // Tool use breakdown
    if let Some(&tool_use_count) = kind_counts.get("tool_use") {
        println!("  - tool calls: {tool_use_count}");
    }
    if let Some(&tool_result_count) = kind_counts.get("tool_result") {
        println!("  - tool results: {tool_result_count}");
    }

    // Top tools
    if !tool_counts.is_empty() {
        let mut tools: Vec<_> = tool_counts.into_iter().collect();
        tools.sort_by(|a, b| b.1.cmp(&a.1));
        let top_tools: Vec<_> = tools
            .iter()
            .take(5)
            .map(|(t, c)| format!("{t} ({c})"))
            .collect();
        println!("Tools used: {}", top_tools.join(", "));
    }
}

/// Formats a duration in a human-readable way.
fn format_duration(dur: chrono::Duration) -> String {
    let total_secs = dur.num_seconds();
    if total_secs < 60 {
        format!("{total_secs}s")
    } else if total_secs < 3600 {
        format!("{}m", total_secs / 60)
    } else {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        if mins > 0 {
            format!("{hours}h {mins}m")
        } else {
            format!("{hours}h")
        }
    }
}

/// Ranked session info tuple for related session results.
type RankedSession = (String, f64, Option<String>, Option<String>, String);

/// Aggregated session score during related session computation.
type SessionScoreEntry = (f64, usize, Option<String>, Option<String>, String);

/// Finds sessions related to a given session using embedding similarity.
pub fn related(session_id: &str, limit: usize) -> Result<()> {
    use crate::db::EMBEDDING_DIM;

    let db_path = config::database_path()?;
    if !db_path.exists() {
        return Err(Error::DatabaseNotFound { path: db_path }.into());
    }

    let db = Database::open(&db_path).context("Failed to open database")?;

    if !db.has_embeddings()? {
        println!("No embeddings found. Run 'glhf index' to enable semantic search.");
        return Ok(());
    }

    // Find and validate session
    let Some((full_session_id, project)) = resolve_session(&db, session_id)? else {
        return Ok(());
    };

    let project_display = project_name(project.as_deref());
    println!("Finding sessions related to: {full_session_id} ({project_display})\n");

    // Get session embedding
    let Some(avg_embedding) = compute_session_embedding(&db, &full_session_id, EMBEDDING_DIM)?
    else {
        return Ok(());
    };

    // Find and rank related sessions
    let mut ranked = find_related_sessions(&db, &avg_embedding, &full_session_id, limit)?;
    if ranked.is_empty() {
        println!("No related sessions found.");
        return Ok(());
    }

    // Normalize scores to 0-1 range
    normalize_ranked_sessions(&mut ranked);

    print_related_sessions(&ranked);
    Ok(())
}

/// Resolves a partial session ID to a full session ID and project.
fn resolve_session(db: &Database, session_id: &str) -> Result<Option<(String, Option<String>)>> {
    let matches = db.find_sessions(session_id)?;

    if matches.is_empty() {
        println!("No sessions found matching: {session_id}");
        return Ok(None);
    }

    if matches.len() > 1 {
        println!("Multiple sessions match '{session_id}':\n");
        for (id, count, project) in &matches {
            println!(
                "  {id} ({count} items) - {}",
                project_name(project.as_deref())
            );
        }
        println!("\nSpecify a more complete session ID.");
        return Ok(None);
    }

    let (full_id, _, project) = matches.into_iter().next().unwrap();
    Ok(Some((full_id, project)))
}

/// Computes an averaged embedding for a session by sampling its documents.
fn compute_session_embedding(
    db: &Database,
    session_id: &str,
    dim: usize,
) -> Result<Option<Vec<f32>>> {
    let doc_ids = db.get_session_doc_ids(session_id)?;
    if doc_ids.is_empty() {
        println!("Session has no documents.");
        return Ok(None);
    }

    // Sample if too many docs
    let sample_ids: Vec<String> = if doc_ids.len() > 100 {
        let step = doc_ids.len() / 100;
        doc_ids.into_iter().step_by(step).take(100).collect()
    } else {
        doc_ids
    };

    let embeddings = db.get_embeddings_for_docs(&sample_ids)?;
    if embeddings.is_empty() {
        println!("No embeddings found for this session.");
        return Ok(None);
    }

    Ok(Some(average_embeddings(&embeddings, dim)))
}

/// Finds sessions related to the given embedding, excluding the source session.
fn find_related_sessions(
    db: &Database,
    embedding: &[f32],
    exclude_session: &str,
    limit: usize,
) -> Result<Vec<RankedSession>> {
    use std::collections::HashMap;

    let similar_docs =
        db.search_vector_excluding_session(embedding, exclude_session, limit * 20)?;
    if similar_docs.is_empty() {
        return Ok(vec![]);
    }

    // Aggregate scores by session
    let mut scores: HashMap<String, SessionScoreEntry> = HashMap::new();

    for doc in &similar_docs {
        if let Some(sess_id) = &doc.session_id {
            let entry = scores.entry(sess_id.clone()).or_insert((
                0.0,
                0,
                doc.project.clone(),
                doc.timestamp.clone(),
                doc.content.clone(),
            ));
            entry.0 += doc.score;
            entry.1 += 1;
        }
    }

    // Convert to ranked list
    let mut ranked: Vec<RankedSession> = scores
        .into_iter()
        .map(|(id, (total, count, proj, ts, content))| {
            (id, total / count as f64, proj, ts, content)
        })
        .collect();

    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.truncate(limit);
    Ok(ranked)
}

/// Prints the list of related sessions.
fn print_related_sessions(sessions: &[RankedSession]) {
    println!("Related sessions:\n");
    for (i, (sess_id, score, proj, timestamp, sample)) in sessions.iter().enumerate() {
        let proj_display = project_name(proj.as_deref());
        let time_display = format_relative_time(timestamp.as_deref());
        let snippet = truncate_text(sample, 60);

        println!(
            "[{}] {} | {} | {} | Score: {:.2}",
            i + 1,
            &sess_id[..sess_id.len().min(8)],
            proj_display,
            time_display,
            score
        );
        println!("    \"{snippet}\"\n");
    }
}

/// Extracts just the project name from a full path.
fn project_name(project: Option<&str>) -> &str {
    project.map_or("unknown", |p| p.rsplit('/').next().unwrap_or(p))
}

/// Averages a list of embeddings into a single embedding.
fn average_embeddings(embeddings: &[Vec<f32>], dim: usize) -> Vec<f32> {
    let mut avg = vec![0.0_f32; dim];
    let count = embeddings.len() as f32;

    for embedding in embeddings {
        for (i, val) in embedding.iter().enumerate() {
            if i < dim {
                avg[i] += val;
            }
        }
    }

    for val in &mut avg {
        *val /= count;
    }

    avg
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
