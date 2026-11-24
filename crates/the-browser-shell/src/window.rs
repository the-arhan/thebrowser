//! Main application window layout, event routing, Space isolation, and state orchestration.

use crate::navbar::NavBar;
use crate::omnibox::Omnibox;
use crate::settings_dialog::SettingsDialog;
use crate::sidebar::SideBar;
use crate::space_dialog::{SpaceDialog, SpaceDialogResult};
use crate::tab_menu::{TabContextMenu, TabMenuAction};
use crate::tab_search::TabSearchDialog;
use crate::tab_strip::TabStrip;
use gtk4::gdk::{Key, ModifierType};
use gtk4::prelude::*;
use gtk4::{
    Application, Box as GtkBox, EventControllerKey, Orientation, Overlay, ProgressBar, Stack,
};
use libadwaita::ApplicationWindow as AdwApplicationWindow;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use the_browser_core::{
    SearchEngine, SpaceId, SpaceInfo, TabId, TabInfo, TabLifecycleState, TabManager, resolve_input,
};
use the_browser_engine::{EngineSession, EngineSettings, EngineView, webkit6};
use the_browser_storage::{BrowserSettings, SessionTab, SpaceRecord, StorageManager};

pub struct BrowserWindow {
    window: AdwApplicationWindow,
    tab_manager: Rc<RefCell<TabManager>>,
    engine_views: Rc<RefCell<HashMap<TabId, EngineView>>>,
    sessions: Rc<RefCell<HashMap<SpaceId, EngineSession>>>,
    settings: webkit6::Settings,
    storage: Option<Rc<StorageManager>>,
    sidebar: Rc<SideBar>,
    tab_strip: Rc<TabStrip>,
    navbar: Rc<NavBar>,
    tab_context_menu: Rc<TabContextMenu>,
    tab_search: Rc<TabSearchDialog>,
    omnibox: Rc<Omnibox>,
    stack: Stack,
    progress_bar: ProgressBar,
    browser_settings: Rc<RefCell<BrowserSettings>>,
    search_engine: Rc<RefCell<SearchEngine>>,
    is_vertical_mode: Rc<RefCell<bool>>,
}

impl BrowserWindow {
    pub fn new(app: &Application, storage: Option<StorageManager>, initial_url: &str) -> Rc<Self> {
        let storage_rc = storage.map(Rc::new);

        // Load persisted settings or fallback to defaults
        let loaded_settings = storage_rc
            .as_ref()
            .and_then(|s| s.load_browser_settings().ok())
            .unwrap_or_default();

        let initial_engine = SearchEngine::from_name(&loaded_settings.search_engine);
        let browser_settings = Rc::new(RefCell::new(loaded_settings));
        let search_engine = Rc::new(RefCell::new(initial_engine));

        let window = AdwApplicationWindow::builder()
            .application(app)
            .title("The Browser")
            .default_width(1200)
            .default_height(800)
            .build();

        window.add_css_class("the-browser-window");

        // Top-level Overlay to host floating Omnibox over the browsing stage
        let root_overlay = Overlay::new();
        window.set_content(Some(&root_overlay));

        // Base horizontal layout: [SideBar, StageWrapper]
        let main_layout = GtkBox::new(Orientation::Horizontal, 0);
        main_layout.add_css_class("main-layout");
        main_layout.set_hexpand(true);
        main_layout.set_vexpand(true);

        // 1. Vertical Sidebar (Primary default navigation)
        let sidebar = Rc::new(SideBar::new());
        main_layout.append(sidebar.widget());

        // 2. Stage Wrapper containing horizontal tab strip (for dual mode) and card-styled stage
        let stage_wrapper = GtkBox::new(Orientation::Vertical, 0);
        stage_wrapper.set_hexpand(true);
        stage_wrapper.set_vexpand(true);

        // Horizontal tab strip (hidden in default vertical mode; toggled via Ctrl+Shift+S)
        let tab_strip = Rc::new(TabStrip::new());
        tab_strip.widget().set_visible(false);
        stage_wrapper.append(tab_strip.widget());

        // Floating Stage Card (rounded viewport with hairline border)
        let stage_card = GtkBox::new(Orientation::Vertical, 0);
        stage_card.add_css_class("stage-card");
        stage_card.set_hexpand(true);
        stage_card.set_vexpand(true);

        // Navigation Toolbar at the top of the Stage Card
        let navbar = Rc::new(NavBar::new());
        stage_card.append(navbar.widget());

        // Progress bar attached flush right under the navbar
        let progress_bar = ProgressBar::new();
        progress_bar.add_css_class("load-progress");
        progress_bar.set_visible(false);
        stage_card.append(&progress_bar);

        // Web views stack canvas
        let stack = Stack::new();
        stack.add_css_class("stage-canvas");
        stack.set_vexpand(true);
        stack.set_hexpand(true);
        stage_card.append(&stack);

        stage_wrapper.append(&stage_card);
        main_layout.append(&stage_wrapper);
        root_overlay.set_child(Some(&main_layout));

        // 3. Tab Context Menu
        let tab_context_menu = Rc::new(TabContextMenu::new());

        // 4. Tab Search Dialog
        let tab_search = Rc::new(TabSearchDialog::new(&window));

        // 5. Floating Omnibox overlay
        let omnibox = Rc::new(Omnibox::new());
        omnibox.set_engine_name(&search_engine.borrow().name);
        root_overlay.add_overlay(omnibox.widget());

        // Initialize Spaces from SQLite
        let mut tab_manager = TabManager::new();
        if let Some(ref s) = storage_rc
            && let Ok(records) = s.load_spaces()
            && !records.is_empty()
        {
            let mut spaces = Vec::new();
            for r in records {
                let active_tid = r.active_tab_id.as_ref().and_then(|tid| TabId::parse(tid));
                let mut sp = SpaceInfo::new(
                    SpaceId::from_string(r.id),
                    r.name,
                    r.accent_color,
                    r.icon,
                    r.position,
                );
                sp.active_tab_id = active_tid;
                spaces.push(sp);
            }
            *tab_manager.spaces_mut() = spaces;
            if let Some(first) = tab_manager.spaces().first() {
                let fid = first.id.clone();
                tab_manager.select_space(&fid);
            }
        }

        let this = Rc::new(Self {
            window: window.clone(),
            tab_manager: Rc::new(RefCell::new(tab_manager)),
            engine_views: Rc::new(RefCell::new(HashMap::new())),
            sessions: Rc::new(RefCell::new(HashMap::new())),
            settings: EngineSettings::create_default(),
            storage: storage_rc,
            sidebar,
            tab_strip,
            navbar,
            tab_context_menu,
            tab_search,
            omnibox,
            stack,
            progress_bar,
            browser_settings,
            search_engine,
            is_vertical_mode: Rc::new(RefCell::new(true)),
        });

        this.setup_events();
        this.setup_keyboard_shortcuts();
        this.refresh_space_ui();

        // Connect window close request to auto-save session state
        {
            let this_clone = Rc::clone(&this);
            window.connect_close_request(move |_| {
                this_clone.save_current_session();
                glib::Propagation::Proceed
            });
        }

        // Periodic background tab snoozing timer (fires every 30 seconds)
        {
            let this_clone = Rc::clone(&this);
            glib::timeout_add_seconds_local(30, move || {
                this_clone.check_and_snooze_background_tabs();
                glib::ControlFlow::Continue
            });
        }

        // Determine startup target: if initial_url is explicitly provided, open it.
        // Otherwise, attempt to restore session from SQLite; if none exists, open home.
        if !initial_url.is_empty() && initial_url != "about:blank" && initial_url != "thebrowser://home" {
            this.open_tab(initial_url);
        } else if !this.restore_session() {
            this.open_tab("thebrowser://home");
        }

        this
    }

