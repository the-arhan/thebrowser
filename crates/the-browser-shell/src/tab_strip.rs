//! Tab bar strip widget coordinating horizontal tab representations with Drag & Drop.

use gtk4::gdk;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, DragSource, DropTarget, GestureClick, Label, Orientation, Spinner,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use the_browser_core::{TabId, TabLifecycleState};

type TabActionCallback = Rc<RefCell<Option<Box<dyn Fn(TabId)>>>>;
type ContextMenuCallback = Rc<RefCell<Option<Box<dyn Fn(TabId, &gtk4::Widget)>>>>;
type SimpleCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;
type TabReorderCallback = Rc<RefCell<Option<Box<dyn Fn(TabId, TabId, bool)>>>>;

pub struct TabItemWidgets {
    pub root: GtkBox,
    pub audio_icon: Label,
    pub title_label: Label,
    pub spinner: Spinner,
}

pub struct TabStrip {
    container: GtkBox,
    items_box: GtkBox,
    tab_widgets: Rc<RefCell<HashMap<TabId, TabItemWidgets>>>,
    active_tab: Rc<RefCell<Option<TabId>>>,
    on_select: TabActionCallback,
    on_close: TabActionCallback,
    on_context_menu: ContextMenuCallback,
    on_new_tab: SimpleCallback,
    on_reorder_tab: TabReorderCallback,
}

