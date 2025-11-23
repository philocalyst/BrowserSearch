use std::error::Error;
use std::time::Instant;

use alfrusco::config::AlfredEnvProvider;
use alfrusco::{execute, Item, Runnable, Workflow};
use clap::Parser;
use log::debug;

use crate::cli::{Cli, Commands};
use crate::error::WorkflowErrorType;

mod bookmarks;
mod browser;
mod cli;
mod db;
mod error;
mod history;
mod search;
mod tabs;
mod tie_break;
mod utils;

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