    pub fn present(&self) {
        self.window.present();
    }

    /// Retrieve or create the persistent isolated WebKitNetworkSession for a Space.
    pub fn session_for_space(&self, space_id: &SpaceId) -> EngineSession {
        let mut sessions = self.sessions.borrow_mut();
        if let Some(s) = sessions.get(space_id) {
            return s.clone();
        }

        let data_dir = glib::user_data_dir()
            .join("the-browser")
            .join("spaces")
            .join(space_id.as_str())
            .join("data");
        let cache_dir = glib::user_cache_dir()
            .join("the-browser")
            .join("spaces")
            .join(space_id.as_str())
            .join("cache");

        let session = EngineSession::persistent(&data_dir, &cache_dir);
        sessions.insert(space_id.clone(), session.clone());
        session
    }

    pub fn active_tab_id(&self) -> Option<TabId> {
        self.tab_manager.borrow().active_tab_id()
    }

    pub fn active_space_id(&self) -> Option<SpaceId> {
        self.tab_manager.borrow().active_space_id().cloned()
    }

    pub fn spaces(&self) -> Vec<SpaceInfo> {
        self.tab_manager.borrow().spaces().to_vec()
    }

    pub fn tabs_for_space(&self, space_id: &SpaceId) -> Vec<TabInfo> {
        self.tab_manager
            .borrow()
            .tabs_for_space(space_id)
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn tabs_for_active_space(&self) -> Vec<TabInfo> {
        if let Some(sp_id) = self.active_space_id() {
            self.tabs_for_space(&sp_id)
        } else {
            Vec::new()
        }
    }

    pub fn get_tab(&self, id: TabId) -> Option<TabInfo> {
        self.tab_manager.borrow().get_tab(id).cloned()
    }

    pub fn active_view(&self) -> Option<EngineView> {
        let active_id = self.tab_manager.borrow().active_tab_id();
        active_id.and_then(|id| self.engine_views.borrow().get(&id).cloned())
    }

    pub fn toggle_sidebar(&self) {
        self.sidebar.toggle_fold();
    }

    pub fn toggle_tab_layout(&self) {
        let mut is_vertical = self.is_vertical_mode.borrow_mut();
        *is_vertical = !*is_vertical;

        if *is_vertical {
            self.sidebar.widget().set_visible(true);
            self.tab_strip.widget().set_visible(false);
        } else {
            self.sidebar.widget().set_visible(false);
            self.tab_strip.widget().set_visible(true);
        }
    }

    pub fn show_omnibox(&self) {
        let current_url = {
            let mgr = self.tab_manager.borrow();
            mgr.active_tab().map(|t| t.url.clone())
        };

        let initial_text = match current_url.as_deref() {
            Some(u) if !u.is_empty() && u != "about:blank" && u != "thebrowser://home" => Some(u),
            _ => None,
        };

        self.omnibox.show(initial_text);
    }

    pub fn hide_omnibox(&self) {
        self.omnibox.hide();
        if let Some(view) = self.active_view() {
            view.widget().grab_focus();
        }
    }

    pub fn show_tab_search(self: &Rc<Self>) {
        let tabs = self.tab_manager.borrow().tabs().to_vec();
        let active_id = self.tab_manager.borrow().active_tab_id();
        let spaces = self.tab_manager.borrow().spaces().to_vec();
        self.tab_search.show_with_tabs(&tabs, active_id, &spaces);
    }

    // --- Spaces Subsystem ---

    pub fn refresh_space_ui(&self) {
        let mgr = self.tab_manager.borrow();
        if let Some(sp) = mgr.active_space() {
            self.sidebar.set_space_info(&sp.name, &sp.accent_color);
        }
        if let Some(sp_id) = mgr.active_space_id() {
            self.sidebar.render_space_pills(mgr.spaces(), sp_id);
        }
    }

    pub fn refresh_tabs_ui_for_active_space(&self) {
        self.sidebar.clear_tabs();
        self.tab_strip.clear_tabs();

        let mgr = self.tab_manager.borrow();
        let active_sp_id = match mgr.active_space_id() {
            Some(id) => id.clone(),
            None => return,
        };

        let space_tabs = mgr.tabs_for_space(&active_sp_id);
        for tab in space_tabs {
            self.sidebar.add_tab(tab.id, &tab.title);
            if tab.is_pinned {
                self.sidebar.update_pin(tab.id, true);
            }
            if tab.is_sleeping() {
                self.sidebar.update_lazy(tab.id, true);
            }
            if tab.load_failed {
                self.sidebar.update_load_failed(tab.id, true);
            }
            if tab.is_playing_audio {
                self.sidebar.update_audio(tab.id, true);
            }

            self.tab_strip.add_tab(tab.id, &tab.title);
            if tab.is_pinned {
                self.tab_strip.update_pin(tab.id, true);
            }
            if tab.is_sleeping() {
                self.tab_strip.update_lazy(tab.id, true);
            }
            if tab.load_failed {
                self.tab_strip.update_load_failed(tab.id, true);
            }
            if tab.is_playing_audio {
                self.tab_strip.update_audio(tab.id, true);
            }
        }
    }

    pub fn switch_space(self: &Rc<Self>, space_id: &SpaceId) {
        let changed = self.tab_manager.borrow_mut().select_space(space_id);
        if !changed {
            return;
        }

        self.refresh_space_ui();
        self.refresh_tabs_ui_for_active_space();

        let maybe_active = self.tab_manager.borrow().active_tab_id();
        if let Some(active_id) = maybe_active {
            self.select_tab(active_id);
        } else {
            // New tab in newly selected space
            self.open_tab("thebrowser://home");
        }

        self.save_current_session();
    }

    pub fn open_create_space_dialog(self: &Rc<Self>) {
        let dialog = SpaceDialog::new(&self.window, None, false);
        let this = Rc::clone(self);
        dialog.connect_result(move |result| {
            if let SpaceDialogResult::Save(space) = result {
                this.create_space(space);
            }
        });
        dialog.present();
    }

    pub fn open_edit_space_dialog(self: &Rc<Self>) {
        let (current_space, can_delete) = {
            let mgr = self.tab_manager.borrow();
            let sp = mgr.active_space().cloned();
            let can_del = mgr.spaces().len() > 1;
            (sp, can_del)
        };

        if let Some(sp) = current_space {
            let dialog = SpaceDialog::new(&self.window, Some(&sp), can_delete);
            let this = Rc::clone(self);
            dialog.connect_result(move |result| match result {
                SpaceDialogResult::Save(updated_sp) => {
                    this.edit_space(updated_sp);
                }
                SpaceDialogResult::Delete { space_id, delete_data } => {
                    this.delete_space(space_id, delete_data);
                }
            });
            dialog.present();
        }
    }

    pub fn create_space(self: &Rc<Self>, mut space: SpaceInfo) {
        space.position = self.tab_manager.borrow().spaces().len() as i32;
        let space_id = self.tab_manager.borrow_mut().add_space(space);

        self.persist_spaces();
        self.switch_space(&space_id);
    }

    pub fn edit_space(self: &Rc<Self>, space: SpaceInfo) {
        let id = space.id.clone();
        self.tab_manager.borrow_mut().update_space(
            &id,
            Some(space.name),
            Some(space.accent_color),
            space.icon,
        );

        self.persist_spaces();
        self.refresh_space_ui();
    }

    pub fn delete_space(self: &Rc<Self>, space_id: SpaceId, delete_data: bool) {
        if self.tab_manager.borrow().spaces().len() <= 1 {
            return;
        }

        // 1. Remove space and all its tabs from memory
        let (_removed_sp, removed_tabs) = self.tab_manager.borrow_mut().remove_space(&space_id);

        for tab in removed_tabs {
            if let Some(view) = self.engine_views.borrow_mut().remove(&tab.id) {
                self.stack.remove(view.widget());
            }
        }

        // 2. Remove session and optionally data directories on disk
        if let Some(session) = self.sessions.borrow_mut().remove(&space_id)
            && delete_data
        {
            if let Some(d) = session.data_dir() {
                let _ = std::fs::remove_dir_all(d);
            }
            if let Some(c) = session.cache_dir() {
                let _ = std::fs::remove_dir_all(c);
            }
        }

        // 3. Update storage
        if let Some(ref storage) = self.storage {
            let _ = storage.delete_space(space_id.as_str());
        }

        self.persist_spaces();

        let maybe_curr = self.tab_manager.borrow().active_space_id().cloned();
        if let Some(curr_sp) = maybe_curr {
            self.switch_space(&curr_sp);
        }
    }

    fn persist_spaces(&self) {
        if let Some(ref storage) = self.storage {
            let mgr = self.tab_manager.borrow();
            let records: Vec<SpaceRecord> = mgr
                .spaces()
                .iter()
                .enumerate()
                .map(|(i, s)| SpaceRecord {
                    id: s.id.to_string(),
                    name: s.name.clone(),
                    accent_color: s.accent_color.clone(),
                    icon: s.icon.clone(),
                    position: i as i32,
                    is_active: Some(&s.id) == mgr.active_space_id(),
                    active_tab_id: s.active_tab_id.map(|t| t.to_string()),
                })
                .collect();
            let _ = storage.save_spaces(&records);
        }
    }

    // --- Tab Transfer Between Spaces ---

    /// Move a tab to a different Space.
    /// Note: A WebKit WebView belongs to an immutable NetworkSession upon creation.
    /// Transferring to another browsing context destroys the original WebView and
    /// instantiates a fresh view bound to the target Space's isolated context.
    pub fn move_tab_to_space(self: &Rc<Self>, tab_id: TabId, target_space_id: SpaceId) {
        let (curr_space, is_active) = {
            let mgr = self.tab_manager.borrow();
            let tab = match mgr.get_tab(tab_id) {
                Some(t) => t,
                None => return,
            };
            (tab.space_id.clone(), Some(tab.id) == mgr.active_tab_id())
        };

        if curr_space == target_space_id {
            return;
        }

        // Safely detach and destroy WebView from source session
        if let Some(view) = self.engine_views.borrow_mut().remove(&tab_id) {
            self.stack.remove(view.widget());
        }

        // Transfer in tab manager model
        self.tab_manager.borrow_mut().move_tab_to_space(tab_id, &target_space_id, None);

        // If moved from the currently visible space, remove from sidebar
        let active_sp_id = self.tab_manager.borrow().active_space_id().cloned().unwrap_or_default();
        if curr_space == active_sp_id {
            self.sidebar.remove_tab(tab_id);
            self.tab_strip.remove_tab(tab_id);

            if is_active {
                let maybe_new = self.tab_manager.borrow().active_tab_id();
                if let Some(new_act) = maybe_new {
                    self.select_tab(new_act);
                }
            }
        }

        self.save_current_session();
    }

    // --- Drag & Drop Reordering ---

    pub fn reorder_tab(self: &Rc<Self>, dragged_id: TabId, target_id: TabId, is_after: bool) {
        let active_sp_id = match self.tab_manager.borrow().active_space_id() {
            Some(id) => id.clone(),
            None => return,
        };

        let (dragged_pos, target_pos) = {
            let mgr = self.tab_manager.borrow();
            let space_tabs = mgr.tabs_for_space(&active_sp_id);
            let d_pos = space_tabs.iter().position(|t| t.id == dragged_id);
            let t_pos = space_tabs.iter().position(|t| t.id == target_id);
            match (d_pos, t_pos) {
                (Some(d), Some(t)) => (d, t),
                _ => return,
            }
        };

        let dest_pos = if is_after {
            if dragged_pos < target_pos { target_pos } else { target_pos + 1 }
        } else if dragged_pos < target_pos {
            target_pos.saturating_sub(1)
        } else {
            target_pos
        };

        if self.tab_manager.borrow_mut().move_tab_in_space(&active_sp_id, dragged_pos, dest_pos) {
            self.refresh_tabs_ui_for_active_space();
            if let Some(act) = self.tab_manager.borrow().active_tab_id() {
                self.sidebar.set_active_tab(act);
                self.tab_strip.set_active_tab(act);
            }
            self.save_current_session();
        }
    }

    pub fn set_tab_pinned(self: &Rc<Self>, tab_id: TabId, is_pinned: bool) {
        self.tab_manager.borrow_mut().set_pinned(tab_id, is_pinned);
        self.sidebar.update_pin(tab_id, is_pinned);
        self.tab_strip.update_pin(tab_id, is_pinned);
        self.save_current_session();
    }

    // --- Intelligent Tab Snoozing ---

    pub fn check_and_snooze_background_tabs(self: &Rc<Self>) {
        let (snooze_enabled, threshold_mins) = {
            let s = self.browser_settings.borrow();
            (s.tab_snooze_enabled, s.tab_snooze_threshold_mins)
        };

        if !snooze_enabled {
            return;
        }

        let threshold_secs = (threshold_mins as u64) * 60;
        let eligible_ids: Vec<TabId> = {
            let mgr = self.tab_manager.borrow();
            mgr.tabs()
                .iter()
                .filter(|t| mgr.check_snooze_eligibility(t.id, threshold_secs))
                .map(|t| t.id)
                .collect()
        };

        for tab_id in eligible_ids {
            self.snooze_tab(tab_id);
        }
    }

    pub fn snooze_tab(self: &Rc<Self>, id: TabId) {
        // Protect active tab
        if Some(id) == self.active_tab_id() {
            return;
        }

        // 1. Serialize session state
        let state_data = if let Some(view) = self.engine_views.borrow().get(&id) {
            view.session_state()
        } else {
            None
        };

        if let Some(data) = state_data {
            self.tab_manager.borrow_mut().set_session_data(id, Some(data));
        }

        // 2. Detach and destroy WebView safely to reclaim memory
        if let Some(view) = self.engine_views.borrow_mut().remove(&id) {
            self.stack.remove(view.widget());
        }

        // 3. Update tab model and visual representation
        self.tab_manager.borrow_mut().snooze_tab(id);
        self.sidebar.update_lifecycle_state(id, TabLifecycleState::Sleeping);
        self.tab_strip.update_lifecycle_state(id, TabLifecycleState::Sleeping);

        self.save_current_session();
    }

    pub fn wake_tab(self: &Rc<Self>, id: TabId) {
        self.select_tab(id);
    }

    pub fn snooze_all_eligible_tabs(self: &Rc<Self>) {
        let eligible_ids: Vec<TabId> = {
            let mgr = self.tab_manager.borrow();
            let act_id = mgr.active_tab_id();
            mgr.tabs()
                .iter()
                .filter(|t| Some(t.id) != act_id && !t.is_sleeping() && !t.is_playing_audio && !t.has_active_downloads)
                .map(|t| t.id)
                .collect()
        };

        for id in eligible_ids {
            self.snooze_tab(id);
        }
    }

    // --- Navigation & Core Dispatch ---

    pub fn navigate_active(self: &Rc<Self>, input: &str) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return;
        }

