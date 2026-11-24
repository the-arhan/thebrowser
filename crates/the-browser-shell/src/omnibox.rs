//! Contextual floating Omnibox modal overlay.

use gtk4::gdk::Key;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Entry, EventControllerKey, GestureClick, Image, Label, Orientation};
use libadwaita::Clamp as AdwClamp;
use std::cell::RefCell;
use std::rc::Rc;

type StringCallback = Rc<RefCell<Option<Box<dyn Fn(&str)>>>>;
type SimpleCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

pub struct Omnibox {
    root: GtkBox,
    entry: Entry,
    engine_badge: Label,
    on_activate: StringCallback,
    on_dismiss: SimpleCallback,
}

impl Omnibox {
    pub fn new() -> Self {
        let root = GtkBox::new(Orientation::Vertical, 0);
        root.add_css_class("omnibox-overlay");
        root.set_valign(gtk4::Align::Fill);
        root.set_halign(gtk4::Align::Fill);
        root.set_visible(false);

        let clamp = AdwClamp::new();
        clamp.set_maximum_size(580);
        clamp.set_tightening_threshold(420);
        clamp.set_valign(gtk4::Align::Start);
        clamp.set_margin_top(72);
        clamp.set_margin_start(16);
        clamp.set_margin_end(16);

        let card = GtkBox::new(Orientation::Vertical, 0);
        card.add_css_class("omnibox-card");

        let input_row = GtkBox::new(Orientation::Horizontal, 10);
        input_row.add_css_class("omnibox-input-row");

        let icon = Image::from_icon_name("edit-find-symbolic");
        icon.add_css_class("omnibox-icon");
        input_row.append(&icon);

        let entry = Entry::new();
        entry.add_css_class("omnibox-entry");
        entry.set_placeholder_text(Some("Search or enter address..."));
        entry.set_hexpand(true);
        input_row.append(&entry);

        let engine_badge = Label::new(Some("DuckDuckGo"));
        engine_badge.add_css_class("omnibox-badge");
        input_row.append(&engine_badge);

        card.append(&input_row);
        clamp.set_child(Some(&card));
        root.append(&clamp);

        let on_activate = Rc::new(RefCell::new(None::<Box<dyn Fn(&str)>>));
        let on_dismiss = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));

        // Consume clicks inside the card so they don't trigger the scrim dismissal
        let card_gesture = GestureClick::new();
        card_gesture.connect_pressed(|_, _, _, _| {
            // Intentionally consumed
        });
        card.add_controller(card_gesture);

        // Clicking on the outer scrim dismisses the Omnibox
        {
            let on_dismiss = Rc::clone(&on_dismiss);
            let scrim_gesture = GestureClick::new();
            scrim_gesture.connect_pressed(move |_, _, _, _| {
                if let Some(ref cb) = *on_dismiss.borrow() {
                    cb();
                }
            });
            root.add_controller(scrim_gesture);
        }

        // Keyboard handler for Escape and Enter
        {
            let on_activate = Rc::clone(&on_activate);
            let on_dismiss = Rc::clone(&on_dismiss);
            let entry_clone = entry.clone();

            let key_controller = EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, keyval, _, _| {
                match keyval {
                    Key::Escape => {
                        if let Some(ref cb) = *on_dismiss.borrow() {
                            cb();
                        }
                        glib::Propagation::Stop
                    }
                    Key::Return | Key::KP_Enter => {
                        let text = entry_clone.text().to_string();
                        if let Some(ref cb) = *on_activate.borrow() {
                            cb(&text);
                        }
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            });
            root.add_controller(key_controller);
        }

        // Activate signal from Entry
        {
            let on_activate = Rc::clone(&on_activate);
            entry.connect_activate(move |entry| {
                let text = entry.text().to_string();
                if let Some(ref cb) = *on_activate.borrow() {
                    cb(&text);
                }
            });
        }

        Self {
            root,
            entry,
            engine_badge,
            on_activate,
            on_dismiss,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.root
    }

    pub fn is_visible(&self) -> bool {
        self.root.is_visible()
    }

    pub fn show(&self, initial_text: Option<&str>) {
        if let Some(text) = initial_text {
            self.entry.set_text(text);
        }
        self.root.set_visible(true);
        self.entry.grab_focus();
        self.entry.select_region(0, -1);
    }

    pub fn hide(&self) {
        self.root.set_visible(false);
    }

    pub fn set_engine_name(&self, name: &str) {
        self.engine_badge.set_text(name);
    }

    pub fn connect_activate<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_activate.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_dismiss<F: Fn() + 'static>(&self, f: F) {
        *self.on_dismiss.borrow_mut() = Some(Box::new(f));
    }
}

impl Default for Omnibox {
    fn default() -> Self {
        Self::new()
    }
}

