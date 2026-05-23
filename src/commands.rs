//! CLI command implementations.

use crate::config;
use crate::db::{Database, SearchResult};
use crate::document::ChunkKind;
use crate::embed::Embedder;
use crate::error::Error;
use crate::filter;
use crate::format::{
    format_date, format_number, format_relative_time, format_seconds_ago, format_size,
    print_result_compact, print_result_header, print_result_with_context, print_session_message,
    print_session_summary, project_name, RESULT_SNIPPET_LEN,
};
use crate::ingest;
use crate::utils::truncate_text;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::time::Instant;

/// Batch size for embedding generation.
const EMBEDDING_BATCH_SIZE: usize = 2048;

fn open_db() -> Result<Database> {
    let db_path = config::database_path()?;
    if !db_path.exists() {
        return Err(Error::DatabaseNotFound { path: db_path }.into());
    }
    Database::open(&db_path).context("Failed to open database")
}

/// Normalizes scores to 0-1 range within the result set.
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
        for r in results.iter_mut() {
            r.score = 1.0;
        }
    } else {
        for r in results.iter_mut() {
            r.score = (r.score - min) / (max - min);
        }
    }
}

fn normalize_ranked_sessions(sessions: &mut [RankedSession]) {
    if sessions.is_empty() {
        return;
    }

    let min = sessions
        .iter()
        .map(|s| s.score)
        .fold(f64::INFINITY, f64::min);
    let max = sessions
        .iter()
        .map(|s| s.score)
        .fold(f64::NEG_INFINITY, f64::max);

    if (max - min).abs() < f64::EPSILON {
        for s in sessions.iter_mut() {
            s.score = 1.0;
        }
    } else {
        for s in sessions.iter_mut() {
            s.score = (s.score - min) / (max - min);
        }
    }
}

/// Search mode for queries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchMode {
    #[default]
    Hybrid,
    Text,
    Semantic,
}

/// Options for search command.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub limit: usize,
    pub mode: SearchMode,
    pub regex: bool,
    pub ignore_case: bool,
    pub before: usize,
    pub after: usize,
    pub tool: Option<String>,
    pub project: Option<String>,
    pub errors: bool,
    pub messages_only: bool,
    pub tools_only: bool,
    pub json: bool,
    pub compact: bool,
    pub show_session_id: bool,
    pub since: Option<DateTime<Utc>>,
    pub show_scores: bool,
    pub exclude_projects: Vec<String>,
    pub exclude_this_project: bool,
    pub include_this_project: bool,
    pub exclude_this_session: bool,
    pub include_this_session: bool,
    pub this_session: bool,
    pub oldest_first: bool,
}

impl SearchOptions {
    pub fn has_filters(&self) -> bool {
        self.tool.is_some()
            || self.project.is_some()
            || self.errors
            || self.messages_only
            || self.tools_only
            || self.since.is_some()
            || !self.exclude_projects.is_empty()
            || self.exclude_this_project
            || self.exclude_this_session
            || self.this_session
    }
}

