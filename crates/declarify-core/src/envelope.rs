//! The framework-owned `TypeMeta + Metadata + Spec + Status` protocol.
//!
//! `Spec` (`S`) and `Status` (`T`) are supplied entirely by the application;
//! everything else here is owned by Declarify. See
//! `docs/01-resource-model.md`.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeMeta {
    pub kind: String,
    #[serde(rename = "apiVersion")]
    pub api_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    pub id: String,
    pub name: Option<String>,
    pub labels: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, String>,
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource<S, T> {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub metadata: Metadata,
    pub spec: S,
    pub status: T,
}
