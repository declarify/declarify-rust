//! Handwritten dependency wiring. Nothing here is generated; regeneration
//! never touches this file. See `docs/02-architecture.md` ("Consumer
//! layout") and `docs/06-extension-model.md` ("backend selection is
//! handwritten composition").

use crate::domain::note_validator::NonEmptyTitleValidator;
use crate::generated::application::note::{DefaultNoteService, NoteRepository, NoteService};
use declarify_runtime::{SystemClock, UuidGenerator};

/// Builds the default `Note` service over an injected repository, using the
/// real system clock, a random-UUID identifier policy, and this
/// application's custom title validator. Selecting a storage backend (a real
/// SQLite/DoltLite repository, once DCL-0003/0006 land, or a test double
/// today) means choosing what to pass in here — nothing generated changes.
pub fn build_note_service(repository: impl NoteRepository) -> impl NoteService {
    DefaultNoteService::new(
        repository,
        SystemClock,
        UuidGenerator,
        NonEmptyTitleValidator,
    )
}
