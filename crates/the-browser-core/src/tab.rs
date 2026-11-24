//! Tab state models, lifecycle transitions, and space-aware tab management.

use crate::space::{SpaceId, SpaceInfo};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Strongly-typed unique identifier for a browser tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub Uuid);

impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(TabId)
    }
}

impl std::str::FromStr for TabId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(TabId)
    }
}

impl From<Uuid> for TabId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TabId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Explicit lifecycle states for memory and activity management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TabLifecycleState {
    Active,
    #[default]
    LoadedBackground,
    EligibleForSnoozing,
    Sleeping,
    Restoring,
    FailedRestoration,
}

/// Metadata and runtime state for a single browser tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: TabId,
    pub space_id: SpaceId,
    pub url: String,
    pub title: String,
    pub is_loading: bool,
    pub load_progress: f64,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_pinned: bool,
    pub is_lazy: bool,
    pub lifecycle_state: TabLifecycleState,
    pub load_failed: bool,
    pub error_message: Option<String>,
    pub session_data: Option<Vec<u8>>,
    pub last_active_time: i64,
    pub is_playing_audio: bool,
    pub has_active_downloads: bool,
}

impl TabInfo {
    pub fn new(url: impl Into<String>, title: impl Into<String>) -> Self {
        Self::new_with_space(SpaceId::default(), url, title)
    }

    pub fn new_with_space(space_id: SpaceId, url: impl Into<String>, title: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Self {
            id: TabId::new(),
            space_id,
            url: url.into(),
            title: title.into(),
            is_loading: false,
            load_progress: 0.0,
            can_go_back: false,
            can_go_forward: false,
            is_pinned: false,
            is_lazy: false,
            lifecycle_state: TabLifecycleState::LoadedBackground,
            load_failed: false,
            error_message: None,
            session_data: None,
            last_active_time: now,
            is_playing_audio: false,
            has_active_downloads: false,
        }
    }

    pub fn is_sleeping(&self) -> bool {
        self.lifecycle_state == TabLifecycleState::Sleeping || self.is_lazy
    }

    pub fn set_sleeping(&mut self, sleeping: bool) {
        self.is_lazy = sleeping;
        if sleeping {
            self.lifecycle_state = TabLifecycleState::Sleeping;
        } else if self.lifecycle_state == TabLifecycleState::Sleeping {
            self.lifecycle_state = TabLifecycleState::LoadedBackground;
        }
    }

    pub fn display_title(&self) -> &str {
        if !self.title.trim().is_empty() {
            &self.title
        } else if !self.url.trim().is_empty() && self.url != "about:blank" {
            &self.url
        } else {
            "New Tab"
        }
    }

    /// Extract hostname portion from the URL for filtering and presentation.
    pub fn hostname(&self) -> &str {
        let trimmed = self.url.trim();
        if let Some(rest) = trimmed.strip_prefix("https://") {
            rest.split(['/', '?', '#', ':']).next().unwrap_or(rest)
        } else if let Some(rest) = trimmed.strip_prefix("http://") {
            rest.split(['/', '?', '#', ':']).next().unwrap_or(rest)
        } else if let Some(rest) = trimmed.strip_prefix("thebrowser://") {
            rest.split(['/', '?', '#']).next().unwrap_or(rest)
        } else {
            trimmed.split(['/', '?', '#', ':']).next().unwrap_or(trimmed)
        }
    }

    /// Check if tab matches a search query across title, hostname, and URL.
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        self.title.to_lowercase().contains(&q)
            || self.url.to_lowercase().contains(&q)
            || self.hostname().to_lowercase().contains(&q)
    }
}

