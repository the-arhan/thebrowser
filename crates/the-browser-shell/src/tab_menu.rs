//! Tab contextual menu with tab lifecycle actions, snoozing, and space transfers.

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Image, Label, Orientation, Popover, Separator};
use std::cell::RefCell;
use std::rc::Rc;
use the_browser_core::{SpaceId, SpaceInfo, TabId};

pub enum TabMenuAction {
    NewTab,
    Duplicate(TabId),
    TogglePin(TabId),
    Reload(TabId),
    Close(TabId),
    CloseOthers(TabId),
    CloseToRight(TabId),
    ReopenClosed,
    Snooze(TabId),
    Wake(TabId),
    MoveToSpace(TabId, SpaceId),
}

type MenuActionCallback = Rc<RefCell<Option<Box<dyn Fn(TabMenuAction)>>>>;

pub struct TabContextMenu {
    popover: Popover,
    target_tab_id: Rc<RefCell<Option<TabId>>>,
    is_pinned: Rc<RefCell<bool>>,
    pin_label: Label,
    snooze_label: Label,
    spaces_box: GtkBox,
    on_action: MenuActionCallback,
}

impl TabContextMenu {
    pub fn new() -> Self {
        let popover = Popover::new();
        popover.add_css_class("tab-context-popover");

        let content = GtkBox::new(Orientation::Vertical, 2);
        content.set_margin_top(4);
        content.set_margin_bottom(4);
        content.set_margin_start(4);
        content.set_margin_end(4);

        let target_tab_id = Rc::new(RefCell::new(None));
        let is_pinned = Rc::new(RefCell::new(false));
        let on_action = Rc::new(RefCell::new(None::<Box<dyn Fn(TabMenuAction)>>));

        let make_item = |icon: &str,
                         label_text: &str,
                         popover: &Popover,
                         on_action: &MenuActionCallback,
                         action_fn: Box<dyn Fn(Option<TabId>) -> Option<TabMenuAction>>|
         -> (Button, Label) {
            let btn = Button::new();
            btn.add_css_class("menu-item-button");

            let row = GtkBox::new(Orientation::Horizontal, 8);
            row.set_hexpand(true);

            let img = Image::from_icon_name(icon);
            row.append(&img);

            let lbl = Label::new(Some(label_text));
            lbl.set_halign(Align::Start);
            lbl.set_hexpand(true);
            row.append(&lbl);

            btn.set_child(Some(&row));

            let pop_clone = popover.clone();
            let on_act = Rc::clone(on_action);
            btn.connect_clicked(move |_| {
                pop_clone.popdown();
                if let Some(ref cb) = *on_act.borrow()
                    && let Some(act) = action_fn(None)
                {
                    cb(act);
                }
            });

            (btn, lbl)
        };

        // 1. New Tab
        {
            let on_act = Rc::clone(&on_action);
            let pop = popover.clone();
            let (btn, _) = make_item(
                "tab-new-symbolic",
                "New Tab",
                &pop,
                &on_act,
                Box::new(move |_| Some(TabMenuAction::NewTab)),
            );
            content.append(&btn);
        }

        // 2. Duplicate Tab
        {
            let on_act = Rc::clone(&on_action);
            let target = Rc::clone(&target_tab_id);
            let pop = popover.clone();
            let (btn, _) = make_item(
                "edit-copy-symbolic",
                "Duplicate Tab",
                &pop,
                &on_act,
                Box::new(move |_| target.borrow().map(TabMenuAction::Duplicate)),
            );
            content.append(&btn);
        }

        // 3. Pin / Unpin Tab
        let pin_label: Label;
        {
            let on_act = Rc::clone(&on_action);
            let target = Rc::clone(&target_tab_id);
            let pop = popover.clone();
            let (btn, lbl) = make_item(
                "view-pin-symbolic",
                "Pin Tab",
                &pop,
                &on_act,
                Box::new(move |_| target.borrow().map(TabMenuAction::TogglePin)),
            );
            pin_label = lbl;
            content.append(&btn);
        }

        // 4. Reload Tab
        {
            let on_act = Rc::clone(&on_action);
            let target = Rc::clone(&target_tab_id);
            let pop = popover.clone();
            let (btn, _) = make_item(
                "view-refresh-symbolic",
                "Reload Tab",
                &pop,
                &on_act,
                Box::new(move |_| target.borrow().map(TabMenuAction::Reload)),
            );
            content.append(&btn);
        }

        // 5. Intelligent Snooze / Wake Action
        let (snooze_btn, snooze_label) = {
            let on_act = Rc::clone(&on_action);
            let target = Rc::clone(&target_tab_id);
            let pop = popover.clone();
            make_item(
                "media-playback-pause-symbolic",
                "Snooze Tab",
                &pop,
                &on_act,
                Box::new(move |_| target.borrow().map(TabMenuAction::Snooze)),
            )
        };
        content.append(&snooze_btn);

        content.append(&Separator::new(Orientation::Horizontal));

        // 6. Cross-Space Movement Section
        let spaces_box = GtkBox::new(Orientation::Vertical, 2);
        content.append(&spaces_box);

        content.append(&Separator::new(Orientation::Horizontal));

        // 7. Close Tab
        {
            let on_act = Rc::clone(&on_action);
            let target = Rc::clone(&target_tab_id);
            let pop = popover.clone();
            let (btn, _) = make_item(
                "window-close-symbolic",
                "Close Tab",
                &pop,
                &on_act,
                Box::new(move |_| target.borrow().map(TabMenuAction::Close)),
            );
            content.append(&btn);
        }

        // 8. Close Other Tabs
        {
            let on_act = Rc::clone(&on_action);
            let target = Rc::clone(&target_tab_id);
            let pop = popover.clone();
            let (btn, _) = make_item(
                "edit-clear-all-symbolic",
                "Close Other Tabs",
                &pop,
                &on_act,
                Box::new(move |_| target.borrow().map(TabMenuAction::CloseOthers)),
            );
            content.append(&btn);
        }

        // 9. Close Tabs to the Right
        {
            let on_act = Rc::clone(&on_action);
            let target = Rc::clone(&target_tab_id);
            let pop = popover.clone();
            let (btn, _) = make_item(
                "go-next-symbolic",
                "Close Tabs to the Right",
                &pop,
                &on_act,
                Box::new(move |_| target.borrow().map(TabMenuAction::CloseToRight)),
            );
            content.append(&btn);
        }

        content.append(&Separator::new(Orientation::Horizontal));

        // 10. Reopen Closed Tab
        {
            let on_act = Rc::clone(&on_action);
            let pop = popover.clone();
            let (btn, _) = make_item(
                "edit-undo-symbolic",
                "Reopen Closed Tab",
                &pop,
                &on_act,
                Box::new(move |_| Some(TabMenuAction::ReopenClosed)),
            );
            content.append(&btn);
        }

        popover.set_child(Some(&content));

        Self {
            popover,
            target_tab_id,
            is_pinned,
            pin_label,
            snooze_label,
            spaces_box,
            on_action,
        }
    }

