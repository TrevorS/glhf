use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use glhf::commands::{SearchMode, SearchOptions};

/// Parses a duration string like "1h", "2d", "1w" or an ISO date into a UTC cutoff timestamp.
fn parse_since(s: &str) -> Result<DateTime<Utc>, String> {
    let s = s.trim();

    // Try relative duration first: 1h, 2d, 3w, etc.
    if let Some(num_str) = s.strip_suffix('h') {
        let hours: i64 = num_str.parse().map_err(|_| format!("Invalid hours: {s}"))?;
        return Ok(Utc::now() - Duration::hours(hours));
    }
    if let Some(num_str) = s.strip_suffix('d') {
        let days: i64 = num_str.parse().map_err(|_| format!("Invalid days: {s}"))?;
        return Ok(Utc::now() - Duration::days(days));
    }
    if let Some(num_str) = s.strip_suffix('w') {
        let weeks: i64 = num_str.parse().map_err(|_| format!("Invalid weeks: {s}"))?;
        return Ok(Utc::now() - Duration::weeks(weeks));
    }

    // Try ISO date: 2024-12-01
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    Err(format!(
        "Invalid duration/date: {s}. Use format like 1h, 2d, 1w, or 2024-12-01"
    ))
}

#[derive(Parser)]
#[command(name = "glhf", about = "Search your Claude Code history")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Search mode for queries.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum CliSearchMode {
    /// Hybrid search combining text and semantic (default)
    #[default]
    Hybrid,
    /// Full-text search only (FTS5)
    Text,
    /// Semantic/vector search only
    Semantic,
}

impl From<CliSearchMode> for SearchMode {
    fn from(mode: CliSearchMode) -> Self {
        match mode {
            CliSearchMode::Hybrid => SearchMode::Hybrid,
            CliSearchMode::Text => SearchMode::Text,
            CliSearchMode::Semantic => SearchMode::Semantic,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Build or update the search index
    Index {
        /// Force a full rebuild of the index
        #[arg(long)]
        rebuild: bool,

        /// Skip embedding generation (text search only)
        #[arg(long)]
        skip_embeddings: bool,
    },

    /// Search indexed content
    Search {
        /// The search query (or regex pattern with -e)
        query: String,

        /// Maximum number of results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Search mode: hybrid, text, or semantic
        #[arg(short, long, default_value = "hybrid")]
        mode: CliSearchMode,

        /// Interpret the query as a regular expression (like grep -e)
        #[arg(short = 'e', long = "regex", conflicts_with = "mode")]
        regex: bool,

        /// Case-insensitive search (like grep -i)
        #[arg(short = 'i', long = "ignore-case")]
        ignore_case: bool,

        /// Show N messages after each match (like grep -A)
        #[arg(short = 'A', long = "after-context", value_name = "NUM")]
        after: Option<usize>,

        /// Show N messages before each match (like grep -B)
        #[arg(short = 'B', long = "before-context", value_name = "NUM")]
        before: Option<usize>,

        /// Show N messages before and after each match (like grep -C)
        #[arg(short = 'C', long = "context", value_name = "NUM")]
        context: Option<usize>,

        /// Filter by tool name (e.g., Bash, Read, Edit, Grep)
        #[arg(short = 't', long = "tool", value_name = "NAME")]
        tool: Option<String>,

        /// Filter by project name (substring match, case-insensitive)
        #[arg(short = 'p', long = "project", value_name = "NAME")]
        project: Option<String>,

        /// Only show error results
        #[arg(long = "errors")]
        errors: bool,

        /// Only show messages (exclude tool calls)
        #[arg(long = "messages-only", conflicts_with_all = ["tools_only", "tool"])]
        messages_only: bool,

        /// Only show tool calls (exclude messages)
        #[arg(long = "tools-only", conflicts_with = "messages_only")]
        tools_only: bool,

        /// Output results as JSON (machine-readable)
        #[arg(long = "json")]
        json: bool,

        /// Only show results since a given time (e.g., 1h, 2d, 1w, or 2024-12-01)
        #[arg(long = "since", value_name = "DURATION", value_parser = parse_since)]
        since: Option<DateTime<Utc>>,
    },

    /// Show index status and statistics
    Status,

    /// View a full conversation session
    Session {
        /// Session ID (partial match supported)
        session_id: String,

        /// Output as JSON (machine-readable)
        #[arg(long = "json")]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index {
            rebuild,
            skip_embeddings,
        } => {
            glhf::commands::index(rebuild, skip_embeddings)?;
        }
        Commands::Search {
            query,
            limit,
            mode,
            regex,
            ignore_case,
            after,
            before,
            context,
            tool,
            project,
            errors,
            messages_only,
            tools_only,
            json,
            since,
        } => {
            let options = SearchOptions {
                limit,
                mode: mode.into(),
                regex,
                ignore_case,
                before: context.or(before).unwrap_or(0),
                after: context.or(after).unwrap_or(0),
                tool,
                project,
                errors,
                messages_only,
                tools_only,
                json,
                since,
            };
            glhf::commands::search(&query, &options)?;
        }
        Commands::Status => {
            glhf::commands::status()?;
        }
        Commands::Session { session_id, json } => {
            glhf::commands::session(&session_id, json)?;
        }
    }

    Ok(())
}
