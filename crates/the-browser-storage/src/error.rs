//! Storage error definitions.

use std::fmt;

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Serialization(serde_json::Error),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(err) => write!(f, "SQLite error: {err}"),
            Self::Io(err) => write!(f, "IO error: {err}"),
            Self::Serialization(err) => write!(f, "Serialization error: {err}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlite(err) => Some(err),
            Self::Io(err) => Some(err),
            Self::Serialization(err) => Some(err),
        }
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}