/// Builds or rebuilds the search index from all conversation files.
pub fn index(skip_embeddings: bool, full_rebuild: bool) -> Result<()> {
    let db_path = config::database_path()?;

    if full_rebuild && db_path.exists() {
        std::fs::remove_file(&db_path)?;
        println!("Full rebuild requested. Deleted existing database.");
    }

    let is_incremental = db_path.exists();

    println!("Discovering conversation files...");
    let start = Instant::now();

    let files_with_mtimes = ingest::discover_with_mtimes().context("Failed to discover files")?;
    let total_files = files_with_mtimes.len();

    if total_files == 0 {
        println!("No conversation files found.");
        return Ok(());
    }

    let mut db = Database::open(&db_path)?;

    let indexed_files = if is_incremental {
        db.get_indexed_files()?
    } else {
        HashMap::new()
    };

    let mut new_files = Vec::new();
    let mut modified_files = Vec::new();
    let mut unchanged_count = 0usize;
    let mut seen_paths = HashSet::new();

    for (path, mtime) in &files_with_mtimes {
        let path_str = path.to_string_lossy().to_string();
        seen_paths.insert(path_str.clone());
        match indexed_files.get(&path_str) {
            Some(&stored_mtime) if stored_mtime == *mtime => {
                unchanged_count += 1;
            }
            Some(_) => modified_files.push((path.clone(), *mtime)),
            None => new_files.push((path.clone(), *mtime)),
        }
    }

    let deleted_files: Vec<_> = indexed_files
        .keys()
        .filter(|p| !seen_paths.contains(*p))
        .cloned()
        .collect();

    let changes = new_files.len() + modified_files.len() + deleted_files.len();

    if is_incremental && changes == 0 {
        println!("Index is up to date ({total_files} files, {unchanged_count} unchanged).");
        if !skip_embeddings {
            embed_missing(&mut db)?;
        }
        let size = db.file_size().unwrap_or(0);
        println!("Database size: {}", format_size(size));
        return Ok(());
    }

    if is_incremental {
        println!(
            "Incremental update: {} new, {} modified, {} deleted, {} unchanged",
            new_files.len(),
            modified_files.len(),
            deleted_files.len(),
            unchanged_count
        );
    } else {
        println!("Building new index from {total_files} files...");
    }

    db.begin_transaction()?;

    for path_str in &deleted_files {
        db.delete_documents_by_source(path_str)?;
    }

    let files_to_process: Vec<_> = modified_files.iter().chain(new_files.iter()).collect();
    let total = files_to_process.len();
    for (i, (path, mtime)) in files_to_process.iter().enumerate() {
        let path_str = path.to_string_lossy().to_string();
        db.delete_documents_by_source(&path_str)?;
        match ingest::parse_jsonl_file(path) {
            Ok(docs) => {
                let count = docs.len();
                db.insert_documents(&docs)?;
                db.upsert_file_meta(&path_str, *mtime, count)?;
            }
            Err(e) => eprintln!("Warning: Failed to parse {}: {e}", path.display()),
        }
        if (i + 1) % 100 == 0 || i + 1 == total {
            eprint!("\rProcessing files: {}/{total}", i + 1);
        }
    }
    eprintln!();

    db.commit_transaction()?;

    let db_time = start.elapsed();
    let total_docs = db.document_count()?;
    println!(
        "Indexed {} documents in {:.2}s",
        total_docs,
        db_time.as_secs_f64()
    );

    if skip_embeddings {
        println!("Skipping embeddings (text search only mode).");
    } else {
        embed_missing(&mut db)?;
    }

    let size = db.file_size().unwrap_or(0);
    println!("\nDatabase size: {}", format_size(size));
    println!("Location: {}", db_path.display());

    Ok(())
}

fn embed_missing(db: &mut Database) -> Result<()> {
    let missing = db.documents_without_embeddings()?;
    if missing.is_empty() {
        return Ok(());
    }

    println!("\nGenerating embeddings for {} documents...", missing.len());
    let embed_start = Instant::now();

    let embedder = Embedder::new().context("Failed to initialize embedder")?;

    let contents: Vec<String> = missing.iter().map(|(_, content)| content.clone()).collect();

    let embeddings = embedder.embed_documents_with_progress(
        &contents,
        EMBEDDING_BATCH_SIZE,
        |done, total| {
            print!("\rEmbedding: {done}/{total} documents");
            std::io::stdout().flush().ok();
        },
    )?;
    println!();

    let mut seen = HashSet::new();
    let embedding_pairs: Vec<_> = missing
        .iter()
        .zip(embeddings.iter())
        .filter(|((id, _), _)| seen.insert(id.clone()))
        .map(|((id, _), e)| (id.as_str(), e.as_slice()))
        .collect();
    db.insert_embeddings(&embedding_pairs)?;

    let embed_time = embed_start.elapsed();
    println!(
        "Generated {} embeddings in {:.2}s",
        embedding_pairs.len(),
        embed_time.as_secs_f64()
    );

    Ok(())
}

fn check_index_freshness(db: &Database) -> Result<()> {
    let indexed_files = db.get_indexed_files()?;
    if indexed_files.is_empty() {
        return Ok(());
    }

    let files_with_mtimes = ingest::discover_with_mtimes().unwrap_or_default();
    let mut seen = HashSet::new();
    let mut new_count = 0usize;
    let mut modified_count = 0usize;

    for (path, mtime) in &files_with_mtimes {
        let path_str = path.to_string_lossy().to_string();
        seen.insert(path_str.clone());
        match indexed_files.get(&path_str) {
            Some(&stored_mtime) if stored_mtime == *mtime => {}
            Some(_) => modified_count += 1,
            None => new_count += 1,
        }
    }

    let deleted_count = indexed_files.keys().filter(|p| !seen.contains(*p)).count();
    let total_stale = new_count + modified_count + deleted_count;

    if total_stale > 0 {
        let last_indexed = indexed_files.values().max().copied().unwrap_or(0);
        let ago = format_seconds_ago(last_indexed);
        eprintln!(
            "Note: Index is {total_stale} files behind (last indexed {ago}). \
             Run `glhf index` to update or `glhf index --full` to rebuild."
        );
    }

    Ok(())
}

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

