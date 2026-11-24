//! Space models representing isolated browsing domains.

use crate::tab::TabId;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Strongly-typed identifier for a Space.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpaceId(pub String);

impl SpaceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SpaceId {
    fn default() -> Self {
        Self::from_string("default")
    }
}

impl fmt::Display for SpaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Metadata and state for an isolated Space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceInfo {
    pub id: SpaceId,
    pub name: String,
    pub accent_color: String,
    pub icon: Option<String>,
    pub position: i32,
    pub active_tab_id: Option<TabId>,
}

impl SpaceInfo {
    pub fn new(
        id: SpaceId,
        name: impl Into<String>,
        accent_color: impl Into<String>,
        icon: Option<String>,
        position: i32,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            accent_color: accent_color.into(),
            icon,
            position,
            active_tab_id: None,
        }
    }

    pub fn default_space() -> Self {
        Self {
            id: SpaceId::from_string("default"),
            name: "Personal".to_string(),
            accent_color: "#6366f1".to_string(), // Modern Indigo
            icon: Some("user-symbolic".to_string()),
            position: 0,
            active_tab_id: None,
        }
    }
}

