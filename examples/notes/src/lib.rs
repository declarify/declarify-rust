//! `notes` is the Declarify Step 0 executable specification.
//!
//! Everything under `src/domain` is handwritten. Nothing under
//! `src/generated` is ever hand-edited — it is produced by
//! `declarify generate` and overwritten wholesale by the next run.

pub mod composition;
pub mod domain;
pub mod generated;
