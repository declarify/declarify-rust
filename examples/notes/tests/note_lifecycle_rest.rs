//! The DCL-0004 exit criterion: "the complete lifecycle passes through REST
//! using only the generated typed client" (docs/08-implementation-plan.md).
//! Binds a real `axum` server (backed by the generated SQLite repository,
//! matching scenario 3 of docs/07-scenarios-and-quality.md: "A real server
//! starts on SQLite") on an OS-assigned loopback port and drives it over
//! real HTTP with `NoteClient` — no direct calls to the service/repository.

use std::sync::Arc;

use notes::domain::{NoteSpec, NoteStatus};
use notes::generated::client::note::NoteClient;
use notes::generated::persistence::note::{migrate, SqliteNoteRepository};
use notes::generated::resources::note::{
    CreateNoteInput, ReplaceNoteSpecInput, UpdateNoteStatusInput,
};

async fn spawn_server() -> String {
    let conn = rusqlite::Connection::open_in_memory().expect("open in-memory SQLite database");
    migrate(&conn).expect("fresh-schema migration succeeds");
    let repository = SqliteNoteRepository::new(conn);
    let service: Arc<dyn notes::generated::application::note::NoteService> =
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

#[tokio::test]
async fn full_lifecycle_through_rest_and_typed_client() {
    let base_url = spawn_server().await;
    let client = NoteClient::new(base_url);

    let created = client
        .create(CreateNoteInput {
            name: Some("first".to_string()),
            spec: NoteSpec {
                title: "Title".into(),
                body: "Body".into(),
            },
        })
        .await
        .expect("create succeeds");
    assert_eq!(created.metadata.revision, 1);
    assert!(!created.status.indexed);

    let fetched = client
        .get(&created.metadata.id)
        .await
        .expect("get succeeds");
    assert_eq!(fetched.spec.title, "Title");

    let listed = client
        .list(declarify_core::PageRequest::default())
        .await
        .expect("list succeeds");
    assert_eq!(listed.items.len(), 1);

    let replaced = client
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
        .await
        .expect("replace_spec succeeds");
    assert_eq!(replaced.metadata.revision, 2);
    assert_eq!(replaced.spec.title, "New title");
    assert!(
        !replaced.status.indexed,
        "replace_spec must preserve status"
    );

    let stale = client
        .replace_spec(
            &created.metadata.id,
            1, // the pre-replace revision, now stale
            ReplaceNoteSpecInput {
                spec: NoteSpec {
                    title: "Stale write".into(),
                    body: "Body".into(),
                },
            },
        )
        .await;
    match stale {
        Err(declarify_runtime::http::ClientError::Api { code, .. }) => {
            assert_eq!(code, "conflict")
        }
        other => panic!("expected a structured `conflict` API error, got {other:?}"),
    }

    let status_updated = client
        .update_status(
            &created.metadata.id,
            replaced.metadata.revision,
            UpdateNoteStatusInput {
                status: NoteStatus { indexed: true },
            },
        )
        .await
        .expect("update_status succeeds");
    assert_eq!(status_updated.metadata.revision, 3);
    assert_eq!(
        status_updated.spec.title, "New title",
        "update_status must preserve spec"
    );
    assert!(status_updated.status.indexed);

    let rejected = client
        .create(CreateNoteInput {
            name: None,
            spec: NoteSpec {
                title: "   ".into(),
                body: String::new(),
            },
        })
        .await;
    match rejected {
        Err(declarify_runtime::http::ClientError::Api { code, .. }) => {
            assert_eq!(code, "invalid_argument")
        }
        other => panic!("expected a structured `invalid_argument` API error, got {other:?}"),
    }

    client
        .delete(&created.metadata.id, status_updated.metadata.revision)
        .await
        .expect("delete succeeds");

    match client.get(&created.metadata.id).await {
        Err(declarify_runtime::http::ClientError::Api { code, .. }) => {
            assert_eq!(code, "not_found")
        }
        other => panic!("expected a structured `not_found` API error, got {other:?}"),
    }
}

#[tokio::test]
async fn replace_spec_without_if_match_is_rejected_as_invalid_argument() {
    let base_url = spawn_server().await;
    let http = reqwest::Client::new();

    let created: notes::generated::resources::note::Note = http
        .post(format!("{base_url}/api/v1/notes"))
        .json(&CreateNoteInput {
            name: None,
            spec: NoteSpec {
                title: "Title".into(),
                body: "Body".into(),
            },
        })
        .send()
        .await
        .expect("create request sent")
        .json()
        .await
        .expect("create response decodes");

    let response = http
        .put(format!("{base_url}/api/v1/notes/{}", created.metadata.id))
        .json(&ReplaceNoteSpecInput {
            spec: NoteSpec {
                title: "New".into(),
                body: "Body".into(),
            },
        })
        .send()
        .await
        .expect("replace_spec request sent");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let body: declarify_runtime::http::ApiErrorBody =
        response.json().await.expect("error body decodes");
    assert_eq!(body.code, "invalid_argument");
}
