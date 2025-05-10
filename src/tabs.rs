//! Browser tab management functionality.
//!
//! Provides functionality to:
//! - List tabs from various browsers (Chrome/Chromium, Safari, Firefox, Arc)
//! - Focus on specific tabs in those browsers
//! - Convert between different browser-specific tab formats

use osascript::JavaScript;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::fmt;

use crate::browser::Browser;

/// Scary browser errors
#[derive(Debug)]
pub enum TabError {
    /// Browser is not running
    BrowserNotRunning(String),
    /// Error executing JavaScript
    ScriptExecution(String),
    /// Error parsing script output
    ParseError(String),
    /// Other error with description
    Other(String),
}

// Implement for the standard error
impl StdError for TabError {}

// Display for debugging
impl fmt::Display for TabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TabError::BrowserNotRunning(browser) => {
                write!(f, "Browser '{}' is not running", browser)
            }
            TabError::ScriptExecution(msg) => write!(f, "Script execution error: {}", msg),
            TabError::ParseError(msg) => write!(f, "Error parsing script output: {}", msg),
            TabError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

// Etc.
impl From<osascript::Error> for TabError {
    fn from(err: osascript::Error) -> Self {
        TabError::ScriptExecution(err.to_string())
    }
}

impl From<serde_json::Error> for TabError {
    fn from(err: serde_json::Error) -> Self {
        TabError::ParseError(err.to_string())
    }
}

/// Represents a browser tab with all necessary metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tab {
    /// Tab title
    pub title: String,

    /// Tab URL
    pub url: String,

    /// Subtitle (usually the URL for display purposes)
    pub subtitle: String,

    /// Window index (0-based)
    pub window_index: usize,

    /// Tab index (0-based)
    pub tab_index: usize,

    /// Space index (for Arc browser, 0-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_index: Option<usize>,

    /// Argument string for focusing this tab
    pub arg: String,
}

/// Response structure from the tab lists
#[derive(Debug, Serialize, Deserialize)]
struct TabList {
    items: Vec<TabItem>,
}

/// Individual tab item information from script response
#[derive(Debug, Serialize, Deserialize)]
struct TabItem {
    title: String,
    url: String,
    subtitle: String,
    #[serde(rename = "windowIndex")]
    window_index: usize,
    #[serde(rename = "tabIndex")]
    tab_index: usize,
    #[serde(
        rename = "spaceIndex",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    space_index: Option<usize>,
    arg: String,
    // Optional fields that may be present but aren't needed directly
    #[serde(
        rename = "quicklookurl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    _quicklookurl: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    _match: Option<String>,
}

    // Choose the appropriate script based on browser type
    let script_content = if browser.contains("Arc") {
        include_str!("./focus-arc.js")
    } else if browser == "Safari" {
        include_str!("./focus-webkit.js")
    } else {
        include_str!("./focus-chromium.js")
    };

    let script = JavaScript::new(script_content);

    // Build the query string
    let query = if browser.contains("Arc") {
        // Arc format: windowIndex,tabIndex (spaceIndex is handled in script)
        format!("{},{}", tab.window_index, tab.tab_index)
    } else if browser == "Safari" {
        // For Safari with a URL to match
        format!("{},{}", tab.window_index, tab.url)
    } else {
        // Standard format: windowIndex,tabIndex
        format!("{},{}", tab.window_index, tab.tab_index)
    };

    // Execute with browser name and query as parameters
    let response: String =
        script.execute_with_params::<_, String>(vec![browser.to_string(), query])?;

    println!("{response}");

    Ok(())
}
