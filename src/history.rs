// src/history.rs
use crate::browser::{get_available_browsers, Browser};
use crate::cache::get_cached_results;
use crate::db::{create_temp_db_copy, query_chrome_history, query_safari_history};
use crate::search::{filter_results, ResultSource, SearchResult};
use crate::utils::fetch_favicons;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rayon::prelude::*;
use std::collections::HashSet;
use std::error::Error;

/// Searches browser history for the given query
pub fn search(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let browsers = get_available_browsers();

    // Get cached results if available
    let cached_results = match get_cached_results("history") {
        Ok(results) => results,
        Err(_) => Vec::new(),
    };

    // Get browser histories
    let browser_histories: Vec<Vec<SearchResult>> = browsers
        .par_iter()
        .filter_map(|(browser, paths)| {
            if let Some(history_path) = &paths.history {
                let result = match browser {
                    Browser::Safari => get_safari_history(history_path),
                    _ => get_chrome_history(history_path),
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

    // Add cached results first (if they match the query)
    for result in filter_results(cached_results, query) {
        if seen_urls.insert(result.url.clone()) {
            all_results.push(result);
        }
    }

    // Add new results from browsers
    for results in browser_histories {
        for result in results {
            if seen_urls.insert(result.url.clone()) {
                all_results.push(result);
            }
        }
    }

    // Sort by last visit (most recent first)
    all_results.sort_by(|a, b| b.last_visit.unwrap_or(0).cmp(&a.last_visit.unwrap_or(0)));

    // Limit to 30 results
    let mut limited_results: Vec<SearchResult> = all_results.into_iter().take(30).collect();

    // After all processing is finished, download the favicons.
    fetch_favicons(&mut limited_results)?;

    Ok(limited_results)
}

/// Get Chrome-based browser history
fn get_chrome_history(db_path: &std::path::Path) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Create a temporary copy of the database
    let (_temp_file, conn) = create_temp_db_copy(db_path)?;

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
        let dt = Utc.timestamp_opt(last_visit, 0).unwrap();
        let formatted_date = dt.format(&date_format).to_string();

        Ok(SearchResult {
            title,
            url,
            subtitle: format!("Last visit: {} (Visits: {})", formatted_date, visit_count),
            favicon: None,
            source: ResultSource::History,
            visit_count: Some(visit_count as u32),
            last_visit: Some(last_visit),
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
fn get_safari_history(db_path: &std::path::Path) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Create a temporary copy of the database
    let (_temp_file, conn) = create_temp_db_copy(db_path)?;

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
        let dt = Utc.timestamp_opt(last_visit, 0).unwrap();
        let formatted_date = dt.format(&date_format).to_string();

        Ok(SearchResult {
            title,
            url,
            subtitle: format!("Last visit: {} (Visits: {})", formatted_date, visit_count),
            favicon: None,
            source: ResultSource::History,
            visit_count: Some(visit_count as u32),
            last_visit: Some(last_visit),
        })
    })?;

    Ok(results)
}
