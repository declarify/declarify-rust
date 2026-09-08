//! List pagination types. See `docs/01-resource-model.md` ("Lists").

use serde::{Deserialize, Serialize};

/// A page of results plus an opaque cursor for the next page, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

/// List parameters. `cursor` is opaque and must be passed back exactly as
/// received from a previous [`Page::next_cursor`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRequest {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
