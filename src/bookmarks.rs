//! Searches bookmarks across all enabled browsers.
//!
//! Provides:
//! - `search(query: &str)` entry point
//! - Generic `search_bookmarks` with browser family parameter
//! - Recursive extractors (`extract_chrome_bookmarks`, `extract_safari_bookmarks`)
//! - Uses serde_json and plist for parsing, rayon for parallelism,
//!   SeaQuery for type-safe SQL, and filter_results to match the query.

use crate::browser::{get_available_browsers, BrowserFamily};
use crate::search::{filter_results, ResultSource, SearchResult};
use plist::Value as PlistValue;
use rayon::prelude::*;
use rusqlite::{params_from_iter, Connection, OpenFlags};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Firefox/Gecko browser tables
#[derive(Iden)]
enum MozBookmarks {
    #[iden = "moz_bookmarks"]
    Table,
    Title,
    #[iden = "type"]
    Type,
    Fk,
}

#[derive(Iden)]
enum MozPlaces {
    #[iden = "moz_places"]
    Table,
    Id,
    Url,
}

#[derive(Iden)]
enum BookmarkAlias {
    #[iden = "b"]
    Table,
}

#[derive(Iden)]
enum PlacesAlias {
    #[iden = "p"]
    Table,
}

/// Search bookmarks across all enabled browsers
pub fn search(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    log::trace!("Beginning bookmarks search");
    let browsers = get_available_browsers();

    // Perform searches in parallel using rayon
    let browser_results: Vec<Vec<SearchResult>> = browsers
        .par_iter()
        .filter_map(|(browser, paths)| {
            if let Some(bookmarks_path) = &paths.bookmarks {
                let result = search_bookmarks(bookmarks_path, query, browser.family());

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

/// Generic bookmark search that dispatches based on browser family
pub fn search_bookmarks(
    bookmark_path: &Path,
    query: &str,
    family: BrowserFamily,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    match family {
        BrowserFamily::Webkit => search_webkit_bookmarks(bookmark_path, query),
        BrowserFamily::Gecko => search_gecko_bookmarks(bookmark_path, query),
        BrowserFamily::Chromium => search_chromium_bookmarks(bookmark_path, query),
        BrowserFamily::Orion => Err("Orion bookmarks not yet supported".into()),
    }
}

/// Search Chromium-based browser bookmarks
fn search_chromium_bookmarks(
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

/// Search WebKit-based browser bookmarks (Safari)
fn search_webkit_bookmarks(
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

/// Search Gecko-based browser bookmarks (Firefox, Zen)
fn search_gecko_bookmarks(
    bookmark_path: &Path,
    query: &str,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Copy the locked db for easy access
    let conn = Connection::open_with_flags(bookmark_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    // Build the query using SeaQuery
    let query_builder = Query::select()
        .columns([(BookmarkAlias::Table, MozBookmarks::Title)])
        .from_as(MozBookmarks::Table, BookmarkAlias::Table)
        .join_as(
            sea_query::JoinType::InnerJoin,
            MozPlaces::Table,
            PlacesAlias::Table,
            Expr::col((BookmarkAlias::Table, MozBookmarks::Fk))
                .equals((PlacesAlias::Table, MozPlaces::Id)),
        )
        .and_where(Expr::col((BookmarkAlias::Table, MozBookmarks::Type)).eq(1))
        .and_where(Expr::col((PlacesAlias::Table, MozPlaces::Url)).is_not_null())
        .and_where(Expr::col((BookmarkAlias::Table, MozBookmarks::Title)).is_not_null())
        .to_owned();

    let (sql, values) = query_builder.build(SqliteQueryBuilder);

    // Execute the query
    let raw = query_gecko_bookmarks(&conn, &sql, values)?;

    // Apply filtering based on query
    Ok(filter_results(raw, query))
}

/// Query Firefox/Gecko bookmarks from the database
fn query_gecko_bookmarks(
    conn: &Connection,
    sql: &str,
    values: sea_query::Values,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Convert SeaQuery Values to rusqlite params
    let params: Vec<rusqlite::types::Value> = values
        .0
        .into_iter()
        .map(|v| match v {
            sea_query::Value::Bool(Some(b)) => rusqlite::types::Value::Integer(b as i64),
            sea_query::Value::TinyInt(Some(i)) => rusqlite::types::Value::Integer(i as i64),
            sea_query::Value::SmallInt(Some(i)) => rusqlite::types::Value::Integer(i as i64),
            sea_query::Value::Int(Some(i)) => rusqlite::types::Value::Integer(i as i64),
            sea_query::Value::BigInt(Some(i)) => rusqlite::types::Value::Integer(i),
            sea_query::Value::TinyUnsigned(Some(i)) => rusqlite::types::Value::Integer(i as i64),
            sea_query::Value::SmallUnsigned(Some(i)) => rusqlite::types::Value::Integer(i as i64),
            sea_query::Value::Unsigned(Some(i)) => rusqlite::types::Value::Integer(i as i64),
            sea_query::Value::BigUnsigned(Some(i)) => rusqlite::types::Value::Integer(i as i64),
            sea_query::Value::Float(Some(f)) => rusqlite::types::Value::Real(f as f64),
            sea_query::Value::Double(Some(f)) => rusqlite::types::Value::Real(f),
            sea_query::Value::String(Some(s)) => rusqlite::types::Value::Text((*s).clone()),
            sea_query::Value::Bytes(Some(b)) => rusqlite::types::Value::Blob((*b).clone()),
            _ => rusqlite::types::Value::Null,
        })
        .collect();

    let mut stmt = conn.prepare(sql)?;
    let results = stmt
        .query_map(params_from_iter(params.iter()), |row| {
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
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}
