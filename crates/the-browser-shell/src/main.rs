//! The Browser entry point.

use the_browser_shell::BrowserApplication;

fn main() -> glib::ExitCode {
    env_logger::init();
    log::info!("Starting The Browser (Milestone 1 Prototype)...");

    let app = BrowserApplication::new();
    app.run()
}
