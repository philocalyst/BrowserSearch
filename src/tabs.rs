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
