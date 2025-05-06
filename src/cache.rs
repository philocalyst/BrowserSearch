// src/cache.rs
use crate::search::SearchResult;
use dirs::data_dir;
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

/// Get the directory where cache files are stored
fn get_cache_dir() -> Option<PathBuf> {
    let data_dir = data_dir()?;
    let cache_dir = data_dir.join("browserSearch");

    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).ok()?;
    }

    Some(cache_dir)
}

/// Get the path to a specific cache file
fn get_cache_file(cache_type: &str) -> Option<PathBuf> {
    Some(get_cache_dir()?.join(format!("{}.cache", cache_type)))
}

/// Save search results to cache
pub fn save_to_cache(cache_type: &str, results: &[SearchResult]) -> Result<(), Box<dyn Error>> {
    if let Some(cache_file) = get_cache_file(cache_type) {
        let encoded = bincode::serialize(results)?;
        let mut file = File::create(cache_file)?;
        file.write_all(&encoded)?;
    }
    Ok(())
}

/// Get cached search results
pub fn get_cached_results(cache_type: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    if let Some(cache_file) = get_cache_file(cache_type) {
        if cache_file.exists() {
            let mut file = File::open(cache_file)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            let results: Vec<SearchResult> = bincode::deserialize(&buffer)?;
            return Ok(results);
        }
    }
    Ok(Vec::new())
}

/// Record the last time the cache was updated
pub fn update_cache_timestamp() -> Result<(), Box<dyn Error>> {
    if let Some(cache_dir) = get_cache_dir() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
            .to_string();

        let timestamp_file = cache_dir.join("last_updated.txt");
        let mut file = File::create(timestamp_file)?;
        file.write_all(timestamp.as_bytes())?;
    }
    Ok(())
}

/// Get the timestamp of the last cache update
pub fn get_cache_timestamp() -> Option<u64> {
    let cache_dir = get_cache_dir()?;
    let timestamp_file = cache_dir.join("last_updated.txt");

    if timestamp_file.exists() {
        let mut file = File::open(timestamp_file).ok()?;
        let mut timestamp = String::new();
        file.read_to_string(&mut timestamp).ok()?;

        return timestamp.parse().ok();
    }

    None
}
