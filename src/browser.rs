//! Discovers installed browsers and their history/bookmarks file paths.

use dirs::home_dir;
use std::collections::HashMap;
use std::fmt;
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
    Orion,
    Vivaldi,
    Arc,
    Chromium,
    Sidekick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserFamily {
    /// Generally based upon Firefox infrastructure
    Gecko,

    /// Generally based upon Chrome infastructure
    Chromium,

    /// Generally based upon Safari infastructure
    Webkit,

    /// Orion
    Orion,
}

impl fmt::Display for Browser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Zen => "Zen",
            Self::Chrome => "Google Chrome",
            Self::Orion => "Orion",
            Self::ChromeBeta => "Google Chrome Beta",
            Self::Brave => "Brave",
            Self::BraveBeta => "Brave Beta",
            Self::Safari => "Safari",
            Self::Firefox => "Firefox",
            Self::Edge => "Microsoft Edge",
            Self::Opera => "Opera",
            Self::Vivaldi => "Vivaldi",
            Self::Arc => "Arc",
            Self::Chromium => "Chromium",
            Self::Sidekick => "Sidekick",
        })
    }
}

impl Browser {
    /// Returns the browser family this browser belongs to
    pub const fn family(&self) -> BrowserFamily {
        match self {
            Self::Chrome
            | Self::ChromeBeta
            | Self::Brave
            | Self::BraveBeta
            | Self::Edge
            | Self::Opera
            | Self::Vivaldi
            | Self::Arc
            | Self::Chromium
            | Self::Sidekick => BrowserFamily::Chromium,

            Self::Firefox | Self::Zen => BrowserFamily::Gecko,
            Self::Safari => BrowserFamily::Webkit,
            Self::Orion => BrowserFamily::Orion,
        }
    }

    /// Get the environment variable name used for configuration
    pub const fn env_var(&self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Orion => "orion",
            Self::Zen => "zen",
            Self::ChromeBeta => "chrome_beta",
            Self::Brave => "brave",
            Self::BraveBeta => "brave_beta",
            Self::Safari => "safari",
            Self::Firefox => "firefox",
            Self::Edge => "edge",
            Self::Opera => "opera",
            Self::Vivaldi => "vivaldi",
            Self::Arc => "arc",
            Self::Chromium => "chromium",
            Self::Sidekick => "sidekick",
        }
    }

    /// Check if this browser is enabled in the workflow configuration
    pub fn is_enabled(&self) -> bool {
        crate::utils::get_env_bool(self.env_var())
    }

    /// Returns all browser variants
    const fn all() -> [Self; 14] {
        [
            Self::Chrome,
            Self::ChromeBeta,
            Self::Brave,
            Self::BraveBeta,
            Self::Safari,
            Self::Firefox,
            Self::Edge,
            Self::Zen,
            Self::Opera,
            Self::Orion,
            Self::Vivaldi,
            Self::Arc,
            Self::Chromium,
            Self::Sidekick,
        ]
    }

    /// Get default history path for this browser relative to home directory
    const fn history_path(&self) -> &'static str {
        match self {
            Self::Chrome => "Library/Application Support/Google/Chrome/Default/History",
            Self::ChromeBeta => "Library/Application Support/Google/ChromeBeta/Default/History",
            Self::Orion => "Library/Application Support/Orion/Defaults/history",
            Self::Brave => {
                "Library/Application Support/BraveSoftware/Brave-Browser/Default/History"
            }
            Self::BraveBeta => {
                "Library/Application Support/BraveSoftware/Brave-Browser-Beta/Default/History"
            }
            Self::Safari => "Library/Safari/History.db",
            Self::Firefox => "Library/Application Support/Firefox/Profiles",
            Self::Zen => "Library/Application Support/zen/Profiles",
            Self::Edge => "Library/Application Support/Microsoft Edge/Default/History",
            Self::Opera => "Library/Application Support/com.operasoftware.Opera/History",
            Self::Vivaldi => "Library/Application Support/Vivaldi/Default/History",
            Self::Arc => "Library/Application Support/Arc/User Data/Default/History",
            Self::Chromium => "Library/Application Support/Chromium/Default/History",
            Self::Sidekick => "Library/Application Support/Sidekick/Default/History",
        }
    }

    /// Get default bookmarks path for this browser relative to home directory
    const fn bookmarks_path(&self) -> &'static str {
        match self {
            Self::Chrome => "Library/Application Support/Google/Chrome/Default/Bookmarks",
            Self::ChromeBeta => "Library/Application Support/Google/ChromeBeta/Default/Bookmarks",
            Self::Orion => "Library/Application Support/Orion/Defaults/favourites.plist",
            Self::Brave => {
                "Library/Application Support/BraveSoftware/Brave-Browser/Default/Bookmarks"
            }
            Self::BraveBeta => {
                "Library/Application Support/BraveSoftware/Brave-Browser-Beta/Default/Bookmarks"
            }
            Self::Safari => "Library/Safari/Bookmarks.plist",
            Self::Firefox => "Library/Application Support/Firefox/Profiles",
            Self::Zen => "Library/Application Support/zen/Profiles",
            Self::Edge => "Library/Application Support/Microsoft Edge/Default/Bookmarks",
            Self::Opera => "Library/Application Support/com.operasoftware.Opera/Bookmarks",
            Self::Vivaldi => "Library/Application Support/Vivaldi/Default/Bookmarks",
            Self::Arc => "Library/Application Support/Arc/User Data/Default/Bookmarks",
            Self::Chromium => "Library/Application Support/Chromium/Default/Bookmarks",
            Self::Sidekick => "Library/Application Support/Sidekick/Default/Bookmarks",
        }
    }
}

/// Represents paths to browser data files
#[derive(Debug)]
pub struct BrowserPaths {
    pub history: Option<PathBuf>,
    pub bookmarks: Option<PathBuf>,
}

impl BrowserPaths {
    fn resolve_gecko_paths(home: &PathBuf, profile_base: &str) -> Self {
        let profile_dir = home.join(profile_base);

        let db_path = fs::read_dir(&profile_dir).ok().and_then(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .find(|path| path.is_dir())
                .and_then(|profile| {
                    let db = profile.join("places.sqlite");
                    db.is_file().then_some(db)
                })
        });

        Self {
            history: db_path.clone(),
            bookmarks: db_path,
        }
    }

    fn from_browser(browser: Browser, home: &PathBuf) -> Self {
        if browser.family() == BrowserFamily::Gecko {
            return Self::resolve_gecko_paths(home, browser.history_path());
        }

        let history = home.join(browser.history_path());
        let bookmarks = home.join(browser.bookmarks_path());

        Self {
            history: history.is_file().then_some(history),
            bookmarks: bookmarks.is_file().then_some(bookmarks),
        }
    }
}

/// Get all available browsers on the system
pub fn get_available_browsers() -> HashMap<Browser, BrowserPaths> {
    let Some(home) = home_dir() else {
        return HashMap::new();
    };

    Browser::all()
        .into_iter()
        .filter(|b| b.is_enabled())
        .map(|browser| (browser, BrowserPaths::from_browser(browser, &home)))
        .collect()
}
