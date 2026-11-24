//! Error types for the core browser domain.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    TabNotFound(String),
    InvalidUrl(String),
    EmptyTabList,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TabNotFound(id) => write!(f, "Tab not found: {id}"),
            Self::InvalidUrl(url) => write!(f, "Invalid URL: {url}"),
            Self::EmptyTabList => write!(f, "Cannot perform operation on empty tab list"),
        }
    }
}

impl std::error::Error for CoreError {}

