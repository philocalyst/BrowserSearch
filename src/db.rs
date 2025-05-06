// src/db.rs
use rusqlite::{Connection, Result as SqliteResult, Row};
use std::error::Error;
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;

/// Create a temporary copy of an SQLite database for safe reading
pub fn create_temp_db_copy(db_path: &Path) -> Result<(NamedTempFile, Connection), Box<dyn Error>> {
    // Create a temporary file
    let temp_file =
        NamedTempFile::new().map_err(|e| format!("Failed to create temporary file: {}", e))?;
    let temp_path = temp_file.path().to_path_buf();

    // Copy the database to the temporary file
    fs::copy(db_path, &temp_path)
        .map_err(|e| format!("Failed to copy database to temporary file: {}", e))?;

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
