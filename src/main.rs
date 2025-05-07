//! Entry point for the browser‐search Alfred workflow.
//!
//! - Parses CLI args: command (`bookmarks`/`history`/default) and query.
//! - Dispatches to bookmarks::search, history::search, or both.
//! - Deduplicates combined results, then calls alfred::output_results.
//! - Uses env_logger and prints execution time to debug.

use std::env;
use std::error::Error;
use std::time::Instant;

use utils::fetch_favicons;

mod alfred;
mod bookmarks;
mod browser;
mod cache;
mod db;
mod history;
mod search;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let start = Instant::now();

    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let query = args.get(2).map(|s| s.as_str()).unwrap_or("");

    // produce one combined Vec<SearchResult>
    let results = match command {
        "bookmarks" => bookmarks::search(query)?,
        "history" => history::search(query)?,
        _ => {
            let mut b = bookmarks::search(query)?;
            let h = history::search(query)?;
            b.extend(h);
            search::deduplicate(b)
        }
    };

    // emit Alfred JSON
    alfred::output_results(&results)?;
    log::debug!("Search completed in {:?}", start.elapsed());
    Ok(())
}
