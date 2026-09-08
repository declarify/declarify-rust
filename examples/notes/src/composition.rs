//! Handwritten dependency wiring. Nothing here is generated; regeneration
//! never touches this file. See `docs/02-architecture.md` ("Consumer
//! layout") and `docs/06-extension-model.md` ("backend selection is
//! handwritten composition").

use std::path::Path;

use crate::domain::note_validator::NonEmptyTitleValidator;
use crate::generated::application::note::{DefaultNoteService, NoteRepository, NoteService};
use crate::generated::persistence::note::{migrate, SqliteNoteRepository};
use declarify_runtime::{SystemClock, UuidGenerator};

/// Builds the default `Note` service over an injected repository, using the
/// real system clock, a random-UUID identifier policy, and this
/// application's custom title validator. Selecting a storage backend (a real
/// SQLite/DoltLite repository, or a test double for tests) means choosing
/// what to pass in here — nothing generated changes.
pub fn build_note_service(repository: impl NoteRepository) -> impl NoteService {
    DefaultNoteService::new(
        repository,
        SystemClock,
        UuidGenerator,
        NonEmptyTitleValidator,
    )
}

/// Opens (creating if necessary) a SQLite-backed `Note` repository at `path`
/// and applies the fresh-schema migration. This is the handwritten
/// connection-creation step `docs/04-storage.md`'s adapter seam calls for;
/// everything else about the SQLite adapter is generated.
pub fn open_sqlite_note_repository(path: &Path) -> rusqlite::Result<SqliteNoteRepository> {
    let conn = rusqlite::Connection::open(path)?;
    migrate(&conn)?;
    Ok(SqliteNoteRepository::new(conn))
}