/// In-memory tab manager coordinating spaces, tab collections, and active selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabManager {
    spaces: Vec<SpaceInfo>,
    active_space_id: Option<SpaceId>,
    tabs: Vec<TabInfo>,
    active_index: Option<usize>,
    closed_tabs: Vec<TabInfo>,
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TabManager {
    pub fn new() -> Self {
        let default_sp = SpaceInfo::default_space();
        let active_sp_id = default_sp.id.clone();
        Self {
            spaces: vec![default_sp],
            active_space_id: Some(active_sp_id),
            tabs: Vec::new(),
            active_index: None,
            closed_tabs: Vec::new(),
        }
    }

    // --- Space Management ---

    pub fn spaces(&self) -> &[SpaceInfo] {
        &self.spaces
    }

    pub fn spaces_mut(&mut self) -> &mut Vec<SpaceInfo> {
        &mut self.spaces
    }

    pub fn active_space_id(&self) -> Option<&SpaceId> {
        self.active_space_id.as_ref()
    }

    pub fn active_space(&self) -> Option<&SpaceInfo> {
        let id = self.active_space_id.as_ref()?;
        self.spaces.iter().find(|s| &s.id == id)
    }

    pub fn active_space_mut(&mut self) -> Option<&mut SpaceInfo> {
        let id = self.active_space_id.clone()?;
        self.spaces.iter_mut().find(|s| s.id == id)
    }

    pub fn get_space(&self, id: &SpaceId) -> Option<&SpaceInfo> {
        self.spaces.iter().find(|s| &s.id == id)
    }

    pub fn get_space_mut(&mut self, id: &SpaceId) -> Option<&mut SpaceInfo> {
        self.spaces.iter_mut().find(|s| &s.id == id)
    }

    pub fn add_space(&mut self, space: SpaceInfo) -> SpaceId {
        let id = space.id.clone();
        self.spaces.push(space);
        id
    }

    pub fn remove_space(&mut self, id: &SpaceId) -> (Option<SpaceInfo>, Vec<TabInfo>) {
        let space_idx = match self.spaces.iter().position(|s| &s.id == id) {
            Some(idx) => idx,
            None => return (None, Vec::new()),
        };

        let removed_space = self.spaces.remove(space_idx);

        // Remove all tabs belonging to this space
        let mut removed_tabs = Vec::new();
        let mut remaining_tabs = Vec::new();
        for tab in self.tabs.drain(..) {
            if &tab.space_id == id {
                removed_tabs.push(tab);
            } else {
                remaining_tabs.push(tab);
            }
        }
        self.tabs = remaining_tabs;

        // If the removed space was active, switch to first available space
        if self.active_space_id.as_ref() == Some(id) {
            if let Some(first) = self.spaces.first() {
                let first_id = first.id.clone();
                self.select_space(&first_id);
            } else {
                self.active_space_id = None;
                self.active_index = None;
            }
        }

        (Some(removed_space), removed_tabs)
    }

    pub fn select_space(&mut self, id: &SpaceId) -> bool {
        if !self.spaces.iter().any(|s| &s.id == id) {
            return false;
        }

        // Save current active tab in current space before switching
        if let Some(curr_sp_id) = &self.active_space_id {
            let active_tid = self.active_tab_id();
            if let Some(curr_sp) = self.spaces.iter_mut().find(|s| &s.id == curr_sp_id) {
                curr_sp.active_tab_id = active_tid;
            }
        }

        self.active_space_id = Some(id.clone());

        // Restore target space's active tab or pick first tab in that space
        let target_tab_id = self
            .spaces
            .iter()
            .find(|s| &s.id == id)
            .and_then(|s| s.active_tab_id);

        if let Some(idx) = target_tab_id.and_then(|tid| self.tabs.iter().position(|t| t.id == tid && &t.space_id == id)) {
            self.active_index = Some(idx);
            return true;
        }

        // Fallback: select first tab belonging to this space
        if let Some(idx) = self.tabs.iter().position(|t| &t.space_id == id) {
            self.active_index = Some(idx);
            let tid = self.tabs[idx].id;
            if let Some(sp) = self.spaces.iter_mut().find(|s| &s.id == id) {
                sp.active_tab_id = Some(tid);
            }
        } else {
            self.active_index = None;
        }

        true
    }

    pub fn update_space(
        &mut self,
        id: &SpaceId,
        name: Option<String>,
        accent_color: Option<String>,
        icon: Option<String>,
    ) -> bool {
        if let Some(space) = self.get_space_mut(id) {
            if let Some(n) = name {
                space.name = n;
            }
            if let Some(c) = accent_color {
                space.accent_color = c;
            }
            if icon.is_some() {
                space.icon = icon;
            }
            true
        } else {
            false
        }
    }

    // --- Tab Queries Filtered by Space ---

    pub fn tabs_for_space(&self, space_id: &SpaceId) -> Vec<&TabInfo> {
        self.tabs.iter().filter(|t| &t.space_id == space_id).collect()
    }

    pub fn active_space_tabs(&self) -> Vec<&TabInfo> {
        match &self.active_space_id {
            Some(sp_id) => self.tabs_for_space(sp_id),
            None => Vec::new(),
        }
    }

    pub fn pinned_tabs_for_space(&self, space_id: &SpaceId) -> Vec<(usize, &TabInfo)> {
        self.tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| &t.space_id == space_id && t.is_pinned)
            .collect()
    }

    pub fn unpinned_tabs_for_space(&self, space_id: &SpaceId) -> Vec<(usize, &TabInfo)> {
        self.tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| &t.space_id == space_id && !t.is_pinned)
            .collect()
    }

    pub fn active_space_pinned_tabs(&self) -> Vec<(usize, &TabInfo)> {
        match &self.active_space_id {
            Some(sp_id) => self.pinned_tabs_for_space(sp_id),
            None => Vec::new(),
        }
    }

    pub fn active_space_unpinned_tabs(&self) -> Vec<(usize, &TabInfo)> {
        match &self.active_space_id {
            Some(sp_id) => self.unpinned_tabs_for_space(sp_id),
            None => Vec::new(),
        }
    }

    // --- Tab Lifecycle & State ---

    pub fn closed_tab_count(&self) -> usize {
        self.closed_tabs.len()
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn tabs(&self) -> &[TabInfo] {
        &self.tabs
    }

    pub fn active_index(&self) -> Option<usize> {
        self.active_index
    }

    pub fn active_tab(&self) -> Option<&TabInfo> {
        self.active_index.and_then(|idx| self.tabs.get(idx))
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut TabInfo> {
        self.active_index.and_then(|idx| self.tabs.get_mut(idx))
    }

    pub fn active_tab_id(&self) -> Option<TabId> {
        self.active_tab().map(|t| t.id)
    }

    /// Open a new tab in the active space at the end of the tab row, setting it as active.
    pub fn open_tab(&mut self, url: impl Into<String>, title: impl Into<String>) -> TabId {
        let space_id = self.active_space_id.clone().unwrap_or_default();
        self.open_tab_in_space(&space_id, url, title)
    }

    /// Open a new tab in a specific space.
    pub fn open_tab_in_space(
        &mut self,
        space_id: &SpaceId,
        url: impl Into<String>,
        title: impl Into<String>,
    ) -> TabId {
        let mut tab = TabInfo::new_with_space(space_id.clone(), url, title);
        tab.lifecycle_state = TabLifecycleState::Active;
        let id = tab.id;

        // Mark previously active tab as LoadedBackground
        if let Some(curr_act) = self.active_tab_mut()
            && curr_act.lifecycle_state == TabLifecycleState::Active
        {
            curr_act.lifecycle_state = TabLifecycleState::LoadedBackground;
        }

        self.tabs.push(tab);
        self.active_index = Some(self.tabs.len() - 1);

        if let Some(sp) = self.spaces.iter_mut().find(|s| &s.id == space_id) {
            sp.active_tab_id = Some(id);
        }

        id
    }

    /// Open a new tab at a specific index, setting it as active.
    pub fn open_tab_at(
        &mut self,
        index: usize,
        url: impl Into<String>,
        title: impl Into<String>,
    ) -> TabId {
        let space_id = self.active_space_id.clone().unwrap_or_default();
        let mut tab = TabInfo::new_with_space(space_id.clone(), url, title);
        tab.lifecycle_state = TabLifecycleState::Active;
        let id = tab.id;
        let insert_idx = index.min(self.tabs.len());

        if let Some(curr_act) = self.active_tab_mut()
            && curr_act.lifecycle_state == TabLifecycleState::Active
        {
            curr_act.lifecycle_state = TabLifecycleState::LoadedBackground;
        }

        self.tabs.insert(insert_idx, tab);
        self.active_index = Some(insert_idx);

        if let Some(sp) = self.spaces.iter_mut().find(|s| s.id == space_id) {
            sp.active_tab_id = Some(id);
        }

        id
    }

    /// Close a tab by its ID. Returns the new active TabId if any tabs remain.
    pub fn close_tab(&mut self, id: TabId) -> Option<TabId> {
        let index = self.tabs.iter().position(|t| t.id == id)?;
        self.close_tab_at(index)
    }

    /// Close a tab by its index. Returns the new active TabId if any tabs remain.
    pub fn close_tab_at(&mut self, index: usize) -> Option<TabId> {
        if index >= self.tabs.len() {
            return self.active_tab_id();
        }

        let removed = self.tabs.remove(index);
        let removed_space_id = removed.space_id.clone();

        // Track closed tab (max 30 items)
        if self.closed_tabs.len() >= 30 {
            self.closed_tabs.remove(0);
        }
        self.closed_tabs.push(removed);

        if self.tabs.is_empty() {
            self.active_index = None;
            for sp in &mut self.spaces {
                sp.active_tab_id = None;
            }
            return None;
        }

        // Adjust active index within active space if possible
        let space_tabs: Vec<usize> = self
            .tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| t.space_id == removed_space_id)
            .map(|(i, _)| i)
            .collect();

        if space_tabs.is_empty() {
            if let Some(sp) = self.spaces.iter_mut().find(|s| s.id == removed_space_id) {
                sp.active_tab_id = None;
            }
            // Fallback to any remaining tab
            self.active_index = Some(0);
        } else {
            // Pick adjacent tab in the same space
            let candidate_idx = space_tabs
                .iter()
                .copied()
                .find(|&i| i >= index)
                .or_else(|| space_tabs.last().copied())
                .unwrap_or(0);

            self.active_index = Some(candidate_idx);
            let new_tid = self.tabs[candidate_idx].id;
            if let Some(sp) = self.spaces.iter_mut().find(|s| s.id == removed_space_id) {
                sp.active_tab_id = Some(new_tid);
            }
        }

        self.active_tab_id()
    }

    pub fn pop_closed_tab(&mut self) -> Option<TabInfo> {
        self.closed_tabs.pop()
    }

    pub fn reopen_closed_tab(&mut self) -> Option<TabInfo> {
        let last_closed = self.closed_tabs.pop()?;
        let space_id = last_closed.space_id.clone();
        let new_id = self.open_tab_in_space(&space_id, last_closed.url, last_closed.title);
        self.get_tab(new_id).cloned()
    }

    pub fn duplicate_tab(&mut self, id: TabId) -> Option<TabId> {
        let (index, space_id, url, title) = {
            let idx = self.tabs.iter().position(|t| t.id == id)?;
            let tab = &self.tabs[idx];
            (idx, tab.space_id.clone(), tab.url.clone(), tab.title.clone())
        };
        let mut new_tab = TabInfo::new_with_space(space_id, url, title);
        new_tab.lifecycle_state = TabLifecycleState::Active;
        let new_id = new_tab.id;

        self.tabs.insert(index + 1, new_tab);
        self.active_index = Some(index + 1);
        Some(new_id)
    }

    pub fn close_other_tabs(&mut self, id: TabId) -> Vec<TabId> {
        let mut closed_ids = Vec::new();
        let target_tab = match self.tabs.iter().find(|t| t.id == id).cloned() {
            Some(t) => t,
            None => return closed_ids,
        };

        let target_space = target_tab.space_id.clone();
        let mut remaining = Vec::new();

        for tab in self.tabs.drain(..) {
            if tab.id == id || tab.space_id != target_space {
                remaining.push(tab);
            } else {
                closed_ids.push(tab.id);
                if self.closed_tabs.len() >= 30 {
                    self.closed_tabs.remove(0);
                }
                self.closed_tabs.push(tab);
            }
        }

        self.tabs = remaining;
        self.active_index = self.tabs.iter().position(|t| t.id == id);
        closed_ids
    }

    pub fn close_tabs_to_right(&mut self, id: TabId) -> Vec<TabId> {
        let mut closed_ids = Vec::new();
        let (index, space_id) = match self.tabs.iter().position(|t| t.id == id) {
            Some(idx) => (idx, self.tabs[idx].space_id.clone()),
            None => return closed_ids,
        };

        let mut i = index + 1;
        while i < self.tabs.len() {
            if self.tabs[i].space_id == space_id {
                let removed = self.tabs.remove(i);
                closed_ids.push(removed.id);
                if self.closed_tabs.len() >= 30 {
                    self.closed_tabs.remove(0);
                }
                self.closed_tabs.push(removed);
            } else {
                i += 1;
            }
        }

        self.active_index = self.tabs.iter().position(|t| t.id == id);
        closed_ids
    }

    pub fn toggle_pin(&mut self, id: TabId) -> bool {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.is_pinned = !tab.is_pinned;
            tab.is_pinned
        } else {
            false
        }
    }

    pub fn set_pinned(&mut self, id: TabId, is_pinned: bool) -> bool {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.is_pinned = is_pinned;
            true
        } else {
            false
        }
    }

    pub fn restore_tab(&mut self, tab: TabInfo, set_active: bool) -> TabId {
        let id = tab.id;
        let space_id = tab.space_id.clone();
        self.tabs.push(tab);
        if set_active || self.active_index.is_none() {
            self.active_index = Some(self.tabs.len() - 1);
            if let Some(sp) = self.spaces.iter_mut().find(|s| s.id == space_id) {
                sp.active_tab_id = Some(id);
            }
        }
        id
    }

    /// Move tab from index `from` to `to` across the global list.
    pub fn move_tab(&mut self, from: usize, to: usize) -> bool {
        let count = self.tabs.len();
        if from >= count || to >= count || from == to {
            return false;
        }

        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);

        // Adjust active index
        if let Some(curr) = self.active_index {
            if curr == from {
                self.active_index = Some(to);
            } else if from < curr && curr <= to {
                self.active_index = Some(curr - 1);
            } else if to <= curr && curr < from {
                self.active_index = Some(curr + 1);
            }
        }

        true
    }

    /// Move tab position within a specific space.
    pub fn move_tab_in_space(
        &mut self,
        space_id: &SpaceId,
        from_space_idx: usize,
        to_space_idx: usize,
    ) -> bool {
        let space_tab_indices: Vec<usize> = self
            .tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| &t.space_id == space_id)
            .map(|(i, _)| i)
            .collect();

        if from_space_idx >= space_tab_indices.len()
            || to_space_idx >= space_tab_indices.len()
            || from_space_idx == to_space_idx
        {
            return false;
        }

        let global_from = space_tab_indices[from_space_idx];
        let global_to = space_tab_indices[to_space_idx];

        self.move_tab(global_from, global_to)
    }

    /// Transfer a tab to a different Space.
    pub fn move_tab_to_space(
        &mut self,
        tab_id: TabId,
        target_space_id: &SpaceId,
        target_index: Option<usize>,
    ) -> bool {
        if !self.spaces.iter().any(|s| &s.id == target_space_id) {
            return false;
        }

        let from_idx = match self.tabs.iter().position(|t| t.id == tab_id) {
            Some(i) => i,
            None => return false,
        };

        let mut tab = self.tabs.remove(from_idx);
        tab.space_id = target_space_id.clone();

        // Calculate insertion index
        let target_space_indices: Vec<usize> = self
            .tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| &t.space_id == target_space_id)
            .map(|(i, _)| i)
            .collect();

        let insert_idx = match target_index {
            Some(t_idx) if t_idx < target_space_indices.len() => target_space_indices[t_idx],
            _ => self.tabs.len(),
        };

        self.tabs.insert(insert_idx, tab);

        // Update active index
        self.active_index = self.tabs.iter().position(|t| t.id == tab_id);

        if let Some(sp) = self.spaces.iter_mut().find(|s| &s.id == target_space_id) {
            sp.active_tab_id = Some(tab_id);
        }

        true
    }

    // --- Tab Snoozing Lifecycle Operations ---

    /// Determine if a tab is eligible for intelligent automatic snoozing.
    pub fn check_snooze_eligibility(&self, tab_id: TabId, threshold_seconds: u64) -> bool {
        let tab = match self.get_tab(tab_id) {
            Some(t) => t,
            None => return false,
        };

        // Protected if active tab in active space
        if Some(tab.id) == self.active_tab_id() {
            return false;
        }

        // Must be in loaded background state
        if tab.lifecycle_state != TabLifecycleState::LoadedBackground && tab.lifecycle_state != TabLifecycleState::EligibleForSnoozing {
            return false;
        }

        // Protected if playing audio
        if tab.is_playing_audio {
            return false;
        }

        // Protected if performing active downloads
        if tab.has_active_downloads {
            return false;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let inactive_duration = (now - tab.last_active_time).max(0) as u64;
        inactive_duration >= threshold_seconds
    }

    /// Mark a tab as snoozed / sleeping.
    pub fn snooze_tab(&mut self, tab_id: TabId) -> bool {
        if let Some(tab) = self.get_tab_mut(tab_id) {
            tab.set_sleeping(true);
            true
        } else {
            false
        }
    }

    /// Mark a tab as restoring from snooze.
    pub fn wake_tab(&mut self, tab_id: TabId) -> bool {
        if let Some(tab) = self.get_tab_mut(tab_id) {
            tab.lifecycle_state = TabLifecycleState::Restoring;
            tab.is_lazy = false;
            true
        } else {
            false
        }
    }

    /// Finalize restoration of a snoozed tab.
    pub fn complete_restoration(&mut self, tab_id: TabId, success: bool) -> bool {
        if let Some(tab) = self.get_tab_mut(tab_id) {
            if success {
                tab.lifecycle_state = TabLifecycleState::Active;
                tab.is_lazy = false;
                tab.load_failed = false;
                tab.error_message = None;
            } else {
                tab.lifecycle_state = TabLifecycleState::FailedRestoration;
                tab.load_failed = true;
                tab.error_message = Some("Failed to restore snoozed tab session".to_string());
            }
            true
        } else {
            false
        }
    }

    pub fn update_audio_playing(&mut self, tab_id: TabId, playing: bool) -> bool {
        if let Some(tab) = self.get_tab_mut(tab_id) {
            tab.is_playing_audio = playing;
            true
        } else {
            false
        }
    }

    pub fn update_active_downloads(&mut self, tab_id: TabId, downloading: bool) -> bool {
        if let Some(tab) = self.get_tab_mut(tab_id) {
            tab.has_active_downloads = downloading;
            true
        } else {
            false
        }
    }

    // --- Tab Selection, Search, and Diagnostics ---

    pub fn search_tabs(&self, query: &str) -> Vec<(usize, &TabInfo)> {
        self.tabs
            .iter()
            .enumerate()
            .filter(|(_, tab)| tab.matches_query(query))
            .collect()
    }

    pub fn pinned_tabs(&self) -> Vec<(usize, &TabInfo)> {
        self.tabs
            .iter()
            .enumerate()
            .filter(|(_, tab)| tab.is_pinned)
            .collect()
    }

    pub fn unpinned_tabs(&self) -> Vec<(usize, &TabInfo)> {
        self.tabs
            .iter()
            .enumerate()
            .filter(|(_, tab)| !tab.is_pinned)
            .collect()
    }

    pub fn select_last_tab(&mut self) -> Option<TabId> {
        let space_tabs: Vec<(usize, TabId)> = match &self.active_space_id {
            Some(sp_id) => self
                .tabs
                .iter()
                .enumerate()
                .filter(|(_, t)| &t.space_id == sp_id)
                .map(|(i, t)| (i, t.id))
                .collect(),
            None => self.tabs.iter().enumerate().map(|(i, t)| (i, t.id)).collect(),
        };

        if let Some((idx, tid)) = space_tabs.last() {
            self.active_index = Some(*idx);
            self.select_tab(*tid);
            Some(*tid)
        } else {
            None
        }
    }

    pub fn set_lazy(&mut self, id: TabId, is_lazy: bool) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.set_sleeping(is_lazy);
        }
    }

    pub fn update_load_failed(&mut self, id: TabId, failed: bool, error: Option<String>) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.load_failed = failed;
            tab.error_message = error;
            if failed {
                tab.is_loading = false;
                tab.lifecycle_state = TabLifecycleState::FailedRestoration;
            }
        }
    }

    pub fn set_session_data(&mut self, id: TabId, data: Option<Vec<u8>>) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.session_data = data;
        }
    }

    pub fn select_tab(&mut self, id: TabId) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            // Update previously active tab to LoadedBackground
            if let Some(curr_idx) = self.active_index
                && curr_idx != index && curr_idx < self.tabs.len()
            {
                let prev = &mut self.tabs[curr_idx];
                if prev.lifecycle_state == TabLifecycleState::Active {
                    prev.lifecycle_state = TabLifecycleState::LoadedBackground;
                }
            }

            self.active_index = Some(index);
            let tab = &mut self.tabs[index];
            tab.lifecycle_state = TabLifecycleState::Active;
            tab.last_active_time = now;
            let space_id = tab.space_id.clone();

            // Synchronize Space's active_tab_id and manager's active_space_id
            self.active_space_id = Some(space_id.clone());
            if let Some(sp) = self.spaces.iter_mut().find(|s| s.id == space_id) {
                sp.active_tab_id = Some(id);
            }

            true
        } else {
            false
        }
    }

    pub fn select_tab_at(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            let id = self.tabs[index].id;
            self.select_tab(id)
        } else {
            false
        }
    }

    pub fn next_tab(&mut self) -> Option<TabId> {
        let space_tabs: Vec<(usize, TabId)> = match &self.active_space_id {
            Some(sp_id) => self
                .tabs
                .iter()
                .enumerate()
                .filter(|(_, t)| &t.space_id == sp_id)
                .map(|(i, t)| (i, t.id))
                .collect(),
            None => self.tabs.iter().enumerate().map(|(i, t)| (i, t.id)).collect(),
        };

        if space_tabs.is_empty() {
            return None;
        }

        let curr_tid = self.active_tab_id();
        let curr_pos = curr_tid
            .and_then(|tid| space_tabs.iter().position(|(_, id)| *id == tid))
            .unwrap_or(0);

        let next_pos = (curr_pos + 1) % space_tabs.len();
        let next_tid = space_tabs[next_pos].1;
        self.select_tab(next_tid);
        Some(next_tid)
    }

    pub fn previous_tab(&mut self) -> Option<TabId> {
        let space_tabs: Vec<(usize, TabId)> = match &self.active_space_id {
            Some(sp_id) => self
                .tabs
                .iter()
                .enumerate()
                .filter(|(_, t)| &t.space_id == sp_id)
                .map(|(i, t)| (i, t.id))
                .collect(),
            None => self.tabs.iter().enumerate().map(|(i, t)| (i, t.id)).collect(),
        };

        if space_tabs.is_empty() {
            return None;
        }

        let curr_tid = self.active_tab_id();
        let curr_pos = curr_tid
            .and_then(|tid| space_tabs.iter().position(|(_, id)| *id == tid))
            .unwrap_or(0);

        let prev_pos = if curr_pos == 0 {
            space_tabs.len() - 1
        } else {
            curr_pos - 1
        };

        let prev_tid = space_tabs[prev_pos].1;
        self.select_tab(prev_tid);
        Some(prev_tid)
    }

    pub fn get_tab(&self, id: TabId) -> Option<&TabInfo> {
        self.tabs.iter().find(|t| t.id == id)
    }

    pub fn get_tab_mut(&mut self, id: TabId) -> Option<&mut TabInfo> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    pub fn update_title(&mut self, id: TabId, title: String) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.title = title;
        }
    }

    pub fn update_url(&mut self, id: TabId, url: String) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.url = url;
        }
    }

    pub fn update_loading(&mut self, id: TabId, is_loading: bool, progress: f64) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.is_loading = is_loading;
            tab.load_progress = progress.clamp(0.0, 1.0);
        }
    }

    pub fn update_navigation_state(&mut self, id: TabId, can_go_back: bool, can_go_forward: bool) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.can_go_back = can_go_back;
            tab.can_go_forward = can_go_forward;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_and_close_tab() {
        let mut manager = TabManager::new();
        assert!(manager.is_empty());

        let id1 = manager.open_tab("https://archlinux.org", "Arch Linux");
        assert_eq!(manager.tab_count(), 1);
        assert_eq!(manager.active_index(), Some(0));
        assert_eq!(manager.active_tab_id(), Some(id1));

        let id2 = manager.open_tab("https://rust-lang.org", "Rust");
        assert_eq!(manager.tab_count(), 2);
        assert_eq!(manager.active_index(), Some(1));
        assert_eq!(manager.active_tab_id(), Some(id2));

        // Close second tab -> first tab becomes active
        let new_active = manager.close_tab(id2);
        assert_eq!(new_active, Some(id1));
        assert_eq!(manager.tab_count(), 1);
        assert_eq!(manager.active_index(), Some(0));

        // Close remaining tab -> empty
        let last = manager.close_tab(id1);
        assert_eq!(last, None);
        assert!(manager.is_empty());
        assert_eq!(manager.active_index(), None);
    }

    #[test]
    fn test_tab_cycling() {
        let mut manager = TabManager::new();
        let id1 = manager.open_tab("1", "Tab 1");
        let id2 = manager.open_tab("2", "Tab 2");
        let id3 = manager.open_tab("3", "Tab 3");

        assert_eq!(manager.active_tab_id(), Some(id3));

        assert_eq!(manager.next_tab(), Some(id1));
        assert_eq!(manager.next_tab(), Some(id2));
        assert_eq!(manager.next_tab(), Some(id3));

        assert_eq!(manager.previous_tab(), Some(id2));
        assert_eq!(manager.previous_tab(), Some(id1));
        assert_eq!(manager.previous_tab(), Some(id3));
    }

    #[test]
    fn test_tab_metadata_update() {
        let mut manager = TabManager::new();
        let id = manager.open_tab("https://example.com", "Example");

        manager.update_title(id, "Updated Example".to_string());
        assert_eq!(manager.get_tab(id).unwrap().title, "Updated Example");

        manager.update_loading(id, true, 0.45);
        let tab = manager.get_tab(id).unwrap();
        assert!(tab.is_loading);
        assert_eq!(tab.load_progress, 0.45);

        manager.update_navigation_state(id, true, false);
        let tab = manager.get_tab(id).unwrap();
        assert!(tab.can_go_back);
        assert!(!tab.can_go_forward);
    }

    #[test]
    fn test_spaces_management_and_isolation() {
        let mut manager = TabManager::new();
        assert_eq!(manager.spaces().len(), 1);

        let work_space = SpaceInfo::new(
            SpaceId::from_string("work"),
            "Work",
            "#06b6d4",
            Some("folder-symbolic".to_string()),
            1,
        );
        let work_id = manager.add_space(work_space);
        assert_eq!(manager.spaces().len(), 2);

        // Open tabs in default space
        let t1 = manager.open_tab("https://archlinux.org", "Arch Linux");
        let t2 = manager.open_tab("https://kernel.org", "Kernel");

        // Switch to work space
        assert!(manager.select_space(&work_id));
        assert_eq!(manager.active_space_id(), Some(&work_id));
        assert_eq!(manager.active_space_tabs().len(), 0);

        // Open tab in work space
        let _tw1 = manager.open_tab("https://github.com", "GitHub");
        assert_eq!(manager.active_space_tabs().len(), 1);
        assert_eq!(manager.tabs_for_space(&SpaceId::default()).len(), 2);

        // Move tab t2 from default space to work space
        assert!(manager.move_tab_to_space(t2, &work_id, None));
        assert_eq!(manager.active_space_tabs().len(), 2);
        assert_eq!(manager.tabs_for_space(&SpaceId::default()).len(), 1);

        // Move tab position within work space
        assert!(manager.move_tab_in_space(&work_id, 1, 0));

        // Selecting t1 should switch active space back to default
        assert!(manager.select_tab(t1));
        assert_eq!(manager.active_space_id(), Some(&SpaceId::default()));

        // Remove work space
        let (removed_sp, removed_tabs) = manager.remove_space(&work_id);
        assert!(removed_sp.is_some());
        assert_eq!(removed_tabs.len(), 2);
        assert_eq!(manager.spaces().len(), 1);
    }

    #[test]
    fn test_snooze_lifecycle_and_protection() {
        let mut manager = TabManager::new();
        let id1 = manager.open_tab("https://site1.com", "Site 1");
        let id2 = manager.open_tab("https://site2.com", "Site 2");

        // id2 is currently active -> protected from snoozing
        assert!(!manager.check_snooze_eligibility(id2, 0));

        // id1 is in LoadedBackground. Simulate elapsed time
        if let Some(t1) = manager.get_tab_mut(id1) {
            t1.last_active_time -= 1800; // 30 minutes ago
        }
        assert!(manager.check_snooze_eligibility(id1, 1700));

        // Audio protection check
        manager.update_audio_playing(id1, true);
        assert!(!manager.check_snooze_eligibility(id1, 1700));
        manager.update_audio_playing(id1, false);
        assert!(manager.check_snooze_eligibility(id1, 1700));

        // Download protection check
        manager.update_active_downloads(id1, true);
        assert!(!manager.check_snooze_eligibility(id1, 1700));
        manager.update_active_downloads(id1, false);
        assert!(manager.check_snooze_eligibility(id1, 1700));

        // Perform snooze
        assert!(manager.snooze_tab(id1));
        let t1 = manager.get_tab(id1).unwrap();
        assert_eq!(t1.lifecycle_state, TabLifecycleState::Sleeping);
        assert!(t1.is_sleeping());

        // Wake and restore
        assert!(manager.wake_tab(id1));
        assert_eq!(manager.get_tab(id1).unwrap().lifecycle_state, TabLifecycleState::Restoring);

        assert!(manager.complete_restoration(id1, true));
        assert_eq!(manager.get_tab(id1).unwrap().lifecycle_state, TabLifecycleState::Active);
    }
}