    pub fn popup_for(
        &self,
        parent: &impl IsA<gtk4::Widget>,
        tab_id: TabId,
        is_pinned: bool,
        is_sleeping: bool,
        current_space_id: &SpaceId,
        available_spaces: &[SpaceInfo],
    ) {
        *self.target_tab_id.borrow_mut() = Some(tab_id);
        *self.is_pinned.borrow_mut() = is_pinned;

        if is_pinned {
            self.pin_label.set_text("Unpin Tab");
        } else {
            self.pin_label.set_text("Pin Tab");
        }

        if is_sleeping {
            self.snooze_label.set_text("Wake Tab");
        } else {
            self.snooze_label.set_text("Snooze Tab");
        }

        // Populate cross-space options
        while let Some(child) = self.spaces_box.first_child() {
            self.spaces_box.remove(&child);
        }

        let other_spaces: Vec<&SpaceInfo> = available_spaces
            .iter()
            .filter(|s| &s.id != current_space_id)
            .collect();

        if !other_spaces.is_empty() {
            let section_lbl = Label::new(Some("MOVE TO SPACE"));
            section_lbl.add_css_class("sidebar-section-title");
            section_lbl.set_halign(Align::Start);
            self.spaces_box.append(&section_lbl);

            for sp in other_spaces {
                let btn = Button::new();
                btn.add_css_class("menu-item-button");

                let row = GtkBox::new(Orientation::Horizontal, 8);
                let dot = Label::builder()
                    .use_markup(true)
                    .label(format!("<span foreground=\"{}\">●</span>", sp.accent_color))
                    .build();
                row.append(&dot);

                let lbl = Label::new(Some(&sp.name));
                lbl.set_halign(Align::Start);
                lbl.set_hexpand(true);
                row.append(&lbl);

                btn.set_child(Some(&row));

                let pop_clone = self.popover.clone();
                let on_act = Rc::clone(&self.on_action);
                let target_sp_id = sp.id.clone();

                btn.connect_clicked(move |_| {
                    pop_clone.popdown();
                    if let Some(ref cb) = *on_act.borrow() {
                        cb(TabMenuAction::MoveToSpace(tab_id, target_sp_id.clone()));
                    }
                });

                self.spaces_box.append(&btn);
            }
        }

        self.popover.set_parent(parent);
        self.popover.popup();
    }

    pub fn connect_action<F: Fn(TabMenuAction) + 'static>(&self, f: F) {
        *self.on_action.borrow_mut() = Some(Box::new(f));
    }
}

impl Default for TabContextMenu {
    fn default() -> Self {
        Self::new()
    }
}
