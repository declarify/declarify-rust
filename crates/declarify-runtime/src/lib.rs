//! Small, type-independent building blocks that generated code depends on:
//! the `Clock`/`IdGenerator` composition seams
//! (`docs/06-extension-model.md`), shared HTTP error/response handling used
//! by every generated REST handler and typed client method, and shared CLI
//! JSON I/O (`docs/05-rest-client-cli.md`).
//!
//! Deliberately narrow: this crate must not grow into a place to hide
//! business behavior that belongs in a handwritten service or validator.

pub mod cli;
pub mod http;

use chrono::{DateTime, Utc};

/// Time policy seam. Generated default services call this instead of
/// `Utc::now()` directly so tests can inject deterministic time.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// The real system clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Identifier policy seam. Generated default services call this to assign a
/// new resource's `metadata.id` on create.
pub trait IdGenerator: Send + Sync {
    fn generate(&self) -> String;
}

/// The default ID generator: a random UUID v4, rendered in hyphenated form.
#[derive(Debug, Clone, Copy, Default)]
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn generate(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_generator_produces_distinct_ids() {
        let gen = UuidGenerator;
        assert_ne!(gen.generate(), gen.generate());
    }

    #[test]
    fn system_clock_is_monotonically_non_decreasing() {
        let clock = SystemClock;
        let first = clock.now();
        let second = clock.now();
        assert!(second >= first);
    }
}
