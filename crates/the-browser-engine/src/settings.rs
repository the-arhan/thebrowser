//! Engine configuration and settings for WebKitGTK 6.0.

use webkit6::Settings;

pub struct EngineSettings;

impl EngineSettings {
    /// Creates an optimized WebKit settings instance.
    pub fn create_default() -> Settings {
        let settings = Settings::new();

        // Modern browser capabilities
        settings.set_enable_developer_extras(true);
        settings.set_enable_javascript(true);
        settings.set_enable_webgl(true);
        settings.set_enable_smooth_scrolling(true);
        settings.set_enable_media_stream(true);
        settings.set_enable_mediasource(true);

        // Standard user-agent name
        settings.set_user_agent_with_application_details(Some("TheBrowser"), Some("0.1.0"));

        settings
    }
}
