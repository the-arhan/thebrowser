use gtk4::prelude::*;
use libadwaita::Application as AdwApplication;
use the_browser_shell::BrowserWindow;

#[test]
fn test_window_reload_and_tab_operations_no_panic() {
    if gtk4::init().is_err() {
        // Skip headless test if no display available
        return;
    }

    let app = AdwApplication::builder()
        .application_id("org.thebrowser.test.reload")
        .build();

    app.connect_activate(|app| {
        let win = BrowserWindow::new(app.upcast_ref(), None, "about:blank");
        
        // 1. Verify active view exists and reload does not panic
        let view = win.active_view().expect("Active view should exist");
        view.reload();
        view.reload_bypass_cache();

        // 2. Open a second tab and verify reload again
        let tab2 = win.open_tab("about:blank");
        let view2 = win.active_view().expect("Active view 2 should exist");
        view2.reload();

        // 3. Switch tabs
        win.select_tab_index(0);
        win.select_tab_index(1);

        // 4. Close tab
        win.close_tab(tab2);

        // Success - quit test
        app.quit();
    });

    app.run_with_args(&["test"]);
}

