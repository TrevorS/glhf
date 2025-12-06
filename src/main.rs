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
        /// The search query
        query: String,

        /// Maximum number of results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Show N messages after each match (like grep -A)
        #[arg(short = 'A', long = "after-context", value_name = "NUM")]
        after: Option<usize>,

        /// Show N messages before each match (like grep -B)
        #[arg(short = 'B', long = "before-context", value_name = "NUM")]
        before: Option<usize>,

        /// Show N messages before and after each match (like grep -C)
        #[arg(short = 'C', long = "context", value_name = "NUM")]
        context: Option<usize>,
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
            after,
            before,
            context,
        } => {
            let options = SearchOptions {
                limit,
                before: context.or(before).unwrap_or(0),
                after: context.or(after).unwrap_or(0),
            };
            glhf::commands::search(&query, options)?;
        }
        Commands::Status => {
            glhf::commands::status()?;
        }
    }

    Ok(())
}
