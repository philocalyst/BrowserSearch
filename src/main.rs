use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use alfrusco::config::{AlfredEnvProvider, WorkflowConfig};
use alfrusco::{execute, AsyncRunnable, Item, Runnable, Workflow, WorkflowError};
use clap::{Parser, Subcommand};
use log::{debug, LevelFilter};

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

#[derive(Subcommand, Clone, Debug)]
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

// Define error types compatible with alfrusco
#[derive(Debug, thiserror::Error)]
pub enum WorkflowErrorType {
    #[error("Search error: {0}")]
    Search(#[from] Box<dyn Error>),
    #[error("Tab management error: {0}")]
    Tab(#[from] tabs::TabError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl WorkflowError for WorkflowErrorType {}

impl Runnable for Cli {
    type Error = WorkflowErrorType;

    fn run(self, workflow: &mut Workflow) -> Result<(), WorkflowErrorType> {
        let start = Instant::now();

        // Configure logging using alfrusco's init_logging (instead of env_logger manually)
        let _ = alfrusco::init_logging(&AlfredEnvProvider);

        // Dispatch based on command
        let search_results = match self.command.clone() {
            Commands::Bookmarks { query } => bookmarks::search(&query)?,
            Commands::History { query } => history::search(&query)?,
            Commands::Search { query } => {
                let mut b = bookmarks::search(&query)?;
                let h = history::search(&query)?;
                b.extend(h);
                search::deduplicate(b)
            }
        };

        // Set the query for filtering
        let query = match &self.command {
            Commands::Bookmarks { query } => query,
            Commands::History { query } => query,
            Commands::Search { query } => query,
        };
        workflow.set_filter_keyword(query.clone());

        // Convert SearchResult to Alfred Items
        let items: Vec<Item> = search_results
            .into_iter()
            .map(|result| {
                let mut item = Item::new(&result.title)
                    .subtitle(&result.subtitle)
                    .arg(&result.url)
                    .valid(true);

                // Add favicon if available
                if let Some(favicon_path) = &result.favicon {
                    item = item.icon_from_image(favicon_path);
                }

                // Add visit count as variable
                if let Some(visit_count) = result.visit_count {
                    item = item.var("visit_count", &visit_count.to_string());
                }

                // Add source as variable
                item = item.var(
                    "source",
                    match result.source {
                        search::ResultSource::Bookmark => "bookmark",
                        search::ResultSource::History => "history",
                    },
                );

                item
            })
            .collect();

        workflow.append_items(items);
        debug!("Search completed in {:?}", start.elapsed());
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    execute(&AlfredEnvProvider, cli, &mut std::io::stdout());
    Ok(())
}
