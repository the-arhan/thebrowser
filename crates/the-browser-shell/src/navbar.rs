//! Comprehensive browser navigation toolbar component.

use gtk4::gdk::Key;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Entry, EventControllerFocus, EventControllerKey, Image, Label,
    MenuButton, Orientation, Popover, Separator,
};
use std::cell::RefCell;
use std::rc::Rc;

type ActionCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;
type StringCallback = Rc<RefCell<Option<Box<dyn Fn(&str)>>>>;

pub struct NavBar {
    root: GtkBox,
    back_btn: Button,
    forward_btn: Button,
    reload_btn: Button,
    _home_btn: Button,
    address_entry: Entry,
    star_btn: Button,
    _settings_btn: Button,
    _menu_btn: MenuButton,
    is_loading: Rc<RefCell<bool>>,
    is_bookmarked: Rc<RefCell<bool>>,
    current_url: Rc<RefCell<String>>,
    is_entry_focused: Rc<RefCell<bool>>,
    on_back: ActionCallback,
    on_forward: ActionCallback,
    on_reload: ActionCallback,
    on_stop: ActionCallback,
    on_home: ActionCallback,
    on_navigate: StringCallback,
    on_toggle_bookmark: ActionCallback,
    on_open_settings: ActionCallback,
    on_search_tabs: ActionCallback,
    on_snooze_tabs: ActionCallback,
    on_new_tab: ActionCallback,
    on_reopen_tab: ActionCallback,
    on_toggle_sidebar: ActionCallback,
    on_toggle_layout: ActionCallback,
    on_clear_history: ActionCallback,
    on_about: ActionCallback,
}