impl TabStrip {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 0);
        container.add_css_class("tab-strip");

        let items_box = GtkBox::new(Orientation::Horizontal, 0);
        items_box.set_hexpand(true);
        container.append(&items_box);

        // Plus / New Tab button at the end of the tab strip
        let new_tab_btn = Button::from_icon_name("list-add-symbolic");
        new_tab_btn.add_css_class("nav-button");
        new_tab_btn.set_tooltip_text(Some("New Tab (Ctrl+T)"));
        container.append(&new_tab_btn);

        let on_new_tab = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));
        {
            let on_new_tab = Rc::clone(&on_new_tab);
            new_tab_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_new_tab.borrow() {
                    cb();
                }
            });
        }

        Self {
            container,
            items_box,
            tab_widgets: Rc::new(RefCell::new(HashMap::new())),
            active_tab: Rc::new(RefCell::new(None)),
            on_select: Rc::new(RefCell::new(None)),
            on_close: Rc::new(RefCell::new(None)),
            on_context_menu: Rc::new(RefCell::new(None)),
            on_new_tab,
            on_reorder_tab: Rc::new(RefCell::new(None)),
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn connect_select<F: Fn(TabId) + 'static>(&self, f: F) {
        *self.on_select.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_close<F: Fn(TabId) + 'static>(&self, f: F) {
        *self.on_close.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_context_menu<F: Fn(TabId, &gtk4::Widget) + 'static>(&self, f: F) {
        *self.on_context_menu.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_new_tab<F: Fn() + 'static>(&self, f: F) {
        *self.on_new_tab.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_reorder_tab<F: Fn(TabId, TabId, bool) + 'static>(&self, f: F) {
        *self.on_reorder_tab.borrow_mut() = Some(Box::new(f));
    }

    pub fn update_pin(&self, id: TabId, is_pinned: bool) {
        if let Some(widgets) = self.tab_widgets.borrow().get(&id) {
            if is_pinned {
                widgets.root.add_css_class("pinned");
            } else {
                widgets.root.remove_css_class("pinned");
            }
        }
    }

    pub fn add_tab(&self, id: TabId, title: &str) {
        let tab_box = GtkBox::new(Orientation::Horizontal, 6);
        tab_box.add_css_class("tab-item");

        let audio_icon = Label::new(Some("🔊"));
        audio_icon.add_css_class("tab-audio-icon");
        audio_icon.set_visible(false);
        tab_box.append(&audio_icon);

        let spinner = Spinner::new();
        spinner.set_visible(false);
        tab_box.append(&spinner);

        let title_label = Label::new(Some(title));
        title_label.add_css_class("tab-title");
        title_label.set_ellipsize(EllipsizeMode::End);
        title_label.set_max_width_chars(25);
        title_label.set_hexpand(true);
        title_label.set_halign(Align::Start);
        tab_box.append(&title_label);

        let close_btn = Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("tab-close-btn");
        close_btn.set_tooltip_text(Some("Close tab"));
        tab_box.append(&close_btn);

        // Click gesture on tab box
        let gesture = GestureClick::new();
        gesture.set_button(0);
        {
            let on_select = Rc::clone(&self.on_select);
            let on_close = Rc::clone(&self.on_close);
            let on_context_menu = Rc::clone(&self.on_context_menu);
            let tab_box_clone = tab_box.clone();
            gesture.connect_pressed(move |gesture, _, _, _| {
                let button = gesture.current_button();
                if button == gdk::BUTTON_PRIMARY {
                    if let Some(ref cb) = *on_select.borrow() {
                        cb(id);
                    }
                } else if button == gdk::BUTTON_MIDDLE {
                    if let Some(ref cb) = *on_close.borrow() {
                        cb(id);
                    }
                } else if button == gdk::BUTTON_SECONDARY
                    && let Some(ref cb) = *on_context_menu.borrow()
                {
                    cb(id, tab_box_clone.upcast_ref());
                }
            });
        }
        tab_box.add_controller(gesture);

        // Close button click
        {
            let on_close = Rc::clone(&self.on_close);
            close_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_close.borrow() {
                    cb(id);
                }
            });
        }

        // Horizontal DragSource
        {
            let drag_source = DragSource::new();
            drag_source.set_actions(gdk::DragAction::MOVE);
            let payload = id.to_string();
            let provider = gdk::ContentProvider::for_value(&payload.to_value());
            drag_source.set_content(Some(&provider));

            let root_clone = tab_box.clone();
            drag_source.connect_drag_begin(move |_, _| {
                root_clone.add_css_class("dragging");
            });

            let root_clone2 = tab_box.clone();
            drag_source.connect_drag_end(move |_, _, _| {
                root_clone2.remove_css_class("dragging");
            });

            tab_box.add_controller(drag_source);
        }

        // Horizontal DropTarget
        {
            let drop_target = DropTarget::new(glib::types::Type::STRING, gdk::DragAction::MOVE);

            let root_motion = tab_box.clone();
            drop_target.connect_motion(move |_, x, _| {
                let w = root_motion.width() as f64;
                if x < w / 2.0 {
                    root_motion.add_css_class("drop-before");
                    root_motion.remove_css_class("drop-after");
                } else {
                    root_motion.add_css_class("drop-after");
                    root_motion.remove_css_class("drop-before");
                }
                gdk::DragAction::MOVE
            });

            let root_leave = tab_box.clone();
            drop_target.connect_leave(move |_| {
                root_leave.remove_css_class("drop-before");
                root_leave.remove_css_class("drop-after");
            });

            let root_drop = tab_box.clone();
            let on_reorder = Rc::clone(&self.on_reorder_tab);
            drop_target.connect_drop(move |_, val, x, _| {
                root_drop.remove_css_class("drop-before");
                root_drop.remove_css_class("drop-after");

                if let Ok(dragged_str) = val.get::<String>()
                    && let Some(dragged_id) = TabId::parse(&dragged_str)
                    && dragged_id != id
                {
                    let w = root_drop.width() as f64;
                    let is_after = x >= w / 2.0;
                    if let Some(ref cb) = *on_reorder.borrow() {
                        cb(dragged_id, id, is_after);
                        return true;
                    }
                }
                false
            });

            tab_box.add_controller(drop_target);
        }

        self.items_box.append(&tab_box);

        self.tab_widgets.borrow_mut().insert(
            id,
            TabItemWidgets {
                root: tab_box,
                audio_icon,
                title_label,
                spinner,
            },
        );
    }

    pub fn remove_tab(&self, id: TabId) {
        if let Some(widgets) = self.tab_widgets.borrow_mut().remove(&id) {
            self.items_box.remove(&widgets.root);
        }
    }

    pub fn set_active_tab(&self, id: TabId) {
        let mut active = self.active_tab.borrow_mut();
        if let Some(old_id) = *active
            && let Some(widgets) = self.tab_widgets.borrow().get(&old_id)
        {
            widgets.root.remove_css_class("active");
        }
        if let Some(widgets) = self.tab_widgets.borrow().get(&id) {
            widgets.root.add_css_class("active");
        }
        *active = Some(id);
    }

    pub fn update_title(&self, id: TabId, title: &str) {
        if let Some(widgets) = self.tab_widgets.borrow().get(&id) {
            widgets.title_label.set_text(title);
        }
    }

    pub fn update_loading(&self, id: TabId, is_loading: bool) {
        if let Some(widgets) = self.tab_widgets.borrow().get(&id) {
            widgets.spinner.set_visible(is_loading);
            if is_loading {
                widgets.spinner.start();
            } else {
                widgets.spinner.stop();
            }
        }
    }

    pub fn update_audio(&self, id: TabId, is_playing: bool) {
        if let Some(widgets) = self.tab_widgets.borrow().get(&id) {
            widgets.audio_icon.set_visible(is_playing);
        }
    }

    pub fn update_lifecycle_state(&self, id: TabId, state: TabLifecycleState) {
        if let Some(widgets) = self.tab_widgets.borrow().get(&id) {
            match state {
                TabLifecycleState::Active => {
                    widgets.root.add_css_class("active");
                    widgets.root.remove_css_class("sleeping");
                    widgets.root.remove_css_class("load-failed");
                }
                TabLifecycleState::LoadedBackground | TabLifecycleState::EligibleForSnoozing => {
                    widgets.root.remove_css_class("active");
                    widgets.root.remove_css_class("sleeping");
                    widgets.root.remove_css_class("load-failed");
                }
                TabLifecycleState::Sleeping => {
                    widgets.root.remove_css_class("active");
                    widgets.root.add_css_class("sleeping");
                }
                TabLifecycleState::Restoring => {
                    widgets.spinner.set_visible(true);
                    widgets.spinner.start();
                }
                TabLifecycleState::FailedRestoration => {
                    widgets.root.add_css_class("load-failed");
                    widgets.spinner.stop();
                    widgets.spinner.set_visible(false);
                }
            }
        }
    }

    pub fn update_lazy(&self, id: TabId, is_lazy: bool) {
        if is_lazy {
            self.update_lifecycle_state(id, TabLifecycleState::Sleeping);
        } else {
            self.update_lifecycle_state(id, TabLifecycleState::LoadedBackground);
        }
    }

    pub fn update_load_failed(&self, id: TabId, failed: bool) {
        if failed {
            self.update_lifecycle_state(id, TabLifecycleState::FailedRestoration);
        } else if let Some(widgets) = self.tab_widgets.borrow().get(&id) {
            widgets.root.remove_css_class("load-failed");
        }
    }

    pub fn clear_tabs(&self) {
        self.tab_widgets.borrow_mut().clear();
        while let Some(child) = self.items_box.first_child() {
            self.items_box.remove(&child);
        }
    }
}

impl Default for TabStrip {
    fn default() -> Self {
        Self::new()
    }
}
