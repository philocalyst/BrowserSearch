//! Searches browser history across enabled browsers.
//!
//! - `search(query: &str)` coordinates loading cached results,
//!   reading each browser’s history via `get_chrome_history` /
//!   `get_safari_history`, merging, deduplicating, sorting, and
//!   limiting to MAX_RESULTS.
//! - After gathering, it calls `fetch_favicons` to populate icons.
use crate::browser::get_available_browsers;
use crate::cache::get_cached_results;
use crate::db::{create_temp_db_copy, query_chrome_history, query_safari_history};
use crate::search::{filter_results, ResultSource, SearchResult};
use crate::tie_break::break_a_tie;
use crate::utils::fetch_favicons;
use chrono::format::strftime;
use jiff::{fmt::strtime, tz::TimeZone, Timestamp};
use nucleo::{Matcher, Utf32Str};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::Path;

/// Searches browser history for the given query
pub fn search(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let browsers = get_available_browsers();

    // Get browser histories
    let browser_histories: Vec<Vec<SearchResult>> = browsers
        .par_iter()
        .filter_map(|(browser, paths)| {
            if let Some(history_path) = &paths.history {
                let result = match browser {
                    b if b.is_safari_like() => get_safari_history(history_path),
                    b if b.is_firefox_like() => get_firefox_history(history_path),
                    b if b.is_chrome_like() => get_chrome_history(history_path),
                    _ => unreachable!("unsupported browser: {:?}", browser),
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

    // |1| Pre-segment your query once:
    let mut query_buf: Vec<char> = Vec::new();
    let query_u32 = Utf32Str::new(query, &mut query_buf);

    // |2| Make a buffer to reuse for every title
    let mut title_buf: Vec<char> = Vec::new();

    use std::collections::HashMap;

    let limited_results: HashMap<u16, Vec<SearchResult>> = all_results
        .into_iter()
        .map(|item| {
            title_buf.clear(); // Clear any leftovers
            let title_u32 = Utf32Str::new(&item.title, &mut title_buf); // Convert to utf32
            let score = matcher_instance
                .fuzzy_match(title_u32, query_u32)
                .unwrap_or(u16::MIN); // If no match is found, fallback to the smallest value of a u16 (0)
            (score, item)
        })
        .take(result_count) // We take here because it's a local maximum, which means it reduces overhead while also maximizing results even if other components have no results to give.
        .fold(HashMap::new(), |mut acc, (score, item)| {
            acc.entry(score).or_default().push(item);
            acc
        });

    let mut final_results: Vec<SearchResult> = Vec::new();
    for score_level in limited_results {
        let score = score_level.0;
        let items = score_level.1;

        // Ignore all results with a score equal to zero
        if score <= 0 {
            continue;
        }

        // If there's only one entry for a score level, we can just add it to the final results
        if items.len() == 1 {
            final_results.push(items.first().unwrap().clone());
        } else {
            final_results.extend(break_a_tie(items, &matcher_instance));
        }
    }

    // After all processing is finished, download the relevant favicons.
    fetch_favicons(&mut final_results)?;

    Ok(final_results)
}

/// Get Chrome-based browser history
fn get_chrome_history(db_path: &Path) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Create a temporary copy of the database
    let (_temp_file, conn) = create_temp_db_copy(db_path, None, None)?;

    // Get the ignored domains from environment
    let ignored_domains: Vec<String> = std::env::var("ignored_domains")
        .unwrap_or_default()
        .split(',')
        .map(String::from)
        .collect();

    // Query the database
    let sql = "SELECT DISTINCT urls.url, urls.title, urls.visit_count,
         (urls.last_visit_time/1000000 + strftime('%s', '1601-01-01')) AS last_visit_time
         FROM urls, visits
         WHERE urls.id = visits.url AND
         urls.title IS NOT NULL AND
         urls.title != ''
         ORDER BY last_visit_time DESC";

    let results = query_chrome_history(&conn, sql, |row| {
        let url: String = row.get(0)?;
        let title: String = row.get(1)?;
        let visit_count: i32 = row.get(2)?;
        let last_visit: i64 = row.get(3)?;

        // Format date based on user preference
        let date_format = std::env::var("date_format").unwrap_or("%d.%m.%Y".to_string());
        let dt = Timestamp::from_second(last_visit)
            .expect("We know this is correct")
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
            last_visit: Some(
                Timestamp::from_second(last_visit).expect("The timestamp should be correct"),
            ),
        })
    })?;

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

/// Get Safari history
fn get_safari_history(db_path: &Path) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Create a temporary copy of the database
    let (_temp_file, conn) = create_temp_db_copy(db_path, None, None)?;

    // Query the database
    let sql = "SELECT history_items.url, history_visits.title, history_items.visit_count,
         (history_visits.visit_time + 978307200) AS last_visit_time
         FROM history_items
         INNER JOIN history_visits
         ON history_visits.history_item = history_items.id
         WHERE history_items.url IS NOT NULL AND
         history_visits.title IS NOT NULL AND
         history_items.url != ''
         ORDER BY visit_count DESC";

    let results = query_safari_history(&conn, sql, |row| {
        let url: String = row.get(0)?;
        let title: String = row.get(1)?;
        let visit_count: i32 = row.get(2)?;
        let last_visit_f: f64 = row.get(3)?;
        let last_visit: i64 = last_visit_f as i64;

        // Format date based on user preference
        let date_format = std::env::var("date_format").unwrap_or("%d.%m.%Y".to_string());
        let dt = Timestamp::from_second(last_visit)
            .expect("We know this is correct")
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
            last_visit: Some(
                Timestamp::from_second(last_visit).expect("The timestamp should be correct"),
            ),
        })
    })?;

    Ok(results)
}

/// Get Firefox history
pub fn get_firefox_history(db_path: &Path) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // copy locked DB out of the way
    let (_tmpfile, conn) = create_temp_db_copy(db_path, None, None)?;

    let sql = r#"
        SELECT
            moz_places.url,
            moz_places.title,
            moz_places.visit_count,
            (moz_historyvisits.visit_date/1000000) AS last_visit_time
        FROM moz_places
        LEFT JOIN moz_historyvisits
            ON moz_places.id = moz_historyvisits.place_id
        WHERE
            moz_places.url   IS NOT NULL
            AND moz_places.title IS NOT NULL
            AND moz_places.url   != ''
        ORDER BY last_visit_time DESC
    "#;

    // Reusing the chrome opperation because of the overlap
    let results = query_chrome_history(&conn, sql, |row| {
        let url: String = row.get(0)?;
        let title: String = row.get(1)?;
        let visit_count: i32 = row.get(2)?;
        let last_visit: i64 = row.get(3)?;

        // format date by user‐configured strftime
        let date_format = std::env::var("date_format").unwrap_or_else(|_| "%d.%m.%Y".into());
        let dt = Timestamp::from_second(last_visit)
            .expect("We know this is correct")
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
            last_visit: Some(
                Timestamp::from_second(last_visit).expect("The timestamp should be correct"),
            ),
        })
    })?;

    Ok(results)
}
