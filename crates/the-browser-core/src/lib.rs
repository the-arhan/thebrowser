//! Domain models, navigation parsing, and core state management for The Browser.

pub mod error;
pub mod navigation;
pub mod space;
pub mod tab;

pub use error::CoreError;
pub use navigation::{NavigationTarget, SearchEngine, resolve_input};
pub use space::{SpaceId, SpaceInfo};
pub use tab::{TabId, TabInfo, TabLifecycleState, TabManager};
