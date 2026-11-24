//! Built-in Home / New Tab page renderer (`thebrowser://home`).

use the_browser_core::SearchEngine;
use the_browser_storage::{HistoryEntry, Shortcut};

/// Sanitize text for HTML rendering to prevent XSS / markup injection.
fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Compute a 1-character monogram for display when no favicon is present.
fn monogram(text: &str) -> String {
    let clean = text
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
        .trim();

    clean
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "●".to_string())
}

/// Extract form action URL and query parameter name from search engine template.
fn extract_form_action(engine: &SearchEngine) -> (&str, &str) {
    if let Some((action, query)) = engine.query_template.split_once('?') {
        let param = query.split('=').next().unwrap_or("q");
        (action, param)
    } else {
        ("https://duckduckgo.com/", "q")
    }
}

/// Generate dark graphite HTML document for `thebrowser://home`.
pub fn generate_home_html(
    engine: &SearchEngine,
    shortcuts: &[Shortcut],
    recent: &[HistoryEntry],
    show_shortcuts: bool,
    show_recent: bool,
) -> String {
    let (form_action, query_param) = extract_form_action(engine);

    let mut shortcuts_html = String::new();
    if show_shortcuts && !shortcuts.is_empty() {
        shortcuts_html.push_str(
            r#"<div class="section">
                <div class="section-title">Pinned &amp; Shortcuts</div>
                <div class="shortcuts-grid">"#,
        );
        for s in shortcuts {
            let escaped_title = escape_html(&s.title);
            let escaped_url = escape_html(&s.url);
            let mono = monogram(&s.title);
            shortcuts_html.push_str(&format!(
                r#"<a href="{escaped_url}" class="shortcut-card" title="{escaped_url}">
                    <div class="shortcut-icon">{mono}</div>
                    <div class="shortcut-label">{escaped_title}</div>
                </a>"#
            ));
        }
        shortcuts_html.push_str("</div></div>");
    }

    let mut recent_html = String::new();
    if show_recent && !recent.is_empty() {
        recent_html.push_str(
            r#"<div class="section">
                <div class="section-title">Recent History</div>
                <div class="recent-list">"#,
        );
        for item in recent.iter().take(8) {
            let escaped_title = escape_html(&item.title);
            let escaped_url = escape_html(&item.url);
            let mono = monogram(&item.title);
            recent_html.push_str(&format!(
                r#"<a href="{escaped_url}" class="recent-item" title="{escaped_url}">
                    <div class="recent-mono">{mono}</div>
                    <div class="recent-info">
                        <div class="recent-title">{escaped_title}</div>
                        <div class="recent-url">{escaped_url}</div>
                    </div>
                </a>"#
            ));
        }
        recent_html.push_str("</div></div>");
    }

    let engine_name = escape_html(&engine.name);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Home — The Browser</title>
    <style>
        :root {{
            --bg: #141416;
            --surface: #1e1e22;
            --surface-hover: #28282e;
            --border: #2c2c32;
            --text-primary: #ebebf0;
            --text-secondary: #92929e;
            --accent: #3584e4;
            --radius-lg: 12px;
            --radius-md: 8px;
            --radius-sm: 6px;
        }}
        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}
        body {{
            background-color: var(--bg);
            color: var(--text-primary);
            font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Ubuntu, sans-serif;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: flex-start;
            padding: 72px 24px 48px;
            user-select: none;
            -webkit-user-select: none;
        }}
        .container {{
            width: 100%;
            max-width: 680px;
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 32px;
        }}
        .brand {{
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 6px;
        }}
        .brand-logo {{
            font-size: 28px;
            font-weight: 700;
            letter-spacing: -0.5px;
            color: var(--text-primary);
        }}
        .brand-tagline {{
            font-size: 13px;
            color: var(--text-secondary);
            font-weight: 400;
        }}
        .search-container {{
            width: 100%;
        }}
        .search-form {{
            position: relative;
            width: 100%;
            display: flex;
            align-items: center;
        }}
        .search-input {{
            width: 100%;
            height: 48px;
            background-color: var(--surface);
            border: 1px solid var(--border);
            border-radius: var(--radius-lg);
            padding: 0 110px 0 46px;
            color: var(--text-primary);
            font-size: 15px;
            outline: none;
            transition: border-color 0.15s ease, box-shadow 0.15s ease;
        }}
        .search-input:focus {{
            border-color: var(--accent);
            box-shadow: 0 0 0 1px var(--accent);
        }}
        .search-icon {{
            position: absolute;
            left: 16px;
            color: var(--text-secondary);
            pointer-events: none;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .search-icon svg {{
            width: 18px;
            height: 18px;
            fill: currentColor;
        }}
        .engine-badge {{
            position: absolute;
            right: 12px;
            background: #2b2b32;
            color: var(--text-secondary);
            font-size: 11px;
            font-weight: 500;
            padding: 4px 8px;
            border-radius: var(--radius-sm);
            pointer-events: none;
            border: 1px solid var(--border);
        }}
        .section {{
            width: 100%;
            display: flex;
            flex-direction: column;
            gap: 12px;
        }}
        .section-title {{
            font-size: 12px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.6px;
            color: var(--text-secondary);
            padding-left: 2px;
        }}
        .shortcuts-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
            gap: 12px;
            width: 100%;
        }}
        .shortcut-card {{
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            gap: 8px;
            padding: 16px 10px;
            background-color: var(--surface);
            border: 1px solid var(--border);
            border-radius: var(--radius-md);
            text-decoration: none;
            color: var(--text-primary);
            transition: all 0.15s ease;
        }}
        .shortcut-card:hover {{
            background-color: var(--surface-hover);
            border-color: #3e3e46;
            transform: translateY(-1px);
        }}
        .shortcut-icon {{
            width: 34px;
            height: 34px;
            border-radius: 8px;
            background-color: #2a2a30;
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: 600;
            font-size: 14px;
            color: var(--text-primary);
            border: 1px solid #36363e;
        }}
        .shortcut-label {{
            font-size: 12px;
            font-weight: 500;
            text-align: center;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            max-width: 90px;
        }}
        .recent-list {{
            display: flex;
            flex-direction: column;
            gap: 6px;
            background-color: var(--surface);
            border: 1px solid var(--border);
            border-radius: var(--radius-lg);
            padding: 6px;
        }}
        .recent-item {{
            display: flex;
            align-items: center;
            gap: 12px;
            padding: 8px 12px;
            border-radius: var(--radius-md);
            text-decoration: none;
            color: var(--text-primary);
            transition: background-color 0.12s ease;
        }}
        .recent-item:hover {{
            background-color: var(--surface-hover);
        }}
        .recent-mono {{
            width: 24px;
            height: 24px;
            border-radius: 6px;
            background-color: #2b2b32;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 11px;
            font-weight: 600;
            color: var(--text-secondary);
            flex-shrink: 0;
        }}
        .recent-info {{
            display: flex;
            flex-direction: column;
            gap: 2px;
            overflow: hidden;
        }}
        .recent-title {{
            font-size: 13px;
            font-weight: 500;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }}
        .recent-url {{
            font-size: 11px;
            color: var(--text-secondary);
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }}
        .footer-shortcuts {{
            margin-top: 16px;
            display: flex;
            gap: 16px;
            font-size: 11px;
            color: #6a6a75;
        }}
        .footer-kbd {{
            background: #202024;
            padding: 2px 6px;
            border-radius: 4px;
            border: 1px solid #2d2d34;
            color: #888894;
            font-family: monospace;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="brand">
            <div class="brand-logo">The Browser</div>
            <div class="brand-tagline">Minimalist, focused, Linux-native.</div>
        </div>

        <div class="search-container">
            <form action="{form_action}" method="GET" class="search-form">
                <div class="search-icon">
                    <svg viewBox="0 0 16 16"><path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1.007 1.007 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0z"/></svg>
                </div>
                <input
                    type="text"
                    name="{query_param}"
                    class="search-input"
                    placeholder="Search with {engine_name} or enter URL..."
                    autocomplete="off"
                    autofocus
                    spellcheck="false"
                >
                <div class="engine-badge">{engine_name}</div>
            </form>
        </div>

        {shortcuts_html}

        {recent_html}

        <div class="footer-shortcuts">
            <div><span class="footer-kbd">Ctrl+L</span> Address Bar</div>
            <div><span class="footer-kbd">Ctrl+T</span> New Tab</div>
            <div><span class="footer-kbd">Ctrl+S</span> Toggle Sidebar</div>
        </div>
    </div>
</body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_home_html_contains_essentials() {
        let engine = SearchEngine::duckduckgo();
        let shortcuts = vec![Shortcut {
            id: "1".to_string(),
            title: "Arch Linux".to_string(),
            url: "https://archlinux.org".to_string(),
            position: 0,
        }];
        let recent = vec![HistoryEntry {
            id: 1,
            url: "https://rust-lang.org".to_string(),
            title: "Rust Programming Language".to_string(),
            visit_count: 5,
            last_visit_time: 12345678,
        }];

        let html = generate_home_html(&engine, &shortcuts, &recent, true, true);
        assert!(html.contains("DuckDuckGo"));
        assert!(html.contains("Arch Linux"));
        assert!(html.contains("https://archlinux.org"));
        assert!(html.contains("Rust Programming Language"));
        assert!(html.contains("https://rust-lang.org"));
        assert!(html.contains("Ctrl+L"));
    }
}
