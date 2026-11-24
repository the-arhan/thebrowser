//! SQLite storage engine for The Browser.

pub mod error;
pub mod manager;
pub mod models;

pub use error::StorageError;
pub use manager::StorageManager;
pub use models::{Bookmark, BrowserSettings, HistoryEntry, SessionTab, Shortcut, SpaceRecord};
