use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::{Parser, Subcommand};
use glhf::commands::SearchOptions;

fn parse_since(s: &str) -> Result<DateTime<Utc>, String> {
    let s = s.trim();

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

    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    Err(format!(
        "Invalid duration/date: {s}. Use format like 1h, 2d, 1w, or 2024-12-01"
    ))
}

#[derive(Parser)]
#[command(name = "glhf", about = "Search your Claude Code history", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build or update the search index (incremental by default)
    Index {
        /// Skip embedding generation (text search only)
        #[arg(long)]
        skip_embeddings: bool,

        /// Force a full rebuild instead of incremental update
        #[arg(long)]
        full: bool,
    },

    /// Search indexed content
    #[command(after_help = "\
EXAMPLES:
    glhf search 'error' --compact              Quick scan of results
    glhf search 'git' -t Bash --compact        Find git commands you ran
    glhf search 'bug' -p myapp --since 1w      Filter by project and time
    glhf search 'failed' --errors --compact    Find errors only
")]
    Search {
        /// The search query
        query: String,

        /// Maximum number of results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Filter by tool name (e.g., Bash, Read, Edit, Grep)
        #[arg(short = 't', long = "tool", value_name = "NAME")]
        tool: Option<String>,

        /// Filter by project name (substring match, use '.' for current)
        #[arg(short = 'p', long = "project", value_name = "NAME")]
        project: Option<String>,

        /// Only show error results
        #[arg(long = "errors")]
        errors: bool,

        /// Only show results since a given time (e.g., 1h, 2d, 1w, or 2024-12-01)
        #[arg(long = "since", value_name = "DURATION", value_parser = parse_since)]
        since: Option<DateTime<Utc>>,

        /// Output results as JSON (machine-readable)
        #[arg(long = "json")]
        json: bool,

        /// Compact output format (one line per result)
        #[arg(long = "compact")]
        compact: bool,
    },

    /// Show index status and statistics
    Status,

    /// View a full conversation session
    #[command(after_help = "\
EXAMPLES:
    glhf session abc123 --summary        Quick overview of a session
    glhf session abc123 --limit 30       First 30 messages only
    glhf session abc123 --json           Machine-readable output
")]
    Session {
        /// Session ID (partial match supported)
        session_id: String,

        /// Output as JSON (machine-readable)
        #[arg(long = "json")]
        json: bool,

        /// Show only first N messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Show session summary without full content
        #[arg(long)]
        summary: bool,
    },

    /// Show recent sessions
    #[command(after_help = "\
EXAMPLES:
    glhf recent                          Show 10 most recent sessions
    glhf recent -l 20                    Show 20 most recent sessions
    glhf recent -p myproject             Filter by project name
")]
    Recent {
        /// Number of sessions to show
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Filter by project name (substring match)
        #[arg(short, long)]
        project: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index {
            skip_embeddings,
            full,
        } => {
            glhf::commands::index(skip_embeddings, full)?;
        }
        Commands::Search {
            query,
            limit,
            tool,
            project,
            errors,
            since,
            json,
            compact,
        } => {
            let options = SearchOptions {
                limit,
                tool,
                project,
                errors,
                since,
                json,
                compact,
            };
            glhf::commands::search(&query, &options)?;
        }
        Commands::Status => {
            glhf::commands::status()?;
        }
        Commands::Session {
            session_id,
            json,
            limit,
            summary,
        } => {
            glhf::commands::session(&session_id, json, limit, summary)?;
        }
        Commands::Recent { limit, project } => {
            glhf::commands::recent(limit, project.as_deref())?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_parse_since_hours(n in 1..1000i64) {
            let input = format!("{n}h");
            let result = parse_since(&input);
            prop_assert!(result.is_ok(), "Failed to parse {input}: {:?}", result);
        }

        #[test]
        fn proptest_parse_since_days(n in 1..1000i64) {
            let input = format!("{n}d");
            let result = parse_since(&input);
            prop_assert!(result.is_ok(), "Failed to parse {input}: {:?}", result);
        }

        #[test]
        fn proptest_parse_since_weeks(n in 1..1000i64) {
            let input = format!("{n}w");
            let result = parse_since(&input);
            prop_assert!(result.is_ok(), "Failed to parse {input}: {:?}", result);
        }

        #[test]
        fn proptest_parse_since_iso_date(
            y in 2000..2030i32,
            m in 1..=12u32,
            d in 1..=28u32,
        ) {
            let input = format!("{y}-{m:02}-{d:02}");
            let result = parse_since(&input);
            prop_assert!(result.is_ok(), "Failed to parse {input}: {:?}", result);
        }

        #[test]
        fn proptest_parse_since_plain_numbers_err(n in 0..10000i64) {
            let input = format!("{n}");
            let result = parse_since(&input);
            prop_assert!(result.is_err(), "Plain number should fail: {input}");
        }

        #[test]
        fn proptest_parse_since_random_never_panics(input in "\\PC{0,50}") {
            let _ = parse_since(&input);
        }
    }
}
