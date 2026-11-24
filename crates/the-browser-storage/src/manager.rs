//! SQLite database management and query interface.

use crate::error::StorageError;
use crate::models::{Bookmark, BrowserSettings, HistoryEntry, SessionTab, Shortcut, SpaceRecord};
use rusqlite::{Connection, params};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct StorageManager {
    conn: Connection,
}

impl StorageManager {
    /// Open a database at the specified file path, initializing schema if needed.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path_ref)?;
        let manager = Self { conn };
        manager.init_schema()?;
        manager.seed_default_shortcuts()?;
        Ok(manager)
    }

    /// Open an in-memory database (useful for isolated unit tests).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let manager = Self { conn };
        manager.init_schema()?;
        manager.seed_default_shortcuts()?;
        Ok(manager)
    }

    fn init_schema(&self) -> Result<(), StorageError> {
        // Enable WAL mode for concurrency and performance
        let _ = self.conn.pragma_update(None, "journal_mode", "WAL");
        let _ = self.conn.pragma_update(None, "synchronous", "NORMAL");

        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS history_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                visit_count INTEGER NOT NULL DEFAULT 1,
                last_visit_time INTEGER NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS history_fts USING fts5(
                title,
                url,
                content='history_entries',
                content_rowid='id'
            );

            CREATE TRIGGER IF NOT EXISTS trg_history_ai AFTER INSERT ON history_entries BEGIN
                INSERT INTO history_fts(rowid, title, url) VALUES (new.id, new.title, new.url);
            END;

            CREATE TRIGGER IF NOT EXISTS trg_history_au AFTER UPDATE ON history_entries BEGIN
                INSERT INTO history_fts(history_fts, rowid, title, url) VALUES('delete', old.id, old.title, old.url);
                INSERT INTO history_fts(rowid, title, url) VALUES (new.id, new.title, new.url);
            END;

            CREATE TRIGGER IF NOT EXISTS trg_history_ad AFTER DELETE ON history_entries BEGIN
                INSERT INTO history_fts(history_fts, rowid, title, url) VALUES('delete', old.id, old.title, old.url);
            END;

            CREATE TABLE IF NOT EXISTS spaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                accent_color TEXT NOT NULL,
                icon TEXT,
                position INTEGER NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 0,
                active_tab_id TEXT
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                title TEXT NOT NULL,
                position INTEGER NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 0,
                is_pinned BOOLEAN NOT NULL DEFAULT 0,
                session_data BLOB,
                space_id TEXT DEFAULT 'default'
            );

            CREATE TABLE IF NOT EXISTS bookmarks (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS shortcuts (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                position INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;

        // Apply backward-compatible migrations for sessions table
        let _ = self.conn.execute("ALTER TABLE sessions ADD COLUMN is_pinned BOOLEAN NOT NULL DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE sessions ADD COLUMN session_data BLOB", []);
        let _ = self.conn.execute("ALTER TABLE sessions ADD COLUMN space_id TEXT DEFAULT 'default'", []);

        Ok(())
    }

    /// Record a page visit, incrementing visit count and updating timestamp and title.
    pub fn record_visit(&self, url: &str, title: &str) -> Result<(), StorageError> {
        let trimmed_url = url.trim();
        if trimmed_url.is_empty()
            || trimmed_url == "about:blank"
            || trimmed_url.starts_with("thebrowser://")
            || trimmed_url.starts_with("about:")
        {
            return Ok(());
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let clean_title = if title.trim().is_empty() {
            trimmed_url
        } else {
            title.trim()
        };

        self.conn.execute(
            r#"
            INSERT INTO history_entries (url, title, visit_count, last_visit_time)
            VALUES (?1, ?2, 1, ?3)
            ON CONFLICT(url) DO UPDATE SET
                title = CASE WHEN excluded.title != '' THEN excluded.title ELSE history_entries.title END,
                visit_count = history_entries.visit_count + 1,
                last_visit_time = excluded.last_visit_time
            "#,
            params![trimmed_url, clean_title, now],
        )?;

        Ok(())
    }

    /// Search history using SQLite FTS5 full-text indexing.
    pub fn search_history(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, StorageError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            let mut stmt = self.conn.prepare(
                "SELECT id, url, title, visit_count, last_visit_time 
                 FROM history_entries 
                 ORDER BY last_visit_time DESC 
                 LIMIT ?1",
            )?;

            let rows = stmt.query_map(params![limit as i64], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    visit_count: row.get(3)?,
                    last_visit_time: row.get(4)?,
                })
            })?;

            let mut results = Vec::new();
            for item in rows {
                results.push(item?);
            }
            return Ok(results);
        }

        let fts_tokens: Vec<String> = trimmed
            .split_whitespace()
            .map(|word| {
                let clean = word.replace('"', "");
                format!("\"{clean}\"*")
            })
            .collect();

        let fts_query = fts_tokens.join(" ");

        let mut stmt = self.conn.prepare(
            r#"
            SELECT e.id, e.url, e.title, e.visit_count, e.last_visit_time
            FROM history_fts f
            JOIN history_entries e ON f.rowid = e.id
            WHERE history_fts MATCH ?1
            ORDER BY bm25(history_fts) ASC, e.visit_count DESC, e.last_visit_time DESC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![fts_query, limit as i64], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                visit_count: row.get(3)?,
                last_visit_time: row.get(4)?,
            })
        })?;

        let mut results = Vec::new();
        for item in rows {
            results.push(item?);
        }
        Ok(results)
    }

    /// Retrieve the most recent history entries up to limit.
    pub fn get_recent_history(&self, limit: usize) -> Result<Vec<HistoryEntry>, StorageError> {
        self.search_history("", limit)
    }

    /// Clear all history entries.
    pub fn clear_history(&self) -> Result<(), StorageError> {
        self.conn.execute("DELETE FROM history_entries", [])?;
        Ok(())
    }

    // --- Bookmarks Subsystem ---

    /// Add a new bookmark.
    pub fn add_bookmark(&self, url: &str, title: &str) -> Result<Bookmark, StorageError> {
        let clean_url = url.trim();
        let clean_title = if title.trim().is_empty() { clean_url } else { title.trim() };
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
        let id = Uuid::new_v4().to_string();

        self.conn.execute(
            r#"
            INSERT INTO bookmarks (id, url, title, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(url) DO UPDATE SET title = excluded.title
            "#,
            params![id, clean_url, clean_title, now],
        )?;

        Ok(Bookmark {
            id,
            url: clean_url.to_string(),
            title: clean_title.to_string(),
            created_at: now,
        })
    }

    /// Remove a bookmark by URL.
    pub fn remove_bookmark(&self, url: &str) -> Result<bool, StorageError> {
        let rows_affected = self.conn.execute("DELETE FROM bookmarks WHERE url = ?1", params![url.trim()])?;
        Ok(rows_affected > 0)
    }

    /// Check if a URL is bookmarked.
    pub fn is_bookmarked(&self, url: &str) -> Result<bool, StorageError> {
        let clean = url.trim();
        if clean.is_empty() || clean.starts_with("thebrowser://") || clean == "about:blank" {
            return Ok(false);
        }

        let mut stmt = self.conn.prepare("SELECT 1 FROM bookmarks WHERE url = ?1 LIMIT 1")?;
        let exists = stmt.exists(params![clean])?;
        Ok(exists)
    }

    /// Get all bookmarks ordered by creation time descending.
    pub fn get_bookmarks(&self) -> Result<Vec<Bookmark>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, created_at FROM bookmarks ORDER BY created_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for b in rows {
            list.push(b?);
        }
        Ok(list)
    }

    // --- Shortcuts Subsystem (New Tab Quick Access) ---

    /// Seed default shortcuts if the shortcuts table is empty.
    pub fn seed_default_shortcuts(&self) -> Result<(), StorageError> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM shortcuts")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;

        if count == 0 {
            let defaults = [
                ("Arch Linux", "https://archlinux.org", 0),
                ("DuckDuckGo", "https://duckduckgo.com", 1),
                ("GitHub", "https://github.com", 2),
                ("Rust Lang", "https://www.rust-lang.org", 3),
                ("Wikipedia", "https://www.wikipedia.org", 4),
            ];

            let mut insert_stmt = self.conn.prepare(
                "INSERT INTO shortcuts (id, title, url, position) VALUES (?1, ?2, ?3, ?4)"
            )?;

            for (title, url, pos) in defaults {
                let id = Uuid::new_v4().to_string();
                let _ = insert_stmt.execute(params![id, title, url, pos]);
            }
        }

        Ok(())
    }

    /// Add a new home shortcut.
    pub fn add_shortcut(&self, title: &str, url: &str) -> Result<Shortcut, StorageError> {
        let clean_title = title.trim();
        let clean_url = url.trim();
        let id = Uuid::new_v4().to_string();

        let mut count_stmt = self.conn.prepare("SELECT COALESCE(MAX(position), -1) + 1 FROM shortcuts")?;
        let next_pos: i32 = count_stmt.query_row([], |row| row.get(0))?;

        self.conn.execute(
            r#"
            INSERT INTO shortcuts (id, title, url, position)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(url) DO UPDATE SET title = excluded.title
            "#,
            params![id, clean_title, clean_url, next_pos],
        )?;

        Ok(Shortcut {
            id,
            title: clean_title.to_string(),
            url: clean_url.to_string(),
            position: next_pos,
        })
    }

    /// Remove a shortcut by ID.
    pub fn remove_shortcut(&self, id: &str) -> Result<bool, StorageError> {
        let rows = self.conn.execute("DELETE FROM shortcuts WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    /// Get all shortcuts ordered by position ascending.
    pub fn get_shortcuts(&self) -> Result<Vec<Shortcut>, StorageError> {
        let mut stmt = self.conn.prepare("SELECT id, title, url, position FROM shortcuts ORDER BY position ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(Shortcut {
                id: row.get(0)?,
                title: row.get(1)?,
                url: row.get(2)?,
                position: row.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for s in rows {
            list.push(s?);
        }
        Ok(list)
    }

    // --- Settings Subsystem ---

    /// Retrieve an individual setting by key.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Set an individual setting by key.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Load the full browser configuration settings object.
    pub fn load_browser_settings(&self) -> Result<BrowserSettings, StorageError> {
        let mut settings = BrowserSettings::default();

        if let Some(theme) = self.get_setting("theme")? {
            settings.theme = theme;
        }
        if let Some(engine) = self.get_setting("search_engine")? {
            settings.search_engine = engine;
        }
        if let Some(home) = self.get_setting("home_url")? {
            settings.home_url = home;
        }
        if let Some(show_sc) = self.get_setting("show_shortcuts")? {
            settings.show_shortcuts = show_sc == "true" || show_sc == "1";
        }
        if let Some(show_rc) = self.get_setting("show_recent")? {
            settings.show_recent = show_rc == "true" || show_rc == "1";
        }
        if let Some(folded) = self.get_setting("sidebar_folded")? {
            settings.sidebar_folded = folded == "true" || folded == "1";
        }
        if let Some(layout) = self.get_setting("tab_layout")? {
            settings.tab_layout = layout;
        }
        if let Some(dusk) = self.get_setting("dusk_dark_mode")? {
            settings.dusk_dark_mode = dusk == "true" || dusk == "1";
        }
        if let Some(snooze) = self.get_setting("tab_snooze_enabled")? {
            settings.tab_snooze_enabled = snooze == "true" || snooze == "1";
        }
        if let Some(thresh) = self.get_setting("tab_snooze_threshold_mins")?
            && let Ok(v) = thresh.parse::<u32>()
        {
            settings.tab_snooze_threshold_mins = v;
        }

        Ok(settings)
    }

    /// Save the full browser configuration settings object.
    pub fn save_browser_settings(&self, settings: &BrowserSettings) -> Result<(), StorageError> {
        self.set_setting("theme", &settings.theme)?;
        self.set_setting("search_engine", &settings.search_engine)?;
        self.set_setting("home_url", &settings.home_url)?;
        self.set_setting("show_shortcuts", if settings.show_shortcuts { "true" } else { "false" })?;
        self.set_setting("show_recent", if settings.show_recent { "true" } else { "false" })?;
        self.set_setting("sidebar_folded", if settings.sidebar_folded { "true" } else { "false" })?;
        self.set_setting("tab_layout", &settings.tab_layout)?;
        self.set_setting("dusk_dark_mode", if settings.dusk_dark_mode { "true" } else { "false" })?;
        self.set_setting("tab_snooze_enabled", if settings.tab_snooze_enabled { "true" } else { "false" })?;
        self.set_setting("tab_snooze_threshold_mins", &settings.tab_snooze_threshold_mins.to_string())?;
        Ok(())
    }

    // --- Spaces Subsystem ---

    /// Save or update list of Spaces.
    pub fn save_spaces(&self, spaces: &[SpaceRecord]) -> Result<(), StorageError> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO spaces (id, name, accent_color, icon, position, is_active, active_tab_id)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    accent_color = excluded.accent_color,
                    icon = excluded.icon,
                    position = excluded.position,
                    is_active = excluded.is_active,
                    active_tab_id = excluded.active_tab_id
                "#,
            )?;

            for sp in spaces {
                stmt.execute(params![
                    sp.id,
                    sp.name,
                    sp.accent_color,
                    sp.icon,
                    sp.position,
                    sp.is_active,
                    sp.active_tab_id,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Load all Spaces ordered by position. If empty, creates and returns the default Space.
    pub fn load_spaces(&self) -> Result<Vec<SpaceRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, accent_color, icon, position, is_active, active_tab_id FROM spaces ORDER BY position ASC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SpaceRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                accent_color: row.get(2)?,
                icon: row.get(3)?,
                position: row.get(4)?,
                is_active: row.get(5)?,
                active_tab_id: row.get(6)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }

        if list.is_empty() {
            let def = SpaceRecord::default();
            self.save_spaces(std::slice::from_ref(&def))?;
            list.push(def);
        }

        Ok(list)
    }

    /// Delete a Space and its associated tab sessions from SQLite.
    pub fn delete_space(&self, space_id: &str) -> Result<(), StorageError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM sessions WHERE space_id = ?1", params![space_id])?;
        tx.execute("DELETE FROM spaces WHERE id = ?1", params![space_id])?;
        tx.commit()?;
        Ok(())
    }

    // --- Session Persistence ---

    /// Save the current open tabs to the sessions table.
    pub fn save_session(&self, tabs: &[SessionTab]) -> Result<(), StorageError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM sessions", [])?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO sessions (id, url, title, position, is_active, is_pinned, session_data, space_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;

            for tab in tabs {
                stmt.execute(params![
                    tab.id,
                    tab.url,
                    tab.title,
                    tab.position,
                    tab.is_active,
                    tab.is_pinned,
                    tab.session_data,
                    tab.space_id.as_deref().unwrap_or("default"),
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Load the saved session tabs.
    pub fn load_session(&self) -> Result<Vec<SessionTab>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, position, is_active, is_pinned, session_data, space_id 
             FROM sessions 
             ORDER BY position ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SessionTab {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                position: row.get(3)?,
                is_active: row.get(4)?,
                is_pinned: row.get(5)?,
                session_data: row.get(6)?,
                space_id: row.get(7)?,
            })
        })?;

        let mut results = Vec::new();
        for item in rows {
            results.push(item?);
        }
        Ok(results)
    }

    /// Clear all stored session tabs.
    pub fn clear_session(&self) -> Result<(), StorageError> {
        self.conn.execute("DELETE FROM sessions", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_search_history() {
        let storage = StorageManager::open_in_memory().unwrap();

        storage.record_visit("https://archlinux.org", "Arch Linux").unwrap();
        storage.record_visit("https://rust-lang.org", "Rust Programming Language").unwrap();
        storage.record_visit("https://archlinux.org", "Arch Linux Portal").unwrap();

        let all = storage.search_history("", 10).unwrap();
        assert_eq!(all.len(), 2);

        let arch = all.iter().find(|e| e.url == "https://archlinux.org").unwrap();
        assert_eq!(arch.visit_count, 2);
        assert_eq!(arch.title, "Arch Linux Portal");

        let search_arch = storage.search_history("Arch", 10).unwrap();
        assert_eq!(search_arch.len(), 1);
        assert_eq!(search_arch[0].url, "https://archlinux.org");

        let search_rust = storage.search_history("Language", 10).unwrap();
        assert_eq!(search_rust.len(), 1);
        assert_eq!(search_rust[0].url, "https://rust-lang.org");

        // Edge case: blank or empty URLs should be safely ignored
        storage.record_visit("", "Empty").unwrap();
        storage.record_visit("about:blank", "Blank").unwrap();
        let after_blank = storage.search_history("", 10).unwrap();
        assert_eq!(after_blank.len(), 2);
    }

    #[test]
    fn test_bookmarks_management() {
        let storage = StorageManager::open_in_memory().unwrap();

        assert!(!storage.is_bookmarked("https://archlinux.org").unwrap());

        let b1 = storage.add_bookmark("https://archlinux.org", "Arch Linux").unwrap();
        assert_eq!(b1.url, "https://archlinux.org");
        assert!(storage.is_bookmarked("https://archlinux.org").unwrap());

        let bookmarks = storage.get_bookmarks().unwrap();
        assert_eq!(bookmarks.len(), 1);

        let removed = storage.remove_bookmark("https://archlinux.org").unwrap();
        assert!(removed);
        assert!(!storage.is_bookmarked("https://archlinux.org").unwrap());
    }

    #[test]
    fn test_shortcuts_management() {
        let storage = StorageManager::open_in_memory().unwrap();

        let initial_shortcuts = storage.get_shortcuts().unwrap();
        assert_eq!(initial_shortcuts.len(), 5); // 5 seeded defaults

        let added = storage.add_shortcut("GitHub Repo", "https://github.com/thebrowser").unwrap();
        assert_eq!(added.title, "GitHub Repo");

        let updated = storage.get_shortcuts().unwrap();
        assert_eq!(updated.len(), 6);

        let removed = storage.remove_shortcut(&added.id).unwrap();
        assert!(removed);
        assert_eq!(storage.get_shortcuts().unwrap().len(), 5);
    }

    #[test]
    fn test_settings_persistence() {
        let storage = StorageManager::open_in_memory().unwrap();

        let mut settings = storage.load_browser_settings().unwrap();
        assert_eq!(settings.theme, "dark");
        assert_eq!(settings.search_engine, "duckduckgo");

        settings.theme = "light".to_string();
        settings.search_engine = "google".to_string();
        settings.sidebar_folded = true;
        storage.save_browser_settings(&settings).unwrap();

        let reloaded = storage.load_browser_settings().unwrap();
        assert_eq!(reloaded.theme, "light");
        assert_eq!(reloaded.search_engine, "google");
        assert!(reloaded.sidebar_folded);
    }

    #[test]
    fn test_save_and_load_session() {
        let storage = StorageManager::open_in_memory().unwrap();

        let tabs = vec![
            SessionTab {
                id: "tab-1".to_string(),
                url: "https://archlinux.org".to_string(),
                title: "Arch Linux".to_string(),
                position: 0,
                is_active: false,
                is_pinned: true,
                session_data: Some(vec![1, 2, 3, 4]),
                space_id: Some("work".to_string()),
            },
            SessionTab {
                id: "tab-2".to_string(),
                url: "https://rust-lang.org".to_string(),
                title: "Rust".to_string(),
                position: 1,
                is_active: true,
                is_pinned: false,
                session_data: None,
                space_id: Some("personal".to_string()),
            },
        ];

        storage.save_session(&tabs).unwrap();
        let loaded = storage.load_session().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].url, "https://archlinux.org");
        assert!(loaded[0].is_pinned);
        assert_eq!(loaded[0].session_data, Some(vec![1, 2, 3, 4]));
        assert_eq!(loaded[0].space_id.as_deref(), Some("work"));

        assert_eq!(loaded[1].url, "https://rust-lang.org");
        assert!(loaded[1].is_active);
        assert!(!loaded[1].is_pinned);
        assert_eq!(loaded[1].session_data, None);

        storage.clear_session().unwrap();
        assert!(storage.load_session().unwrap().is_empty());
    }

    #[test]
    fn test_spaces_management() {
        let storage = StorageManager::open_in_memory().unwrap();

        // Default space auto-created on initial load
        let initial_spaces = storage.load_spaces().unwrap();
        assert_eq!(initial_spaces.len(), 1);
        assert_eq!(initial_spaces[0].id, "default");
        assert_eq!(initial_spaces[0].name, "Personal");

        let work_space = SpaceRecord {
            id: "work-space".to_string(),
            name: "Work Domain".to_string(),
            accent_color: "#06b6d4".to_string(),
            icon: Some("folder-symbolic".to_string()),
            position: 1,
            is_active: false,
            active_tab_id: Some("tab-work-1".to_string()),
        };

        storage.save_spaces(&[initial_spaces[0].clone(), work_space]).unwrap();

        let updated = storage.load_spaces().unwrap();
        assert_eq!(updated.len(), 2);
        assert_eq!(updated[1].id, "work-space");
        assert_eq!(updated[1].name, "Work Domain");

        // Save session tab associated with work space
        let tab = SessionTab {
            id: "tab-work-1".to_string(),
            url: "https://github.com".to_string(),
            title: "GitHub".to_string(),
            position: 0,
            is_active: true,
            is_pinned: false,
            session_data: None,
            space_id: Some("work-space".to_string()),
        };
        storage.save_session(&[tab]).unwrap();
        assert_eq!(storage.load_session().unwrap().len(), 1);

        // Delete space and ensure its sessions are cleaned up
        storage.delete_space("work-space").unwrap();
        let remaining_spaces = storage.load_spaces().unwrap();
        assert_eq!(remaining_spaces.len(), 1);
        assert_eq!(remaining_spaces[0].id, "default");
        assert_eq!(storage.load_session().unwrap().len(), 0);
    }
}
