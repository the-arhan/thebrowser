use gtk4::prelude::*;
use libadwaita::Application as AdwApplication;
use the_browser_core::{SpaceId, SpaceInfo, TabLifecycleState};
use the_browser_shell::BrowserWindow;
use the_browser_storage::StorageManager;

#[test]
fn test_milestone02_spaces_dnd_snoozing_complete() {
    if gtk4::init().is_err() {
        return;
    }

    let app = AdwApplication::builder()
        .application_id("org.thebrowser.test.milestone02")
        .build();

    app.connect_activate(|app| {
        let temp_dir = std::env::temp_dir().join(format!("thebrowser_spaces_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("spaces_session.db");

        let space_work_id = SpaceId::from_string("space-work-100");
        let space_personal_id = SpaceId::from_string("space-personal-200");

        // ====================================================================
        // DECISION 1: Spaces with Isolated Browsing Data & Session Persistence
        // ====================================================================
        {
            let storage = StorageManager::open(&db_path).expect("open db");
            let win = BrowserWindow::new(app.upcast_ref(), Some(storage), "thebrowser://home");

            // Verify default space exists
            let default_spaces = win.spaces();
            assert!(!default_spaces.is_empty(), "Should have at least default space");
            let default_id = win.active_space_id().expect("active space");

            // Verify session isolation directory paths
            let _session_def = win.session_for_space(&default_id);
            let session_work = win.session_for_space(&space_work_id);
            let session_pers = win.session_for_space(&space_personal_id);

            assert_ne!(
                session_work.data_dir(),
                session_pers.data_dir(),
                "Spaces must have distinct persistent data directories"
            );
            assert_ne!(
                session_work.cache_dir(),
                session_pers.cache_dir(),
                "Spaces must have distinct cache directories"
            );

            // Create Space 1: "Work"
            let work_space = SpaceInfo::new(
                space_work_id.clone(),
                "Work".to_string(),
                "#6366f1".to_string(),
                Some("folder-symbolic".to_string()),
                1,
            );
            win.create_space(work_space);
            assert_eq!(win.active_space_id(), Some(space_work_id.clone()));

            // Open tabs in Work space
            let _work_tab1 = win.open_tab("https://github.com");
            let work_tab2 = win.open_tab("https://docs.rs");

            let work_tabs = win.tabs_for_space(&space_work_id);
            assert!(work_tabs.len() >= 2, "Work space should have at least 2 tabs");

            // Create Space 2: "Personal"
            let pers_space = SpaceInfo::new(
                space_personal_id.clone(),
                "Personal".to_string(),
                "#f43f5e".to_string(),
                Some("user-symbolic".to_string()),
                2,
            );
            win.create_space(pers_space);
            assert_eq!(win.active_space_id(), Some(space_personal_id.clone()));

            // Personal space should start with its own tabs
            let pers_tab1 = win.open_tab("https://monkeytype.com");
            let pers_tabs = win.tabs_for_space(&space_personal_id);
            assert!(pers_tabs.iter().any(|t| t.id == pers_tab1));

            // Switch back to Work space
            win.switch_space(&space_work_id);
            assert_eq!(win.active_space_id(), Some(space_work_id.clone()));

            // Move work_tab2 from Work to Personal space
            win.move_tab_to_space(work_tab2, space_personal_id.clone());

            // Verify work_tab2 moved out of Work space and into Personal space
            let updated_work_tabs = win.tabs_for_space(&space_work_id);
            assert!(
                !updated_work_tabs.iter().any(|t| t.id == work_tab2),
                "Tab should be removed from Work space"
            );

            let updated_pers_tabs = win.tabs_for_space(&space_personal_id);
            assert!(
                updated_pers_tabs.iter().any(|t| t.id == work_tab2),
                "Tab should be present in Personal space"
            );

            // ====================================================================
            // DECISION 2: Tab Drag-and-Drop Reordering and Pinning
            // ====================================================================
            let tab_to_reorder = win.open_tab("https://archlinux.org");
            let curr_tabs = win.tabs_for_active_space();
            assert!(curr_tabs.len() >= 2);

            let first_tab = curr_tabs[0].id;
            win.reorder_tab(tab_to_reorder, first_tab, false);

            let reordered = win.tabs_for_active_space();
            let new_first = reordered.first().expect("first tab").id;
            assert_eq!(new_first, tab_to_reorder, "tab_to_reorder should now be at front");

            // Pin and unpin
            win.set_tab_pinned(tab_to_reorder, true);
            let tab_info_pinned = win.get_tab(tab_to_reorder).expect("tab info");
            assert!(tab_info_pinned.is_pinned, "tab should be pinned");

            win.set_tab_pinned(tab_to_reorder, false);
            let tab_info_unpinned = win.get_tab(tab_to_reorder).expect("tab info");
            assert!(!tab_info_unpinned.is_pinned, "tab should be unpinned");

            // ====================================================================
            // DECISION 3: Intelligent Tab Snoozing and Lifecycle States
            // ====================================================================
            let active_id = win.active_tab_id().expect("active tab");
            let bg_tab = win.open_tab("https://crates.io");
            win.select_tab(active_id); // ensure bg_tab is in background

            // Protected from snooze: active tab
            win.snooze_tab(active_id);
            let active_state = win.get_tab(active_id).expect("active tab info");
            assert_eq!(
                active_state.lifecycle_state,
                TabLifecycleState::Active,
                "Active tab must not be snoozed"
            );

            // Snooze background tab
            win.snooze_tab(bg_tab);
            let bg_state = win.get_tab(bg_tab).expect("bg tab info");
            assert_eq!(
                bg_state.lifecycle_state,
                TabLifecycleState::Sleeping,
                "Background tab should be snoozed"
            );
            assert!(bg_state.is_sleeping());

            // Wake background tab
            win.wake_tab(bg_tab);
            let bg_woken = win.get_tab(bg_tab).expect("bg tab info");
            assert_eq!(
                bg_woken.lifecycle_state,
                TabLifecycleState::Active,
                "Woken tab should be active"
            );

            // Snooze sweep
            win.select_tab(active_id);
            win.snooze_all_eligible_tabs();
            let bg_resnoozed = win.get_tab(bg_tab).expect("bg tab info");
            assert_eq!(
                bg_resnoozed.lifecycle_state,
                TabLifecycleState::Sleeping,
                "Sweep should snooze eligible background tab"
            );

            // Save session before closing
            win.save_current_session();
        }

        // Verify SQLite restoration in a second instance
        {
            let storage = StorageManager::open(&db_path).expect("open db again");
            let win2 = BrowserWindow::new(app.upcast_ref(), Some(storage), "");

            let restored_spaces = win2.spaces();
            assert!(
                restored_spaces.iter().any(|s| s.id == space_work_id),
                "Work space should be restored from database"
            );
            assert!(
                restored_spaces.iter().any(|s| s.id == space_personal_id),
                "Personal space should be restored from database"
            );

            // Test space deletion with data cleanup
            win2.delete_space(space_personal_id.clone(), true);
            let remaining = win2.spaces();
            assert!(
                !remaining.iter().any(|s| s.id == space_personal_id),
                "Deleted space should no longer be present"
            );
        }

        // Test completed successfully
        app.quit();
    });

    app.run_with_args(&["the-browser-test"]);
}