#[allow(clippy::too_many_arguments)]
fn execute_search(
    db: &Database,
    query: &str,
    options: &SearchOptions,
    mode: SearchMode,
    chunk_kind: Option<ChunkKind>,
    has_filters: bool,
    resolved_project: Option<&str>,
    current_project: Option<&str>,
    current_session: Option<&str>,
) -> Result<Vec<SearchResult>> {
    let has_sql_filters = chunk_kind.is_some() || options.tool.is_some() || options.errors;
    let fetch_limit = options.limit * 2;

    let mut results = match mode {
        SearchMode::Text => {
            if has_sql_filters {
                db.search_fts_filtered(
                    query,
                    fetch_limit,
                    chunk_kind,
                    options.tool.as_deref(),
                    options.errors,
                )?
            } else {
                db.search_fts(query, fetch_limit)?
            }
        }
        SearchMode::Semantic | SearchMode::Hybrid => {
            let embedder = Embedder::new().context("Failed to initialize embedder")?;
            let query_embedding = embedder.embed_query(query)?;

            if has_sql_filters {
                if mode == SearchMode::Semantic {
                    db.search_vector_filtered(
                        &query_embedding,
                        fetch_limit,
                        chunk_kind,
                        options.tool.as_deref(),
                        options.errors,
                    )?
                } else {
                    db.search_hybrid_filtered(
                        query,
                        &query_embedding,
                        fetch_limit,
                        chunk_kind,
                        options.tool.as_deref(),
                        options.errors,
                    )?
                }
            } else if mode == SearchMode::Semantic {
                db.search_vector(&query_embedding, fetch_limit)?
            } else {
                db.search_hybrid(query, &query_embedding, fetch_limit)?
            }
        }
    };

    if has_filters {
        results.retain(|r| {
            filter::filter_result(
                r,
                options,
                resolved_project,
                current_project,
                current_session,
            )
        });
    }
    results.truncate(options.limit);

    Ok(results)
}

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

/// Searches the database and prints results to stdout.
#[allow(clippy::too_many_lines)]
pub fn search(query: &str, options: &SearchOptions) -> Result<()> {
    if query.trim().is_empty() {
        anyhow::bail!("Search query cannot be empty");
    }

    let db = open_db()?;
    check_index_freshness(&db)?;

    let chunk_kind = options.messages_only.then_some(ChunkKind::Message);
    let in_claude_code = std::env::var("CLAUDECODE").is_ok();

    let mut options = options.clone();
    let mut showed_exclusion_hint = false;
    if in_claude_code && !options.include_this_project && !options.include_this_session {
        if !options.exclude_this_project && !options.include_this_project {
            options.exclude_this_project = true;
            showed_exclusion_hint = true;
        }
        if !options.exclude_this_session && !options.include_this_session && !options.this_session {
            options.exclude_this_session = true;
        }
    }

    let current_project = if options.exclude_this_project {
        filter::current_project_name()
    } else {
        None
    };
    let current_session = if options.exclude_this_session || options.this_session {
        filter::detect_current_session()
    } else {
        None
    };

    let resolved_project = filter::resolve_project_filter(options.project.as_deref());
    let has_filters = options.has_filters();

    let mut results = if options.regex {
        let mut results = db.search_regex(query, options.limit * 2, options.ignore_case)?;
        if has_filters {
            results.retain(|r| {
                filter::filter_result(
                    r,
                    &options,
                    resolved_project.as_deref(),
                    current_project.as_deref(),
                    current_session.as_deref(),
                )
            });
        }
        results.truncate(options.limit);
        results
    } else {
        let effective_mode = get_effective_mode(&db, options.mode)?;
        execute_search(
            &db,
            query,
            &options,
            effective_mode,
            chunk_kind,
            has_filters,
            resolved_project.as_deref(),
            current_project.as_deref(),
            current_session.as_deref(),
        )?
    };

    if options.oldest_first {
        results.reverse();
    }

    normalize_scores(&mut results);

    if options.json {
        if results.is_empty() {
            println!("[]");
        } else {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        return Ok(());
    }

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
                print_result_with_context(result, options.before, options.after, &session_messages);
            } else {
                println!(
                    "    \"{}\"\n",
                    truncate_text(&result.content, RESULT_SNIPPET_LEN)
                );
            }
        }
    }

    if showed_exclusion_hint {
        println!("\nTip: Results from this project/session are auto-excluded.");
        println!("Use --include-this-project or --include-this-session to include them.");
    }
    Ok(())
}