        if trimmed == "thebrowser://settings" || trimmed == "about:settings" {
            self.open_settings_dialog();
            return;
        }

        let engine = self.search_engine.borrow().clone();
        let target = resolve_input(trimmed, &engine);
        let target_url = target.target_url().to_string();

        if let Some(view) = self.active_view() {
            if target_url == "thebrowser://home" || target_url == "about:home" {
                let (shortcuts, recent) = if let Some(ref storage) = self.storage {
                    let sc = storage.get_shortcuts().unwrap_or_default();
                    let rc = storage.get_recent_history(10).unwrap_or_default();
                    (sc, rc)
                } else {
                    (Vec::new(), Vec::new())
                };
                let settings = self.browser_settings.borrow();
                view.load_home(
                    &engine,
                    &shortcuts,
                    &recent,
                    settings.show_shortcuts,
                    settings.show_recent,
                );
                if let Some(id) = self.active_tab_id() {
                    self.tab_manager.borrow_mut().update_url(id, "thebrowser://home".to_string());
                    self.tab_manager.borrow_mut().update_title(id, "Home".to_string());
                    self.sidebar.update_title(id, "Home");
                    self.tab_strip.update_title(id, "Home");
                    self.navbar.set_url("thebrowser://home");
                    self.navbar.set_bookmarked(false);
                }
            } else {
                view.load_uri(&target_url);
            }
            self.save_current_session();
        }
    }

    pub fn duplicate_tab(self: &Rc<Self>, id: TabId) {
        let (url, title, session_data) = {
            let mgr = self.tab_manager.borrow();
            let t = match mgr.get_tab(id) {
                Some(t) => t,
                None => return,
            };
            (t.url.clone(), t.title.clone(), t.session_data.clone())
        };

        let new_id = self.open_tab(&url);
        self.tab_manager.borrow_mut().update_title(new_id, title.clone());
        self.sidebar.update_title(new_id, &title);
        self.tab_strip.update_title(new_id, &title);

        if let Some(view) = self.engine_views.borrow().get(&id)
            && let Some(state) = view.session_state()
            && let Some(new_view) = self.engine_views.borrow().get(&new_id)
        {
            new_view.restore_session_state(&state);
        } else if let Some(state) = session_data
            && let Some(new_view) = self.engine_views.borrow().get(&new_id)
        {
            new_view.restore_session_state(&state);
        }

        self.save_current_session();
    }

    pub fn toggle_pin(self: &Rc<Self>, id: TabId) {
        let is_pinned = self.tab_manager.borrow_mut().toggle_pin(id);
        self.sidebar.update_pin(id, is_pinned);
        self.tab_strip.update_pin(id, is_pinned);
        self.save_current_session();
    }

    pub fn close_other_tabs(self: &Rc<Self>, id: TabId) {
        let closed_ids = self.tab_manager.borrow_mut().close_other_tabs(id);
        for closed_id in closed_ids {
            if let Some(view) = self.engine_views.borrow_mut().remove(&closed_id) {
                self.stack.remove(view.widget());
            }
            self.sidebar.remove_tab(closed_id);
            self.tab_strip.remove_tab(closed_id);
        }
        self.select_tab(id);
        self.save_current_session();
    }

    pub fn close_tabs_to_right(self: &Rc<Self>, id: TabId) {
        let closed_ids = self.tab_manager.borrow_mut().close_tabs_to_right(id);
        for closed_id in closed_ids {
            if let Some(view) = self.engine_views.borrow_mut().remove(&closed_id) {
                self.stack.remove(view.widget());
            }
            self.sidebar.remove_tab(closed_id);
            self.tab_strip.remove_tab(closed_id);
        }
        if let Some(active_id) = self.active_tab_id() {
            self.select_tab(active_id);
        }
        self.save_current_session();
    }

    pub fn reopen_closed_tab(self: &Rc<Self>) {
        let tab_info = self.tab_manager.borrow_mut().pop_closed_tab();
        if let Some(tab_info) = tab_info {
            let target_url = if tab_info.url.is_empty() {
                "thebrowser://home"
            } else {
                &tab_info.url
            };
            let new_id = self.open_tab(target_url);
            self.tab_manager.borrow_mut().update_title(new_id, tab_info.title.clone());
            self.sidebar.update_title(new_id, &tab_info.title);
            self.tab_strip.update_title(new_id, &tab_info.title);

            if let Some(ref data) = tab_info.session_data
                && let Some(view) = self.engine_views.borrow().get(&new_id)
            {
                view.restore_session_state(data);
            }
            self.save_current_session();
        }
    }

    pub fn move_tab(self: &Rc<Self>, from: usize, to: usize) -> bool {
        let moved = self.tab_manager.borrow_mut().move_tab(from, to);
        if moved {
            self.save_current_session();
        }
        moved
    }

    pub fn save_current_session(&self) {
        let storage = match &self.storage {
            Some(s) => s,
            None => return,
        };

        let tabs_snapshot = self.tab_manager.borrow().tabs().to_vec();
        let active_id = self.tab_manager.borrow().active_tab_id();
        let views = self.engine_views.borrow();

        let session_tabs: Vec<SessionTab> = tabs_snapshot
            .iter()
            .enumerate()
            .map(|(idx, tab)| {
                let session_data = views
                    .get(&tab.id)
                    .and_then(|v| v.session_state())
                    .or_else(|| tab.session_data.clone());

                SessionTab {
                    id: tab.id.to_string(),
                    position: idx as i32,
                    url: tab.url.clone(),
                    title: tab.title.clone(),
                    is_pinned: tab.is_pinned,
                    is_active: Some(tab.id) == active_id,
                    session_data,
                    space_id: Some(tab.space_id.to_string()),
                }
            })
            .collect();

        let _ = storage.save_session(&session_tabs);
        self.persist_spaces();
    }

    pub fn restore_session(self: &Rc<Self>) -> bool {
        let storage = match &self.storage {
            Some(s) => s,
            None => return false,
        };

        let saved_tabs = match storage.load_session() {
            Ok(tabs) if !tabs.is_empty() => tabs,
            _ => return false,
        };

        self.sidebar.clear_tabs();
        self.tab_strip.clear_tabs();
        self.engine_views.borrow_mut().clear();
        while let Some(child) = self.stack.first_child() {
            self.stack.remove(&child);
        }

        let active_idx = saved_tabs.iter().position(|t| t.is_active).unwrap_or(0);
        let mut active_id = None;

        for (idx, saved_tab) in saved_tabs.iter().enumerate() {
            let is_active = idx == active_idx;
            let space_id = saved_tab
                .space_id
                .as_ref()
                .map(|s| SpaceId::from_string(s.clone()))
                .unwrap_or_default();

            let mut info = TabInfo::new_with_space(space_id.clone(), &saved_tab.url, &saved_tab.title);
            info.is_pinned = saved_tab.is_pinned;
            info.set_sleeping(!is_active);
            info.session_data = saved_tab.session_data.clone();
            let id = info.id;

            if is_active {
                active_id = Some(id);
                self.tab_manager.borrow_mut().select_space(&space_id);
            }

            self.tab_manager.borrow_mut().restore_tab(info, is_active);
        }

        self.refresh_space_ui();
        self.refresh_tabs_ui_for_active_space();

        if let Some(id) = active_id {
            self.select_tab(id);
        } else {
            let maybe_first = self.tab_manager.borrow().tabs().first().map(|t| t.id);
            if let Some(first_id) = maybe_first {
                self.select_tab(first_id);
            }
        }

        true
    }

    pub fn toggle_active_bookmark(self: &Rc<Self>) {
        let (url, title) = {
            let mgr = self.tab_manager.borrow();
            let t = match mgr.active_tab() {
                Some(t) => t,
                None => return,
            };
            (t.url.clone(), t.title.clone())
        };

        if url.is_empty() || url == "about:blank" || url.starts_with("thebrowser://") {
            return;
        }

        if let Some(ref storage) = self.storage {
            let is_bookmarked = storage.is_bookmarked(&url).unwrap_or(false);
            if is_bookmarked {
                let _ = storage.remove_bookmark(&url);
                self.navbar.set_bookmarked(false);
            } else {
                let _ = storage.add_bookmark(&url, &title);
                self.navbar.set_bookmarked(true);
            }
        }
    }

    pub fn open_settings_dialog(self: &Rc<Self>) {
        let dialog = SettingsDialog::new(
            &self.window,
            self.browser_settings.borrow().clone(),
            self.storage.clone(),
        );

        let this = Rc::clone(self);
        dialog.connect_settings_changed(move |settings| {
            *this.browser_settings.borrow_mut() = settings.clone();
            let new_engine = SearchEngine::from_name(&settings.search_engine);
            *this.search_engine.borrow_mut() = new_engine.clone();
            this.omnibox.set_engine_name(&new_engine.name);
        });

        dialog.present();
    }

    fn setup_events(self: &Rc<Self>) {
        // Omnibox commit navigation
        {
            let this = Rc::clone(self);
            self.omnibox.connect_activate(move |input_text| {
                this.navigate_active(input_text);
                this.hide_omnibox();
            });
        }

        // Omnibox dismiss
        {
            let this = Rc::clone(self);
            self.omnibox.connect_dismiss(move || {
                this.hide_omnibox();
            });
        }

        // Tab Search selection
        {
            let this = Rc::clone(self);
            self.tab_search.connect_select_tab(move |id| {
                this.select_tab(id);
            });
        }

        // --- Navigation Bar Events ---
        {
            let this = Rc::clone(self);
            self.navbar.connect_back(move || {
                if let Some(view) = this.active_view() {
                    view.go_back();
                }
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_forward(move || {
                if let Some(view) = this.active_view() {
                    view.go_forward();
                }
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_reload(move || {
                if let Some(view) = this.active_view() {
                    view.reload();
                }
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_stop(move || {
                if let Some(view) = this.active_view() {
                    view.stop_loading();
                }
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_home(move || {
                this.navigate_active("thebrowser://home");
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_navigate(move |text| {
                this.navigate_active(text);
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_toggle_bookmark(move || {
                this.toggle_active_bookmark();
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_open_settings(move || {
                this.open_settings_dialog();
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_new_tab(move || {
                this.open_tab("thebrowser://home");
                this.navbar.focus_address();
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_reopen_tab(move || {
                this.reopen_closed_tab();
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_search_tabs(move || {
                this.show_tab_search();
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_snooze_tabs(move || {
                this.snooze_all_eligible_tabs();
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_toggle_sidebar(move || {
                this.toggle_sidebar();
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_toggle_layout(move || {
                this.toggle_tab_layout();
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_clear_history(move || {
                if let Some(ref s) = this.storage {
                    let _ = s.clear_history();
                }
            });
        }

        {
            let this = Rc::clone(self);
            self.navbar.connect_about(move || {
                this.open_settings_dialog();
            });
        }

        // --- Sidebar Events ---
        {
            let this = Rc::clone(self);
            self.sidebar.connect_select(move |id| {
                this.select_tab(id);
            });
        }

        {
            let this = Rc::clone(self);
            self.sidebar.connect_close(move |id| {
                this.close_tab(id);
            });
        }

        {
            let this = Rc::clone(self);
            self.sidebar.connect_new_tab(move || {
                this.open_tab("thebrowser://home");
                this.navbar.focus_address();
            });
        }

        {
            let this = Rc::clone(self);
            self.sidebar.connect_select_space(move |sp_id| {
                this.switch_space(&sp_id);
            });
        }

        {
            let this = Rc::clone(self);
            self.sidebar.connect_new_space(move || {
                this.open_create_space_dialog();
            });
        }

        {
            let this = Rc::clone(self);
            self.sidebar.connect_edit_space(move || {
                this.open_edit_space_dialog();
            });
        }

        {
            let this = Rc::clone(self);
            self.sidebar.connect_reorder_tab(move |dragged_id, target_id, is_after| {
                this.reorder_tab(dragged_id, target_id, is_after);
            });
        }

        {
            let this = Rc::clone(self);
            self.sidebar.connect_move_tab_to_space(move |tab_id, target_sp_id| {
                this.move_tab_to_space(tab_id, target_sp_id);
            });
        }

        {
            let this = Rc::clone(self);
            self.sidebar.connect_set_pinned(move |tab_id, is_pinned| {
                this.set_tab_pinned(tab_id, is_pinned);
            });
        }

        {
            let this = Rc::clone(self);
            self.sidebar.connect_context_menu(move |id, widget| {
                let (is_pinned, is_sleeping, space_id) = this
                    .tab_manager
                    .borrow()
                    .get_tab(id)
                    .map(|t| (t.is_pinned, t.is_sleeping(), t.space_id.clone()))
                    .unwrap_or((false, false, SpaceId::default()));
                let spaces = this.tab_manager.borrow().spaces().to_vec();
                this.tab_context_menu.popup_for(widget, id, is_pinned, is_sleeping, &space_id, &spaces);
            });
        }

        // --- Horizontal Tab Strip Events ---
        {
            let this = Rc::clone(self);
            self.tab_strip.connect_select(move |id| {
                this.select_tab(id);
            });
        }

        {
            let this = Rc::clone(self);
            self.tab_strip.connect_close(move |id| {
                this.close_tab(id);
            });
        }

        {
            let this = Rc::clone(self);
            self.tab_strip.connect_new_tab(move || {
                this.open_tab("thebrowser://home");
                this.navbar.focus_address();
            });
        }

        {
            let this = Rc::clone(self);
            self.tab_strip.connect_reorder_tab(move |dragged_id, target_id, is_after| {
                this.reorder_tab(dragged_id, target_id, is_after);
            });
        }

        {
            let this = Rc::clone(self);
            self.tab_strip.connect_context_menu(move |id, widget| {
                let (is_pinned, is_sleeping, space_id) = this
                    .tab_manager
                    .borrow()
                    .get_tab(id)
                    .map(|t| (t.is_pinned, t.is_sleeping(), t.space_id.clone()))
                    .unwrap_or((false, false, SpaceId::default()));
                let spaces = this.tab_manager.borrow().spaces().to_vec();
                this.tab_context_menu.popup_for(widget, id, is_pinned, is_sleeping, &space_id, &spaces);
            });
        }

        // --- Tab Context Menu Actions ---
        {
            let this = Rc::clone(self);
            self.tab_context_menu.connect_action(move |action| match action {
                TabMenuAction::NewTab => {
                    this.open_tab("thebrowser://home");
                    this.navbar.focus_address();
                }
                TabMenuAction::Duplicate(id) => {
                    this.duplicate_tab(id);
                }
                TabMenuAction::TogglePin(id) => {
                    this.toggle_pin(id);
                }
                TabMenuAction::Reload(id) => {
                    if let Some(view) = this.engine_views.borrow().get(&id) {
                        view.reload();
                    }
                }
                TabMenuAction::Snooze(id) => {
                    this.snooze_tab(id);
                }
                TabMenuAction::Wake(id) => {
                    this.wake_tab(id);
                }
                TabMenuAction::MoveToSpace(tab_id, target_sp) => {
                    this.move_tab_to_space(tab_id, target_sp);
                }
                TabMenuAction::Close(id) => {
                    this.close_tab(id);
                }
                TabMenuAction::CloseOthers(id) => {
                    this.close_other_tabs(id);
                }
                TabMenuAction::CloseToRight(id) => {
                    this.close_tabs_to_right(id);
                }
                TabMenuAction::ReopenClosed => {
                    this.reopen_closed_tab();
                }
            });
        }
    }

    fn setup_keyboard_shortcuts(self: &Rc<Self>) {
        let key_controller = EventControllerKey::new();
        let this = Rc::clone(self);

        key_controller.connect_key_pressed(move |_, keyval, _, state| {
            let ctrl = state.contains(ModifierType::CONTROL_MASK);
            let alt = state.contains(ModifierType::ALT_MASK);
            let shift = state.contains(ModifierType::SHIFT_MASK);

            match (ctrl, alt, shift, keyval) {
                // Ctrl+L: Focus Navigation Address Bar
                (true, false, false, Key::l) | (true, false, false, Key::L) => {
                    this.navbar.focus_address();
                    glib::Propagation::Stop
                }
                // Ctrl+Shift+L: Floating Omnibox
                (true, false, true, Key::l) | (true, false, true, Key::L) => {
                    this.show_omnibox();
                    glib::Propagation::Stop
                }
                // Ctrl+T: New Tab with Address Bar focused
                (true, false, false, Key::t) | (true, false, false, Key::T) => {
                    this.open_tab("thebrowser://home");
                    this.navbar.focus_address();
                    glib::Propagation::Stop
                }
                // Ctrl+Shift+T: Reopen closed tab
                (true, false, true, Key::t) | (true, false, true, Key::T) => {
                    this.reopen_closed_tab();
                    glib::Propagation::Stop
                }
                // Ctrl+Shift+A: Tab search dialog
                (true, false, true, Key::a) | (true, false, true, Key::A) => {
                    this.show_tab_search();
                    glib::Propagation::Stop
                }
                // Ctrl+D: Bookmark active page
                (true, false, false, Key::d) | (true, false, false, Key::D) => {
                    this.toggle_active_bookmark();
                    glib::Propagation::Stop
                }
                // Ctrl+W: Close Tab (or dismiss Omnibox if visible)
                (true, false, false, Key::w) | (true, false, false, Key::W) => {
                    if this.omnibox.is_visible() {
                        this.hide_omnibox();
                    } else if let Some(id) = this.active_tab_id() {
                        this.close_tab(id);
                    }
                    glib::Propagation::Stop
                }
                // Ctrl+Shift+S: Toggle Horizontal / Vertical Tab Layout
                (true, false, true, Key::s) | (true, false, true, Key::S) => {
                    this.toggle_tab_layout();
                    glib::Propagation::Stop
                }
                // Ctrl+S: Fold / Unfold Sidebar
                (true, false, false, Key::s) | (true, false, false, Key::S) => {
                    this.toggle_sidebar();
                    glib::Propagation::Stop
                }
                // Escape: Dismiss Omnibox
                (false, false, false, Key::Escape) => {
                    if this.omnibox.is_visible() {
                        this.hide_omnibox();
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                }
                // Ctrl+R / F5: Reload
                (true, false, false, Key::r) | (true, false, false, Key::R) | (false, false, false, Key::F5) => {
                    if let Some(view) = this.active_view() {
                        view.reload();
                    }
                    glib::Propagation::Stop
                }
                // Ctrl+Shift+R / Ctrl+F5: Hard reload (bypass cache)
                (true, false, true, Key::r) | (true, false, true, Key::R) | (true, false, false, Key::F5) => {
                    if let Some(view) = this.active_view() {
                        view.reload_bypass_cache();
                    }
                    glib::Propagation::Stop
                }
                // Alt+Left: Back
                (false, true, false, Key::Left) => {
                    if let Some(view) = this.active_view() {
                        view.go_back();
                    }
                    glib::Propagation::Stop
                }
                // Alt+Right: Forward
                (false, true, false, Key::Right) => {
                    if let Some(view) = this.active_view() {
                        view.go_forward();
                    }
                    glib::Propagation::Stop
                }
                // Ctrl+Tab / Ctrl+Page_Down: Next tab
                (true, false, false, Key::Tab) | (true, false, false, Key::Page_Down) => {
                    let next_id = this.tab_manager.borrow_mut().next_tab();
                    if let Some(next_id) = next_id {
                        this.select_tab(next_id);
                    }
                    glib::Propagation::Stop
                }
                // Ctrl+Shift+Tab / Ctrl+Page_Up: Previous tab
                (true, false, true, Key::ISO_Left_Tab) | (true, false, false, Key::Page_Up) => {
                    let prev_id = this.tab_manager.borrow_mut().previous_tab();
                    if let Some(prev_id) = prev_id {
                        this.select_tab(prev_id);
                    }
                    glib::Propagation::Stop
                }
                // Ctrl+1 through Ctrl+8: Switch to tab index
                (true, false, false, Key::_1) => { this.select_tab_index(0); glib::Propagation::Stop }
                (true, false, false, Key::_2) => { this.select_tab_index(1); glib::Propagation::Stop }
                (true, false, false, Key::_3) => { this.select_tab_index(2); glib::Propagation::Stop }
                (true, false, false, Key::_4) => { this.select_tab_index(3); glib::Propagation::Stop }
                (true, false, false, Key::_5) => { this.select_tab_index(4); glib::Propagation::Stop }
                (true, false, false, Key::_6) => { this.select_tab_index(5); glib::Propagation::Stop }
                (true, false, false, Key::_7) => { this.select_tab_index(6); glib::Propagation::Stop }
                (true, false, false, Key::_8) => { this.select_tab_index(7); glib::Propagation::Stop }
                // Ctrl+9: Switch to LAST tab (standard browser behavior)
                (true, false, false, Key::_9) => {
                    let last_id = this.tab_manager.borrow_mut().select_last_tab();
                    if let Some(id) = last_id {
                        this.select_tab(id);
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });

        self.window.add_controller(key_controller);
    }

    fn load_target_into_view(&self, view: &EngineView, url: &str) {
        if url == "thebrowser://home" || url == "about:home" || url.is_empty() || url == "about:blank" {
            let (shortcuts, recent) = if let Some(ref storage) = self.storage {
                let sc = storage.get_shortcuts().unwrap_or_default();
                let rc = storage.get_recent_history(10).unwrap_or_default();
                (sc, rc)
            } else {
                (Vec::new(), Vec::new())
            };
            let engine = self.search_engine.borrow().clone();
            let settings = self.browser_settings.borrow();
            view.load_home(
                &engine,
                &shortcuts,
                &recent,
                settings.show_shortcuts,
                settings.show_recent,
            );
        } else {
            view.load_uri(url);
        }
    }

    fn wire_engine_view_signals(self: &Rc<Self>, view: &EngineView, tab_id: TabId) {
        let this = Rc::clone(self);

        view.setup_signals(
            // on_title
            {
                let this = Rc::clone(&this);
                move |new_title| {
                    this.tab_manager.borrow_mut().update_title(tab_id, new_title.to_string());
                    this.sidebar.update_title(tab_id, new_title);
                    this.tab_strip.update_title(tab_id, new_title);

                    let is_active = this.active_tab_id() == Some(tab_id);
                    if is_active {
                        this.window.set_title(Some(&format!("{new_title} — The Browser")));
                    }

                    if let Some(ref storage) = this.storage {
                        let url = {
                            let mgr = this.tab_manager.borrow();
                            mgr.get_tab(tab_id).map(|t| t.url.clone()).unwrap_or_default()
                        };
                        let _ = storage.record_visit(&url, new_title);
                    }

                    this.save_current_session();
                }
            },
            // on_uri
            {
                let this = Rc::clone(&this);
                move |new_uri| {
                    this.tab_manager.borrow_mut().update_url(tab_id, new_uri.to_string());
                    this.tab_manager.borrow_mut().update_load_failed(tab_id, false, None);
                    this.sidebar.update_load_failed(tab_id, false);
                    this.tab_strip.update_load_failed(tab_id, false);

                    let is_active = this.active_tab_id() == Some(tab_id);
                    if is_active {
                        this.navbar.set_url(new_uri);
                        let is_bm = this
                            .storage
                            .as_ref()
                            .and_then(|s| s.is_bookmarked(new_uri).ok())
                            .unwrap_or(false);
                        this.navbar.set_bookmarked(is_bm);
                    }

                    this.save_current_session();
                }
            },
            // on_progress
            {
                let this = Rc::clone(&this);
                move |progress| {
                    this.tab_manager.borrow_mut().update_loading(tab_id, true, progress);
                    let is_active = this.active_tab_id() == Some(tab_id);
                    if is_active {
                        this.progress_bar.set_fraction(progress);
                    }
                }
            },
            // on_loading
            {
                let this = Rc::clone(&this);
                move |is_loading| {
                    let progress = if is_loading { 0.1 } else { 0.0 };
                    this.tab_manager.borrow_mut().update_loading(tab_id, is_loading, progress);
                    this.sidebar.update_loading(tab_id, is_loading);
                    this.tab_strip.update_loading(tab_id, is_loading);

                    let is_active = this.active_tab_id() == Some(tab_id);
                    if is_active {
                        this.progress_bar.set_visible(is_loading);
                        this.navbar.set_loading(is_loading);
                        if !is_loading {
                            this.progress_bar.set_fraction(0.0);
                        }
                    }
                }
            },
            // on_nav_state
            {
                let this = Rc::clone(&this);
                move |can_back, can_forward| {
                    this.tab_manager.borrow_mut().update_navigation_state(tab_id, can_back, can_forward);
                    let is_active = this.active_tab_id() == Some(tab_id);
                    if is_active {
                        this.navbar.set_can_go_back(can_back);
                        this.navbar.set_can_go_forward(can_forward);
                    }
                }
            },
            // on_close
            {
                let this = Rc::clone(&this);
                move || {
                    this.close_tab(tab_id);
                }
            },
            // on_new_tab (target="_blank")
            {
                let this = Rc::clone(&this);
                move |target_url| {
                    this.open_tab(target_url);
                }
            },
            // on_load_failed
            {
                let this = Rc::clone(&this);
                move |error_msg| {
                    this.tab_manager.borrow_mut().update_load_failed(tab_id, true, Some(error_msg.to_string()));
                    this.sidebar.update_load_failed(tab_id, true);
                    this.tab_strip.update_load_failed(tab_id, true);
                }
            },
        );
    }

    /// Lazily instantiates the EngineView bound to its Space's isolated NetworkSession.
    fn ensure_engine_view_for_tab(self: &Rc<Self>, id: TabId) -> Option<EngineView> {
        if let Some(view) = self.engine_views.borrow().get(&id) {
            return Some(view.clone());
        }

        let (url, session_data, space_id) = {
            let mgr = self.tab_manager.borrow();
            let t = mgr.get_tab(id)?;
            (t.url.clone(), t.session_data.clone(), t.space_id.clone())
        };

        // Obtain isolated browsing session for this Space
        let space_session = self.session_for_space(&space_id);
        let view = EngineView::new(&space_session, &self.settings);
        let stack_name = id.to_string();
        self.stack.add_named(view.widget(), Some(&stack_name));

        self.wire_engine_view_signals(&view, id);

        // Connect audio playing notifications
        {
            let this = Rc::clone(self);
            let tab_id = id;
            view.connect_audio_notify(move |is_playing| {
                this.tab_manager.borrow_mut().update_audio_playing(tab_id, is_playing);
                this.sidebar.update_audio(tab_id, is_playing);
                this.tab_strip.update_audio(tab_id, is_playing);
            });
        }

        self.tab_manager.borrow_mut().set_lazy(id, false);
        self.sidebar.update_lazy(id, false);
        self.tab_strip.update_lazy(id, false);

        self.engine_views.borrow_mut().insert(id, view.clone());

        if let Some(data) = session_data {
            if !view.restore_session_state(&data) {
                self.load_target_into_view(&view, &url);
                self.tab_manager.borrow_mut().complete_restoration(id, false);
            } else {
                self.tab_manager.borrow_mut().complete_restoration(id, true);
            }
        } else {
            self.load_target_into_view(&view, &url);
            self.tab_manager.borrow_mut().complete_restoration(id, true);
        }

        Some(view)
    }

    pub fn open_tab(self: &Rc<Self>, input: &str) -> TabId {
        let (resolved_url, initial_title) = if input.is_empty() || input == "about:blank" || input == "thebrowser://home" {
            ("thebrowser://home".to_string(), "Home".to_string())
        } else {
            let engine = self.search_engine.borrow().clone();
            let target = resolve_input(input, &engine);
            let url = target.target_url().to_string();
            let title = if url == "thebrowser://home" {
                "Home".to_string()
            } else {
                url.clone()
            };
            (url, title)
        };

        let id = self.tab_manager.borrow_mut().open_tab(&resolved_url, &initial_title);

        // Add to both vertical sidebar and horizontal tab strip
        self.sidebar.add_tab(id, &initial_title);
        self.tab_strip.add_tab(id, &initial_title);

        // Select newly opened tab
        self.select_tab(id);

        id
    }

    pub fn select_tab(self: &Rc<Self>, id: TabId) {
        // Ensure EngineView is initialized (handles lazy background tabs)
        self.ensure_engine_view_for_tab(id);

        let _ = self.tab_manager.borrow_mut().select_tab(id);

        let stack_name = id.to_string();
        self.stack.set_visible_child_name(&stack_name);
        self.sidebar.set_active_tab(id);
        self.tab_strip.set_active_tab(id);

        let tab_info = self.tab_manager.borrow().get_tab(id).cloned();
        if let Some(tab) = tab_info {
            self.progress_bar.set_visible(tab.is_loading);
            self.progress_bar.set_fraction(tab.load_progress);

            self.navbar.set_can_go_back(tab.can_go_back);
            self.navbar.set_can_go_forward(tab.can_go_forward);
            self.navbar.set_loading(tab.is_loading);
            self.navbar.set_url(&tab.url);

            let is_bm = self
                .storage
                .as_ref()
                .and_then(|s| s.is_bookmarked(&tab.url).ok())
                .unwrap_or(false);
            self.navbar.set_bookmarked(is_bm);

            let title_str = tab.display_title();
            self.window.set_title(Some(&format!("{title_str} — The Browser")));
        }

        self.save_current_session();
    }

    pub fn select_tab_index(self: &Rc<Self>, index: usize) {
        let target_id = {
            let mut mgr = self.tab_manager.borrow_mut();
            if mgr.select_tab_at(index) {
                mgr.active_tab_id()
            } else {
                None
            }
        };
        if let Some(id) = target_id {
            self.select_tab(id);
        }
    }

    pub fn close_tab(self: &Rc<Self>, id: TabId) {
        let is_last = self.tab_manager.borrow().tab_count() <= 1;
        if is_last {
            let is_already_home = {
                let mgr = self.tab_manager.borrow();
                mgr.get_tab(id).map(|t| t.url == "thebrowser://home").unwrap_or(false)
            };

            if is_already_home {
                if let Some(ref storage) = self.storage {
                    let _ = storage.clear_session();
                }
                self.window.close();
            } else {
                self.navigate_active("thebrowser://home");
                self.tab_manager.borrow_mut().update_title(id, "Home".to_string());
                self.sidebar.update_title(id, "Home");
                self.tab_strip.update_title(id, "Home");
                self.save_current_session();
            }
            return;
        }

        let new_active_id = self.tab_manager.borrow_mut().close_tab(id);

        if let Some(view) = self.engine_views.borrow_mut().remove(&id) {
            self.stack.remove(view.widget());
        }

        self.sidebar.remove_tab(id);
        self.tab_strip.remove_tab(id);

        if let Some(active_id) = new_active_id {
            self.select_tab(active_id);
        }

        self.save_current_session();
    }
}
