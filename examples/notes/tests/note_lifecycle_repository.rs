//! The DCL-0003 exit criterion ("the full resource lifecycle passes
//! directly through SQLite") and the DCL-0006 one ("the notes example can
//! select DoltLite on supported platforms and pass the same lifecycle
//! tests") together: this file's tests exercise the one generated
//! `SqliteNoteRepository` directly, with no SQL/REST/CLI layering above it.
//!
//! Which physical database it actually talks to is a Cargo feature choice,
//! not anything in this file or in `composition.rs` (see the doc comment on
//! `composition::open_sqlite_note_repository` for why): by default the
//! `sqlite-backend` feature is active and this is real SQLite. Run the same
//! tests against DoltLite instead — unmodified, satisfying
//! `docs/07-scenarios-and-quality.md` scenario 5's "without regenerating
//! domain, API, client, or CLI code and passes the same lifecycle tests" as
//! literally as possible — with, on Linux/macOS:
//!
//! ```text
//! cargo test -p notes --no-default-features --features doltlite-backend \
//!   --test note_lifecycle_repository
//! ```

mod support;

use notes::composition::build_note_service;
use notes::generated::persistence::note::{migrate, SqliteNoteRepository};

#[test]
fn full_lifecycle_through_the_generated_repository() {
    let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
    migrate(&conn).expect("fresh-schema migration succeeds");
    let repository = SqliteNoteRepository::new(conn);
    let service = build_note_service(repository);

    support::run_full_lifecycle(service);
}

#[test]
fn open_note_repository_creates_a_working_file_backed_database() {
    let dir = std::env::temp_dir().join(format!("declarify-notes-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("notes.db");

    let repository =
        notes::composition::open_sqlite_note_repository(&db_path).expect("open repository");
    let service = build_note_service(repository);
    support::run_full_lifecycle(service);

    std::fs::remove_dir_all(&dir).ok();
}
