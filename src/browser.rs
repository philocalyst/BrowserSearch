//! Discovers installed browsers and their history/bookmarks file paths.
//!
//! Defines:
//! - `Browser` enum with variants for supported browsers
//! - `BrowserPaths` struct holding optional history/bookmarks paths
//! - `get_available_browsers` that reads HOME and environment flags
//!   to return only enabled & existing browser paths.

use dirs::home_dir;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Supported browser types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Browser {
    Chrome,
    ChromeBeta,
    Brave,
    BraveBeta,
    Safari,
    Firefox,
    Edge,
    Zen,
    Opera,
    Vivaldi,
    Arc,
    Chromium,
    Sidekick,
}

impl Browser {
    /// Returns true if this browser is based on Chromium.
    pub fn is_chrome_like(&self) -> bool {
        matches!(
            self,
            Browser::Chrome
                | Browser::ChromeBeta
                | Browser::Brave
                | Browser::BraveBeta
                | Browser::Edge
                | Browser::Opera
                | Browser::Vivaldi
                | Browser::Arc
                | Browser::Chromium
                | Browser::Sidekick
        )
    }

    /// Returns true if this browser is based on Firefox.
    pub fn is_firefox_like(&self) -> bool {
        matches!(self, Browser::Firefox | Browser::Zen)
    }

    /// Returns true if this browser is based on WebKit.
    pub fn is_safari_like(&self) -> bool {
        matches!(self, Browser::Safari)
    }

    /// Get the display name of the browser
    pub fn name(&self) -> &'static str {
        match self {
            Browser::Zen => "Zen",
            Browser::Chrome => "Google Chrome",
            Browser::ChromeBeta => "Google Chrome Beta",
            Browser::Brave => "Brave",
            Browser::BraveBeta => "Brave Beta",
            Browser::Safari => "Safari",
            Browser::Firefox => "Firefox",
            Browser::Edge => "Microsoft Edge",
            Browser::Opera => "Opera",
            Browser::Vivaldi => "Vivaldi",
            Browser::Arc => "Arc",
            Browser::Chromium => "Chromium",
            Browser::Sidekick => "Sidekick",
        }
    }

    /// Get the environment variable name used for configuration
    pub fn env_var(&self) -> &'static str {
        match self {
            Browser::Chrome => "chrome",
            Browser::Zen => "zen",
            Browser::ChromeBeta => "chrome_beta",
            Browser::Brave => "brave",
            Browser::BraveBeta => "brave_beta",
            Browser::Safari => "safari",
            Browser::Firefox => "firefox",
            Browser::Edge => "edge",
            Browser::Opera => "opera",
            Browser::Vivaldi => "vivaldi",
            Browser::Arc => "arc",
            Browser::Chromium => "chromium",
            Browser::Sidekick => "sidekick",
        }
    }

    /// Check if this browser is enabled in the workflow configuration
    pub fn is_enabled(&self) -> bool {
        crate::utils::get_env_bool(self.env_var())
    }
}

/// Represents paths to browser data files
#[derive(Debug)]
pub struct BrowserPaths {
    pub history: Option<PathBuf>,
    pub bookmarks: Option<PathBuf>,
}

/// Get all available browsers on the system
pub fn get_available_browsers() -> HashMap<Browser, BrowserPaths> {
    let mut browsers = HashMap::new();
    let home = match home_dir() {
        Some(path) => path,
        None => return browsers,
    };

    // Define paths for each browser
    let browser_configs = [
        (
            Browser::Chrome,
            "Library/Application Support/Google/Chrome/Default/History",
            "Library/Application Support/Google/Chrome/Default/Bookmarks",
        ),
        (
            Browser::Brave,
            "Library/Application Support/BraveSoftware/Brave-Browser/Default/History",
            "Library/Application Support/BraveSoftware/Brave-Browser/Default/Bookmarks",
        ),
        (
            Browser::BraveBeta,
            "Library/Application Support/BraveSoftware/Brave-Browser-Beta/Default/History",
            "Library/Application Support/BraveSoftware/Brave-Browser-Beta/Default/Bookmarks",
        ),
        (
            Browser::Safari,
            "Library/Safari/History.db",
            "Library/Safari/Bookmarks.plist",
        ),
        (
            Browser::Firefox,
            "Library/Application Support/Firefox/Profiles",
            "Library/Application Support/Firefox/Profiles",
        ),
        (
            Browser::Zen,
            "Library/Application Support/zen/Profiles",
            "Library/Application Support/zen/Profiles",
        ),
        (
            Browser::Edge,
            "Library/Application Support/Microsoft Edge/Default/History",
            "Library/Application Support/Microsoft Edge/Default/Bookmarks",
        ),
        (
            Browser::Opera,
            "Library/Application Support/com.operasoftware.Opera/History",
            "Library/Application Support/com.operasoftware.Opera/Bookmarks",
        ),
        (
            Browser::Vivaldi,
            "Library/Application Support/Vivaldi/Default/History",
            "Library/Application Support/Vivaldi/Default/Bookmarks",
        ),
        (
            Browser::Arc,
            "Library/Application Support/Arc/User Data/Default/History",
            "Library/Application Support/Arc/User Data/Default/Bookmarks",
        ),
        (
            Browser::Chromium,
            "Library/Application Support/Chromium/Default/History",
            "Library/Application Support/Chromium/Default/Bookmarks",
        ),
        (
            Browser::Sidekick,
            "Library/Application Support/Sidekick/Default/History",
            "Library/Application Support/Sidekick/Default/Bookmarks",
        ),
        (
            Browser::ChromeBeta,
            "Library/Application Support/Google/ChromeBeta/Default/History",
            "Library/Application Support/Google/ChromeBeta/Default/Bookmarks",
        ),
    ];

    for (browser, history_path, bookmarks_path) in browser_configs {
        if !browser.is_enabled() {
            continue;
        }

        // Join the home directory as paths are relative.
        // Mutable for Firefox support later.
        let mut history = home.join(history_path);
        let mut bookmarks = home.join(bookmarks_path);

        // If it's a variant of firefox.
        if browser.is_firefox_like() {
            // Scan each profile directory for places.sqlite
            if let Ok(entries) = fs::read_dir(&history) {
                // Get stored profiles
                for entry in entries.flatten() {
                    let profile_dir = entry.path();
                    if profile_dir.is_dir() {
                        // Scan for a places.sqlite in each
                        let db = profile_dir.join("places.sqlite");
                        if db.is_file() {
                            // history _and_ bookmarks live in the same DB
                            history.push(db.clone());
                            bookmarks.push(db);
                            break; // Only support for one entry for now
                        }
                    }
                }
            }
        }

        browsers.insert(
            browser,
            BrowserPaths {
                history: if history.exists() {
                    Some(history)
                } else {
                    None
                },
                bookmarks: if bookmarks.exists() {
                    Some(bookmarks)
                } else {
                    None
                },
            },
        );
    }

    browsers
}
