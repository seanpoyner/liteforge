//! Usage tracking system for Forge CLI.

pub mod db;

pub use db::{RequestRecord, UsageDb};

use std::path::PathBuf;

/// Get the default usage database path.
pub fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("forge")
        .join("usage.db")
}

/// Ensure the database directory exists.
pub fn ensure_db_dir() -> std::io::Result<PathBuf> {
    let path = default_db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(path)
}
