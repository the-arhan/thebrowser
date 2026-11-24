//! WebKitGTK 6.0 WebView encapsulation and signal delegation.

use crate::dusk::Dusk;
use crate::session::EngineSession;
use gtk4::prelude::*;
use std::rc::Rc;
use webkit6::prelude::*;
use webkit6::{LoadEvent, Settings, UserContentManager, WebView};

/// High-level wrapper over `webkit6::WebView`.
#[derive(Clone)]
pub struct EngineView {
    inner: WebView,
    session: EngineSession,
    content_manager: UserContentManager,
}

impl EngineView {
    /// Construct a new EngineView instance with the provided session and settings.
    pub fn new(session: &EngineSession, settings: &Settings) -> Self {
        let content_manager = UserContentManager::new();
        // Attach intelligent Dusk dark fallback stylesheet
        Dusk::attach(&content_manager);

        let view = WebView::builder()
            .network_session(session.inner())
            .settings(settings)
            .user_content_manager(&content_manager)
            .build();

        Self {
            inner: view,
            session: session.clone(),
            content_manager,
        }
    }

    pub fn inner(&self) -> &WebView {
        &self.inner
    }

    pub fn session(&self) -> &EngineSession {
        &self.session
    }

    pub fn is_playing_audio(&self) -> bool {
        self.inner.is_playing_audio()
    }

