//! Left-hand vertical tab sidebar with Space switcher and native GTK4 drag-and-drop.

use gtk4::gdk;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, DragSource, DropTarget, GestureClick, Label, Orientation,
    PolicyType, Popover, ScrolledWindow, Separator, Spinner,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use the_browser_core::{SpaceId, SpaceInfo, TabId, TabLifecycleState};

type TabActionCallback = Rc<RefCell<Option<Box<dyn Fn(TabId)>>>>;
type ContextMenuCallback = Rc<RefCell<Option<Box<dyn Fn(TabId, &gtk4::Widget)>>>>;
type SimpleCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;
type FoldCallback = Rc<RefCell<Option<Box<dyn Fn(bool)>>>>;
type SpaceSelectCallback = Rc<RefCell<Option<Box<dyn Fn(SpaceId)>>>>;
type TabReorderCallback = Rc<RefCell<Option<Box<dyn Fn(TabId, TabId, bool)>>>>;
type MoveToSpaceCallback = Rc<RefCell<Option<Box<dyn Fn(TabId, SpaceId)>>>>;
type SetPinnedCallback = Rc<RefCell<Option<Box<dyn Fn(TabId, bool)>>>>;

struct SidebarItemWidgets {
    root: GtkBox,
    monogram: Label,
    audio_icon: Label,
    title_label: Label,
    spinner: Spinner,
    close_btn: Button,
    is_pinned: bool,
}

pub struct SideBar {
    root: GtkBox,
    space_btn: Button,
    space_dot: Label,
    space_label: Label,
    space_pills_box: GtkBox,
    fold_btn: Button,
    new_tab_btn: Button,
    pinned_section: GtkBox,
    pinned_flow: GtkBox,
    tabs_box: GtkBox,
    tab_widgets: Rc<RefCell<HashMap<TabId, SidebarItemWidgets>>>,
    active_tab: Rc<RefCell<Option<TabId>>>,
    is_folded: Rc<RefCell<bool>>,
    spaces_list: Rc<RefCell<Vec<SpaceInfo>>>,
    current_space_id: Rc<RefCell<SpaceId>>,

    // Callbacks
    on_select: TabActionCallback,
    on_close: TabActionCallback,
    on_context_menu: ContextMenuCallback,
    on_new_tab: SimpleCallback,
    on_toggle_fold: FoldCallback,
    on_select_space: SpaceSelectCallback,
    on_new_space: SimpleCallback,
    on_edit_space: SimpleCallback,
    on_reorder_tab: TabReorderCallback,
    on_move_tab_to_space: MoveToSpaceCallback,
    on_set_pinned: SetPinnedCallback,
}

impl SideBar {
    pub fn new() -> Self {
        let root = GtkBox::new(Orientation::Vertical, 0);
        root.add_css_class("sidebar-pane");
        root.set_size_request(210, -1);

        // 1. Header with Space Switcher Button, New Tab, and Fold button
        let header_box = GtkBox::new(Orientation::Horizontal, 4);
        header_box.add_css_class("sidebar-header");

        let space_btn = Button::new();
        space_btn.add_css_class("space-switcher-btn");
        space_btn.set_hexpand(true);

        let space_pill = GtkBox::new(Orientation::Horizontal, 6);
        let space_dot = Label::new(Some("●"));
        space_dot.add_css_class("space-accent-dot");
        space_pill.append(&space_dot);

        let space_label = Label::new(Some("Personal"));
        space_label.add_css_class("space-title");
        space_label.set_halign(Align::Start);
        space_label.set_ellipsize(EllipsizeMode::End);
        space_label.set_hexpand(true);
        space_pill.append(&space_label);

        let space_chevron = Label::new(Some("▾"));
        space_chevron.add_css_class("space-chevron");
        space_pill.append(&space_chevron);

        space_btn.set_child(Some(&space_pill));
        header_box.append(&space_btn);

        let new_tab_btn = Button::from_icon_name("list-add-symbolic");
        new_tab_btn.add_css_class("nav-button");
        new_tab_btn.set_tooltip_text(Some("New Tab (Ctrl+T)"));
        header_box.append(&new_tab_btn);

        let fold_btn = Button::from_icon_name("view-sidebar-start-symbolic");
        fold_btn.add_css_class("nav-button");
        fold_btn.set_tooltip_text(Some("Toggle Sidebar (Ctrl+S)"));
        header_box.append(&fold_btn);

        root.append(&header_box);

        // 2. Space Quick-Switcher Pills Row
        let space_pills_box = GtkBox::new(Orientation::Horizontal, 4);
        space_pills_box.add_css_class("space-pills-row");
        root.append(&space_pills_box);

        // 3. Pinned Tabs Section
        let pinned_section = GtkBox::new(Orientation::Vertical, 4);
        pinned_section.add_css_class("sidebar-pinned-section");
        pinned_section.set_visible(false);

        let pinned_label = Label::new(Some("PINNED"));
        pinned_label.add_css_class("sidebar-section-title");
        pinned_label.set_halign(Align::Start);
        pinned_section.append(&pinned_label);

        let pinned_flow = GtkBox::new(Orientation::Vertical, 2);
        pinned_flow.add_css_class("sidebar-pinned-grid");
        pinned_section.append(&pinned_flow);

        root.append(&pinned_section);

        // 4. Scrollable normal tab items list
        let scrolled = ScrolledWindow::new();
        scrolled.set_policy(PolicyType::Never, PolicyType::Automatic);
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);
        scrolled.add_css_class("sidebar-scroll");