impl NavBar {
    pub fn new() -> Self {
        let root = GtkBox::new(Orientation::Horizontal, 6);
        root.add_css_class("nav-bar");
        root.set_hexpand(true);
        root.set_valign(Align::Center);

        // 1. Navigation controls (Back, Forward, Reload/Stop, Home)
        let nav_group = GtkBox::new(Orientation::Horizontal, 2);
        nav_group.add_css_class("nav-button-group");

        let back_btn = Button::from_icon_name("go-previous-symbolic");
        back_btn.add_css_class("nav-button");
        back_btn.set_tooltip_text(Some("Back (Alt+Left)"));
        back_btn.set_sensitive(false);
        nav_group.append(&back_btn);

        let forward_btn = Button::from_icon_name("go-next-symbolic");
        forward_btn.add_css_class("nav-button");
        forward_btn.set_tooltip_text(Some("Forward (Alt+Right)"));
        forward_btn.set_sensitive(false);
        nav_group.append(&forward_btn);

        let reload_btn = Button::from_icon_name("view-refresh-symbolic");
        reload_btn.add_css_class("nav-button");
        reload_btn.set_tooltip_text(Some("Reload (Ctrl+R / F5)"));
        nav_group.append(&reload_btn);

        let home_btn = Button::from_icon_name("go-home-symbolic");
        home_btn.add_css_class("nav-button");
        home_btn.set_tooltip_text(Some("Home (thebrowser://home)"));
        nav_group.append(&home_btn);

        root.append(&nav_group);

        // 2. Central Address Bar + Bookmark button
        let address_box = GtkBox::new(Orientation::Horizontal, 0);
        address_box.add_css_class("address-container");
        address_box.set_hexpand(true);

        let address_entry = Entry::new();
        address_entry.add_css_class("nav-address-entry");
        address_entry.set_placeholder_text(Some("Search or enter address..."));
        address_entry.set_hexpand(true);
        address_entry.set_primary_icon_name(Some("edit-find-symbolic"));
        address_box.append(&address_entry);

        let star_btn = Button::from_icon_name("non-starred-symbolic");
        star_btn.add_css_class("star-button");
        star_btn.set_tooltip_text(Some("Bookmark this page (Ctrl+D)"));
        address_box.append(&star_btn);

        root.append(&address_box);

        // 3. Settings & Action Menu button
        let actions_group = GtkBox::new(Orientation::Horizontal, 2);
        actions_group.add_css_class("nav-actions-group");

        let settings_btn = Button::from_icon_name("emblem-system-symbolic");
        settings_btn.add_css_class("nav-button");
        settings_btn.set_tooltip_text(Some("Settings (thebrowser://settings)"));
        actions_group.append(&settings_btn);

        let menu_btn = MenuButton::new();
        menu_btn.set_icon_name("open-menu-symbolic");
        menu_btn.add_css_class("nav-button");
        menu_btn.set_tooltip_text(Some("Main Menu"));
        actions_group.append(&menu_btn);

        root.append(&actions_group);

        let is_loading = Rc::new(RefCell::new(false));
        let is_bookmarked = Rc::new(RefCell::new(false));
        let current_url = Rc::new(RefCell::new(String::new()));
        let is_entry_focused = Rc::new(RefCell::new(false));

        let on_back: ActionCallback = Rc::new(RefCell::new(None));
        let on_forward: ActionCallback = Rc::new(RefCell::new(None));
        let on_reload: ActionCallback = Rc::new(RefCell::new(None));
        let on_stop: ActionCallback = Rc::new(RefCell::new(None));
        let on_home: ActionCallback = Rc::new(RefCell::new(None));
        let on_navigate: StringCallback = Rc::new(RefCell::new(None));
        let on_toggle_bookmark: ActionCallback = Rc::new(RefCell::new(None));
        let on_open_settings: ActionCallback = Rc::new(RefCell::new(None));
        let on_search_tabs: ActionCallback = Rc::new(RefCell::new(None));
        let on_snooze_tabs: ActionCallback = Rc::new(RefCell::new(None));
        let on_new_tab: ActionCallback = Rc::new(RefCell::new(None));
        let on_reopen_tab: ActionCallback = Rc::new(RefCell::new(None));
        let on_toggle_sidebar: ActionCallback = Rc::new(RefCell::new(None));
        let on_toggle_layout: ActionCallback = Rc::new(RefCell::new(None));
        let on_clear_history: ActionCallback = Rc::new(RefCell::new(None));
        let on_about: ActionCallback = Rc::new(RefCell::new(None));

        // Connect button signals
        {
            let on_back = Rc::clone(&on_back);
            back_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_back.borrow() {
                    cb();
                }
            });
        }

        {
            let on_forward = Rc::clone(&on_forward);
            forward_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_forward.borrow() {
                    cb();
                }
            });
        }

        {
            let on_reload = Rc::clone(&on_reload);
            let on_stop = Rc::clone(&on_stop);
            let is_loading = Rc::clone(&is_loading);
            reload_btn.connect_clicked(move |_| {
                if *is_loading.borrow() {
                    if let Some(ref cb) = *on_stop.borrow() {
                        cb();
                    }
                } else if let Some(ref cb) = *on_reload.borrow() {
                    cb();
                }
            });
        }

        {
            let on_home = Rc::clone(&on_home);
            home_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_home.borrow() {
                    cb();
                }
            });
        }

        {
            let on_toggle_bookmark = Rc::clone(&on_toggle_bookmark);
            star_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_toggle_bookmark.borrow() {
                    cb();
                }
            });
        }

        {
            let on_open_settings = Rc::clone(&on_open_settings);
            settings_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_open_settings.borrow() {
                    cb();
                }
            });
        }

        // Track focus in address entry
        {
            let is_entry_focused_enter = Rc::clone(&is_entry_focused);
            let is_entry_focused_exit = Rc::clone(&is_entry_focused);
            let focus_ctrl = EventControllerFocus::new();
            focus_ctrl.connect_enter(move |_| {
                *is_entry_focused_enter.borrow_mut() = true;
            });
            let current_url_exit = Rc::clone(&current_url);
            let entry_exit = address_entry.clone();
            focus_ctrl.connect_leave(move |_| {
                *is_entry_focused_exit.borrow_mut() = false;
                // If exited without submitting, restore active URL
                let cur = current_url_exit.borrow().clone();
                if !cur.is_empty() && cur != "about:blank" {
                    entry_exit.set_text(&cur);
                }
            });
            address_entry.add_controller(focus_ctrl);
        }

        // Address entry activate (Enter)
        {
            let on_navigate = Rc::clone(&on_navigate);
            address_entry.connect_activate(move |entry| {
                let text = entry.text().to_string();
                if let Some(ref cb) = *on_navigate.borrow() {
                    cb(&text);
                }
            });
        }

        // Key handler for address entry (Escape restores URL and unfocuses)
        {
            let key_ctrl = EventControllerKey::new();
            let current_url = Rc::clone(&current_url);
            let entry_clone = address_entry.clone();
            key_ctrl.connect_key_pressed(move |_, keyval, _, _| {
                if keyval == Key::Escape {
                    let cur = current_url.borrow().clone();
                    entry_clone.set_text(&cur);
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            });
            address_entry.add_controller(key_ctrl);
        }

        // Build Action Menu Popover
        let menu_popover = Popover::new();
        menu_popover.add_css_class("nav-menu-popover");
        let menu_box = GtkBox::new(Orientation::Vertical, 2);
        menu_box.set_margin_top(4);
        menu_box.set_margin_bottom(4);
        menu_box.set_margin_start(4);
        menu_box.set_margin_end(4);

        // Helper to add menu action button
        let add_menu_item = |box_widget: &GtkBox,
                             popover: &Popover,
                             icon: &str,
                             label_text: &str,
                             shortcut: &str,
                             action: ActionCallback| {
            let row = Button::new();
            row.add_css_class("menu-item-button");

            let row_box = GtkBox::new(Orientation::Horizontal, 8);
            row_box.set_hexpand(true);

            let img = Image::from_icon_name(icon);
            row_box.append(&img);

            let lbl = Label::new(Some(label_text));
            lbl.set_halign(Align::Start);
            lbl.set_hexpand(true);
            row_box.append(&lbl);

            if !shortcut.is_empty() {
                let sc_lbl = Label::new(Some(shortcut));
                sc_lbl.add_css_class("menu-shortcut-label");
                row_box.append(&sc_lbl);
            }

            row.set_child(Some(&row_box));

            let pop_clone = popover.clone();
            row.connect_clicked(move |_| {
                pop_clone.popdown();
                if let Some(ref cb) = *action.borrow() {
                    cb();
                }
            });

            box_widget.append(&row);
        };

        add_menu_item(&menu_box, &menu_popover, "tab-new-symbolic", "New Tab", "Ctrl+T", Rc::clone(&on_new_tab));
        add_menu_item(&menu_box, &menu_popover, "edit-undo-symbolic", "Reopen Closed Tab", "Ctrl+Shift+T", Rc::clone(&on_reopen_tab));
        add_menu_item(&menu_box, &menu_popover, "system-search-symbolic", "Search Tabs", "Ctrl+Shift+A", Rc::clone(&on_search_tabs));
        add_menu_item(&menu_box, &menu_popover, "media-playback-pause-symbolic", "Snooze Background Tabs", "", Rc::clone(&on_snooze_tabs));
        
        menu_box.append(&Separator::new(Orientation::Horizontal));
        
        add_menu_item(&menu_box, &menu_popover, "view-sidebar-start-symbolic", "Toggle Sidebar", "Ctrl+S", Rc::clone(&on_toggle_sidebar));
        add_menu_item(&menu_box, &menu_popover, "view-continuous-symbolic", "Toggle Tab Layout", "Ctrl+Shift+S", Rc::clone(&on_toggle_layout));
        
        menu_box.append(&Separator::new(Orientation::Horizontal));
        
        add_menu_item(&menu_box, &menu_popover, "emblem-system-symbolic", "Settings", "", Rc::clone(&on_open_settings));
        add_menu_item(&menu_box, &menu_popover, "user-trash-symbolic", "Clear Browsing History", "", Rc::clone(&on_clear_history));
        add_menu_item(&menu_box, &menu_popover, "help-about-symbolic", "About The Browser", "", Rc::clone(&on_about));

        menu_popover.set_child(Some(&menu_box));
        menu_btn.set_popover(Some(&menu_popover));

        Self {
            root,
            back_btn,
            forward_btn,
            reload_btn,
            _home_btn: home_btn,
            address_entry,
            star_btn,
            _settings_btn: settings_btn,
            _menu_btn: menu_btn,
            is_loading,
            is_bookmarked,
            current_url,
            is_entry_focused,
            on_back,
            on_forward,
            on_reload,
            on_stop,
            on_home,
            on_navigate,
            on_toggle_bookmark,
            on_open_settings,
            on_search_tabs,
            on_snooze_tabs,
            on_new_tab,
            on_reopen_tab,
            on_toggle_sidebar,
            on_toggle_layout,
            on_clear_history,
            on_about,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.root
    }

    pub fn set_can_go_back(&self, can_back: bool) {
        self.back_btn.set_sensitive(can_back);
    }

    pub fn set_can_go_forward(&self, can_forward: bool) {
        self.forward_btn.set_sensitive(can_forward);
    }

    pub fn set_loading(&self, loading: bool) {
        *self.is_loading.borrow_mut() = loading;
        if loading {
            self.reload_btn.set_icon_name("process-stop-symbolic");
            self.reload_btn.set_tooltip_text(Some("Stop loading (Escape)"));
        } else {
            self.reload_btn.set_icon_name("view-refresh-symbolic");
            self.reload_btn.set_tooltip_text(Some("Reload (Ctrl+R / F5)"));
        }
    }

    pub fn set_url(&self, url: &str) {
        *self.current_url.borrow_mut() = url.to_string();
        // Only update text if the user is not actively typing in the address entry
        if !*self.is_entry_focused.borrow() {
            if url == "about:blank" || url == "thebrowser://home" {
                self.address_entry.set_text("");
            } else {
                self.address_entry.set_text(url);
            }
        }
    }

    pub fn set_bookmarked(&self, bookmarked: bool) {
        *self.is_bookmarked.borrow_mut() = bookmarked;
        if bookmarked {
            self.star_btn.set_icon_name("starred-symbolic");
            self.star_btn.add_css_class("starred");
            self.star_btn.set_tooltip_text(Some("Remove bookmark"));
        } else {
            self.star_btn.set_icon_name("non-starred-symbolic");
            self.star_btn.remove_css_class("starred");
            self.star_btn.set_tooltip_text(Some("Bookmark this page (Ctrl+D)"));
        }
    }

    pub fn focus_address(&self) {
        self.address_entry.grab_focus();
        self.address_entry.select_region(0, -1);
    }

    pub fn connect_back<F: Fn() + 'static>(&self, f: F) {
        *self.on_back.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_forward<F: Fn() + 'static>(&self, f: F) {
        *self.on_forward.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_reload<F: Fn() + 'static>(&self, f: F) {
        *self.on_reload.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_stop<F: Fn() + 'static>(&self, f: F) {
        *self.on_stop.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_home<F: Fn() + 'static>(&self, f: F) {
        *self.on_home.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_navigate<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_navigate.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_toggle_bookmark<F: Fn() + 'static>(&self, f: F) {
        *self.on_toggle_bookmark.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_open_settings<F: Fn() + 'static>(&self, f: F) {
        *self.on_open_settings.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_search_tabs<F: Fn() + 'static>(&self, f: F) {
        *self.on_search_tabs.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_snooze_tabs<F: Fn() + 'static>(&self, f: F) {
        *self.on_snooze_tabs.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_new_tab<F: Fn() + 'static>(&self, f: F) {
        *self.on_new_tab.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_reopen_tab<F: Fn() + 'static>(&self, f: F) {
        *self.on_reopen_tab.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_toggle_sidebar<F: Fn() + 'static>(&self, f: F) {
        *self.on_toggle_sidebar.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_toggle_layout<F: Fn() + 'static>(&self, f: F) {
        *self.on_toggle_layout.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_clear_history<F: Fn() + 'static>(&self, f: F) {
        *self.on_clear_history.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_about<F: Fn() + 'static>(&self, f: F) {
        *self.on_about.borrow_mut() = Some(Box::new(f));
    }
}

impl Default for NavBar {
    fn default() -> Self {
        Self::new()
    }
}
