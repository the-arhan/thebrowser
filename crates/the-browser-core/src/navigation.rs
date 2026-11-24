//! Navigation target resolution and search engine abstraction.

use serde::{Deserialize, Serialize};

/// Target for browser navigation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigationTarget {
    /// A valid direct web URL (http, https, file, about).
    Url(String),
    /// A search query resolved through a search engine.
    Search {
        query: String,
        url: String,
    },
    /// An empty or blank target.
    Blank,
}

impl NavigationTarget {
    pub fn target_url(&self) -> &str {
        match self {
            Self::Url(url) => url,
            Self::Search { url, .. } => url,
            Self::Blank => "about:blank",
        }
    }
}

/// Search engine configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchEngine {
    pub name: String,
    pub query_template: String,
}

impl SearchEngine {
    pub fn new(name: impl Into<String>, query_template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            query_template: query_template.into(),
        }
    }

    /// Privacy-focused DuckDuckGo default engine.
    pub fn duckduckgo() -> Self {
        Self::new("DuckDuckGo", "https://duckduckgo.com/?q=%s")
    }

    /// Google search engine.
    pub fn google() -> Self {
        Self::new("Google", "https://www.google.com/search?q=%s")
    }

    /// Startpage privacy search engine.
    pub fn startpage() -> Self {
        Self::new("Startpage", "https://www.startpage.com/sp/search?query=%s")
    }

    /// Bing search engine.
    pub fn bing() -> Self {
        Self::new("Bing", "https://www.bing.com/search?q=%s")
    }

    /// Brave search engine.
    pub fn brave() -> Self {
        Self::new("Brave", "https://search.brave.com/search?q=%s")
    }

    /// Match engine by name (case-insensitive) or default to DuckDuckGo.
    pub fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "google" => Self::google(),
            "startpage" => Self::startpage(),
            "bing" => Self::bing(),
            "brave" => Self::brave(),
            _ => Self::duckduckgo(),
        }
    }

    /// Build a search URL for a given query string.
    pub fn build_url(&self, query: &str) -> String {
        let encoded = url::form_urlencoded::byte_serialize(query.trim().as_bytes()).collect::<String>();
        self.query_template.replace("%s", &encoded)
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::duckduckgo()
    }
}

/// Resolves user omnibox text into a structured navigation target.
///
/// Rules:
/// 1. Empty or whitespace-only input -> `Blank` ("about:blank").
/// 2. Explicit protocols (`http://`, `https://`, `about:`, `file://`) -> `Url`.
/// 3. Localhost, loopback (`127.0.0.1`), IPv4 with port -> `Url` with `http://`.
/// 4. Strings without spaces that contain a dot (domain name pattern) -> `Url` with `https://`.
/// 5. Everything else -> `Search`.
pub fn resolve_input(input: &str, engine: &SearchEngine) -> NavigationTarget {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed == "about:blank" {
        return NavigationTarget::Blank;
    }

    // Custom internal shortcuts
    if trimmed.eq_ignore_ascii_case("home") || trimmed.eq_ignore_ascii_case("about:home") {
        return NavigationTarget::Url("thebrowser://home".to_string());
    }
    if trimmed.eq_ignore_ascii_case("settings") || trimmed.eq_ignore_ascii_case("about:settings") {
        return NavigationTarget::Url("thebrowser://settings".to_string());
    }

    // Direct protocols
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("about:")
        || trimmed.starts_with("file://")
        || trimmed.starts_with("thebrowser://")
    {
        return NavigationTarget::Url(trimmed.to_string());
    }

    // Check if input looks like a local host or address
    if is_local_address(trimmed) {
        return NavigationTarget::Url(format!("http://{trimmed}"));
    }

    // Check if input looks like a domain name
    if is_domain_pattern(trimmed) {
        return NavigationTarget::Url(format!("https://{trimmed}"));
    }

    // Otherwise, treat as search query
    NavigationTarget::Search {
        query: trimmed.to_string(),
        url: engine.build_url(trimmed),
    }
}

fn is_local_address(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    lower == "localhost"
        || lower.starts_with("localhost:")
        || lower.starts_with("127.0.0.1")
        || lower.starts_with("0.0.0.0")
        || lower.starts_with("[::1]")
}

