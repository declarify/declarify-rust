//! Shared conformance suite for any `NoteService`/`NoteRepository`
//! implementation, so every storage backend can be run through the same
//! lifecycle test. See `docs/07-scenarios-and-quality.md` scenario 5 ("the
//! same application selects [a different backend] ... and passes the same
//! lifecycle tests").
//!
//! This file is compiled once per integration-test binary that declares
//! `mod support;`, and each binary only exercises the subset it needs (e.g.
//! `cli_smoke.rs` only calls `spawn_server`), so an unused-item warning here
//! is expected, not a real dead-code smell.
#![allow(dead_code)]

use std::sync::Arc;

use notes::domain::{NoteSpec, NoteStatus};
use notes::generated::application::note::NoteService;
use notes::generated::persistence::note::{migrate, SqliteNoteRepository};
use notes::generated::resources::note::{
    CreateNoteInput, ReplaceNoteSpecInput, UpdateNoteStatusInput,
};

/// Spins up a real `axum` server, backed by an in-memory SQLite database, on
/// an OS-assigned loopback port, and returns its base URL. Used by every
/// test that must drive the API over real HTTP (the typed client, and the
/// compiled CLI binary).
pub async fn spawn_server() -> String {
    let conn = rusqlite::Connection::open_in_memory().expect("open in-memory SQLite database");
    migrate(&conn).expect("fresh-schema migration succeeds");
    let repository = SqliteNoteRepository::new(conn);
    let service: Arc<dyn NoteService> =
        Arc::new(notes::composition::build_note_service(repository));
    let router = notes::generated::http::note::routes(service);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback port");
    let addr = listener.local_addr().expect("local address");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("server error");
    });

    format!("http://{addr}")
}

pub fn run_full_lifecycle(service: impl NoteService) {
    let created = service
        .create(CreateNoteInput {
            name: Some("first".to_string()),
            spec: NoteSpec {
                title: "Title".into(),
                body: "Body".into(),
            },
        })
        .expect("create succeeds");
    assert_eq!(created.metadata.revision, 1);
    assert!(!created.status.indexed);

    let fetched = service.get(&created.metadata.id).expect("get succeeds");
    assert_eq!(fetched.spec.title, "Title");

    let listed = service
        .list(declarify_core::PageRequest::default())
        .expect("list succeeds");
    assert_eq!(listed.items.len(), 1);

    let replaced = service
        .replace_spec(
            &created.metadata.id,
            created.metadata.revision,
            ReplaceNoteSpecInput {
                spec: NoteSpec {
                    title: "New title".into(),
                    body: "Body".into(),
                },
            },
        )
        .expect("replace_spec succeeds");
    assert_eq!(replaced.metadata.revision, 2);
    assert_eq!(replaced.spec.title, "New title");
    assert!(
        !replaced.status.indexed,
        "replace_spec must preserve status"
    );

    let stale = service.replace_spec(
        &created.metadata.id,
        1, // the pre-replace revision, now stale
        ReplaceNoteSpecInput {
            spec: NoteSpec {
                title: "Stale write".into(),
                body: "Body".into(),
            },
        },
    );
    assert_eq!(stale.unwrap_err(), declarify_core::Error::Conflict);

    let status_updated = service
        .update_status(
            &created.metadata.id,
            replaced.metadata.revision,
            UpdateNoteStatusInput {
                status: NoteStatus { indexed: true },
            },
        )
        .expect("update_status succeeds");
    assert_eq!(status_updated.metadata.revision, 3);
    assert_eq!(
        status_updated.spec.title, "New title",
        "update_status must preserve spec"
    );
    assert!(status_updated.status.indexed);

    let rejected = service.create(CreateNoteInput {
        name: None,
        spec: NoteSpec {
            title: "   ".into(),
            body: String::new(),
        },
    });
    assert!(
        matches!(rejected, Err(declarify_core::Error::InvalidArgument(_))),
        "the handwritten NonEmptyTitleValidator must reject a blank title"
    );

    service
        .delete(&created.metadata.id, status_updated.metadata.revision)
        .expect("delete succeeds");
    assert_eq!(
        service.get(&created.metadata.id).unwrap_err(),
        declarify_core::Error::NotFound
    );
}
