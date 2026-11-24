//! Application lifecycle and Libadwaita orchestration.

use crate::theme;
use crate::window::BrowserWindow;
use gio::ApplicationFlags;
use gio::prelude::*;
use libadwaita::{Application as AdwApplication, ColorScheme, StyleManager};
use std::cell::RefCell;
use std::rc::Rc;
use the_browser_storage::StorageManager;

pub struct BrowserApplication {
    app: AdwApplication,
    _window: Rc<RefCell<Option<Rc<BrowserWindow>>>>,
}

impl BrowserApplication {
    pub fn new() -> Self {
        let app = AdwApplication::builder()
            .application_id("org.thebrowser.TheBrowser")
            .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
            .build();

        let window: Rc<RefCell<Option<Rc<BrowserWindow>>>> = Rc::new(RefCell::new(None));

        app.connect_startup(|_| {
            libadwaita::init().expect("Failed to initialize Libadwaita");
            theme::install_theme();

            // Enforce minimalist graphite dark appearance
            let style_mgr = StyleManager::default();
            style_mgr.set_color_scheme(ColorScheme::ForceDark);
        });

        {
            let window_ref = Rc::clone(&window);
            app.connect_command_line(move |app, cmdline| {
                let args = cmdline.arguments();
                let requested_url = if args.len() > 1 {
                    args[1].to_str().unwrap_or("").to_string()
                } else {
                    String::new()
                };

                let mut win_borrow = window_ref.borrow_mut();
                if let Some(ref win) = *win_borrow {
                    // Window is already running: open URL in a new tab and bring to front
                    let target = if requested_url.is_empty() {
                        "thebrowser://home"
                    } else {
                        &requested_url
                    };
                    win.open_tab(target);
                    win.present();
                } else {
                    // Initialize storage at standard Linux XDG data directory
                    let data_dir = glib::user_data_dir().join("the-browser");
                    let db_path = data_dir.join("browser.db");

                    let storage = match StorageManager::open(&db_path) {
                        Ok(mgr) => {
                            log::info!("Initialized SQLite database at: {}", db_path.display());
                            Some(mgr)
                        }
                        Err(err) => {
                            log::warn!("Failed to initialize database on disk ({err}), falling back to in-memory");
                            StorageManager::open_in_memory().ok()
                        }
                    };

                    let browser_window = BrowserWindow::new(app.upcast_ref(), storage, &requested_url);
                    browser_window.present();
                    *win_borrow = Some(browser_window);
                }

                glib::ExitCode::SUCCESS
            });
        }

        Self { app, _window: window }
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.run()
    }
}

impl Default for BrowserApplication {
    fn default() -> Self {
        Self::new()
    }
}
