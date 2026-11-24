3# The Browser

An independent, distraction-free desktop web browser engineered in **Rust** for **Arch Linux**, built on **GTK 4**, **Libadwaita**, **WebKitGTK 6.0**, and **SQLite 3**.

Inspired by the minimalist philosophy of Search Browser for macOS, The Browser strips away promotional clutter, heavy toolbars, synchronization accounts, and telemetry, delivering a fast, quiet, keyboard-centric web browsing tool.

---

## 1. Architecture & Workspace Structure

The project is structured as a modular Cargo workspace:

```
the-browser/
├── Cargo.toml                      # Workspace manifest
├── crates/
│   ├── the-browser-core/           # Domain models, URL/search resolution, tab lifecycle
│   │   ├── src/navigation.rs       # Domain & query detection algorithms
│   │   ├── src/tab.rs              # TabManager in-memory state machine
│   │   └── src/error.rs            # Core error types
│   ├── the-browser-storage/        # SQLite persistence layer
│   │   ├── src/manager.rs          # WAL-mode database interface & FTS5 search
│   │   ├── src/models.rs           # History and session data models
│   │   └── src/error.rs            # Storage error handling
│   ├── the-browser-engine/         # WebKitGTK 6.0 abstraction layer
│   │   ├── src/view.rs             # WebView lifecycle & signal delegation
│   │   ├── src/session.rs          # NetworkSession & cookie jar isolation
│   │   └── src/settings.rs         # Engine capabilities & user agent
│   └── the-browser-shell/          # Native GTK4 + Libadwaita desktop UI
│       ├── src/app.rs              # Libadwaita application lifecycle
│       ├── src/window.rs           # Main browser window & event controllers
│       ├── src/tab_strip.rs        # Custom minimalist tab strip widget
│       ├── src/theme.rs            # Restrained graphite dark CSS stylesheet
│       └── src/main.rs             # Application binary entry point
```

---

## 2. Arch Linux Build Instructions

### 2.1 System Dependencies
Ensure the official Arch Linux development toolchain and required GUI/rendering libraries are installed:

```bash
sudo pacman -S --needed \
    base-devel \
    rust \
    cargo \
    gtk4 \
    libadwaita \
    webkitgtk-6.0 \
    sqlite \
    pkgconf
```

### 2.2 Compilation and Running
Clone the repository, enter the workspace directory, and build:

```bash
cd the-browser

# Verify all crates compile cleanly
cargo check --workspace

# Run automated test suites (core domain & storage tests)
cargo test --workspace

# Compile optimized debug or release binary
cargo build

# Launch The Browser with default homepage (archlinux.org)
cargo run

# Launch The Browser pointing to a specific URL or query
cargo run -- https://news.ycombinator.com
cargo run -- "arch linux rust"
```

The compiled binary will be placed at `target/debug/the-browser` (or `target/release/the-browser`).

---

## 3. UI Master Overhaul & Implemented Features

- **Browsing Stage Card Framing:**
  - Distinct floating surface (`.stage-card`) with 10px corner radius, hairline graphite border (`#27272a`), and 6px gutters over the `#121214` application frame.
  - Flush 2px progress bar (`#38bdf8`) running along the top card boundary during active page loads.
  - No permanent toolbars or address bars cluttering the browsing viewport.
- **Contextual Floating Omnibox (`Ctrl+L` / `Ctrl+T`):**
  - Suspended modal card (`AdwClamp`, 580px max width) elevated in the upper third of the window over a 65% opacity dark scrim.
  - Auto-focuses and selects existing URLs for immediate overtyping.
  - Automatic URL resolution vs DuckDuckGo search queries.
  - Clean dismissal on `Enter` (navigates), `Escape`, or clicking anywhere on the outer scrim.
- **Vertical Tab Sidebar (Primary Default Navigation):**
  - Left-hand vertical tab column (`sidebar.rs`) featuring a Space indicator pill, pinned tab area, and tab items with clean letter monograms.
  - Real-time loading spinners and hover-revealed close triggers.
  - **Sidebar Folding (`Ctrl+S`):** Collapses between full width (210px with titles) and compact monogram icon rail (46px).
- **Dual-Mode Tab Layout (`Ctrl+Shift+S`):**
  - Instant hotkey switching between the vertical sidebar and an alternative top horizontal tab strip without reloading web views or losing tab state.
- **Intelligent Dark Browsing Architecture (Dusk):**
  - Native `prefers-color-scheme: dark` propagated via `AdwStyleManager::ForceDark`.
  - Built-in declarative `UserStyleSheet` ("Dusk") injected at `document-start` into `WebKitUserContentManager`.
  - Darkens light-only websites while strictly preserving media (images, videos, SVGs, canvases) from color inversion.
- **SQLite History Storage:**
  - Visits automatically logged to `~/.local/share/the-browser/browser.db` in SQLite **WAL mode**.
  - Sub-millisecond full-text search across past titles and URLs using **FTS5**.
- **Keyboard-First Shortcuts:**
  - `Ctrl+L` — Open / focus floating Omnibox.
  - `Ctrl+T` — Open new tab and activate Omnibox.
  - `Ctrl+W` — Dismiss Omnibox (if open) or close active tab.
  - `Ctrl+S` — Fold / unfold vertical sidebar.
  - `Ctrl+Shift+S` — Toggle between vertical sidebar and horizontal tab strip.
  - `Ctrl+R` / `F5` — Reload current page.
  - `Ctrl+Shift+R` / `Ctrl+F5` — Hard reload (bypass cache).
  - `Alt+Left` — Navigate back in history.
  - `Alt+Right` — Navigate forward in history.
  - `Ctrl+Tab` / `Ctrl+PageDown` — Next tab in circular order.
  - `Ctrl+Shift+Tab` / `Ctrl+PageUp` — Previous tab in circular order.
  - `Ctrl+1` .. `Ctrl+9` — Switch directly to tab index 1–9.
  - `Escape` — Dismiss Omnibox and return focus to web content.

---

## 4. Known Limitations & Upcoming Milestones

### Known Limitations (Milestone 1 Scope)
- **Tab Layout:** Currently provides a horizontal minimalist tab bar; vertical sidebar layout (`Side.swift` equivalent) is scheduled for Milestone 2.
- **Tab Sleeping:** Web views remain in memory during the active session; automated snapshotting and process release under memory pressure will arrive in Milestone 2.
- **Single Session Partition:** All non-private tabs currently share the default network session; multi-tenant Spaces (`WebKitNetworkSession` partitioning) will be implemented in Milestone 3.
- **Extensions:** WebExtension controller support is deferred pending upstream WebKitGTK stabilization; Milestone 4 will introduce UserScript/UserStyleSheet management.

### Roadmap to Milestone 2
- Dual-mode tab navigation (hotkey toggle between horizontal strip and vertical sidebar).
- Split View dual-pane presentation using `GtkPaned`.
- Tab sleeping engine via `WebKitWebViewSessionState` serialization and raster snapshotting.
- Omnibox dropdown with live SQLite FTS5 history and bookmark suggestions.

