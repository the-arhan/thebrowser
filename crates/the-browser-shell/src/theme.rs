//! Minimalist dark theme and CSS stylesheet for The Browser.

use gtk4::gdk::Display;
use gtk4::CssProvider;

const CSS_THEME: &str = r#"
/* Global Window and Backgrounds */
window.the-browser-window {
    background-color: #121214;
    color: #f4f4f5;
}

/* Base Horizontal Layout Container */
.main-layout {
    background-color: #121214;
}

/* Browsing Stage Card */
.stage-card {
    background-color: #09090b;
    border: 1px solid #27272a;
    border-radius: 10px;
    margin: 6px;
    transition: border-color 150ms ease-in-out;
}

/* Stage Canvas */
.stage-canvas {
    background-color: #09090b;
}

/* Progress Indicator Flush on Top of Stage */
.load-progress {
    min-height: 2px;
    border: none;
    background-color: transparent;
}

.load-progress trough {
    background-color: transparent;
    border: none;
    min-height: 2px;
}

.load-progress progress {
    background-color: #38bdf8;
    border: none;
    min-height: 2px;
    border-radius: 1px;
}

/* Sleek Icon Buttons */
.nav-button {
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 5px;
    color: #a1a1aa;
    min-width: 26px;
    min-height: 26px;
    transition: background-color 120ms ease-in-out, color 120ms ease-in-out;
}

.nav-button:hover:not(:disabled) {
    background-color: #27272a;
    color: #f4f4f5;
}

.nav-button:disabled {
    opacity: 0.35;
}

/* Vertical Sidebar Pane */
.sidebar-pane {
    background-color: #141416;
    border-right: 1px solid #202024;
    transition: min-width 150ms cubic-bezier(0.2, 0.0, 0.0, 1.0);
}

.sidebar-header {
    padding: 10px 10px 8px 10px;
    border-bottom: 1px solid #1c1c20;
}

.space-pill {
    padding: 4px 8px;
    background-color: #1e1e22;
    border: 1px solid #27272a;
    border-radius: 6px;
}

.space-dot {
    color: #38bdf8;
    font-size: 8px;
}

.space-title {
    font-size: 11px;
    font-weight: 600;
    color: #e4e4e7;
    letter-spacing: 0.2px;
}

.sidebar-scroll {
    padding: 6px;
}

.sidebar-tab-list {
    background-color: transparent;
}

/* Individual Vertical Tab Item */
.sidebar-tab-item {
    background-color: transparent;
    color: #a1a1aa;
    border-radius: 6px;
    padding: 6px 8px;
    min-height: 32px;
    border: 1px solid transparent;
    transition: all 100ms ease-in-out;
}

.sidebar-tab-item:hover {
    background-color: #1e1e22;
    color: #e4e4e7;
}

.sidebar-tab-item.active {
    background-color: #27272a;
    color: #f4f4f5;
    border-color: #38383f;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

.tab-monogram {
    font-size: 11px;
    font-weight: 700;
    color: #a1a1aa;
    background-color: #222226;
    border: 1px solid #2d2d32;
    border-radius: 4px;
    min-width: 18px;
    min-height: 18px;
    padding: 1px 4px;
}

.sidebar-tab-item.active .tab-monogram {
    color: #38bdf8;
    border-color: #38bdf8;
    background-color: #1e293b;
}

.sidebar-tab-title {
    font-size: 12px;
    font-weight: 500;
}

.sidebar-tab-close {
    background: transparent;
    border: none;
    border-radius: 4px;
    padding: 2px;
    color: #71717a;
    min-width: 16px;
    min-height: 16px;
    opacity: 0.0;
    transition: opacity 100ms ease-in-out, background-color 100ms ease-in-out;
}

.sidebar-tab-item:hover .sidebar-tab-close,
.sidebar-tab-item.active .sidebar-tab-close {
    opacity: 1.0;
}

.sidebar-tab-close:hover {
    background-color: #3f3f46;
    color: #f4f4f5;
}

/* Folded Sidebar */
.sidebar-pane.folded {
    border-right: 1px solid #202024;
}

.sidebar-pane.folded .sidebar-tab-item {
    padding: 6px 4px;
}

/* Floating Omnibox Modal Scrim & Card */
.omnibox-overlay {
    background-color: rgba(9, 9, 11, 0.65);
}

.omnibox-card {
    background-color: #1c1c20;
    border: 1px solid #38383f;
    border-radius: 10px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.65);
    padding: 4px 6px;
    transition: box-shadow 150ms ease-in-out, border-color 150ms ease-in-out;
}

.omnibox-card:focus-within {
    border-color: #52525b;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.75), 0 0 0 1px #52525b;
}

.omnibox-input-row {
    padding: 4px 10px;
    min-height: 38px;
}

