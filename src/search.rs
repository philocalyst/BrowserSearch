//! Core search‐result types and matching utilities.
//!
//! Defines:
//! - `SearchResult` struct and `ResultSource` enum
//! - `deduplicate` to remove duplicate URLs (preferring bookmarks)
//! - `matches` supporting AND (`&`) / OR (`|`) / substring
//! - `filter_results` to apply the query to title, url, or subtitle.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents a generic search result from either bookmarks or history
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub subtitle: String,
    pub favicon: Option<String>,
    pub source: ResultSource,
    pub visit_count: Option<u32>,
    pub last_visit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResultSource {
    Bookmark,
    History,
}

/// Deduplicate results based on URL
pub fn deduplicate(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();

    // Prefer bookmarks over history when deduplicating
    let mut sorted_results = results;
    sorted_results.sort_by(|a, b| {
        if a.source == ResultSource::Bookmark && b.source == ResultSource::History {
            std::cmp::Ordering::Less
        } else if a.source == ResultSource::History && b.source == ResultSource::Bookmark {
            std::cmp::Ordering::Greater
        } else {
            a.title.cmp(&b.title)
        }
    });

    for result in sorted_results {
        if seen.insert(result.url.clone()) {
            unique.push(result);
        }
    }

    unique
}

/// Matches a search query against text
/// Supports & (AND) and | (OR) operators
pub fn matches(query: &str, text: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let text_lower = text.to_lowercase();

    if query.contains('&') {
        // AND search
        query
            .split('&')
            .map(str::trim)
            .all(|term| text_lower.contains(&term.to_lowercase()))
    } else if query.contains('|') {
        // OR search
        query
            .split('|')
            .map(str::trim)
            .any(|term| text_lower.contains(&term.to_lowercase()))
    } else {
        // Simple search
        text_lower.contains(&query.to_lowercase())
    }
}

/// Filter results based on search criteria
pub fn filter_results(results: Vec<SearchResult>, query: &str) -> Vec<SearchResult> {
    if query.is_empty() {
        return results;
    }

    results
        .into_iter()
        .filter(|result| {
            matches(query, &result.title)
                || matches(query, &result.url)
                || matches(query, &result.subtitle)
        })
        .collect()
}
