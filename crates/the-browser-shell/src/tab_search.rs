//! Interactive tab search and quick switcher dialog with Space awareness.

use gtk4::gdk::Key;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Entry, EventControllerKey, Label, ListBox, ListBoxRow,
    Orientation, PolicyType, ScrolledWindow, Window,
};
use std::cell::RefCell;
use std::rc::Rc;
use the_browser_core::{SpaceInfo, TabId, TabInfo};

type TabSelectCallback = Rc<RefCell<Option<Box<dyn Fn(TabId)>>>>;

pub struct TabSearchDialog {
    window: Window,
    entry: Entry,
    list_box: ListBox,
    open_tabs: Rc<RefCell<Vec<TabInfo>>>,
    spaces: Rc<RefCell<Vec<SpaceInfo>>>,
    filtered_ids: Rc<RefCell<Vec<TabId>>>,
    on_select_tab: TabSelectCallback,
}

impl TabSearchDialog {
    pub fn new(parent: &impl IsA<gtk4::Window>) -> Self {
        let window = Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Search Tabs")
            .default_width(580)
            .default_height(440)
            .decorated(false)
            .build();

        window.add_css_class("tab-search-window");

        let container = GtkBox::new(Orientation::Vertical, 0);
        container.add_css_class("tab-search-card");

        // 1. Search Entry Row
        let search_row = GtkBox::new(Orientation::Horizontal, 8);
        search_row.add_css_class("tab-search-entry-row");

        let entry = Entry::new();
        entry.add_css_class("tab-search-entry");
        entry.set_placeholder_text(Some("Search open tabs (title, domain, or URL)..."));
        entry.set_primary_icon_name(Some("edit-find-symbolic"));
        entry.set_hexpand(true);
        search_row.append(&entry);

        container.append(&search_row);

        // 2. Results List
        let scrolled = ScrolledWindow::new();
        scrolled.set_policy(PolicyType::Never, PolicyType::Automatic);
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);

        let list_box = ListBox::new();
        list_box.add_css_class("tab-search-list");
        scrolled.set_child(Some(&list_box));

        container.append(&scrolled);
        window.set_child(Some(&container));

        let open_tabs = Rc::new(RefCell::new(Vec::new()));
        let spaces = Rc::new(RefCell::new(Vec::new()));
        let filtered_ids = Rc::new(RefCell::new(Vec::new()));
        let on_select_tab = Rc::new(RefCell::new(None::<Box<dyn Fn(TabId)>>));

        // Connect search entry text change
        {
            let open_tabs = Rc::clone(&open_tabs);
            let spaces = Rc::clone(&spaces);
            let filtered_ids = Rc::clone(&filtered_ids);
            let list_box = list_box.clone();
            entry.connect_changed(move |entry| {
                let query = entry.text().to_string();
                Self::refresh_list(&list_box, &open_tabs.borrow(), &spaces.borrow(), &filtered_ids, &query);
            });
        }

        // Connect list row activation (click or enter)
        {
            let on_select = Rc::clone(&on_select_tab);
            let filtered_ids = Rc::clone(&filtered_ids);
            let win_clone = window.clone();
            list_box.connect_row_activated(move |_, row| {
                let idx = row.index() as usize;
                if let Some(&tab_id) = filtered_ids.borrow().get(idx) {
                    win_clone.close();
                    if let Some(ref cb) = *on_select.borrow() {
                        cb(tab_id);
                    }
                }
            });
        }

