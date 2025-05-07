//! Searches bookmarks across all enabled browsers.
//!
//! Provides:
//! - `search(query: &str)` entry point
//! - `search_chrome_bookmarks` / `search_safari_bookmarks`
//! - Recursive extractors (`extract_chrome_bookmarks`,
//!   `extract_safari_bookmarks`)
//! - Uses serde_json and plist for parsing, rayon for parallelism,
//!   and filter_results to match the query.

use crate::browser::get_available_browsers;
use crate::db::{create_temp_db_copy, query_firefox_bookmarks};
use crate::search::{filter_results, ResultSource, SearchResult};
use plist::Value as PlistValue;
use rayon::prelude::*;
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Search bookmarks across all enabled browsers
pub fn search(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    log::trace!("Beginning bookmarks search");
    let browsers = get_available_browsers();

    // Perform searches in parallel using rayon
    let browser_results: Vec<Vec<SearchResult>> = browsers
        .par_iter()
        .filter_map(|(browser, paths)| {
            if let Some(bookmarks_path) = &paths.bookmarks {
                let result = match browser {
                    b if b.is_safari_like() => search_safari_bookmarks(&bookmarks_path, query),
                    b if b.is_firefox_like() => search_firefox_bookmarks(&bookmarks_path, query),
                    b if b.is_chrome_like() => search_chrome_bookmarks(&bookmarks_path, query),
                    _ => unreachable!("unsupported browser: {:?}", browser),
                };

                match result {
                    Ok(results) => Some(results),
                    Err(e) => {
                        log::error!("Error searching {:?} bookmarks: {}", browser, e);
                        None
                    }
                }
            } else {
                None
            }
        })
        .collect();

    // Collect results
    let mut all_results = Vec::new();
    for results in browser_results {
        all_results.extend(results);
    }

    // Deduplicate by URL
    let mut seen = std::collections::HashSet::new();
    all_results.retain(|result| seen.insert(result.url.clone()));

    // Sort alphabetically
    all_results.sort_by(|a, b| a.title.cmp(&b.title));

    Ok(all_results)
}

/// Search Chrome-based browser bookmarks
fn search_chrome_bookmarks(
    bookmark_path: &Path,
    query: &str,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Read the bookmarks file
    let mut file = File::open(bookmark_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse JSON
    let bookmarks: Value = serde_json::from_str(&contents)?;

    // Extract all bookmarks
    let mut results = Vec::new();
    if let Some(roots) = bookmarks.get("roots") {
        extract_chrome_bookmarks(roots, &mut results);
    }

    // Search bookmarks
    let matching_results = filter_results(results, query);
    Ok(matching_results)
}

/// Recursively extract bookmarks from Chrome JSON structure
fn extract_chrome_bookmarks(value: &Value, results: &mut Vec<SearchResult>) {
    if let Some(obj) = value.as_object() {
        // Check if this is a bookmark
        if let (Some(Value::String(url)), Some(Value::String(name)), Some(Value::String(typ))) =
            (obj.get("url"), obj.get("name"), obj.get("type"))
        {
            if typ == "url" {
                results.push(SearchResult {
                    title: name.clone(),
                    url: url.clone(),
                    subtitle: url.clone(),
                    favicon: None,
                    source: ResultSource::Bookmark,
                    visit_count: None,
                    last_visit: None,
                });
            }
        }

        // Check for children (folders)
        if let Some(Value::Array(children)) = obj.get("children") {
            for child in children {
                extract_chrome_bookmarks(child, results);
            }
        }

        // Recursively check all properties
        for (_, v) in obj {
            extract_chrome_bookmarks(v, results);
        }
    } else if let Some(arr) = value.as_array() {
        for item in arr {
            extract_chrome_bookmarks(item, results);
        }
    }
}

/// Search Safari bookmarks
fn search_safari_bookmarks(
    bookmark_path: &Path,
    query: &str,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Read the plist file
    let file = File::open(bookmark_path)?;
    let value = plist::from_reader(file)?;

    // Extract all bookmarks
    let mut results = Vec::new();
    extract_safari_bookmarks(&value, &mut results);

    // Search bookmarks
    let matching_results = filter_results(results, query);
    Ok(matching_results)
}

/// Recursively extract bookmarks from Safari plist structure
fn extract_safari_bookmarks(value: &PlistValue, results: &mut Vec<SearchResult>) {
    match value {
        PlistValue::Dictionary(dict) => {
            // Check if this is a bookmark
            if let (Some(PlistValue::String(url)), Some(PlistValue::Dictionary(uri_dict))) =
                (dict.get("URLString"), dict.get("URIDictionary"))
            {
                if let Some(PlistValue::String(title)) = uri_dict.get("title") {
                    results.push(SearchResult {
                        title: title.clone(),
                        url: url.clone(),
                        subtitle: url.clone(),
                        favicon: None,
                        source: ResultSource::Bookmark,
                        visit_count: None,
                        last_visit: None,
                    });
                }
            }

            // Check for children (folders)
            if let Some(PlistValue::Array(children)) = dict.get("Children") {
                for child in children {
                    extract_safari_bookmarks(child, results);
                }
            }

            // Recursively check all values
            for (_, v) in dict {
                extract_safari_bookmarks(v, results);
            }
        }
        PlistValue::Array(arr) => {
            for item in arr {
                extract_safari_bookmarks(item, results);
            }
        }
        _ => {}
    }
}

/// Firefox bookmarks (SQLite)
fn search_firefox_bookmarks(
    bookmark_path: &Path,
    query: &str,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Copy the locked db for easy access
    let (_tmp, conn) = create_temp_db_copy(bookmark_path, None, None)?;

    // grab every “real” bookmark (type=1) and where the data isn't sparse.
    let sql = r#"
        SELECT b.title, p.url
          FROM moz_bookmarks AS b
          JOIN moz_places   AS p ON b.fk = p.id
         WHERE b.type = 1
           AND p.url   IS NOT NULL
           AND b.title IS NOT NULL
    "#;

    // Query the firefox bookmarks
    let raw: Vec<SearchResult> = query_firefox_bookmarks(&conn, sql, |row| {
        let title: String = row.get(0)?;
        let url: String = row.get(1)?;
        Ok(SearchResult {
            title: title.clone(),
            url: url.clone(),
            subtitle: url,
            favicon: None,
            source: ResultSource::Bookmark,
            visit_count: None,
            last_visit: None,
        })
    })?;

    // finally apply your existing filter_results
    Ok(raw)
}