        let tabs_box = GtkBox::new(Orientation::Vertical, 2);
        tabs_box.add_css_class("sidebar-tab-list");
        scrolled.set_child(Some(&tabs_box));

        root.append(&scrolled);

        let is_folded = Rc::new(RefCell::new(false));
        let spaces_list = Rc::new(RefCell::new(Vec::<SpaceInfo>::new()));
        let current_space_id = Rc::new(RefCell::new(SpaceId::default()));

        let on_select = Rc::new(RefCell::new(None));
        let on_close = Rc::new(RefCell::new(None));
        let on_context_menu = Rc::new(RefCell::new(None));
        let on_new_tab = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));
        let on_toggle_fold = Rc::new(RefCell::new(None));
        let on_select_space = Rc::new(RefCell::new(None::<Box<dyn Fn(SpaceId)>>));
        let on_new_space = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));
        let on_edit_space = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));
        let on_reorder_tab = Rc::new(RefCell::new(None::<Box<dyn Fn(TabId, TabId, bool)>>));
        let on_move_tab_to_space = Rc::new(RefCell::new(None::<Box<dyn Fn(TabId, SpaceId)>>));
        let on_set_pinned = Rc::new(RefCell::new(None::<Box<dyn Fn(TabId, bool)>>));

        // New Tab button handler
        {
            let on_new_tab = Rc::clone(&on_new_tab);
            new_tab_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_new_tab.borrow() {
                    cb();
                }
            });
        }

        // Drop target on pinned section (drag tab into pinned section -> pins tab)
        {
            let on_set_pinned = Rc::clone(&on_set_pinned);
            let drop_pinned = DropTarget::new(glib::types::Type::STRING, gdk::DragAction::MOVE);
            drop_pinned.connect_drop(move |_, val, _, _| {
                if let Ok(dragged_str) = val.get::<String>()
                    && let Some(dragged_id) = TabId::parse(&dragged_str)
                    && let Some(ref cb) = *on_set_pinned.borrow()
                {
                    cb(dragged_id, true);
                    return true;
                }
                false
            });
            pinned_section.add_controller(drop_pinned);
        }

        // Space switcher popover menu
        {
            let spaces_ref = Rc::clone(&spaces_list);
            let curr_sp_ref = Rc::clone(&current_space_id);
            let on_sel_sp = Rc::clone(&on_select_space);
            let on_new_sp = Rc::clone(&on_new_space);
            let on_edit_sp = Rc::clone(&on_edit_space);

            space_btn.connect_clicked(move |btn| {
                let popover = Popover::new();
                popover.set_parent(btn);
                popover.add_css_class("space-menu-popover");

                let menu_box = GtkBox::new(Orientation::Vertical, 4);
                menu_box.set_margin_top(6);
                menu_box.set_margin_bottom(6);
                menu_box.set_margin_start(6);
                menu_box.set_margin_end(6);

                let header = Label::new(Some("Spaces"));
                header.add_css_class("menu-header-label");
                header.set_halign(Align::Start);
                menu_box.append(&header);

                let current_id = curr_sp_ref.borrow().clone();

                for space in spaces_ref.borrow().iter() {
                    let row_btn = Button::new();
                    row_btn.add_css_class("menu-item-button");

                    let row_box = GtkBox::new(Orientation::Horizontal, 8);
                    let dot = Label::builder()
                        .use_markup(true)
                        .label(format!("<span foreground=\"{}\">●</span>", space.accent_color))
                        .build();
                    row_box.append(&dot);

                    let name_lbl = Label::new(Some(&space.name));
                    name_lbl.set_halign(Align::Start);
                    name_lbl.set_hexpand(true);
                    row_box.append(&name_lbl);

                    if space.id == current_id {
                        let check = Label::new(Some("✓"));
                        check.add_css_class("space-active-check");
                        row_box.append(&check);
                    }

                    row_btn.set_child(Some(&row_box));

                    let sp_id = space.id.clone();
                    let on_sel = Rc::clone(&on_sel_sp);
                    let pop_clone = popover.clone();
                    row_btn.connect_clicked(move |_| {
                        pop_clone.popdown();
                        if let Some(ref cb) = *on_sel.borrow() {
                            cb(sp_id.clone());
                        }
                    });

                    menu_box.append(&row_btn);
                }

                menu_box.append(&Separator::new(Orientation::Horizontal));

                // Edit Active Space
                {
                    let edit_btn = Button::with_label("Edit Active Space...");
                    edit_btn.add_css_class("menu-item-button");
                    let on_edit = Rc::clone(&on_edit_sp);
                    let pop_clone = popover.clone();
                    edit_btn.connect_clicked(move |_| {
                        pop_clone.popdown();
                        if let Some(ref cb) = *on_edit.borrow() {
                            cb();
                        }
                    });
                    menu_box.append(&edit_btn);
                }

                // New Space Action
                {
                    let add_btn = Button::with_label("+ New Space");
                    add_btn.add_css_class("menu-item-button");
                    let on_new = Rc::clone(&on_new_sp);
                    let pop_clone = popover.clone();
                    add_btn.connect_clicked(move |_| {
                        pop_clone.popdown();
                        if let Some(ref cb) = *on_new.borrow() {
                            cb();
                        }
                    });
                    menu_box.append(&add_btn);
                }

                popover.set_child(Some(&menu_box));
                popover.popup();
            });
        }

        Self {
            root,
            space_btn,
            space_dot,
            space_label,
            space_pills_box,
            fold_btn,
            new_tab_btn,
            pinned_section,
            pinned_flow,
            tabs_box,
            tab_widgets: Rc::new(RefCell::new(HashMap::new())),
            active_tab: Rc::new(RefCell::new(None)),
            is_folded,
            spaces_list,
            current_space_id,
            on_select,
            on_close,
            on_context_menu,
            on_new_tab,
            on_toggle_fold,
            on_select_space,
            on_new_space,
            on_edit_space,
            on_reorder_tab,
            on_move_tab_to_space,
            on_set_pinned,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.root
    }

    pub fn is_folded(&self) -> bool {
        *self.is_folded.borrow()
    }

    pub fn set_folded(&self, folded: bool) {
        *self.is_folded.borrow_mut() = folded;

        if folded {
            self.root.add_css_class("folded");
            self.root.set_size_request(46, -1);
            self.space_btn.set_visible(false);
            self.space_pills_box.set_visible(false);
            self.new_tab_btn.set_visible(false);
        } else {
            self.root.remove_css_class("folded");
            self.root.set_size_request(210, -1);
            self.space_btn.set_visible(true);
            self.space_pills_box.set_visible(true);
            self.new_tab_btn.set_visible(true);
        }

        for widgets in self.tab_widgets.borrow().values() {
            if folded {
                widgets.title_label.set_visible(false);
                widgets.close_btn.set_visible(false);
                widgets.root.set_tooltip_text(Some(&widgets.title_label.text()));
            } else {
                widgets.title_label.set_visible(true);
                widgets.close_btn.set_visible(!widgets.is_pinned);
                widgets.root.set_tooltip_text(None);
            }
        }

        if let Some(ref cb) = *self.on_toggle_fold.borrow() {
            cb(folded);
        }
    }

    pub fn toggle_fold(&self) -> bool {
        let new_state = !self.is_folded();
        self.set_folded(new_state);
        new_state
    }

    pub fn set_space_info(&self, name: &str, accent_color: &str) {
        self.space_label.set_text(name);
        self.space_dot.set_markup(&format!("<span foreground=\"{accent_color}\">●</span>"));
    }

    pub fn render_space_pills(&self, spaces: &[SpaceInfo], active_id: &SpaceId) {
        *self.spaces_list.borrow_mut() = spaces.to_vec();
        *self.current_space_id.borrow_mut() = active_id.clone();

        while let Some(child) = self.space_pills_box.first_child() {
            self.space_pills_box.remove(&child);
        }

        for space in spaces {
            let pill_btn = Button::new();
            pill_btn.add_css_class("space-quick-pill");

            let pill_content = GtkBox::new(Orientation::Horizontal, 4);
            let dot = Label::builder()
                .use_markup(true)
                .label(format!("<span foreground=\"{}\">●</span>", space.accent_color))
                .build();
            pill_content.append(&dot);

            let lbl = Label::new(Some(&space.name));
            pill_content.append(&lbl);
            pill_btn.set_child(Some(&pill_content));

            if &space.id == active_id {
                pill_btn.add_css_class("active");
            }

            // Click to switch space
            let sp_id = space.id.clone();
            let on_sel = Rc::clone(&self.on_select_space);
            pill_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_sel.borrow() {
                    cb(sp_id.clone());
                }
            });

            // Drop target on Space pill: dragging tab to a space pill transfers the tab!
            let target_sp_id = space.id.clone();
            let on_move = Rc::clone(&self.on_move_tab_to_space);
            let pill_clone = pill_btn.clone();
            let drop_target = DropTarget::new(glib::types::Type::STRING, gdk::DragAction::MOVE);

            let pill_motion = pill_clone.clone();
            drop_target.connect_motion(move |_, _, _| {
                pill_motion.add_css_class("drop-target-hover");
                gdk::DragAction::MOVE
            });

            let pill_leave = pill_clone.clone();
            drop_target.connect_leave(move |_| {
                pill_leave.remove_css_class("drop-target-hover");
            });

            let pill_drop = pill_clone.clone();
            drop_target.connect_drop(move |_, val, _, _| {
                pill_drop.remove_css_class("drop-target-hover");
                if let Ok(dragged_str) = val.get::<String>()
                    && let Some(dragged_id) = TabId::parse(&dragged_str)
                    && let Some(ref cb) = *on_move.borrow()
                {
                    cb(dragged_id, target_sp_id.clone());
                    return true;
                }
                false
            });

            pill_btn.add_controller(drop_target);
            self.space_pills_box.append(&pill_btn);
        }
    }

    pub fn add_tab(&self, id: TabId, title: &str) {
        let tab_box = GtkBox::new(Orientation::Horizontal, 6);
        tab_box.add_css_class("sidebar-tab-item");

        let initial_letter = monogram_for(title);
        let monogram = Label::new(Some(&initial_letter));
        monogram.add_css_class("tab-monogram");
        tab_box.append(&monogram);

        let audio_icon = Label::new(Some("🔊"));
        audio_icon.add_css_class("tab-audio-icon");
        audio_icon.set_visible(false);
        tab_box.append(&audio_icon);

        let spinner = Spinner::new();
        spinner.set_visible(false);
        tab_box.append(&spinner);

        let title_label = Label::new(Some(title));
        title_label.add_css_class("sidebar-tab-title");
        title_label.set_ellipsize(EllipsizeMode::End);
        title_label.set_max_width_chars(18);
        title_label.set_hexpand(true);
        title_label.set_halign(Align::Start);
        tab_box.append(&title_label);

        let close_btn = Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("sidebar-tab-close");
        close_btn.set_tooltip_text(Some("Close tab (Ctrl+W)"));
        tab_box.append(&close_btn);

        let folded = self.is_folded();
        if folded {
            title_label.set_visible(false);
            close_btn.set_visible(false);
            tab_box.set_tooltip_text(Some(title));
        }

        // Mouse gestures: Click = select, Middle = close, Right = context menu
        {
            let on_select = Rc::clone(&self.on_select);
            let on_close = Rc::clone(&self.on_close);
            let on_context_menu = Rc::clone(&self.on_context_menu);
            let tab_box_clone = tab_box.clone();
            let gesture = GestureClick::new();
            gesture.set_button(0);
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
            tab_box.add_controller(gesture);
        }

        // Close button click
        {
            let on_close = Rc::clone(&self.on_close);
            close_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *on_close.borrow() {
                    cb(id);
                }
            });
        }

        // Native GTK4 DragSource for tab reordering
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

        // Native GTK4 DropTarget with hairline insertion cues (.drop-before / .drop-after)
        {
            let drop_target = DropTarget::new(glib::types::Type::STRING, gdk::DragAction::MOVE);

            let root_motion = tab_box.clone();
            drop_target.connect_motion(move |_, _x, y| {
                let h = root_motion.height() as f64;
                if y < h / 2.0 {
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
            drop_target.connect_drop(move |_, val, _x, y| {
                root_drop.remove_css_class("drop-before");
                root_drop.remove_css_class("drop-after");

                if let Ok(dragged_str) = val.get::<String>()
                    && let Some(dragged_id) = TabId::parse(&dragged_str)
                    && dragged_id != id
                {
                    let h = root_drop.height() as f64;
                    let is_after = y >= h / 2.0;
                    if let Some(ref cb) = *on_reorder.borrow() {
                        cb(dragged_id, id, is_after);
                        return true;
                    }
                }
                false
            });

            tab_box.add_controller(drop_target);
        }

        self.tabs_box.append(&tab_box);

        self.tab_widgets.borrow_mut().insert(
            id,
            SidebarItemWidgets {
                root: tab_box,
                monogram,
                audio_icon,
                title_label,
                spinner,
                close_btn,
                is_pinned: false,
            },
        );
    }

    pub fn remove_tab(&self, id: TabId) {
        if let Some(widgets) = self.tab_widgets.borrow_mut().remove(&id) {
            if widgets.is_pinned {
                self.pinned_flow.remove(&widgets.root);
                if self.pinned_flow.first_child().is_none() {
                    self.pinned_section.set_visible(false);
                }
            } else {
                self.tabs_box.remove(&widgets.root);
            }
        }
    }

    pub fn clear_tabs(&self) {
        self.tab_widgets.borrow_mut().clear();
        while let Some(child) = self.tabs_box.first_child() {
            self.tabs_box.remove(&child);
        }
        while let Some(child) = self.pinned_flow.first_child() {
            self.pinned_flow.remove(&child);
        }
        self.pinned_section.set_visible(false);
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
            widgets.monogram.set_text(&monogram_for(title));
            if self.is_folded() {
                widgets.root.set_tooltip_text(Some(title));
            }
        }
    }

    pub fn update_loading(&self, id: TabId, is_loading: bool) {
        if let Some(widgets) = self.tab_widgets.borrow().get(&id) {
            widgets.spinner.set_visible(is_loading);
            if is_loading {
                widgets.spinner.start();
                widgets.monogram.set_visible(false);
            } else {
                widgets.spinner.stop();
                widgets.monogram.set_visible(true);
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
                    widgets.monogram.set_text("💤");
                }
                TabLifecycleState::Restoring => {
                    widgets.spinner.set_visible(true);
                    widgets.spinner.start();
                    widgets.monogram.set_visible(false);
                }
                TabLifecycleState::FailedRestoration => {
                    widgets.root.add_css_class("load-failed");
                    widgets.monogram.set_text("⚠");
                    widgets.monogram.set_visible(true);
                    widgets.spinner.stop();
                    widgets.spinner.set_visible(false);
                }
            }
        }
    }

    pub fn update_pin(&self, id: TabId, is_pinned: bool) {
        if let Some(widgets) = self.tab_widgets.borrow_mut().get_mut(&id) {
            if widgets.is_pinned == is_pinned {
                return;
            }

            widgets.is_pinned = is_pinned;
            if is_pinned {
                self.tabs_box.remove(&widgets.root);
                self.pinned_flow.append(&widgets.root);
                widgets.root.add_css_class("pinned");
                widgets.close_btn.set_visible(false);
                self.pinned_section.set_visible(true);
            } else {
                self.pinned_flow.remove(&widgets.root);
                self.tabs_box.append(&widgets.root);
                widgets.root.remove_css_class("pinned");
                if !self.is_folded() {
                    widgets.close_btn.set_visible(true);
                }
                if self.pinned_flow.first_child().is_none() {
                    self.pinned_section.set_visible(false);
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
            widgets.monogram.set_text(&monogram_for(&widgets.title_label.text()));
        }
    }

    // --- Connect Callback Helpers ---

    pub fn connect_context_menu<F: Fn(TabId, &gtk4::Widget) + 'static>(&self, f: F) {
        *self.on_context_menu.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_select<F: Fn(TabId) + 'static>(&self, f: F) {
        *self.on_select.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_close<F: Fn(TabId) + 'static>(&self, f: F) {
        *self.on_close.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_new_tab<F: Fn() + 'static>(&self, f: F) {
        *self.on_new_tab.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_select_space<F: Fn(SpaceId) + 'static>(&self, f: F) {
        *self.on_select_space.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_new_space<F: Fn() + 'static>(&self, f: F) {
        *self.on_new_space.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_edit_space<F: Fn() + 'static>(&self, f: F) {
        *self.on_edit_space.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_reorder_tab<F: Fn(TabId, TabId, bool) + 'static>(&self, f: F) {
        *self.on_reorder_tab.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_move_tab_to_space<F: Fn(TabId, SpaceId) + 'static>(&self, f: F) {
        *self.on_move_tab_to_space.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_set_pinned<F: Fn(TabId, bool) + 'static>(&self, f: F) {
        *self.on_set_pinned.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_toggle_fold<F: Fn(bool) + 'static>(&self, f: F) {
        let btn = self.fold_btn.clone();
        let this_cb = Rc::clone(&self.on_toggle_fold);
        *this_cb.borrow_mut() = Some(Box::new(f));

        let is_folded = Rc::clone(&self.is_folded);
        let root = self.root.clone();
        let space_btn = self.space_btn.clone();
        let space_pills_box = self.space_pills_box.clone();
        let new_tab_btn = self.new_tab_btn.clone();
        let tab_widgets = Rc::clone(&self.tab_widgets);

        btn.connect_clicked(move |_| {
            let next_state = !*is_folded.borrow();
            *is_folded.borrow_mut() = next_state;

            if next_state {
                root.add_css_class("folded");
                root.set_size_request(46, -1);
                space_btn.set_visible(false);
                space_pills_box.set_visible(false);
                new_tab_btn.set_visible(false);
            } else {
                root.remove_css_class("folded");
                root.set_size_request(210, -1);
                space_btn.set_visible(true);
                space_pills_box.set_visible(true);
                new_tab_btn.set_visible(true);
            }

            for widgets in tab_widgets.borrow().values() {
                if next_state {
                    widgets.title_label.set_visible(false);
                    widgets.close_btn.set_visible(false);
                    widgets.root.set_tooltip_text(Some(&widgets.title_label.text()));
                } else {
                    widgets.title_label.set_visible(true);
                    widgets.close_btn.set_visible(!widgets.is_pinned);
                    widgets.root.set_tooltip_text(None);
                }
            }

            if let Some(ref cb) = *this_cb.borrow() {
                cb(next_state);
            }
        });
    }
}

impl Default for SideBar {
    fn default() -> Self {
        Self::new()
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
