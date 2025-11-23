use clap::{Parser, Subcommand};

/// A CLI tool for searching browser bookmarks and history via Alfred.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Increase verbosity (e.g., -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all log output
    #[arg(short, long)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
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
