//! Entry point for the browser‐search Alfred workflow.
//!
//! - Parses CLI args: command (`bookmarks`/`history`/`search`) and query.
//! - Dispatches to bookmarks::search, history::search, or both.
//! - Deduplicates combined results, then calls alfred::output_results.
//! - Uses env_logger for structured logging and prints execution time to debug.

use std::error::Error;
use std::time::Instant;

use clap::{Parser, Subcommand};
use log::{debug, LevelFilter};

mod alfred;
mod bookmarks;
mod browser;
mod cache;
mod db;
mod history;
mod search;
mod tabs;
mod tie_break;
mod utils;

/// A CLI tool for searching browser bookmarks and history via Alfred.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Increase verbosity (e.g., -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress all log output
    #[arg(short, long)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Search only bookmarks
    Bookmarks {
        #[arg(default_value_t = String::new(), help = "The search query")]
        query: String,
    },
    /// Search only history
    History {
        #[arg(default_value_t = String::new(), help = "The search query")]
        query: String,
    },
    /// Search both bookmarks and history (default if no specific subcommand or query given)
    Search {
        #[arg(default_value_t = String::new(), help = "The search query")]
        query: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let start = Instant::now();

    // Configure env_logger based on verbosity and quiet flags
    let mut log_builder = env_logger::Builder::from_default_env();
    if cli.quiet {
        log_builder.filter_level(LevelFilter::Off);
    } else {
        match cli.verbose {
            0 => log_builder.filter_level(LevelFilter::Info), // Default to Info if not quiet and no -v
            1 => log_builder.filter_level(LevelFilter::Debug),
            2 => log_builder.filter_level(LevelFilter::Trace),
            _ => log_builder.filter_level(LevelFilter::Trace), // More than 2 -v also means Trace
        };
    }
    log_builder.init();

    // Determine the query and search type
    let search_results = match cli.command {
        Commands::Bookmarks { query } => bookmarks::search(&query)?,
        Commands::History { query } => history::search(&query)?,
        Commands::Search { query } => {
            let mut b = bookmarks::search(&query)?;
            let h = history::search(&query)?;
            b.extend(h);
            search::deduplicate(b)
        }
    };

    // Emit Alfred JSON
    alfred::output_results(&search_results)?;
    debug!("Search completed in {:?}", start.elapsed());
    Ok(())
}