/// Prints database status information to stdout.
#[allow(clippy::cast_possible_wrap)]
pub fn status() -> Result<()> {
    let db_path = config::database_path()?;

    if !db_path.exists() {
        println!("No database found.");
        println!("Run 'glhf index' to build the search index.");
        return Ok(());
    }

    let db = open_db()?;
    let doc_count = db.document_count()?;
    let embedding_count = db.embedding_count()?;
    let size = db.file_size().unwrap_or(0);
    let stats = db.status_stats()?;

    println!("Database Status");
    println!("{}", "─".repeat(15));
    println!("Location:    {}", db_path.display());
    println!("Size:        {}", format_size(size));
    println!(
        "Documents:   {} ({} with embeddings)",
        format_number(doc_count as i64),
        format_number(embedding_count as i64),
    );

    if embedding_count == 0 && doc_count > 0 {
        println!("\n  Note: No embeddings found. Run 'glhf index' to enable semantic search.");
    }

    println!("\nSessions & Projects");
    println!("{}", "─".repeat(19));
    println!("Sessions:    {}", format_number(stats.session_count));
    println!("Projects:    {}", format_number(stats.project_count));

    if !stats.top_projects.is_empty() {
        println!("\nMost active projects:");
        for (project, doc_cnt, sess_cnt) in &stats.top_projects {
            let name = project_name(Some(project.as_str()));
            println!(
                "  {:<20} {:>6} docs   {:>4} sessions",
                truncate_text(name, 20),
                format_number(*doc_cnt),
                format_number(*sess_cnt),
            );
        }
    }

    println!("\nContent Breakdown");
    println!("{}", "─".repeat(17));

    let message_count = stats.chunk_counts.get("message").copied().unwrap_or(0);
    let user_count = stats.role_counts.get("user").copied().unwrap_or(0);
    let assistant_count = stats.role_counts.get("assistant").copied().unwrap_or(0);
    let tool_use_count = stats.chunk_counts.get("tool_use").copied().unwrap_or(0);

    println!(
        "Messages:    {} ({} user / {} assistant)",
        format_number(message_count),
        format_number(user_count),
        format_number(assistant_count),
    );
    println!("Tool calls:  {}", format_number(tool_use_count));
    println!(
        "Tool results: {} ({} errors)",
        format_number(stats.tool_result_count),
        format_number(stats.error_count),
    );

    if !stats.tool_counts.is_empty() {
        println!("\nTop tools:");
        for (tool, count) in &stats.tool_counts {
            println!("  {:<14} {:>6}", tool, format_number(*count));
        }
    }

    if stats.earliest_timestamp.is_some() || stats.latest_timestamp.is_some() {
        println!("\nTimeline");
        println!("{}", "─".repeat(8));

        if let Some(ref ts) = stats.earliest_timestamp {
            println!(
                "First indexed: {} ({})",
                format_date(Some(ts.as_str())),
                format_relative_time(Some(ts.as_str())),
            );
        }
        if let Some(ref ts) = stats.latest_timestamp {
            println!(
                "Last indexed:  {} ({})",
                format_date(Some(ts.as_str())),
                format_relative_time(Some(ts.as_str())),
            );
        }
    }

    Ok(())
}