.omnibox-icon {
    color: #71717a;
}

.omnibox-entry {
    background-color: transparent;
    border: none;
    box-shadow: none;
    color: #f4f4f5;
    font-size: 14px;
    font-family: system-ui, -apple-system, sans-serif;
    outline: none;
}

.omnibox-entry:focus {
    box-shadow: none;
    outline: none;
}

.omnibox-badge {
    font-size: 10px;
    font-weight: 600;
    color: #71717a;
    background-color: #27272a;
    border-radius: 4px;
    padding: 2px 6px;
    letter-spacing: 0.2px;
}

/* Horizontal Tab Strip (Alternative Dual Mode) */
.tab-strip {
    background-color: #121214;
    border-bottom: 1px solid #202024;
    padding: 4px 8px 0 8px;
}

.tab-item {
    background-color: #18181b;
    color: #a1a1aa;
    border-top: 1px solid #27272a;
    border-left: 1px solid #27272a;
    border-right: 1px solid #27272a;
    border-bottom: 1px solid #202024;
    border-radius: 6px 6px 0 0;
    padding: 5px 10px;
    margin-right: 3px;
    min-width: 110px;
    transition: all 120ms ease-in-out;
}

.tab-item:hover {
    background-color: #202024;
    color: #e4e4e7;
}

.tab-item.active {
    background-color: #27272a;
    color: #f4f4f5;
    border-top: 1px solid #3f3f46;
    border-left: 1px solid #3f3f46;
    border-right: 1px solid #3f3f46;
    border-bottom: 1px solid #27272a;
}

.tab-title {
    font-size: 12px;
    font-weight: 500;
}

.tab-close-btn {
    background: transparent;
    border: none;
    border-radius: 4px;
    padding: 2px;
    color: #71717a;
    min-width: 18px;
    min-height: 18px;
}

.tab-close-btn:hover {
    background-color: #3f3f46;
    color: #f4f4f5;
}

/* Navigation Toolbar */
.nav-bar {
    background-color: #141416;
    border-bottom: 1px solid #202024;
    padding: 6px 8px;
    min-height: 40px;
}

.nav-button-group {
    background-color: transparent;
}

.address-container {
    background-color: #1b1b1f;
    border: 1px solid #2a2a30;
    border-radius: 8px;
    padding: 0 4px 0 8px;
    transition: all 120ms ease-in-out;
}

.address-container:focus-within {
    border-color: #38bdf8;
    box-shadow: 0 0 0 1px #38bdf8;
}

.nav-address-entry {
    background-color: transparent;
    border: none;
    box-shadow: none;
    color: #f4f4f5;
    font-size: 13px;
    font-family: system-ui, -apple-system, sans-serif;
    outline: none;
}

.nav-address-entry:focus {
    box-shadow: none;
    outline: none;
}

.star-button {
    background: transparent;
    border: none;
    border-radius: 4px;
    padding: 4px;
    color: #71717a;
    min-width: 24px;
    min-height: 24px;
    transition: color 120ms ease-in-out;
}

.star-button:hover {
    color: #f4f4f5;
}

.star-button.starred {
    color: #eab308;
}

.nav-actions-group {
    background-color: transparent;
}

/* Popover Menus & Context Menu */
.nav-menu-popover contents,
.tab-context-popover contents {
    background-color: #1a1a1e;
    border: 1px solid #2d2d34;
    border-radius: 8px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.6);
    padding: 4px;
}

.menu-item-button {
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 6px 10px;
    color: #e4e4e7;
    font-size: 13px;
    min-height: 30px;
    transition: background-color 100ms ease-in-out;
}

.menu-item-button:hover {
    background-color: #27272a;
    color: #f4f4f5;
}

.menu-shortcut-label {
    font-size: 11px;
    color: #71717a;
    font-family: monospace;
}

.destructive-action {
    background-color: #ef4444;
    color: #ffffff;
    border-radius: 6px;
    border: none;
    font-size: 12px;
    font-weight: 600;
    padding: 4px 10px;
}

.destructive-action:hover {
    background-color: #dc2626;
}

/* Pinned Tabs & Sections */
.sidebar-pinned-section {
    padding: 6px 8px 4px;
    border-bottom: 1px solid #1c1c20;
}

.sidebar-section-title {
    font-size: 9px;
    font-weight: 700;
    color: #71717a;
    letter-spacing: 0.6px;
    padding-left: 2px;
}

.sidebar-pinned-grid {
    padding: 2px 0;
}

.sidebar-tab-item.pinned {
    background-color: #1a1a1d;
    border: 1px solid #2e2e34;
}

.sidebar-tab-item.pinned.active {
    border-color: #38bdf8;
    background-color: #1e293b;
}

