use anyhow::Result;
use clap::{Parser, Subcommand};
use glhf::commands::SearchOptions;

#[derive(Parser)]
#[command(name = "glhf", about = "Search your Claude Code history")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build or update the search index
    Index {
        /// Force a full rebuild of the index
        #[arg(long)]
        rebuild: bool,
    },

    /// Search indexed content
    Search {
        /// The search query (or regex pattern with -e)
        query: String,

        /// Maximum number of results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Interpret the query as a regular expression (like grep -e)
        #[arg(short = 'e', long = "regex")]
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

        /// Only show tool results that were errors
        #[arg(long = "errors")]
        errors: bool,

        /// Only show messages (exclude `tool_use` and `tool_result`)
        #[arg(long = "messages-only")]
        messages_only: bool,

        /// Only show tool calls (`tool_use` and `tool_result`)
        #[arg(long = "tools-only")]
        tools_only: bool,
    },

    /// Show index status and statistics
    Status,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index { rebuild } => {
            glhf::commands::index(rebuild)?;
        }
        Commands::Search {
            query,
            limit,
            regex,
            ignore_case,
            after,
            before,
            context,
            tool,
            errors,
            messages_only,
            tools_only,
        } => {
            let options = SearchOptions {
                limit,
                regex,
                ignore_case,
                before: context.or(before).unwrap_or(0),
                after: context.or(after).unwrap_or(0),
                tool,
                errors,
                messages_only,
                tools_only,
            };
            glhf::commands::search(&query, &options)?;
        }
        Commands::Status => {
            glhf::commands::status()?;
        }
    }

    Ok(())
}
