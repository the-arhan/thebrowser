//! Storage data models.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceRecord {
    pub id: String,
    pub name: String,
    pub accent_color: String,
    pub icon: Option<String>,
    pub position: i32,
    pub is_active: bool,
    pub active_tab_id: Option<String>,
}

impl Default for SpaceRecord {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Personal".to_string(),
            accent_color: "#6366f1".to_string(),
            icon: Some("user-symbolic".to_string()),
            position: 0,
            is_active: true,
            active_tab_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visit_count: i64,
    pub last_visit_time: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTab {
    pub id: String,
    pub url: String,
    pub title: String,
    pub position: i32,
    pub is_active: bool,
    pub is_pinned: bool,
    pub session_data: Option<Vec<u8>>,
    pub space_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shortcut {
    pub id: String,
    pub title: String,
    pub url: String,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSettings {
    pub theme: String,             // "dark", "light", "system"
    pub search_engine: String,      // "duckduckgo", "google", "startpage", "bing", "brave"
    pub home_url: String,           // default: "thebrowser://home"
    pub show_shortcuts: bool,
    pub show_recent: bool,
    pub sidebar_folded: bool,
    pub tab_layout: String,         // "vertical", "horizontal"
    pub dusk_dark_mode: bool,       // intelligent dark mode fallback
    pub tab_snooze_enabled: bool,
    pub tab_snooze_threshold_mins: u32,
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            search_engine: "duckduckgo".to_string(),
            home_url: "thebrowser://home".to_string(),
            show_shortcuts: true,
            show_recent: true,
            sidebar_folded: false,
            tab_layout: "vertical".to_string(),
            dusk_dark_mode: true,
            tab_snooze_enabled: true,
            tab_snooze_threshold_mins: 30,
        }
    }
}