fn is_domain_pattern(input: &str) -> bool {
    if input.contains(' ') {
        return false;
    }

    // Extract host portion (before slash or question mark)
    let host = input
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(input);

    let host_no_port = host
        .split(':')
        .next()
        .unwrap_or(host);

    if !host_no_port.contains('.') {
        return false;
    }

    let parts: Vec<&str> = host_no_port.split('.').collect();
    if parts.len() < 2 {
        return false;
    }

    // Ensure all labels are valid domain characters
    for part in &parts {
        if part.is_empty() || part.starts_with('-') || part.ends_with('-') {
            return false;
        }
        if !part.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return false;
        }
    }

    let tld = parts.last().unwrap();
    // TLD should be alphabetic and at least 2 chars, or an all-numeric IPv4 component
    (tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()))
        || parts.iter().all(|p| p.parse::<u8>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_empty_and_blank() {
        let engine = SearchEngine::default();
        assert_eq!(resolve_input("", &engine), NavigationTarget::Blank);
        assert_eq!(resolve_input("   ", &engine), NavigationTarget::Blank);
        assert_eq!(resolve_input("about:blank", &engine), NavigationTarget::Blank);
        assert_eq!(resolve_input("home", &engine), NavigationTarget::Url("thebrowser://home".to_string()));
        assert_eq!(resolve_input("about:home", &engine), NavigationTarget::Url("thebrowser://home".to_string()));
        assert_eq!(resolve_input("settings", &engine), NavigationTarget::Url("thebrowser://settings".to_string()));
        assert_eq!(resolve_input("thebrowser://home", &engine), NavigationTarget::Url("thebrowser://home".to_string()));
    }

    #[test]
    fn test_resolve_explicit_urls() {
        let engine = SearchEngine::default();
        assert_eq!(
            resolve_input("https://archlinux.org", &engine),
            NavigationTarget::Url("https://archlinux.org".to_string())
        );
        assert_eq!(
            resolve_input("http://example.com/test?a=1", &engine),
            NavigationTarget::Url("http://example.com/test?a=1".to_string())
        );
        assert_eq!(
            resolve_input("file:///tmp/test.html", &engine),
            NavigationTarget::Url("file:///tmp/test.html".to_string())
        );
    }

    #[test]
    fn test_resolve_domains() {
        let engine = SearchEngine::default();
        assert_eq!(
            resolve_input("archlinux.org", &engine),
            NavigationTarget::Url("https://archlinux.org".to_string())
        );
        assert_eq!(
            resolve_input("docs.rs/gtk4/latest", &engine),
            NavigationTarget::Url("https://docs.rs/gtk4/latest".to_string())
        );
        assert_eq!(
            resolve_input("github.com/driceroland/Search", &engine),
            NavigationTarget::Url("https://github.com/driceroland/Search".to_string())
        );
    }

    #[test]
    fn test_resolve_local() {
        let engine = SearchEngine::default();
        assert_eq!(
            resolve_input("localhost", &engine),
            NavigationTarget::Url("http://localhost".to_string())
        );
        assert_eq!(
            resolve_input("localhost:8080/api", &engine),
            NavigationTarget::Url("http://localhost:8080/api".to_string())
        );
        assert_eq!(
            resolve_input("127.0.0.1:3000", &engine),
            NavigationTarget::Url("http://127.0.0.1:3000".to_string())
        );
    }

    #[test]
    fn test_resolve_search_queries() {
        let engine = SearchEngine::default();
        assert_eq!(
            resolve_input("rust gtk4 webkit6", &engine),
            NavigationTarget::Search {
                query: "rust gtk4 webkit6".to_string(),
                url: "https://duckduckgo.com/?q=rust+gtk4+webkit6".to_string(),
            }
        );
        assert_eq!(
            resolve_input("arch linux wiki", &engine),
            NavigationTarget::Search {
                query: "arch linux wiki".to_string(),
                url: "https://duckduckgo.com/?q=arch+linux+wiki".to_string(),
            }
        );
        assert_eq!(
            resolve_input("hello", &engine),
            NavigationTarget::Search {
                query: "hello".to_string(),
                url: "https://duckduckgo.com/?q=hello".to_string(),
            }
        );
        assert_eq!(
            resolve_input("what is rust?", &engine),
            NavigationTarget::Search {
                query: "what is rust?".to_string(),
                url: "https://duckduckgo.com/?q=what+is+rust%3F".to_string(),
            }
        );
    }

    #[test]
    fn test_resolve_advanced_urls() {
        let engine = SearchEngine::default();
        assert_eq!(
            resolve_input("http://192.168.1.1:8080/router", &engine),
            NavigationTarget::Url("http://192.168.1.1:8080/router".to_string())
        );
        assert_eq!(
            resolve_input("gitlab.archlinux.org/archlinux/packaging", &engine),
            NavigationTarget::Url("https://gitlab.archlinux.org/archlinux/packaging".to_string())
        );
        assert_eq!(
            resolve_input("[::1]:8000", &engine),
            NavigationTarget::Url("http://[::1]:8000".to_string())
        );
    }
}
