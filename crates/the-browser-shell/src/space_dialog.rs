//! Modal dialog for creating and editing isolated Spaces.

use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, CheckButton, Entry, FlowBox, Label,
    Orientation, Window,
};
use std::cell::RefCell;
use std::rc::Rc;
use the_browser_core::{SpaceId, SpaceInfo};

pub const ACCENT_COLORS: &[(&str, &str, &str)] = &[
    ("Indigo", "#6366f1", "accent-indigo"),
    ("Cyan", "#06b6d4", "accent-cyan"),
    ("Emerald", "#10b981", "accent-emerald"),
    ("Amber", "#f59e0b", "accent-amber"),
    ("Rose", "#f43f5e", "accent-rose"),
    ("Purple", "#a855f7", "accent-purple"),
    ("Graphite", "#64748b", "accent-graphite"),
];

pub const ICONS: &[(&str, &str)] = &[
    ("Personal", "user-symbolic"),
    ("Work", "folder-symbolic"),
    ("Starred", "starred-symbolic"),
    ("Science", "applications-science-symbolic"),
    ("Projects", "view-grid-symbolic"),
    ("Web", "globe-symbolic"),
];

pub enum SpaceDialogResult {
    Save(SpaceInfo),
    Delete { space_id: SpaceId, delete_data: bool },
}

type ResultCallback = Rc<RefCell<Option<Box<dyn Fn(SpaceDialogResult)>>>>;

pub struct SpaceDialog {
    window: Window,
    on_result: ResultCallback,
}

