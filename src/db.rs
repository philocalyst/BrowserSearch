//! Helpers for safely querying SQLite browser databases.
//!
//! - `create_temp_db_copy` to make a read-only tempfile copy,
//!   verifying integrity before use.
//! - `query_chrome_history` and `query_safari_history` wrappers to
//!   prepare, execute, and map query results.

use rusqlite::{Connection, Result as SqliteResult, Row};
use std::error::Error;
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;

/// Create a temporary copy of an SQLite database for safe reading
pub fn create_temp_db_copy(
    db_path: &Path,
    temp_dir: Option<&Path>,
    temp_filename_prefix: Option<&str>,
) -> Result<(NamedTempFile, Connection), Box<dyn Error>> {
    log::trace!("Begining the creation of the temporary database");
    // Define temp file path components

    let prefix = temp_filename_prefix.unwrap_or("db_temp");

    // Try to find an existing temp file with the prefix in the provided temp directory
    let temp_file = if let Some(dir) = temp_dir {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }

        // Check for existing temp files with our prefix
        let entries = fs::read_dir(dir)?;
        let mut existing_temp_path = None;

        // Iterate through canidates
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if file_name_str.starts_with(prefix) && entry.path().is_file() {
                // Found a match!
                existing_temp_path = Some(entry.path());
                break;
            }
        }

        if let Some(path) = existing_temp_path {
            // Check if we can read the existing temp file
            match fs::metadata(&path) {
                Ok(_) => {
                    let temp_file = NamedTempFile::new_in(dir)?;
                    let temp_path = temp_file.path().to_path_buf();
                    fs::copy(&path, &temp_path)?;
                    fs::remove_file(path)?; // Remove the old file we found
                    temp_file
                }
                Err(_) => {
                    // Couldn't access existing file, create a new one
                    NamedTempFile::with_prefix_in(prefix, dir)?
                }
            }
        } else {
            // No existing file found, create a new one
            NamedTempFile::with_prefix_in(prefix, dir)?
        }
    } else {
        // No specific directory specified, create a new temp file in the default location
        NamedTempFile::with_prefix(prefix)?
    };

    let temp_path = temp_file.path().to_path_buf();

    // Copy the database to the temporary file if needed
    // Compare file sizes to determine if copy is needed
    let db_metadata = fs::metadata(db_path)?;
    let temp_metadata = fs::metadata(&temp_path)?;

    if temp_metadata.len() != db_metadata.len() {
        // Different sizes, we need to copy
        fs::copy(db_path, &temp_path)?;
    } else {
        // Same size, but we should verify it's actually a valid SQLite database
        match Connection::open(&temp_path) {
            Ok(conn) => {
                // Try a simple query to verify integrity
                if conn
                    .query_row("PRAGMA integrity_check", [], |row| {
                        let result: String = row.get(0)?;
                        Ok(result)
                    })
                    .is_err()
                {
                    // Database seems corrupted, copy again
                    fs::copy(db_path, &temp_path)?;
                }
            }
            Err(_) => {
                // Couldn't open the database, copy again
                fs::copy(db_path, &temp_path)?;
            }
        }
    }

    // Connect to the temporary database
    let conn = Connection::open(&temp_path)?;
    Ok((temp_file, conn))
}

/// Execute a query on a Chrome-based history database
pub fn query_chrome_history<F, T>(
    conn: &Connection,
    query: &str,
    row_mapper: F,
) -> SqliteResult<Vec<T>>
where
    F: FnMut(&Row<'_>) -> SqliteResult<T>,
{
    let mut stmt = conn.prepare(query)?;
    let results = stmt
        .query_map([], row_mapper)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

/// Execute a query on a Safari history database
pub fn query_safari_history<F, T>(
    conn: &Connection,
    query: &str,
    row_mapper: F,
) -> SqliteResult<Vec<T>>
where
    F: FnMut(&Row<'_>) -> SqliteResult<T>,
{
    let mut stmt = conn.prepare(query)?;
    let results = stmt
        .query_map([], row_mapper)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

pub fn query_firefox_bookmarks<F, T>(
    conn: &Connection,
    sql: &str,
    mut row_mapper: F,
) -> SqliteResult<Vec<T>>
where
    F: FnMut(&Row<'_>) -> SqliteResult<T>,
{
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map([], |row| row_mapper(row))?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(rows)
}
