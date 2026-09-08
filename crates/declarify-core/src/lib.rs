//! Shared, framework-owned types for Declarify resources: the
//! `TypeMeta + Metadata + Spec + Status` envelope, list pagination, the
//! application/service-level error type, and the `Operation` vocabulary
//! shared by the compile-time declaration macro and the external source
//! scanner. See `docs/01-resource-model.md` and `docs/02-architecture.md`.

mod envelope;
mod error;
mod page;

pub use envelope::{Metadata, Resource, TypeMeta};
pub use error::Error;
pub use page::{Page, PageRequest};

use std::fmt;

/// A resource operation that a `declarify::resource!` declaration may enable.
///
/// The wire/keyword form (used in the declaration DSL) is the `snake_case`
/// name returned by [`Operation::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Operation {
    Create,
    Get,
    List,
    UpdateSpec,
    UpdateStatus,
    Delete,
}

impl Operation {
    /// All operations, in the stable order used for diagnostics and codegen.
    pub const ALL: [Operation; 6] = [
        Operation::Create,
        Operation::Get,
        Operation::List,
        Operation::UpdateSpec,
        Operation::UpdateStatus,
        Operation::Delete,
    ];

    /// The declaration-surface keyword for this operation, e.g. `"update_spec"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Operation::Create => "create",
            Operation::Get => "get",
            Operation::List => "list",
            Operation::UpdateSpec => "update_spec",
            Operation::UpdateStatus => "update_status",
            Operation::Delete => "delete",
        }
    }

    /// Parses a declaration-surface keyword into an [`Operation`].
    ///
    /// Returns `None` for any identifier outside the fixed Step 0 vocabulary.
    pub fn parse(keyword: &str) -> Option<Operation> {
        Operation::ALL.into_iter().find(|op| op.as_str() == keyword)
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_every_operation_keyword() {
        for op in Operation::ALL {
            assert_eq!(Operation::parse(op.as_str()), Some(op));
        }
    }

    #[test]
    fn rejects_unknown_keyword() {
        assert_eq!(Operation::parse("archive"), None);
    }
}
