//! Native Libadwaita Preferences Window for browser configuration.

use gtk4::prelude::*;
use gtk4::{Align, Button, Entry, StringList, Switch};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage, PreferencesWindow};
use std::cell::RefCell;
use std::rc::Rc;
use the_browser_storage::{BrowserSettings, StorageManager};

type SettingsChangedCallback = Rc<RefCell<Option<Box<dyn Fn(&BrowserSettings)>>>>;
type ActionCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

pub struct SettingsDialog {
    window: PreferencesWindow,
    _settings: Rc<RefCell<BrowserSettings>>,
    _storage: Option<Rc<StorageManager>>,
    on_settings_changed: SettingsChangedCallback,
    on_clear_history: ActionCallback,
}

impl SettingsDialog {
    pub fn new(
        parent: &impl IsA<gtk4::Window>,
        initial_settings: BrowserSettings,
        storage: Option<Rc<StorageManager>>,
    ) -> Self {
        let window = PreferencesWindow::builder()
            .transient_for(parent)
            .modal(true)
            .title("Settings")
            .default_width(620)
            .default_height(580)
            .build();

        let settings = Rc::new(RefCell::new(initial_settings));
        let on_settings_changed = Rc::new(RefCell::new(None::<Box<dyn Fn(&BrowserSettings)>>));
        let on_clear_history = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));

        // Helper to notify and persist
        let notify_and_save = {
            let settings = Rc::clone(&settings);
            let storage = storage.clone();
            let on_changed = Rc::clone(&on_settings_changed);
            move || {
                let current = settings.borrow().clone();
                if let Some(ref s) = storage {
                    let _ = s.save_browser_settings(&current);
                }
                if let Some(ref cb) = *on_changed.borrow() {
                    cb(&current);
                }
            }
        };

        // --- PAGE 1: General Preferences ---
        let general_page = PreferencesPage::builder()
            .title("General")
            .icon_name("preferences-system-symbolic")
            .build();

        // Group 1: Appearance
        let appearance_group = PreferencesGroup::builder()
            .title("Appearance")
            .description("Customize the visual presentation of The Browser")
            .build();

        // Theme Combo
        let theme_model = StringList::new(&["Dark", "Light", "System"]);
        let theme_row = ComboRow::builder()
            .title("Theme")
            .subtitle("Interface color scheme")
            .model(&theme_model)
            .build();

        let initial_theme_idx = match settings.borrow().theme.as_str() {
            "light" => 1,
            "system" => 2,
            _ => 0, // default dark
        };
        theme_row.set_selected(initial_theme_idx);

        {
            let settings = Rc::clone(&settings);
            let notify = notify_and_save.clone();
            theme_row.connect_selected_notify(move |row| {
                let choice = match row.selected() {
                    1 => "light",
                    2 => "system",
                    _ => "dark",
                };
                settings.borrow_mut().theme = choice.to_string();
                notify();
            });
        }
        appearance_group.add(&theme_row);

        // Dusk Dark Mode Switch Row
        let dusk_row = ActionRow::builder()
            .title("Dusk Dark Mode")
            .subtitle("Intelligently adapt bright websites to comfortable dark themes")
            .build();
        let dusk_switch = Switch::builder()
            .active(settings.borrow().dusk_dark_mode)
            .valign(Align::Center)
            .build();

        {
            let settings = Rc::clone(&settings);
            let notify = notify_and_save.clone();
            dusk_switch.connect_active_notify(move |sw| {
                settings.borrow_mut().dusk_dark_mode = sw.is_active();
                notify();
            });
        }
        dusk_row.add_suffix(&dusk_switch);
        dusk_row.set_activatable_widget(Some(&dusk_switch));
        appearance_group.add(&dusk_row);
        general_page.add(&appearance_group);

        // Group 2: Search Engine
        let search_group = PreferencesGroup::builder()
            .title("Search")
            .description("Default search engine for omnibox and address bar queries")
            .build();

        let engine_model = StringList::new(&["DuckDuckGo", "Google", "Startpage", "Bing", "Brave"]);
        let engine_row = ComboRow::builder()
            .title("Search Engine")
            .subtitle("Default engine for search queries")
            .model(&engine_model)
            .build();

        let initial_engine_idx = match settings.borrow().search_engine.to_lowercase().as_str() {
            "google" => 1,
            "startpage" => 2,
            "bing" => 3,
            "brave" => 4,
            _ => 0, // DuckDuckGo
        };
        engine_row.set_selected(initial_engine_idx);

        {
            let settings = Rc::clone(&settings);
            let notify = notify_and_save.clone();
            engine_row.connect_selected_notify(move |row| {
                let choice = match row.selected() {
                    1 => "google",
                    2 => "startpage",
                    3 => "bing",
                    4 => "brave",
                    _ => "duckduckgo",
                };
                settings.borrow_mut().search_engine = choice.to_string();
                notify();
            });
        }
        search_group.add(&engine_row);
        general_page.add(&search_group);

        // Group 3: Home & Startup
        let home_group = PreferencesGroup::builder()
            .title("Home and New Tab")
            .description("Configure the built-in home page")
            .build();

        let home_url_row = ActionRow::builder()
            .title("Home Page URL")
            .subtitle("URL loaded when opening home")
            .build();
        let home_entry = Entry::builder()
            .text(&settings.borrow().home_url)
            .valign(Align::Center)
            .hexpand(true)
            .build();

        {
            let settings = Rc::clone(&settings);
            let notify = notify_and_save.clone();
            home_entry.connect_changed(move |entry| {
                settings.borrow_mut().home_url = entry.text().to_string();
                notify();
            });
        }
        home_url_row.add_suffix(&home_entry);
        home_group.add(&home_url_row);

        let shortcuts_row = ActionRow::builder()
            .title("Show Quick Shortcuts")
            .subtitle("Display pinned shortcuts grid on the home page")
            .build();
        let shortcuts_switch = Switch::builder()
            .active(settings.borrow().show_shortcuts)
            .valign(Align::Center)
            .build();

        {
            let settings = Rc::clone(&settings);
            let notify = notify_and_save.clone();
            shortcuts_switch.connect_active_notify(move |sw| {
                settings.borrow_mut().show_shortcuts = sw.is_active();
                notify();
            });
        }
        shortcuts_row.add_suffix(&shortcuts_switch);
        shortcuts_row.set_activatable_widget(Some(&shortcuts_switch));
        home_group.add(&shortcuts_row);

        let recent_row = ActionRow::builder()
            .title("Show Recent History")
            .subtitle("Display recent browsing history list on the home page")
            .build();
        let recent_switch = Switch::builder()
            .active(settings.borrow().show_recent)
            .valign(Align::Center)
            .build();

        {
            let settings = Rc::clone(&settings);
            let notify = notify_and_save.clone();
            recent_switch.connect_active_notify(move |sw| {
                settings.borrow_mut().show_recent = sw.is_active();
                notify();
            });
        }
        recent_row.add_suffix(&recent_switch);
        recent_row.set_activatable_widget(Some(&recent_switch));
        home_group.add(&recent_row);
        general_page.add(&home_group);

        // Group 3.5: Resource Management (Tab Snoozing)
        let resource_group = PreferencesGroup::builder()
            .title("Resource Management")
            .description("Configure automatic background tab unloading")
            .build();

        let snooze_row = ActionRow::builder()
            .title("Intelligent Tab Snoozing")
            .subtitle("Automatically unload inactive background tabs to reduce memory usage")
            .build();

        let snooze_switch = Switch::builder()
            .active(settings.borrow().tab_snooze_enabled)
            .valign(Align::Center)
            .build();

        {
            let settings = Rc::clone(&settings);
            let notify = notify_and_save.clone();
            snooze_switch.connect_active_notify(move |sw| {
                settings.borrow_mut().tab_snooze_enabled = sw.is_active();
                notify();
            });
        }
        snooze_row.add_suffix(&snooze_switch);
        snooze_row.set_activatable_widget(Some(&snooze_switch));
        resource_group.add(&snooze_row);

        let threshold_row = ComboRow::builder()
            .title("Inactivity Threshold")
            .subtitle("Time elapsed before an inactive tab becomes eligible for snoozing")
            .build();

        let threshold_model = StringList::new(&["5 minutes", "15 minutes", "30 minutes", "1 hour"]);
        threshold_row.set_model(Some(&threshold_model));
        let selected_thresh_idx = match settings.borrow().tab_snooze_threshold_mins {
            5 => 0,
            15 => 1,
            60 => 3,
            _ => 2,
        };
        threshold_row.set_selected(selected_thresh_idx);

        {
            let settings = Rc::clone(&settings);
            let notify = notify_and_save.clone();
            threshold_row.connect_selected_notify(move |row| {
                let mins = match row.selected() {
                    0 => 5,
                    1 => 15,
                    3 => 60,
                    _ => 30,
                };
                settings.borrow_mut().tab_snooze_threshold_mins = mins;
                notify();
            });
        }
        resource_group.add(&threshold_row);
        general_page.add(&resource_group);

        // Group 4: Data & Privacy
        let data_group = PreferencesGroup::builder()
            .title("Privacy and Data")
            .description("Manage local browsing data and history")
            .build();

        let clear_row = ActionRow::builder()
            .title("Clear Browsing History")
            .subtitle("Delete all visited page records and search terms")
            .build();

        let clear_btn = Button::with_label("Clear");
        clear_btn.add_css_class("destructive-action");
        clear_btn.set_valign(Align::Center);

        {
            let on_clear = Rc::clone(&on_clear_history);
            let storage = storage.clone();
            clear_btn.connect_clicked(move |btn| {
                if let Some(ref s) = storage {
                    let _ = s.clear_history();
                }
                if let Some(ref cb) = *on_clear.borrow() {
                    cb();
                }
                btn.set_sensitive(false);
                btn.set_label("Cleared");
            });
        }
        clear_row.add_suffix(&clear_btn);
        data_group.add(&clear_row);
        general_page.add(&data_group);

        window.add(&general_page);

        // --- PAGE 2: About ---
        let about_page = PreferencesPage::builder()
            .title("About")
            .icon_name("help-about-symbolic")
            .build();

        let about_group = PreferencesGroup::builder()
            .title("The Browser")
            .description("Independent Linux-native desktop browser")
            .build();

        let version_row = ActionRow::builder()
            .title("Version")
            .subtitle("v0.1.0 (Alpha Prototype)")
            .build();
        about_group.add(&version_row);

        let platform_row = ActionRow::builder()
            .title("Platform")
            .subtitle("Arch Linux / Wayland and X11")
            .build();
        about_group.add(&platform_row);

        let engine_info_row = ActionRow::builder()
            .title("Engine")
            .subtitle("WebKitGTK 6.0")
            .build();
        about_group.add(&engine_info_row);

        let ui_row = ActionRow::builder()
            .title("Desktop UI")
            .subtitle("GTK4 + Libadwaita")
            .build();
        about_group.add(&ui_row);

        let storage_row = ActionRow::builder()
            .title("Persistence")
            .subtitle("SQLite (WAL mode + FTS5)")
            .build();
        about_group.add(&storage_row);

        let license_row = ActionRow::builder()
            .title("License")
            .subtitle("MIT Open Source")
            .build();
        about_group.add(&license_row);

        about_page.add(&about_group);
        window.add(&about_page);

        Self {
            window,
            _settings: settings,
            _storage: storage,
            on_settings_changed,
            on_clear_history,
        }
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn connect_settings_changed<F: Fn(&BrowserSettings) + 'static>(&self, f: F) {
        *self.on_settings_changed.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_clear_history<F: Fn() + 'static>(&self, f: F) {
        *self.on_clear_history.borrow_mut() = Some(Box::new(f));
    }
}
