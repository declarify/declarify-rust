//! The DCL-0003 exit criterion: the full `Note` resource lifecycle passes
//! directly through the generated SQLite repository (docs/08-implementation
//! -plan.md, "the full resource lifecycle passes directly through SQLite").
//! Uses an in-memory SQLite database so the test needs no filesystem
//! cleanup; `composition::open_sqlite_note_repository` (the handwritten,
//! file-backed constructor) is exercised separately below.

mod support;

use notes::composition::build_note_service;
use notes::generated::persistence::note::{migrate, SqliteNoteRepository};

#[test]
fn full_lifecycle_through_generated_sqlite_repository() {
    let conn = rusqlite::Connection::open_in_memory().expect("open in-memory SQLite database");
    migrate(&conn).expect("fresh-schema migration succeeds");
    let repository = SqliteNoteRepository::new(conn);
    let service = build_note_service(repository);

    support::run_full_lifecycle(service);
}

#[test]
fn open_sqlite_note_repository_creates_a_working_file_backed_database() {
    let dir = std::env::temp_dir().join(format!("declarify-notes-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("notes.sqlite3");

    let repository =
        notes::composition::open_sqlite_note_repository(&db_path).expect("open sqlite repository");
    let service = build_note_service(repository);
    support::run_full_lifecycle(service);

    std::fs::remove_dir_all(&dir).ok();
}