    pub fn connect_audio_notify<F: Fn(bool) + 'static>(&self, f: F) {
        self.inner.connect_is_playing_audio_notify(move |v| {
            f(v.is_playing_audio());
        });
    }

    pub fn content_manager(&self) -> &UserContentManager {
        &self.content_manager
    }

    pub fn widget(&self) -> &gtk4::Widget {
        self.inner.upcast_ref()
    }

    pub fn load_html(&self, html: &str, base_uri: Option<&str>) {
        self.inner.load_html(html, base_uri);
    }

    pub fn load_home(
        &self,
        engine: &the_browser_core::SearchEngine,
        shortcuts: &[the_browser_storage::Shortcut],
        recent: &[the_browser_storage::HistoryEntry],
        show_shortcuts: bool,
        show_recent: bool,
    ) {
        let html = crate::home::generate_home_html(engine, shortcuts, recent, show_shortcuts, show_recent);
        self.load_html(&html, Some("thebrowser://home"));
    }

    pub fn load_uri(&self, uri: &str) {
        log::debug!("EngineView: Loading URI: {uri}");
        if uri == "thebrowser://home" || uri == "about:home" {
            let default_engine = the_browser_core::SearchEngine::duckduckgo();
            let html = crate::home::generate_home_html(&default_engine, &[], &[], false, false);
            self.load_html(&html, Some("thebrowser://home"));
        } else {
            self.inner.load_uri(uri);
        }
    }

    pub fn reload(&self) {
        self.inner.reload();
    }

    pub fn reload_bypass_cache(&self) {
        self.inner.reload_bypass_cache();
    }

    pub fn go_back(&self) {
        self.inner.go_back();
    }

    pub fn go_forward(&self) {
        self.inner.go_forward();
    }

    pub fn stop_loading(&self) {
        self.inner.stop_loading();
    }

    pub fn can_go_back(&self) -> bool {
        self.inner.can_go_back()
    }

    pub fn can_go_forward(&self) -> bool {
        self.inner.can_go_forward()
    }

    pub fn uri(&self) -> Option<String> {
        self.inner.uri().map(|s| s.to_string())
    }

    pub fn title(&self) -> Option<String> {
        self.inner.title().map(|s| s.to_string())
    }

    pub fn is_loading(&self) -> bool {
        self.inner.is_loading()
    }

    pub fn estimated_load_progress(&self) -> f64 {
        self.inner.estimated_load_progress()
    }

    /// Retrieve serialized WebKit session state bytes (history and state).
    pub fn session_state(&self) -> Option<Vec<u8>> {
        self.inner
            .session_state()
            .and_then(|s| s.serialize())
            .map(|b| b.to_vec())
    }

    /// Restore WebKit session state from serialized bytes.
    pub fn restore_session_state(&self, data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }
        let bytes = glib::Bytes::from(data);
        let state = webkit6::WebViewSessionState::new(&bytes);
        self.inner.restore_session_state(&state);
        true
    }

    /// Connects all lifecycle and navigation events to the given callbacks.
    #[allow(clippy::too_many_arguments)]
    pub fn setup_signals<FTitle, FUri, FProgress, FLoading, FNavState, FClose, FNewTab, FLoadFailed>(
        &self,
        on_title: FTitle,
        on_uri: FUri,
        on_progress: FProgress,
        on_loading: FLoading,
        on_nav_state: FNavState,
        on_close: FClose,
        on_new_tab: FNewTab,
        on_load_failed: FLoadFailed,
    ) where
        FTitle: Fn(&str) + 'static,
        FUri: Fn(&str) + 'static,
        FProgress: Fn(f64) + 'static,
        FLoading: Fn(bool) + 'static,
        FNavState: Fn(bool, bool) + 'static,
        FClose: Fn() + 'static,
        FNewTab: Fn(&str) + 'static,
        FLoadFailed: Fn(&str) + 'static,
    {
        let on_title = Rc::new(on_title);
        let on_uri = Rc::new(on_uri);
        let on_progress = Rc::new(on_progress);
        let on_loading = Rc::new(on_loading);
        let on_nav_state = Rc::new(on_nav_state);
        let on_close = Rc::new(on_close);
        let on_new_tab = Rc::new(on_new_tab);
        let on_load_failed = Rc::new(on_load_failed);

        // Title changes
        {
            let on_title = Rc::clone(&on_title);
            self.inner.connect_title_notify(move |view| {
                if let Some(title) = view.title() {
                    on_title(&title);
                }
            });
        }

        // URI changes
        {
            let on_uri = Rc::clone(&on_uri);
            self.inner.connect_uri_notify(move |view| {
                if let Some(uri) = view.uri() {
                    on_uri(&uri);
                }
            });
        }

        // Progress updates
        {
            let on_progress = Rc::clone(&on_progress);
            self.inner.connect_estimated_load_progress_notify(move |view| {
                on_progress(view.estimated_load_progress());
            });
        }

        // Loading state changes
        {
            let on_loading = Rc::clone(&on_loading);
            let on_nav_state = Rc::clone(&on_nav_state);
            self.inner.connect_is_loading_notify(move |view| {
                let loading = view.is_loading();
                on_loading(loading);
                on_nav_state(view.can_go_back(), view.can_go_forward());
            });
        }

        // Load changed events for navigation state synchronization
        {
            let on_nav_state = Rc::clone(&on_nav_state);
            let on_uri = Rc::clone(&on_uri);
            self.inner.connect_load_changed(move |view, event| {
                on_nav_state(view.can_go_back(), view.can_go_forward());
                if event == LoadEvent::Committed && let Some(uri) = view.uri() {
                    on_uri(&uri);
                }
            });
        }

        // Window / tab close requested by JavaScript
        {
            let on_close = Rc::clone(&on_close);
            self.inner.connect_close(move |_| {
                on_close();
            });
        }

        // Handle target="_blank" or window.open by delegating to new tab handler
        {
            let on_new_tab = Rc::clone(&on_new_tab);
            self.inner.connect_create(move |_view, action| {
                if let Some(req) = action.request() && let Some(uri) = req.uri() {
                    on_new_tab(&uri);
                }
                // Returning None prevents WebKit from opening an unmanaged popup window
                None
            });
        }

        // Navigation failure notification
        {
            let on_load_failed = Rc::clone(&on_load_failed);
            self.inner.connect_load_failed(move |_view, _event, _failing_uri, error| {
                on_load_failed(&error.to_string());
                false
            });
        }
    }
}
