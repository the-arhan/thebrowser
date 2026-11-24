//! The Browser Shell - GTK4 and Libadwaita desktop application.

pub mod app;
pub mod navbar;
pub mod omnibox;
pub mod settings_dialog;
pub mod sidebar;
pub mod space_dialog;
pub mod tab_menu;
pub mod tab_search;
pub mod tab_strip;
pub mod theme;
pub mod window;

pub use app::BrowserApplication;
pub use navbar::NavBar;
pub use omnibox::Omnibox;
pub use settings_dialog::SettingsDialog;
pub use sidebar::SideBar;
pub use space_dialog::{SpaceDialog, SpaceDialogResult};
pub use tab_menu::{TabContextMenu, TabMenuAction};
pub use tab_search::TabSearchDialog;
pub use tab_strip::TabStrip;
pub use window::BrowserWindow;