/* Sleeping (Lazy) and Failed Tab States */
.sidebar-tab-item.sleeping,
.tab-item.sleeping {
    opacity: 0.65;
}

.sidebar-tab-item.load-failed,
.tab-item.load-failed {
    border-color: #ef4444;
    color: #f87171;
}

.sidebar-tab-item.load-failed .tab-monogram {
    color: #ef4444;
    border-color: #ef4444;
}

/* Tab Search Card & Dialog */
.tab-search-card {
    background-color: #18181b;
    border: 1px solid #38383f;
    border-radius: 12px;
    box-shadow: 0 20px 48px rgba(0, 0, 0, 0.75);
    overflow: hidden;
}

.tab-search-entry-row {
    background-color: #141416;
    border-bottom: 1px solid #27272a;
    padding: 8px 12px;
}

.tab-search-entry {
    background-color: transparent;
    border: none;
    box-shadow: none;
    color: #f4f4f5;
    font-size: 14px;
    font-family: system-ui, -apple-system, sans-serif;
    outline: none;
}

.tab-search-entry:focus {
    box-shadow: none;
    outline: none;
}

.tab-search-list {
    background-color: transparent;
    padding: 6px;
}

.tab-search-row {
    border-radius: 6px;
    transition: background-color 100ms ease-in-out;
}

.tab-search-row:hover,
.tab-search-row:selected {
    background-color: #27272a;
    color: #f4f4f5;
}

.tab-search-title {
    font-size: 13px;
    font-weight: 600;
    color: #f4f4f5;
}

.tab-search-url {
    font-size: 11px;
    color: #71717a;
}

.tab-badge {
    font-size: 10px;
    font-weight: 600;
    color: #38bdf8;
    background-color: #0c4a6e;
    border-radius: 4px;
    padding: 2px 6px;
}

.tab-badge-dim {
    font-size: 10px;
    font-weight: 500;
    color: #71717a;
    background-color: #27272a;
    border-radius: 4px;
    padding: 2px 6px;
}

.tab-badge-fail {
    font-size: 10px;
    font-weight: 600;
    color: #f87171;
    background-color: #450a0a;
    border-radius: 4px;
    padding: 2px 6px;
}

/* Drag and drop reordering indicators */
.sidebar-tab-item.drop-before,
.tab-item.drop-before {
    border-top: 2px solid #38bdf8 !important;
}

.sidebar-tab-item.drop-after,
.tab-item.drop-after {
    border-bottom: 2px solid #38bdf8 !important;
}

.sidebar-tab-item.dragging,
.tab-item.dragging {
    opacity: 0.35;
}

/* Space switcher */
.space-switcher-btn {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    padding: 4px 6px;
    color: #f4f4f5;
    font-weight: 600;
    transition: background-color 100ms ease-in-out;
}

.space-switcher-btn:hover {
    background-color: #202024;
    border-color: #27272a;
}

.space-accent-dot {
    font-size: 14px;
    margin-right: 4px;
}

.space-pills-row {
    padding: 2px 8px 6px 8px;
}

.space-quick-pill {
    background-color: #1c1c1f;
    border: 1px solid #27272a;
    border-radius: 12px;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 500;
    color: #a1a1aa;
    transition: all 120ms ease-in-out;
}

.space-quick-pill:hover {
    background-color: #27272a;
    color: #f4f4f5;
}

.space-quick-pill.active {
    background-color: #2e2e34;
    color: #f4f4f5;
    border-color: #52525b;
    font-weight: 600;
}

.space-quick-pill.drop-target-hover {
    border-color: #38bdf8;
    box-shadow: 0 0 0 1px #38bdf8;
}

/* Accent Color Classes */
.accent-indigo { background-color: #6366f1 !important; }
.accent-cyan { background-color: #06b6d4 !important; }
.accent-emerald { background-color: #10b981 !important; }
.accent-amber { background-color: #f59e0b !important; }
.accent-rose { background-color: #f43f5e !important; }
.accent-purple { background-color: #a855f7 !important; }
.accent-graphite { background-color: #64748b !important; }

/* Audio & Snoozing Indicators */
.tab-audio-icon {
    font-size: 11px;
    color: #38bdf8;
    margin-right: 2px;
}

.sidebar-tab-item.sleeping .sidebar-tab-title,
.tab-item.sleeping .tab-title {
    opacity: 0.55;
    font-style: italic;
}

.tab-badge-space {
    font-size: 10px;
    font-weight: 600;
    color: #a78bfa;
    background-color: #2e1065;
    border-radius: 4px;
    padding: 2px 6px;
}
"#;

pub fn install_theme() {
    if let Some(display) = Display::default() {
        let provider = CssProvider::new();
        provider.load_from_string(CSS_THEME);

        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        log::info!("Installed minimalist dark CSS theme");
    }
}
