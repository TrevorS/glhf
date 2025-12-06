use anyhow::Result;
use clap::{Parser, Subcommand};

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
        Commands::Search { query, limit } => {
            glhf::commands::search(&query, limit)?;
        }
        Commands::Status => {
            glhf::commands::status()?;
        }
    }

    Ok(())
}
