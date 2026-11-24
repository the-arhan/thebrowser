//! Network session abstraction for data isolation.

use std::path::{Path, PathBuf};
use webkit6::NetworkSession;

#[derive(Clone)]
pub struct EngineSession {
    inner: NetworkSession,
    data_dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
}

impl EngineSession {
    /// Returns the global default network session.
    pub fn default_session() -> Self {
        let session = NetworkSession::default()
            .unwrap_or_else(|| NetworkSession::new(None, None));
        Self {
            inner: session,
            data_dir: None,
            cache_dir: None,
        }
    }

    /// Creates an ephemeral (private browsing) network session.
    /// All cookies, cache, and storage exist strictly in memory.
    pub fn ephemeral() -> Self {
        Self {
            inner: NetworkSession::new_ephemeral(),
            data_dir: None,
            cache_dir: None,
        }
    }

    /// Creates a persistent isolated network session with dedicated storage directories.
    pub fn persistent(data_dir: impl AsRef<Path>, cache_dir: impl AsRef<Path>) -> Self {
        let d = data_dir.as_ref().to_path_buf();
        let c = cache_dir.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&d);
        let _ = std::fs::create_dir_all(&c);

        let session = NetworkSession::new(
            d.to_str(),
            c.to_str(),
        );
        session.set_itp_enabled(true);
        Self {
            inner: session,
            data_dir: Some(d),
            cache_dir: Some(c),
        }
    }

    pub fn inner(&self) -> &NetworkSession {
        &self.inner
    }

    pub fn is_ephemeral(&self) -> bool {
        self.inner.is_ephemeral()
    }

    pub fn data_dir(&self) -> Option<&Path> {
        self.data_dir.as_deref()
    }

    pub fn cache_dir(&self) -> Option<&Path> {
        self.cache_dir.as_deref()
    }
}

impl Default for EngineSession {
    fn default() -> Self {
        Self::default_session()
    }
}
