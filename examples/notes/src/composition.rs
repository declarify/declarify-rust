//! Handwritten dependency wiring. Nothing here is generated; regeneration
//! never touches this file. See `docs/02-architecture.md` ("Consumer
//! layout") and `docs/06-extension-model.md` ("backend selection is
//! handwritten composition").
//!
//! `SqliteNoteRepository` is generated once, against the `rusqlite` API
//! surface, and works unchanged against either supported storage backend:
//! the default `sqlite-backend` Cargo feature backs it with real `rusqlite`;
//! the `doltlite-backend` feature (Linux/macOS only) swaps in `rusqdoltlite`
//! instead, which links under the identical `rusqlite` extern crate name (a
//! true drop-in replacement — see `docs/09-decisions.md` for why backend
//! selection is therefore a Cargo feature here, not a second Rust type/
//! constructor to choose between at the call site below).

use std::path::Path;

use crate::domain::note_validator::NonEmptyTitleValidator;
use crate::generated::application::note::{DefaultNoteService, NoteRepository, NoteService};
use crate::generated::persistence::note::{migrate, SqliteNoteRepository};
use declarify_runtime::{SystemClock, UuidGenerator};

/// Builds the default `Note` service over an injected repository, using the
/// real system clock, a random-UUID identifier policy, and this
/// application's custom title validator. Selecting a storage backend (a real
/// repository, or a test double for tests) means choosing what to pass in
/// here — nothing generated changes.
pub fn build_note_service(repository: impl NoteRepository) -> impl NoteService {
    DefaultNoteService::new(
        repository,
        SystemClock,
        UuidGenerator,
        NonEmptyTitleValidator,
    )
}

/// Opens (creating if necessary) a `Note` repository at `path` and applies
/// the fresh-schema migration. This is the handwritten connection-creation
/// step `docs/04-storage.md`'s adapter seam calls for; everything else about
/// the adapter is generated. Which physical database this actually talks to
/// — SQLite or DoltLite — is decided by which of the `sqlite-backend`
/// (default) / `doltlite-backend` Cargo features this crate was built with.
pub fn open_sqlite_note_repository(path: &Path) -> rusqlite::Result<SqliteNoteRepository> {
    let conn = rusqlite::Connection::open(path)?;
    migrate(&conn)?;
    Ok(SqliteNoteRepository::new(conn))
}