/// Lists all indexed projects with stats.
pub fn projects() -> Result<()> {
    let db = open_db()?;
    let projects = db.list_projects()?;

    if projects.is_empty() {
        println!("No projects found.");
        return Ok(());
    }

    println!("Projects ({} total)", projects.len());
    println!("{}", "─".repeat(50));

    for (project, doc_count, last_activity) in &projects {
        let display_name = project_name(Some(project.as_str()));
        let time_display = format_relative_time(last_activity.as_deref());

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
pub fn session(session_id: &str, json: bool, limit: Option<usize>, summary: bool) -> Result<()> {
    let db = open_db()?;
    let matches = db.find_sessions(session_id)?;

    if matches.is_empty() {
        println!("No sessions found matching: {session_id}");
        return Ok(());
    }

    if matches.len() > 1 {
        println!("Multiple sessions match '{session_id}':\n");
        for (id, count, project) in &matches {
            let project_display = project_name(project.as_deref());
            println!("  {id} ({count} items) - {project_display}");
        }
        println!("\nSpecify a more complete session ID.");
        return Ok(());
    }

    let (full_session_id, _, project) = &matches[0];
    let messages = db.get_session_messages(full_session_id)?;

    if messages.is_empty() {
        println!("Session {full_session_id} has no messages.");
        return Ok(());
    }

    if summary {
        print_session_summary(full_session_id, project.as_deref(), &messages);
        return Ok(());
    }

    if json {
        let output_messages: Vec<_> = if let Some(n) = limit {
            messages.into_iter().take(n).collect()
        } else {
            messages
        };
        println!("{}", serde_json::to_string_pretty(&output_messages)?);
        return Ok(());
    }

    let project_display = project_name(project.as_deref());

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

struct RankedSession {
    session_id: String,
    score: f64,
    project: Option<String>,
    timestamp: Option<String>,
    sample_content: String,
}

struct SessionScoreAgg {
    total_score: f64,
    doc_count: usize,
    project: Option<String>,
    timestamp: Option<String>,
    sample_content: String,
}

const RELATED_MIN_RAW_SCORE: f64 = 0.1;
const RELATED_MIN_NORMALIZED_SCORE: f64 = 0.05;

/// Finds sessions related to a given session using embedding similarity.
pub fn related(session_id: &str, limit: usize) -> Result<()> {
    use crate::db::EMBEDDING_DIM;

    let db = open_db()?;

    if !db.has_embeddings()? {
        println!("No embeddings found. Run 'glhf index' to enable semantic search.");
        return Ok(());
    }

    let Some((full_session_id, project)) = resolve_session(&db, session_id)? else {
        return Ok(());
    };

    let project_display = project_name(project.as_deref());
    println!("Finding sessions related to: {full_session_id} ({project_display})\n");

    let Some(avg_embedding) = compute_session_embedding(&db, &full_session_id, EMBEDDING_DIM)?
    else {
        return Ok(());
    };

    let mut ranked = find_related_sessions(&db, &avg_embedding, &full_session_id, limit * 2)?;

    ranked.retain(|s| s.score >= RELATED_MIN_RAW_SCORE);

    if ranked.is_empty() {
        println!("No related sessions found.");
        return Ok(());
    }

    normalize_ranked_sessions(&mut ranked);

    ranked.retain(|s| s.score >= RELATED_MIN_NORMALIZED_SCORE);
    ranked.truncate(limit);

    if ranked.is_empty() {
        println!("No related sessions found.");
        return Ok(());
    }

    println!("Related sessions:\n");
    for (i, s) in ranked.iter().enumerate() {
        let proj_display = project_name(s.project.as_deref());
        let time_display = format_relative_time(s.timestamp.as_deref());
        let snippet = truncate_text(&s.sample_content, 60);

        println!(
            "[{}] {} | {} | {} | Score: {:.2}",
            i + 1,
            &s.session_id[..s.session_id.len().min(8)],
            proj_display,
            time_display,
            s.score
        );
        println!("    \"{snippet}\"\n");
    }
    Ok(())
}

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

fn find_related_sessions(
    db: &Database,
    embedding: &[f32],
    exclude_session: &str,
    limit: usize,
) -> Result<Vec<RankedSession>> {
    let similar_docs =
        db.search_vector_excluding_session(embedding, exclude_session, limit * 20)?;
    if similar_docs.is_empty() {
        return Ok(vec![]);
    }

    let mut scores: HashMap<String, SessionScoreAgg> = HashMap::new();

    for doc in &similar_docs {
        if let Some(sess_id) = &doc.session_id {
            let entry = scores.entry(sess_id.clone()).or_insert(SessionScoreAgg {
                total_score: 0.0,
                doc_count: 0,
                project: doc.project.clone(),
                timestamp: doc.timestamp.clone(),
                sample_content: doc.content.clone(),
            });
            entry.total_score += doc.score;
            entry.doc_count += 1;
        }
    }

    let mut ranked: Vec<RankedSession> = scores
        .into_iter()
        .map(|(id, agg)| RankedSession {
            session_id: id,
            score: agg.total_score / agg.doc_count as f64,
            project: agg.project,
            timestamp: agg.timestamp,
            sample_content: agg.sample_content,
        })
        .collect();

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.truncate(limit);
    Ok(ranked)
}

/// Shows recent sessions across all projects.
pub fn recent(limit: usize, project_filter: Option<&str>) -> Result<()> {
    let db = open_db()?;
    let sessions = db.get_recent_sessions(limit, project_filter)?;

    if sessions.is_empty() {
        if let Some(filter) = project_filter {
            println!("No sessions found for project: {filter}");
        } else {
            println!("No sessions found. Run `glhf index` first.");
        }
        return Ok(());
    }

    println!("Recent Sessions ({} total)", sessions.len());
    println!("{}", "─".repeat(60));

    for session in &sessions {
        let proj_display = project_name(Some(&session.project));
        let time = format_relative_time(Some(&session.last_activity));
        let short_id = &session.session_id[..8.min(session.session_id.len())];

        println!(
            "{:<20} {:>6} msgs  {:>10}  {}",
            proj_display, session.message_count, time, short_id
        );
    }

    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::SearchResult;
    use proptest::prelude::*;

    fn make_result(id: &str, score: f64) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            score,
            chunk_kind: "message".to_string(),
            content: format!("content {id}"),
            project: None,
            session_id: None,
            role: None,
            tool_name: None,
            tool_id: None,
            tool_input: None,
            is_error: None,
            timestamp: None,
        }
    }

    proptest! {
        #[test]
        fn proptest_normalize_scores_in_range(
            scores in prop::collection::vec(0.0f64..1000.0, 2..50)
        ) {
            let mut results: Vec<SearchResult> = scores
                .iter()
                .enumerate()
                .map(|(i, &s)| make_result(&format!("r{i}"), s))
                .collect();
            normalize_scores(&mut results);
            for r in &results {
                prop_assert!(r.score >= 0.0 && r.score <= 1.0,
                    "Score out of range: {}", r.score);
            }
        }

        #[test]
        fn proptest_normalize_scores_max_is_one(
            scores in prop::collection::vec(0.0f64..1000.0, 2..50)
                .prop_filter("need distinct values", |v| {
                    let min = v.iter().copied().fold(f64::INFINITY, f64::min);
                    let max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    (max - min).abs() > f64::EPSILON
                })
        ) {
            let mut results: Vec<SearchResult> = scores
                .iter()
                .enumerate()
                .map(|(i, &s)| make_result(&format!("r{i}"), s))
                .collect();
            normalize_scores(&mut results);
            let max_score = results.iter().map(|r| r.score).fold(f64::NEG_INFINITY, f64::max);
            prop_assert!((max_score - 1.0).abs() < f64::EPSILON,
                "Max score should be 1.0, got {max_score}");
        }

        #[test]
        fn proptest_normalize_scores_preserves_ordering(
            scores in prop::collection::vec(0.0f64..1000.0, 2..50)
        ) {
            let mut results: Vec<SearchResult> = scores
                .iter()
                .enumerate()
                .map(|(i, &s)| make_result(&format!("r{i}"), s))
                .collect();

            let original_scores: Vec<f64> = results.iter().map(|r| r.score).collect();
            normalize_scores(&mut results);

            for i in 0..results.len() {
                for j in (i + 1)..results.len() {
                    if original_scores[i] > original_scores[j] {
                        prop_assert!(results[i].score >= results[j].score);
                    } else if original_scores[i] < original_scores[j] {
                        prop_assert!(results[i].score <= results[j].score);
                    }
                }
            }
        }

        #[test]
        fn proptest_normalize_scores_all_same(
            score in 0.0f64..1000.0,
            count in 1..20usize,
        ) {
            let mut results: Vec<SearchResult> = (0..count)
                .map(|i| make_result(&format!("r{i}"), score))
                .collect();
            normalize_scores(&mut results);
            for r in &results {
                prop_assert!((r.score - 1.0).abs() < f64::EPSILON,
                    "All-same scores should normalize to 1.0, got {}", r.score);
            }
        }
    }

    #[test]
    fn test_normalize_scores_empty_is_noop() {
        let mut results: Vec<SearchResult> = vec![];
        normalize_scores(&mut results);
        assert!(results.is_empty());
    }
}
