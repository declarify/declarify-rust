//! Public entry point for consuming applications.
//!
//! Consumers depend on this crate alone and write `declarify::resource! { .. }`
//! against their own `Spec`/`Status` types. Internal crates
//! (`declarify-core`, `declarify-macros`, `declarify-schema`,
//! `declarify-codegen`) are implementation details of the generator and CLI.

pub use declarify_macros::resource;
