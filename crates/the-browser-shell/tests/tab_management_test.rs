use gtk4::prelude::*;
use libadwaita::Application as AdwApplication;
use the_browser_core::TabInfo;
use the_browser_shell::{BrowserWindow, TabSearchDialog};
use the_browser_storage::StorageManager;

#[test]
fn test_tab_search_dialog_component() {
    if gtk4::init().is_err() {
        return;
    }

    let dummy_win = gtk4::Window::new();
    let dialog = TabSearchDialog::new(&dummy_win);

    let tab1 = TabInfo::new("https://archlinux.org", "Arch Linux — Operating System");
    let mut tab2 = TabInfo::new("https://rust-lang.org", "Rust Programming Language");
    tab2.is_pinned = true;
    let mut tab3 = TabInfo::new("https://crates.io", "Rust Package Registry");
    tab3.is_lazy = true;

    let tabs = vec![tab1, tab2, tab3];
    dialog.show_with_tabs(&tabs, None, &[]);
    dialog.connect_select_tab(|_id| {});
}

#[test]
fn test_milestone01_tab_management_and_session_continuity() {
    if gtk4::init().is_err() {
        return;
    }

    let app = AdwApplication::builder()
        .application_id("org.thebrowser.test.tab_management")
        .build();

    app.connect_activate(|app| {
        // Use a temp file SQLite DB so two window instances can share state
        let temp_dir = std::env::temp_dir().join(format!("thebrowser_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("test_session.db");

        {
            let storage = StorageManager::open(&db_path).expect("open db");
            let win = BrowserWindow::new(app.upcast_ref(), Some(storage), "thebrowser://home");

            let _id1 = win.active_tab_id().expect("tab 1");
            let id2 = win.open_tab("https://archlinux.org");
            let id3 = win.open_tab("https://doc.rust-lang.org");

            // Pin tab 2
            win.toggle_pin(id2);

            // Reorder: move tab 0 to 1
            win.move_tab(0, 1);

            // Tab Search dialog
            win.show_tab_search();

            // Select tab index
            win.select_tab_index(0);
            win.select_tab_index(2);

            // Duplicate tab 3
            win.duplicate_tab(id3);

            // Close active tab and verify
            if let Some(active) = win.active_tab_id() {
                win.close_tab(active);
            }

            // Reopen closed tab
            win.reopen_closed_tab();

            // Explicitly save session before teardown
            win.save_current_session();
        }

        // Verify session was persisted in SQLite
        {
            let storage = StorageManager::open(&db_path).expect("open db again");
            let saved_tabs = storage.load_session().expect("load session");
            assert!(
                saved_tabs.len() >= 2,
                "Expected at least 2 saved tabs, found {}",
                saved_tabs.len()
            );

            let has_pinned = saved_tabs.iter().any(|t| t.is_pinned);
            assert!(has_pinned, "Expected at least one pinned tab in session");

            // Open Window 2 with no initial URL to trigger session restoration
            let win2 = BrowserWindow::new(app.upcast_ref(), Some(storage), "");

            // Window 2 should have restored tabs lazily
            let _win2_active = win2.active_tab_id().expect("active tab in win2");

            // Switching to another restored tab should instantiate its view on-demand
            win2.select_tab_index(0);
            assert_ne!(win2.active_tab_id(), None);

            // Final tab close behavior
            let active_id = win2.active_tab_id().unwrap();
            win2.close_other_tabs(active_id);
            // Now only 1 tab remains; closing it should reset to home rather than hard panic
            win2.close_tab(active_id);
        }

        // Clean up temp file
        let _ = std::fs::remove_dir_all(&temp_dir);

        app.quit();
    });

    app.run_with_args(&["test"]);
}