        // Keyboard navigation (Arrow keys, Enter, Escape)
        {
            let on_select = Rc::clone(&on_select_tab);
            let filtered_ids = Rc::clone(&filtered_ids);
            let list_box_clone = list_box.clone();
            let win_clone = window.clone();
            let key_ctrl = EventControllerKey::new();

            key_ctrl.connect_key_pressed(move |_, keyval, _, _| {
                match keyval {
                    Key::Escape => {
                        win_clone.close();
                        glib::Propagation::Stop
                    }
                    Key::Return | Key::KP_Enter => {
                        if let Some(selected_row) = list_box_clone.selected_row() {
                            let idx = selected_row.index() as usize;
                            if let Some(&tab_id) = filtered_ids.borrow().get(idx) {
                                win_clone.close();
                                if let Some(ref cb) = *on_select.borrow() {
                                    cb(tab_id);
                                }
                                return glib::Propagation::Stop;
                            }
                        }
                        glib::Propagation::Proceed
                    }
                    Key::Down => {
                        let cur_idx = list_box_clone.selected_row().map(|r| r.index()).unwrap_or(-1);
                        let total = filtered_ids.borrow().len() as i32;
                        if total > 0 {
                            let next_idx = (cur_idx + 1).min(total - 1);
                            if let Some(row) = list_box_clone.row_at_index(next_idx) {
                                list_box_clone.select_row(Some(&row));
                            }
                        }
                        glib::Propagation::Stop
                    }
                    Key::Up => {
                        let cur_idx = list_box_clone.selected_row().map(|r| r.index()).unwrap_or(0);
                        let total = filtered_ids.borrow().len() as i32;
                        if total > 0 {
                            let prev_idx = (cur_idx - 1).max(0);
                            if let Some(row) = list_box_clone.row_at_index(prev_idx) {
                                list_box_clone.select_row(Some(&row));
                            }
                        }
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            });
            window.add_controller(key_ctrl);
        }

        Self {
            window,
            entry,
            list_box,
            open_tabs,
            spaces,
            filtered_ids,
            on_select_tab,
        }
    }

    pub fn show_with_tabs(&self, tabs: &[TabInfo], active_tab_id: Option<TabId>, spaces: &[SpaceInfo]) {
        *self.open_tabs.borrow_mut() = tabs.to_vec();
        *self.spaces.borrow_mut() = spaces.to_vec();
        self.entry.set_text("");
        Self::refresh_list(
            &self.list_box,
            tabs,
            spaces,
            &self.filtered_ids,
            "",
        );

        // Highlight currently active tab if present
        if let Some(act_id) = active_tab_id
            && let Some(idx) = self.filtered_ids.borrow().iter().position(|id| *id == act_id)
            && let Some(row) = self.list_box.row_at_index(idx as i32)
        {
            self.list_box.select_row(Some(&row));
        } else if let Some(row) = self.list_box.row_at_index(0) {
            self.list_box.select_row(Some(&row));
        }

        self.window.present();
        self.entry.grab_focus();
    }

    fn refresh_list(
        list_box: &ListBox,
        tabs: &[TabInfo],
        spaces: &[SpaceInfo],
        filtered_ids: &Rc<RefCell<Vec<TabId>>>,
        query: &str,
    ) {
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let mut ids = Vec::new();
        for tab in tabs {
            if tab.matches_query(query) {
                ids.push(tab.id);

                let row = ListBoxRow::new();
                row.add_css_class("tab-search-row");

                let item_box = GtkBox::new(Orientation::Horizontal, 10);
                item_box.set_margin_top(6);
                item_box.set_margin_bottom(6);
                item_box.set_margin_start(10);
                item_box.set_margin_end(10);

                let monogram_text = monogram_for(&tab.title);
                let monogram = Label::new(Some(&monogram_text));
                monogram.add_css_class("tab-monogram");
                item_box.append(&monogram);

                let info_box = GtkBox::new(Orientation::Vertical, 2);
                info_box.set_hexpand(true);

                let title_label = Label::new(Some(tab.display_title()));
                title_label.add_css_class("tab-search-title");
                title_label.set_halign(Align::Start);
                title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                info_box.append(&title_label);

                let url_display = if tab.url.is_empty() || tab.url == "about:blank" {
                    "Empty page".to_string()
                } else {
                    tab.hostname().to_string()
                };

                let url_label = Label::new(Some(&url_display));
                url_label.add_css_class("tab-search-url");
                url_label.set_halign(Align::Start);
                url_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                info_box.append(&url_label);

                item_box.append(&info_box);

                // Space Badge
                if let Some(sp) = spaces.iter().find(|s| s.id == tab.space_id) {
                    let space_badge = Label::new(Some(&sp.name));
                    space_badge.add_css_class("tab-badge-space");
                    item_box.append(&space_badge);
                }

                if tab.is_playing_audio {
                    let audio_badge = Label::new(Some("🔊 Audio"));
                    audio_badge.add_css_class("tab-badge");
                    item_box.append(&audio_badge);
                }

                if tab.is_pinned {
                    let pin_badge = Label::new(Some("Pinned"));
                    pin_badge.add_css_class("tab-badge");
                    item_box.append(&pin_badge);
                }

                if tab.is_lazy {
                    let lazy_badge = Label::new(Some("Sleeping"));
                    lazy_badge.add_css_class("tab-badge-dim");
                    item_box.append(&lazy_badge);
                }

                if tab.load_failed {
                    let fail_badge = Label::new(Some("Failed"));
                    fail_badge.add_css_class("tab-badge-fail");
                    item_box.append(&fail_badge);
                }

                row.set_child(Some(&item_box));
                list_box.append(&row);
            }
        }

        *filtered_ids.borrow_mut() = ids;
    }

    pub fn connect_select_tab<F: Fn(TabId) + 'static>(&self, f: F) {
        *self.on_select_tab.borrow_mut() = Some(Box::new(f));
    }
}

fn monogram_for(text: &str) -> String {
    let clean = text
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
        .trim();

    clean
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "●".to_string())
}
