//! Searches browser history across enabled browsers.
//!
//! - `search(query: &str)` coordinates loading cached results,
//!   reading each browser's history via `get_chrome_history` /
//!   `get_safari_history`, merging, deduplicating, sorting, and
//!   limiting to MAX_RESULTS.
//! - After gathering, it calls `fetch_favicons` to populate icons.

use crate::browser::get_available_browsers;
use crate::search::{ResultSource, SearchResult};
use crate::tie_break::break_a_tie;
use crate::utils::fetch_favicons;
use jiff::{fmt::strtime, Timestamp};
use nucleo::{Matcher, Utf32Str};
use rayon::prelude::*;
use rusqlite::{params_from_iter, Connection, OpenFlags, Row};
use sea_query::{Alias, Expr, Func, Iden, Query, SqliteQueryBuilder};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::Path;

// Chrome/Chromium browser tables
#[derive(Iden)]
enum ChromeUrls {
    #[iden = "urls"]
    Table,
    Id,
    Url,
    Title,
    VisitCount,
    LastVisitTime,
}

#[derive(Iden)]
enum ChromeVisits {
    #[iden = "visits"]
    Table,
    Url,
}

// Safari/Orion browser tables
#[derive(Iden)]
enum HistoryItems {
    #[iden = "history_items"]
    Table,
    #[iden = "ID"]
    Id,
    #[iden = "URL"]
    Url,
    #[iden = "TITLE"]
    Title,
    #[iden = "VISIT_COUNT"]
    VisitCount,
}

#[derive(Iden)]
enum SafariVisits {
    #[iden = "visits"]
    Table,
    #[iden = "HISTORY_ITEM_ID"]
    HistoryItemId,
    #[iden = "VISIT_TIME"]
    VisitTime,
}

// Firefox browser tables
#[derive(Iden)]
enum MozPlaces {
    #[iden = "moz_places"]
    Table,
    Id,
    Url,
    Title,
    VisitCount,
}

#[derive(Iden)]
enum MozHistoryVisits {
    #[iden = "moz_historyvisits"]
    Table,
    PlaceId,
    VisitDate,
}