impl SpaceDialog {
    pub fn new(
        parent: &impl IsA<gtk4::Window>,
        existing_space: Option<&SpaceInfo>,
        can_delete: bool,
    ) -> Self {
        let is_edit = existing_space.is_some();
        let title_text = if is_edit { "Edit Space" } else { "Create Space" };

        let window = Window::builder()
            .transient_for(parent)
            .modal(true)
            .title(title_text)
            .default_width(420)
            .default_height(480)
            .resizable(false)
            .build();

        window.add_css_class("space-dialog-window");

        let container = GtkBox::new(Orientation::Vertical, 16);
        container.add_css_class("space-dialog-card");
        container.set_margin_top(20);
        container.set_margin_bottom(20);
        container.set_margin_start(20);
        container.set_margin_end(20);

        // Header Title
        let header = Label::new(Some(title_text));
        header.add_css_class("space-dialog-title");
        header.set_halign(Align::Start);
        container.append(&header);

        // Space Name Field
        let name_label = Label::new(Some("Space Name"));
        name_label.add_css_class("space-field-label");
        name_label.set_halign(Align::Start);
        container.append(&name_label);

        let name_entry = Entry::new();
        name_entry.add_css_class("space-name-entry");
        name_entry.set_placeholder_text(Some("e.g. Work, Research, Personal..."));
        if let Some(sp) = existing_space {
            name_entry.set_text(&sp.name);
        } else {
            name_entry.set_text("New Space");
        }
        container.append(&name_entry);

        // Accent Color Palette
        let color_label = Label::new(Some("Accent Color"));
        color_label.add_css_class("space-field-label");
        color_label.set_halign(Align::Start);
        container.append(&color_label);

        let selected_color = Rc::new(RefCell::new(
            existing_space
                .map(|s| s.accent_color.clone())
                .unwrap_or_else(|| "#6366f1".to_string()),
        ));

        let color_flow = FlowBox::new();
        color_flow.set_max_children_per_line(7);
        color_flow.set_selection_mode(gtk4::SelectionMode::None);
        color_flow.set_row_spacing(6);
        color_flow.set_column_spacing(6);

        let color_buttons = Rc::new(RefCell::new(Vec::<Button>::new()));

        for (name, hex, css_class) in ACCENT_COLORS {
            let btn = Button::new();
            btn.add_css_class("accent-color-btn");
            btn.add_css_class(css_class);
            btn.set_tooltip_text(Some(name));
            btn.set_size_request(32, 32);

            if *selected_color.borrow() == *hex {
                btn.add_css_class("selected");
            }

            let sel_col = Rc::clone(&selected_color);
            let hex_str = hex.to_string();
            let all_btns = Rc::clone(&color_buttons);
            let btn_clone = btn.clone();

            btn.connect_clicked(move |_| {
                *sel_col.borrow_mut() = hex_str.clone();
                for b in all_btns.borrow().iter() {
                    b.remove_css_class("selected");
                }
                btn_clone.add_css_class("selected");
            });

            color_buttons.borrow_mut().push(btn.clone());
            color_flow.append(&btn);
        }
        container.append(&color_flow);

        // Icon Selector
        let icon_label = Label::new(Some("Icon"));
        icon_label.add_css_class("space-field-label");
        icon_label.set_halign(Align::Start);
        container.append(&icon_label);

        let selected_icon = Rc::new(RefCell::new(
            existing_space
                .and_then(|s| s.icon.clone())
                .unwrap_or_else(|| "user-symbolic".to_string()),
        ));

        let icon_flow = FlowBox::new();
        icon_flow.set_max_children_per_line(6);
        icon_flow.set_selection_mode(gtk4::SelectionMode::None);
        icon_flow.set_row_spacing(6);
        icon_flow.set_column_spacing(6);

        let icon_buttons = Rc::new(RefCell::new(Vec::<Button>::new()));

        for (desc, icon_name) in ICONS {
            let btn = Button::from_icon_name(icon_name);
            btn.add_css_class("space-icon-btn");
            btn.set_tooltip_text(Some(desc));
            btn.set_size_request(36, 36);

            if *selected_icon.borrow() == *icon_name {
                btn.add_css_class("selected");
            }

            let sel_ic = Rc::clone(&selected_icon);
            let name_str = icon_name.to_string();
            let all_ic_btns = Rc::clone(&icon_buttons);
            let btn_clone = btn.clone();

            btn.connect_clicked(move |_| {
                *sel_ic.borrow_mut() = name_str.clone();
                for b in all_ic_btns.borrow().iter() {
                    b.remove_css_class("selected");
                }
                btn_clone.add_css_class("selected");
            });

            icon_buttons.borrow_mut().push(btn.clone());
            icon_flow.append(&btn);
        }
        container.append(&icon_flow);

        // Delete checkbox & button (if editing existing space and deletion allowed)
        let delete_box = GtkBox::new(Orientation::Vertical, 6);
        let delete_check = CheckButton::with_label("Delete persistent website data (cookies, storage, cache)");
        delete_check.set_active(true);

        if is_edit && can_delete {
            delete_box.append(&delete_check);
        }

        let on_result: ResultCallback = Rc::new(RefCell::new(None));

        // Action Buttons Row (Save, Cancel, Delete)
        let actions_row = GtkBox::new(Orientation::Horizontal, 8);
        actions_row.set_margin_top(16);

        let cancel_btn = Button::with_label("Cancel");
        cancel_btn.add_css_class("flat");
        let win_cancel = window.clone();
        cancel_btn.connect_clicked(move |_| {
            win_cancel.close();
        });
        actions_row.append(&cancel_btn);

        // Spacer
        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        actions_row.append(&spacer);

        if is_edit && can_delete && let Some(sp) = existing_space {
            let del_btn = Button::with_label("Delete Space");
            del_btn.add_css_class("destructive-action");

            let target_sp_id = sp.id.clone();
            let on_res = Rc::clone(&on_result);
            let win_del = window.clone();
            let chk = delete_check.clone();

            del_btn.connect_clicked(move |_| {
                let delete_data = chk.is_active();
                win_del.close();
                if let Some(ref cb) = *on_res.borrow() {
                    cb(SpaceDialogResult::Delete {
                        space_id: target_sp_id.clone(),
                        delete_data,
                    });
                }
            });

            actions_row.append(&del_btn);
        }

        let save_btn = Button::with_label(if is_edit { "Save Changes" } else { "Create Space" });
        save_btn.add_css_class("suggested-action");

        {
            let on_res = Rc::clone(&on_result);
            let win_save = window.clone();
            let name_entry = name_entry.clone();
            let sel_col = Rc::clone(&selected_color);
            let sel_ic = Rc::clone(&selected_icon);
            let existing_id = existing_space.map(|s| s.id.clone());
            let existing_pos = existing_space.map(|s| s.position).unwrap_or(0);
            let existing_active_tab = existing_space.and_then(|s| s.active_tab_id);

            save_btn.connect_clicked(move |_| {
                let name = name_entry.text().to_string();
                let clean_name = if name.trim().is_empty() {
                    "Untitled Space".to_string()
                } else {
                    name.trim().to_string()
                };

                let id = existing_id.clone().unwrap_or_default();
                let space = SpaceInfo {
                    id,
                    name: clean_name,
                    accent_color: sel_col.borrow().clone(),
                    icon: Some(sel_ic.borrow().clone()),
                    position: existing_pos,
                    active_tab_id: existing_active_tab,
                };

                win_save.close();
                if let Some(ref cb) = *on_res.borrow() {
                    cb(SpaceDialogResult::Save(space));
                }
            });
        }

        actions_row.append(&save_btn);
        container.append(&actions_row);

        window.set_child(Some(&container));

        Self { window, on_result }
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn connect_result<F: Fn(SpaceDialogResult) + 'static>(&self, f: F) {
        *self.on_result.borrow_mut() = Some(Box::new(f));
    }
}
