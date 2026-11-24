use gtk4::prelude::*;
use libadwaita::Application as AdwApplication;
use the_browser_shell::{
    BrowserWindow, NavBar, Omnibox, SettingsDialog, SideBar, TabContextMenu, TabMenuAction,
};
use the_browser_storage::{BrowserSettings, StorageManager};

#[test]
fn test_ui_overhaul_suite() {
    if gtk4::init().is_err() {
        return;
    }

    // 1. Omnibox lifecycle
    let omnibox = Omnibox::new();
    assert!(!omnibox.is_visible());
    omnibox.show(Some("https://archlinux.org"));
    assert!(omnibox.is_visible());
    omnibox.hide();
    assert!(!omnibox.is_visible());

    // 2. Sidebar folding lifecycle
    let sidebar = SideBar::new();
    assert!(!sidebar.is_folded());
    assert!(sidebar.toggle_fold());
    assert!(sidebar.is_folded());
    assert!(!sidebar.toggle_fold());
    assert!(!sidebar.is_folded());

    // 3. Navigation Bar component verification
    let navbar = NavBar::new();
    navbar.set_can_go_back(true);
    navbar.set_can_go_forward(false);
    navbar.set_loading(true);
    navbar.set_loading(false);
    navbar.set_url("https://archlinux.org");
    navbar.set_bookmarked(true);
    navbar.set_bookmarked(false);
    navbar.focus_address();

    // 4. Tab Context Menu component verification
    let tab_menu = TabContextMenu::new();
    tab_menu.connect_action(|action| {
        if let TabMenuAction::NewTab = action {}
    });

    // 5. Settings Dialog component verification
    let dummy_win = gtk4::Window::new();
    let settings = BrowserSettings::default();
    let storage = StorageManager::open_in_memory().ok().map(std::rc::Rc::new);
    let settings_dialog = SettingsDialog::new(&dummy_win, settings, storage);
    settings_dialog.connect_settings_changed(|_| {});
    settings_dialog.connect_clear_history(|| {});

    // 6. BrowserWindow integration with real in-memory SQLite storage
    let app = AdwApplication::builder()
        .application_id("org.thebrowser.test.ui_overhaul")
        .build();

    app.connect_activate(|app| {
        let storage = StorageManager::open_in_memory().ok();
        let win = BrowserWindow::new(app.upcast_ref(), storage, "about:blank");

        // Omnibox
        win.show_omnibox();
        win.hide_omnibox();

        // Sidebar folding
        win.toggle_sidebar();
        win.toggle_sidebar();

        // Tab layout dual-mode switch
        win.toggle_tab_layout(); // horizontal
        win.toggle_tab_layout(); // vertical

        // Navigation to home
        win.navigate_active("thebrowser://home");

        // Tab operations
        let tab1 = win.active_tab_id().expect("initial tab");
        let tab2 = win.open_tab("https://archlinux.org");
        let tab3 = win.open_tab("https://rust-lang.org");

        win.toggle_pin(tab2);
        win.duplicate_tab(tab3);

        win.close_tabs_to_right(tab2);
        win.reopen_closed_tab();

        win.toggle_active_bookmark();
        win.toggle_active_bookmark();

        win.close_tab(tab1);

        app.quit();
    });

    app.run_with_args(&["test"]);
}