/// Searches browser history for the given query
pub fn search(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let browsers = get_available_browsers();

    // Get browser histories
    let browser_histories: Vec<Vec<SearchResult>> = browsers
        .par_iter()
        .filter_map(|(browser, paths)| {
            if let Some(history_path) = &paths.history {
                let result = match browser.family() {
                    crate::browser::BrowserFamily::Webkit => get_safari_history(history_path),
                    crate::browser::BrowserFamily::Gecko => get_firefox_history(history_path),
                    crate::browser::BrowserFamily::Chromium => get_chrome_history(history_path),
                    crate::browser::BrowserFamily::Orion => get_orion_history(history_path),
                };

                match result {
                    Ok(results) => Some(results),
                    Err(e) => {
                        log::error!("Error searching {:?} history: {}", browser, e);
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
    let mut seen_urls = HashSet::new();

    // Add new results from browsers
    for results in browser_histories {
        for result in results {
            if seen_urls.insert(result.url.clone()) {
                all_results.push(result);
            }
        }
    }

    let result_count: usize = std::env::var("MAX_RESULTS")
        .unwrap_or("30".to_string())
        .parse()?;

    let config = nucleo::Config::DEFAULT;

    let mut matcher_instance = Matcher::new(config);

    // Pre-segment your query once
    let mut query_buf: Vec<char> = Vec::new();
    let query_u32 = Utf32Str::new(query, &mut query_buf);

    // Make a buffer to reuse for every title
    let mut title_buf: Vec<char> = Vec::new();

    let limited_results: HashMap<u16, Vec<SearchResult>> = all_results
        .into_iter()
        .map(|item| {
            title_buf.clear();
            let title_u32 = Utf32Str::new(&item.title, &mut title_buf);
            let score = matcher_instance
                .fuzzy_match(title_u32, query_u32)
                .unwrap_or(u16::MIN);
            (score, item)
        })
        .fold(HashMap::new(), |mut acc, (score, item)| {
            acc.entry(score).or_default().push(item);
            acc
        });

    let mut final_results: Vec<SearchResult> = Vec::new();
    for (score, items) in limited_results {
        // Ignore all results with a score equal to zero
        if score == 0 {
            continue;
        }

        // If there's only one entry for a score level, we can just add it to the final results
        if items.len() == 1 {
            final_results.push(items.first().unwrap().clone());
        } else {
            final_results.extend(break_a_tie(items, &matcher_instance));
        }
    }

    let count = result_count.min(final_results.len());
    let top_slice: &mut [SearchResult] = &mut final_results[..count];

    // After all processing is finished, download the relevant favicons.
    fetch_favicons(top_slice)?;

    Ok(final_results)
}

/// Get Chrome-based browser history
fn get_chrome_history(db_path: &Path) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Create a temporary copy of the database
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    // Get the ignored domains from environment
    let ignored_domains: Vec<String> = std::env::var("ignored_domains")
        .unwrap_or_default()
        .split(',')
        .map(String::from)
        .collect();

    // Build the query using SeaQuery
    // (urls.last_visit_time/1000000 + strftime('%s', '1601-01-01'))
    let last_visit_expr = Expr::expr(Expr::col(ChromeUrls::LastVisitTime).div(1000000)).add(
        Func::cust(Alias::new("strftime"))
            .arg("%s")
            .arg("1601-01-01"),
    );

    let query_builder = Query::select()
        .distinct()
        .columns([ChromeUrls::Url, ChromeUrls::Title, ChromeUrls::VisitCount])
        .expr_as(last_visit_expr, Alias::new("last_visit_time"))
        .from(ChromeUrls::Table)
        .from(ChromeVisits::Table)
        .and_where(Expr::col(ChromeUrls::Id).equals((ChromeVisits::Table, ChromeVisits::Url)))
        .and_where(Expr::col(ChromeUrls::Title).is_not_null())
        .and_where(Expr::col(ChromeUrls::Title).ne(""))
        .order_by(Alias::new("last_visit_time"), sea_query::Order::Desc)
        .to_owned();

    let (sql, values) = query_builder.build(SqliteQueryBuilder);

    // Execute query
    let results = execute_chrome_query(&conn, &sql, values)?;

    // Filter out ignored domains
    let filtered_results = results
        .into_iter()
        .filter(|result| {
            !ignored_domains
                .iter()
                .any(|domain| result.url.contains(domain))
        })
        .collect();

    Ok(filtered_results)
}

/// Get Safari history using the custom schema
fn get_safari_history(db_path: &Path) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Create a temporary copy of the database
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    // Build the query using SeaQuery
    // (visits.VISIT_TIME + 978307200)
    let last_visit_expr = Expr::col((SafariVisits::Table, SafariVisits::VisitTime)).add(978307200);

    let query_builder = Query::select()
        .columns([
            (HistoryItems::Table, HistoryItems::Url),
            (HistoryItems::Table, HistoryItems::Title),
            (HistoryItems::Table, HistoryItems::VisitCount),
        ])
        .expr_as(last_visit_expr, Alias::new("last_visit_time"))
        .from(HistoryItems::Table)
        .inner_join(
            SafariVisits::Table,
            Expr::col((SafariVisits::Table, SafariVisits::HistoryItemId))
                .equals((HistoryItems::Table, HistoryItems::Id)),
        )
        .and_where(Expr::col((HistoryItems::Table, HistoryItems::Url)).is_not_null())
        .and_where(Expr::col((HistoryItems::Table, HistoryItems::Url)).ne(""))
        .order_by(
            (HistoryItems::Table, HistoryItems::VisitCount),
            sea_query::Order::Desc,
        )
        .to_owned();

    let (sql, values) = query_builder.build(SqliteQueryBuilder);

    // Execute query
    let results = execute_safari_query(&conn, &sql, values, true)?;

    Ok(results)
}

/// Get Orion history
fn get_orion_history(db_path: &Path) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Create a temporary copy of the database
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    // Build the query using SeaQuery
    // (visits.VISIT_TIME + 978307200)
    let last_visit_expr = Expr::col((SafariVisits::Table, SafariVisits::VisitTime)).add(978307200);

    let query_builder = Query::select()
        .columns([
            (HistoryItems::Table, HistoryItems::Url),
            (HistoryItems::Table, HistoryItems::Title),
            (HistoryItems::Table, HistoryItems::VisitCount),
        ])
        .expr_as(last_visit_expr, Alias::new("last_visit_time"))
        .from(HistoryItems::Table)
        .inner_join(
            SafariVisits::Table,
            Expr::col((SafariVisits::Table, SafariVisits::HistoryItemId))
                .equals((HistoryItems::Table, HistoryItems::Id)),
        )
        .and_where(Expr::col((HistoryItems::Table, HistoryItems::Url)).is_not_null())
        .and_where(Expr::col((HistoryItems::Table, HistoryItems::Title)).is_not_null())
        .and_where(Expr::col((HistoryItems::Table, HistoryItems::Url)).ne(""))
        .order_by(
            (HistoryItems::Table, HistoryItems::VisitCount),
            sea_query::Order::Desc,
        )
        .to_owned();

    let (sql, values) = query_builder.build(SqliteQueryBuilder);

    // Execute query
    let results = execute_safari_query(&conn, &sql, values, false)?;

    Ok(results)
}

/// Get Firefox history
pub fn get_firefox_history(db_path: &Path) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Copy locked DB out of the way
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    // Build the query using SeaQuery
    // (moz_historyvisits.visit_date/1000000)
    let last_visit_expr =
        Expr::col((MozHistoryVisits::Table, MozHistoryVisits::VisitDate)).div(1000000);

    let query_builder = Query::select()
        .columns([
            (MozPlaces::Table, MozPlaces::Url),
            (MozPlaces::Table, MozPlaces::Title),
            (MozPlaces::Table, MozPlaces::VisitCount),
        ])
        .expr_as(last_visit_expr, Alias::new("last_visit_time"))
        .from(MozPlaces::Table)
        .left_join(
            MozHistoryVisits::Table,
            Expr::col((MozPlaces::Table, MozPlaces::Id))
                .equals((MozHistoryVisits::Table, MozHistoryVisits::PlaceId)),
        )
        .and_where(Expr::col((MozPlaces::Table, MozPlaces::Url)).is_not_null())
        .and_where(Expr::col((MozPlaces::Table, MozPlaces::Title)).is_not_null())
        .and_where(Expr::col((MozPlaces::Table, MozPlaces::Url)).ne(""))
        .order_by(Alias::new("last_visit_time"), sea_query::Order::Desc)
        .to_owned();

    let (sql, values) = query_builder.build(SqliteQueryBuilder);

    // Execute query
    let results = execute_chrome_query(&conn, &sql, values)?;

    Ok(results)
}

/// Execute Chrome/Firefox query and map results
fn execute_chrome_query(
    conn: &Connection,
    sql: &str,
    values: sea_query::Values,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let params = convert_values_to_params(values);
    let mut stmt = conn.prepare(sql)?;

    let results = stmt
        .query_map(params_from_iter(params.iter()), |row| map_chrome_row(row))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

/// Execute Safari/Orion query and map results
fn execute_safari_query(
    conn: &Connection,
    sql: &str,
    values: sea_query::Values,
    allow_missing_title: bool,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let params = convert_values_to_params(values);
    let mut stmt = conn.prepare(sql)?;

    let results = stmt
        .query_map(params_from_iter(params.iter()), |row| {
            map_safari_row(row, allow_missing_title)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

/// Map Chrome/Firefox row to SearchResult
fn map_chrome_row(row: &Row) -> rusqlite::Result<SearchResult> {
    let url: String = row.get(0)?;
    let title: String = row.get(1)?;
    let visit_count: i32 = row.get(2)?;
    let last_visit: i64 = row.get(3)?;

    // Format date based on user preference
    let date_format = std::env::var("date_format").unwrap_or("%d.%m.%Y".to_string());
    let dt = Timestamp::from_second(last_visit)
        .expect("Valid timestamp")
        .in_tz("UTC")
        .unwrap();
    let formatted_date = strtime::format(&date_format, &dt).unwrap();

    Ok(SearchResult {
        title,
        url,
        subtitle: format!("Last visit: {} (Visits: {})", formatted_date, visit_count),
        favicon: None,
        source: ResultSource::History,
        visit_count: Some(visit_count as u32),
        last_visit: Some(Timestamp::from_second(last_visit).expect("Valid timestamp")),
    })
}

/// Map Safari/Orion row to SearchResult
fn map_safari_row(row: &Row, allow_missing_title: bool) -> rusqlite::Result<SearchResult> {
    let url: String = row.get(0)?;
    let title: String = if allow_missing_title {
        row.get(1).unwrap_or_else(|_| url.clone())
    } else {
        row.get(1)?
    };
    let visit_count: i32 = row.get(2)?;
    let last_visit_f: f64 = row.get(3)?;
    let last_visit: i64 = last_visit_f as i64;

    // Format date based on user preference
    let date_format = std::env::var("date_format").unwrap_or("%d.%m.%Y".to_string());
    let dt = Timestamp::from_second(last_visit)
        .expect("Valid timestamp")
        .in_tz("UTC")
        .unwrap();
    let formatted_date = strtime::format(&date_format, &dt).unwrap();

    Ok(SearchResult {
        title,
        url,
        subtitle: format!("Last visit: {} (Visits: {})", formatted_date, visit_count),
        favicon: None,
        source: ResultSource::History,
        visit_count: Some(visit_count as u32),
        last_visit: Some(Timestamp::from_second(last_visit).expect("Valid timestamp")),
    })
}

/// Convert SeaQuery Values to rusqlite params
fn convert_values_to_params(values: sea_query::Values) -> Vec<rusqlite::types::Value> {
    values
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
        .collect()
}
